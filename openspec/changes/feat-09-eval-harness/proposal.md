## Why

Six capabilities (`feat-01`–`feat-06`) plus the retroactive `feat-08-web-ui` are implemented and
correctness-parity-tested against the Python/Rust reference implementations
(`openspec/IMPLEMENTATION-STATUS.md`), but nothing in this workspace measures *performance*. A
sweep of `mayor/rig`, `polecats/furiosa/plate_solver`, `crew`, `witness`,
`openspec/changes/*`, `notes/*.md`, `CODEBASE-REVIEW.md`, and a grep for
`criterion`/`[[bench]]` across every `Cargo.toml`/`*.rs` in the workspace turned up nothing —
there is no existing benchmark or perf-comparison tooling to build on or duplicate.

The whole point of porting tetra3/cedar to Rust was speed and predictable latency
(`openspec/PRD.md`'s "Problem"/"Goals" sections). That claim has never been measured end-to-end
against the two things it replaces: the original Python `tetra3`, and the
cedar-detect(Rust gRPC)/cedar-solve(Python) flow that cedar-solve itself recommends as its
higher-performance alternative to `tetra3`'s own detector
(`reference-solutions/cedar-solve/tetra3/tetra3.py`'s module docstring). This change specifies a
harness to make that measurement, and to re-make it on demand as the solver/detector evolve — not
a one-off script, but a governed, re-runnable capability, which is why it gets a proper OpenSpec
proposal the way `feat-08-web-ui` retroactively governed a load-bearing dev harness instead of
being left as ungoverned tooling the way `tools/parity/`'s existing fixture-capture scripts are.

## What Changes

- Introduce the `eval-harness` capability: a re-runnable benchmark-and-parity harness that
  exercises three "lost-in-space" plate-solving implementations — original Python `tetra3`
  (`reference-solutions/tetra3`), the cedar-detect(gRPC)/cedar-solve(Python) flow
  (`reference-solutions/cedar-detect`, `reference-solutions/cedar-solve`), and the new Rust
  workflow's `ps-grpc` service — against a shared, checked-in image corpus.
- The harness measures both client-observed wall-clock latency and each system's own
  self-reported algorithm-only timing (already present on the wire in both `cedar_detect.proto`
  and `plate_solver.proto`, and in tetra3/cedar-solve's own solve-result dict), and cross-checks
  RA/Dec/Roll/FOV/matched-star output for parity within established tolerances.
- Results render to a machine-readable `results.json` plus human-readable
  `docs/benchmarks/report.md` and `docs/benchmarks/report.html`, so the comparison can be
  re-produced and re-read after future changes to `ps-detect`/`ps-solve`/`ps-grpc`.
- This proposal specifies the capability only; the harness implementation is tracked by
  `tasks.md` and built in a later pass, the same sequencing `feat-01`–`feat-06` followed.

## Capabilities

### New Capabilities
- `eval-harness`: a re-runnable performance-and-parity benchmark comparing the three plate-solver
  implementations on a shared image corpus, reporting wall-clock and self-reported algorithm
  timing plus solve-output parity to a machine-readable and two human-readable documents.

### Modified Capabilities

(none — `eval-harness` is a read-only consumer of `ps-grpc`, `ps-db`, and
`reference-solutions/`; it changes no other capability's requirements.)

## Impact

- **New code**: `tools/parity/benchmark/` (orchestrator, per-system adapters, corpus definition,
  parity checks, report renderer — Python, reusing the existing `tools/parity/` venv
  convention), one small Cargo example `ps-db/examples/npz_to_native.rs` (converts a reference
  `.npz` catalog to the native format `ps-grpc` requires, reusing `ps_db::importer::import_npz` /
  `ps_db::loader::save_native`, which `ps-grpc/src/service.rs`'s own tests already call this way).
- **New environment**: a second Python venv, `tools/parity/.venv-tetra3-orig`, alongside the
  existing `tools/parity/.venv` (cedar-solve) — required because both packages install a
  top-level module literally named `tetra3` and cannot coexist in one interpreter.
- **New output**: `docs/benchmarks/report.md`, `docs/benchmarks/report.html` (committed,
  regenerated per run); `tools/parity/benchmark/results.json` and a generated native database
  copy (both gitignored, regenerable).
- **No changes** to any product crate's runtime behavior (`ps-core`, `ps-detect`, `ps-db`,
  `ps-dbgen`, `ps-solve`, `ps-grpc`, `ps-web`) or to `reference-solutions/` (read-only throughout).
