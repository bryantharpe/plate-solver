# Benchmark: plate-solver vs tetra3 vs cedar-solve

Head-to-head accuracy and speed comparison of this repo's Rust solver against
the two Python reference implementations it derives from.

## Method

- **Corpus**: the 8 calibrated night-sky frames
  `2019-07-29T204726_Alt{40,60}_Azi{±45,±135}_Try1.jpg` (1024×768, ~11°
  horizontal FOV) shipped with cedar-solve (`examples/data/medium_fov/`,
  photos by the tetra3/cedar authors — see `credits.txt` there).
- **Solvers**, each running its own shipped code path end-to-end (star
  detection + solve) with `fov_estimate=11`, `solve_timeout=10000`, and its
  stock defaults otherwise:
  - `plate-solver-rs` — this repo, release build
    (`crates/plate-solver/examples/bench_corpus.rs`), cedar-solve's
    `default_database.npz` (HIP, 1,010,981 patterns), cedar-detect-style
    detection (σ=8).
  - `tetra3` — [esa/tetra3](https://github.com/esa/tetra3) @ `f9fa2eb`,
    Python 3.13, its own `default_database.npz` and
    `get_centroids_from_image` (σ=2 default).
  - `cedar-solve` — [smroid/cedar-solve](https://github.com/smroid/cedar-solve)
    @ `1a8a1d7`, Python 3.13, its own `default_database.npz` and Python
    centroider.
- **Timing**: per image, 1 warmup call then 5 timed calls; the wall-clock of
  each full `solve_from_image` call (detection + solve, database preloaded).
  Medians reported.
- **Accuracy**: boresight (RA/Dec) angular separation and |ΔFOV| against
  tetra3's solution for the same image, plus each solver's own reported RMSE
  over its matched stars.
- **Machine**: AMD Ryzen 9 9950X3D (16C/32T), 64 GB RAM, Linux 6.17,
  rustc 1.97.1, Python 3.13.11. Single-threaded solves.

Each solver uses its own stock pattern database, so this is an "as-shipped"
comparison, not an isolated algorithm comparison. (The Rust solver reads the
cedar-format `.npz` databases; tetra3's own database omits the
largest-edge/hash arrays of the cedar format.)

## Reproduce

```bash
scripts/fetch-references.sh
python3 -m venv .venv && .venv/bin/pip install numpy pillow scipy

cargo run --release -p plate-solver --example bench_corpus -- \
    references/cedar-solve/tetra3/data/default_database.npz \
    references/cedar-solve/examples/data/medium_fov 11 \
    > docs/benchmarks/results-plate-solver-rs.json

.venv/bin/python tools/benchmark/run_python.py references/tetra3 tetra3 \
    > docs/benchmarks/results-tetra3.json
.venv/bin/python tools/benchmark/run_python.py references/cedar-solve cedar-solve \
    > docs/benchmarks/results-cedar-solve.json

python3 tools/benchmark/report.py docs/benchmarks/results-*.json
```

Results tables: [RESULTS.md](RESULTS.md). Raw per-run data: the
`results-*.json` files in this directory.
