//! I/O types and coordinate conventions for star detection.
//!
//! ## External contract
//!
//! The API accepts an 8-bit `GrayImage` and returns `Vec<StarDescription>`
//! sorted brightest-first. Centroids use `(x, y)` ordering — `centroid_x` is
//! the column index, `centroid_y` is the row index.
//!
//! ## Internal convention
//!
//! Internally we use `(y, x)` = (row, column) to match image row-major layout.
//! The conversion happens at the crate boundary in `get_stars_from_image`.

use image::GrayImage;

/// Load a GrayImage from a file path.
pub fn load_grayscale(path: &std::path::Path) -> Result<GrayImage, image::ImageError> {
    Ok(image::open(path)?.to_luma8())
}
