## Context

The three systems being compared are architecturally different in ways that directly shape this
design, each verified by hand against the actual files rather than assumed:

- **Incompatible catalogs.** `reference-solutions/tetra3/tetra3/data/default_database.npz` (49MB;
  `props_packed` has no `hash_table_type` field; magnitude 7.0) and
  `reference-solutions/cedar-solve/tetra3/data/default_database.npz` (14MB;
  `hash_table_type='linear_probe'`; magnitude 8.0; 1,010,981 patterns) are different builds —
  different md5, different `props_packed` schema, confirmed by reading both structured arrays
  directly. Original `tetra3.py`'s `_insert_at_index`/`_get_table_index_from_hash` (lines
  ~113–137) hard-code quadratic probing with no `linear_probe` parameter at all, so it cannot
  correctly read cedar-solve's linear-probe catalog. tetra3-original must therefore use its own
  bundled catalog; cedar-flow and the new workflow share cedar-solve's catalog, converted to
  `ps-db`'s native format via `ps_db::importer::import_npz` → `ps_db::loader::save_native` — the
  same two calls `ps-grpc/src/service.rs` (~lines 620–635) already uses to build a test database.
- **A Python package-name collision.** `reference-solutions/tetra3/setup.py` (`find_packages()`)
  and `reference-solutions/cedar-solve/pyproject.toml`
  (`[tool.setuptools.packages.find]` → `include = ["tetra3*"]`) both install a top-level package
  named `tetra3`. One venv cannot `import tetra3` to reach both; one shadows the other.
- **Two already-known, already-tracked limitations in the new workflow** (`CODEBASE-REVIEW.md`
  `C1`/`C2`), relevant to how the harness measures it, not something this change fixes:
  `ps_solve::solve_from_image` (`ps-solve/src/lib.rs:610-624`) hard-codes `sigma=4.0,
  noise_estimate=1.0, binning=1` regardless of request parameters, and `ps-grpc`'s
  `Solution.t_extract_ms` is hard-coded `0.0` for both `SolveFromCentroids` and `SolveFromImage`
  (`ps-grpc/src/service.rs` ~lines 275, 336, the latter with an explicit code comment explaining
  why). A true self-reported extraction time for the new workflow can only come from a standalone
  `ExtractCentroids` call.
- **A shared image corpus already exists**, no downloads needed: 11 files are byte-identical
  (verified by md5sum) between `reference-solutions/cedar-detect/test_data/` and
  `reference-solutions/cedar-solve/examples/data/medium_fov/` — the 8
  `2019-07-29T204726_AltXX_AziYYY_Try1.jpg` real-sky photos, `hale_bopp.jpg` (9 astronomical
  images, a valid solve expected), plus `tree.jpg` and `test_5mp_g100_e50ms.jpg` (2
  non-astronomical images, no valid solve expected — useful as detection stress tests).
  `2019-07-29T204726_Alt40_Azi-135_Try1.jpg` is also `tools/parity/capture_solve.py`'s existing
  reference image.
- **Both wall-clock and algorithm-only timing are already exposed on the wire**: the
  `algorithm_time` field on `CentroidsResult` (both `cedar_detect.proto` and
  `plate_solver.proto`, self-reported via `Instant::now()`/`elapsed()` server-side in both
  `cedar_detect_server.rs` and `ps-grpc/src/service.rs`), and tetra3/cedar-solve's own solve-dict
  timing keys. This satisfies the instruction to measure at the gRPC layer directly for the two
  systems that expose gRPC servers, without writing new instrumentation.
- **This host is Linux/aarch64** (not the PRD's RPi-4B/mobile target hardware), and its existing
  `tools/parity/.venv` needs `libjpeg-dev`/`zlib1g-dev` installed before it will build here —
  `Pillow<9` (cedar-solve's own declared upper bound) ships no linux/aarch64 wheel below 9.2.0, so
  it must compile from source, which fails without those headers. This is a one-time,
  human-run environment step (`tasks.md`), not a spec-level concern, but the eventual report must
  disclose the host it ran on rather than imply mobile-representative numbers.

## Goals / Non-Goals

**Goals:**
- Compare all three systems on one shared, checked-in image corpus.
- Capture both client-observed wall-clock latency and each system's self-reported
  algorithm-only latency, for the detect stage and the full solve pipeline.
- Cross-check solve output (RA/Dec/Roll/FOV/matched stars/status) for parity within tolerances
  already established in this repo (`IMPLEMENTATION-STATUS.md`), clearly distinguishing the
  same-catalog comparison (`ps_grpc` vs `cedar_flow`) from cross-catalog comparisons involving
  `tetra3_original`.
- Produce both a machine-readable (`results.json`) and two human-readable (`report.md`,
  `report.html`) outputs from one run.
- Be re-runnable on demand via one documented command, so it can be re-executed after future
  changes to `ps-detect`/`ps-solve`/`ps-grpc` to see whether performance improved or regressed.

**Non-Goals:**
- Fixing `C1` (solver-hot-loop allocation) or `C2` (`solve_from_image` ignoring request params) —
  this change discloses them as measurement caveats, it does not resolve them.
- Measuring on RPi-4B-class or mobile hardware (the PRD's actual target) — out of reach in this
  environment; disclosed as a limitation instead of implied.
- A generic, pluggable benchmarking framework for arbitrary future comparisons — this harness is
  scoped to exactly this three-way comparison.
- Automatic cross-run regression detection/diffing. "Judging performance over time" is satisfied
  by committing each run's `report.md`/`report.html` and comparing them via normal git history —
  building a dedicated diffing/trend subsystem is not justified by anything asked for yet.

## Decisions

- **tetra3-original runs as a subprocess under its own venv**, not an in-process import, because
  it cannot share a Python process with cedar-solve (package-name collision, above). The two Rust
  gRPC servers are also long-lived subprocesses; tetra3-original's runner batches N iterations
  per invocation and returns one JSON blob, keeping interpreter/import startup out of the timed
  region, consistent with how the Rust servers aren't re-spawned per call either.
- **tetra3-original uses its own bundled catalog**; there is no reconciliation available (its
  hash-table code cannot read cedar-solve's linear-probe format, and the source HIP/TYC catalogs
  needed to rebuild a matching one aren't in-repo per `feat-04-database-generation`'s deferred
  count-parity item). Its results are therefore reported as a **cross-catalog** comparison,
  labeled as such everywhere they appear, never silently presented as strict parity.
- **gRPC is the measurement interface for cedar-detect and ps-grpc**, called directly rather than
  through any additional client wrapper, per the explicit instruction that library performance is
  the target and gRPC is an acceptable, already-fair interface for the two systems that expose it.
- **Explicit, shared detection parameters** (`sigma=4.0`, `detect_hot_pixels=true`,
  `normalize_rows=false`, no binning) are passed to every system rather than left at
  divergent defaults — `sigma=4.0` specifically matches the value the new workflow's
  `solve_from_image` is hard-coded to (`C2`), so using anything else there would manufacture a
  false discrepancy rather than reveal a real one.
- **Both timing numbers are captured wherever available**: wall-clock around the call, and each
  system's self-reported algorithm-only time. For the new workflow's extraction stage this means
  a standalone `ExtractCentroids` call, since `Solution.t_extract_ms` is always `0.0` there.
- **The two non-astronomical images run the full detect+solve pipeline**, not detect-only,
  bounded to 1 iteration and a 5-second solve timeout — deliberately chosen over both "exclude
  them" and "detect-only" so the corpus still stress-tests the full pipeline's no-match path,
  while bounding the cost of the new solver's known combinatorial blow-up on non-matching input.
- **Harness code lives under `tools/parity/benchmark/`**, reusing the existing `tools/parity/`
  venv and README conventions rather than introducing a new top-level directory; rendered reports
  go to `docs/benchmarks/`, alongside the existing `docs/screenshots/`.

## Risks / Trade-offs

- [Cross-catalog comparison] → `tetra3_original`'s RA/Dec/matched-star numbers are not directly
  comparable to the other two systems' at the same strictness as `ps_grpc` vs `cedar_flow`;
  mitigated by labeling every cross-catalog result explicitly rather than folding it into the
  primary parity table.
- [`C1` combinatorial blow-up on non-matching input] → the new workflow's solver can allocate
  heavily searching for a match that doesn't exist; mitigated by the 1-iteration/5-second bound
  on the two stress images.
- [Dev-host not mobile-representative] → results measured on a Linux/aarch64 desktop-class
  sandbox, not the PRD's RPi-4B/mobile target; the report states the host explicitly rather than
  implying the numbers generalize to mobile.
- [Environment setup friction] → the existing `tools/parity/.venv` needs OS packages
  (`libjpeg-dev`, `zlib1g-dev`) this host doesn't have and no passwordless `sudo`; a one-time
  human-run install step, tracked in `tasks.md`, not a recurring cost once done.

## Migration Plan

Greenfield — no existing behavior changes. The first run establishes a baseline recorded in
`docs/benchmarks/report.md`; subsequent runs are triggered manually (no CI in this repo, per
`STATUS.md`) after changes to `ps-detect`, `ps-solve`, or `ps-grpc`, and their reports are
committed alongside the change that motivated re-running them.

## Open Questions

None blocking. Exact iteration/warmup counts and the harness-proposed Roll/FOV tolerances (no
prior end-to-end tolerance exists for those two fields in this repo — only for pure attitude-math
given an already-known rotation matrix) are implementation-time tuning parameters, exposed as CLI
flags rather than fixed in this spec.
