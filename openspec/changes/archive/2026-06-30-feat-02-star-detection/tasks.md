## 1. Crate & image abstraction

- [x] 1.1 Create the `ps-detect` crate depending on `ps-core` and `image` (0.25)
- [x] 1.2 Define the 8-bit `GrayImage` row-major access wrapper and `StarDescription` type

## 2. Statistics

- [x] 2.1 Implement histogram utilities (`stats_for_histogram`, `estimate_dark_level`, `remove_stars_from_histogram`)
- [x] 2.2 Implement `estimate_noise_from_image` (3 de-starred midline cuts, darkest stddev, floor 0.2)
- [x] 2.3 Implement `estimate_background_from_image_region`

## 3. Binning

- [x] 3.1 Implement 2×2 box-filter binning with histogram and the binning cascade (1/2/4/8) + `higher_res_image`
- [x] 3.2 Implement optional per-row dark-level normalization (bias 2.0)
- [x] 3.3 Provide a pluggable `set_binner` hook

## 4. Candidate scan

- [x] 4.1 Implement the per-row cache-line `row_min` coarse threshold
- [x] 4.2 Implement `gate_star_1d` (7-pixel integer gate, all qualification tests, tie-breaks)
- [x] 4.3 Implement hot-pixel `classify_pixel` (Dark/Hot/Bright) + `all_bright_are_hot` over the full-res block

## 5. Blobs & 2-D gate

- [x] 5.1 Implement `form_blobs_from_candidates` (vertical ±3 merge, union-find forwarding)
- [x] 5.2 Implement `gate_star_2d` (core/neighbors/margin/perimeter boxes, all 7 tests, perimeter noise inflation)

## 6. Centroid & brightness

- [x] 6.1 Implement `compute_peak_coord` / `peak_coord_1d` (axis projections + quadratic interpolation, degenerate cases)
- [x] 6.2 Implement `compute_brightness` (perimeter-subtracted inset sum, saturation, peak) and brightness-descending sort

## 7. Entry points & parity

- [x] 7.1 Wire `get_stars_from_image` end-to-end (binning, hot pixels, return_binned, input-coordinate scaling)
- [ ] 7.2 Implement `summarize_region_of_interest` (auto-exposure/focus helper)
- [x] 7.3 Add parity tests vs cedar-detect on `test_data` (±0.1 px, same ordering); benchmark ms/Mpx
