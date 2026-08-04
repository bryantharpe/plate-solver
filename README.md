# plate-solver

A fast Rust implementation of the **tetra3 / cedar-solve "lost-in-space"
plate-solving algorithm**: give it a night-sky image and a rough field-of-view
guess, and it tells you where the camera was pointing — right ascension,
declination, roll, refined FOV, and the matched catalog stars — with no other
prior knowledge.

On the reference corpus it produces the **same solutions as tetra3 and
cedar-solve (boresight agreement within arcseconds) at about half the wall
time**, full image → solution in ~8 ms on a desktop CPU against a
million-pattern database. See [the benchmark](docs/benchmarks/RESULTS.md) for
the tables and [methodology](docs/benchmarks/README.md) to reproduce them.

## Where it came from

- [**tetra3**](https://github.com/esa/tetra3) (ESA, Apache-2.0) is the
  original Python implementation: geometric hashing of 4-star patterns —
  five rotation/scale-invariant edge ratios quantized into a key, hashed
  into a precomputed sky database — followed by attitude solving (Wahba/SVD)
  and a binomial false-alarm test.
- [**cedar-solve**](https://github.com/smroid/cedar-solve) and
  [**cedar-detect**](https://github.com/smroid/cedar-detect)
  (Steven Rosenthal) evolved tetra3 for the Cedar star-tracker: a faster
  star detector, database layout extensions (largest-edge and 16-bit hash
  pre-filters), and a gRPC service surface.
- **This repo** is a from-scratch Rust implementation of that algorithm
  family, written against [written specifications](openspec/) with the
  Python implementations used only as test oracles. It reads cedar-format
  `.npz` pattern databases directly, matches cedar-detect's centroids to
  0.1 px on upstream's own test images (`crates/star-detection/tests/parity.rs`),
  and reproduces the reference solutions end-to-end
  (`crates/ps-web/tests/solve_integration.rs`).

Nothing from the upstream projects is vendored or redistributed here;
`scripts/fetch-references.sh` clones them at pinned commits into a
gitignored `references/` directory for parity tests and benchmarks.

## How it compares

8-frame calibrated corpus, each solver running its own shipped end-to-end
path (star detection + solve, stock defaults, database preloaded — median of
5 warm runs, AMD Ryzen 9 9950X3D):

| solver | solved | median wall time | agreement with tetra3 (worst case) |
|---|---|---|---|
| **plate-solver (this repo)** | 8/8 | **8.4 ms** | 18.7″ boresight |
| tetra3 (Python) | 8/8 | 16.2 ms | — (baseline) |
| cedar-solve (Python) | 8/8 | 15.9 ms | 13.6″ boresight |

Full tables, per-image data, and known gaps: [docs/benchmarks/RESULTS.md](docs/benchmarks/RESULTS.md).

## Try it in the browser

```bash
scripts/fetch-references.sh    # pinned upstream checkouts (database + test images)
cargo run --release -p ps-web -- \
    --db references/cedar-solve/tetra3/data/default_database.npz
```

Then open <http://127.0.0.1:8080>: drag in a star-field photo, set the FOV
estimate, and get the solution with a matched-star overlay and an Aladin sky
view. `POST /api/solve` (multipart: `image`, `fov_estimate`, optional
`timeout_ms`/`match_radius`/`match_threshold`/`fov_max_error`/`distortion`)
returns the same result as JSON — see [crates/ps-web](crates/ps-web/README.md).

Or from the command line:

```bash
cargo run --release -p plate-solver --example solve_image -- \
    references/cedar-solve/tetra3/data/default_database.npz \
    references/cedar-solve/examples/data/medium_fov/2019-07-29T204726_Alt40_Azi-135_Try1.jpg \
    11
```

## Crates

| Crate | What it owns |
|---|---|
| [`math-core`](crates/math-core) | Attitude (Wahba/SVD), pattern keys and hashing, pinhole camera, FOV refinement, residuals |
| [`star-detection`](crates/star-detection) | Noise estimation, binning, centroiding (cedar-detect-compatible, parity-tested) |
| [`pattern-database`](crates/pattern-database) | `.npz` database loader (eager or mmap), KD-tree, key→candidates lookup |
| [`database-generation`](crates/database-generation) | `tetra3-gen-db` CLI: catalog parsing (BSC5/HIP/TYC), proper motion, pattern enumeration, serialization |
| [`plate-solver`](crates/plate-solver) | The solve loop: preparation, candidate generation, verification, refinement |
| [`grpc-service`](crates/grpc-service) | tonic service surface for [`proto/plate_solver.proto`](proto/plate_solver.proto) |
| [`ps-web`](crates/ps-web) | Web test harness: axum server + embedded React UI |

Design documents live in [`openspec/`](openspec/) (capability specs and PRD)
and [`docs/algorithms/`](docs/algorithms/) (algorithm write-ups, including a
[tetra3 vs cedar comparison](docs/algorithms/08-tetra3-vs-cedar-comparison.md)).

## Building and testing

```bash
cargo build --release        # needs protobuf-compiler for grpc-service
cargo test --workspace       # self-contained; reference-based tests skip if references/ absent
scripts/fetch-references.sh  # enable the cedar-detect parity + reference solve tests
cargo test --workspace       # now includes them
```

Generating a pattern database from a star catalog:

```bash
cargo run --release -p database-generation -- --help   # tetra3-gen-db
```

## Status

Working today: solve-from-image and solve-from-centroids as a library, the
web harness, database generation, and the benchmark suite. Not finished:
a runnable gRPC server binary (the tonic service surface exists as a
library), distortion estimation polish (the refinement-stage RMSE gap noted
in the benchmark), and mobile (UniFFI) bindings.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

This project reimplements, and is tested against, algorithms from
[tetra3](https://github.com/esa/tetra3) (ESA, Apache-2.0),
[cedar-solve](https://github.com/smroid/cedar-solve) (Steven Rosenthal,
Apache-2.0), and [cedar-detect](https://github.com/smroid/cedar-detect)
(Steven Rosenthal, FSL-1.1 — used only as a local test oracle, never
redistributed). Reference test imagery is credited in the upstream
repositories.
