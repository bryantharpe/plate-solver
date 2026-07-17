//! Star detection pipeline entry point.
//!
//! Implements the full front-end detection algorithm: noise estimation, optional
//! binning cascade, one-dimensional row gating, hot-pixel rejection, blob
//! formation, two-dimensional gating, sub-pixel centroiding, and brightness
//! measurement. The result is returned brightest-first.

use crate::binning::{build_cascade, max_size, BinnedImage};
use crate::centroid::measure_star;
use crate::noise::estimate_noise;
use crate::star::Star;

/// Detect stars in an 8-bit grayscale image.
///
/// `image` is laid out row-major with `width` pixels per row. `sigma` is the
/// significance threshold in units of the estimated RMS noise. `binning` must
/// be one of 1, 2, 4, or 8. When `normalize_rows` is true and `binning > 1`,
/// each row's dark level is shifted to a fixed bias before binning. When
/// `detect_hot_pixels` is true, isolated hot pixels are rejected using the
/// full-resolution image.
///
/// Returned centroids are in full-resolution input-image coordinates where
/// `(0.5, 0.5)` is the center of the top-left pixel. Stars are sorted by
/// background-subtracted brightness descending.
pub fn detect_stars(
    image: &[u8],
    width: usize,
    height: usize,
    sigma: f64,
    binning: usize,
    normalize_rows: bool,
    detect_hot_pixels: bool,
) -> Vec<Star> {
    assert_eq!(
        image.len(),
        width * height,
        "image length must equal width * height"
    );

    if width < 7 || height < 1 {
        return Vec::new();
    }

    // Build the binning cascade. For binning > 1 we detect on the most-binned
    // image and centroid on the one-level-less-binned image.
    let (detection, higher_res) = build_cascade(image, width, height, binning, normalize_rows);

    // Recompute the noise estimate on the detection image, then apply the floor.
    let noise = estimate_noise(&detection.data, detection.width, detection.height);

    // Integer thresholds used by the 1-D gate and hot-pixel classifier.
    let sigma_noise_2 = ((2.0 * sigma * noise) + 0.5).round() as i16;
    let sigma_noise_2 = sigma_noise_2.max(2);
    let sigma_noise_3 = ((3.0 * sigma * noise) + 0.5).round() as i16;
    let sigma_noise_3 = sigma_noise_3.max(3);

    // Scan the detection image for 1-D candidates.
    let candidates = scan_image_for_candidates(&detection, sigma_noise_2, sigma_noise_3);

    // Optionally reject hot pixels, using the full-resolution image.
    let filtered = if detect_hot_pixels {
        candidates
            .into_iter()
            .filter(|c| !all_bright_are_hot(image, width, height, c.x, c.y, binning, sigma_noise_2))
            .collect()
    } else {
        candidates
    };

    // Form blobs from vertically-adjacent candidates.
    let blobs = form_blobs_from_candidates(filtered);

    // Apply the 2-D gate, centroid, and measure brightness.
    let mut stars = Vec::new();
    let max_size = detection.max_size;
    for blob in blobs {
        if let Some(star) = gate_star_2d(
            &blob,
            &detection,
            higher_res.as_ref(),
            image,
            width,
            height,
            binning,
            noise,
            sigma,
            max_size,
        ) {
            stars.push(star);
        }
    }

    // Brightest-first ordering.
    stars.sort();
    stars
}

/// A single 1-D candidate emitted by the row gate.
#[derive(Clone, Copy, Debug)]
struct Candidate {
    x: usize,
    y: usize,
}

/// Scan every row of the detection image and emit 1-D candidates.
fn scan_image_for_candidates(
    image: &BinnedImage,
    sigma_noise_2: i16,
    sigma_noise_3: i16,
) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let width = image.width;
    let height = image.height;
    if width < 7 {
        return candidates;
    }

    for y in 0..height {
        let row_start = y * width;
        let row = &image.data[row_start..row_start + width];

        // Cheap row-min estimate: sample every 64th pixel.
        let mut row_min = 255u8;
        for &p in row.iter().step_by(64) {
            if p < row_min {
                row_min = p;
            }
        }
        let threshold = row_min.saturating_add((sigma_noise_2 as u8) / 2);

        for center_x in 3..(width - 3) {
            let center_pixel = row[center_x];
            if center_pixel < threshold {
                continue;
            }
            let gate = &row[center_x - 3..center_x + 4];
            if gate_star_1d(gate, sigma_noise_2, sigma_noise_3) {
                candidates.push(Candidate { x: center_x, y });
            }
        }
    }

    candidates
}

/// Apply the 7-pixel horizontal 1-D gate.
///
/// Pixels are labeled `|lb lm l C r rm rb|`. Returns true if the center pixel
/// qualifies as a star candidate.
fn gate_star_1d(gate: &[u8], sigma_noise_2: i16, sigma_noise_3: i16) -> bool {
    let lb = gate[0] as i16;
    let lm = gate[1] as i16;
    let l = gate[2] as i16;
    let c = gate[3] as i16;
    let r = gate[4] as i16;
    let rm = gate[5] as i16;
    let rb = gate[6] as i16;

    // Significance: 2*C - (lb+rb) must exceed sigma_noise_2.
    let center_minus_background_2 = c + c - (lb + rb);
    if center_minus_background_2 < sigma_noise_2 {
        return false;
    }

    // Local maximum vs immediate neighbors.
    if l > c || c < r {
        return false;
    }

    // Strictly brighter than margins.
    if lm >= c || c <= rm {
        return false;
    }

    // Flat-top tie-breaking: claim exactly one center per flat peak.
    if l == c && lm > r {
        return false;
    }
    if c == r && l <= rm {
        return false;
    }

    // Uniform background: border pixels must not differ too much.
    if (lb - rb).abs() > sigma_noise_3 {
        return false;
    }

    true
}

/// A blob is a set of vertically-adjacent 1-D candidates.
#[derive(Clone, Debug, Default)]
struct Blob {
    candidates: Vec<Candidate>,
    recipient: Option<usize>,
}

/// Merge candidates that are vertically adjacent within ±3 in x.
fn form_blobs_from_candidates(candidates: Vec<Candidate>) -> Vec<Blob> {
    if candidates.is_empty() {
        return Vec::new();
    }

    let max_y = candidates.iter().map(|c| c.y).max().unwrap_or(0);
    let mut by_row: Vec<Vec<usize>> = vec![Vec::new(); max_y + 1];
    let mut blobs: Vec<Blob> = Vec::with_capacity(candidates.len());

    for (id, cand) in candidates.into_iter().enumerate() {
        blobs.push(Blob {
            candidates: vec![cand],
            recipient: None,
        });
        by_row[cand.y].push(id);
    }

    for row in 1..=max_y {
        for &current_id in &by_row[row] {
            let current_rep = representative(&blobs, current_id);
            let current_x = blobs[current_rep].candidates[0].x as i32;
            for &prev_id in &by_row[row - 1] {
                let prev_rep = representative(&blobs, prev_id);
                let prev_x = blobs[prev_rep].candidates[0].x as i32;
                if prev_x < current_x - 3 {
                    continue;
                }
                if prev_x > current_x + 3 {
                    break;
                }

                if prev_rep == current_rep {
                    continue;
                }

                // Merge the previous blob into the current one.
                let mut donated = core::mem::take(&mut blobs[prev_rep].candidates);
                blobs[prev_rep].recipient = Some(current_rep);
                blobs[current_rep].candidates.append(&mut donated);
            }
        }
    }

    blobs
        .into_iter()
        .filter(|b| !b.candidates.is_empty())
        .collect()
}

/// Follow recipient pointers until we reach the blob that still owns candidates.
fn representative(blobs: &[Blob], mut id: usize) -> usize {
    loop {
        if let Some(recipient) = blobs[id].recipient {
            id = recipient;
        } else {
            return id;
        }
    }
}

/// Apply the 2-D gate to a blob, and if it passes, centroid and measure it.
fn gate_star_2d(
    blob: &Blob,
    image: &BinnedImage,
    higher_res: Option<&BinnedImage>,
    full_res: &[u8],
    full_width: usize,
    full_height: usize,
    binning: usize,
    noise: f64,
    sigma: f64,
    max_size: usize,
) -> Option<Star> {
    // Compute the core bounding box from the blob's candidate coordinates.
    let x_min = blob.candidates.iter().map(|c| c.x).min()?;
    let x_max = blob.candidates.iter().map(|c| c.x).max()?;
    let y_min = blob.candidates.iter().map(|c| c.y).min()?;
    let y_max = blob.candidates.iter().map(|c| c.y).max()?;

    let core_width = x_max - x_min + 1;
    let core_height = y_max - y_min + 1;

    // Size test and image-edge test.
    if core_width > max_size || core_height > max_size {
        return None;
    }
    if x_min < 3 || x_max + 3 >= image.width || y_min < 3 || y_max + 3 >= image.height {
        return None;
    }

    // Concentric boxes.
    let core_left = x_min;
    let core_top = y_min;
    let neighbors_left = x_min - 1;
    let neighbors_top = y_min - 1;
    let neighbors_width = core_width + 2;
    let neighbors_height = core_height + 2;
    let margin_left = x_min - 2;
    let margin_top = y_min - 2;
    let margin_width = core_width + 4;
    let margin_height = core_height + 4;
    let perimeter_left = x_min - 3;
    let perimeter_top = y_min - 3;
    let perimeter_width = core_width + 6;
    let perimeter_height = core_height + 6;

    // Core mean.
    let core_mean = roi_mean(&image.data, image.width, core_left, core_top, core_width, core_height);

    // Inner-core brightness test (only for cores >= 3x3).
    if core_width >= 3 && core_height >= 3 {
        let outer_core_mean = perimeter_mean(
            &image.data,
            image.width,
            core_left,
            core_top,
            core_width,
            core_height,
        );
        if core_mean < outer_core_mean {
            return None;
        }
    }

    // Neighbor mean (perimeter of neighbors box, corners excluded).
    let neighbor_mean = perimeter_mean_excluding_corners(
        &image.data,
        image.width,
        neighbors_left,
        neighbors_top,
        neighbors_width,
        neighbors_height,
    );
    if core_mean < neighbor_mean {
        return None;
    }

    // Margin mean (perimeter of margin box).
    let margin_mean = perimeter_mean(
        &image.data,
        image.width,
        margin_left,
        margin_top,
        margin_width,
        margin_height,
    );
    if core_mean <= margin_mean {
        return None;
    }

    // Perimeter statistics.
    let (perimeter_mean, perimeter_stddev, perimeter_min, perimeter_max) = perimeter_stats(
        &image.data,
        image.width,
        perimeter_left,
        perimeter_top,
        perimeter_width,
        perimeter_height,
    );

    // Uniform perimeter.
    if (perimeter_max - perimeter_min) as f64 > 3.0 * sigma * noise {
        return None;
    }

    // Significance with locally-inflated noise.
    let max_noise = noise.max(perimeter_stddev);
    if core_mean - perimeter_mean < sigma * max_noise {
        return None;
    }

    // The blob passes all 2-D gates. Measure it.
    if binning == 1 {
        measure_star(
            &image.data,
            image.width,
            margin_left,
            margin_top,
            margin_width,
            margin_height,
        )
    } else {
        // Translate the margin box into higher-res image coordinates (scale=2).
        let higher = higher_res?;
        let hr_left = margin_left * 2;
        let hr_top = margin_top * 2;
        let hr_width = (margin_width * 2).min(higher.width - hr_left);
        let hr_height = (margin_height * 2).min(higher.height - hr_top);

        let mut star = measure_star(
            &higher.data,
            higher.width,
            hr_left,
            hr_top,
            hr_width,
            hr_height,
        )?;

        // Scale centroid from higher-res space back to input-image coordinates.
        let upsample = (binning as f64) / 2.0;
        star.x *= upsample;
        star.y *= upsample;
        Some(star)
    }
}

/// Mean of a rectangular ROI.
fn roi_mean(data: &[u8], width: usize, left: usize, top: usize, w: usize, h: usize) -> f64 {
    let mut sum = 0u64;
    for y in 0..h {
        let row_start = (top + y) * width + left;
        for x in 0..w {
            sum += data[row_start + x] as u64;
        }
    }
    sum as f64 / (w * h) as f64
}

/// Mean of the 1-pixel perimeter ring of a box.
fn perimeter_mean(
    data: &[u8],
    width: usize,
    left: usize,
    top: usize,
    w: usize,
    h: usize,
) -> f64 {
    let (sum, count) = perimeter_sum_count(data, width, left, top, w, h);
    sum as f64 / count as f64
}

/// Mean of the perimeter ring excluding the four corners.
fn perimeter_mean_excluding_corners(
    data: &[u8],
    width: usize,
    left: usize,
    top: usize,
    w: usize,
    h: usize,
) -> f64 {
    let (mut sum, mut count) = perimeter_sum_count(data, width, left, top, w, h);
    if w >= 2 && h >= 2 {
        let right = left + w - 1;
        let bottom = top + h - 1;
        sum -= data[top * width + left] as u64;
        sum -= data[top * width + right] as u64;
        sum -= data[bottom * width + left] as u64;
        sum -= data[bottom * width + right] as u64;
        count -= 4;
    }
    sum as f64 / count as f64
}

/// Sum and pixel count of a perimeter ring.
fn perimeter_sum_count(
    data: &[u8],
    width: usize,
    left: usize,
    top: usize,
    w: usize,
    h: usize,
) -> (u64, usize) {
    let mut sum = 0u64;
    let mut count = 0usize;
    let right = left + w - 1;
    let bottom = top + h - 1;

    // Top and bottom rows.
    for x in left..=right {
        sum += data[top * width + x] as u64;
        sum += data[bottom * width + x] as u64;
        count += 2;
    }
    // Left and right edges, excluding corners.
    for y in (top + 1)..bottom {
        sum += data[y * width + left] as u64;
        sum += data[y * width + right] as u64;
        count += 2;
    }

    (sum, count)
}

/// Mean, stddev, min, and max of a perimeter ring.
fn perimeter_stats(
    data: &[u8],
    width: usize,
    left: usize,
    top: usize,
    w: usize,
    h: usize,
) -> (f64, f64, u8, u8) {
    let right = left + w - 1;
    let bottom = top + h - 1;

    let mut min = 255u8;
    let mut max = 0u8;
    let mut sum = 0u64;
    let mut count = 0usize;

    for x in left..=right {
        let v_top = data[top * width + x];
        let v_bot = data[bottom * width + x];
        min = min.min(v_top).min(v_bot);
        max = max.max(v_top).max(v_bot);
        sum += v_top as u64 + v_bot as u64;
        count += 2;
    }
    for y in (top + 1)..bottom {
        let v_left = data[y * width + left];
        let v_right = data[y * width + right];
        min = min.min(v_left).min(v_right);
        max = max.max(v_left).max(v_right);
        sum += v_left as u64 + v_right as u64;
        count += 2;
    }

    let mean = sum as f64 / count as f64;
    let mut dev2 = 0.0f64;
    for x in left..=right {
        let d1 = data[top * width + x] as f64 - mean;
        let d2 = data[bottom * width + x] as f64 - mean;
        dev2 += d1 * d1 + d2 * d2;
    }
    for y in (top + 1)..bottom {
        let d1 = data[y * width + left] as f64 - mean;
        let d2 = data[y * width + right] as f64 - mean;
        dev2 += d1 * d1 + d2 * d2;
    }
    let stddev = (dev2 / count as f64).sqrt();

    (mean, stddev, min, max)
}

/// Check whether every bright backing pixel of a (possibly binned) candidate is hot.
fn all_bright_are_hot(
    full_res: &[u8],
    full_width: usize,
    full_height: usize,
    x: usize,
    y: usize,
    binning: usize,
    sigma_noise_2: i16,
) -> bool {
    if binning == 1 {
        if y >= full_height || x >= full_width {
            return true;
        }
        let row_start = y * full_width;
        let gate = &full_res[row_start + x - 3..row_start + x + 4];
        return classify_pixel(gate, sigma_noise_2) != PixelClass::Bright;
    }

    let x_full = x * binning;
    let y_full = y * binning;
    for yi in 0..binning {
        let backing_y = y_full + yi;
        if backing_y >= full_height {
            continue;
        }
        let row_start = backing_y * full_width;
        for xi in 0..binning {
            let backing_x = x_full + xi;
            if backing_x >= full_width {
                continue;
            }
            let gate = &full_res[row_start + backing_x - 3..row_start + backing_x + 4];
            if classify_pixel(gate, sigma_noise_2) == PixelClass::Bright {
                return false;
            }
        }
    }
    true
}

/// Classification of a pixel for hot-pixel rejection.
#[derive(Debug, Eq, PartialEq)]
enum PixelClass {
    Dark,
    Bright,
    Hot,
}

/// Classify a pixel using its 7-pixel horizontal gate.
fn classify_pixel(gate: &[u8], sigma_noise_2: i16) -> PixelClass {
    let lb = gate[0] as i16;
    let l = gate[2] as i16;
    let c = gate[3] as i16;
    let r = gate[4] as i16;
    let rb = gate[6] as i16;

    let est_background_2 = lb + rb;
    let center_minus_background_2 = c + c - est_background_2;
    if center_minus_background_2 < sigma_noise_2 {
        return PixelClass::Dark;
    }

    let neighbor_sum_minus_background = (l + r) - est_background_2;
    if 4 * neighbor_sum_minus_background <= center_minus_background_2 / 2 {
        return PixelClass::Hot;
    }

    PixelClass::Bright
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_star_1d_rejects_weak_center() {
        // Background ~10, center 12: not significant.
        let gate = [10, 10, 10, 12, 10, 10, 10];
        assert!(!gate_star_1d(&gate, 10, 15));
    }

    #[test]
    fn gate_star_1d_accepts_bright_peak() {
        // Strong center with uniform background.
        let gate = [10, 10, 20, 100, 20, 10, 10];
        assert!(gate_star_1d(&gate, 10, 15));
    }

    #[test]
    fn classify_pixel_detects_hot() {
        // Isolated bright pixel with dark neighbors.
        let gate = [10, 10, 10, 100, 10, 10, 10];
        assert_eq!(classify_pixel(&gate, 10), PixelClass::Hot);
    }

    #[test]
    fn classify_pixel_detects_bright() {
        // Star-like: neighbors share flux.
        let gate = [10, 10, 60, 100, 60, 10, 10];
        assert_eq!(classify_pixel(&gate, 10), PixelClass::Bright);
    }
}
