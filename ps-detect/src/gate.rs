//! 1-D row gate and hot-pixel rejection for star detection.
//!
//! Scans each row with a 7-pixel horizontal gate to find candidate star centers,
//! then rejects isolated hot pixels using full-resolution backing pixel analysis.

use crate::GrayImage;
use std::cmp;

/// Result of the 1-D gate test on a single pixel.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GateResult {
    /// This pixel is a valid star candidate center.
    Candidate,
    /// This pixel does not meet the significance or shape criteria.
    Uninteresting,
}

/// Classification of a single pixel for hot-pixel rejection.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PixelHotType {
    /// Pixel is not significantly brighter than background.
    Dark,
    /// Pixel is bright with neighbor support (real star-like).
    Bright,
    /// Pixel is bright but isolated (hot pixel).
    Hot,
}

/// A candidate star center from the 1-D row scan.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CandidateFrom1D {
    pub x: i32,
    pub y: i32,
}

/// 1-D gate test for a single pixel using a 7-pixel window.
///
/// Gate layout: `|lb lm l C r rm rb|` (indices 0-6).
///
/// All 7 tests in order:
/// 1. **Significance**: `2*C - (lb+rb) >= sigma_noise_2`, else Uninteresting
/// 2. **Local max vs neighbors**: `l <= C && C >= r`, else Uninteresting
/// 3. **Brighter than margins**: `lm < C && C > rm`, else Uninteresting
/// 4. **Tie-break left**: if `l == C` and `lm > r` -> Uninteresting (left pixel claims it)
/// 5. **Tie-break right**: if `C == r` and `l <= rm` -> Uninteresting (right pixel claims it)
/// 6. **Uniform background**: `|lb - rb| <= sigma_noise_3`, else Uninteresting
///
/// If all pass -> `GateResult::Candidate`.
pub fn gate_star_1d(gate: &[u8], sigma_noise_2: i16, sigma_noise_3: i16) -> GateResult {
    assert!(gate.len() == 7, "gate must have exactly 7 pixels");

    let lb = gate[0] as i16;
    let lm = gate[1] as i16;
    let l = gate[2] as i16;
    let c = gate[3] as i16;
    let r = gate[4] as i16;
    let rm = gate[5] as i16;
    let rb = gate[6] as i16;

    // 1. Significance test
    if 2 * c - (lb + rb) < sigma_noise_2 {
        return GateResult::Uninteresting;
    }

    // 2. Local max vs immediate neighbors
    if !(l <= c && c >= r) {
        return GateResult::Uninteresting;
    }

    // 3. Brighter than margins
    if !(lm < c && c > rm) {
        return GateResult::Uninteresting;
    }

    // 4. Tie-break left: if l == C and lm > r, left pixel claims it
    if l == c && lm > r {
        return GateResult::Uninteresting;
    }

    // 5. Tie-break right: if C == r and l <= rm, right pixel claims it
    if c == r && l <= rm {
        return GateResult::Uninteresting;
    }

    // 6. Uniform background: |lb - rb| <= sigma_noise_3
    let bg_diff = if lb > rb { lb - rb } else { rb - lb };
    if bg_diff > sigma_noise_3 {
        return GateResult::Uninteresting;
    }

    GateResult::Candidate
}

/// Classify a single pixel as Dark, Bright, or Hot using a 7-pixel gate.
///
/// Same 7-pixel layout `|lb lm l C r rm rb|`.
///
/// Returns `(PixelHotType, representative_value)`.
pub fn classify_pixel(gate: &[u8], sigma_noise_2: i16) -> (PixelHotType, u8) {
    assert!(gate.len() == 7, "gate must have exactly 7 pixels");

    let lb = gate[0] as i16;
    let l = gate[2] as i16;
    let c = gate[3] as i16;
    let r = gate[4] as i16;
    let rb = gate[6] as i16;

    // 1. Dark test
    if 2 * c - (lb + rb) < sigma_noise_2 {
        return (PixelHotType::Dark, gate[3]);
    }

    // 2. Hot test: 4 * ((l+r) - (lb+rb)) <= (2*C - (lb+rb)) / 2
    let left_side = 4 * ((l + r) - (lb + rb));
    let right_side = (2 * c - (lb + rb)) / 2;
    if left_side <= right_side {
        return (PixelHotType::Hot, ((l + r) / 2) as u8);
    }

    // 3. Bright: has neighbor support
    (PixelHotType::Bright, gate[3])
}

/// Check if ALL backing pixels in a binning block are Dark or Hot (no Bright pixel found).
///
/// Returns `true` if every backing pixel is classified as Dark or Hot (i.e., this is a hot pixel).
pub fn all_bright_are_hot(
    full_res_image: &GrayImage,
    x: i32,
    y: i32,
    binning: u32,
    sigma_noise_2: i16,
) -> bool {
    let binning = binning as i32;
    let (full_width, _full_height) = full_res_image.dimensions();
    let full_width = full_width as i32;

    let x_full = x * binning;
    let y_full = y * binning;

    for by in y_full..y_full + binning {
        for bx in x_full..x_full + binning {
            // Extract 7-pixel gate from row by
            let gate_start = cmp::max(0, bx - 3);
            let gate_end = cmp::min(full_width, bx + 4);

            if (gate_end - gate_start) < 7 {
                // Not enough context, skip this backing pixel (edge of image)
                continue;
            }

            // Build the 7-pixel gate centered on bx
            let raw = full_res_image.as_raw();
            let mut gate = [0u8; 7];
            for i in 0..7i32 {
                let col = gate_start + i;
                gate[i as usize] = raw[(by as usize * full_width as usize) + col as usize];
            }

            let (ptype, _val) = classify_pixel(&gate, sigma_noise_2);
            if ptype == PixelHotType::Bright {
                return false;
            }
        }
    }

    true
}

/// Scan an image row-by-row for 1-D star candidates.
///
/// Uses cache-line sampling (stride=64) for `row_min` and a coarse
/// pre-filter threshold to avoid running the gate on most background pixels.
pub fn scan_image_for_candidates(
    image: &GrayImage,
    noise_estimate: f64,
    sigma: f64,
) -> Vec<CandidateFrom1D> {
    let (width, height) = image.dimensions();
    let width = width as i32;
    let height = height as i32;

    // Compute integer thresholds
    let sigma_noise_2 = cmp::max((2.0 * sigma * noise_estimate + 0.5) as i16, 2);
    let sigma_noise_3 = cmp::max((3.0 * sigma * noise_estimate + 0.5) as i16, 3);

    let raw = image.as_raw();
    let width_usize = width as usize;
    let mut candidates = Vec::new();

    for y in 0..height {
        let row_start = y as usize * width_usize;

        // Sample every 64th pixel to find row_min (cache-line stride)
        let mut row_min = 255u8;
        for x_sample in (0..width).step_by(64) {
            let val = raw[row_start + x_sample as usize];
            if val < row_min {
                row_min = val;
            }
        }

        // Coarse pre-filter threshold
        let threshold = row_min.saturating_add((sigma_noise_2 as u8) / 2);

        // Scan columns, skipping 3 edge columns on each side
        for x in 3..(width - 3) {
            let pixel_val = raw[row_start + x as usize];
            if pixel_val >= threshold {
                // Extract 7-pixel gate: [x-3 .. x+4]
                let gate_start = (x - 3) as usize;
                let gate = &raw[row_start + gate_start..row_start + gate_start + 7];

                if gate_star_1d(gate, sigma_noise_2, sigma_noise_3) == GateResult::Candidate {
                    candidates.push(CandidateFrom1D { x, y });
                }
            }
        }
    }

    candidates
}

/// Filter out candidates that are backed only by hot pixels.
///
/// Returns `(filtered_candidates, hot_pixel_count)`.
pub fn reject_hot_pixels(
    candidates: &[CandidateFrom1D],
    full_res_image: &GrayImage,
    binning: u32,
    sigma_noise_2: i16,
) -> (Vec<CandidateFrom1D>, usize) {
    let mut filtered = Vec::with_capacity(candidates.len());
    let mut hot_pixel_count = 0usize;

    for cand in candidates {
        if binning == 1 {
            // With binning=1, each candidate is a single pixel.
            // Still check hot-pixel status since the spec says it works for binning=1 too.
            let is_hot = all_bright_are_hot(full_res_image, cand.x, cand.y, binning, sigma_noise_2);
            if is_hot {
                hot_pixel_count += 1;
            } else {
                filtered.push(*cand);
            }
        } else {
            let is_hot = all_bright_are_hot(full_res_image, cand.x, cand.y, binning, sigma_noise_2);
            if is_hot {
                hot_pixel_count += 1;
            } else {
                filtered.push(*cand);
            }
        }
    }

    (filtered, hot_pixel_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- gate_star_1d tests ----

    #[test]
    fn test_gate_star_1d_significant_peak() {
        // Clear peak: [20, 25, 30, 200, 30, 25, 20]
        let gate = [20u8, 25, 30, 200, 30, 25, 20];
        assert_eq!(
            gate_star_1d(&gate, 10, 10),
            GateResult::Candidate,
            "strong peak should be a candidate"
        );
    }

    #[test]
    fn test_gate_star_1d_low_significance() {
        // Gentle slope: [100, 101, 102, 105, 102, 101, 100]
        // significance = 2*105 - (100+100) = 110 < 200
        let gate = [100u8, 101, 102, 105, 102, 101, 100];
        assert_eq!(
            gate_star_1d(&gate, 200, 10),
            GateResult::Uninteresting,
            "low significance should be uninteresting"
        );
    }

    #[test]
    fn test_gate_star_1d_not_local_max() {
        // Center not >= right neighbor: [20, 20, 200, 20, 200, 20, 20]
        // l=200 > C=20, fails local max test
        let gate = [20u8, 20, 200, 20, 200, 20, 20];
        assert_eq!(
            gate_star_1d(&gate, 10, 10),
            GateResult::Uninteresting,
            "center not local max should be uninteresting"
        );
    }

    #[test]
    fn test_gate_star_1d_non_uniform_background() {
        // [10, 20, 50, 200, 50, 20, 200] -- |lb-rb| = |10-200| = 190 > sigma_noise_3=10
        let gate = [10u8, 20, 50, 200, 50, 20, 200];
        assert_eq!(
            gate_star_1d(&gate, 10, 10),
            GateResult::Uninteresting,
            "non-uniform background should be uninteresting"
        );
    }

    #[test]
    fn test_gate_star_1d_tie_break_left() {
        // l==C and lm>r: [20, 30, 200, 200, 25, 20, 20]
        // l=200 == C=200, lm=30 > r=25 -> left pixel claims it
        let gate = [20u8, 30, 200, 200, 25, 20, 20];
        assert_eq!(
            gate_star_1d(&gate, 10, 10),
            GateResult::Uninteresting,
            "tie-break left should be uninteresting"
        );
    }

    #[test]
    fn test_gate_star_1d_tie_break_right() {
        // C==r and l<=rm: [20, 25, 200, 200, 30, 20, 20]
        // C=200 == r=30? No. Let me reconsider.
        // Actually: l=200, C=200, r=30. l==C is true. lm=25 > r=30? No (25 < 30).
        // So tie-break left doesn't fire. Then check tie-break right: C==r? 200==30? No.
        // So it would pass. Let me construct the correct test case.
        // For tie-break right: C==r and l<=rm.
        // Gate: [20, 25, 10, 200, 200, 30, 20]
        // C=200, r=200 -> C==r is true. l=10 <= rm=30 -> true. Uninteresting.
        let gate = [20u8, 25, 10, 200, 200, 30, 20];
        assert_eq!(
            gate_star_1d(&gate, 10, 10),
            GateResult::Uninteresting,
            "tie-break right should be uninteresting"
        );
    }

    // ---- classify_pixel tests ----

    #[test]
    fn test_classify_pixel_dark() {
        // Below significance: [100, 100, 100, 102, 100, 100, 100]
        // 2*102 - (100+100) = 4 < 50
        let gate = [100u8, 100, 100, 102, 100, 100, 100];
        let (ptype, val) = classify_pixel(&gate, 50);
        assert_eq!(ptype, PixelHotType::Dark);
        assert_eq!(val, 102);
    }

    #[test]
    fn test_classify_pixel_hot() {
        // Isolated bright pixel: [10, 10, 10, 200, 10, 10, 10]
        // significance = 2*200 - (10+10) = 380 >= 50 -> not dark
        // Hot test: 4 * ((10+10) - (10+10)) = 0 <= (380)/2 = 190 -> true, Hot
        let gate = [10u8, 10, 10, 200, 10, 10, 10];
        let (ptype, val) = classify_pixel(&gate, 50);
        assert_eq!(ptype, PixelHotType::Hot);
        // representative value = (l+r)/2 = (10+10)/2 = 10
        assert_eq!(val, 10);
    }

    #[test]
    fn test_classify_pixel_bright() {
        // Bright pixel with neighbor support: [10, 15, 150, 200, 150, 15, 10]
        // significance = 2*200 - (10+10) = 380 >= 50 -> not dark
        // Hot test: 4 * ((150+150) - (10+10)) = 4 * 280 = 1120
        // right_side = 380 / 2 = 190
        // 1120 <= 190? No -> Bright
        let gate = [10u8, 15, 150, 200, 150, 15, 10];
        let (ptype, val) = classify_pixel(&gate, 50);
        assert_eq!(ptype, PixelHotType::Bright);
        assert_eq!(val, 200);
    }

    // ---- scan_image_for_candidates tests ----

    #[test]
    fn test_scan_uniform_row_no_candidates() {
        // 256-wide image with all pixels = 100
        let img = GrayImage::from_raw(256, 1, vec![100u8; 256]).unwrap();
        let candidates = scan_image_for_candidates(&img, 10.0, 8.0);
        // sigma_noise_2 = max((2*8*10+0.5) as i16, 2) = max(160, 2) = 160
        // threshold = 100 + 160/2 = 180
        // No pixel >= 180, so no candidates
        assert!(
            candidates.is_empty(),
            "uniform image should have no candidates"
        );
    }

    #[test]
    fn test_scan_single_peak() {
        // Create a 256-wide row with a peak at position 100
        let mut row = vec![20u8; 256];
        row[97] = 25; // lb
        row[98] = 30; // lm
        row[99] = 50; // l
        row[100] = 200; // C (peak)
        row[101] = 50; // r
        row[102] = 30; // rm
        row[103] = 25; // rb

        let img = GrayImage::from_raw(256, 1, row).unwrap();
        let candidates = scan_image_for_candidates(&img, 1.0, 8.0);
        // sigma_noise_2 = max((2*8*1+0.5) as i16, 2) = max(16, 2) = 16
        // sigma_noise_3 = max((3*8*1+0.5) as i16, 3) = max(24, 3) = 24
        // threshold = 20 + 16/2 = 28
        // row[100] = 200 >= 28, gate passes
        assert_eq!(candidates.len(), 1, "should find exactly one candidate");
        assert_eq!(candidates[0].x, 100);
        assert_eq!(candidates[0].y, 0);
    }

    #[test]
    fn test_edge_columns_skipped() {
        // Peak at x=1 should not produce a candidate (within 3 edge columns)
        let mut row = vec![20u8; 256];
        // Place a bright pixel at x=1
        row[1] = 200;

        let img = GrayImage::from_raw(256, 1, row).unwrap();
        let candidates = scan_image_for_candidates(&img, 1.0, 8.0);
        // x=1 is within the first 3 columns, so it should be skipped
        assert!(
            candidates.is_empty(),
            "peak at x=1 should not produce a candidate (edge column)"
        );
    }

    // ---- all_bright_are_hot tests ----

    #[test]
    fn test_all_bright_are_hot() {
        // Create a small image with an isolated bright pixel at (5, 5)
        let mut pixels = vec![20u8; 16 * 16];
        // Set up an isolated hot pixel: no neighbor support
        pixels[5 + 5 * 16] = 200; // the bright pixel
                                  // Neighbors stay at 20

        let img = GrayImage::from_raw(16, 16, pixels).unwrap();

        // sigma_noise_2 = 50 (large enough that the pixel passes significance)
        // For the gate centered on (5,5): [20,20,20,200,20,20,20]
        // classify_pixel: significance = 2*200-(20+20)=360 >= 50 -> not dark
        // Hot test: 4*((20+20)-(20+20))=0 <= 360/2=180 -> true, Hot
        let result = all_bright_are_hot(&img, 5, 5, 1, 50);
        assert!(result, "isolated bright pixel should be classified as hot");
    }

    #[test]
    fn test_all_bright_are_hot_with_neighbor_support() {
        // Create an image where the bright pixel has neighbor support
        let mut pixels = vec![20u8; 16 * 16];
        pixels[5 + 5 * 16] = 200; // C
        pixels[4 + 5 * 16] = 150; // l (left neighbor)
        pixels[6 + 5 * 16] = 150; // r (right neighbor)

        let img = GrayImage::from_raw(16, 16, pixels).unwrap();

        // Gate: [20,20,150,200,150,20,20]
        // classify_pixel: significance = 2*200-(20+20)=360 >= 50 -> not dark
        // Hot test: 4*((150+150)-(20+20))=4*260=1040 <= 360/2=180? No -> Bright
        let result = all_bright_are_hot(&img, 5, 5, 1, 50);
        assert!(!result, "pixel with neighbor support should NOT be all-hot");
    }

    // ---- reject_hot_pixels tests ----

    #[test]
    fn test_reject_hot_pixels_empty() {
        let img = GrayImage::from_raw(16, 16, vec![20u8; 256]).unwrap();
        let (filtered, hot_count) = reject_hot_pixels(&[], &img, 1, 50);
        assert!(filtered.is_empty());
        assert_eq!(hot_count, 0);
    }

    #[test]
    fn test_reject_hot_pixels_all_hot() {
        // Image with isolated bright pixels
        let mut pixels = vec![20u8; 32 * 32];
        pixels[10 + 10 * 32] = 200;

        let img = GrayImage::from_raw(32, 32, pixels).unwrap();

        let candidates = vec![CandidateFrom1D { x: 10, y: 10 }];
        let (filtered, hot_count) = reject_hot_pixels(&candidates, &img, 1, 50);

        assert!(filtered.is_empty(), "isolated pixel should be rejected");
        assert_eq!(hot_count, 1);
    }

    #[test]
    fn test_reject_hot_pixels_keeps_real_stars() {
        // Image with a bright pixel that has neighbor support
        let mut pixels = vec![20u8; 32 * 32];
        pixels[15 + 15 * 32] = 200; // C
        pixels[14 + 15 * 32] = 150; // l
        pixels[16 + 15 * 32] = 150; // r

        let img = GrayImage::from_raw(32, 32, pixels).unwrap();

        let candidates = vec![CandidateFrom1D { x: 15, y: 15 }];
        let (filtered, hot_count) = reject_hot_pixels(&candidates, &img, 1, 50);

        assert_eq!(filtered.len(), 1, "real star should be kept");
        assert_eq!(hot_count, 0);
    }
}
