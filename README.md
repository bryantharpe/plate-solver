# plate-solver

A Rust implementation of the tetra3 / cedar-solve "lost-in-space" star-field plate solver,
with a port of the cedar-detect star detector. Give it a photo of the night sky and it
returns where the camera was pointing (RA, Dec, roll) and its field of view.

It is built from the specifications in [`openspec/`](openspec/) and held to parity with the
reference implementations vendored in [`reference-solutions/`](reference-solutions/) by CI.
The original v1 implementation is preserved at tag
[`v1-original`](../../releases/tag/v1-original).

## Usage

Everything runs through `cargo` from the repository root. Nothing is installed, and there is
no gRPC server binary: `grpc-service` is a library.

### Solve an image

```sh
cargo run --release -p plate-solver --example solve_image -- \
    --db reference-solutions/cedar-solve/tetra3/data/default_database.npz \
    reference-solutions/cedar-solve/examples/data/medium_fov/orion_belt.jpg
```

```text
reference-solutions/cedar-solve/examples/data/medium_fov/orion_belt.jpg: MATCH_FOUND  RA 84.0093  Dec -0.7694  Roll 2.56  FOV 10.15 deg  k -0.003  matches 21  RMSE 30"  (22 stars, detect 0 ms, solve 8 ms)
  sky map: https://aladin.cds.unistra.fr/AladinLite/?target=84.0093%20-0.7694&fov=12
```

RA, Dec, Roll and FOV are in degrees. `k` is the fitted radial distortion (negative is barrel).
RMSE is the residual over the matched stars, in arcseconds. Pass several images to solve them in
one run; the database loads once. PNG, JPEG, BMP and PNM files are read.

The database must cover your camera's field of view. The cedar database used above covers
10–30°; for other lenses, build one (below).

| Flag | Default | Meaning |
|------|---------|---------|
| `--db PATH` | required | Pattern database (`.npz`): one built below, or a cedar/tetra3 database. |
| `--fov DEG` | none | Horizontal FOV estimate. Leave it off for the first solve: a wrong hint with a tight `--fov-max-error` prevents solving. |
| `--fov-max-error DEG` | 10 | How far the true FOV may be from `--fov`. |
| `--distortion K` | 0 | Radial distortion `k` from an earlier solve with the same lens. Speeds up and steadies wide-field solves. |
| `--bin N` | 2 | Bin the image by 1, 2, 4 or 8 before detection. |
| `--sigma S` | 8 | Detection significance threshold. |
| `--stars N` | 30 | Number of brightest detected stars passed to the solver. |
| `--dark PATH` | none | Dark frame (lens capped, same exposure and gain) subtracted before detection. Removes hot pixels. |
| `--timeout MS` | 5000 | Solve timeout in milliseconds. |
| `--json` | off | Print one JSON object per image instead of text. |

### Build a pattern database

```sh
cargo run --release -p database-generation -- hip_main.dat hip_30_60.npz --min-fov 30 --max-fov 60
```

Set `--min-fov` and `--max-fov` (degrees) to bracket your camera's horizontal FOV. The catalog is
the Hipparcos main catalog, `hip_main.dat` (53 MB), from CDS:
<https://cdsarc.cds.unistra.fr/ftp/I/239/hip_main.dat>. `tyc_main.dat` and BSC5 catalogs also
work; the type is detected from the file name (`hip`, `tyc` or `bsc5`). `--help` lists the other
options: magnitude limit, proper-motion epoch and pattern density.

To try it without downloading anything, build from the BSC5 catalog vendored for cedar's tests
(about a minute, 32 MB) and solve with the result:

```sh
cargo run --release -p database-generation -- reference-solutions/cedar-solve/tests/data/bsc5 bsc5_10_30.npz --min-fov 10 --max-fov 30
```

## Repository layout

| Path | What it is |
|------|------------|
| `crates/star-detection` | Star detection: noise estimation, binning, sub-pixel centroiding and brightness. |
| `crates/math-core` | Shared numerics: unit vectors, the pinhole camera model, attitude (Wahba/SVD), FOV refinement. |
| `crates/pattern-database` | The pattern database: `.npz` loader, KD-tree and pattern lookup. |
| `crates/database-generation` | Builds pattern databases from star catalogs (Hipparcos, Tycho, BSC5). |
| `crates/plate-solver` | The solver (`solve_from_centroids`, `solve_from_image`) and the `solve_image` example. |
| `crates/grpc-service` | The `PlateSolver` gRPC service, as a library. |
| `openspec/` | The specifications: capability specs, PRD and project conventions. Start at `openspec/project.md` and `openspec/PRD.md`. |
| `proto/` | The gRPC interface contracts (`plate_solver.proto`, `cedar_detect.proto`). |
| `reference-solutions/` | **The oracle**: vendored cedar-solve, tetra3 and cedar-detect, with their sample images and databases. |
| `scripts/ci/`, `.github/workflows/ci.yml` | CI, including the end-to-end parity gate. |
| `docs/` | Audits, e.g. `docs/audit-e2e-parity.md`. |

## Correctness

"Correct" means agreeing with cedar. CI's `e2e-eval` job solves the 21 sample images in
`reference-solutions/cedar-solve/examples/data/medium_fov/` from centroids (E1), from pixels
(E2) and over gRPC (E3), and compares every answer with cedar's; `e2e-canary` proves that gate
can still fail. [`ENVIRONMENT.md`](ENVIRONMENT.md) lists what CI enforces on `main` and which
files define "correct".

## Development

Building `grpc-service` needs `protoc` (Debian/Ubuntu: `protobuf-compiler`; if it can't find
`google/protobuf/duration.proto`, also install `libprotobuf-dev`). CI runs, among other checks:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
```
