# 04 — Star Detection (cedar-detect, Rust)

cedar-detect is a from-scratch, high-performance star detector. Same job as doc 03 (image →
brightest-first `(y, x)` centroids) but designed for speed and robustness on real cameras:
localized thresholding, adaptive noise estimation, hot-pixel rejection, trail rejection,
and tolerance of bright interlopers (moon, streetlights). On a Raspberry Pi 4B it runs
<10 ms per megapixel.

Source: `cedar-detect/src/algorithm.rs` (core), `image_funcs.rs` (binning),
`histogram_funcs.rs` (statistics). Images are **8-bit grayscale only**.

The key design move: spend almost all the time in **one efficient raster pass** that emits a
few hundred/thousand *candidates*, then apply expensive 2-D scrutiny only to those.

---

## 1. Public entry points

```rust
estimate_noise_from_image(image) -> f64
estimate_background_from_image_region(image, roi) -> (mean, stddev)
get_stars_from_image(image, noise_estimate, sigma, normalize_rows,
                     binning, detect_hot_pixels, return_binned_image)
    -> (Vec<StarDescription>, hot_pixel_count, Option<binned_image>, histogram[256])
summarize_region_of_interest(image, roi, noise_estimate, sigma) -> RegionOfInterestSummary
```

`StarDescription` fields: `centroid_x`, `centroid_y` (f64, pixel space of the input image,
`(0.5,0.5)`=center of top-left pixel), `peak_value` (u8, brightest pixel, not background
subtracted), `brightness` (f64, background-subtracted region sum), `num_saturated` (u16).

**Parameters that matter:**

- `sigma` — significance threshold; a pixel must exceed background by `sigma·noise`.
  Typical **5–10** (Python clients default **8.0**). Lower → more stars, more false
  positives.
- `binning` — 1, 2, 4, or 8. Detect on a binned copy (for oversampled/soft images) while
  reporting centroids in input-image coordinates; centroiding is done on the
  one-level-less-binned image for sub-pixel accuracy.
- `detect_hot_pixels` — classify & reject isolated hot pixels (input should be full-res).
- `normalize_rows` — equalize per-row dark levels (a fix for the IMX296 sensor); only used
  when binning.

The whole image is scanned **except the 3 leftmost and 3 rightmost columns** (the 7-pixel
horizontal gate needs 3 pixels of context on each side).

---

## 2. Pipeline overview

```
                        ┌─ estimate_noise_from_image (darkest of 3 midline cuts)
   image (8-bit) ──┬───►│
                   │    └─ (optional) bin 2x/4x/8x  ──► detect_image, higher_res_image
                   │
                   ├──► scan_image_for_candidates  (per-row: row-min threshold → 7px 1D gate)
                   │         │  emits CandidateFrom1D{x,y} in raster order
                   │         ▼
                   ├──► hot-pixel filter (all_bright_are_hot / classify_pixel)  [if detect_hot_pixels]
                   │         ▼
                   ├──► form_blobs_from_candidates (merge vertically-adjacent candidates)
                   │         ▼
                   └──► gate_star_2d per blob (core/neighbors/margin/perimeter box tests)
                             │  → centroid (compute_peak_coord) + brightness (compute_brightness)
                             ▼
                        Vec<StarDescription>, sorted by brightness descending
```

---

## 3. Noise estimation — `estimate_noise_from_image`

Goal: a robust RMS noise estimate that isn't fooled by a bright region (moon/streetlamp).

1. `cut_size = min(50, width/4)`.
2. Take **three horizontal cuts** (1 pixel tall, `cut_size` wide) centered at the vertical
   midline, at `x ≈ width/4`, `width/2`, `3·width/4`. (Horizontal cuts within one row avoid
   between-row offset noise, again an IMX296 consideration.)
3. For each cut compute statistics via `stats_for_roi` (which **removes stars** from the
   histogram first — see §9), giving `(mean, median, stddev)`.
4. **Pick the darkest cut by mean** and return its `stddev`. (Darkest = least likely to
   contain an interloper.)

A `noise_floor = 0.2` is applied later (`noise = max(noise, 0.2)`): if the background was
"crushed to black", use a minimum noise so thresholds don't collapse to zero.

---

## 4. Binning — `image_funcs.rs`

`bin_and_histogram_2x2(image, normalize_rows)` → `{binned, histogram[256]}`:

- **2×2 box filter**: each output pixel = `(p1+p2+p3+p4)/4` (integer average) of the 2×2
  block. Output dims `width/2 × height/2` (even bounds enforced via `& !1`).
- Builds a histogram of the binned pixel values along the way.
- If `normalize_rows`: first run `apply_row_normalization` — per row, build a histogram,
  estimate that row's dark level (`estimate_dark_level`, §9), and shift the row so its dark
  level equals a fixed `bias = 2.0` (clamped to `0..255`). This removes per-row offset
  banding before binning.

`bin_2x2(image)` is the same box filter without the histogram. Both are pluggable via
`set_binner` (a process can install SIMD/accelerated versions); the defaults above are used
otherwise.

**Binning cascade in `get_stars_from_image`:**

- `binning == 1`: detect on the input; `higher_res_image = input`.
- `binning == 2`: `detect_image = bin2x2(input)`, `higher_res_image = input`.
- `binning == 4`: `detect_image = bin2x2(bin2x2(input))`, `higher_res_image = bin2x2(input)`.
- `binning == 8`: one more level; `higher_res_image` is the 4× image.

So detection runs on the most-binned image (better SNR for soft/oversampled stars), while
centroiding uses the **one-level-less-binned** image and the result is scaled back up by
`binning/2` to reach input-image coordinates. When binned, the noise estimate is recomputed
on `detect_image`.

`max_size` (max blob extent) `= width/100` for `binning==1`, else `width/100/binning + 1`.

---

## 5. The 1-D row scan — `scan_image_for_candidates`

This is the hot loop. For each row:

1. **Estimate the row minimum cheaply**: sample every 64th pixel (one per cache line),
   take the min → `row_min`. Set a coarse `threshold = row_min + sigma_noise_2/2`
   (saturating add). Most pixels are below this and skip the expensive gate.
2. For each `center_x` in `3 .. width−3` with `pixel ≥ threshold`, extract the **7-pixel
   horizontal gate** `[center_x−3 .. center_x+3]` and run `gate_star_1d`.
3. (When `compute_histogram`) accumulate a 256-bin histogram of all scanned pixels.

Precomputed integer thresholds (so the gate uses cheap integer math):

```
sigma_noise_2 = max( round(2·sigma·noise), 2 )      # 2× the sigma·noise value
sigma_noise_3 = max( round(3·sigma·noise), 3 )      # 3× the sigma·noise value
```

### `gate_star_1d(gate, sigma_noise_2, sigma_noise_3) -> Candidate | Uninteresting`

Label the 7 pixels `|lb lm l C r rm rb|` (left-border, left-margin, left, **Center**,
right, right-margin, right-border). For the center to be a star candidate, **all** must
hold (checked in this cost-optimized order):

1. **Significantly above background** (eliminates the vast majority first):
   `est_background_2 = lb + rb`; require `2·C − est_background_2 ≥ sigma_noise_2`.
   (Border pixels estimate local sky; the `×2` keeps everything in integers.)
2. **Local maximum vs immediate neighbors**: `l ≤ C` and `C ≥ r`.
3. **Strictly brighter than margins**: `lm < C` and `C > rm`.
4. **Tie-breaking** so a flat-topped star is claimed by exactly one center:
   - if `l == C` and `lm > r`: reject (the left pixel will be its own candidate's center).
   - if `C == r` and `l ≤ rm`: reject (the right pixel will be the center).
5. **Uniform background**: `|lb − rb| ≤ sigma_noise_3` (rejects edges/gradients where the
   two borders differ too much — e.g. the lunar terminator).

The two **margin** pixels (`lm`, `rm`) exist so a slightly defocused star (spread across
`l C r`) still passes; a bare 5-pixel window would be too tight. Output is just
`CandidateFrom1D{x, y}` — one per qualifying center, in raster order.

---

## 6. Hot-pixel rejection — `all_bright_are_hot` / `classify_pixel`

If `detect_hot_pixels`, each 1-D candidate is checked against the **full-resolution** image
before blob formation. For every full-res pixel backing the (possibly binned) candidate
location (a `binning × binning` block, each examined with its own 7-pixel gate),
`classify_pixel` returns one of:

```
Dark   : 2·C − (lb+rb) < sigma_noise_2                         → not bright at all
Hot    : 4·((l+r) − (lb+rb)) ≤ (2C − (lb+rb)) / 2              → bright but isolated (no neighbor support)
                                                                 (reported value = (l+r)/2, i.e. interpolated)
Bright : otherwise                                            → bright with neighbor support (real-star-like)
```

The Hot test says: a real star deposits flux into its neighbors, so the left+right neighbor
excess must be a meaningful fraction (here ≥ 1/8) of the center excess. An isolated spike
(no neighbor support) is a hot pixel. `all_bright_are_hot` returns **true** only if *none*
of the backing full-res pixels are `Bright` (i.e. every bright contributor is an isolated
hot pixel). Such candidates are dropped and counted in `hot_pixel_count`. Otherwise the
candidate proceeds.

> Requirement: for hot-pixel detection to work, pass the **full-resolution** image (so
> isolated spikes are distinguishable from spread starlight). If stars are focused to a
> single pixel they can be misclassified as hot — defocus slightly so each star spreads to
> a small peak + neighbors.

---

## 7. Blob formation — `form_blobs_from_candidates`

The 1-D scan can flag the same (vertically-extended) star on several adjacent rows. Blob
formation merges connected candidates:

1. Bucket candidates by row (`labeled_candidates_by_row[y]`). Each starts as its own
   singleton blob.
2. Scanning rows top→bottom, for each candidate look at the previous row's candidates; if
   one is within **±3 in x**, merge the previous blob into the current one. Within a row,
   candidates are never adjacent (the 1-D gate guarantees one center per peak), so only
   vertical adjacency is checked.
3. Merging follows `recipient_blob` forwarding pointers (union-find-like): a donor whose
   candidates were already merged elsewhere is followed to its recipient.
4. Return the non-empty blobs.

`±3` in x reflects the gate width tolerance, so a slightly slanted/extended star stays one
blob.

---

## 8. The 2-D gate — `gate_star_2d`

Run on each blob (only hundreds/thousands of these, so it can be thorough). Define four
concentric boxes around the blob's bounding box (the **core**):

```
core      = bounding box of all blob candidate coords
neighbors = core expanded by 1 px on all sides
margin    = core expanded by 2 px
perimeter = core expanded by 3 px       (the perimeter ring = the sky background sample)
```

Tests (reject the blob — return `None` — on any failure):

1. **Size**: `core_width ≤ max_size` and `core_height ≤ max_size` (else a bright bleeding
   blob or non-star structure). And the perimeter must not run off the image edge
   (`core ± 3` within bounds).
2. **Inner-core brightness** (only if core ≥ 3×3): `core_mean ≥ outer_core_mean`
   (perimeter ring of the core). Catches ring/arc shapes with dark interiors (e.g. a lit
   crater rim) that aren't condensed stars.
3. **Core ≥ neighbors**: `core_mean ≥ neighbor_mean` (neighbors ring, **excluding corners**).
4. **Core > margin**: `core_mean > margin_mean` (the margin ring).
5. **Background & local noise** from the **perimeter** ring:
   - `background_est = mean(perimeter)`.
   - `perimeter_stddev = RMS(perimeter − background_est)`; `max_noise = max(noise,
     perimeter_stddev)` — in clutter (lit foreground) the perimeter is noisy, raising the
     bar and suppressing spurious detections.
6. **Uniform perimeter**: `perimeter_max − perimeter_min ≤ 3·sigma·noise` (rejects blobs
   straddling a brightness edge).
7. **Significance**: `core_mean − background_est ≥ sigma·max_noise` (the actual
   "is it a star" test, now with the locally-inflated noise).

If all pass, the blob is a star. Compute centroid and brightness:

- **binning == 1**: centroid via `compute_peak_coord(image, margin)`, brightness via
  `compute_brightness(image, margin)`.
- **binned**: translate the `margin` box into `higher_res_image` space (×2), centroid and
  measure brightness there, then scale the centroid by `binning/2` back to input-image
  coordinates.

Final centroid is `(peak_x + 0.5, peak_y + 0.5)` (pixel-center convention).

### 8.1 Sub-pixel centroid — `compute_peak_coord` / `peak_coord_1d`

Project the box onto its x- and y-axes (sum columns → `horizontal_projection`, sum rows →
`vertical_projection`), find the peak of each 1-D projection independently, refine to
sub-pixel by **quadratic interpolation** of the peak and its two neighbors:

```
p = 0.5·(a − c) / (a − 2b + c)          # a,b,c = left, peak, right projection values; p ∈ [−0.5, 0.5]
coord = peak_index + p
```

(Reference: Smith, "Quadratic Interpolation of Spectral Peaks".) Degenerate cases: a run of
equal-value pixels → midpoint of the run; a peak at the box edge → the edge index
(no interpolation).

### 8.2 Brightness — `compute_brightness`

```
background_est = mean(perimeter ring of bounding_box)
inset          = bounding_box shrunk by 1 px      # the inner pixels
brightness     = Σ over inset of (pixel − background_est)   (clamped to ≥ 0)
num_saturated  = count of pixels == 255 in inset
peak_value     = max pixel in inset
```

Stars are finally **sorted by `brightness` descending** — this brightest-first order is
what the plate solver relies on.

---

## 9. Histogram utilities — `histogram_funcs.rs`

- `stats_for_histogram(hist) -> {mean, median, stddev}` — moments over a value histogram.
- `estimate_dark_level(hist, npoints) -> f32` — **mean of the darkest 1%** of pixels (falls
  back to the lowest non-zero bin if 1% rounds to 0). Used for row normalization.
- `get_level_for_fraction(hist, fraction)` — value at a cumulative fraction (percentile).
- `average_top_values(hist, n)` — mean of the `n` brightest entries (≥1).
- `remove_stars_from_histogram(hist, sigma)` — the de-starring used by `stats_for_roi`:
  1. Copy the histogram, sloppily trim the brightest 10% (`trim_histogram` to keep 90%).
  2. Compute mean/stddev of that trimmed copy.
  3. `star_cutoff = mean + sigma·max(stddev, 1)`; **zero out all bins ≥ cutoff** in the
     original histogram. What remains is the background-only distribution.

`stats_for_roi(image, roi)` builds the ROI histogram, calls `remove_stars_from_histogram`
with `sigma = 8.0`, then `stats_for_histogram`. This is how noise and background are
measured without star contamination.

---

## 10. Region summary (auto-exposure / focus helper) — `summarize_region_of_interest`

Not part of solving, but provided for applications. Slides the 7-pixel `classify_pixel`
gate over an ROI to find the peak (hot pixels excluded, ties broken toward image center),
returns a 256-bin histogram (for auto-exposure), a sub-pixel peak location (for focusing,
via the same quadratic centroid), and the mean of a 3×3 box at the peak (`peak_value`). The
ROI must exclude the 3 outer rows/columns.

---

## 11. Caveats (designed-in limitations)

- Detects **only condensed star-like spots** — not a general source extractor.
- **Crowding**: closely-spaced stars, a star next to a hot pixel, or image-shake "doubled"
  stars usually fail to detect (acceptable, even desirable, for plate solving).
- **8-bit only**; convert higher bit-depth first (and consider centroiding in the original
  high-bit image yourself for max accuracy).
- Tolerates **≤ ~1 pixel of motion blur**; overexposed bright stars that bleed too far are
  rejected (the `max_size` gate).
- Optical aberrations near wide-field corners can extend stars enough to be missed.
- Reports raw pixel `(x, y)`; **lens distortion correction is the solver's job** (doc 02
  §4, doc 06).

---

## 12. Rebuild checklist (cedar-detect)

1. 8-bit grayscale `GrayImage` abstraction with raw row-major access.
2. `estimate_noise_from_image`: 3 midline cuts, de-star each, take darkest stddev; floor 0.2.
3. Optional 2×/4×/8× binning cascade with `higher_res_image` retained; per-row dark-level
   normalization option.
4. `scan_image_for_candidates`: per-row cache-line min → coarse threshold → 7-pixel
   `gate_star_1d` with integer `sigma_noise_2`/`sigma_noise_3`. Skip 3 edge columns.
5. Hot-pixel `classify_pixel` (Dark/Bright/Hot) + `all_bright_are_hot` over full-res block.
6. `form_blobs_from_candidates`: vertical-adjacency merge (±3 x), union-find forwarding.
7. `gate_star_2d`: core/neighbors/margin/perimeter boxes, 7 tests, perimeter-based
   background + local noise inflation.
8. `compute_peak_coord` (axis projections + quadratic interpolation) and
   `compute_brightness` (perimeter-background-subtracted inset sum).
9. Sort by brightness descending; return centroids in input-image coordinates
   (`+0.5` pixel-center, ×`binning/2` if binned).
