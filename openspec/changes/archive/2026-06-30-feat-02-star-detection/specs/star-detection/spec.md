## ADDED Requirements

### Requirement: Input and output contract

The system SHALL accept an 8-bit grayscale image and return a list of star centroids ordered
brightest-first, each with sub-pixel `(x, y)` position in full-resolution image coordinates
where `(0.5, 0.5)` is the center of the top-left pixel, plus `peak_value`, background-subtracted
`brightness`, and `num_saturated`. The 3 leftmost and 3 rightmost columns SHALL be excluded from
scanning (the 7-pixel horizontal gate needs 3 pixels of context per side). (Ref: doc 04 §1.)

#### Scenario: Brightest-first ordering
- **WHEN** detection returns multiple stars
- **THEN** they are sorted by background-subtracted `brightness` descending

#### Scenario: Pixel-center convention
- **WHEN** a centroid is reported
- **THEN** `(0.5, 0.5)` corresponds to the center of the top-left pixel and integer floor gives
  the pixel index

### Requirement: Noise estimation

The system SHALL estimate RMS noise robustly by taking three 1-pixel-tall horizontal cuts of
width `min(50, width/4)` centered on the vertical midline at `x ≈ width/4, width/2, 3·width/4`,
computing de-starred statistics for each (stars removed from the histogram first), and returning
the standard deviation of the **darkest cut by mean**. A noise floor of `0.2` SHALL be applied
(`noise = max(noise, 0.2)`). (Ref: doc 04 §3, §9.)

#### Scenario: Darkest cut chosen
- **WHEN** one of the three cuts overlaps a bright interloper (e.g. the moon)
- **THEN** the noise estimate comes from the darkest (lowest-mean) cut, not the bright one

#### Scenario: Noise floor applied
- **WHEN** the background is crushed to near-black so the measured stddev is below `0.2`
- **THEN** the returned noise is `0.2`

### Requirement: Binning cascade

The system SHALL support binning factors 1, 2, 4, 8. For `binning > 1` it SHALL detect on the
most-binned image (better SNR for soft/oversampled stars) while centroiding on the
one-level-less-binned `higher_res_image` and scaling centroids by `binning/2` back to
input-image coordinates; the noise estimate SHALL be recomputed on the detection image. Each
2×2 bin is the integer average `(p1+p2+p3+p4)/4`. `max_size` SHALL be `width/100` for
`binning==1`, else `width/100/binning + 1`. (Ref: doc 04 §4.)

#### Scenario: Centroids reported in input coordinates
- **WHEN** detection runs with `binning = 4`
- **THEN** returned centroids are expressed in full-resolution input-image coordinates

#### Scenario: Optional row normalization
- **WHEN** `normalize_rows` is set
- **THEN** each row's dark level is shifted to a fixed bias of `2.0` before binning

### Requirement: One-dimensional row gate

The system SHALL scan each row by first estimating a cheap `row_min` (sampling every 64th
pixel) to set a coarse threshold `row_min + sigma_noise_2/2` that most pixels skip, then
applying the 7-pixel gate `gate_star_1d` to each center pixel at or above threshold, using
integer thresholds `sigma_noise_2 = max(round(2·sigma·noise), 2)` and
`sigma_noise_3 = max(round(3·sigma·noise), 3)`. A center pixel `C` with borders `lb,rb`,
margins `lm,rm`, neighbors `l,r` SHALL qualify only if: `2·C − (lb+rb) ≥ sigma_noise_2`;
`l ≤ C ≥ r`; `lm < C > rm`; the flat-top tie-breaks hold; and `|lb − rb| ≤ sigma_noise_3`
(uniform background). (Ref: doc 04 §5.)

#### Scenario: Significance test eliminates background
- **WHEN** a center pixel's excess over its border estimate is below `sigma_noise_2`
- **THEN** it is not emitted as a candidate

#### Scenario: Non-uniform background rejected
- **WHEN** the two border pixels differ by more than `sigma_noise_3` (e.g. a brightness edge)
- **THEN** the center is rejected

#### Scenario: One center per flat-topped peak
- **WHEN** adjacent pixels tie at the peak value
- **THEN** the tie-break rules claim exactly one center for the peak

### Requirement: Hot-pixel rejection

When `detect_hot_pixels` is enabled, the system SHALL classify each 1-D candidate against the
**full-resolution** image: a backing pixel is `Hot` when it is bright but isolated
(`4·((l+r)−(lb+rb)) ≤ (2C−(lb+rb))/2`, i.e. neighbors carry < 1/8 of the center excess), `Dark`
when not bright, else `Bright`. A candidate SHALL be dropped (and counted) when **all** backing
bright pixels are hot (no neighbor-supported `Bright` pixel). (Ref: doc 04 §6.)

#### Scenario: Isolated spike rejected
- **WHEN** a candidate is backed only by an isolated bright pixel with no neighbor support
- **THEN** it is classified hot and excluded from the star list, incrementing the hot-pixel count

#### Scenario: Neighbor-supported star kept
- **WHEN** a candidate's bright pixel deposits flux into its neighbors
- **THEN** it is classified `Bright` and proceeds to blob formation

### Requirement: Blob formation

The system SHALL merge vertically adjacent candidates into blobs: bucket candidates by row,
and scanning top-to-bottom merge a candidate with a previous-row candidate within ±3 in x,
following union-find-style recipient forwarding. Within a row candidates are never adjacent
(the 1-D gate guarantees one center per peak), so only vertical adjacency is checked. (Ref:
doc 04 §7.)

#### Scenario: Vertically extended star becomes one blob
- **WHEN** the same star triggers candidates on several adjacent rows within ±3 x
- **THEN** they merge into a single blob

### Requirement: Two-dimensional gate

The system SHALL apply a 2-D gate per blob using four concentric boxes — `core` (blob bounding
box), `neighbors` (+1), `margin` (+2), `perimeter` (+3) — rejecting the blob on any failure of:
size (`core_width,core_height ≤ max_size` and perimeter within image bounds); inner-core
brightness (`core_mean ≥ outer_core_mean` when core ≥ 3×3); `core_mean ≥ neighbor_mean`
(corners excluded); `core_mean > margin_mean`; uniform perimeter
(`perimeter_max − perimeter_min ≤ 3·sigma·noise`); and significance
`core_mean − mean(perimeter) ≥ sigma·max(noise, perimeter_stddev)`. (Ref: doc 04 §8.)

#### Scenario: Locally inflated noise suppresses clutter
- **WHEN** the perimeter ring is noisy (lit foreground)
- **THEN** the significance bar uses `max(noise, perimeter_stddev)`, raising the threshold

#### Scenario: Oversized bleeding blob rejected
- **WHEN** a blob exceeds `max_size` in width or height
- **THEN** it is rejected as non-star structure

### Requirement: Sub-pixel centroid

The system SHALL compute the centroid by projecting the measurement box onto its x and y axes,
finding each 1-D projection peak, and refining to sub-pixel via quadratic interpolation
`p = 0.5·(a − c)/(a − 2b + c)` of the peak `b` and its neighbors `a,c`; a run of equal values
resolves to the run midpoint and a peak at the box edge takes the edge index. The final centroid
is `(peak_x + 0.5, peak_y + 0.5)`. (Ref: doc 04 §8.1.)

#### Scenario: Quadratic refinement
- **WHEN** a projection peak has unequal neighbors
- **THEN** the sub-pixel offset is `0.5·(a−c)/(a−2b+c)` within `[−0.5, 0.5]`

### Requirement: Brightness measurement and ordering

The system SHALL compute brightness as the perimeter-background-subtracted sum over the inset
(bounding box shrunk by 1 px), clamped to ≥ 0, with `num_saturated` = count of pixels == 255 and
`peak_value` = max inset pixel, then SHALL sort all detected stars by `brightness` descending.
(Ref: doc 04 §8.2.)

#### Scenario: Background-subtracted brightness
- **WHEN** brightness is computed for a star
- **THEN** the perimeter-ring mean is subtracted from the inset sum and the result is ≥ 0

### Requirement: Documented detection limitations

The system SHALL detect only condensed star-like spots, MAY miss closely-crowded stars, stars
adjacent to hot pixels, or doubled stars from shake, SHALL operate on 8-bit input only, SHALL
tolerate ≲1 px motion blur, and SHALL leave lens-distortion correction to the solver. (Ref:
doc 04 §11.)

#### Scenario: Distortion is not corrected here
- **WHEN** centroids are produced
- **THEN** they are raw pixel positions with no lens-distortion correction applied

### Requirement: Parity with cedar-detect

The system SHALL produce centroids matching the cedar-detect reference on the
`reference-solutions/cedar-detect/test_data` images within tolerance: matched centroids agree
within ≈±0.1 px and the brightest-first ordering agrees. (Ref: doc 04; doc 08 §2.)

#### Scenario: Reference-image parity
- **WHEN** a reference test image is processed with matching parameters (`sigma = 8`)
- **THEN** the detected centroids correspond to cedar-detect's within ±0.1 px with the same
  brightness ranking for the common stars
