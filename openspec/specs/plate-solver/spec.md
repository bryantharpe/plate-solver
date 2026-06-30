# plate-solver Specification

## Purpose
TBD - created by archiving change feat-05-plate-solver. Update Purpose after archive.
## Requirements
### Requirement: Solve inputs and defaults

The system SHALL provide `solve_from_centroids(star_centroids, size, fov_estimate,
fov_max_error, match_radius=0.01, match_threshold=1e-5, solve_timeout=5000, distortion=0,
match_max_error=0.002, …)` taking brightest-first `(y,x)` centroids and `size=(height,width)`,
and a `solve_from_image` wrapper that extracts centroids first and records extraction time. When
`fov_estimate` is absent, the system SHALL start from the database FOV-range midpoint. (Ref:
doc 06 §1–2; doc 08 §2.)

#### Scenario: Brightest-first requirement
- **WHEN** centroids are supplied
- **THEN** the solver treats them as brightest-first and searches bright stars first

#### Scenario: Default FOV from DB range
- **WHEN** no `fov_estimate` is provided
- **THEN** `fov_initial` is the midpoint of the database `[min_fov, max_fov]`

### Requirement: Preparation

The system SHALL prepare the solve by: setting `fov_initial`; dividing `match_threshold` by
`num_patterns` (Bonferroni); limiting centroids to the brightest `verification_stars_per_fov`;
undistorting centroids when a scalar `distortion` `k` is given (else deferring `k` estimation);
**cluster-busting** the centroids to a pattern-centroid subset using the database density rule
(separation in pixels `= width·separation_for_density(fov_initial, verification_stars_per_fov)
/ fov_initial`); and precomputing centroid vectors once. With fewer than 4 centroids it SHALL
return failure with status `TOO_FEW`. (Ref: doc 06 §3.)

#### Scenario: Threshold Bonferroni-corrected
- **WHEN** preparation runs against a database of `num_patterns`
- **THEN** the working threshold is `match_threshold / num_patterns`

#### Scenario: Cluster-busting limits pattern centroids
- **WHEN** centroids form a tight cluster
- **THEN** cluster-busting thins them so the cluster cannot dominate pattern formation

#### Scenario: Too few centroids
- **WHEN** fewer than 4 centroids remain
- **THEN** the solver returns status `TOO_FEW`

### Requirement: Image-pattern iteration

The system SHALL iterate 4-star image patterns over the cluster-busted centroids using
breadth-first combinations (brightest combinations first), checking `solve_timeout` (→ status
`TIMEOUT`) and a cancellation flag (→ status `CANCELLED`) on each iteration. (Ref: doc 06 §4.1.)

#### Scenario: Timeout bounds the search
- **WHEN** elapsed time exceeds `solve_timeout`
- **THEN** the solver stops and returns status `TIMEOUT`

#### Scenario: Cancellation honored
- **WHEN** a cancellation is requested mid-solve
- **THEN** the solver stops and returns status `CANCELLED`

### Requirement: Candidate-key generation

The system SHALL, per image pattern, compute the 6 sorted edge angles, the largest edge, and the
5 edge ratios, form a tolerance band `ratio ± p_max_err` (with `p_max_err = match_max_error`
clamped to at least the database `pattern_max_error`), enumerate candidate keys as the cartesian
product of per-ratio bin ranges, and order them **nearest-first** by squared distance to the
measured key so the most likely keys are tried first. (Ref: doc 06 §4.2–4.3.)

#### Scenario: Tolerance band from match_max_error
- **WHEN** the band half-width is computed
- **THEN** `p_max_err` is `match_max_error` clamped to be at least the database `pattern_max_error`

#### Scenario: Nearest-first ordering
- **WHEN** candidate keys in the band are enumerated
- **THEN** they are tried in ascending squared distance from the measured pattern key

### Requirement: Candidate gathering and pre-filters

The system SHALL, per candidate key, look up the database probe chain and apply the 16-bit key
pre-filter, the largest-edge/FOV pre-filter, and the edge-ratio band test (every catalog ratio
strictly inside the band) to yield the valid catalog patterns for verification. (Ref: doc 06
§4.4; feat-03-pattern-database.)

#### Scenario: Only band-consistent patterns verified
- **WHEN** a catalog pattern has an edge ratio outside the band
- **THEN** it is excluded before verification

### Requirement: Verification — attitude

The system SHALL verify a valid catalog pattern by: computing a coarse FOV from the largest-edge
ratio (or from focal length when no estimate is given); pairing the 4 image and 4 catalog stars
by centroid-distance order; solving the SVD attitude `R`; and rejecting the candidate when
`det(R) < 0` (a reflection). (Ref: doc 06 §5 steps 1–3.)

#### Scenario: Reflection rejected
- **WHEN** the candidate attitude has `det(R) < 0`
- **THEN** the candidate is rejected and the next is tried

#### Scenario: Correspondence by centroid order
- **WHEN** image and catalog pattern stars are paired
- **THEN** both are ordered by ascending distance from their pattern centroid so star m ↔ star m

### Requirement: Verification — projection and match

The system SHALL gather catalog stars within the diagonal-FOV radius of the implied boresight
(`R` row 0), derotate and project them to pixels (keeping in-frame), trim to the brightest (cedar
keeps `2·num_centroids` as a fudge factor), and match projected catalog stars to image centroids
within `match_radius·width`, made unique 1:1. (Ref: doc 06 §5 steps 4–6.)

#### Scenario: Diagonal FOV gathers candidates
- **WHEN** nearby catalog stars are gathered
- **THEN** the radius is the diagonal FOV `fov·√(w²+h²)/w` about the boresight

#### Scenario: Unique one-to-one matches
- **WHEN** projected catalog stars are matched to image centroids
- **THEN** each centroid and each catalog star participates in at most one match

### Requirement: Verification — false-alarm acceptance

The system SHALL accept a candidate when the binomial false-alarm probability is below the
corrected threshold: with `n` centroids, `Nc` nearby catalog stars, `m` matches,
`prob_single = Nc·match_radius²`, accept when `binom.cdf(n−(m−2), n, 1−prob_single) <
match_threshold`. The first accepted candidate wins; otherwise the next candidate is tried.
(Ref: doc 06 §5 step 7; doc 02 §9.)

#### Scenario: First acceptable match wins
- **WHEN** a candidate's false-alarm probability is below the corrected threshold
- **THEN** it is accepted immediately and the search stops

### Requirement: Refinement and outputs

On acceptance the system SHALL re-fit the attitude over **all** matched stars, extract RA/Dec/Roll,
refine FOV (and distortion `k` by least squares when `distortion` was not a fixed scalar),
compute residuals (RMSE, P90E, MAXE in arcsec), and return a solution with `RA, Dec, Roll, FOV`
(degrees), `distortion`, `RMSE`, `Matches`, `Prob` (= `prob_mismatch·num_patterns`),
`epoch_equinox`, `epoch_proper_motion`, `T_solve`, status `MATCH_FOUND`, and optional
`matched_centroids`/`matched_stars`/`matched_catID`, catalog list, rotation matrix, and target
pixel/sky conversions on request. (Ref: doc 06 §6.)

#### Scenario: Attitude re-fit over all matches
- **WHEN** a candidate is accepted
- **THEN** the returned attitude is recomputed from all matched stars, not just the 4 pattern stars

#### Scenario: Optional matches returned on request
- **WHEN** matched output is requested
- **THEN** the solution includes matched centroids `(y,x)`, matched star RA/Dec/mag, and catalog IDs

### Requirement: Status codes and failure

The system SHALL report a status of `MATCH_FOUND`, `NO_MATCH`, `TIMEOUT`, `CANCELLED`, or
`TOO_FEW`, and on any non-match SHALL return a solution with the attitude fields unset and
`T_solve` populated. (Ref: doc 06 §7.)

#### Scenario: Exhaustion returns NO_MATCH
- **WHEN** all image patterns are exhausted without acceptance
- **THEN** the solver returns status `NO_MATCH` with unset attitude fields

### Requirement: Reference parity

The system SHALL match the cedar reference on the reference test images: RA/Dec within a few
arcseconds and the set of matched catalog IDs identical. (Ref: doc 06 §10; PRD parity contract.)

#### Scenario: Reference image solves to parity
- **WHEN** a reference test image is solved with the same database and FOV estimate as cedar
- **THEN** the returned RA/Dec are within a few arcseconds of cedar's and the matched catalog IDs
  are identical

