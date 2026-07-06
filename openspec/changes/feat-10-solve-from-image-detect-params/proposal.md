## Why

`SolveFromImage` is the product's one-call end-to-end RPC — the grpc-service spec calls it out
specifically ("SolveFromImage is one call end-to-end": image + FOV → solution, extracting
centroids internally). It is also the *only* RPC that does detection server-side as part of a
solve. Today that internal detection silently ignores **every** detection field on the request's
`CentroidsRequest` and hardcodes the detection parameters instead:

- `ps-solve/src/lib.rs` `solve_from_image` calls `ps_detect::get_stars_from_image` with fixed
  `noise_estimate=1.0`, `sigma=4.0`, `normalize_rows=false`, `binning=1`, `detect_hot_pixels=true`,
  `return_binned=false`, regardless of what the client sent on `SolveFromImageRequest.extract`.
- `ps-grpc/src/service.rs` `solve_from_image` never reads `extract.sigma` / `binning` /
  `detect_hot_pixels` / `normalize_rows` / `use_binned_for_star_candidates` / `return_binned`
  and never estimates noise from the image (unlike `ExtractCentroids`, which does).

This is CODEBASE-REVIEW **C2** and Phase H task **H2** (the SP4 run log's named "next" open Phase
H task: "next: H2"). It is a correctness contract mismatch at the public API boundary: a client
that requests `binning=2` or `sigma=8` gets `binning=1` / `sigma=4.0` behavior with no error.

It is also the measured root cause of the one real solve-latency outlier on the corpus.
`hale_bopp.jpg` `NoMatch`'es in **0.168 s** under `solve_from_image` (8855-combo full-space
exhaustion, `C(23,4)`, `combo_count` defaults), while the *same* image `MATCH_FOUND`'s in the
benchmark's solve stage (which uses `SolveFromCentroids` with centroids from a standalone
`ExtractCentroids` that estimates noise from the image). The divergence is exactly the hardcoded
`noise_estimate=1.0` vs the image-estimated noise the standalone extractor uses — different
noise → different detection threshold → too-few/wrong centroids → NoMatch and a full
combinatorial walk. hale_bopp's own golden detection fixture (SD1/SD6) was captured at `sigma=8`.
Threading the request's detection params collapses the 0.168 s outlier to a sub-ms match.

## What Changes

- `ps-solve` gains an explicit detection-parameters argument on `solve_from_image`: a new
  `DetectParams` struct (sigma, noise_estimate, binning, normalize_rows, detect_hot_pixels,
  return_binned, use_binned_for_star_candidates) with `Default` equal to **today's hardcoded
  values**, and a new `solve_from_image_with_detect(db, image, params, detect)`. The existing
  `solve_from_image(db, image, params)` becomes a one-line wrapper that calls the new function
  with `DetectParams::default()` — byte-for-byte preserving current behavior (and the
  `sv6_solve_from_image_parity` test).
- `ps-grpc` `SolveFromImage` estimates noise from the input image (the same
  `estimate_noise_from_image` helper `ExtractCentroids` uses), resolves the effective binning
  with the same rule `ExtractCentroids` uses, and threads the request's `extract` detection
  fields into `DetectParams`. The two paths (`ExtractCentroids` and `SolveFromImage`) can no
  longer drift on detection semantics.
- No proto change. `SolveFromImageRequest` already carries `CentroidsRequest extract` with all
  the needed fields. No `ps-detect` change — only the arguments `ps-solve` passes to
  `ps_detect::get_stars_from_image` change (the FU-A "ps-detect internals untouched" constraint
  is respected: no file under `ps-detect/src/` is modified).
- No solve/matching math change. The search, verification, and attitude-recovery code are
  untouched. `combos_examined` is unchanged on every image that already matches.

## Capabilities

### New Capabilities

(none.)

### Modified Capabilities

- `plate-solver`: `solve_from_image` accepts detection parameters; a default set (cedar
  defaults) is provided for callers that don't supply them. Existing behavior is preserved for
  callers that use the default.
- `grpc-service`: `SolveFromImage` honors the detection fields of its `CentroidsRequest`
  (`extract`) — sigma, binning, detect_hot_pixels, normalize_rows, use_binned_for_star_candidates,
  return_binned — and estimates noise from the image (the same estimation `ExtractCentroids`
  uses) instead of hardcoding detection parameters.

## Impact

- **Modified code:** `ps-solve/src/lib.rs` (additive `DetectParams` struct + `Default`, new
  `solve_from_image_with_detect` fn, `solve_from_image` becomes a thin wrapper; all
  construction sites of `Solution` already carry `t_extract` per FUA.1). `ps-grpc/src/service.rs`
  (`solve_from_image` handler: estimate noise, resolve effective binning, build `DetectParams`
  from `extract`, call the new fn; lift the effective-binning rule at `service.rs:179-193` into a
  small shared helper used by both `ExtractCentroids` and `SolveFromImage`).
- **Untouched:** `ps-detect/src/*` (FU-A constraint), the proto, `SolveFromCentroids`,
  `ps-core`, `ps-db`, `ps-dbgen`, `ps-web`, `reference-solutions/`, `openspec/specs/` (until
  archive).
- **No tolerance change:** RA/Dec 10 arcsec, matched IDs exact, centroids ±0.1 px — verbatim
  from `openspec/IMPLEMENTATION-STATUS.md`. Never loosened, never `#[ignore]`d, never stubbed.
- **Measurement consequence (not an acceptance criterion):** `hale_bopp.jpg` under
  `SolveFromImage` with `sigma=8` is expected to go from `NO_MATCH` (0.168 s) to `MATCH_FOUND`
  (sub-ms). This is a *consequence* of honoring the request, not a value baked into the spec —
  the spec requires the request be honored, not that any particular image match. Golden values
  for any new test are captured from the reference, never hand-fabricated.
- **Benchmark headline ratios:** the eval-harness solve stage uses `SolveFromCentroids` (not
  `SolveFromImage`), so this change does **not** move the report's `ps_grpc vs cedar_flow`
  solve-stage ratio. Its measured effect is on the `SolveFromImage` RPC worst case. This is
  stated up front so the re-measurement bead (H2.3) reports honestly, including "no change to
  the headline ratio by design."