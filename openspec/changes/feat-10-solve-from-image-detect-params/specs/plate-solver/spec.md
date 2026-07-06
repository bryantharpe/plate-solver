## MODIFIED Requirements

### Requirement: Solve inputs and defaults

The system SHALL provide `solve_from_centroids(star_centroids, size, fov_estimate,
fov_max_error, match_radius=0.01, match_threshold=1e-5, solve_timeout=5000, distortion=0,
match_max_error=0.002, …)` taking brightest-first `(y,x)` centroids and `size=(height,width)`,
and a `solve_from_image` wrapper that extracts centroids first and records extraction time. When
`fov_estimate` is absent, the system SHALL start from the database FOV-range midpoint. (Ref:
doc 06 §1–2; doc 08 §2.)

**`solve_from_image` SHALL accept explicit detection parameters** (`DetectParams`: sigma,
noise_estimate, binning, normalize_rows, detect_hot_pixels, return_binned,
use_binned_for_star_candidates) and SHALL pass them to the detection call. A `Default` set equal
to the cedar detection defaults (sigma=4.0, noise_estimate=1.0, binning=1, normalize_rows=false,
detect_hot_pixels=true, return_binned=false, use_binned_for_star_candidates=false) SHALL be
provided so callers that do not supply explicit detection parameters get the prior behavior
unchanged. (Ref: CODEBASE-REVIEW C2; Phase H H2.)

#### Scenario: Brightest-first requirement
- **WHEN** centroids are supplied
- **THEN** the solver treats them as brightest-first and searches bright stars first

#### Scenario: Default FOV from DB range
- **WHEN** no `fov_estimate` is provided
- **THEN** `fov_initial` is the midpoint of the database `[min_fov, max_fov]`

#### Scenario: Defaults preserve prior behavior
- **WHEN** `solve_from_image` is called without explicit `DetectParams`
- **THEN** detection runs with sigma=4.0, noise_estimate=1.0, binning=1, normalize_rows=false,
  detect_hot_pixels=true, return_binned=false — identical to the pre-change behavior
- **AND** the result is byte-for-byte identical to calling `solve_from_image` before this change

#### Scenario: Explicit detection parameters honored
- **WHEN** a caller supplies `solve_from_image_with_detect` with `DetectParams { sigma: 8.0, binning: 2, … }`
- **THEN** the internal `get_stars_from_image` call receives `sigma=8.0`, `binning=2`, and the
  caller's other detection fields — not the defaults

#### Scenario: Extraction time still reported
- **WHEN** `solve_from_image_with_detect` runs
- **THEN** `Solution.t_extract` reflects the wall-clock of the internal detection + centroid
  collection (FUA.1, unchanged)

#### Scenario: Solve math untouched
- **WHEN** `solve_from_image_with_detect` is called with `DetectParams::default()`
- **THEN** the search loop, verification, attitude recovery, and `combos_examined` are identical
  to the pre-change `solve_from_image` on the same image — detection parameters change detection,
  not solving