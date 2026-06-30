## ADDED Requirements

### Requirement: Celestial unit-vector conversion

The system SHALL convert between celestial coordinates `(RA, Dec)` in radians and equatorial
unit vectors using `x = cos(RA)cos(Dec)`, `y = sin(RA)cos(Dec)`, `z = sin(Dec)`, and the
inverse `RA = atan2(y, x) mod 2π`, `Dec = arcsin(z)`. (Ref: doc 02 §1.3.)

#### Scenario: Forward conversion produces a unit vector
- **WHEN** `(RA, Dec)` are converted to `(x, y, z)`
- **THEN** the result has unit norm within 1e-12
- **AND** `x = cos(RA)cos(Dec)`, `y = sin(RA)cos(Dec)`, `z = sin(Dec)`

#### Scenario: Round-trip is identity
- **WHEN** `(RA, Dec)` with `Dec` in `(-π/2, π/2)` are converted to a vector and back
- **THEN** the recovered `(RA mod 2π, Dec)` equals the input within 1e-12

### Requirement: Angular distance via 2·arcsin(d/2)

The system SHALL compute the central angle between two unit vectors at chord (Euclidean)
distance `d` as `angle = 2·arcsin(d/2)`, and the inverse `d = 2·sin(angle/2)`. This form MUST
be used everywhere (pattern edges, residuals, FOV math) in preference to `arccos(u·v)` for
small-angle conditioning. (Ref: doc 02 §2.)

#### Scenario: Angle/chord inversion
- **WHEN** an angle is converted to a chord distance and back
- **THEN** the recovered angle equals the input within 1e-12

#### Scenario: Small-angle conditioning
- **WHEN** two unit vectors are separated by a sub-arcsecond angle
- **THEN** `2·arcsin(d/2)` returns the angle without the precision loss that `arccos(u·v)`
  exhibits near 1

### Requirement: Pinhole projection — pixels to camera vectors

The system SHALL map pixel centroids `(y, x)` to camera-frame unit vectors `(i, j, k)` for a
rectilinear lens of horizontal field of view `fov` and image `width`, using
`scale_factor = 2·tan(fov/2)/width`, assigning `(k, j) = (img_center − centroid)·scale_factor`
with `img_center = [height/2, width/2]`, `i = 1` (boresight), then normalizing each vector to
unit length. (Ref: doc 02 §3.1.)

#### Scenario: Image center maps to the boresight
- **WHEN** the centroid equals the image center `[height/2, width/2]`
- **THEN** the resulting unit vector is the boresight `(1, 0, 0)`

#### Scenario: Horizontal edge maps to tan(fov/2)
- **WHEN** a centroid lies at the horizontal image edge (`width/2` from center in x)
- **THEN** before normalization its `j` component equals `tan(fov/2)`

### Requirement: Pinhole projection — camera vectors to pixels

The system SHALL map derotated camera-frame vectors back to pixel coordinates using
`scale_factor = −width/(2·tan(fov/2))`, `centroids = scale_factor · vec[:, (k, j)] / vec[:, i]`,
offset by `[height/2, width/2]`, and SHALL return the indices of vectors that fall inside the
image (`0 < y < height`, `0 < x < width`). Vectors with non-positive boresight component
(`i ≤ 0`, behind the camera) MUST be excluded. (Ref: doc 02 §3.2.)

#### Scenario: Projection inverts unprojection
- **WHEN** an in-frame centroid is converted to a vector and projected back at the same `fov`
- **THEN** the recovered `(y, x)` equals the original within 1e-9

#### Scenario: Behind-camera vectors are dropped
- **WHEN** a vector has boresight component `i ≤ 0`
- **THEN** it is not returned in the in-frame `keep` set

### Requirement: Lens distortion — undistort centroids

The system SHALL undistort centroids in closed form for a single-parameter radial model where
`k` is the fractional displacement at the half-width radius: center the coords, compute radius
`r`, scale by `(1 − k'·r²)/(1 − k)` with `k' = k·(2/width)²`, then decenter. `k < 0` is barrel,
`k > 0` pincushion. (Ref: doc 02 §4.1.)

#### Scenario: Zero distortion is identity
- **WHEN** centroids are undistorted with `k = 0`
- **THEN** the output equals the input exactly

#### Scenario: Center pixel is fixed
- **WHEN** a centroid at the image center is undistorted with any `k`
- **THEN** its position is unchanged

### Requirement: Lens distortion — distort centroids

The system SHALL apply the forward distortion `r_u → r_d` by Newton–Raphson inversion of the
undistortion model (default `tol = 1e-6`, `maxiter = 30`), so that distorting then undistorting
with the same `k` round-trips. (Ref: doc 02 §4.2.)

#### Scenario: Distort/undistort round-trip
- **WHEN** centroids are distorted and then undistorted with the same `k`
- **THEN** the recovered positions equal the originals within `tol`

#### Scenario: Convergence bound
- **WHEN** the Newton iteration is run
- **THEN** it terminates within `maxiter` iterations or when `max|error| < tol`

### Requirement: Attitude determination via Wahba/SVD

The system SHALL solve Wahba's problem for the least-squares rotation relating ordered,
corresponding image-frame and catalog-frame unit-vector sets via the cross-covariance
`H = image_vectorsᵀ · catalog_vectors`, `U, S, Vᵀ = svd(H)`, `R = U·Vᵀ`. (Ref: doc 02 §5.)

#### Scenario: Recovers a known rotation
- **WHEN** catalog vectors are rotated by a known `R0` to produce image vectors and the solver
  is run on the pair
- **THEN** the returned `R` equals `R0` within 1e-9

#### Scenario: Uses 2·arcsin residual convention
- **WHEN** residuals are computed after attitude
- **THEN** per-star angle uses `2·arcsin(d/2)` of the chord between matched vectors

### Requirement: Reflection guard via det(R)

The system SHALL treat a candidate attitude whose rotation matrix has `det(R) < 0` (a
reflection, not a proper rotation) as a false positive to be rejected, matching cedar's fast
filter. (Ref: doc 02 §5, doc 06 §5.)

#### Scenario: Reflection is rejected
- **WHEN** an attitude candidate yields `det(R) < 0`
- **THEN** the candidate is rejected as a reflection

### Requirement: RA/Dec/Roll extraction from attitude

The system SHALL extract pointing from `R` (rows are camera axes in the celestial frame) as
`RA = atan2(R[0,1], R[0,0]) mod 2π`, `Dec = atan2(R[0,2], ‖R[1:3,2]‖)`,
`Roll = atan2(R[1,2], R[2,2]) mod 2π`, where row 0 is the boresight (image-center) direction.
(Ref: doc 02 §5.1.)

#### Scenario: Boresight row gives image-center RA/Dec
- **WHEN** RA/Dec are extracted from `R`
- **THEN** they equal the spherical coordinates of `R[0, :]`

### Requirement: Edge-ratio pattern key

The system SHALL compute a 4-star pattern's key by: forming all `C(4,2)=6` pairwise edge angles
(`2·arcsin(½‖vi−vj‖)`), sorting ascending, taking the largest edge `L` as normalizer, computing
the 5 ratios `e[m]/L` for `m = 0..4`, and quantizing each as `key[m] = int(ratio·pattern_bins)`,
with `pattern_bins = round(1/(4·pattern_max_error))`. The 5-tuple is the pattern key, the
rotation- and scale-invariant fingerprint. (Ref: doc 02 §6.1.)

#### Scenario: Key is rotation- and scale-invariant
- **WHEN** the same 4 stars are presented at a different orientation and angular scale
- **THEN** the resulting edge-ratio pattern key is identical

#### Scenario: Bins follow the max-error formula
- **WHEN** `pattern_max_error = 0.001`
- **THEN** `pattern_bins = 250` (and `0.005 → 50`)

### Requirement: Pattern key hash and table index

The system SHALL pack a pattern key into a 64-bit positional code
`key_hash = Σ_m key[m]·pattern_bins^m`, and map it to a table index as
`(key_hash · _MAGIC_RAND) mod table_size` for quadratic-probe tables (`_MAGIC_RAND =
2654435761`), or `key_hash mod table_size` for linear-probe (prime-sized) tables. 64-bit
overflow wraps intentionally. (Ref: doc 02 §6.2.)

#### Scenario: Distinct keys get distinct 64-bit codes
- **WHEN** two different 5-tuples within `[0, pattern_bins)` are packed
- **THEN** their `key_hash` values differ

#### Scenario: Magic constant value
- **WHEN** the quadratic-probe index is computed
- **THEN** `_MAGIC_RAND` equals `2654435761` (`⌊2³²/φ⌋`)

### Requirement: Open-addressing probe sequence

The system SHALL resolve hash-table collisions by probing slots `i = (hash_index + offset(c))
mod table_size` for `c = 0, 1, 2, …`, where `offset(c) = c·c` (quadratic) or `c` (linear),
inserting at the first all-zero slot, and on lookup returning every occupied slot up to the
first empty one. (Ref: doc 02 §6.3.)

#### Scenario: Lookup returns the full probe chain
- **WHEN** several patterns collide into the same starting slot
- **THEN** lookup returns all occupied slots in probe order until the first empty slot

#### Scenario: Insertion mirrors lookup ordering
- **WHEN** a pattern is inserted under probing
- **THEN** a subsequent lookup of the same key visits that slot before the first empty slot

### Requirement: 16-bit key pre-filter

The system SHALL store, per pattern, the low 16 bits of its `key_hash` (`key_hash & 0xFFFF`) so
that lookup can cheaply discard probe-chain slots whose stored 16-bit hash differs from the
query's, before any vector math. (Ref: doc 02 §6.4.)

#### Scenario: Mismatched 16-bit hash is discarded
- **WHEN** a probed slot's stored 16-bit hash differs from the query `key_hash & 0xFFFF`
- **THEN** that slot is discarded before edge-ratio comparison

### Requirement: Centroid-distance pattern ordering

The system SHALL order the 4 stars of a pattern by ascending distance from the pattern centroid
(mean of the 4 unit vectors), so that the m-th image star corresponds to the m-th catalog star
for attitude solving. (Ref: doc 02 §7.)

#### Scenario: Deterministic correspondence
- **WHEN** an image pattern and its matching catalog pattern are each ordered by centroid
  distance
- **THEN** star `m` of one corresponds to star `m` of the other

### Requirement: FOV estimation from a matched pattern

The system SHALL estimate FOV from a matched pattern either by scaling a supplied estimate
(`fov = catalog_largest_edge / image_largest_edge · fov_initial`) or, absent an estimate, by
solving focal length from the largest pixel distance and the catalog largest angle
(`f = largest_pixel_dist / 2 / tan(catalog_largest_edge/2)`, `fov = 2·arctan((width/2)/f)`). The
diagonal FOV used to gather nearby stars is `fov·√(w²+h²)/w`. (Ref: doc 02 §8.)

#### Scenario: Scale a supplied estimate
- **WHEN** a `fov_estimate` is provided and a catalog pattern matches
- **THEN** the refined `fov` equals `catalog_largest_edge/image_largest_edge · fov_estimate`

#### Scenario: Diagonal FOV relation
- **WHEN** a diagonal FOV is needed
- **THEN** it equals `fov·√(width²+height²)/width`

### Requirement: Fine FOV and distortion refinement

The system SHALL refine FOV after a match: with no distortion, `fov *= mean(angle_catalog /
angle_camera)` over matched pairs; with distortion, least-squares solve for focal length `f` and
`k` from rows `A = [t, r³]`, `b = [r]` (per matched star), then `f /= (1−k)`,
`fov = 2·arctan(1/f)`. (Ref: doc 02 §8.1.)

#### Scenario: No-distortion FOV refinement
- **WHEN** distortion is not estimated
- **THEN** `fov` is scaled by the mean ratio of catalog-to-camera matched angles

### Requirement: Binomial false-alarm test

The system SHALL accept or reject a candidate attitude by a binomial false-alarm probability,
not a fixed match count: with `n` extracted centroids, `Nc` projected nearby catalog stars, `m`
matches, and `match_radius` (fraction of width), compute `prob_single = Nc·match_radius²` and
`prob_mismatch = binom.cdf(n − (m − 2), n, 1 − prob_single)`, accepting when
`prob_mismatch < match_threshold`. `match_threshold` MUST be divided by `num_patterns`
(Bonferroni); reported `Prob` MUST be `prob_mismatch · num_patterns`. The `−2` accounts for the
two degrees of freedom consumed fitting the attitude. (Ref: doc 02 §9.)

#### Scenario: More matches lower the false-alarm probability
- **WHEN** the match count `m` increases with `n`, `Nc`, `match_radius` fixed
- **THEN** `prob_mismatch` decreases

#### Scenario: Bonferroni correction over patterns tried
- **WHEN** the test runs against a database of `num_patterns` patterns
- **THEN** the effective threshold is `match_threshold / num_patterns` and the reported
  probability is `prob_mismatch · num_patterns`

### Requirement: Residual statistics

The system SHALL compute solution residuals by projecting matched image vectors to the sky and
comparing to matched catalog vectors: per-star `angle = 2·arcsin(‖diff‖/2)`,
`RMSE = rad2deg(sqrt(mean(angle²)))·3600` arcseconds, plus `P90E` (90th percentile) and `MAXE`
(maximum) in arcseconds. (Ref: doc 02 §10.)

#### Scenario: RMSE reported in arcseconds
- **WHEN** residuals are computed over matched stars
- **THEN** `RMSE`, `P90E`, and `MAXE` are returned in arcseconds, with `P90E ≤ MAXE`

### Requirement: Numerical conventions and reference parity

The system SHALL compute in f64 and MAY store database vectors as f32 (reference dtype), and
SHALL hold to the binding conventions: `(y,x)` pixels with `(0.5,0.5)` = top-left pixel center,
the `2·arcsin(d/2)` angle form, and `pattern_bins = round(1/(4·pattern_max_error))`. Outputs
SHALL match the Python reference (tetra3/cedar) within stated tolerances, enforced as the
correctness contract for this and dependent capabilities. (Ref: doc 02; project.md §5.)

#### Scenario: Parity with reference primitives
- **WHEN** angle, projection, distortion round-trip, attitude, pattern key, and false-alarm
  outputs are computed for captured reference inputs
- **THEN** each matches the cedar reference value within tolerance (angles/projections < 1e-9;
  identical integer pattern keys; false-alarm probability within 1e-6 relative)
