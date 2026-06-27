//! Noise and background estimation from image regions.
//!
//! Estimates RMS noise robustly using three de-starred midline cuts.
//! The darkest cut (lowest mean) is selected to avoid bright interlopers.

use crate::histogram::{remove_stars_from_histogram, stats_for_histogram, HistogramStats};
use crate::GrayImage;
use imageproc::rect::Rect;

/// Build a 256-bin histogram of all pixels in the given ROI.
fn histogram_for_roi(image: &GrayImage, roi: &Rect) -> [u32; 256] {
    let mut histogram = [0_u32; 256];
    let (width, _height) = image.dimensions();
    let width = width as usize;
    assert!(roi.left() >= 0);
    assert!(roi.top() >= 0);
    let raw = image.as_raw();
    let left = roi.left() as usize;
    let top = roi.top() as usize;
    let bottom = roi.bottom() as usize;
    let w = roi.width() as usize;
    if w == 0 || roi.height() == 0 {
        return histogram;
    }
    for y in top..=bottom {
        let row_start = y * width;
        let row_slice = &raw[row_start + left..row_start + left + w];
        for &pixel in row_slice.iter() {
            histogram[pixel as usize] += 1;
        }
    }
    histogram
}

/// Compute de-starred statistics for the given ROI.
fn stats_for_roi(image: &GrayImage, roi: &Rect) -> HistogramStats {
    let mut hist = histogram_for_roi(image, roi);
    remove_stars_from_histogram(&mut hist, 8.0);
    stats_for_histogram(&hist)
}

/// Estimate the RMS noise of the given image.
///
/// Samples three horizontal cuts along the image's midline, computes
/// de-starred statistics for each, and returns the standard deviation
/// of the darkest cut (lowest mean).
pub fn estimate_noise_from_image(image: &GrayImage) -> f64 {
    let (width, height) = image.dimensions();

    let cut_size = std::cmp::min(50, width / 4);

    // Sample three areas across the horizontal midline of the image.
    let mut stats_arr = [
        stats_for_roi(
            image,
            &Rect::at((width / 4 - cut_size / 2) as i32, (height / 2) as i32).of_size(cut_size, 1),
        ),
        stats_for_roi(
            image,
            &Rect::at((width * 2 / 4 - cut_size / 2) as i32, (height / 2) as i32)
                .of_size(cut_size, 1),
        ),
        stats_for_roi(
            image,
            &Rect::at((width * 3 / 4 - cut_size / 2) as i32, (height / 2) as i32)
                .of_size(cut_size, 1),
        ),
    ];
    // Pick the darkest cut by mean value.
    stats_arr.sort_by(|a, b| {
        a.mean
            .partial_cmp(&b.mean)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    stats_arr[0].stddev
}

/// Estimate the background and noise level of the given image region.
///
/// Returns `(mean, stddev)` for the de-starred pixel distribution in `roi`.
pub fn estimate_background_from_image_region(image: &GrayImage, roi: &Rect) -> (f64, f64) {
    let stats = stats_for_roi(image, roi);
    (stats.mean, stats.stddev)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GrayImage as ImgGrayImage;

    #[test]
    fn test_histogram_uniform() {
        // 10x10 image with all pixels = 50
        let mut img = ImgGrayImage::new(10, 10);
        for pixel in img.pixels_mut() {
            pixel.0[0] = 50;
        }
        let roi = Rect::at(0, 0).of_size(10, 10);
        let hist = histogram_for_roi(&img, &roi);
        assert_eq!(hist[50], 100);
        assert_eq!(hist.iter().sum::<u32>(), 100);
    }

    #[test]
    fn test_histogram_for_roi_partial() {
        // 10x10 image, ROI is 5x5 at (2,2)
        let mut img = ImgGrayImage::new(10, 10);
        for pixel in img.pixels_mut() {
            pixel.0[0] = 10;
        }
        // Set a few pixels in the ROI to 20
        for y in 2..7 {
            for x in 2..7 {
                img.put_pixel(x, y, image::Luma([20]));
            }
        }
        let roi = Rect::at(2, 2).of_size(5, 5);
        let hist = histogram_for_roi(&img, &roi);
        // All 25 pixels in ROI should be value 20
        assert_eq!(hist[20], 25);
    }

    #[test]
    fn test_estimate_noise_uniform_image() {
        // Uniform image has zero noise
        let img = ImgGrayImage::new(100, 100);
        let noise = estimate_noise_from_image(&img);
        assert_eq!(noise, 0.0);
    }

    #[test]
    fn test_estimate_background_uniform() {
        // All pixels = 50
        let mut img = ImgGrayImage::new(100, 50);
        for pixel in img.pixels_mut() {
            pixel.0[0] = 50;
        }
        let roi = Rect::at(0, 0).of_size(100, 50);
        let (mean, stddev) = estimate_background_from_image_region(&img, &roi);
        assert_eq!(mean, 50.0);
        assert_eq!(stddev, 0.0);
    }
}
