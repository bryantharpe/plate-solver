//! Refinement stage: re-fit attitude over all matches, extract RA/Dec/Roll,
//! refine FOV/distortion, compute residuals, and assemble the final solution.

use crate::status::{Solution, SolveStatus};
use math_core::{
    attitude::{extract_radec_roll, solve_attitude},
    fov::{refine_fov, refine_fov_with_distortion},
    residuals::{residual_stats, ResidualStats},
    PinholeCamera, UnitVector,
};

/// Outcome of verifying a single candidate, including the matched star pairs
/// needed by the refinement stage.
#[derive(Debug, Clone, Default)]
pub struct VerificationOutcome {
    /// Whether the candidate was accepted by the false-alarm test.
    pub accepted: bool,
    /// Coarse rotation matrix from the initial 4-star pattern solve.
    pub rotation: Option<math_core::attitude::RotationMatrix>,
    /// Matched image centroids in pixel coordinates.
    pub matched_centroids: Vec<(f64, f64)>,
    /// Matched catalog star unit vectors in the celestial frame.
    pub matched_stars: Vec<UnitVector>,
    /// Catalog star indices for each matched pair.
    pub matched_catalog_ids: Vec<usize>,
    /// False-alarm probability reported by the binomial test.
    pub match_probability: Option<f64>,
    /// Coarse horizontal FOV used during verification, in radians.
    pub coarse_fov: f64,
}

/// Refine an accepted candidate into a fully populated `Solution`.
///
/// Steps:
/// 1. Re-fit attitude over all matched pairs with the SVD Wahba solver.
/// 2. Extract RA/Dec/Roll from the refined rotation matrix.
/// 3. Refine FOV (and optionally distortion) from the matched pairs.
/// 4. Recompute residuals (RMSE/P90E/MAXE) in arcseconds.
/// 5. Assemble the final `Solution` with camera, matches, and status.
pub fn refine_solution(
    _ctx: &crate::status::SolveContext,
    _candidate: &pattern_database::Candidate,
    _outcome: &VerificationOutcome,
    _width: f64,
    _height: f64,
) -> Solution {
    todo!("implement refinement and solution assembly")
}

/// Solve attitude over all matched pairs and extract RA/Dec/Roll.
fn fit_attitude(
    image_vectors: &[UnitVector],
    catalog_vectors: &[UnitVector],
) -> Option<(math_core::attitude::RotationMatrix, f64, f64, f64)> {
    let rotation = solve_attitude(image_vectors, catalog_vectors)?;
    let (ra, dec, roll) = extract_radec_roll(&rotation);
    Some((rotation, ra, dec, roll))
}

/// Refine FOV from matched pairs, optionally estimating distortion.
fn refine_camera(
    fov: f64,
    width: f64,
    height: f64,
    image_vectors: &[UnitVector],
    catalog_vectors: &[UnitVector],
    estimate_distortion: bool,
) -> (PinholeCamera, Option<f64>) {
    if estimate_distortion {
        let (refined_fov, k) = refine_fov_with_distortion(fov, width, height, image_vectors, catalog_vectors);
        (PinholeCamera::new(width, height, refined_fov), Some(k))
    } else {
        let refined_fov = refine_fov(fov, width, height, image_vectors, catalog_vectors, None).0;
        (PinholeCamera::new(width, height, refined_fov), None)
    }
}

/// Compute residual statistics for the matched pairs.
fn match_residuals(
    image_vectors: &[UnitVector],
    catalog_vectors: &[UnitVector],
) -> ResidualStats {
    residual_stats(image_vectors, catalog_vectors)
}
