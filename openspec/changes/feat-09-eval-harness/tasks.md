## 1. Environment & fixtures

- [x] 1.1 Human-in-the-loop: install `libjpeg-dev`/`zlib1g-dev` (needed for `Pillow<9` to build
      from source on linux/aarch64 hosts; skip if the host already has usable wheels)
- [x] 1.2 Create `tools/parity/.venv-tetra3-orig`, `pip install -e reference-solutions/tetra3`,
      freeze to `tools/parity/requirements-tetra3-orig.txt`
- [x] 1.3 Extend `tools/parity/.venv` with cedar-solve's `[cedar-detect]` extra (`grpcio`,
      `protobuf`) plus `grpcio-tools`; re-freeze `tools/parity/requirements.txt`; document both
      venvs and why they're separate in `tools/parity/README.md`
- [x] 1.4 Release-build `ps-grpc` (`cargo build --release -p ps-grpc`) and `cedar-detect-server`
      (`cargo build --release --manifest-path reference-solutions/cedar-detect/Cargo.toml --bin
      cedar-detect-server`)
- [x] 1.5 Generate Python gRPC stubs for `plate_solver.proto` via a new
      `tools/parity/benchmark/compile_plate_solver_proto.py` (mirroring
      `reference-solutions/cedar-solve/scripts/compile_proto.py`'s invocation shape); commit the
      generated `tools/parity/benchmark/generated/plate_solver_pb2*.py`
- [x] 1.6 Add `ps-db/examples/npz_to_native.rs` (`import_npz` → `save_native`); run it once against
      `reference-solutions/cedar-solve/tetra3/data/default_database.npz` to produce the gitignored
      `tools/parity/benchmark/generated/shared_catalog.bin`
- [x] 1.7 Add `tools/parity/benchmark/corpus.py`: the 11-image list (9 astronomical + 2 stress),
      sourced from `reference-solutions/cedar-detect/test_data/`
- [x] 1.8 `.gitignore`: `tools/parity/.venv-tetra3-orig`, `tools/parity/benchmark/generated/*.bin`,
      `tools/parity/benchmark/results.json`
- [x] 1.9 Exit check: spawn each server standalone once, call one RPC by hand (`GetInfo` for
      ps-grpc, a tiny `ExtractCentroids` for cedar-detect), confirm both respond

## 2. Harness

- [x] 2.1 `tools/parity/benchmark/servers.py`: subprocess lifecycle (spawn, health-check via a
      cheap RPC with a deadline, `atexit`/`SIGINT`/`SIGTERM`-guarded teardown) for
      `cedar-detect-server` and `ps-grpc`
- [x] 2.2 `tools/parity/benchmark/adapters.py`: uniform `detect(...)` /
      `solve_from_image(...)` interface over `TetraOriginalAdapter` (subprocess),
      `CedarFlowAdapter` (gRPC `ExtractCentroids` + in-process cedar-solve `solve_from_centroids`),
      `PsGrpcAdapter` (gRPC `ExtractCentroids`/`SolveFromImage`/`SolveFromCentroids`)
- [x] 2.3 `tools/parity/benchmark/tetra3_original_runner.py`: runs under `.venv-tetra3-orig`,
      loads `reference-solutions/tetra3/tetra3/data/default_database.npz` once, batches N
      iterations, prints one JSON blob
- [x] 2.4 Explicit shared parameters (`sigma=4.0`, `detect_hot_pixels=true`,
      `normalize_rows=false`, no binning, `match_radius`, `match_threshold`, `distortion=0.0`)
      threaded through every adapter call, not left as divergent per-system defaults
- [x] 2.5 Dual timing capture in every adapter: client wall-clock (`time.perf_counter()`) and each
      system's self-reported algorithm-only time; new workflow's extraction time comes from a
      standalone `ExtractCentroids` call, never from `Solution.t_extract_ms`
- [x] 2.6 `tools/parity/benchmark/run_benchmark.py`: CLI entrypoint (configurable iteration/warmup
      counts, defaulting to detect warmup=3/N=20, solve warmup=1/N=5, stress-image N=1/5s
      timeout); writes `results.json` with run metadata (host arch/cpu count, shared params,
      iteration counts, which catalog each system used, known-limitations list)

## 3. Parity check

- [x] 3.1 `tools/parity/benchmark/parity.py`: RA/Dec within 10 arcsec, matched catalog IDs
      exact/near-exact, detection centroids within ±0.1px — reusing
      `openspec/IMPLEMENTATION-STATUS.md`'s established tolerances verbatim
- [x] 3.2 Propose and document Roll (0.01°) and FOV (0.1% relative) tolerances, labeled in the
      report as harness-defined (no prior end-to-end tolerance exists for these two fields)
- [x] 3.3 Three pairwise comparisons per astronomical image: `ps_grpc` vs `cedar_flow` (primary,
      same catalog), `ps_grpc` vs `tetra3_original` and `cedar_flow` vs `tetra3_original` (both
      labeled "cross-catalog sanity check")
- [x] 3.4 Mismatches recorded as `flagged: true` in `results.json`, always surfaced in the report;
      the run always completes regardless of any single image's mismatch
- [x] 3.5 Stress images excluded from RA/Dec comparison but their solve `status` (expect
      NO_MATCH/TOO_FEW everywhere) is still cross-checked and reported

## 4. Report generation

- [x] 4.1 `tools/parity/benchmark/report.py` (stdlib only: `json`, `statistics`, `html`): reads
      `results.json`, deterministically emits `docs/benchmarks/report.md` and
      `docs/benchmarks/report.html` (self-contained, no CDN/JS framework)
- [x] 4.2 Report content: headline speedup summary; methodology & environment disclosure
      (Linux/aarch64 host, not the PRD's RPi-4B/mobile target; iteration/warmup counts;
      cross-catalog caveat; `C1`/`C2` citations); per-image tables (wall-clock + self-reported,
      detect + solve, all three systems); aggregate median-speedup table over the 9 astronomical
      images; parity results table with flags; reproduction commands appendix

## 5. Execution & sanity check

- [x] 5.1 Run the full pipeline once end-to-end (env setup → `run_benchmark.py` →
      `report.py`)
- [x] 5.2 Spot-check: the `cedar_flow` row for
      `2019-07-29T204726_Alt40_Azi-135_Try1.jpg` matches
      `ps-solve/tests/fixtures/reference_solve.json`'s existing captured values.
      RA/Dec pass per-axis within 10 arcsec (the only tolerance
      `IMPLEMENTATION-STATUS.md` actually establishes for a from-image
      comparison against this fixture — the same check `ps-solve`'s own
      `sv6_solve_from_image_parity` runs on this exact pair) and FOV passes
      within the harness-defined 0.1% relative bound. Roll (0.0265° vs the
      harness-defined 0.01° same-detector bound) and matched-ID count (57 vs
      19, symdiff 38 vs the harness-defined same-detector bound of 2) diverge
      from the fixture, but this is a proven artifact of the fixture being
      captured via `capture_solve.py`'s Python `tetra3.get_centroids_from_image`
      (20 centroids) versus the live harness's `cedar-detect-server` gRPC
      extractor (127 centroids on this run) against the same catalog — the
      19 reference matched IDs are an exact subset of the 57 fresh IDs,
      confirming same field/pointing, just more stars matched. The
      same-detector `ps_grpc` vs `cedar_flow` pairwise comparison (the
      harness's actual same-catalog gate) is unflagged for this image.
- [x] 5.3 Confirm `report.md`/`report.html` re-render byte-identical from the same `results.json`
      run twice, and that no `NaN`/`None` leaks into formatted output
