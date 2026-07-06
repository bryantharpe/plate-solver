## 1. Library: `DetectParams` + `solve_from_image_with_detect`

- [ ] 1.1 — Add `DetectParams` + `solve_from_image_with_detect`; make `solve_from_image` a wrapper
      — deps: none — In `ps-solve/src/lib.rs`: add `#[derive(Debug, Clone, Copy)] pub struct
      DetectParams { pub sigma: f64, pub noise_estimate: f64, pub binning: u32, pub
      normalize_rows: bool, pub detect_hot_pixels: bool, pub return_binned: bool, pub
      use_binned_for_star_candidates: bool }` with a `Default` impl returning **today's hardcoded
      values** verbatim (`sigma: 4.0, noise_estimate: 1.0, binning: 1, normalize_rows: false,
      detect_hot_pixels: true, return_binned: false, use_binned_for_star_candidates: false`).
      Rename the current `solve_from_image` body to `pub fn solve_from_image_with_detect(db,
      image, params, detect: &DetectParams) -> Solution`, replacing the six literals passed to
      `ps_detect::get_stars_from_image` with `detect.noise_estimate`, `detect.sigma`,
      `detect.normalize_rows`, `detect.binning`, `detect.detect_hot_pixels`, `detect.return_binned`.
      Add a one-line `pub fn solve_from_image(db, image, params) -> Solution` wrapper calling
      `solve_from_image_with_detect(db, image, params, &DetectParams::default())`. Document the
      struct fields and the default in a doc comment; cite CODEBASE-REVIEW C2 / Phase H H2.
      — AC: `cargo build -p ps-solve` green; `cargo test -p ps-solve` green; `sv6_solve_from_image_parity`
      (`ps-solve/src/lib.rs` ~:1547) unchanged and green (wrapper preserves today's literals via
      `Default`); a new unit test asserts a non-default `sigma` reaches detection — use the
      count of detected centroids as the proxy (run `solve_from_image_with_detect` on a fixture
      with `sigma=4.0` vs `sigma=8.0` and assert the returned `Solution` differs in
      `combos_examined` or matched count, *or* assert directly against `ps-detect`'s public
      `get_stars_from_image` with two `sigma` values — do NOT modify any file under `ps-detect/src/`);
      no file under `ps-detect/src/` is touched (grep-verify the diff). — commit:
      "feat(ps-solve): add DetectParams and solve_from_image_with_detect (C2/H2)"

## 2. gRPC handler: thread request detection fields + estimate noise

- [ ] 2.1 — `SolveFromImage` honors `extract` detection fields; shared effective-binning helper
      — deps: 1.1 — In `ps-grpc/src/service.rs`: (a) lift the effective-binning rule at
      `service.rs:179-193` into a small `fn resolve_effective_binning(use_binned: bool,
      return_binned: bool, binning: Option<i32>) -> Result<u32, Status>` and call it from both
      `extract_centroids` (replacing the inline block) and the new `solve_from_image` handler
      (mechanical extract, behavior-preserving — `cargo test -p ps-grpc` green proves it). (b) In
      `solve_from_image` (~:292-362): after building `GrayImage`, estimate noise via
      `estimate_noise_from_image(&image)` (same helper `extract_centroids` uses at ~:166); build
      `DetectParams` from `extract_req` — `sigma: extract_req.sigma`, `noise_estimate` (the
      estimated value), `binning: effective_binning` (from the shared helper), `normalize_rows:
      extract_req.normalize_rows`, `detect_hot_pixels: extract_req.detect_hot_pixels`,
      `return_binned: extract_req.return_binned`, `use_binned_for_star_candidates:
      extract_req.use_binned_for_star_candidates`; call `ps_solve_image_with_detect(&self.db,
      &image, &solve_params, &detect_params)` (the `crate::plate_solver` re-export of
      `solve_from_image_with_detect`); keep `sol.t_extract * 1000.0` for the wire field (FUA.1).
      Map `SolveParams` from `req.params` as today. Behavior of `extract_centroids` is unchanged
      (same shared helper, same inputs). — AC: `cargo build -p ps-grpc` green (needs `protoc`);
      `cargo test -p ps-grpc` green; existing `extract_centroids_*` tests unchanged; existing
      `solve_from_image_parity` (`service.rs:638`) still green; grep confirms the inline
      effective-binning block at `:179-193` is replaced by a call to the shared helper; the
      `solve_from_image` handler reads `extract_req.sigma`/`binning`/`normalize_rows`/
      `detect_hot_pixels`/`return_binned`/`use_binned_for_star_candidates`.
      — commit: "feat(ps-grpc): SolveFromImage honors request detection params (C2/H2)"
- [ ] 2.2 — Golden test: `SolveFromImage` with `sigma=8` solves hale_bopp — deps: 2.1 — Add a
      `ps-grpc` test (`solve_from_image_hale_bopp_sigma8` or extend the existing
      `solve_from_image_parity` with an additional case — prefer a separate test to keep the
      reference-image case isolated) that calls the `SolveFromImage` RPC on `hale_bopp.jpg` with
      `extract.sigma = 8.0` (and otherwise default detection fields) and asserts `status ==
      MATCH_FOUND` and RA/Dec within 10 arcsec of the golden attitude. **Capture the golden
      attitude from the reference**, not hand-fabricated: run the standalone
      `ExtractCentroids`+`SolveFromCentroids` path (the harness's `CedarFlowAdapter` or
      `PsGrpcAdapter` composed path) on `hale_bopp.jpg` at `sigma=8` and record the recovered
      RA/Dec as the fixture (commit the fixture under `ps-grpc/tests/fixtures/` or
      `ps-solve/tests/fixtures/`, following the existing fixture convention). Also assert
      `t_extract_ms > 0` (FUA.1 still holds on `SolveFromImage`). — AC: `cargo test -p ps-grpc`
      green; the new test passes; golden fixture committed and traceable to the reference run.
      — commit: "test(ps-grpc): SolveFromImage sigma=8 solves hale_bopp (C2/H2 golden)"

## 3. Re-measurement + parity STOP

- [ ] 3.1 — Re-measure and record — deps: 2.2 — Rebuild release (`cargo build --release -p
      ps-grpc`), re-run the eval harness (`tools/parity/.venv/bin/python
      tools/parity/benchmark/run_benchmark.py` + `parity.py` + `report.py`, regenerating
      `docs/benchmarks/report.md`/`.html`). Record in `notes/solve-perf-measurements.md` (new
      H2 section): (a) `hale_bopp.jpg` under the `SolveFromImage` RPC with `sigma=8` — before
      (0.168 s `NO_MATCH`, `combos_examined=8855`) vs after (sub-ms `MATCH_FOUND`); (b) the
      report's `ps_grpc vs cedar_flow` solve-stage ratio is **unchanged by design** (the solve
      stage uses `SolveFromCentroids`, not `SolveFromImage`) — state this explicitly so the record
      is honest; (c) the `ps_grpc_vs_cedar_flow` primary parity table is all-✓. Parity STOP rule:
      any change in the parity table, any `sv6_*` failure, or any `combos_examined` change on the
      8 images that already match means a bug — STOP and investigate, do not proceed. — AC:
      report regenerated; measurements note updated with the H2 before/after table; parity
      identical-green; honest verdict recorded (including "headline solve-stage ratio unchanged
      by design; win is on the `SolveFromImage` RPC worst case"). — commit:
      "docs: record C2/H2 SolveFromImage-detect-params measurements"

## Gates (apply to every bead)

- `cargo build -p <crate>` and `cargo test -p <crate>` exit 0; `cargo test --workspace` exits 0
  with the named parity tests (`sv6_solve_from_centroids_parity`,
  `sv6_solve_from_image_parity`) green within tolerance.
- `cargo clippy -p <crate>` introduces no new warnings.
- Never weaken a gate: never `#[ignore]` or delete a parity test, never loosen a stated
  tolerance (RA/Dec 10 arcsec, matched IDs exact, centroids ±0.1 px), never stub/short-circuit a
  check or delete an assertion to pass — fix the code.
- `ps-detect/src/*` is untouched (grep-verify the diff at every bead — the FU-A constraint).
- `combos_examined` is unchanged on the 8 images that already match (the search loop is
  untouched — any change is a logic bug, STOP).
- `ps_grpc_vs_cedar_flow` harness parity table all-✓ at every bead.
- Judge: independent GLM-5.2 peer with tool access (per the 2026-07-05 Decisions Log — real
  Anthropic Sonnet is not reachable in this environment's LiteLLM router). The empirical gates
  are model-independent facts; the gate is the source of truth.