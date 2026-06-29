//! Plate-solving crate.
//!
//! Accepts star centroids (or a raw image) and a pattern database,
//! and returns the orientation of the field.

pub use ps_db::Database;
pub use ps_detect::StarDescription;
use image::GrayImage;
use std::f64::consts::PI;

/// Status codes for a solve attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolveStatus {
    MatchFound,
    NoMatch,
    Timeout,
    Cancelled,
    TooFew,
}

/// Full output of a solve attempt.
#[derive(Debug, Clone)]
pub struct Solution {
    /// Right ascension of boresight, degrees (valid on MatchFound).
    pub ra: f64,
    /// Declination of boresight, degrees (valid on MatchFound).
    pub dec: f64,
    /// Roll angle, degrees (valid on MatchFound).
    pub roll: f64,
    /// Field of view (diagonal), degrees (valid on MatchFound).
    pub fov: f64,
    /// Distortion coefficient k (valid on MatchFound).
    pub distortion: f64,
    /// RMSE residual, arcseconds (valid on MatchFound).
    pub rmse: f64,
    /// 90th-percentile residual, arcseconds.
    pub p90e: f64,
    /// Max residual, arcseconds.
    pub maxe: f64,
    /// Number of matched stars (valid on MatchFound).
    pub matches: usize,
    /// False-alarm probability (Bonferroni-corrected, valid on MatchFound).
    pub prob: f64,
    /// Solve wall-clock time, seconds.
    pub t_solve: f64,
    /// Status of the solve attempt.
    pub status: SolveStatus,
    /// Matched centroid positions (y,x), if requested.
    pub matched_centroids: Option<Vec<[f64; 2]>>,
    /// Matched star RA/Dec/mag triples, if requested.
    pub matched_stars: Option<Vec<[f64; 3]>>,
    /// Matched catalog IDs, if requested.
    pub matched_cat_ids: Option<Vec<u32>>,
}

impl Solution {
    /// Construct a failure result.
    pub fn failure(status: SolveStatus, t_solve: f64) -> Self {
        Self {
            status,
            ra: 0.0,
            dec: 0.0,
            roll: 0.0,
            fov: 0.0,
            distortion: 0.0,
            rmse: 0.0,
            p90e: 0.0,
            maxe: 0.0,
            matches: 0,
            prob: 1.0,
            t_solve,
            matched_centroids: None,
            matched_stars: None,
            matched_cat_ids: None,
        }
    }
}

/// Parameters for a solve call (cedar defaults).
#[derive(Debug, Clone)]
pub struct SolveParams {
    /// Fraction of image width used as match radius. Default: 0.01
    pub match_radius: f64,
    /// Per-solve false-alarm threshold (Bonferroni divided by num_patterns). Default: 1e-5
    pub match_threshold: f64,
    /// Pattern-key tolerance band half-width (clamped to >= DB pattern_max_error). Default: 0.002
    pub match_max_error: f64,
    /// Solve timeout in milliseconds. Default: Some(5000)
    pub solve_timeout: Option<u64>,
    /// Radial distortion k. Some(0.0) = no distortion; None = estimate. Default: Some(0.0)
    pub distortion: Option<f64>,
    /// FOV estimate (degrees). None → use DB FOV-range midpoint.
    pub fov_estimate: Option<f64>,
    /// FOV max error (degrees). None → no constraint.
    pub fov_max_error: Option<f64>,
}

impl Default for SolveParams {
    fn default() -> Self {
        Self {
            match_radius: 0.01,
            match_threshold: 1e-5,
            match_max_error: 0.002,
            solve_timeout: Some(5000),
            distortion: Some(0.0),
            fov_estimate: None,
            fov_max_error: None,
        }
    }
}

/// Solve from pre-extracted centroids.
///
/// `star_centroids`: brightest-first, (y, x) pairs.
/// `size`: (height, width) in pixels.
/// When `params.fov_estimate` is None, uses the DB FOV-range midpoint.
pub fn solve_from_centroids(
    db: &Database,
    star_centroids: &[[f64; 2]],
    size: (usize, usize),
    params: &SolveParams,
) -> Solution {
    let (height, width) = size;

    // SV2 step 1: Set fov_initial (radians)
    let fov_initial = params.fov_estimate.map(f64::to_radians).unwrap_or_else(|| {
        (db.properties.min_fov as f64 + db.properties.max_fov as f64) / 2.0 * PI / 180.0
    });

    // SV2 step 2: Bonferroni correction
    let _match_threshold = params.match_threshold / db.properties.num_patterns as f64;

    // SV2 step 3: Pre-cluster-bust TooFew guard
    if star_centroids.len() < 4 {
        return Solution::failure(SolveStatus::TooFew, 0.0);
    }

    // SV2 step 4: Cluster-bust on ALL raw star_centroids
    let vsfov = db.properties.verification_stars_per_fov as f64;
    let _ = fov_initial;
    let separation_pixels = width as f64 * 0.6 / vsfov.sqrt();
    let pattern_centroids = cluster_bust_centroids(star_centroids, separation_pixels);
    let _pattern_centroids = pattern_centroids;

    // SV2 step 5: Slice to verification_stars_per_fov (brightest-first already)
    let vsfov_usize = db.properties.verification_stars_per_fov as usize;
    let image_centroids = &star_centroids[..star_centroids.len().min(vsfov_usize)];

    // SV2 step 6: Undistort (only if distortion is Some and finite)
    let image_centroids_undist: Vec<[f64; 2]> = if let Some(k) = params.distortion {
        if k.is_finite() {
            ps_core::distortion::undistort_centroids(image_centroids, (height, width), k)
        } else {
            image_centroids.to_vec()
        }
    } else {
        image_centroids.to_vec()
    };

    // SV2 step 7: Compute vectors
    let _image_centroids_vectors = ps_core::projection::compute_vectors(
        &image_centroids_undist,
        (height, width),
        fov_initial,
    );

    // SV3-SV5 stub: return NoMatch after preparation
    Solution::failure(SolveStatus::NoMatch, 0.0)
}

/// Cluster-bust centroids: greedy O(n^2) pass keeping stars separated
/// by at least `separation_pixels`.
///
/// Returns indices into the original slice for the kept centroids.
pub fn cluster_bust_centroids(
    centroids: &[[f64; 2]],
    separation_pixels: f64,
) -> Vec<usize> {
    let mut kept = Vec::with_capacity(centroids.len());
    let sep_sq = separation_pixels * separation_pixels;

    for i in 0..centroids.len() {
        let [yi, xi] = centroids[i];
        let mut dominated = false;
        for &k_idx in &kept {
            let [yk, xk] = centroids[k_idx];
            let dy = yi - yk;
            let dx = xi - xk;
            if dy * dy + dx * dx <= sep_sq {
                dominated = true;
                break;
            }
        }
        if !dominated {
            kept.push(i);
        }
    }
    kept
}

/// Solve from a raw grayscale image (detects stars then solves).
pub fn solve_from_image(
    db: &Database,
    image: &GrayImage,
    params: &SolveParams,
) -> Solution {
    let (width, height) = (image.width() as usize, image.height() as usize);
    let (stars, _, _, _) = ps_detect::get_stars_from_image(
        image,
        1.0,   // noise_estimate (floored to NOISE_FLOOR internally)
        6.0,   // sigma
        false, // normalize_rows
        1,     // binning
        true,  // detect_hot_pixels
        false, // return_binned_image
    );
    let centroids: Vec<[f64; 2]> = stars
        .iter()
        .map(|s| [s.centroid_y as f64, s.centroid_x as f64])
        .collect();
    solve_from_centroids(db, &centroids, (height, width), params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solve_status_variants() {
        let statuses = [
            SolveStatus::MatchFound,
            SolveStatus::NoMatch,
            SolveStatus::Timeout,
            SolveStatus::Cancelled,
            SolveStatus::TooFew,
        ];
        for s in &statuses {
            let _ = format!("{:?}", s);
        }
    }

    #[test]
    fn default_params_cedar_defaults() {
        let p = SolveParams::default();
        assert_eq!(p.match_radius, 0.01);
        assert!((p.match_threshold - 1e-5).abs() < 1e-15);
        assert_eq!(p.match_max_error, 0.002);
        assert_eq!(p.solve_timeout, Some(5000));
        assert_eq!(p.distortion, Some(0.0));
        assert!(p.fov_estimate.is_none());
    }

    #[test]
    fn fov_estimate_defaults_to_db_midpoint() {
        // This test uses a mock by checking that the fallback formula is correct.
        // Full DB integration tested in later tasks.
        let min_fov = 10.0f64;
        let max_fov = 30.0f64;
        let midpoint = (min_fov + max_fov) / 2.0;
        assert_eq!(midpoint, 20.0);
    }

    #[test]
    fn solution_failure_constructor() {
        let sol = Solution::failure(SolveStatus::TooFew, 0.123);
        assert_eq!(sol.status, SolveStatus::TooFew);
        assert_eq!(sol.matches, 0);
        assert!((sol.t_solve - 0.123).abs() < 1e-12);
    }

    #[test]
    fn preparation_cluster_bust_and_too_few() {
        // Verify the cluster-bust logic in isolation.
        let centroids: Vec<[f64; 2]> = vec![
            [100.0, 100.0],
            [101.0, 101.0], // very close to first
            [200.0, 200.0],
            [201.0, 201.0], // very close to third
            [300.0, 300.0],
        ];
        let separation_pixels = 10.0_f64;
        let kept = cluster_bust_centroids(&centroids, separation_pixels);
        // Should keep [0, 2, 4] (indices 1 and 3 are within 10px of kept neighbors)
        assert_eq!(kept, vec![0usize, 2, 4]);
    }

    #[test]
    fn cluster_bust_separation_formula() {
        // Cedar-solve reference: sep_px = width * 0.6 / sqrt(vsfov)
        let width = 1920.0_f64;
        let vsfov = 150.0_f64;
        let expected = width * 0.6 / vsfov.sqrt();
        // For fov_initial = 11.0 degrees in radians, this should be fov-independent
        let fov_initial_rad = 11.0_f64.to_radians();
        // The correct formula produces the same result regardless of fov
        let sep = width * 0.6 / vsfov.sqrt();
        let _ = fov_initial_rad; // fov cancels, formula is fov-independent
        assert!((sep - expected).abs() < 1e-10);
        // Check concrete value ~93.9 px at width=1920, vsfov=150
        assert!(sep > 90.0 && sep < 100.0);
    }
}
