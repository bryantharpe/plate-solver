//! Star detection crate.
//!
//! Accepts 8-bit grayscale images and returns brightest-first star centroids.
//!
//! ## Coordinate conventions
//!
//! - External API: `(x, y)` where `x` = column (rightward), `y` = row (downward)
//! - Internal computation: `(y, x)` matching image row-major layout
//! - `(0.5, 0.5)` = center of the top-left pixel
//! - Integer floor of a centroid gives the pixel index

pub mod binning;
pub mod blob;
pub mod detect;
pub mod gate;
pub mod histogram;
pub mod io;
pub mod noise;

pub use binning::{set_binner, Binned2x2Result};
pub use blob::{form_blobs_from_candidates, gate_star_2d, Blob};
pub use detect::get_stars_from_image;
pub use gate::{
    reject_hot_pixels, scan_image_for_candidates, CandidateFrom1D, GateResult, PixelHotType,
};
pub use histogram::HistogramStats;
pub use image::GrayImage;

/// A borrowed view over 8-bit grayscale pixel data — the zero-copy counterpart
/// to the owned `GrayImage`. Input-facing detection functions take this by
/// reference so callers holding a borrowed buffer (e.g. an mmap) need not copy.
pub type GrayImageView<'a> = image::ImageBuffer<image::Luma<u8>, &'a [u8]>;

/// Build a `GrayImageView` borrowing the pixel data of an owned `GrayImage`.
/// Infallible: `img.as_raw()` is always exactly `width * height` bytes.
pub fn as_view(img: &GrayImage) -> GrayImageView<'_> {
    image::ImageBuffer::from_raw(img.width(), img.height(), img.as_raw().as_slice())
        .expect("GrayImage's own raw buffer is always the correct size for its dimensions")
}

/// Noise floor applied to the estimated noise value.
pub const NOISE_FLOOR: f64 = 0.2;

/// A detected star description.
///
/// Centroids are in **input-image** coordinates (full resolution),
/// even when binning is used. `(0.5, 0.5)` = top-left pixel center.
#[derive(Debug, Copy, Clone)]
pub struct StarDescription {
    /// Sub-pixel x position (column, rightward increasing)
    pub centroid_x: f64,
    /// Sub-pixel y position (row, downward increasing)
    pub centroid_y: f64,
    /// Brightest pixel value in the star inset (NOT background-subtracted)
    pub peak_value: u8,
    /// Background-subtracted region sum, clamped to >= 0
    pub brightness: f64,
    /// Count of pixels == 255 in the star inset
    pub num_saturated: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `GrayImageView` built directly over a raw `&[u8]` slice (the shape a
    /// zero-copy caller, e.g. an mmap-backed reader, would use) must behave
    /// identically to `as_view(&owned_image)` — proving `as_view` really is a
    /// zero-copy no-op wrapper and not something that reinterprets the data.
    #[test]
    fn raw_slice_view_matches_as_view() {
        // Small synthetic image with a few bright "stars" well clear of edges.
        let width = 40u32;
        let height = 40u32;
        let mut pixels = vec![20u8; (width * height) as usize];
        for &(cx, cy) in &[(10u32, 10u32), (28u32, 15u32), (15u32, 30u32)] {
            for dy in 0..3 {
                for dx in 0..3 {
                    let x = cx + dx - 1;
                    let y = cy + dy - 1;
                    pixels[(y * width + x) as usize] = 220;
                }
            }
        }

        let owned = GrayImage::from_raw(width, height, pixels.clone()).unwrap();
        let view_via_as_view = as_view(&owned);
        let raw_slice: &[u8] = &pixels;
        let view_via_raw = GrayImageView::from_raw(width, height, raw_slice).unwrap();

        assert_eq!(view_via_as_view.dimensions(), view_via_raw.dimensions());
        assert_eq!(view_via_as_view.as_raw(), view_via_raw.as_raw());

        let (stars_a, hot_a, _binned_a, _hist_a) =
            get_stars_from_image(&view_via_as_view, 1.0, 4.0, false, 1, true, false).unwrap();
        let (stars_b, hot_b, _binned_b, _hist_b) =
            get_stars_from_image(&view_via_raw, 1.0, 4.0, false, 1, true, false).unwrap();

        assert_eq!(hot_a, hot_b);
        assert_eq!(stars_a.len(), stars_b.len());
        for (a, b) in stars_a.iter().zip(stars_b.iter()) {
            assert_eq!(a.centroid_x, b.centroid_x);
            assert_eq!(a.centroid_y, b.centroid_y);
            assert_eq!(a.peak_value, b.peak_value);
            assert_eq!(a.brightness, b.brightness);
            assert_eq!(a.num_saturated, b.num_saturated);
        }
    }
}
