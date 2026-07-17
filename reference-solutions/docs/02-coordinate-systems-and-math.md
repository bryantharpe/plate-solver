# 02 — Coordinate Systems & Mathematical Toolbox

Everything in the solver rests on a small set of geometric primitives. This document
specifies each one exactly: conventions, formulas, and the source functions that
implement them. With this document you can re-derive every transform used in detection,
database generation, and solving.

All function names below refer to `tetra3.py` (identical math in `tetra3/` and
`cedar-solve/`, except where noted).

---

## 1. Coordinate conventions

### 1.1 Pixel / image coordinates

- A centroid is `(y, x)`: `y` = row index increasing **downward**, `x` = column index
  increasing **rightward**. Origin at top-left corner.
- `(0.5, 0.5)` is the **center of the top-left pixel**. So the integer pixel index of a
  centroid is `floor(coord)`. (This is why centroid computations add `+0.5`.)
- `size = (height, width)` in pixels throughout.
- The **image center** is `[height/2, width/2]`.

### 1.2 Camera-frame unit vectors `(i, j, k)`

A direction in the camera frame is a 3-vector `[i, j, k]`:

- `i` — **boresight**, pointing out along the optical axis (component index 0).
- `j` — image **x** (horizontal), index 1.
- `k` — image **y** (vertical), index 2.

### 1.3 Celestial-frame unit vectors

For a star at right ascension `RA` (α) and declination `Dec` (δ), both in radians:

```
x = cos(RA) · cos(Dec)
y = sin(RA) · cos(Dec)
z = sin(Dec)
```

This is the standard equatorial unit vector. The inverse:

```
RA  = atan2(y, x)         (mod 2π)
Dec = arcsin(z) = 90° − arccos(z)
```

`star_table` columns are `[RA, Dec, x, y, z, magnitude]` (RA/Dec radians).

---

## 2. Angular distance between unit vectors (used everywhere)

Given two unit vectors separated by **chord (Euclidean) distance** `d = ‖u − v‖`, the
**central angle** between them is:

```
angle = 2 · arcsin(d / 2)          # _angle_from_distance(d)  (cedar)
```

and the inverse:

```
d = 2 · sin(angle / 2)             # _distance_from_angle(angle)  (cedar)
```

This `2·arcsin(d/2)` form is used in preference to `arccos(u·v)` because it is numerically
well-conditioned for small angles (where `arccos` loses precision). tetra3 inlines
`2*np.arcsin(.5*pdist(...))`; cedar factors it into the two helpers above. Pattern edges,
residuals, and FOV math all use this.

> **Rebuild note:** be consistent. Pattern edge angles, catalog edge angles, and
> verification residuals must all use the same `2·arcsin(d/2)` convention or the
> tolerances won't line up.

---

## 3. Pinhole camera projection

A rectilinear (pinhole) lens with horizontal field of view `fov` (radians) and image
`width` maps between pixel coordinates and camera-frame unit vectors.

### 3.1 Pixels → vectors — `_compute_vectors(centroids, size, fov)`

```
scale_factor = tan(fov/2) / width * 2
img_center   = [height/2, width/2]
# For each centroid (y, x):
vec[2] (=k, image-y) and vec[1] (=j, image-x) come from (img_center − centroid)*scale_factor
vec[0] (=i, boresight) = 1
# then normalize vec to unit length
```

Precisely (note the reversed slice `[2:0:-1]` assigns `(y_offset → k, x_offset → j)`):

```
star_vectors[:, [2, 1]] = (img_center − centroid) · scale_factor      # k, j
star_vectors[:, 0]      = 1                                           # i (boresight)
star_vectors /= ‖star_vectors‖                                        # normalize rows
```

`scale_factor = 2·tan(fov/2)/width` is the per-pixel tangent increment: a pixel `width/2`
from center (i.e. the horizontal edge) maps to `tan(fov/2)`, as required.

### 3.2 Vectors → pixels — `_compute_centroids(vectors, size, fov)`

The inverse projection of (already derotated) camera-frame vectors back to pixels:

```
scale_factor = −width / 2 / tan(fov/2)
centroids = scale_factor · vectors[:, [2, 1]] / vectors[:, [0]]   # divide by boresight comp.
centroids += [height/2, width/2]
```

- tetra3's version takes a `trim` flag; when trimming it returns only centroids inside the
  image plus the kept indices.
- cedar's version **always** returns `(centroids, keep)` where `keep` are the indices with
  `0 < y < height` and `0 < x < width`.

A vector with non-positive boresight component (`i ≤ 0`, i.e. behind the camera) projects
nonsensically; the in-frame `keep` filter removes those.

---

## 4. Lens distortion model

A single-parameter radial distortion `k`, defined as the distortion **at `width/2` from
center**. `k < 0` = barrel; `k > 0` = pincushion. The relation between undistorted radius
`r_u` and distorted radius `r_d` (both as fractions where `width/2` ↔ 1):

```
r_u = r_d · (1 − k'·r_d²) / (1 − k)      where  k' = k · (2/width)²
```

The `1/(1−k)` normalization makes `k` exactly the fractional displacement at the
half-width radius. Two helpers:

### 4.1 Undistort — `_undistort_centroids(centroids, size, k)`

Closed form (one shot). Center the coords, scale radially, decenter:

```
centroids −= [height/2, width/2]
r_dist     = ‖centroids‖
scale      = (1 − k'·r_dist²) / (1 − k)        # cedar uses k' = k·(2/width)²
centroids *= scale
centroids += [height/2, width/2]
```

> **Subtle version difference.** The original tetra3 computes
> `scale = (1 − k·(‖c‖/width·2)²)/(1−k)` (i.e. it normalizes the radius to half-width
> *inside* the square). cedar instead pre-scales `k` to `k' = k·(2/width)²` and uses the
> raw pixel radius. The two are algebraically the same model; just don't mix the forms.

### 4.2 Distort — `_distort_centroids(centroids, size, k, tol=1e-6, maxiter=30)`

The forward map `r_u → r_d` has no closed form, so it's inverted with Newton–Raphson:

```
centroids −= [height/2, width/2]
r_undist   = ‖centroids‖                        # (cedar: raw radius; tetra3: /width*2)
r_dist     = r_undist.copy()                    # initial guess
repeat up to maxiter:
    r_undist_est = r_dist · (1 − k'·r_dist²)/(1 − k)
    dru_drd      = derivative of the above w.r.t. r_dist
    error        = r_undist − r_undist_est
    r_dist      += error / dru_drd
    stop when max|error| < tol
centroids *= r_dist / r_undist
centroids += [height/2, width/2]
```

> The two implementations differ in the derivative `dru_drd` (tetra3:
> `(1 − 3k·r²)/(1−k)` using the half-width-normalized radius; cedar:
> `(1 − 2k'·r)/(1−k)`). Newton converges regardless to the same fixed point of each
> version's own forward model; reproduce one model consistently.

Distortion is applied to centroids **before** projecting to vectors (undistort), and the
forward distortion is used to place catalog stars back into the distorted image (e.g. for
`target_sky_coord` and `return_catalog`).

---

## 5. Attitude determination (Wahba's problem) — `_find_rotation_matrix`

Given two ordered, corresponding sets of unit vectors — `image_vectors` (camera frame) and
`catalog_vectors` (celestial frame), `N×3` each — find the least-squares-optimal rotation
relating them:

```
H = image_vectorsᵀ · catalog_vectors        # 3×3 cross-covariance
U, S, Vᵀ = svd(H)
R = U · Vᵀ
```

This is the classic SVD solution to **Wahba's problem** (Kabsch/Markley). `R` rotates the
celestial vectors into the camera frame (and `Rᵀ` the reverse).

**Caveat — proper rotation:** the bare `U·Vᵀ` can be a reflection (`det = −1`). cedar
guards against this at solve time by **rejecting candidates with `det(R) < 0`** (a fast
false-positive filter). A fully general implementation would instead flip the sign of the
last column of `U` (multiply by `diag(1,1,sign(det(U·Vᵀ)))`); cedar opts to simply discard
such matches because a true match never produces a reflection. The pairing order
(section 7) is what makes the two vector sets correspond row-for-row.

### 5.1 Extracting RA / Dec / Roll from `R`

With `R` as built above (rows are the camera axes expressed in the celestial frame):

```
RA   = atan2(R[0,1], R[0,0])  mod 360°        # boresight azimuth
Dec  = atan2(R[0,2], ‖R[1:3, 2]‖)             # boresight elevation
Roll = atan2(R[1,2], R[2,2])  mod 360°        # rotation about boresight
```

- Row 0 of `R`, `R[0,:]`, is the **boresight unit vector in the celestial frame** — i.e.
  the image-center direction. `RA`/`Dec` above are its spherical coordinates.
- `Roll` is the celestial-north orientation about the boresight.
  - cedar documents Roll as: *rotation of celestial north relative to image "up" (toward
    y=0); 0 when north and up coincide; positive = north counter-clockwise from up.*

### 5.2 Mapping arbitrary pixels ↔ sky (cedar standalone helpers)

- `transform_to_celestial_coords(image_coords, w, h, fov, R, k)`: undistort → `_compute_vectors`
  → rotate by `Rᵀ` → `(RA, Dec)`.
- `transform_to_image_coords(celestial_coords, w, h, fov, R, k)`: `(RA,Dec)`→vector →
  rotate by `R` → `_compute_centroids` (keep in-FOV) → `_distort_centroids`.

These are exactly how the solver fills `RA_target/Dec_target` and `x_target/y_target`.

---

## 6. The pattern key (geometric hash) math

This is the fingerprint that makes content-addressable matching possible.

### 6.1 Edges, ratios, key

For a 4-star pattern with unit vectors `v0..v3`:

1. Compute all `C(4,2)=6` pairwise **edge angles**: `edge_ij = 2·arcsin(½‖vi − vj‖)`.
2. **Sort** the 6 edges ascending: `e[0] ≤ … ≤ e[5]`.
3. **Largest edge** `L = e[5]` (the normalizer; also stored for FOV checks).
4. **Edge ratios**: `ratio[m] = e[m] / L` for `m = 0..4` → **5 values in (0, 1]**.
5. **Quantize**: `key[m] = int(ratio[m] · pattern_bins)`. With `pattern_bins` bins, each
   ratio lands in `0 .. pattern_bins`.
6. The 5-tuple `key = (key[0], …, key[4])` is the **pattern key**.

`pattern_bins = round(1 / (4 · pattern_max_error))`. Defaults:
- tetra3: `pattern_max_error = 0.005` → `pattern_bins = 50`.
- cedar:  `pattern_max_error = 0.001` → `pattern_bins = 250`.

The `1/4` factor accounts for the worst-case error propagation in a normalized ratio (both
numerator and denominator carry measurement error), so a per-ratio quantization error of
`pattern_max_error` is achieved.

### 6.2 Key → table index (two-step hash)

Two functions (cedar splits what tetra3 calls `_key_to_index` into two):

**Step 1 — pack the key into a 64-bit integer** (`_compute_pattern_key_hash`):

```
key_hash = Σ_m  key[m] · pattern_bins^m            # base-pattern_bins positional encoding
```

This gives each distinct key a unique value in `[0, pattern_bins^5)`. Computed in
`uint64`; for `pattern_bins=250`, `250^5 ≈ 9.8e11` fits in 64 bits.

**Step 2 — map to a table index** (`_pattern_key_hash_to_index`):

```
# quadratic-probe table (default):
index = (key_hash · _MAGIC_RAND) mod table_size          # _MAGIC_RAND = 2654435761
# linear-probe table:
index = key_hash mod table_size
```

`_MAGIC_RAND = 2654435761 = ⌊2³²/φ⌋` (Knuth's multiplicative-hash constant) scrambles the
key so adjacent keys don't cluster in the table. (For linear probing, cedar skips the
multiply because the table size is prime, which already disperses keys; see doc 05 §
hashing.) Multiplication overflow is intentionally ignored (mod 2⁶⁴ wraparound).

### 6.3 Open-addressing the table

The table (`pattern_catalog`) stores at most one pattern per row, so collisions are
resolved by probing successive slots until an empty one (all-zero row) is found:

```
# _insert_at_index(pattern, hash_index, table, linear_probe):
for c = 0, 1, 2, …:
    i = (hash_index + (c if linear_probe else c·c)) mod table_size
    if table[i] is all zeros: table[i] = pattern; return i
```

Lookup mirrors insertion, returning **every** occupied slot in the probe sequence up to
the first empty one (those are the collision candidates):

```
# _get_table_indices_from_hash(hash_index, table, linear_probe):
found = []
for c = 0, 1, 2, …:
    i = (hash_index + (c if linear_probe else c·c)) mod table_size
    if table[i] is all zeros: return found
    else: found.append(i)
```

Because the table is sized larger than the number of patterns (≈2×, prime in cedar), probe
chains stay short. **Row 0 must never legitimately be all-zero** for a real pattern — this
works because pattern entries are star *indices* and the all-zero sentinel is reserved
(index 0 is the brightest star, which is handled so it doesn't produce a genuinely
all-zero row; in practice patterns contain ≥2 distinct nonzero indices).

> **Quadratic vs linear probing tradeoff (cedar):** quadratic probing assumes the whole
> table is in RAM (random access is cheap). Linear probing keeps a probe chain in
> contiguous memory — better when the table is memory-mapped / too big for RAM. cedar
> chooses table size `next_prime(2·N)` (quadratic) or `next_prime(3·N)` (linear), `N` =
> number of patterns.

### 6.4 The 16-bit key pre-filter (cedar)

cedar additionally stores `pattern_key_hashes[i] = key_hash & 0xFFFF` (low 16 bits). At
lookup time, after gathering probe-chain candidates, it keeps only those whose stored
16-bit hash equals the query's `key_hash & 0xFFFF`. This cheaply discards rows that share a
table slot but have a *different* pattern key (true hash-table collisions), before the
expensive per-pattern vector math.

---

## 7. Making two patterns correspond: the centroid-distance ordering

To run Wahba's problem you need the 4 image stars paired 1:1 with the 4 catalog stars. The
pattern key is order-independent (it sorts edges), so a separate, **deterministic ordering**
recovers correspondence:

```
centroid    = mean of the 4 pattern unit vectors
for each star: radius = ‖star_vector − centroid‖
order the 4 stars by ascending radius
```

Both the database (at build, "presort") and the solver order patterns this way, so the
`m`-th image star corresponds to the `m`-th catalog star. (If the DB wasn't presorted, the
solver sorts the catalog pattern at match time too.) This ordering is well-defined as long
as the 4 radii are distinct — which they generically are.

---

## 8. FOV estimation from a pattern

Two ways the solver turns a matched pattern into a FOV estimate:

**(a) Scale the supplied estimate** (fast; when `fov_estimate` is given):

```
fov = catalog_largest_edge / image_pattern_largest_edge · fov_initial
```

The largest edge is a direct angular-size proxy; the ratio rescales the user's FOV guess.

**(b) Camera projection** (when no estimate): solve for focal length from the largest
*pixel* distance and the catalog's largest *angle*:

```
f   = image_pattern_largest_pixel_distance / 2 / tan(catalog_largest_edge / 2)
fov = 2 · arctan( (width/2) / f )
```

### 8.1 Fine FOV (and distortion) refinement after a match

- **No distortion** (`k = None`): compare all matched mutual angles:
  `fov *= mean( angle_catalog / angle_camera )`.
- **With distortion**: least-squares solve for focal length `f` and distortion `k`
  simultaneously. For each matched star let `t` = tangent of its angle from boresight
  (from the derotated catalog vector: `‖derot[1:]‖ / derot[0]`) and `r` = its distorted
  pixel radius scaled to half-width. The distortion model `r_u = r_d(1 − k·r_d²)/(1−k)`
  rearranges to a linear system in `(f, k)`:

  ```
  A = [ t ,  r³ ] ,  b = [ r ]          # one row per matched star
  (f, k) = lstsq(A, b)
  f = f / (1 − k)                        # correct focal length to horizontal FOV
  fov = 2 · arctan(1 / f)                # f in units of width/2
  ```

The diagonal FOV used to gather nearby catalog stars is always
`fov_diagonal = fov · √(width² + height²) / width`.

---

## 9. The false-match probability test (the verification statistic)

After projecting nearby catalog stars and counting matches, decide accept/reject with a
binomial false-alarm probability — *not* a fixed match count.

Definitions for one candidate attitude:

- `num_extracted_stars` — image centroids considered (`n`).
- `num_nearby_catalog_stars` — catalog stars projected into the FOV (`Nc`).
- `num_star_matches` — how many projected catalog stars fell within `match_radius` of an
  image centroid (`m`).
- `match_radius` — match tolerance as a fraction of image width (default `0.01`).

```
prob_single_star_mismatch = Nc · match_radius²      # chance a random centroid "hits" some catalog star
prob_mismatch = binom.cdf( n − (m − 2),  n,  1 − prob_single_star_mismatch )
```

Interpretation:

- `prob_single_star_mismatch` ≈ the fraction of the image area covered by match disks
  around the `Nc` projected catalog stars — i.e. the probability that an *unrelated*
  centroid coincidentally lands on a catalog star.
- The binomial CDF is the probability of getting **at least** `m` matches purely by chance
  out of `n` centroids. The `−2` subtracts the **two degrees of freedom** consumed because
  the attitude itself was *fit* to the pattern (2 of the matches are "used up" defining the
  orientation), making the test conservative.
- **Accept** if `prob_mismatch < match_threshold`.

`match_threshold` is **divided by the number of patterns** in the database before use
(`match_threshold / num_patterns`), a Bonferroni-style correction for having tried many
patterns (controls the family-wise false-positive rate over the whole search). Reported
`Prob` is `prob_mismatch · num_patterns` (the corrected value). Defaults: tetra3
`match_threshold = 1e-3`; cedar `1e-5`.

> **Why this is the crux:** it lets the solver accept a match from as few as ~5–6
> coincident stars when the FOV is sparse, yet demand more in dense fields — automatically,
> from first principles. It is the single most important idea distinguishing a robust
> lost-in-space solver from a brittle nearest-neighbor matcher.

---

## 10. Residuals (solution quality)

After the final attitude, project matched image vectors to the sky (`Rᵀ`) and compare to
the matched catalog vectors:

```
d        = ‖final_match_vector − catalog_vector‖      # per matched star (chord)
angle    = 2·arcsin(d/2)
RMSE     = rad2deg( sqrt(mean(angle²)) ) · 3600        # arcseconds
P90E     = 90th-percentile angle in arcseconds          (cedar)
MAXE     = max angle in arcseconds                       (cedar)
```

tetra3 reports only `RMSE`; cedar adds `P90E` and `MAXE`. Typical good solutions: RMSE on
the order of arcseconds to tens of arcseconds.

---

## 11. Small number-theory helpers (cedar)

For sizing the (prime) hash table:

- `_is_prime(n)` — trial division by odd numbers up to `√n`.
- `_next_prime(n)` — next prime strictly greater than `n` (steps through odd numbers).

These pick a prime table size, which improves key dispersion (especially for the linear-
probe / `mod`-only index function).
