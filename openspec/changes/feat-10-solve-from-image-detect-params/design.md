## Context

`solve_from_image` is the only RPC that fuses detection and solving server-side. Every other
solve path (`SolveFromCentroids`) receives centroids that were extracted by a separate
`ExtractCentroids` call — which honors the request's `sigma`/`binning`/`normalize_rows`/
`detect_hot_pixels` and estimates noise from the image. `SolveFromImage` does neither: it
hardcodes the detection args. The result is that `SolveFromImage` and
`ExtractCentroids`+`SolveFromCentroids` are not guaranteed to produce the same solve on the same
image — they diverge whenever the client's detection params differ from the hardcoded defaults
or whenever image-estimated noise differs from the hardcoded `1.0`. This change closes that
gap.

Verified against the actual files (line numbers drift — `grep` before editing):

- `ps-solve/src/lib.rs` `solve_from_image` (~:664-685) calls `ps_detect::get_stars_from_image`
  with literals `1.0, 4.0, false, 1, true, false` (noise, sigma, normalize_rows, binning,
  detect_hot_pixels, return_binned). `SolveParams` (~:90-107) carries only solve-side knobs
  (match_radius/threshold/timeout/distortion/fov_*); it has no detection fields.
- `ps-grpc/src/service.rs` `ExtractCentroids` (~:107-260) does: take image buffer (FUA.2),
  `estimate_noise_from_image(&image)` (~:166), resolve `effective_binning` from
  `use_binned || return_binned` and `req.binning` (~:179-193), call `get_stars_from_image` with
  the request's `sigma`/`normalize_rows`/`detect_hot_pixels`/`effective_binning`/`return_binned`.
- `ps-grpc/src/service.rs` `solve_from_image` (~:292-362) does none of that: it takes the image,
  builds `GrayImage`, maps `SolveParams` only, and calls `ps_solve_image(&self.db, &image,
  &solve_params)` — the `extract` request's detection fields are read only to get the image, then
  discarded.
- `SolveFromImageRequest` (`ps-grpc/proto/plate_solver.proto`:79-82) = `{ CentroidsRequest
  extract = 1; SolveParams params = 2; }`. `CentroidsRequest` already carries `sigma`, `binning`,
  `detect_hot_pixels`, `normalize_rows`, `return_binned`, `use_binned_for_star_candidates`,
  `estimate_background_region`. **Every field needed is already on the wire.** No proto change.
- `ps-detect::get_stars_from_image(image, noise_estimate, sigma, normalize_rows, binning,
  detect_hot_pixels, return_binned)` is the public entry point; changing what `ps-solve` passes
  to it does not modify `ps-detect` (the FU-A "ps-detect internals untouched" constraint is
  respected — no file under `ps-detect/src/` is touched, only call-site arguments in `ps-solve`
  and `ps-grpc`).
- The benchmark harness (`tools/parity/benchmark/adapters.py`) solve stage composes
  `ExtractCentroids` + `SolveFromCentroids` and measures that combined wall-clock
  (`adapters.py:~225-235`); it does **not** call the fused `SolveFromImage` RPC for the solve
  stage. So this change does not move the report's solve-stage ratio — see H2.3's honest
  re-measurement.

## Goals / Non-Goals

**Goals:**

- `SolveFromImage` honors every detection field on its `CentroidsRequest` (`extract`), exactly
  as `ExtractCentroids` does.
- `solve_from_image` (the library fn) accepts explicit detection params; the existing signature
  preserves today's behavior via a default, so `sv6_solve_from_image_parity` is unchanged.
- `SolveFromImage` and `ExtractCentroids`+`SolveFromCentroids` produce the same solve on the same
  image+params (a parity *improvement* — today they diverge via noise=1.0 vs estimated noise).
- No accuracy regression. Tolerances fixed in `openspec/IMPLEMENTATION-STATUS.md` are never
  loosened. The `ps_grpc_vs_cedar_flow` harness parity table stays all-✓.

**Non-Goals:**

- No `ps-detect` internal change (FU-A constraint). Only the arguments `ps-solve` passes to
  `ps_detect::get_stars_from_image` change.
- No proto change. `SolveFromImageRequest` already carries `CentroidsRequest`.
- No change to `SolveFromCentroids` (it has no detection step — `t_extract_ms == 0.0` is correct
  there and stays).
- No change to the solve/matching math (search, verification, attitude recovery, false-alarm).
  `combos_examined` is unchanged on every image that already matches — it counts iterations of
  an untouched loop.
- Not a mobile port, not a memory audit, not a parallelism change (FU-C is separate, spec-only).
- Not baking any specific image's outcome into the spec. hale_bopp matching at sigma=8 is a
  *consequence* of honoring the request, not an acceptance criterion.

## Decisions

### D1. Backward-compatible wrapper, not a signature break

Add `pub struct DetectParams` with `Default` = **today's hardcoded values** (sigma=4.0,
noise_estimate=1.0, binning=1, normalize_rows=false, detect_hot_pixels=true, return_binned=false,
use_binned_for_star_candidates=false). Add `pub fn solve_from_image_with_detect(db, image,
params, detect: &DetectParams) -> Solution` containing the current body with the literals
replaced by `detect.*` fields. Make the existing `pub fn solve_from_image(db, image, params)` a
one-line wrapper: `solve_from_image_with_detect(db, image, params, &DetectParams::default())`.

**Why:** `sv6_solve_from_image_parity` (`ps-solve/src/lib.rs` ~:1547) calls `solve_from_image`
and asserts `MATCH_FOUND` + RA/Dec within 10 arcsec on `2019-07-29T204726_Alt40_Azi-135_Try1.jpg`.
With the wrapper, that call gets `DetectParams::default()` = today's literals → identical output →
the test stays green by construction, not by re-tuning. This is the primary risk control: the
parity gate cannot move because the default path is byte-identical to the pre-change path.

### D2. Noise is estimated by the handler, not the library default

The library `DetectParams::default().noise_estimate = 1.0` (preserves the current wrapper
behavior). The `SolveFromImage` *handler* estimates noise from the image with the same
`estimate_noise_from_image` helper `ExtractCentroids` uses (`service.rs:166`) and passes that
into `DetectParams.noise_estimate`. This makes `SolveFromImage`'s detection identical to
`ExtractCentroids`'s detection on the same image+params — the load-bearing consistency property.
The library default stays `1.0` so the standalone `solve_from_image` wrapper (and `sv6`) is
unchanged.

### D3. Effective-binning rule is shared, not duplicated

`ExtractCentroids` resolves effective binning at `service.rs:179-193`:
`effective_binning = if use_binned || return_binned { match req.binning { None=>2, Some(2)|Some(4)=>*, other=>INVALID_ARGUMENT } } else { 1 }`.
Lift this into a small pure helper (e.g. `fn resolve_effective_binning(use_binned, return_binned,
binning: Option<i32>) -> Result<u32, Status>`) used by both handlers. This is in scope because the
two paths must not drift on binning semantics for the same `CentroidsRequest`. It is a mechanical
extract-and-call, behavior-preserving.

### D4. SolveParams is not extended with detection fields

Detection params live on `DetectParams` (a new struct), not on `SolveParams`. `SolveParams`
already has a clean solve-only meaning (match_radius/threshold/timeout/distortion/fov_*);
overloading it with detection fields would muddy the boundary between "how to detect" and "how
to solve," and would force every `solve_from_centroids` caller to carry detection fields it
doesn't use. The handler composes: `DetectParams` (from `extract`) + `SolveParams` (from
`params`).

### D5. Parity STOP rule (carried forward from SP2.1 / FU-A)

At every bead: `cargo test --workspace` green; `sv6_solve_from_centroids_parity` (19/19 matched
IDs exact, RA/Dec within 10 arcsec) green; `sv6_solve_from_image_parity` green; the harness
`ps_grpc_vs_cedar_flow` primary table all-✓ (centroids exact, RA within ~1″, Dec within ~0.2″,
Roll exact, FOV within 0.1%, matched IDs exact). After D1/D2, `SolveFromImage` at the same
detection params as a paired `ExtractCentroids` **should** produce the same solve as the composed
path — any divergence is a bug, STOP and investigate (don't paper over). `combos_examined` is
unchanged on the 8 images that already match (the search loop is untouched).

## Risks

- **Behavior change at a public API.** `SolveFromImage`'s output may change vs the pre-change
  hardcoded-params behavior (it now honors the request). This is the intended correctness fix.
  Risk is bounded by: (a) the benchmark solve stage doesn't use this RPC, so the headline ratios
  are insulated; (b) `solve_from_image_parity` (`service.rs:638`) uses the reference image at
  sigma≈4, which matches under both the old hardcoded path and the new honored-params path; (c)
  the wrapper keeps the library default path identical.
- **Golden value for the new hale_bopp test.** H2.2 asserts `SolveFromImage` with `sigma=8` on
  hale_bopp returns `MATCH_FOUND`. The matched-ID set / RA/Dec golden must be captured from the
  reference (the standalone `ExtractCentroids`+`SolveFromCentroids` path at sigma=8), never
  hand-fabricated — same rule as every other golden value in this repo.
- **Judge model.** Per the 2026-07-05 Decisions Log, this environment's LiteLLM router exposes
  only GLM-family models; `ps-judge`/real Sonnet are not reachable. Each bead is judged by an
  **independent GLM-5.2 peer with tool access** (fresh context, re-runs the cargo gate, grep-
  verifies invariants). The empirical gates (test results, `combos_examined`, `t_extract_ms`,
  float-order) are model-independent facts, so a same-model peer judge is real verification; its
  weakness (same-family blind spots) is mitigated by the adversarial prompt and by the gate
  being the source of truth. A stronger judge requires a different API key/router — left to the
  user.

## Sequencing

This change is independent of the open FU-A/FU-B beads (FUA.3 re-measure, FUB.1-3 exhaustion
trim) and can be slung in parallel or before them. It does not depend on FU-C (rayon parallel
search) and does not block it. It touches no file the FU beads touch (`ps-solve/src/lib.rs` is
shared with FUB.2, but the regions differ: this change is above the search loop at ~:664, FUB.2
is inside the verification path at ~:300-560 — merge conflicts, if any, are mechanical).