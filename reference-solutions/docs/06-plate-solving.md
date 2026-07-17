# 06 — Plate Solving (Identification & Attitude Recovery)

This is the core of the system: given centroids + an approximate FOV, find where the camera
points. It implements the two-stage geometric-hash match from doc 01 §2 using the math of
doc 02. Both tetra3 and cedar-solve are covered; the structure is the same, and the
differences are flagged. Functions: `solve_from_image` (a thin wrapper) and
`solve_from_centroids` (the engine).

```
centroids (y,x, brightest first)  +  fov_estimate
        │
        ▼
[A] PREP: convert to vectors, (cedar) cluster-bust, limit count, apply known distortion
        │
        ▼
[B] for each 4-star IMAGE PATTERN (brightest-first combinations):
        compute edge-ratio key + tolerance band
        for each candidate KEY in the band (cedar: sorted nearest-first):
            hash → probe table → gather catalog patterns
            (cedar) 16-bit key pre-filter; (both, if available) largest-edge/FOV pre-filter
            for each catalog pattern whose edge ratios fall in the band:
        ┌────────────────────────────────────────────────────────────────┐
        │ [C] VERIFY:                                                      │
        │   coarse FOV from largest-edge ratio                            │
        │   pair image⇄catalog stars (centroid-distance order)            │
        │   attitude R = Wahba/SVD;  (cedar) reject det(R)<0              │
        │   gather catalog stars in diagonal FOV; derotate; project       │
        │   match projected↔image centroids within match_radius           │
        │   false-alarm probability (binomial);  accept if < threshold    │
        └────────────────────────────────────────────────────────────────┘
        on accept ─► [D] REFINE & RETURN
        │
        ▼
[E] exhausted / timeout / cancelled  ─►  failure dict
```

---

## 1. `solve_from_image` (wrapper)

```python
solve_from_image(image, fov_estimate=None, fov_max_error=None, ..., **kwargs)
```

1. Read `(width, height)` from the PIL image.
2. `centroids = get_centroids_from_image(image, **kwargs)` (doc 03; or swap in cedar-detect
   centroids). Time it → `T_extract`.
3. `solution = solve_from_centroids(centroids, (height, width), <forwarded args>)`.
4. Attach `T_extract`; return (handling the tuple form when the centroider returned
   moments/images).

Everything substantive is in `solve_from_centroids`.

---

## 2. `solve_from_centroids` — inputs & defaults

```python
# tetra3
solve_from_centroids(star_centroids, size, fov_estimate=None, fov_max_error=None,
                     pattern_checking_stars=8, match_radius=.01, match_threshold=1e-3,
                     solve_timeout=None, target_pixel=None, distortion=0,
                     return_matches=False, return_visual=False)
# cedar
solve_from_centroids(star_centroids, size, fov_estimate=None, fov_max_error=None,
                     match_radius=.01, match_threshold=1e-5, solve_timeout=5000,
                     target_pixel=None, target_sky_coord=None, distortion=0,
                     return_matches=False, return_catalog=False, return_visual=False,
                     return_rotation_matrix=False, match_max_error=.002,
                     pattern_checking_stars=None)   # ignored in cedar
```

- `star_centroids` — `(N,2)` `(y,x)`, **brightest first** (essential — the search tries
  bright stars first).
- `size = (height, width)`.
- `fov_estimate` (deg) — strongly recommended. If `None`, start from the midpoint of the
  DB's `[min_fov, max_fov]`. `fov_max_error` (deg) bounds acceptable FOV deviation.
- `match_radius` — match tolerance, **fraction of image width** (default 0.01 → matches
  within `0.01·width` px).
- `match_threshold` — max accepted false-alarm probability. **Internally divided by
  `num_patterns`** (doc 02 §9). tetra3 `1e-3`; cedar `1e-5`.
- `match_max_error` (cedar) — pattern-key tolerance band half-width; clamped to be ≥ the
  DB's `pattern_max_error`. tetra3 uses the DB's `pattern_max_error` directly as `p_max_err`.
- `solve_timeout` (ms) — tetra3 default `None` (no timeout); cedar default `5000`.
- `distortion` — scalar `k` (known distortion to undistort) or `None` (estimate it). tetra3
  also accepted a `(min,max)` **range** to search; **cedar removed range support** (warns
  and treats as `None`).
- `target_pixel` — extra image pixels to also report RA/Dec for. cedar adds
  `target_sky_coord` (sky → image x/y).

---

## 3. [A] Preparation

1. `fov_initial` = `fov_estimate` (radians) or DB-range midpoint.
2. `match_threshold /= num_patterns` (Bonferroni; doc 02 §9). `num_patterns` is the stored
   count (cedar) or `pattern_catalog.shape[0] // 2` (tetra3).
3. **Centroid count limit**: keep at most `verification_stars_per_fov` centroids
   (tetra3: first `num_stars`; cedar: first `verification_stars_per_fov`, default 150). The
   solver works with the brightest of these.
4. **Distortion**: if a scalar `k` is given, `image_centroids_undist =
   _undistort_centroids(image_centroids, size, k)`; if `None`, use centroids as-is and plan
   to estimate `k` later; (tetra3 range mode: pre-undistort at several `k` values).
5. **(cedar) cluster-bust the centroids**: thin them with the same density rule as DB build,
   producing `pattern_centroids_inds` — the subset used to *form patterns* (matching still
   uses all kept centroids). Separation in pixels:
   ```
   sep_px = width · separation_for_density(fov_initial, verification_stars_per_fov) / fov_initial
   ```
   Greedy keep via a centroid KD-tree. This stops a tight cluster from generating only tiny,
   useless patterns. tetra3 has **no** centroid cluster-busting — it just uses the brightest
   `pattern_checking_stars`.
6. **(cedar) precompute all centroid vectors** once: `image_centroids_vectors =
   _compute_vectors(image_centroids_undist, size, fov_initial)`.
7. If `< pattern_size (4)` centroids → return failure with `status = TOO_FEW` (cedar).

---

## 4. [B] Iterating image patterns and candidate keys

### 4.1 Which 4-star image patterns to try

- **tetra3**: `itertools.combinations(range(min(N, pattern_checking_stars)), 4)` — all
  C(8,4)=70 patterns of the **8 brightest** centroids (default `pattern_checking_stars=8`).
- **cedar**: `breadth_first_combinations(pattern_centroids_inds, 4)` — over **all** cluster-
  busted centroids, but ordered so the brightest stars' combinations come first (doc on
  `breadth_first_combinations`). Potentially huge → relies on `solve_timeout`.

On each iteration check `solve_timeout` (→ `status=TIMEOUT`) and, in cedar, the
`_cancelled` flag (→ `status=CANCELLED`; see `cancel_solve`).

### 4.2 Image pattern key + tolerance band

For the 4 chosen centroids' vectors:

```
edge_angles_sorted = sort(2·asin(½·pdist(vectors)))     # 6 edges
image_pattern_largest_edge = edge_angles_sorted[-1]
image_pattern  = edge_angles_sorted[:-1] / largest      # 5 ratios
ratio_min = image_pattern − p_max_err                   # tolerance band (p_max_err = match_max_error / DB error)
ratio_max = image_pattern + p_max_err
image_pattern_key = int(image_pattern · p_bins)         # the nominal 5-int key
```

The band accounts for centroiding/FOV error: the true catalog ratios may differ by up to
`p_max_err`.

### 4.3 Enumerate candidate keys in the band

```
key_space_min = max(0,      ratio_min · p_bins)         # per-ratio bin range
key_space_max = min(p_bins, ratio_max · p_bins)
candidate keys = cartesian product of [min..max] over the 5 positions
```

- **tetra3**: sorts each key ascending, dedups, hashes all (unordered).
- **cedar**: tags each candidate key with its squared distance to `image_pattern_key` and
  **sorts nearest-first**, so the most-likely keys (closest to the measured pattern) are
  tried first — faster time-to-solution.

For each candidate key: `key_hash = _compute_pattern_key_hash(key, p_bins)`;
`hash_index = _pattern_key_hash_to_index(key_hash, table_size, linear_probe)`.

### 4.4 Gather catalog patterns for a key (`_get_all_patterns_for_index`, cedar)

1. `hash_match_inds = _get_table_indices_from_hash(hash_index, table, linear_probe)` — the
   probe chain (all occupied slots up to the first empty). If empty → skip.
2. **(cedar) 16-bit key pre-filter**: keep only slots where
   `pattern_key_hashes[slot] == key_hash & 0xFFFF` (discards hash-table collisions cheaply).
3. **largest-edge / FOV pre-filter** (when `pattern_largest_edge` exists and both
   `fov_estimate` and `fov_max_error` are set): for each candidate compute the FOV it would
   imply, `fov2 = largest_edge/1000 / image_pattern_largest_edge · fov_estimate`, and keep
   only `|fov2 − fov_estimate| < fov_max_error`.
4. For the survivors, fetch their 4 star vectors and compute the **6 catalog edge angles**
   (sorted): `catalog_pattern_edges`. Return `(edges, vectors)`.

5. **Edge-ratio band test**: `catalog_edge_ratios = edges[:, :-1] / edges[:, -1]`; keep
   catalog patterns whose every ratio lies strictly inside `(ratio_min, ratio_max)`. These
   are the **valid_patterns** that proceed to verification.

---

## 5. [C] Verification of one candidate (the heart)

For each valid catalog pattern:

1. **Coarse FOV** (doc 02 §8):
   - if `fov_estimate` given: `fov = catalog_largest_edge / image_pattern_largest_edge ·
     fov_initial`.
   - else: `f = image_pattern_largest_pixel_distance / 2 / tan(catalog_largest_edge/2)`;
     `fov = 2·arctan(width/2/f)` (caching the largest pixel distance).
   - (tetra3 with `fov_estimate` & `fov_max_error`) skip immediately if
     `|fov − fov_estimate| > fov_max_error`.
2. **Pair the stars**: recompute the 4 image pattern vectors at this `fov`, order them by
   distance from their centroid (doc 02 §7). Order the catalog pattern the same way (cedar's
   DB is presorted, so usually already ordered; if `presort_patterns` is false, sort here).
   Now image star `m` ↔ catalog star `m`.
3. **Attitude**: `R = _find_rotation_matrix(image_pattern_vectors, catalog_pattern_vectors)`
   (Wahba/SVD, doc 02 §5). **(cedar)** if `det(R) < 0` → reject (reflection, false positive).
4. **Nearby catalog stars**: `image_center_vector = R[0, :]`; gather catalog stars within
   the **diagonal** FOV radius:
   ```
   fov_diagonal = fov · √(w²+h²)/w
   nearby = _get_nearby_*stars(image_center_vector, fov_diagonal/2)
   ```
   (tetra3 `_get_nearby_stars`: cartesian-cube prefilter then dot-product test; cedar
   `_get_nearby_catalog_stars`: KD-tree `query_ball_point`, returned sorted = brightest
   first.)
5. **Project**: derotate nearby catalog vectors into the camera frame (`R · v`), project to
   pixels via `_compute_centroids` (keep only in-frame). Trim to the brightest:
   - tetra3: keep `len(image_centroids)`.
   - cedar: keep `2·num_centroids` (a 2× "fudge factor" — image brightness ranking may not
     match catalog ranking, so keep extra to improve match count).
6. **Match**: `_find_centroid_matches(image_centroids_undist, nearby_centroids,
   width·match_radius)` — pairs within radius, made **unique 1:1** (via `np.unique` on each
   column). `m = num_star_matches`.
7. **False-alarm test** (doc 02 §9):
   ```
   prob_single = num_nearby · match_radius²
   prob_mismatch = binom.cdf(n − (m − 2), n, 1 − prob_single)
   accept if prob_mismatch < match_threshold
   ```
   If not accepted, continue to the next candidate.

---

## 6. [D] Refinement & outputs (on accept)

1. **Re-fit attitude with ALL matches** (not just the 4 pattern stars):
   `matched_image_vectors` ↔ `matched_catalog_vectors`, `R = _find_rotation_matrix(...)`.
   This is the accurate attitude.
2. **Extract RA/Dec/Roll** from `R` (doc 02 §5.1).
3. **Refine FOV (and distortion)**:
   - `distortion is None`: `fov *= mean(angle_catalog / angle_camera)` over matched pairs;
     `k = None`.
   - `distortion` numeric: least-squares solve for focal length `f` and `k` (doc 02 §8.1),
     `fov = 2·arctan(1/f)`; re-undistort centroids with the solved `k`.
   (tetra3 had additional logic for the distortion-range case; cedar dropped ranges.)
4. **Residuals** (doc 02 §10): project matched image vectors to sky, compare to catalog
   vectors → `RMSE` (arcsec). cedar also computes `P90E` (90th-pct) and `MAXE` (max).
5. **Build the solution dict** and return immediately (first acceptable match wins):

   Common keys: `RA, Dec, Roll, FOV` (deg), `distortion` (the `k`, or `None`), `RMSE`,
   `Matches`, `Prob` (= `prob_mismatch · num_patterns`, the corrected probability),
   `epoch_equinox`, `epoch_proper_motion`, `T_solve` (ms). cedar adds `P90E`, `MAXE`, and a
   `status` (`MATCH_FOUND`).

   Optional, on request:
   - `target_pixel` → `RA_target`, `Dec_target` (undistort → vector → `Rᵀ` → RA/Dec).
   - `target_sky_coord` (cedar) → `x_target`, `y_target` (vector → `R` → project → distort;
     `None` for points outside the FOV).
   - `return_matches` → `matched_centroids` (y,x), `matched_stars` (RA,Dec,mag),
     `matched_catID`, and (cedar) `pattern_centroids`.
   - `return_catalog` (cedar) → `catalog_stars` list of `(RA, Dec, mag, y, x)` for all nearby
     catalog stars (distorted back to image space).
   - `return_visual` → a PIL RGB overlay: white = input centroids, dark-orange = undistorted
     centroids (large outline on the 4 pattern stars), green = matched-with-distortion
     centroids and green circles on matched catalog stars, red circles on unmatched catalog
     stars.
   - `return_rotation_matrix` (cedar) → `rotation_matrix` (3×3 list).

---

## 7. [E] Failure

If all image patterns are exhausted (or timeout/cancel), return a dict with all solution
fields `None` except `T_solve` (and cedar's `status` ∈ {`NO_MATCH`, `TIMEOUT`, `CANCELLED`,
`TOO_FEW`}). tetra3 returns the same minus `status`.

---

## 8. Key helper methods

- `_get_nearby_stars(vector, radius)` (tetra3): cartesian bounding-cube prefilter on
  `star_table[:,2:5]` (within `2·sin(radius/2)` per axis) then exact `dot > cos(radius)`.
- `_get_nearby_catalog_stars(vector, radius)` (cedar): `star_kd_tree.query_ball_point(vector,
  2·sin(radius/2))`, returned **sorted** (brightest first, since `star_table` is brightness-
  sorted).
- `_get_all_patterns_for_index(...)` (cedar): the candidate-gathering + pre-filters of §4.4.
- `_get_matched_star_data(centroids, star_indices)`: assembles `matched_centroids`,
  `matched_stars` (deg), `matched_catID`.
- `cancel_solve()` (cedar): sets `_cancelled` so a running/next solve aborts with
  `CANCELLED`.

---

## 9. Performance characteristics

- Quoted: ~10 ms/solve (excluding extraction), ~10 arcsec (50 µrad) accuracy, pure
  lost-in-space (no prior).
- The pre-filters (largest-edge/FOV, 16-bit key hash) and nearest-first key ordering exist
  to cut the candidate count before the expensive per-pattern SVD + projection + match.
- Providing `fov_estimate` and `fov_max_error` is the single biggest speedup: it enables the
  instant largest-edge FOV filter and bounds the candidate set.
- cedar's `solve_timeout` (default 5 s) bounds the breadth-first search, which over all
  centroid combinations could otherwise be enormous.

---

## 10. Worked example (defaults, FOV ≈ 11°)

```python
import tetra3
t3 = tetra3.Tetra3()                       # loads default_database
result = t3.solve_from_image(image, fov_estimate=11, fov_max_error=0.5, max_area=300)
# -> {'RA': 271.9, 'Dec': -23.9, 'Roll': 187.1, 'FOV': 11.40, 'distortion': 0.0,
#     'RMSE': 12.3, 'Matches': 27, 'Prob': 3e-14, 'T_solve': 9.8, 'T_extract': 25.1, ...}
```

Using cedar-detect for extraction instead: get `(y,x)` centroids over gRPC (doc 07), then
`t3.solve_from_centroids(centroids, (height, width), fov_estimate=11)`.

---

## 11. Rebuild checklist (solver)

1. Prep: vectorize centroids (`_compute_vectors`), apply known distortion, limit to
   `verification_stars_per_fov`, (cedar) cluster-bust to get pattern centroids, scale
   `match_threshold /= num_patterns`.
2. Outer loop over 4-star image patterns (brightest-first; cedar breadth-first + timeout +
   cancel).
3. Pattern key + tolerance band → candidate keys (cedar: nearest-first sorted).
4. Hash → probe chain → (cedar 16-bit + largest-edge/FOV) pre-filters → edge-ratio band test
   → `valid_patterns`.
5. Per candidate: coarse FOV, centroid-distance pairing, SVD attitude (cedar reject
   `det<0`), gather diagonal-FOV catalog stars, derotate+project, unique 1:1 match,
   binomial false-alarm accept test.
6. On accept: re-fit attitude over all matches, extract RA/Dec/Roll, refine FOV (+`k` via
   least squares), residuals, assemble outputs.
7. Failure/timeout/cancel handling with status codes.
