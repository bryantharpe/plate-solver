//! Pinhole camera projection: pixel centroids `[y, x]` ↔ camera-frame unit
//! vectors `(i, j, k)`. Reference: doc 02 §3 (`_compute_vectors` /
//! `_compute_centroids`).
//!
//! Forward path scales pixel offsets by `tan(fov/2) * 2 / width` and normalises
//! the resulting `(1, j, k)` vector. Inverse path recovers pixel coordinates
//! from the camera-frame ratio `k/i` and `j/i`, clamped to the image extent.

use nalgebra::Vector3;

/// Project pixel centroids `[y, x]` to camera-frame unit vectors `(i, j, k)`.
///
/// `size = (height, width)` in pixels; `fov` is the horizontal field of view
/// in radians. Each centroid is the centre of a pixel (row, col) with
/// `(0.5, 0.5)` being the centre of the top-left pixel.
pub fn compute_vectors(
    centroids: &[[f64; 2]],
    size: (usize, usize),
    fov: f64,
) -> Vec<Vector3<f64>> {
    let height = size.0 as f64;
    let width = size.1 as f64;
    let half_fov_tan = (fov / 2.0).tan();
    let scale_factor = half_fov_tan / width * 2.0;
    let img_center_y = height / 2.0;
    let img_center_x = width / 2.0;

    centroids
        .iter()
        .map(|&[y, x]| {
            let k = (img_center_y - y) * scale_factor;
            let j = (img_center_x - x) * scale_factor;
            let i = 1.0;
            Vector3::new(i, j, k).normalize()
        })
        .collect()
}

/// Inverse projection of (derotated) camera-frame vectors back to pixel
/// centroids `[y, x]`.
///
/// Returns all centroids (one per input vector, index-aligned) plus the
/// `keep` index list containing only vectors that are in front of and inside
/// the frame: `i > 0 && 0 < y < height && 0 < x < width`.
pub fn compute_centroids(
    vectors: &[Vector3<f64>],
    size: (usize, usize),
    fov: f64,
) -> (Vec<[f64; 2]>, Vec<usize>) {
    let height = size.0 as f64;
    let width = size.1 as f64;
    let half_fov_tan = (fov / 2.0).tan();
    let scale_factor = -width / 2.0 / half_fov_tan;
    let img_center_y = height / 2.0;
    let img_center_x = width / 2.0;

    let mut centroids = Vec::with_capacity(vectors.len());
    let mut keep = Vec::with_capacity(vectors.len());

    for (idx, v) in vectors.iter().enumerate() {
        let y = scale_factor * v.z / v.x + img_center_y;
        let x = scale_factor * v.y / v.x + img_center_x;
        centroids.push([y, x]);
        if v.x > 0.0 && y > 0.0 && y < height && x > 0.0 && x < width {
            keep.push(idx);
        }
    }

    (centroids, keep)
}
