## MODIFIED Requirements

### Requirement: SolveFromImage is one call end-to-end

`SolveFromImage` SHALL extract centroids internally and return a `Solution` in a single
response. It SHALL honor the detection fields of its `CentroidsRequest` (`extract`) — sigma,
binning, detect_hot_pixels, normalize_rows, use_binned_for_star_candidates, return_binned — and
SHALL estimate noise from the input image using the same estimation `ExtractCentroids` uses,
rather than hardcoding detection parameters. It SHALL resolve the effective binning with the
same rule `ExtractCentroids` uses, so the two paths cannot drift on detection semantics.
(Ref: doc 07 §1; CODEBASE-REVIEW C2; Phase H H2.)

#### Scenario: SolveFromImage is one call end-to-end
- **WHEN** a client calls `SolveFromImage` with an image and a FOV estimate
- **THEN** the service extracts centroids and returns a `Solution` in a single response

#### Scenario: Client sigma honored
- **WHEN** a client sends `SolveFromImage` with `extract.sigma = 8.0`
- **THEN** the internal detection runs with sigma=8.0, not a hardcoded default

#### Scenario: Client binning honored
- **WHEN** a client sends `SolveFromImage` with `extract.binning = 2` and
  `use_binned_for_star_candidates = true`
- **THEN** the internal detection runs with effective binning 2

#### Scenario: Noise estimated, not hardcoded
- **WHEN** `SolveFromImage` runs
- **THEN** the detection noise estimate is derived from the input image — the same value an
  `ExtractCentroids` call on the same image would return as `noise_estimate`
- **AND** it is not a constant such as `1.0`

#### Scenario: Consistency with ExtractCentroids + SolveFromCentroids
- **WHEN** `SolveFromImage` is called with the same `CentroidsRequest` detection fields as a
  paired `ExtractCentroids` call on the same image
- **THEN** the recovered attitude and matched catalog IDs match the composed
  `ExtractCentroids` + `SolveFromCentroids` path within parity tolerances (RA/Dec within 10
  arcsec, matched IDs exact)

#### Scenario: Effective binning shared with ExtractCentroids
- **WHEN** `SolveFromImage` resolves effective binning
- **THEN** it uses the same effective-binning rule as `ExtractCentroids` (None→2,
  Some(2)|Some(4)→that value, other→INVALID_ARGUMENT when binning is requested; 1 otherwise)

#### Scenario: No accuracy regression
- **WHEN** `SolveFromImage` is called on the reference image at `sigma = 4.0`
- **THEN** the result remains `MATCH_FOUND` with RA/Dec within 10 arcsec of the reference —
  the change honors the request, it does not regress the matched case