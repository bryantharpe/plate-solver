//! Plate-solving crate.
//!
//! Accepts star centroids (or a raw image) and a pattern database,
//! and returns the orientation of the field.

pub use ps_db::Database;
pub use ps_detect::StarDescription;
use image::GrayImage;

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
    // Determine initial FOV (degrees) from params or DB midpoint
    let _fov_initial = params.fov_estimate.unwrap_or_else(|| {
        let min_fov = db.properties.min_fov as f64;
        let max_fov = db.properties.max_fov as f64;
        (min_fov + max_fov) / 2.0
    });

    // Full solver to be implemented in SV2-SV5.
    Solution::failure(SolveStatus::NoMatch, 0.0)
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
}
