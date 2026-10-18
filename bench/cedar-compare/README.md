# cedar-compare

Times this repo's Rust pipeline (`star-detection` + `plate-solver`) against the code it
re-implements, cedar-detect + cedar-solve, and checks that the two agree. Both sides use
the 21 images in `reference-solutions/cedar-solve/examples/data/medium_fov/` and cedar's
own example settings (`examples/test_tetra3.py`): sigma 8, 2x2 binning, `normalize_rows`,
hot-pixel rejection, the brightest 30 stars, no FOV hint, distortion 0, a 5 s timeout and
cedar's `default_database.npz`.

The harness is not part of the Cargo workspace (it is in the root `exclude` list) and does
not run in CI.

## Prerequisites

- Linux. WSL2 works. cedar-detect-server receives frames from its Python client through
  `/dev/shm`.
- A Rust toolchain that builds the workspace (MSRV 1.96).
- `protoc` and the protobuf well-known-type includes, for cedar-detect's `build.rs`. On
  Debian/Ubuntu: `sudo apt install protobuf-compiler libprotobuf-dev`. If the includes are
  somewhere else, point `PROTOC_INCLUDE` at them.
- Python 3.12 as `python3.12` (or set `PYTHON`), with `venv`. On Debian/Ubuntu that is
  `python3.12-venv`. The first run needs network access to install `requirements.txt`.

## Run

```bash
bench/cedar-compare/run.sh
```

The script:

1. Builds this harness and `cedar-detect-server` in release mode with `--locked`. The
   server is built as a dependency of this package, so nothing is written into
   `reference-solutions/`.
2. Creates a venv and installs the pinned `requirements.txt`.
3. Runs three passes over every image. Each pass times `BENCH_REPS` repetitions per image
   and keeps the median:
   - **cedar** (`cedar_bench.py`): PIL decodes the JPEG to gray, cedar-detect-server
     detects stars over gRPC, and cedar-solve's `solve_from_centroids` solves. This pass
     also saves PIL's gray frames as PGM.
   - **rust A** (`src/main.rs` on the JPEGs): the `image` crate decodes, as the product does.
   - **rust B** (`src/main.rs` on PIL's PGM frames): the same pixels cedar saw. This is the
     main comparison.

   Each Rust pass times, on the same frame, `star_detection::detect_stars`, cedar-detect
   called in-process, `solve_from_centroids` on our top 30, and `solve_from_image`.
4. Writes `bench-data.json` with `summarize.py` and prints the headline numbers plus one
   line per image.

Pass image names (e.g. `run.sh virgo orion2`) to run a subset. Optional environment:

| Variable | Default | |
|---|---|---|
| `CARGO_TARGET_DIR` | `bench/cedar-compare/target` | build output |
| `BENCH_VENV` | `bench/cedar-compare/.venv` | Python venv |
| `BENCH_OUT` | `bench/cedar-compare/out` | raw output and summary |
| `BENCH_REPS` | `7` | timed repetitions per image |
| `PYTHON` | `python3.12` | interpreter used to create the venv |
| `BENCH_GRPC_PORT` | `50061` | cedar-detect-server port |
| `BENCH_COMMIT` | `git rev-parse --short HEAD` | commit recorded in the summary when git can't tell |

On WSL with the checkout under `/mnt/c`, put the first three under `~/.cache` (ext4).
Building or importing Python packages over `/mnt/c` is slow and adds noise to the timings.
Git for Windows worktrees can't be read by WSL's git, so pass `BENCH_COMMIT` there.

`BENCH_OUT` ends up holding `cedar.jsonl`, `rust.jsonl` (A) and `rust_pgm.jsonl` (B), one
JSON object per image; their `.err` logs; `gray/*.pgm`; `meta.json` (commit, date,
machine, tool versions); and `bench-data.json`.

## Reading the numbers

All times are milliseconds. Each per-image time is the median of the repetitions. The
summary takes the median across the images cedar solved.

- `detection_identical`: images where our star count equals cedar-detect's and each of our
  top-30 centroids is within 0.1 px of one of cedar-detect's top 30.
- `solved_rust` / `solved_cedar` / `n`: images that came back `MATCH_FOUND` (Rust from
  pass B), out of `n`.
- `stage.detect`:
  - `rust`: `detect_stars`.
  - `cedar_inproc`: cedar-detect's algorithm called directly from Rust, the fair
    algorithm-to-algorithm comparison.
  - `cedar_grpc`: what a cedar-solve user pays, including the Python → gRPC →
    shared-memory round trip.
  - `ratio_*`: cedar ÷ Rust, given both as the median of the per-image ratios and as the
    ratio of totals.
- `stage.solve`: `solve_from_centroids` on the top 30, ours against tetra3's.
  `rust_max` / `cedar_max` are the slowest image's times.
- `stage.e2e`: detect + solve, Rust (in-process) against cedar (gRPC detect + Python
  solve). Decoding is not included. `*_total` sums all solved images.
- `stage.decode`: JPEG → gray, the `image` crate against PIL.
- `agree_same_pixels` (pass B), `agree_as_shipped` (pass A) and `agree_solve_from_image`
  (pass A, `solve_from_image`): over images both sides solved:
  - `pos`: the angle between the two pointing solutions.
  - `roll`, `fov`: the differences in roll and field of view.
  - All three are in arcseconds.
  - `same_stars`: how many images matched exactly the same HIP stars.

  Pass A differs from cedar only because the `image` crate and PIL decode JPEGs slightly
  differently.
- `setup`: one-time costs. Rust database load (both passes), Python import of
  numpy/PIL/tetra3, tetra3's database load, and the first gRPC call, which includes
  starting the server.

## Results are machine-specific

Timings depend on the CPU, OS, power state, background load and file system, and they move
by tens of percent between runs on one machine. Compare ratios from a single run on one
machine, not absolute times across machines. Detection, solve status and the agreement
figures did not change between runs here.

Reference run, 2026-10-18, commit 9882288: Qualcomm Oryon (12 threads), WSL2 Ubuntu arm64,
Rust 1.99.1, Python 3.12.15, 7 repetitions. Four more runs of `run.sh` that day gave
identical agreement figures; the timing ranges below cover all five runs.

| | Rust | cedar |
|---|---|---|
| detection identical | 21 / 21 | |
| solved | 20 / 21 | 20 / 21 (`roof` fails on both) |
| detect, median | 2.0–2.4 ms | 2.1–2.5 ms in-process, 3.7–4.0 ms over gRPC |
| solve, median | 0.027–0.039 ms | 1.16–1.47 ms |
| detect + solve, median | 2.0–2.4 ms | 5.2–5.3 ms |
| same pixels: max position difference | 0.0005″ (20 / 20 matched the same stars) | |
