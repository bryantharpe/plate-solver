//! FOV estimation and refinement (doc 02 §7).
//!
//! Coarse FOV from a matched catalog pattern, diagonal FOV derivation,
//! and fine refinement with / without distortion modelling.

use crate::angle::angle_from_distance;
use nalgebra::{Matrix2, Vector2};

/// Estimate FOV from a matched catalog pattern.
///
/// If `fov_estimate` is provided (Mode 1):
///   fov = catalog_largest_edge / image_pattern_largest_edge * fov_estimate
///
/// If `fov_estimate` is None (Mode 2):
///   f = image_pattern_largest_pixel_distance / 2 / tan(catalog_largest_edge / 2)
///   fov = 2 * atan(width / 2 / f)
///
/// All angles in radians. `catalog_largest_edge` is the largest angular edge of
/// the catalog pattern. `image_pattern_largest_edge` is the same computed from
/// image centroid camera vectors (chord distance).
/// `image_pattern_largest_pixel_distance` is the max pairwise pixel distance among
/// the 4 pattern stars.
pub fn estimate_fov_from_pattern(
    catalog_largest_edge: f64,
    image_pattern_largest_edge: f64,
    image_pattern_largest_pixel_distance: f64,
    fov_estimate: Option<f64>,
    width: f64,
) -> f64 {
    match fov_estimate {
        Some(fov_est) => catalog_largest_edge / image_pattern_largest_edge * fov_est,
        None => {
            let f = image_pattern_largest_pixel_distance / 2.0 / (catalog_largest_edge / 2.0).tan();
            2.0 * (width / 2.0 / f).atan()
        }
    }
}

/// Diagonal FOV from horizontal FOV.
/// fov_diagonal = fov * sqrt(width^2 + height^2) / width
pub fn diagonal_fov(fov: f64, width: f64, height: f64) -> f64 {
    fov * (width * width + height * height).sqrt() / width
}

/// Fine FOV refinement without distortion.
///
/// Compare pairwise angles between matched image vectors and matched catalog vectors,
/// then scale: fov *= mean(angles_catalog / angles_camera).
///
/// `matched_image_vectors` and `matched_catalog_vectors` are arrays of 3-D unit vectors,
/// each with the same length N (the number of matched stars).
/// Compute all C(N,2) pairwise chord distances via chord distance between unit vectors,
/// then convert back to angles via `angle_from_distance`.
pub fn refine_fov_no_distortion(
    fov_coarse: f64,
    matched_image_vectors: &[[f64; 3]],
    matched_catalog_vectors: &[[f64; 3]],
) -> f64 {
    let n = matched_image_vectors.len();

    // Compute pairwise chord distances for image and catalog vectors.
    let mut camera_angles = Vec::with_capacity(n * (n - 1) / 2);
    let mut catalog_angles = Vec::with_capacity(n * (n - 1) / 2);

    for i in 0..n {
        for j in (i + 1)..n {
            // Chord distance: norm(a - b)
            let dx_i = matched_image_vectors[i][0] - matched_image_vectors[j][0];
            let dy_i = matched_image_vectors[i][1] - matched_image_vectors[j][1];
            let dz_i = matched_image_vectors[i][2] - matched_image_vectors[j][2];
            let chord_i = (dx_i * dx_i + dy_i * dy_i + dz_i * dz_i).sqrt();

            let dx_c = matched_catalog_vectors[i][0] - matched_catalog_vectors[j][0];
            let dy_c = matched_catalog_vectors[i][1] - matched_catalog_vectors[j][1];
            let dz_c = matched_catalog_vectors[i][2] - matched_catalog_vectors[j][2];
            let chord_c = (dx_c * dx_c + dy_c * dy_c + dz_c * dz_c).sqrt();

            camera_angles.push(angle_from_distance(chord_i));
            catalog_angles.push(angle_from_distance(chord_c));
        }
    }

    let mut ratio_sum = 0.0;
    for (ca, cg) in camera_angles.iter().zip(catalog_angles.iter()) {
        ratio_sum += cg / ca;
    }
    let mean_ratio = ratio_sum / catalog_angles.len() as f64;
    fov_coarse * mean_ratio
}

/// Fine FOV + distortion refinement via least squares.
///
/// Derotate catalog vectors by R: derotated = R @ catalog_vectors
/// For each matched star i:
///   t[i] = norm(derotated[i][1..]) / derotated[i][0]   (tangent of boresight angle)
///   r[i] = norm(image_centroid[i] - [height/2, width/2]) / width * 2
///         (distorted pixel radius, scaled to half-width)
///
/// Solve A @ [f, k] = b via least squares, where each row of A is [t[i], r[i]^3]
/// and b[i] = r[i].
/// Then: f = f / (1 - k), fov = 2 * atan(1 / f)
///
/// Return (fov, k).
///
/// For least squares, use nalgebra: `A.lu().solve(&b)` where A is a D×2 matrix
/// and b is a D×1.
pub fn refine_fov_with_distortion(
    matched_image_centroids: &[[f64; 2]], // Nx2, (y, x) — these are DISTORTED centroids
    matched_catalog_vectors: &[[f64; 3]], // Nx3, catalog unit vectors
    rotation_matrix: &[[f64; 3]; 3],      // 3x3 rotation matrix (row-major)
    width: f64,
    height: f64,
) -> (f64, f64) {
    let n = matched_image_centroids.len();
    let cy0 = height / 2.0;
    let cx0 = width / 2.0;

    // Build the normal-equation accumulators: A^T A (2x2) and A^T b (2x1).
    // Each row of A is [tangent, r^3], b[i] = r.
    let mut ata = Matrix2::<f64>::zeros();
    let mut atb = Vector2::<f64>::zeros();

    for i in 0..n {
        // Derotate: derotated = R @ catalog_vector[i]
        let cv = &matched_catalog_vectors[i];
        let d0 = rotation_matrix[0][0] * cv[0]
            + rotation_matrix[0][1] * cv[1]
            + rotation_matrix[0][2] * cv[2];
        let d1 = rotation_matrix[1][0] * cv[0]
            + rotation_matrix[1][1] * cv[1]
            + rotation_matrix[1][2] * cv[2];
        let d2 = rotation_matrix[2][0] * cv[0]
            + rotation_matrix[2][1] * cv[1]
            + rotation_matrix[2][2] * cv[2];

        // Tangent: norm([d1, d2]) / d0
        let tangent = (d1 * d1 + d2 * d2).sqrt() / d0;

        // Distorted pixel radius (scaled to half-width)
        let centroid_y = matched_image_centroids[i][0];
        let centroid_x = matched_image_centroids[i][1];
        let dy = centroid_y - cy0;
        let dx = centroid_x - cx0;
        let r = (dy * dy + dx * dx).sqrt() / width * 2.0;

        let r3 = r * r * r;

        // Accumulate A^T A and A^T b.
        ata[(0, 0)] += tangent * tangent;
        ata[(0, 1)] += tangent * r3;
        ata[(1, 0)] += r3 * tangent;
        ata[(1, 1)] += r3 * r3;
        atb[0] += tangent * r;
        atb[1] += r3 * r;
    }

    // Solve the 2x2 normal equations: (A^T A) x = A^T b
    let sol = ata
        .lu()
        .solve(&atb)
        .expect("LU solve failed for FOV+distortion normal equations");

    let f_raw = sol[(0, 0)];
    let k = sol[(1, 0)];
    let f_sol = f_raw / (1.0 - k);
    let fov = 2.0_f64 * (1.0_f64 / f_sol).atan();

    (fov, k)
}
