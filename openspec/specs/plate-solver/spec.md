# plate-solver Specification

## Purpose

The identification engine — the half of the pipeline that turns centroids into an attitude, with
**no prior pointing knowledge** (the *lost-in-space* problem). Given a brightest-first centroid
list, an image size, and a rough FOV, it recovers where the camera was pointing: RA, Dec, Roll,
plus refined FOV and lens distortion, and optionally the catalog identity and pixel position of
every matched star.

It works in two stages, and the split is the whole trick. Cheap **geometric candidate
generation** hashes 4-star patterns by their edge ratios and pulls candidates from the database.
Then **authoritative verification** recovers a trial attitude, projects the full catalog
neighborhood back into the image, counts matches, and accepts or rejects on a **binomial
false-alarm test** — so a candidate is confirmed by evidence the geometric hash never saw. The
false-alarm test is what makes the solver trustworthy rather than merely fast: without it, a
lucky hash collision is indistinguishable from a real match.

Bounded by `solve_timeout`, and reports a status code on failure rather than guessing.

## Requirements

### Requirement: Solve inputs and defaults

The system SHALL provide `solve_from_centroids(star_centroids, size, fov_estimate, fov_max_error,
match_radius=0.01, match_threshold=1e-5, solve_timeout=5000, distortion=0, match_max_error=0.002,
…)` taking brightest-first `(y,x)` centroids and `size=(height,width)`, and a `solve_from_image`
wrapper that extracts centroids first and records extraction time. When `fov_estimate` is absent,
the system SHALL start from the database FOV-range midpoint. (Ref: doc 06 §1–2; doc 08 §2.)

#### Scenario: Brightest-first requirement
- **WHEN** centroids are supplied
- **THEN** the solver treats them as brightest-first and searches bright stars first

#### Scenario: Default FOV from DB range
- **WHEN** no `fov_estimate` is provided
- **THEN** `fov_initial` is the midpoint of the database `[min_fov, max_fov]`

### Requirement: Noise estimated from the image when not supplied

`solve_from_image`'s `noise_estimate` (in `DetectParams`) SHALL be optional; when the caller does
not supply one, the system SHALL estimate noise **from the input image** — the same estimation the
centroid-extraction path performs — and SHALL NOT substitute a constant. (Ref: doc 06 §1–2; doc 08
§2; doc 04 §noise estimation.)

#### Scenario: Noise estimated from the image, never a constant
- **WHEN** `solve_from_image` runs without an explicit `noise_estimate`
- **THEN** the detection noise estimate is derived from the input image — the same value the
  extraction path would report for that image
- **AND** it is not a fixed constant such as `1.0`

### Requirement: Detection parameters pass through solve_from_image

`solve_from_image` SHALL take explicit detection parameters (`DetectParams`: `sigma`,
`noise_estimate`, `binning`, `normalize_rows`, `detect_hot_pixels`, `return_binned`,
`use_binned_for_star_candidates`) and pass them through to the detection call. It SHALL NOT hardcode
detection parameters. Detection parameters affect detection only: the search loop, verification,
attitude recovery, and `combos_examined` are unaffected by them. (Ref: doc 06 §1–2; doc 08 §2; doc
04 §noise estimation.)

#### Scenario: Explicit detection parameters honored
- **WHEN** `solve_from_image` is called with `DetectParams { sigma: 8.0, binning: 2, … }`
- **THEN** the internal detection call receives `sigma=8.0`, `binning=2`, and the caller's other
  detection fields — not substituted defaults

#### Scenario: Detection parameters do not perturb solve math
- **WHEN** `solve_from_image` and `solve_from_centroids` are run on the same resulting centroids
- **THEN** the search loop, verification, attitude recovery, and `combos_examined` agree —
  detection parameters change which centroids are found, not how they are solved

### Requirement: Preparation

The system SHALL prepare the solve by: setting `fov_initial`; dividing `match_threshold` by
`num_patterns` (Bonferroni); limiting centroids to the brightest `verification_stars_per_fov`;
undistorting centroids when a scalar `distortion` `k` is given (else deferring `k` estimation);
**cluster-busting** the centroids to a pattern-centroid subset using the database density rule; and
precomputing centroid vectors once. (Ref: doc 06 §3.)

#### Scenario: Cluster-busting separation
- **WHEN** the centroids are cluster-busted
- **THEN** the separation in pixels SHALL be `= width·separation_for_density(fov_initial, verification_stars_per_fov) / fov_initial`

#### Scenario: Fewer than 4 centroids fail
- **WHEN** fewer than 4 centroids are given
- **THEN** the system SHALL return failure with status `TOO_FEW`

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
refine FOV (and distortion `k` by least squares when `distortion` was not a fixed scalar), compute
residuals (RMSE, P90E, MAXE in arcsec), and return a solution with `RA, Dec, Roll, FOV` (degrees),
`distortion`, `RMSE`, `Matches`, `Prob` (= `prob_mismatch·num_patterns`), `epoch_equinox`,
`epoch_proper_motion`, `T_solve`, status `MATCH_FOUND`. (Ref: doc 06 §6.)

#### Scenario: Optional outputs listed
- **WHEN** the caller requests them
- **THEN** the solution SHALL also carry the optional `matched_centroids`/`matched_stars`/`matched_catID`, catalog list, rotation matrix, and target pixel/sky conversions

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

### Requirement: Cedar's detection settings

The system SHALL provide `DetectParams::cedar()`: the settings cedar-solve's example script passes
to cedar-detect, with which the parity goldens' star lists were made. (Ref:
`crates/plate-solver/src/solve.rs` `DetectParams::cedar`,
`crates/plate-solver/examples/solve_image.rs`; tests: `cedar_detects_as_its_example_script_does`,
and `every_sample_gives_cedars_answer` in `crates/finder/tests/api.rs`.)

#### Scenario: Default settings kept
- **WHEN** `DetectParams::default()` is used
- **THEN** it SHALL be unchanged: binning 1, no row levelling and no hot-pixel rejection

#### Scenario: Cedar's settings listed
- **WHEN** `DetectParams::cedar()` is called
- **THEN** the settings are sigma 8, 2×2 binning, rows levelled, hot pixels rejected and star candidates taken from the binned image, with no binned image returned and the noise estimated from the image

#### Scenario: Cedar's answer from these settings
- **WHEN** a frame is turned gray with `star_detection::gray` and detected with these settings
- **THEN** it solves to cedar's answer; the finder console and the `solve_image` example both read frames this way

#### Scenario: The settings
- **WHEN** `DetectParams::cedar()` is read
- **THEN** sigma is 8 and binning 2, rows are levelled, hot pixels are rejected, candidates
  come from the binned image, nothing is returned binned and no noise estimate is fixed

#### Scenario: The default is unchanged
- **WHEN** `DetectParams::default()` is read
- **THEN** binning is 1, and rows are not levelled nor hot pixels rejected
