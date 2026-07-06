//! Blob formation and 2-D star gate for star detection (SD5).
//!
//! Groups 1-D candidates into spatial blobs, then applies concentric-box
//! brightness criteria to confirm each blob is a genuine star.

use crate::gate::CandidateFrom1D;
use crate::GrayImageView;
use crate::StarDescription;
use imageproc::rect::Rect;
use std::cmp;

/// A spatial blob of connected 1-D candidates.
pub struct Blob {
    pub candidates: Vec<CandidateFrom1D>,
    /// If `candidates` is empty, this blob was merged into another.
    pub recipient_blob: Option<usize>,
}

#[derive(Copy, Clone)]
struct LabeledCandidate {
    candidate: CandidateFrom1D,
    blob_id: usize,
}

// ---------------------------------------------------------------------------
// Pixel iterators
// ---------------------------------------------------------------------------

#[inline(always)]
fn for_each_pixel_in_roi<F>(image: &GrayImageView<'_>, roi: &Rect, mut f: F)
where
    F: FnMut(i32, i32, u8),
{
    let (width, height) = {
        let (w, h) = image.dimensions();
        (w as usize, h as usize)
    };
    assert!(roi.left() >= 0);
    assert!(roi.top() >= 0);
    assert!(roi.right() < width as i32);
    assert!(roi.bottom() < height as i32);
    let raw = image.as_raw();
    let left = roi.left() as usize;
    let top = roi.top() as usize;
    let bottom = roi.bottom() as usize;
    let w = roi.width() as usize;
    if w == 0 || roi.height() == 0 {
        return;
    }
    for y in top..=bottom {
        let row_start = y * width;
        let row_slice = &raw[row_start + left..row_start + left + w];
        for (i, &pixel) in row_slice.iter().enumerate() {
            f((left + i) as i32, y as i32, pixel);
        }
    }
}

#[inline(always)]
fn for_each_perimeter_pixel<F>(image: &GrayImageView<'_>, roi: &Rect, mut f: F)
where
    F: FnMut(i32, i32, u8),
{
    let (width, height) = {
        let (w, h) = image.dimensions();
        (w as usize, h as usize)
    };
    assert!(roi.left() >= 0);
    assert!(roi.top() >= 0);
    assert!(roi.right() < width as i32);
    assert!(roi.bottom() < height as i32);
    let raw = image.as_raw();
    let left = roi.left() as usize;
    let top = roi.top() as usize;
    let bottom = roi.bottom() as usize;
    let w = roi.width() as usize;
    if w == 0 || roi.height() == 0 {
        return;
    }
    let right = left + w - 1;

    // Top row.
    let top_row_start = top * width;
    let top_row_slice = &raw[top_row_start + left..top_row_start + left + w];
    for (i, &pixel) in top_row_slice.iter().enumerate() {
        f((left + i) as i32, top as i32, pixel);
    }
    if bottom > top {
        // Middle rows.
        for y in (top + 1)..bottom {
            let row_start = y * width;
            f(left as i32, y as i32, raw[row_start + left]);
            if w > 1 {
                f(right as i32, y as i32, raw[row_start + right]);
            }
        }
        // Bottom row.
        let bot_row_start = bottom * width;
        let bot_row_slice = &raw[bot_row_start + left..bot_row_start + left + w];
        for (i, &pixel) in bot_row_slice.iter().enumerate() {
            f((left + i) as i32, bottom as i32, pixel);
        }
    }
}

// ---------------------------------------------------------------------------
// Brightness / centroid helpers
// ---------------------------------------------------------------------------

/// Compute background-subtracted brightness of a region.
///
/// The outer perimeter of `bounding_box` is used for background estimation;
/// the inner pixels are background-subtracted and summed.
///
/// Returns `(brightness, num_saturated, peak_value)`.
fn compute_brightness(image: &GrayImageView<'_>, bounding_box: &Rect) -> (f64, u16, u8) {
    let mut boundary_sum: i32 = 0;
    let mut boundary_count: i32 = 0;
    for_each_perimeter_pixel(image, bounding_box, |_x, _y, pixel_value| {
        boundary_sum += pixel_value as i32;
        boundary_count += 1;
    });
    let background_est = boundary_sum as f64 / boundary_count as f64;

    let inset = Rect::at(bounding_box.left() + 1, bounding_box.top() + 1)
        .of_size(bounding_box.width() - 2, bounding_box.height() - 2);

    let mut num_saturated: u16 = 0;
    let mut sum = 0.0f64;
    let mut peak_value: u8 = 0;
    for_each_pixel_in_roi(image, &inset, |_x, _y, pixel_value| {
        if pixel_value == 255_u8 {
            num_saturated += 1;
        }
        if pixel_value > peak_value {
            peak_value = pixel_value;
        }
        sum += pixel_value as f64 - background_est;
    });
    (f64::max(sum, 0.0), num_saturated, peak_value)
}

/// Compute sub-pixel peak coordinates via projection + quadratic interpolation.
fn compute_peak_coord(image: &GrayImageView<'_>, bounding_box: &Rect) -> (f64, f64) {
    let mut horizontal_projection = vec![0u32; bounding_box.width() as usize];
    let mut vertical_projection = vec![0u32; bounding_box.height() as usize];
    let x0 = bounding_box.left();
    let y0 = bounding_box.top();
    for_each_pixel_in_roi(image, bounding_box, |x, y, pixel_value| {
        horizontal_projection[(x - x0) as usize] += pixel_value as u32;
        vertical_projection[(y - y0) as usize] += pixel_value as u32;
    });
    let peak_x = x0 as f64 + peak_coord_1d(horizontal_projection);
    let peak_y = y0 as f64 + peak_coord_1d(vertical_projection);
    (peak_x, peak_y)
}

/// Find the 1-D peak coordinate with quadratic interpolation.
fn peak_coord_1d(values: Vec<u32>) -> f64 {
    let mut peak_val = 0u32;
    let mut peak_ind = 0usize;
    let mut in_run = false;
    let mut peak_run_length = 0usize;
    for (ind, val) in values.iter().enumerate() {
        if *val > peak_val || ind == 0 {
            peak_val = *val;
            peak_ind = ind;
            peak_run_length = 1;
            in_run = true;
        } else if *val == peak_val {
            if in_run {
                peak_run_length += 1;
            }
        } else {
            in_run = false;
        }
    }

    // Run of equal-length values: return mid-coord.
    if peak_run_length > 1 {
        return peak_ind as f64 + (peak_run_length - 1) as f64 / 2.0;
    }
    // Peak at either end: return its coord.
    if peak_ind == 0 || peak_ind == values.len() - 1 {
        return peak_ind as f64;
    }

    // Quadratic interpolation.
    let a = values[peak_ind - 1] as f64;
    let b = values[peak_ind] as f64;
    let c = values[peak_ind + 1] as f64;
    let p = 0.5 * (a - c) / (a - 2.0 * b + c);
    assert!(p >= -0.5);
    assert!(p <= 0.5);

    peak_ind as f64 + p
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Group 1-D row-scan candidates into spatial blobs.
///
/// Candidates within a horizontal distance of 3 pixels between adjacent rows
/// are merged into the same blob. Returns only non-empty blobs.
pub fn form_blobs_from_candidates(candidates: Vec<CandidateFrom1D>, max_y: usize) -> Vec<Blob> {
    let mut labeled_candidates_by_row = vec![Vec::<LabeledCandidate>::new(); max_y + 1];

    let mut blobs: Vec<Blob> = Vec::with_capacity(candidates.len());
    for (next_blob_id, candidate) in candidates.into_iter().enumerate() {
        blobs.push(Blob {
            candidates: vec![candidate],
            recipient_blob: None,
        });
        labeled_candidates_by_row[candidate.y as usize].push(LabeledCandidate {
            candidate,
            blob_id: next_blob_id,
        });
    }

    // Merge adjacent blobs across rows.
    for rownum in 1..labeled_candidates_by_row.len() {
        for rc in &labeled_candidates_by_row[rownum] {
            let rc_pos = rc.candidate.x;
            for prev_row_rc in &labeled_candidates_by_row[rownum - 1] {
                let prev_row_rc_pos = prev_row_rc.candidate.x;
                if prev_row_rc_pos < rc_pos - 3 {
                    continue;
                }
                if prev_row_rc_pos > rc_pos + 3 {
                    break;
                }
                // Adjacent: absorb the previous row blob's candidates.
                let recipient_blob_id = rc.blob_id;
                let mut donor_blob_id = prev_row_rc.blob_id;
                let mut donated_candidates: Vec<CandidateFrom1D>;
                loop {
                    let donor_blob = &mut blobs[donor_blob_id];
                    if !donor_blob.candidates.is_empty() {
                        donated_candidates = donor_blob.candidates.drain(..).collect();
                        assert_eq!(donor_blob.recipient_blob, None);
                        donor_blob.recipient_blob = Some(recipient_blob_id);
                        let recipient_blob = &mut blobs[recipient_blob_id];
                        recipient_blob.candidates.append(&mut donated_candidates);
                        break;
                    }
                    // Donor was already merged; follow the chain.
                    let merged_blob_id = donor_blob
                        .recipient_blob
                        .expect("if candidates are empty, must have been merged");
                    if merged_blob_id == recipient_blob_id {
                        break;
                    }
                    donor_blob_id = merged_blob_id;
                }
            }
        }
    }

    // Return only non-empty blobs.
    let mut non_empty_blobs = Vec::<Blob>::new();
    for blob in blobs {
        if !blob.candidates.is_empty() {
            assert_eq!(blob.recipient_blob, None);
            non_empty_blobs.push(blob);
        }
    }
    non_empty_blobs
}

/// Apply the 2-D star gate to a blob.
///
/// Uses concentric boxes (core / neighbors / margin / perimeter) to verify
/// that the blob is a genuine star-like spot, then computes centroid and
/// brightness via the higher-resolution image.
#[allow(clippy::too_many_arguments)]
pub fn gate_star_2d(
    blob: &Blob,
    image: &GrayImageView<'_>,
    higher_res_image: &GrayImageView<'_>,
    binning: u32,
    noise_estimate: f64,
    sigma: f64,
    max_width: u32,
    max_height: u32,
) -> Option<StarDescription> {
    let (image_width, image_height) = image.dimensions();

    // Compute bounding box of all blob candidates.
    let mut x_min = u32::MAX;
    let mut x_max = 0u32;
    let mut y_min = u32::MAX;
    let mut y_max = 0u32;
    for candidate in &blob.candidates {
        x_min = cmp::min(x_min, candidate.x as u32);
        x_max = cmp::max(x_max, candidate.x as u32);
        y_min = cmp::min(y_min, candidate.y as u32);
        y_max = cmp::max(y_max, candidate.y as u32);
    }
    let core_x_min = x_min as i32;
    let core_x_max = x_max as i32;
    let core_y_min = y_min as i32;
    let core_y_max = y_max as i32;
    let core_width = (core_x_max - core_x_min) as u32 + 1;
    let core_height = (core_y_max - core_y_min) as u32 + 1;

    // Reject if too large.
    if core_width > max_width || core_height > max_height {
        return None;
    }

    // Reject if expansion goes past image boundary.
    if core_x_min - 3 < 0
        || core_x_max + 3 >= image_width as i32
        || core_y_min - 3 < 0
        || core_y_max + 3 >= image_height as i32
    {
        return None;
    }

    // Define concentric boxes.
    let core = Rect::at(core_x_min, core_y_min).of_size(core_width, core_height);
    let neighbors =
        Rect::at(core_x_min - 1, core_y_min - 1).of_size(core_width + 2, core_height + 2);
    let margin = Rect::at(core_x_min - 2, core_y_min - 2).of_size(core_width + 4, core_height + 4);
    let perimeter =
        Rect::at(core_x_min - 3, core_y_min - 3).of_size(core_width + 6, core_height + 6);

    // Compute core mean.
    let mut core_sum: i32 = 0;
    let mut core_count: i32 = 0;
    for_each_pixel_in_roi(image, &core, |_x, _y, pixel_value| {
        core_sum += i32::from(pixel_value);
        core_count += 1;
    });
    let core_mean = core_sum as f64 / core_count as f64;

    // Inner-core check (for blobs >= 3x3).
    if core_width >= 3 && core_height >= 3 {
        let mut outer_core_sum: i32 = 0;
        let mut outer_core_count: i32 = 0;
        for_each_perimeter_pixel(image, &core, |_x, _y, pixel_value| {
            outer_core_sum += i32::from(pixel_value);
            outer_core_count += 1;
        });
        let outer_core_mean = outer_core_sum as f64 / outer_core_count as f64;
        if core_mean < outer_core_mean {
            return None;
        }
    }

    // Neighbor mean (perimeter of neighbors box, excluding corners).
    let mut neighbor_sum: i32 = 0;
    let mut neighbor_count: i32 = 0;
    for_each_perimeter_pixel(image, &neighbors, |x, y, pixel_value| {
        let is_corner = (x == neighbors.left() || x == neighbors.right())
            && (y == neighbors.top() || y == neighbors.bottom());
        if !is_corner {
            neighbor_sum += i32::from(pixel_value);
            neighbor_count += 1;
        }
    });
    let neighbor_mean = neighbor_sum as f64 / neighbor_count as f64;
    if core_mean < neighbor_mean {
        return None;
    }

    // Margin mean (full perimeter of margin box).
    let mut margin_sum: i32 = 0;
    let mut margin_count: i32 = 0;
    for_each_perimeter_pixel(image, &margin, |_x, _y, pixel_value| {
        margin_sum += i32::from(pixel_value);
        margin_count += 1;
    });
    let margin_mean = margin_sum as f64 / margin_count as f64;
    if core_mean <= margin_mean {
        return None;
    }

    // Perimeter statistics: background estimate, min, max, stddev.
    let mut perimeter_sum: i32 = 0;
    let mut perimeter_count: i32 = 0;
    let mut perimeter_min = image
        .get_pixel(perimeter.left() as u32, perimeter.top() as u32)
        .0[0];
    let mut perimeter_max = perimeter_min;
    for_each_perimeter_pixel(image, &perimeter, |_x, _y, pixel_value| {
        perimeter_sum += i32::from(pixel_value);
        perimeter_count += 1;
        if pixel_value < perimeter_min {
            perimeter_min = pixel_value;
        } else if pixel_value > perimeter_max {
            perimeter_max = pixel_value;
        }
    });
    let background_est = perimeter_sum as f64 / perimeter_count as f64;

    // Perimeter stddev.
    let mut perimeter_dev_2: f64 = 0.0;
    for_each_perimeter_pixel(image, &perimeter, |_x, _y, pixel_value| {
        let res = i32::from(pixel_value) as f64 - background_est;
        perimeter_dev_2 += res * res;
    });
    let perimeter_stddev = (perimeter_dev_2 / perimeter_count as f64).sqrt();
    let max_noise_estimate = f64::max(noise_estimate, perimeter_stddev);

    // Perimeter uniformity check (uses noise_estimate, NOT max_noise_estimate).
    if (i32::from(perimeter_max) - i32::from(perimeter_min)) as f64 > 3.0 * sigma * noise_estimate {
        return None;
    }

    // Core significance vs background.
    if core_mean - background_est < sigma * max_noise_estimate {
        return None;
    }

    // Star passes all gates — compute centroid and brightness.
    let brightness;
    let num_saturated;
    let mut x;
    let mut y;
    let peak_value;

    if binning != 1 {
        // Scale margin to higher_res_image coords (multiply by 2), clamp.
        let left = margin.left() as u32 * 2;
        let top = margin.top() as u32 * 2;
        let width = margin.width() * 2;
        let height = margin.height() * 2;
        let adj_width = cmp::min(left + width, higher_res_image.width()) - left;
        let adj_height = cmp::min(top + height, higher_res_image.height()) - top;
        let higher_res_margin = Rect::at(left as i32, top as i32).of_size(adj_width, adj_height);
        (brightness, num_saturated, peak_value) =
            compute_brightness(higher_res_image, &higher_res_margin);
        (x, y) = compute_peak_coord(higher_res_image, &higher_res_margin);
        let upsample = (binning / 2) as f64;
        x *= upsample;
        y *= upsample;
    } else {
        (brightness, num_saturated, peak_value) = compute_brightness(image, &margin);
        (x, y) = compute_peak_coord(image, &margin);
    }

    Some(StarDescription {
        centroid_x: x + 0.5,
        centroid_y: y + 0.5,
        peak_value,
        brightness,
        num_saturated,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GrayImage;

    #[test]
    fn test_for_each_pixel_in_roi_1x1() {
        let image = GrayImage::from_raw(1, 1, vec![127u8]).unwrap();
        let view = crate::as_view(&image);
        let mut pixels = Vec::new();
        for_each_pixel_in_roi(&view, &Rect::at(0, 0).of_size(1, 1), |x, y, p| {
            pixels.push((x, y, p));
        });
        assert_eq!(pixels, vec![(0, 0, 127)]);
    }

    #[test]
    fn test_for_each_pixel_in_roi_3x3() {
        let image = GrayImage::from_raw(3, 3, vec![0, 1, 2, 127, 253, 254, 255, 0, 1]).unwrap();
        let view = crate::as_view(&image);
        let mut pixels = Vec::new();
        for_each_pixel_in_roi(&view, &Rect::at(0, 0).of_size(3, 3), |x, y, p| {
            pixels.push((x, y, p));
        });
        assert_eq!(
            pixels,
            vec![
                (0, 0, 0),
                (1, 0, 1),
                (2, 0, 2),
                (0, 1, 127),
                (1, 1, 253),
                (2, 1, 254),
                (0, 2, 255),
                (1, 2, 0),
                (2, 2, 1),
            ]
        );
    }

    #[test]
    fn test_for_each_perimeter_pixel_3x3() {
        let image = GrayImage::from_raw(3, 3, vec![0, 1, 2, 127, 253, 254, 255, 0, 1]).unwrap();
        let view = crate::as_view(&image);
        let mut pixels = Vec::new();
        for_each_perimeter_pixel(&view, &Rect::at(0, 0).of_size(3, 3), |x, y, p| {
            pixels.push((x, y, p));
        });
        assert_eq!(
            pixels,
            vec![
                (0, 0, 0),
                (1, 0, 1),
                (2, 0, 2),
                (0, 1, 127),
                (2, 1, 254),
                (0, 2, 255),
                (1, 2, 0),
                (2, 2, 1),
            ]
        );
    }

    #[test]
    fn test_peak_coord_1d_single_peak() {
        // Clear peak at index 3: [10, 50, 100, 200, 100, 50, 10]
        let values = vec![10, 50, 100, 200, 100, 50, 10];
        let result = peak_coord_1d(values);
        // Quadratic interp: a=100, b=200, c=100 -> p = 0.5*(100-100)/(100-400+100) = 0
        assert!((result - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_peak_coord_1d_offset_peak() {
        // Peak at index 2 with asymmetric neighbors: [10, 80, 200, 150, 10]
        let values = vec![10, 80, 200, 150, 10];
        let result = peak_coord_1d(values);
        // a=80, b=200, c=150 -> p = 0.5*(80-150)/(80-400+150) = 0.5*(-70)/(-170) = 0.2059
        let expected = 2.0 + 0.5 * (80.0 - 150.0) / (80.0 - 400.0 + 150.0);
        assert!((result - expected).abs() < 1e-9);
    }

    #[test]
    fn test_peak_coord_1d_run_of_peaks() {
        // Run of equal peaks: [10, 200, 200, 200, 10]
        let values = vec![10, 200, 200, 200, 10];
        let result = peak_coord_1d(values);
        // peak_ind=1, run_length=3 -> 1 + (3-1)/2 = 2.0
        assert!((result - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_peak_coord_1d_edge_peak() {
        // Peak at index 0: [200, 100, 50, 10]
        let values = vec![200, 100, 50, 10];
        let result = peak_coord_1d(values);
        assert!((result - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_form_blobs_single_candidate() {
        let candidates = vec![CandidateFrom1D { x: 10, y: 5 }];
        let blobs = form_blobs_from_candidates(candidates, 10);
        assert_eq!(blobs.len(), 1);
        assert_eq!(blobs[0].candidates.len(), 1);
    }

    #[test]
    fn test_form_blobs_merge_adjacent() {
        // Two candidates in adjacent rows, same x -> should merge.
        let candidates = vec![
            CandidateFrom1D { x: 10, y: 5 },
            CandidateFrom1D { x: 10, y: 6 },
        ];
        let blobs = form_blobs_from_candidates(candidates, 10);
        assert_eq!(blobs.len(), 1);
        assert_eq!(blobs[0].candidates.len(), 2);
    }

    #[test]
    fn test_form_blobs_no_merge_far_apart() {
        // Two candidates far apart horizontally -> should not merge.
        let candidates = vec![
            CandidateFrom1D { x: 10, y: 5 },
            CandidateFrom1D { x: 20, y: 6 },
        ];
        let blobs = form_blobs_from_candidates(candidates, 10);
        assert_eq!(blobs.len(), 2);
    }

    #[test]
    fn test_form_blobs_merge_within_tolerance() {
        // Candidates within x-distance of 3 should merge.
        let candidates = vec![
            CandidateFrom1D { x: 10, y: 5 },
            CandidateFrom1D { x: 13, y: 6 }, // exactly at boundary
        ];
        let blobs = form_blobs_from_candidates(candidates, 10);
        assert_eq!(blobs.len(), 1);
        assert_eq!(blobs[0].candidates.len(), 2);
    }

    #[test]
    fn test_form_blobs_no_merge_just_outside() {
        // Candidates at x-distance of 4 should not merge.
        let candidates = vec![
            CandidateFrom1D { x: 10, y: 5 },
            CandidateFrom1D { x: 14, y: 6 }, // just outside boundary
        ];
        let blobs = form_blobs_from_candidates(candidates, 10);
        assert_eq!(blobs.len(), 2);
    }

    #[test]
    fn test_gate_star_2d_reject_too_close_to_edge() {
        // Candidate at x=3, y=3 with binning=1 -> perimeter extends to x=-3 which is < 0
        let candidates = vec![CandidateFrom1D { x: 3, y: 3 }];
        let blob = Blob {
            candidates,
            recipient_blob: None,
        };
        // Small image 20x20
        let image = GrayImage::from_raw(20, 20, vec![50u8; 400]).unwrap();
        let view = crate::as_view(&image);
        let result = gate_star_2d(&blob, &view, &view, 1, 5.0, 8.0, 100, 100);
        assert!(result.is_none());
    }

    #[test]
    fn test_gate_star_2d_reject_too_large() {
        // Blob spanning too wide
        let candidates = vec![
            CandidateFrom1D { x: 10, y: 10 },
            CandidateFrom1D { x: 20, y: 10 },
        ];
        let blob = Blob {
            candidates,
            recipient_blob: None,
        };
        let image = GrayImage::from_raw(100, 100, vec![50u8; 10000]).unwrap();
        let view = crate::as_view(&image);
        let result = gate_star_2d(&blob, &view, &view, 1, 5.0, 8.0, 5, 5);
        assert!(result.is_none(), "core width 11 > max_width 5");
    }

    #[test]
    fn test_compute_brightness_basic() {
        // Create a simple image with a bright center and dark border.
        let mut pixels = vec![20u8; 7 * 7];
        // Set inner pixels (rows 1..=5, cols 1..=5) to higher values
        for y in 1..=5 {
            for x in 1..=5 {
                pixels[y * 7 + x] = 100;
            }
        }
        let image = GrayImage::from_raw(7, 7, pixels).unwrap();
        let view = crate::as_view(&image);
        let box_rect = Rect::at(0, 0).of_size(7, 7);
        let (brightness, num_saturated, peak_value) = compute_brightness(&view, &box_rect);
        assert_eq!(peak_value, 100);
        assert_eq!(num_saturated, 0);
        // brightness should be positive since inner pixels are brighter than border
        assert!(brightness > 0.0);
    }
}
