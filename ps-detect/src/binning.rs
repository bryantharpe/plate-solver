//! Binning cascade and row normalization for star detection.
//!
//! Supports binning factors 1, 2, 4, 8 via repeated 2x2 box filtering.
//! Detection runs on the most-binned image; centroiding uses one level less binned.

use crate::GrayImage;
use crate::histogram::estimate_dark_level;
use std::sync::OnceLock;

/// Result of a 2x2 binning operation including a pixel histogram.
pub struct Binned2x2Result {
    pub binned: GrayImage,
    pub histogram: [u32; 256],
}

/// Function type for 2x2 binning with histogram.
pub type BinAndHistoFn = fn(&GrayImage, bool) -> Binned2x2Result;
/// Function type for 2x2 binning without histogram.
pub type Bin2x2Fn = fn(&GrayImage) -> GrayImage;

static BIN_AND_HISTO_FN: OnceLock<BinAndHistoFn> = OnceLock::new();
static BIN2X2_FN: OnceLock<Bin2x2Fn> = OnceLock::new();

/// Registers pluggable binning functions via OnceLock.
///
/// Allows SIMD/accelerated replacements at runtime. Subsequent calls
/// are no-ops (OnceLock ignores them).
pub fn set_binner(bin_and_histo: BinAndHistoFn, bin2x2: Bin2x2Fn) {
    let _ = BIN_AND_HISTO_FN.set(bin_and_histo);
    let _ = BIN2X2_FN.set(bin2x2);
}

/// Bin an image 2x2, dispatching to a registered function or the default.
pub fn bin_2x2(image: &GrayImage) -> GrayImage {
    match BIN2X2_FN.get() {
        Some(f) => f(image),
        None => bin_2x2_default(image),
    }
}

/// Default 2x2 box-filter implementation.
///
/// Each output pixel is the integer average of a 2x2 block:
/// `(p1 + p2 + p3 + p4) / 4` using u16 accumulation then divide by 4.
/// Odd trailing rows/columns are dropped via `& !1` masking.
fn bin_2x2_default(image: &GrayImage) -> GrayImage {
    let (width, height) = image.dimensions();
    let new_width = width / 2;
    let new_height = height / 2;
    let mut resized_image = Vec::with_capacity((new_width * new_height) as usize);
    let source_pixels = image.as_raw();
    for y in (0..height & !1).step_by(2) {
        for x in (0..width & !1).step_by(2) {
            let p1 = source_pixels[(y * width + x) as usize] as u16;
            let p2 = source_pixels[(y * width + x + 1) as usize] as u16;
            let p3 = source_pixels[((y + 1) * width + x) as usize] as u16;
            let p4 = source_pixels[((y + 1) * width + x + 1) as usize] as u16;
            resized_image.push(((p1 + p2 + p3 + p4) / 4) as u8);
        }
    }
    GrayImage::from_raw(new_width, new_height, resized_image).unwrap()
}

/// Bin an image 2x2 with histogram, dispatching to a registered function or the default.
///
/// If `normalize_rows` is true, row normalization is applied before binning.
pub fn bin_and_histogram_2x2(image: &GrayImage, normalize_rows: bool) -> Binned2x2Result {
    match BIN_AND_HISTO_FN.get() {
        Some(f) => f(image, normalize_rows),
        None => bin_and_histogram_2x2_default(image, normalize_rows),
    }
}

/// Default 2x2 box-filter with histogram construction.
///
/// If `normalize_rows` is true, applies row normalization before binning.
fn bin_and_histogram_2x2_default(image: &GrayImage, normalize_rows: bool) -> Binned2x2Result {
    let normalized;
    let source_image = if normalize_rows {
        normalized = apply_row_normalization(image);
        &normalized
    } else {
        image
    };
    let (width, height) = source_image.dimensions();

    // 2x2 box filter.
    let new_width = width / 2;
    let new_height = height / 2;
    let mut resized_image = Vec::with_capacity((new_width * new_height) as usize);
    let mut histogram = [0u32; 256];

    let source_pixels = source_image.as_raw();

    for y in (0..height & !1).step_by(2) {
        for x in (0..width & !1).step_by(2) {
            let p1 = source_pixels[(y * width + x) as usize] as u16;
            let p2 = source_pixels[(y * width + x + 1) as usize] as u16;
            let p3 = source_pixels[((y + 1) * width + x) as usize] as u16;
            let p4 = source_pixels[((y + 1) * width + x + 1) as usize] as u16;

            let avg = ((p1 + p2 + p3 + p4) / 4) as u8;

            resized_image.push(avg);
            histogram[avg as usize] += 1;
        }
    }

    let output_image = GrayImage::from_raw(new_width, new_height, resized_image).unwrap();
    Binned2x2Result { binned: output_image, histogram }
}

/// Apply per-row dark-level normalization to an image.
///
/// For each row:
/// 1. Build a per-row histogram.
/// 2. Estimate the dark level (mean of bottom 1%).
/// 3. Compute `adjust = round(2.0 - dark)`.
/// 4. Shift every pixel by `adjust`, clamping to [0, 255].
///
/// Returns a new GrayImage of the same dimensions.
fn apply_row_normalization(image: &GrayImage) -> GrayImage {
    let (width, height) = image.dimensions();
    let mut normalized_pixels = Vec::with_capacity((width * height) as usize);
    let source_pixels = image.as_raw();

    for y in 0..height {
        // Build histogram for this row.
        let mut row_histogram = [0u32; 256];
        let row_start = (y * width) as usize;
        let row_end = ((y + 1) * width) as usize;

        for &pixel in &source_pixels[row_start..row_end] {
            row_histogram[pixel as usize] += 1;
        }

        // Get estimated dark level for this row.
        let row_dark_level = estimate_dark_level(&row_histogram, width as usize);
        let bias = 2.0_f32;
        let adjust = (bias - row_dark_level).round() as i16;

        // Normalize row pixels.
        for &pixel in &source_pixels[row_start..row_end] {
            let adjusted = pixel as i16 + adjust;
            let normalized = adjusted.clamp(0, 255) as u8;
            normalized_pixels.push(normalized);
        }
    }

    GrayImage::from_raw(width, height, normalized_pixels).unwrap()
}

/// Build a binning cascade for the given image.
///
/// Returns `(detect_image, higher_res_image, binned_2x_for_return)`:
///
/// | binning | detect_image          | higher_res_image    |
/// |---------|----------------------|---------------------|
/// | 1       | input                | Some(input)         |
/// | 2       | bin2x2(input)        | Some(input)         |
/// | 4       | bin2x2(bin2x2(input))| Some(bin2x2(input)) |
/// | 8       | one more bin2x2      | Some(4x image)      |
///
/// `normalize_rows` is applied on the first binning step only.
/// The third return value is always `None` (reserved for future use).
pub fn build_binning_cascade(
    image: &GrayImage,
    binning: u32,
    normalize_rows: bool,
) -> (GrayImage, Option<GrayImage>, Option<GrayImage>) {
    match binning {
        1 => (image.clone(), Some(image.clone()), None),
        2 => {
            let binned = bin_and_histogram_2x2(image, normalize_rows).binned;
            (binned, Some(image.clone()), None)
        }
        4 => {
            let step1 = bin_and_histogram_2x2(image, normalize_rows).binned;
            let step2 = bin_and_histogram_2x2(&step1, false).binned;
            (step2, Some(step1), None)
        }
        8 => {
            let step1 = bin_and_histogram_2x2(image, normalize_rows).binned;
            let step2 = bin_and_histogram_2x2(&step1, false).binned;
            let step3 = bin_and_histogram_2x2(&step2, false).binned;
            (step3, Some(step2), None)
        }
        _ => panic!("binning must be 1, 2, 4, or 8, got {}", binning),
    }
}

/// Compute the maximum star size for the given width and binning factor.
///
/// - `binning == 1`: `width / 100`
/// - `binning > 1`: `width / 100 / binning + 1`
pub fn compute_max_size(width: u32, binning: u32) -> u32 {
    if binning == 1 {
        width / 100
    } else {
        width / 100 / binning + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bin_2x2_uniform() {
        // Uniform 100x100 image of value 128
        let img = GrayImage::from_raw(100, 100, vec![128u8; 100 * 100]).unwrap();
        let result = bin_2x2(&img);
        assert_eq!(result.dimensions(), (50, 50));
        for pixel in result.pixels() {
            assert_eq!(pixel[0], 128);
        }
    }

    #[test]
    fn test_bin_2x2_known_pattern() {
        // 4x4 image with values 1..16 (row-major: row y has y*4+1 .. y*4+4)
        let mut pixels = Vec::with_capacity(16);
        for y in 0..4u32 {
            for x in 0..4u32 {
                pixels.push((y * 4 + x + 1) as u8);
            }
        }
        let img = GrayImage::from_raw(4, 4, pixels).unwrap();
        let result = bin_2x2(&img);
        assert_eq!(result.dimensions(), (2, 2));

        // Top-left: [1,2,5,6] -> avg=3
        assert_eq!(result.get_pixel(0, 0)[0], 3);
        // Top-right: [3,4,7,8] -> avg=5
        assert_eq!(result.get_pixel(1, 0)[0], 5);
        // Bottom-left: [9,10,13,14] -> avg=11
        assert_eq!(result.get_pixel(0, 1)[0], 11);
        // Bottom-right: [11,12,15,16] -> avg=13
        assert_eq!(result.get_pixel(1, 1)[0], 13);
    }

    #[test]
    fn test_bin_2x2_odd_dimensions() {
        // 5x5 image: rows 0-4, cols 0-4; output should be 2x2 (last row/col dropped)
        let img = GrayImage::from_raw(5, 5, vec![42u8; 25]).unwrap();
        let result = bin_2x2(&img);
        assert_eq!(result.dimensions(), (2, 2));
        for pixel in result.pixels() {
            assert_eq!(pixel[0], 42);
        }
    }

    #[test]
    fn test_row_normalization_dark_row() {
        // Image with 2 rows: row 0 all 10, row 1 all 50. Width=100.
        let mut pixels = vec![10u8; 100];
        pixels.extend_from_slice(&vec![50u8; 100]);
        let img = GrayImage::from_raw(100, 2, pixels).unwrap();

        let normalized = apply_row_normalization(&img);
        assert_eq!(normalized.dimensions(), (100, 2));

        // Row 0: dark_level ~ 10.0 (all pixels are 10), adjust = round(2.0 - 10.0) = -8
        // So pixel = 10 + (-8) = 2
        let row0_pixel = normalized.get_pixel(0, 0)[0];
        assert_eq!(row0_pixel, 2);

        // Row 1: dark_level ~ 50.0, adjust = round(2.0 - 50.0) = -48
        // So pixel = 50 + (-48) = 2
        let row1_pixel = normalized.get_pixel(0, 1)[0];
        assert_eq!(row1_pixel, 2);
    }

    #[test]
    fn test_binning_cascade_1() {
        let img = GrayImage::from_raw(100, 100, vec![77u8; 100 * 100]).unwrap();
        let (detect, higher_res, _extra) = build_binning_cascade(&img, 1, false);
        assert_eq!(detect.dimensions(), (100, 100));
        assert!(higher_res.is_some());
        assert_eq!(higher_res.unwrap().dimensions(), (100, 100));
    }

    #[test]
    fn test_binning_cascade_2() {
        let img = GrayImage::from_raw(100, 100, vec![77u8; 100 * 100]).unwrap();
        let (detect, higher_res, _extra) = build_binning_cascade(&img, 2, false);
        assert_eq!(detect.dimensions(), (50, 50));
        assert!(higher_res.is_some());
        assert_eq!(higher_res.unwrap().dimensions(), (100, 100));
    }

    #[test]
    fn test_binning_cascade_4() {
        let img = GrayImage::from_raw(100, 100, vec![77u8; 100 * 100]).unwrap();
        let (detect, higher_res, _extra) = build_binning_cascade(&img, 4, false);
        assert_eq!(detect.dimensions(), (25, 25));
        assert!(higher_res.is_some());
        assert_eq!(higher_res.unwrap().dimensions(), (50, 50));
    }

    #[test]
    fn test_binning_cascade_invalid() {
        let img = GrayImage::from_raw(100, 100, vec![77u8; 100 * 100]).unwrap();
        let result = std::panic::catch_unwind(|| {
            build_binning_cascade(&img, 3, false);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_max_size() {
        // binning=1: width / 100
        assert_eq!(compute_max_size(2048, 1), 20);
        assert_eq!(compute_max_size(1000, 1), 10);

        // binning>1: width / 100 / binning + 1
        assert_eq!(compute_max_size(2048, 2), 11); // 2048/100=20, 20/2=10, +1=11
        assert_eq!(compute_max_size(2048, 4), 6);  // 2048/100=20, 20/4=5, +1=6
        assert_eq!(compute_max_size(2048, 8), 3);  // 2048/100=20, 20/8=2, +1=3
    }

    #[test]
    fn test_bin_and_histogram_consistency() {
        // bin_and_histogram_2x2 with normalize_rows=false should produce
        // the same pixels as bin_2x2.
        let mut pixels = Vec::with_capacity(16);
        for y in 0..4u32 {
            for x in 0..4u32 {
                pixels.push((y * 4 + x + 1) as u8);
            }
        }
        let img = GrayImage::from_raw(4, 4, pixels).unwrap();

        let binned = bin_2x2(&img);
        let result = bin_and_histogram_2x2(&img, false);
        assert_eq!(binned.as_raw(), result.binned.as_raw());

        // Histogram should sum to output pixel count
        let total: u32 = result.histogram.iter().sum();
        assert_eq!(total, 4);
    }

    #[test]
    fn test_set_binner_pluggable() {
        // Verify set_binner doesn't panic and dispatch still works.
        // OnceLock only accepts once; since tests run in parallel, just
        // verify it doesn't crash on a double-set attempt.
        set_binner(bin_and_histogram_2x2_default, bin_2x2_default);

        let img = GrayImage::from_raw(4, 4, vec![100u8; 16]).unwrap();
        let result = bin_2x2(&img);
        assert_eq!(result.dimensions(), (2, 2));
    }
}
