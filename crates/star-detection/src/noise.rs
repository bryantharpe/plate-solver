//! Robust RMS noise estimation from an 8-bit grayscale image.
//!
//! The estimator takes three 1-pixel-tall horizontal cuts across the image,
//! removes bright star-like outliers from each cut, and returns the standard
//! deviation of the darkest remaining cut (by mean). A floor of `0.2` is
//! applied so the detection threshold never collapses to zero on black-clipped
//! backgrounds.

/// Minimum returned noise level, in ADU.
const NOISE_FLOOR: f64 = 0.2;

/// Fraction of the cut width treated as bright outliers and discarded before
/// computing statistics. This matches the "de-star" step in the specification.
const OUTLIER_FRACTION: f64 = 0.1;

/// Estimate RMS noise from a grayscale image.
///
/// `image` is an 8-bit grayscale buffer laid out row-major with `width` pixels
/// per row. The estimator:
///
/// 1. Takes three 1-pixel-tall horizontal cuts of width `min(50, width/4)`
///    centered vertically on the image and horizontally at approximately
///    `width/4`, `width/2`, and `3*width/4`.
/// 2. For each cut, discards the brightest 10% of pixels (de-starring).
/// 3. Computes the mean and standard deviation of the remaining pixels.
/// 4. Returns the standard deviation of the cut with the lowest mean,
///    clamped to at least `0.2`.
///
/// # Panics
///
/// Panics if `image.len()` is not exactly `width * height`.
pub fn estimate_noise(image: &[u8], width: usize, height: usize) -> f64 {
    assert_eq!(
        image.len(),
        width * height,
        "image length must equal width * height"
    );

    if width == 0 || height == 0 {
        return NOISE_FLOOR;
    }

    let cut_width = (width / 4).clamp(1, 50);
    let cut_half = cut_width / 2;
    let mid_y = height / 2;

    let centers_x = [width / 4, width / 2, 3 * width / 4];

    let mut best_stddev = 0.0f64;
    let mut best_mean = f64::INFINITY;

    for &cx in &centers_x {
        let start_x = cx.saturating_sub(cut_half).min(width.saturating_sub(cut_width));
        let y = mid_y.min(height - 1);
        let row_offset = y * width;

        let mut pixels: Vec<u8> = image[row_offset + start_x..row_offset + start_x + cut_width]
            .to_vec();

        // De-star: discard the brightest outliers.
        pixels.sort_unstable();
        let keep = (pixels.len() as f64 * (1.0 - OUTLIER_FRACTION)).floor() as usize;
        let kept = &pixels[..keep.max(1).min(pixels.len())];

        let mean = mean_u8(kept);
        let stddev = stddev_u8(kept, mean);

        if mean < best_mean {
            best_mean = mean;
            best_stddev = stddev;
        }
    }

    best_stddev.max(NOISE_FLOOR)
}

fn mean_u8(values: &[u8]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let sum: u64 = values.iter().map(|&v| u64::from(v)).sum();
    sum as f64 / values.len() as f64
}

fn stddev_u8(values: &[u8], mean: f64) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let variance: f64 = values
        .iter()
        .map(|&v| {
            let d = f64::from(v) - mean;
            d * d
        })
        .sum::<f64>()
        / (values.len() - 1) as f64;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn darkest_cut_is_chosen() {
        // 100x10 image. Cut 1 and 3 are dark background (mean ~10). Cut 2 is
        // bright (mean ~100). The estimator must pick a dark cut.
        let width = 100;
        let height = 10;
        let mut image = vec![10u8; width * height];

        // Bright interloper over the middle cut: x in [45, 55], all rows.
        for y in 0..height {
            for x in 45..=55 {
                image[y * width + x] = 200;
            }
        }

        let noise = estimate_noise(&image, width, height);
        // Dark cuts have stddev ~0; floor applies.
        assert!((noise - 0.2).abs() < 1e-9, "expected floor 0.2, got {}", noise);
    }

    #[test]
    fn noise_floor_is_applied() {
        // Completely black image: measured stddev is 0, so floor must apply.
        let image = vec![0u8; 80 * 20];
        let noise = estimate_noise(&image, 80, 20);
        assert!((noise - 0.2).abs() < 1e-9, "expected 0.2, got {}", noise);
    }

    #[test]
    fn noisy_background_returns_above_floor() {
        // 200x20 image with Gaussian-ish noise around 50 ADU.
        let width = 200;
        let height = 20;
        let mut image = vec![50u8; width * height];
        // Add small perturbations to every pixel.
        for (i, p) in image.iter_mut().enumerate() {
            let delta = ((i % 7) as i32) - 3;
            *p = (*p as i32 + delta).clamp(0, 255) as u8;
        }

        let noise = estimate_noise(&image, width, height);
        assert!(noise > 0.2, "expected noise above floor, got {}", noise);
    }
}
