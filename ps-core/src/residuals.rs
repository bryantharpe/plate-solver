//! Residual statistics (doc 02 §9).
//!
//! Computes RMSE, P90E and MAXE angular residuals between matched image
//! vectors (rotated to celestial frame) and catalog vectors.

use crate::angle::angle_from_distance;

/// Residual statistics in arcseconds.
pub struct ResidualStats {
    /// Root-mean-square error in arcseconds.
    pub rmse_arcsec: f64,
    /// 90th-percentile error in arcseconds.
    pub p90e_arcsec: f64,
    /// Maximum error in arcseconds.
    pub maxe_arcsec: f64,
}

/// Compute residual statistics from matched vectors.
///
/// `final_match_vectors` — camera vectors of matched image centroids, rotated
/// to celestial frame by R^T.
/// `matched_catalog_vectors` — corresponding catalog unit vectors.
/// Both are Nx3 arrays with the same length.
///
/// Algorithm:
/// 1. Compute chord distance for each pair: norm(final[i] - catalog[i])
/// 2. Sort distances ascending
/// 3. Convert to angles using angle_from_distance (2*asin(d/2))
/// 4. P90E index: floor(0.9 * (N - 1))
/// 5. Convert to arcseconds: rad2deg(angle) * 3600
/// 6. RMSE: sqrt(mean(angles^2)) in arcsec
/// 7. P90E: angle at p90_index in arcsec
/// 8. MAXE: last angle in arcsec
///
/// Invariant: p90e_arcsec <= maxe_arcsec
pub fn compute_residuals(
    final_match_vectors: &[[f64; 3]],
    matched_catalog_vectors: &[[f64; 3]],
) -> ResidualStats {
    let n = final_match_vectors.len();

    // Step 1: Compute chord distances.
    let mut distances = Vec::with_capacity(n);
    for i in 0..n {
        let dx = final_match_vectors[i][0] - matched_catalog_vectors[i][0];
        let dy = final_match_vectors[i][1] - matched_catalog_vectors[i][1];
        let dz = final_match_vectors[i][2] - matched_catalog_vectors[i][2];
        distances.push((dx * dx + dy * dy + dz * dz).sqrt());
    }

    // Step 2: Sort ascending.
    distances.sort_by(|a, b| a.total_cmp(b));

    // Step 3: Convert to angles.
    let angles: Vec<f64> = distances.iter().map(|&d| angle_from_distance(d)).collect();

    // Step 4: P90 index.
    let p90_index = (0.9 * (n as f64 - 1.0)) as usize;

    // Steps 5-8: Convert to arcseconds and compute stats.
    const RAD_TO_DEG: f64 = 180.0 / std::f64::consts::PI;
    let angles_arcsec: Vec<f64> = angles.iter().map(|&a| a * RAD_TO_DEG * 3600.0).collect();

    let rmse_arcsec = (angles_arcsec.iter().map(|&a| a * a).sum::<f64>() / n as f64).sqrt();
    let p90e_arcsec = angles_arcsec[p90_index];
    let maxe_arcsec = angles_arcsec[n - 1];

    ResidualStats {
        rmse_arcsec,
        p90e_arcsec,
        maxe_arcsec,
    }
}
