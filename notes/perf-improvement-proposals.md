# Perf round-3 specs — DBL (ps-db lookup trim) / FU-C (parallel search) / ZCS (zero-copy shmem)

_Authored 2026-07-06. Upgraded from proposal form to implementation-spec form
(FU-A/FU-B fidelity) per user decision 2026-07-06: all open design decisions
below are **resolved at spec level**; no product code has been written.
Beaded into `plan.md` under "Perf round-3 beads (DBL / ZCS / FU-C)". Grounded
in the FUB.1/FUB.2/FUB.3 measurements in `notes/solve-perf-measurements.md`._

## Where the time goes (post-FU-B baseline, FUB.3 2026-07-05)

- MatchFound path: effectively solved (t_solve ≈ 0.07 ms median, 1.43× vs
  cedar_flow). Exhaustion path (NoMatch/weak-signal): ~65–67 ms / 8,855 combos
  / **7.6 µs/combo** on hale_bopp — this is what a `solve_timeout` budget buys.
- Exhaustion attribution: `ps_db::lookup::lookup_pattern` **~51%** (2,151,765
  calls/solve, ~15.6 ns each, memory-bound); `ps_db::nearby_stars` **~15%**;
  verify-block collects ~10% (FUB.2 tried, no detectable win, reverted);
  candidate_keys build+sort ~15% (FUB.2 tried, no win, reverted).
- ps-solve-scope allocation levers are **exhausted by measurement** (FUB.2).
  Do not re-attempt them; the levers below are ps-db (DBL), parallelism
  (FU-C), and the gRPC/detect API surface (ZCS).

## Constraints carried forward, unchanged

No accuracy regression, ever: never loosen a tolerance, `#[ignore]` a parity
test, or stub a check. SP2.1's parity STOP rule applies to every bead: the 9
`ps_grpc_vs_cedar_flow` `primary_same_catalog` harness comparisons must remain
identical-green; any change means a bug — STOP. Measure before/after every
step with the named metric; **revert any step the measurement says didn't
help** (FUB.2 regime). `combos_examined` = 8,855 on hale_bopp defaults is an
invariant for DBL and for FU-C-flag-off (it counts iterations, not
allocations).

**ps-detect constraint, narrowed (user decision 2026-07-06):** detect
*algorithms and pixel math* remain untouched — but **API-surface changes
(signature/view-type threading) are authorized for ZCS**, gated on
byte-identical detect parity fixtures.

## Benchmark environment facts (beads must respect these)

- Benchmark host is **aarch64** (FUB.3). `perf` is unavailable
  (`perf_event_paranoid=4`, no samply/valgrind) — profiling uses temporary
  `Instant` instrumentation, fully reverted after (FUB.1 method).
- Noise floor on `combo_count` t_solve is ~±10% (~5 ms) at n=10–12 (FUB.2).
  Any claimed win must clear it: use ≥12 runs, report median+mean+stdev, and
  treat SNR<1 as "no win" (FUB.2's standard).
- Metric commands:
  `cargo run --release -p ps-solve --example combo_count` (µs/combo =
  t_solve ÷ combos_examined on hale_bopp), and the eval harness
  (`tools/parity/benchmark/run_benchmark.py` → `parity.py` → `report.py`).

---

# DBL — Trim `ps_db::lookup::lookup_pattern` (+ `nearby_stars`), the dominant exhaustion cost (~66% combined)

## Motivation (measured, FUB.1)

2,151,765 `lookup_pattern` calls per hale_bopp exhaustion (243 candidate keys
× 8,855 combos) ≈ ~33.6 ms of 65 ms. Only 0.13% of lookups produce a candidate
slot; the cost is hashing + **random memory access**: each probe iteration
reads `db.key_hashes[slot]` (u16) and `db.largest_edge[slot]` (f16) — two
independent random loads into two parallel multi-MB arrays
(`ps-db/src/lookup.rs:72,78,84`), i.e. up to two cache misses per probe where
one would do. `nearby_stars` (`ps-db/src/lib.rs:189`) adds ~10 ms across the
~1.4–2.8 k verify-block entries.

## Resolved design decisions

- **D-DBL-1 (probe-pair table, additive):** add
  `pub probe_pairs: Vec<u32>` to `Database` (`ps-db/src/lib.rs:115`), packed
  `(key_hashes[i] as u32) << 16 | largest_edge[i].to_bits() as u32`, built by
  a shared helper `fn build_probe_pairs(key_hashes: &[u16], largest_edge: &[f16]) -> Vec<u32>`
  called at **all three `Database` construction sites**:
  `ps-db/src/importer.rs:332`, `ps-db/src/loader.rs:394`,
  `ps-db/src/lib.rs:139` (empty DB → empty vec). The existing `key_hashes` /
  `largest_edge` fields **stay** — the verification path reads
  `db.largest_edge[slot]` directly (`ps-solve/src/lib.rs` A1) and fires on
  only 0.13% of lookups. Memory cost: +4 bytes/slot (doubles the probe-array
  footprint); quantify at load time and record the number in the measurements
  note. No on-disk format change.
- **D-DBL-2 (scope):** `lookup_pattern_mmap` (`ps-db/src/mmap.rs:387-446`) is
  **NOT in scope** — ps-solve takes `&Database` only (`ps-solve/src/lib.rs:130`);
  the mmap variant is unused on the hot path. Add a code comment on it noting
  (a) it is intentionally not optimized here, (b) its `probe_slots` call
  (`ps-core/src/pattern.rs:164`) **eagerly allocates a `Vec<u64>` of
  `num_slots` entries per lookup** — a latent perf bug if that path ever goes
  hot — and (c) H10's dedup is the vehicle for unifying the two. Do not
  refactor it in DBL.
- **D-DBL-3 (prefetch portability):** the benchmark host is aarch64, where
  stable Rust has no prefetch intrinsic. Use a private helper:
  `#[inline(always)] fn prefetch_read(p: *const u8)` —
  aarch64: `core::arch::asm!("prfm pldl1keep, [{0}]", in(reg) p, options(nostack, preserves_flags, readonly))`;
  x86_64: `core::arch::x86_64::_mm_prefetch::<{_MM_HINT_T0}>(p as *const i8)`;
  other targets: no-op. Prefetch is advisory — it cannot change results.
- **D-DBL-4 (batching shape, API-preserving):** hash math is already public in
  `ps_core::pattern`. Add to ps-db:
  `pub fn lookup_pattern_prehashed(db, full_hash: u64, largest_edge_rad, coarse_fov_rad) -> Vec<usize>`
  (the existing body, minus the two hash lines) and
  `pub fn prefetch_probe_start(db, full_hash: u64)` (computes the initial
  probe index, issues `prefetch_read` on `probe_pairs[index]`).
  `lookup_pattern` becomes a 3-line wrapper (hash → prehashed) so existing
  callers/tests are untouched. ps-solve's inner loop
  (`ps-solve/src/lib.rs:264-283`) then: precompute the 243 `full_hash` values
  right after `candidate_keys` is built (`:237`), and when processing key
  `i`, first call `prefetch_probe_start` for key `i+1`. Early-exit-on-match
  semantics unchanged (prefetch is speculative reads only).
- **D-DBL-5 (nearby_stars shape):** the final `inds.sort_unstable()`
  (`ps-db/src/lib.rs:201`) makes output order independent of kd-query order —
  so the safe levers are: check the pinned kiddo version for
  `within_unsorted_iter` (skip the intermediate `Vec<NearestNeighbour>` +
  `map().collect()`), and/or an `_into(&mut Vec<usize>)` variant with a scratch
  buffer hoisted to the caller (`ps-solve/src/lib.rs:350`). Output must be the
  identical sorted index list. Keep `nearby_stars` as a wrapper.

## Beads (see plan.md for full AC): DBL.1 pair-table, DBL.2 prehash+prefetch, DBL.3 nearby_stars, DBL.4 re-measure + decision gate

Each step individually measured (µs/combo, ≥12 runs, SNR rule) and reverted if
it doesn't clear the noise floor. New equivalence test (DBL.1/DBL.2): sweep a
grid of pattern keys (include: hit keys, miss keys, keys whose probe chain
crosses an empty slot, FOV-filter Some/None) against the checked-in fixture DB
and assert old-path vs new-path candidate lists **identical including order**.

## Honest expectation

The probe loop is memory-bound; interleaving (≤2× fewer misses) + prefetch
(latency hiding) have real headroom against ~33.6 ms — a 30–50% lookup
reduction would be ~10–17 ms off 65 ms (~15–25%), more than everything FU-B
could reach. But no number is promised until measured; each step carries the
FUB.2 revert rule. Zero effect on easy images (1–2 combos).

---

# FU-C — Parallel combination search behind the `rayon` flag (~cores× on exhaustion)

**Status: approved for beading 2026-07-06 (previously spec-only per user
decision 2026-07-04). Implementation remains hard-gated on FUC.0 — a ps-judge
Job B design adjudication — and on H6 landing the `rayon` feature flag.**
The full original spec text stands: `notes/solve-perf-followups-spec.md` §FU-C
is the normative reference; this section only adds sequencing and the bead
decomposition.

- **FUC.0 (Job B, no code):** ps-judge rules on: ordered find-first mechanism
  (rayon `find_first` sequential-consistency sufficiency vs explicit chunk-of-K
  scan with lowest-global-index acceptance), chunk size K, timeout/cancel
  coarsening (up to K−1 extra combos after cancel — acceptable?),
  `combos_examined` policy (AtomicU64-exact vs documented
  approximate-under-rayon), and interaction with H6's deadline→cancel_flag
  wiring. Ruling is appended to this file; FUC.1 may not start without it.
- **FUC.1 (implement per ruling):** parallelize the `'outer` loop in
  `solve_from_centroids` (`ps-solve/src/lib.rs:194`) behind the H6 `rayon`
  feature. Flag **off** (default, and on mobile per PRD): byte-identical —
  provable from the diff (parallel path entirely behind the feature gate).
  Flag **on**: same matched results on all 9 images, plus a determinism test
  (same input solved twice → identical `Solution`, including matched-ID order).
- **FUC.2 (measure):** exhaustion wall-clock flag-on vs flag-off
  (`combo_count` hale_bopp) + full harness with flag on; parity STOP rule
  applies to both configurations. Measured against the **post-DBL.4 serial
  baseline** (sequencing: FUC.0 deps H6 + DBL.4) so the speedup isn't inflated
  by an unoptimized serial path.

Expected: near-linear on exhaustion, ~zero on easy images; a `solve_timeout`
budget covers ~cores× more combos. Report both honestly.

---

# ZCS — True zero-copy shmem in `ps-grpc` via a borrowed-image view API

## Motivation

FU-A removed the inline-path clone but deliberately left the shmem path
copying the **whole frame** per request — `mmap.to_vec()` at
`ps-grpc/src/service.rs:144` (ExtractCentroids) and `:328` (SolveFromImage) —
because fixing it required touching ps-detect's API, out of scope then
(comments at `:138-143` / `:322-327` say exactly this). ~0.79 MB memcpy at
1024×768, ~5 MB at 5 MP, on the path built for high-rate camera clients.

## Resolved design decision: concrete view type, NOT generics

`get_stars_from_image` already borrows its input; only the container type
(`GrayImage = ImageBuffer<Luma<u8>, Vec<u8>>`) forces ownership. Genericizing
over `C: Deref<Target=[u8]>` was considered and **rejected**: ps-detect's
binner dispatch stores **fn pointers in statics**
(`ps-detect/src/binning.rs:17-22` — `BinAndHistoFn`/`Bin2x2Fn` in `OnceLock`,
mirroring cedar-detect's `image_funcs.rs`), and statics can't hold generic
fns; generics would also monomorphize every caller. Instead:

- Add `pub type GrayImageView<'a> = image::ImageBuffer<image::Luma<u8>, &'a [u8]>;`
  to `ps-detect/src/lib.rs` (next to the existing `pub use image::GrayImage`,
  `:27`), plus a helper
  `pub fn as_view(img: &GrayImage) -> GrayImageView<'_>`
  (`ImageBuffer::from_raw(w, h, img.as_raw().as_slice()).unwrap()` — infallible
  for an exact-size slice). One concrete type → codegen and pixel math
  identical, diff mechanical.

## Signature inventory (the full list — nothing else changes)

**Input-facing params flip `&GrayImage` → `&GrayImageView<'_>`; all owned
return types stay `GrayImage`:**

| file | item |
|---|---|
| `ps-detect/src/detect.rs:15` | `get_stars_from_image` — `image` param only; `Option<GrayImage>` return stays owned |
| `ps-detect/src/binning.rs:17,19` | `BinAndHistoFn`, `Bin2x2Fn` type aliases → `for<'a> fn(&GrayImageView<'a>, …)`; `set_binner` (`:28`) follows |
| `ps-detect/src/binning.rs:34,67` | `bin_2x2`, `bin_and_histogram_2x2` — input params; `Binned2x2Result.binned` stays owned `GrayImage` |
| `ps-detect/src/gate.rs:174` | `scan_image_for_candidates` — input param (plus any private image-taking helpers in gate.rs, e.g. the hot-pixel routines, compiler-guided) |
| `ps-detect/src/noise.rs:47,81` | `estimate_noise_from_image`, `estimate_background_from_image_region` |
| `ps-detect/src/blob.rs:277` | `gate_star_2d` — **both** `image` and `higher_res_image` params (binned owned images convert via `as_view` at the call site) |
| `ps-detect/src/io.rs:17` | `load_grayscale` — unchanged (returns owned) |

**Callers to update (compiler-guided, mechanical):** `ps-solve/src/lib.rs:664`
(`solve_from_image` — its own `image: &GrayImage` param becomes
`&GrayImageView<'_>`, threading zero-copy end-to-end) and `:669`, `:1661`;
`ps-grpc/src/service.rs:13,150,197,326`; any `ps-web` call site of
`solve_from_image`; `tools/parity/_capture_binning/src/main.rs:22`; every
ps-detect unit/parity test (owned fixture image → `as_view(&img)` at the call).

## gRPC wiring (after the view API exists)

- Shmem path: validate `mmap.len() == width*height` (checks exist), build
  `GrayImageView` directly over `&mmap[..]`, keep the mmap alive for the
  request duration (bind it before the view in the same scope). Delete
  `mmap.to_vec()` + the two out-of-scope comments at both sites.
- Inline path: unchanged semantics (owned Vec moved into `GrayImage`, then
  `as_view` at the detect call — still zero-copy).
- **Decision rule for the "is shmem exercised?" question (FU-A's open
  caveat):** check whether any current client/harness path sets `shmem_name`
  (grep tools/parity + ps-web). If none does, ZCS still lands (the view API
  benefits any in-process embedder), but the measurement bead uses a synthetic
  shmem client at 1024×768 **and** 5 MP and records "no current client
  exercises shmem" verbatim in the measurements note.

## Accuracy gates

Every ps-detect parity fixture **byte-identical** (SD1/SD6 detect_parity,
binning/gate/noise parity — this is a signature change, not a pixel-math
change; ANY centroid/histogram diff is a bug: STOP). ExtractCentroids /
SolveFromImage byte-identical on fixture images via inline AND shmem paths
(new test if shmem coverage is missing). `cargo test --workspace` green;
cedar-detect interop tests green; harness parity STOP rule as always.

## Honest expectation

~0.3–1 ms/request at 1 MP (memcpy + page-faults + allocator pressure), scaling
with resolution — several ms/frame at 5 MP, which at camera rates is the
difference between noise and bottleneck. Zero effect on the inline path.

---

## Suggested ordering (encoded in bead deps)

DBL.1→DBL.4 first (largest measured serial win; fixes FU-C's baseline).
ZCS.1→ZCS.3 any time (disjoint files from DBL). FUC.0 only after H6 **and**
DBL.4; FUC.1/FUC.2 after the Job B ruling.

## Non-goals (all three)

- Detect algorithm/pixel-math changes; accuracy/matching behavior changes of
  any kind (hale_bopp default-params `SolveFromImage` NoMatch is feat-10/H2;
  tree.jpg stress false-positive is accuracy-domain, separate).
- `lookup_pattern_mmap`/`probe_slots` refactor (comment only; H10 is the
  dedup vehicle).
- Loosening any tolerance, `#[ignore]`-ing any test, weakening any gate.
- Python-client / harness-side overhead.
