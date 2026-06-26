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

pub mod io;

pub use image::GrayImage;

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
