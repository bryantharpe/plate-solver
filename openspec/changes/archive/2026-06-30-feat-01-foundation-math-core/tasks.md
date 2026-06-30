## 1. Crate scaffolding

- [x] 1.1 Create the Cargo workspace and the `ps-core` crate (lib, no_std-friendly where practical)
- [x] 1.2 Add `nalgebra` dependency; set up the parity-test fixtures directory and harness skeleton

## 2. Coordinates & angles

- [x] 2.1 Implement `(RA,Dec) ↔ (x,y,z)` conversions with round-trip tests
- [x] 2.2 Implement `angle_from_distance` / `distance_from_angle` (`2·arcsin(d/2)`) with inversion tests

## 3. Projection & distortion

- [x] 3.1 Implement pixels→vectors (`compute_vectors`) with center/edge scenarios
- [x] 3.2 Implement vectors→pixels (`compute_centroids`) with in-frame `keep` filter and round-trip test
- [x] 3.3 Implement `undistort_centroids` (closed form) and `distort_centroids` (Newton–Raphson) with round-trip test

## 4. Attitude

- [x] 4.1 Implement Wahba/SVD `find_rotation_matrix` (`H=ImgᵀCat`, `R=U·Vᵀ`) with known-rotation test
- [x] 4.2 Implement `det(R)<0` reflection guard
- [x] 4.3 Implement RA/Dec/Roll extraction from `R`

## 5. Pattern key & hashing

- [x] 5.1 Implement edge-ratio key (6 edges → sort → normalize → quantize) with invariance tests
- [x] 5.2 Implement 64-bit `key_hash` packing and table index (`·_MAGIC_RAND mod` / `mod`)
- [x] 5.3 Implement open-addressing insert/lookup (quadratic & linear) and the 16-bit pre-filter
- [x] 5.4 Implement centroid-distance pattern ordering

## 6. FOV, false-alarm, residuals

- [x] 6.1 Implement FOV estimation + fine FOV/distortion least-squares refinement
- [x] 6.2 Implement the binomial false-alarm test (with `−2` DoF and `/num_patterns` correction)
- [x] 6.3 Implement residual statistics (RMSE/P90E/MAXE in arcseconds)

## 7. Parity

- [x] 7.1 Add parity fixtures captured from the Python reference and assert all primitives within tolerance
