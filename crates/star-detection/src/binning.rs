//! Binning cascade for multi-scale star detection.
//!
//! Supports binning factors 1, 2, 4, and 8. Each 2×2 bin is the integer average
//! `(p1 + p2 + p3 + p4) / 4`. The cascade also computes `max_size` for the
//! downstream 2-D gate and can optionally normalize each row's dark level to a
//! fixed bias before binning.

/// Supported binning factors.
pub const BINNING_FACTORS: [usize; 4] = [1, 2, 4, 8];

/// Result of one level of the binning cascade.
#[derive(Clone, Debug, PartialEq)]
pub struct BinnedImage {
    /// Binned pixel data, row-major.
    pub data: Vec<u8>,
    /// Width of the binned image in pixels.
    pub width: usize,
    /// Height of the binned image in pixels.
    pub height: usize,
    /// Binning factor that produced this image relative to the original input.
    pub binning: usize,
    /// Maximum allowed core size for the 2-D gate at this binning level.
    pub max_size: usize,
}

impl BinnedImage {
    /// Pixel value at `(y, x)` in binned coordinates.
    ///
    /// Returns `None` if the coordinate is out of bounds.
    pub fn get(&self, y: usize, x: usize) -> Option<u8> {
        if y < self.height && x < self.width {
            Some(self.data[y * self.width + x])
        } else {
            None
        }
    }
}

/// Build a binning cascade for the given input image.
///
/// `image` is an 8-bit grayscale buffer laid out row-major with `width` pixels
/// per row. `binning` must be one of 1, 2, 4, or 8. When `normalize_rows` is
/// true, each row's dark level is shifted to a fixed bias of `2.0` before
/// binning.
///
/// Returns the most-binned detection image and, for `binning > 1`, the
/// one-level-less-binned higher-resolution image used for centroiding.
///
/// # Panics
///
/// Panics if `image.len()` is not exactly `width * height`, or if `binning`
/// is not a supported factor.
pub fn build_cascade(
    image: &[u8],
    width: usize,
    height: usize,
    binning: usize,
    normalize_rows: bool,
) -> (BinnedImage, Option<BinnedImage>) {
    assert!(
        BINNING_FACTORS.contains(&binning),
        "unsupported binning factor: {binning}"
    );
    assert_eq!(
        image.len(),
        width * height,
        "image length must equal width * height"
    );

    // Build the full cascade from the original resolution up to `binning`.
    let mut levels: Vec<BinnedImage> = Vec::with_capacity(4);
    let mut current = if normalize_rows {
        normalize_rows_to_bias(image, width, height, 2.0)
    } else {
        image.to_vec()
    };
    let mut current_width = width;
    let mut current_height = height;

    levels.push(BinnedImage {
        data: current.clone(),
        width: current_width,
        height: current_height,
        binning: 1,
        max_size: max_size(width, 1),
    });

    let mut current_binning = 1usize;
    while current_binning < binning {
        (current, current_width, current_height) =
            bin_by_two(&current, current_width, current_height);
        current_binning *= 2;
        levels.push(BinnedImage {
            data: current.clone(),
            width: current_width,
            height: current_height,
            binning: current_binning,
            max_size: max_size(width, current_binning),
        });
    }

    let detection = levels.pop().expect("cascade has at least one level");
    let higher_res = levels.pop();
    (detection, higher_res)
}

/// Compute `max_size` for the 2-D gate.
///
/// * `binning == 1`: `width / 100`
/// * `binning > 1`: `width / 100 / binning + 1`
pub fn max_size(full_width: usize, binning: usize) -> usize {
    if binning == 1 {
        (full_width / 100).max(1)
    } else {
        (full_width / 100 / binning + 1).max(1)
    }
}

/// Bin an image by a factor of 2 in each dimension.
///
/// Each 2×2 block is replaced by the integer average of its four pixels.
/// Odd widths/heights drop the last row/column, matching the spec's integer
/// average semantics.
fn bin_by_two(image: &[u8], width: usize, height: usize) -> (Vec<u8>, usize, usize) {
    let new_width = width / 2;
    let new_height = height / 2;
    if new_width == 0 || new_height == 0 {
        return (image.to_vec(), width, height);
    }

    let mut out = Vec::with_capacity(new_width * new_height);
    for y in 0..new_height {
        for x in 0..new_width {
            let base = 2 * y * width + 2 * x;
            let sum = u16::from(image[base])
                + u16::from(image[base + 1])
                + u16::from(image[base + width])
                + u16::from(image[base + width + 1]);
            out.push((sum / 4) as u8);
        }
    }
    (out, new_width, new_height)
}

/// Shift each row's dark level to a fixed bias.
///
/// The "dark level" of a row is estimated as its minimum pixel value. Every
/// pixel in the row is shifted by `bias - row_min` and clamped to `[0, 255]`.
fn normalize_rows_to_bias(image: &[u8], width: usize, height: usize, bias: f64) -> Vec<u8> {
    let mut out = Vec::with_capacity(image.len());
    for y in 0..height {
        let row = &image[y * width..(y + 1) * width];
        let row_min = *row.iter().min().unwrap_or(&0);
        let shift = bias - f64::from(row_min);
        for &p in row {
            let shifted = f64::from(p) + shift;
            out.push(shifted.clamp(0.0, 255.0) as u8);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binning_one_is_identity() {
        let image: Vec<u8> = (0..64).map(|v| (v % 256) as u8).collect();
        let (detection, higher) = build_cascade(&image, 8, 8, 1, false);
        assert_eq!(detection.data, image);
        assert_eq!(detection.binning, 1);
        assert!(higher.is_none());
    }

    #[test]
    fn binning_two_averages_2x2_blocks() {
        // 4x4 image with a 2x2 bright block in the top-left.
        let image = vec![
            10, 10, 0, 0, // row 0
            10, 10, 0, 0, // row 1
            0, 0, 20, 20, // row 2
            0, 0, 20, 20, // row 3
        ];
        let (detection, higher) = build_cascade(&image, 4, 4, 2, false);
        assert_eq!(detection.width, 2);
        assert_eq!(detection.height, 2);
        assert_eq!(detection.binning, 2);
        // Integer average of the four 10-valued pixels is (40/4) = 10.
        assert_eq!(detection.data, vec![10, 0, 0, 20]);
        assert!(higher.is_some());
        let higher = higher.unwrap();
        assert_eq!(higher.binning, 1);
        assert_eq!(higher.data, image);
    }

    #[test]
    fn binning_four_centroids_in_input_coordinates() {
        // 8x8 image with a bright 2x2 spot at full-resolution (2,2)-(3,3).
        // After binning by 4 the spot becomes a single bright binned pixel.
        let mut image = vec![0u8; 8 * 8];
        for y in 2..=3 {
            for x in 2..=3 {
                image[y * 8 + x] = 100;
            }
        }
        let (detection, higher) = build_cascade(&image, 8, 8, 4, false);
        assert_eq!(detection.binning, 4);
        assert_eq!(detection.width, 2);
        assert_eq!(detection.height, 2);
        // The bright spot lands in binned pixel (0,0) because (2,2) maps to
        // bin (0,0). Its value is 100/4 = 25.
        assert_eq!(detection.get(0, 0), Some(25));
        assert!(higher.is_some());
        assert_eq!(higher.as_ref().unwrap().binning, 2);
    }

    #[test]
    fn max_size_matches_spec() {
        assert_eq!(max_size(100, 1), 1);
        assert_eq!(max_size(200, 1), 2);
        assert_eq!(max_size(200, 2), 2); // 200/100/2 + 1 = 2
        assert_eq!(max_size(400, 4), 2); // 400/100/4 + 1 = 2
        assert_eq!(max_size(800, 8), 2); // 800/100/8 + 1 = 2
    }

    #[test]
    fn normalize_rows_shifts_dark_level_to_bias() {
        // Two rows: row 0 has min 2, row 1 has min 52. After normalization the
        // dark level of each row should sit at bias 2.0.
        let mut image = vec![0u8; 8 * 2];
        for x in 0..8 {
            image[x] = (x + 2) as u8; // row 0: 2..9, min = 2
            image[8 + x] = (50 + x + 2) as u8; // row 1: 52..59, min = 52
        }
        let (detection, _) = build_cascade(&image, 8, 2, 1, true);
        // Row 0 shifted by 0, row 1 shifted by -50.
        assert_eq!(detection.data[0], 2);
        assert_eq!(detection.data[7], 9);
        assert_eq!(detection.data[8], 2);
        assert_eq!(detection.data[15], 9);
    }
}
