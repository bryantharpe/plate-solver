# 03 — Star Detection (tetra3 Python)

This is tetra3's built-in centroider, `get_centroids_from_image`, plus its helper
`crop_and_downsample_image`. It is a classic **threshold → connected-components → moments**
spot extractor. It is what `solve_from_image` calls by default in both `tetra3/` and
`cedar-solve/` (the two implementations are byte-for-byte identical apart from comment
whitespace). cedar-detect (doc 04) is a faster, more robust alternative for the same job.

Output contract: an `(N, 2)` array of `(y, x)` centroids, **sorted brightest-first**
(by integrated pixel sum). `(0.5, 0.5)` = center of the top-left pixel.

---

## 1. Signature and parameters

```python
get_centroids_from_image(
    image,                          # PIL.Image or array-convertible
    sigma=2,                        # threshold = noise_std · sigma
    image_th=None,                  # explicit threshold; if set, sigma/sigma_mode ignored
    crop=None,                      # see crop_and_downsample_image
    downsample=None,                # integer downsample factor
    filtsize=25,                    # odd; window size for local bg/noise filters
    bg_sub_mode='local_mean',       # background subtraction method
    sigma_mode='global_root_square',# noise-std estimation method
    binary_open=True,               # 3x3 cross binary opening of the mask
    centroid_window=None,           # if set, re-centroid in a square window of this size
    max_area=100,                   # reject blobs with > this many pixels
    min_area=5,                     # reject blobs with < this many pixels
    max_sum=None,                   # reject blobs brighter than this (sum)
    min_sum=None,                   # reject blobs dimmer than this (sum)
    max_axis_ratio=None,            # reject elongated blobs (major/minor > this)
    max_returned=None,              # keep at most this many (brightest)
    return_moments=False,           # also return per-blob statistics
    return_images=False,            # also return intermediate images (tuning aid)
)
```

> **Tuning workflow (important).** Pass `return_images=True` to get a dict of intermediate
> images; inspect `binary_mask` (raw detections) and `final_centroids` (green = kept,
> red = rejected) and adjust parameters. Whatever extraction kwargs work are then passed
> straight through `solve_from_image(image, **extract_dict)`.

### Recommended settings (from the docstring)

- **Best quality** (slow): `bg_sub_mode='local_median'`, `sigma_mode='local_median_abs'`,
  modest `filtsize` (~15), very sharp image, sufficient camera bit depth.
- **Fast & recommended default**: `bg_sub_mode='local_mean'`,
  `sigma_mode='global_root_square'`, larger `filtsize` (≥25).
- **Bring-your-own**: do background subtraction/thresholding yourself, then pass
  `bg_sub_mode=None` and your `image_th`.

---

## 2. The 10-step algorithm

### Step 1 — Normalize to a 2-D float image

- Copy the raw image (kept for the optional visualization).
- Convert to `float32` ndarray.
- If 3-channel color → grayscale with **luminance weights**: `0.299·R + 0.587·G + 0.114·B`.
- If single-channel 3-D `(H,W,1)` → squeeze to 2-D. Assert the result is 2-D.

### Step 2 — Crop and downsample

Call `crop_and_downsample_image(image, crop, downsample, sum_when_downsample=True,
return_offsets=True)` → `(image, (offs_h, offs_w))`. Downsampling **sums** pixels (keeps
photon counts), so star sums are preserved across binning. (Full spec in §4.) Record the
new `(height, width)` and the crop offsets for later coordinate restoration.

### Step 3 — Background subtraction (`bg_sub_mode`)

Subtract an estimated background from every pixel. Four modes:

| mode | operation |
|---|---|
| `local_median` | `image −= median_filter(image, size=filtsize)` |
| `local_mean` *(default)* | `image −= uniform_filter(image, size=filtsize)` |
| `global_median` | `image −= median(image)` |
| `global_mean` | `image −= mean(image)` |

`local_*` modes require an **odd** `filtsize`. `local` modes adapt to spatially-varying
background (gradients, vignetting); `global` modes are cheaper. If `bg_sub_mode is None`,
skip (you must then supply `image_th`).

### Step 4 — Estimate the threshold (`sigma_mode`), unless `image_th` given

If `image_th` is `None`, estimate the per-image (or per-pixel) **noise standard deviation**
and set `image_th = img_std · sigma`. Four modes:

| mode | `img_std` |
|---|---|
| `local_median_abs` | `median_filter(|image|, filtsize) · 1.48` (per pixel) |
| `local_root_square` | `sqrt(uniform_filter(image², filtsize))` (per pixel) |
| `global_median_abs` | `median(|image|) · 1.48` (scalar) |
| `global_root_square` *(default)* | `sqrt(mean(image²))` (scalar) |

- `1.48` converts a **median absolute deviation** to an equivalent Gaussian σ
  (`1/Φ⁻¹(0.75) ≈ 1.4826`); robust to outliers (the stars themselves).
- `root_square` modes assume the (background-subtracted) image is ~zero-mean noise, so RMS
  ≈ σ. `local_*` variants give a per-pixel threshold map; `global_*` give one scalar.
- `image_th` may itself be a per-pixel array (for local modes).

### Step 5 — Threshold → binary mask

```
bin_mask = image > image_th
if binary_open:  bin_mask = binary_opening(bin_mask)   # 3x3 cross structuring element
```

Binary opening (erosion then dilation) removes single-pixel noise specks and thin bridges
between blobs, cleaning the mask. (scipy's default structuring element is the 3×3 cross.)

### Step 6 — Label connected regions

`(labels, num_labels) = scipy.ndimage.label(bin_mask)`. Each connected blob gets an integer
label `1..num_labels`. If `num_labels < 1`, return empty results immediately (shape-correct
empties for whichever return flags are set).

### Step 7 — Per-blob statistics and filtering (`calc_stats`)

For each labeled region, `scipy.ndimage.labeled_comprehension` runs `calc_stats(a, p)`
where `a` = the pixel **values** in the region and `p` = their flat positions. It computes:

```
(y, x)  = unravel(p)                 # pixel coords of region members
area    = number of pixels
m0      = Σ a                         # zeroth moment = brightness (sum)
m1_x    = Σ(x·a) / m0                 # first moment = centroid x
m1_y    = Σ(y·a) / m0                 # first moment = centroid y
```

Then **reject** the blob (sentinel `sum = NaN`, but the centroid is still recorded) if it
fails any active limit: `area < min_area` (5), `area > max_area` (100), `m0 < min_sum`,
`m0 > max_sum`.

If `return_moments` or `max_axis_ratio` is set, also compute **second central moments** and
the **axis ratio** (shape/elongation), used to reject trailed/elongated objects:

```
m2_xx = Σ((x−m1_x)²·a)/m0      m2_yy = Σ((y−m1_y)²·a)/m0      m2_xy = Σ((x−m1_x)(y−m1_y)·a)/m0
major = sqrt(2·(m2_xx + m2_yy + sqrt((m2_xx−m2_yy)² + 4·m2_xy²)))
minor = sqrt(2·max(0, m2_xx + m2_yy − sqrt((m2_xx−m2_yy)² + 4·m2_xy²)))
axis_ratio = major / max(minor, 1e-9)
```

`major`/`minor` are proportional to the eigenvalues of the 2×2 covariance (the moment
ellipse). Reject if `axis_ratio > max_axis_ratio` (e.g. `1.5` to drop satellite/aircraft
streaks). The returned centroid is `(m1_y + 0.5, m1_x + 0.5)` — the `+0.5` converts from
the "pixel corner" indexing of `unravel_index` to the "pixel center" convention.

Each `calc_stats` returns 8 floats: `(m0, m1_y+.5, m1_x+.5, m2_xx, m2_yy, m2_xy, area,
axis_ratio)`. Rows with `NaN` in column 0 are the rejects.

### Step 8 — Sort by brightness, truncate

```
order = argsort(−sum)                 # brightest (largest sum) first
if max_returned: order = order[:max_returned]
extracted = extracted[order]
```

### Step 9 — Optional window re-centroiding (`centroid_window`)

If `centroid_window` is set, recompute each centroid using a **square window** of that
width centered on the blob, instead of only the thresholded pixels. For each kept blob:
clamp a `centroid_window × centroid_window` box inside the image around `floor(centroid)`,
then compute the intensity-weighted centroid over the whole window:

```
xc = Σ(window · X) / Σ window         (X, Y are +0.5-centered coordinate grids)
yc = Σ(window · Y) / Σ window
centroid = (yc, xc) + (offs_y, offs_x)
```

This captures flux outside the hard threshold mask (more accurate for bright stars), at
the cost of sensitivity to nearby contaminants.

### Step 10 — Undo crop & downsample (restore original-image coordinates)

```
if downsample: centroid *= downsample
if crop:       centroid += (offs_h, offs_w)
```

so the returned `(y, x)` are in the **original** image's pixel frame.

### Return values

- Default: `extracted[:, 1:3]` → the `(N, 2)` `(y, x)` centroid array.
- `return_moments=True`: append `[sum, area, (xx,yy,xy) second moments, axis_ratio]`.
- `return_images=True`: append the `images_dict` (keys: `converted_input`,
  `cropped_and_downsampled`, `removed_background`, `binary_mask`, `final_centroids`).
- When both moments and images are requested, the result is a nested tuple — the solver
  detects a tuple return and uses element 0 (the centroids) for solving, reassembling the
  rest into its own return.

---

## 3. The visualization (`return_images`)

For tuning, the raw image is drawn over: 16-bit images are down-shifted to 8-bit, mono is
promoted to RGB, then **green circles** are drawn on accepted centroids and **red circles**
on rejected ones (radius `0.01·width`, scaled/offset back to original coords if
downsampled/cropped). The `binary_mask` image shows exactly what the threshold caught.

---

## 4. `crop_and_downsample_image`

```python
crop_and_downsample_image(image, crop=None, downsample=None,
                          sum_when_downsample=True, return_offsets=False)
```

Cropping is applied **before** downsampling. Input must be 2-D.

### Crop specification (`crop`)

- **Scalar** `c`: crop to centered `1/c` of the image. Requires `c` to divide both
  dimensions. e.g. `crop=2` → centered half-size.
- **2-tuple** `(h, w)`: centered region of that size.
- **4-tuple** `(h, w, offset_down, offset_right)`: region of size `(h,w)` offset from
  center.

The cropped size is rounded **up** to be divisible by the future downsample factor
(`divisor = downsample or 2`), clamped to the image, and the offset is clamped to stay
inside the image. Returns `(offs_h, offs_w)` = the applied top-left offset.

### Downsample (`downsample`)

Integer factor `d`. The (cropped) image must be divisible by `d` in both dimensions. The
image is reshaped to `(H/d, d, W/d, d)` and reduced over the two `d` axes:

- `sum_when_downsample=True` *(default)*: **sum** the `d×d` block (preserves total flux —
  important so star sums stay comparable). Integer inputs are promoted to float to avoid
  overflow, then clipped back to the integer range.
- `False`: **mean** of the block.

`get_centroids_from_image` always calls this with `sum_when_downsample=True`.

---

## 5. Where this fits

`solve_from_image(image, **kwargs)` → `get_centroids_from_image(image, **kwargs)` →
`solve_from_centroids(centroids, (height, width), ...)`. The detection kwargs and the solve
kwargs are passed in the same call; tetra3 forwards unknown kwargs to the centroider.

**Practical guidance baked into the code/docstring:**

- Default `sigma=2` is low; `solve_from_image` examples often raise effective robustness
  via `min_sum`, `max_area`, `max_axis_ratio`.
- For a 512×512+ camera at ~10° FOV reaching magnitude ~7, the default extraction is a
  reasonable starting point.
- This detector has **no hot-pixel handling and no explicit noise-floor adaptivity** beyond
  the chosen `sigma_mode`; that is precisely what cedar-detect (doc 04) adds.
