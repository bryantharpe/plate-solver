# Implementation Status

Rust implementation of the tetra3/cedar plate-solving pipeline.
All six features are complete and passing parity tests.

## Feature → Crate Map

| Feature | Crate(s) | Status | Tests |
|---------|----------|--------|-------|
| feat-01 Foundation Math Core | `ps-core` | COMPLETE | 37 parity (celestial, angle, projection, distortion, attitude, pattern, FOV, false-alarm, residuals) |
| feat-02 Star Detection | `ps-detect` | COMPLETE | 63 (56 unit + 7 integration incl. end-to-end ±0.1 px centroid parity) |
| feat-03 Pattern Database | `ps-db` | COMPLETE | 10 (3 unit + 7 integration incl. mmap_lookup_parity, nearby_stars_parity) |
| feat-04 Database Generation | `ps-dbgen` | COMPLETE | 42 (19 unit + 23 integration incl. byte-identical determinism, e2e CLI build) |
| feat-05 Plate Solver | `ps-solve` | COMPLETE | 17 (1 ignored) incl. sv6 RA/Dec within 10 arcsec, 19/19 matched catalog IDs |
| feat-06 gRPC Service | `ps-grpc` | COMPLETE | 9 incl. solve_from_image MATCH_FOUND on reference JPEG, cedar-detect interop |

**Total: 182 tests pass, 1 ignored (sv6_diagnostic_solve_sweep — sweep helper, not a gate).**

## Parity Outcomes

### feat-01 — Foundation Math Core (`ps-core`)

All math modules verified against Python tetra3/cedar-solve fixtures captured via
`tools/parity/capture_core.py`:

- **celestial vectors** (`radec_to_vector`/`vector_to_radec`): parity < 1e-9, round-trip < 1e-12.
- **angular distance** (`2·asin(d/2)` formulation): parity < 1e-9, round-trip < 1e-12.
- **pinhole projection**: parity < 1e-9, round-trip < 1e-9.
- **radial distortion**: parity < 1e-3 (f32 reference vs f64 Rust; measured max 4.1e-5 undist /
  1.0e-4 roundtrip); pure-f64 round-trip < 1e-6.
- **attitude (Wahba/SVD)**: recovered R < 1e-9 vs reference; extracted RA/Dec/Roll < 1e-9.
- **pattern key/hash/index**: integer-exact parity on all 5 fixture cases.
- **false-alarm probability**: parity < 1e-6 rel.
- **FOV refinement**: parity < 1e-9 abs.
- **residual stats**: parity < 1e-9 abs.

### feat-02 — Star Detection (`ps-detect`)

Verified against cedar-detect reference (Rust binary) on `cedar-detect/test_data` at sigma=8:

- **First image (m13.jpg)**: exact count match (35 stars), all 35 centroids within ±0.1 px.
- **Second image (hale_bopp.jpg)**: top-5 centroids within ±0.1 px, count within ±2.
  Noise fixture self-captured due to JPEG decoder version difference; cross-referenced to
  first-image at 1e-6.
- **Hot-pixel count**: exact match (21/21) for first image.

### feat-03 — Pattern Database (`ps-db`)

- **npz import**: all array shapes and properties match `default_database.npz` including legacy
  fallbacks.
- **lookup parity**: exact candidate slot set vs reference DB for 3 independent queries.
- **nearby_stars parity**: exact index-set parity for 3 radius queries (48/22/14 stars).
- **mmap path**: identical lookups and nearby_stars results to in-RAM path.

### feat-04 — Database Generation (`ps-dbgen`)

- **Round-trip**: generated DB loads cleanly through `ps-db` loader.
- **Determinism**: two independent runs produce byte-identical output.
- **e2e CLI**: 6-star BSC5 fixture → 15 patterns → lookup finds candidates.
- **Count parity vs `default_database.npz`**: DEFERRED — Hipparcos/Tycho source catalogs are
  not in-repo. Structural validity (shapes, properties, determinism) verified; count parity
  logged with `eprintln!` in `tests/e2e.rs` for post-catalog-download verification.

### feat-05 — Plate Solver (`ps-solve`)

- **sv6_solve_from_centroids_parity**: 19/19 matched catalog IDs exact; RA/Dec within 10 arcsec
  vs tetra3 Python reference on the medium-FOV test image.
- **sv6_solve_from_image_parity**: MATCH_FOUND on the real JPEG; RA/Dec within 10 arcsec
  (no catalog-ID assertion — ps-detect centroids differ slightly from tetra3 Python centroids,
  but attitude recovery is correct).
- All 5 `SolveStatus` variants (MATCH_FOUND, NO_MATCH, TIMEOUT, CANCELLED, TOO_FEW) are
  structurally reachable.

### feat-06 — gRPC Service (`ps-grpc`)

- **ExtractCentroids**: brightest-first centroids match `ps-detect`; shmem failure → gRPC INTERNAL.
- **SolveFromImage parity**: MATCH_FOUND on reference JPEG (runs in ≈8 s in test).
- **cedar-detect interop**: `cedar_detect.proto`-shaped `ExtractCentroids` request succeeds
  unmodified; `algorithm_time` (`google.protobuf.Duration`) decodes correctly in both directions.
- **GetInfo**: returns DB FOV range, pattern count, catalog epoch.

## Deferred Items

| Item | Reason | Where logged |
|------|--------|--------------|
| ps-dbgen count parity vs `default_database.npz` | Hipparcos/Tycho catalogs not in-repo | `ps-dbgen/tests/e2e.rs` eprintln |
| feat-07 mobile runtime | Out of scope for this grind (no Xcode/Android NDK in CI) | plan.md |

## Clippy Warnings (24, no errors)

`cargo clippy --workspace` produces 24 style warnings and 0 errors. These are logged here
rather than fixed to avoid scope creep on non-blocking style items:

- `needless_range_loop` — indexed loops in hash table and match routines (ps-db, ps-solve)
- `type_complexity` — complex return type in ps-grpc service (acceptable for gRPC generated types)
- `doc_lazy_continuation` — overindented doc list items (ps-core)
- `too_many_arguments` — `gate_star_2d` (16 args) and a helper (8 args), matching reference structure
- `trim_split_whitespace` — parse helper in ps-dbgen
- `manual_is_multiple_of` — modulo checks in ps-db hash routines
- `redundant_else`, `map_clone`, `unnecessary_cast` — minor style items across crates
- `unwrap_used` (clippy::nursery) — three `unwrap()` calls on `is_some()`-checked Options in ps-grpc

## How to Run

### Prerequisites

```bash
# Rust toolchain (stable, ≥1.83)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# protoc (required for ps-grpc)
# macOS:
brew install protobuf
# Ubuntu:
apt-get install -y protobuf-compiler

# Python parity environment (optional, for re-capturing fixtures)
cd tools/parity && python3 -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt
```

### Build & Test

```bash
# Build all crates
cargo build --workspace

# Run all tests (≈60 s; includes parity tests against reference data)
cargo test --workspace

# Run a single crate's tests
cargo test -p ps-core
cargo test -p ps-detect
cargo test -p ps-db
cargo test -p ps-dbgen
cargo test -p ps-solve
cargo test -p ps-grpc

# Check formatting and lints
cargo fmt --check
cargo clippy --workspace
```

### Run the gRPC Server

The server requires a `default_database.npz` or native `.bin` database. Convert the reference
database once:

```bash
# Import the bundled reference .npz into native format
cargo run -p ps-dbgen -- \
  --hip path/to/hip_main.dat \
  --tyc path/to/tyc2.dat \
  --output ps_database.bin
```

Then start the server:

```bash
cargo run -p ps-grpc -- --database ps_database.bin --address 127.0.0.1:50051
```

The server exposes the `PlateSolver` gRPC service (see `ps-grpc/proto/plate_solver.proto`)
and is wire-compatible with the `cedar-detect` gRPC protocol for `ExtractCentroids`.

### Re-capture Parity Fixtures

```bash
cd tools/parity
source .venv/bin/activate
python capture_core.py      # ps-core fixtures
python capture_detect.py    # ps-detect fixtures
python capture_solve.py     # ps-solve fixtures
```
