## 1. Crate & API

- [x] 1.1 Create the `ps-solve` crate depending on `ps-core`, `ps-db`, and `ps-detect`
- [x] 1.2 Define `Solution`/`SolveStatus` types and the `solve_from_centroids` / `solve_from_image` signatures

## 2. Preparation

- [x] 2.1 Implement fov_initial, `match_threshold/=num_patterns`, centroid count limit
- [x] 2.2 Implement known-distortion undistort and the deferred-`k` path
- [x] 2.3 Implement centroid cluster-busting (pixel separation rule, KD-tree) and the `TOO_FEW` guard
- [x] 2.4 Precompute centroid vectors once

## 3. Candidate generation

- [x] 3.1 Implement breadth-first image-pattern iteration with `solve_timeout` + cancellation checks
- [x] 3.2 Implement the image-pattern key, tolerance band, and nearest-first candidate-key enumeration
- [x] 3.3 Wire `ps-db` lookup + 16-bit/largest-edge pre-filters + edge-ratio band → valid patterns

## 4. Verification

- [x] 4.1 Implement coarse FOV (estimate-scaled and focal-length variants)
- [x] 4.2 Pair stars by centroid order; SVD attitude; reject `det(R)<0`
- [x] 4.3 Gather diagonal-FOV nearby stars, derotate+project, trim to `2·num_centroids`
- [x] 4.4 Unique 1:1 matching within `match_radius·width`
- [x] 4.5 Binomial false-alarm acceptance test

## 5. Refinement & outputs

- [x] 5.1 Re-fit attitude over all matches; extract RA/Dec/Roll
- [x] 5.2 Refine FOV (+ distortion `k` least squares); residuals RMSE/P90E/MAXE
- [x] 5.3 Assemble the solution dict + optional outputs (matches, catalog, rotation, target pixel/sky)
- [x] 5.4 Status codes and failure paths (MATCH_FOUND/NO_MATCH/TIMEOUT/CANCELLED/TOO_FEW)

## 6. Parity

- [x] 6.1 Parity tests vs cedar on reference images (RA/Dec arcsec, identical matched catalog IDs)
