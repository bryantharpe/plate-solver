# Codebase Review — plate-solver (Rust)

> Scan performed 2026-06-29 on branch `feat/rust-implementation` (HEAD `8932c62`).
> Scope: the six `ps-*` implementation crates. `reference-solutions/` and `openspec/`
> consulted as sources of truth, not audited. Method: full `cargo build/test/clippy/fmt`
> run + focused deep read of hotspots (`ps-solve/src/lib.rs`, `ps-grpc/src/service.rs`,
> `ps-detect/*`, `ps-db/mmap.rs|loader.rs|importer.rs`) + skim of the rest.
>
> Lenses: (1) Correctness & stability, (2) Code quality & structure,
> (3) Maintainability & testing, (4) Performance & mobile-readiness.

## Tooling snapshot (actual vs documented)

| Check | Documented | Actual (this scan) |
|---|---|---|
| `cargo build --workspace` | — | ✅ exit 0 |
| `cargo test --workspace` | 182 pass, 1 ignored | ✅ **182 passed, 0 failed, 1 ignored** (`sv6_diagnostic_solve_sweep`) |
| `cargo clippy --workspace` | 24 warnings, 0 errors | ✅ warnings only, 0 errors (3 `unnecessary_unwrap` in `ps-grpc`, `needless_range_loop`, `type_complexity`, `too_many_arguments`, etc.) |
| `cargo fmt --check` | clean | ✅ clean |

The `IMPLEMENTATION-STATUS.md` claims match the actual tooling output. This is a genuine
positive — the doc is accurate, not aspirational.

---

## Top concerns (most severe first)

### C1. ~648 MB transient allocation in the solver hot loop — mobile blocker
**File:** `ps-solve/src/lib.rs:582-594` (`combinations_4`), called at `:190`.

`solve_from_centroids` materializes **every** 4-element combination of pattern centroids into
a `Vec<[usize; 4]>` *before* iterating:

```rust
fn combinations_4(n: usize) -> Vec<[usize; 4]> { … result.push([a,b,c,d]); … }
…
'outer: for combo in combinations_4(num_pattern_centroids) {
```

`num_pattern_centroids` is bounded by `verification_stars_per_fov`. The bundled
`default_database.npz` ships **vsfov = 150** (confirmed from `props_packed`). C(150,4) =
**20,260,275** combos × 32 B = **~648 MB** allocated up front, held for the whole solve.

- On a phone (the stated build target, PRD §mobile-runtime) this is an instant OOM.
- The reference (cedar-solve) iterates combinations lazily (`itertools.combinations`) and
  abandons on timeout — so it never pays this upfront cost.
- Severity is high because it's on the happy path of the flagship RPC, not an edge case.
- **Fix:** replace the eager `Vec` with a lazy iterator (a 4-deep nested loop or an
  `itertools`-style combinations iterator) so the timeout/cancel checks can fire between
  combos and nothing is materialized.

### C2. `solve_from_image` hardcodes detection params and ignores the request's
**File:** `ps-solve/src/lib.rs:597-612`.

```rust
let (stars, _, _, _) = ps_detect::get_stars_from_image(
    image, 1.0, 4.0, false, 1, true, false,
);
```

`solve_from_image` calls detection with fixed `sigma=4.0`, `noise_estimate=1.0`,
`binning=1`, `normalize_rows=false`. But the gRPC `SolveFromImageRequest` carries an
`extract: CentroidsRequest` with client-chosen `sigma`, `binning`, `normalize_rows`,
`detect_hot_pixels` (`ps-grpc/src/service.rs:280-342`) — and `ps_solve_image` **ignores all of
it**. The client's `sigma`/`binning` are silently dropped. The `sv6_solve_from_image_parity`
test passes only because it happens to use sigma≈4 / binning 1. A client that requests
binning=2 or sigma=8 gets sigma=4/binning=1 behavior with no error. This is a correctness
contract mismatch at the public API boundary.

### C3. `debug_assert!` guards an `unsafe` alignment contract — UB in release builds
**File:** `ps-db/src/mmap.rs:75-76`.

```rust
debug_assert_eq!(ptr.align_offset(4), 0);
unsafe { std::slice::from_raw_parts(ptr as *const [f32; 6], self.star_table_count) }
```

`from_raw_parts` requires the pointer be aligned for `[f32; 6]`. The alignment is checked
with `debug_assert!`, which is **stripped in release**. If the file layout ever produces a
misaligned `star_table` offset, a release build silently constructs an aliased, misaligned
slice = **undefined behavior**. The sibling accessors (`key_hashes` `:94`, `largest_edge`
`:112`) do a *runtime* check and `panic!` on misalignment — `star_table` should match that
pattern (runtime check → `Status::internal` / `Err`), not rely on a debug assertion. Today it
happens to be 4-byte aligned because of the 8-byte section layout, but that's an invariant
the code doesn't enforce in release.

### C4. gRPC boundary trusts client `width`/`height` with truncating casts
**Files:** `ps-grpc/src/service.rs:134-135, 263-264, 310-311`.

```rust
let width  = input_image.width as u32;   // width is i32 from the wire
let height = input_image.height as u32;
…
let expected_len = (width * height) as usize;   // u32 multiply
```

- `i32 as u32` wraps negatives to huge values (e.g. `-1` → `4_294_967_295`).
- `width * height` is a `u32` multiply; in **debug** it panics on overflow, in **release**
  it wraps silently — so `expected_len` can be wrong, and the subsequent length check is the
  only thing standing between a bad request and `GrayImage::from_raw`. `from_raw` does reject
  mismatches (`:151`, `:327`), so this isn't directly exploitable, but the casts are
  unsound-by-construction and the `as u32` should be a `u32::try_from(i32).map_err(...)?`
  with an explicit range check. `solve_from_centroids` does the same `req.height as usize`
  from i32 (`:262-263`) with **no** dimension validation at all before indexing math.

### C5. `panic!` / `assert!` on the gRPC handler thread on realistic input
**Files:** `ps-detect/src/detect.rs:32,36`; `ps-detect/src/blob.rs:193-194,40,70`;
`ps-detect/src/gate.rs:50,100`; `ps-db/src/mmap.rs:101,114`.

- `detect.rs:36` — `panic!("Invalid binning argument {}, must be 1, 2, 4, or 8", binning)`.
  The gRPC layer guards binning to {1,2,4} (`service.rs:168-181`), but `get_stars_from_image`
  is `pub` and any other caller (or a future RPC) gets a panic, not an error.
- `blob.rs:193-194` — `assert!(p >= -0.5)` / `assert!(p <= 0.5)` in `peak_coord_1d` fire when
  the quadratic-interpolation denominator `(a - 2b + c)` is ~0, i.e. on a **flat noise region**
  where `a == b == c`. That's realistic astronomical input, and it takes down the handler
  thread with an assertion panic. No test covers this case.
- `mmap.rs:101,114` — `panic!` on misaligned `key_hashes`/`largest_edge` offsets on a
  malformed DB file. A corrupt/truncated DB shouldn't crash the server; it should surface as
  a load error.

The gRPC service runs these on tokio worker threads; a panic in tonic's default config
terminates the connection (and with some panic-handlers, the worker). These should be
`Result`-returning.

### C6. No concurrency story for the solver; whole DB shared but solve is single-threaded
**File:** `ps-grpc/src/service.rs:23-31` (`db: Arc<Database>`), `ps-solve/src/lib.rs`.

The DB is shared across all gRPC workers via `Arc<Database>` and is correctly
`Send+Sync` (ps-db: `MmappedDatabase` is read-only; `Database` is plain owned data — sound).
But `solve_from_centroids` is fully **serial** — no `rayon`/parallelism anywhere — and each
solve can take ~8–20 s (the `sv6` tests use `solve_timeout=120_000` ms and finish in ~21 s of
test wall-time). Under concurrent `SolveFrom*` load, N requests occupy N worker threads at
~100% CPU each with zero shared-state benefit. The `SolveParams.cancel_flag` exists but no
RPC deadline is wired to it, so a client that drops the request still keeps the server
computing until its own `solve_timeout`. PRD lists rayon as "optional, feature-gated, off on
mobile" — but there's no feature flag at all today; it's just absent.

### C7. Duplicated file-format and lookup logic across `ps-db`
**Files:** `ps-db/src/loader.rs` vs `mmap.rs` (header/section parsing, ~150 lines duplicated);
`ps-db/src/lookup.rs` vs `mmap.rs:387-446` (`lookup_pattern` vs `lookup_pattern_mmap`,
same probe/pre-filter algorithm).

Two independent readers for the same binary format is a maintenance hazard: a format
change must be made in two places, and the in-RAM and mmap paths can silently diverge (the
`test_mmap_lookup_parity` test guards this at the result level, but not at the structural
level). Worth extracting a shared section parser + a single lookup routine parameterized
over accessor (`&[T]` vs mmap-backed slice).

### C8. Deferred parity leaves a real correctness gap unverified
**Files:** `openspec/IMPLEMENTATION-STATUS.md:60-62, 84-87`; `ps-dbgen/tests/e2e.rs`.

`ps-dbgen` count parity vs `default_database.npz` is **DEFERRED** — Hipparcos/Tycho source
catalogs aren't in-repo, so the DB generator has never been checked to produce the same
pattern count (1,010,981 per the bundled npz) as the reference. Structural validity
(shapes/determinism) is tested, but the actual generation correctness is only logged via
`eprintln!`. This means the one pipeline stage that turns catalogs into the lookup table is
the least-verified stage. A silent regression in `patterns.rs`/`hash_insert.rs` could ship
unnoticed.

### C9. `unwrap()`/`expect()` on logic invariants inside hot loops
**Files:** `ps-detect/src/blob.rs:251` (`.expect("…must have been merged")`), `:242,:265`
(`assert_eq!` on merge bookkeeping); `ps-dbgen/src/hash_insert.rs:53,160` (`unwrap()` on
pattern max, `panic!("hash table full")`).

These assert invariants of self-authored algorithms rather than returning errors. They're
probably correct by construction, but a future refactor that breaks the merge-chain or
probe invariant turns a data issue into a panic in the offline DB builder. `hash_insert`
panicking "hash table full" is acceptable for a CLI builder but worth a `Result`.

### C10. Style/lint debt is small but permanent
24 clippy warnings, all non-blocking, documented in `IMPLEMENTATION-STATUS.md` as
intentionally left. Most are fine, but a few are worth fixing because they hide intent:
`gate.rs:236-253` (`reject_hot_pixels` has an `if binning==1 { … } else { … }` where **both
branches are identical** — dead conditional that suggests a missing specialization);
`apply_legacy_fallbacks` takes **16 `Option` arguments** (`ps-db/src/lib.rs:212`) and is
called with 16 `None`s in ~6 test sites — a builder/config struct would be far clearer and
prevent argument-order bugs.

---

## Top positives

### P1. Numerical parity is treated as a hard contract — and it's real
Every math primitive in `ps-core` has a parity test against captured Python fixtures
(`ps-core/tests/*_parity.rs`), with tight tolerances (<1e-9 for most, <1e-3 for f32-storage
paths, integer-exact for pattern key/hash). The binding conventions are documented at the
top of `ps-core/src/lib.rs` and enforced (f64 compute / f32 storage, `2·asin(d/2)` angles,
`(y,x)` pixel order, `pattern_bins = round(1/(4·pattern_max_error))`). The 182-test suite
actually runs and passes, and the end-to-end `sv6_solve_from_image_parity` recovers RA/Dec
within 10 arcsec of the Python reference on a real JPEG. This is the single most important
correctness property of the project and it's genuinely held.

### P2. Clean workspace decomposition with one-responsibility crates
`ps-core` (math, no I/O) → `ps-detect` → `ps-db` → `ps-dbgen` (CLI) → `ps-solve` → `ps-grpc`.
Dependencies flow strictly downward; `ps-core` has zero non-math deps. Feature flags are
used sensibly (`kd-tree` gated on `kiddo` in `ps-db`/`ps-dbgen`). Build is fast and clean.
This is exactly the structure the OpenSpec `project.md` specified, and it survives contact
with the code.

### P3. Honest, accurate implementation-status documentation
`openspec/IMPLEMENTATION-STATUS.md` enumerates per-feature test counts, parity outcomes with
measured tolerances, the deferred items, *and* the 24 clippy warnings — rather than
declaring victory. The tooling numbers it claims (182 pass / 1 ignored / 24 warnings)
reproduced exactly on this scan. Deferred items are explicitly logged with where/why. This
kind of doc is rare and high-value.

### P4. Sound concurrency foundation for the DB
`MmappedDatabase` is read-only-mmap backed, owns its `Mmap` handle, exposes slices borrowing
`&self` (so no use-after-free), and is `Send+Sync` correctly (`mmap.rs:59-60`) — concurrent
gRPC readers share it safely. `catalog_entry()` copies 4 u32s rather than handing out mmap
references, sidestepping lifetime pain. The KD-tree is built once (`&mut self`) then read
concurrently. This is a well-thought-out read-only-shared-resource design.

### P5. gRPC service correctly handles the cedar-detect wire interop
`service.rs` does the `(x,y)↔(y,x)` swap at the service boundary (exactly as `project.md` §4
mandates), validates image dimensions, returns proper `tonic::Status` codes
(`InvalidArgument`/`Internal`) for the failure paths it *does* guard, and has a real
cedar-detect-protocol interop test (`cedar_detect_interop`, `:748`) that encodes/decodes in
both directions. The `algorithm_time` `Duration` round-trips through both protos.

### P6. Determinism is a tested property, not an assumption
`ps-dbgen` has a byte-identical-determinism test (two independent runs produce identical
output). For an offline-generated, content-addressed pattern database that ships to devices,
this matters: it makes DB builds reproducible and cacheable. Paired with the f32-storage
decision, it keeps the DB small and stable.

### P7. Failure paths in the solver are exercised, not just the happy path
`solve_from_centroids` has explicit tests for `Timeout`, `Cancelled`, `TooFew`, `NoMatch`,
reflection-rejection (`det(R)<0`), and binomial-rejection-above-threshold
(`sv4_mismatch_above_threshold`). All five `SolveStatus` variants are proven reachable. The
solver doesn't silently hang or return a bogus match on bad input — it returns a typed
status. (The gap is that it sometimes *panics* before reaching that typed status — see C5.)

### P8. Conservative, intentional dependency pinning
`tonic 0.11` / `prost 0.12` pinned to match cedar-detect for interop; `image =0.25.1` /
`imageproc =0.25.0` pinned with `default-features=false` to avoid edition-2024 transitive
deps that break on Rust 1.83. These are documented in `Cargo.toml` comments. Pinning for
interop rather than chasing latest is the right call for a parity-bound reimplementation.

---

## Severity-ranked fix list (recommended order)

1. **C1** — lazy combinations iterator. Biggest mobile-readiness win, contained to one
   function.
2. **C2** — thread client detection params through `solve_from_image` (or remove them from
   the RPC). API correctness.
3. **C5** — convert `panic!`/`assert!` on handler-reachable paths to `Result`, starting with
   `peak_coord_1d` (flat-profile divide-by-zero) and `get_stars_from_image` binning.
4. **C3** — promote the `star_table` alignment `debug_assert!` to a runtime check returning
   a load error.
5. **C4** — `u32::try_from` + explicit bounds on `width`/`height` at the gRPC boundary.
6. **C6** — wire RPC deadline → `cancel_flag`; decide on the rayon feature flag.
7. **C8** — stand up a count-parity check for `ps-dbgen` (even a checked-in small-catalog
   fixture with an expected count).
8. **C7, C10** — structural cleanups (shared parser, config struct for the 16-arg fallback,
   dead `if/else` in `reject_hot_pixels`).

## Notes / caveats

- The reference `default_database.npz` is **not** generated by `ps-dbgen` in this repo; it's
  the Python-built artifact. So C8 (count parity) is the gap between "the format loads" and
  "our generator reproduces it."
- `feat-07 mobile-runtime` (UniFFI, on-device packaging) is explicitly deferred and out of
  this scan's scope; C1/C6 are the mobile-readiness findings *visible* in the current crates.
- Only the `ps-*` crates were audited for defects; `reference-solutions/` was treated as the
  read-only oracle, per the repo's own `project.md` §2.