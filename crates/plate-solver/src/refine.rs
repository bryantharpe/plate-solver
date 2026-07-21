//! Refinement stage: re-fit attitude over all matches, extract RA/Dec/Roll,
//! refine FOV/distortion, compute residuals, and assemble the final solution.

use crate::status::{Solution, SolveStatus, VerificationOutcome};
use math_core::{
    attitude::{extract_radec_roll, solve_attitude},
    fov::{refine_fov, refine_fov_with_distortion},
    residuals::residual_stats,
    PinholeCamera, UnitVector,
};

/// Refine an accepted candidate into a fully populated `Solution`.
///
/// Steps:
/// 1. Re-fit attitude over all matched pairs with the SVD Wahba solver.
/// 2. Extract RA/Dec/Roll from the refined rotation matrix.
/// 3. Refine FOV (and optionally distortion) from the matched pairs.
/// 4. Recompute residuals (RMSE/P90E/MAXE) in arcseconds.
/// 5. Assemble the final `Solution` with camera, matches, and status.
pub fn refine_solution(
    ctx: &crate::status::SolveContext,
    _candidate: &pattern_database::Candidate,
    outcome: &VerificationOutcome,
    width: f64,
    height: f64,
) -> Solution {
    if !outcome.accepted || outcome.matched_image_vectors.is_empty() {
        return Solution {
            status: Some(SolveStatus::NoMatch),
            fov_used: Some(ctx.fov_initial),
            ..Solution::default()
        };
    }

    // 1. Re-fit attitude over all matched pairs.
    let (_rotation, ra, dec, roll) =
        match fit_attitude(&outcome.matched_image_vectors, &outcome.matched_stars) {
            Some(r) => r,
            None => {
                return Solution {
                    status: Some(SolveStatus::NoMatch),
                    fov_used: Some(ctx.fov_initial),
                    ..Solution::default()
                }
            }
        };

    // 2. Refine FOV and optionally distortion.
    let estimate_distortion = ctx.distortion != 0.0;
    let (camera, refined_distortion) = refine_camera(
        outcome.coarse_fov,
        width,
        height,
        &outcome.matched_image_vectors,
        &outcome.matched_stars,
        estimate_distortion,
    );

    // 3. Residuals in arcseconds.
    let stats = residual_stats(&outcome.matched_image_vectors, &outcome.matched_stars);

    // 4. Assemble solution.
    Solution {
        status: Some(SolveStatus::MatchFound),
        camera: Some(camera),
        matched_centroids: outcome.matched_centroids.clone(),
        matched_stars: outcome.matched_stars.clone(),
        matched_catalog_ids: outcome.matched_catalog_ids.clone(),
        match_probability: outcome.match_probability,
        fov_used: Some(camera.fov),
        pattern_candidates: vec![*_candidate],
        ra: Some(ra),
        dec: Some(dec),
        roll: Some(roll),
        rmse: Some(stats.rmse),
        p90e: Some(stats.p90e),
        maxe: Some(stats.maxe),
        distortion: refined_distortion,
    }
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
        let (refined_fov, k) =
            refine_fov_with_distortion(fov, width, height, image_vectors, catalog_vectors);
        (PinholeCamera::new(width, height, refined_fov), Some(k))
    } else {
        let refined_fov = refine_fov(fov, width, height, image_vectors, catalog_vectors, None).0;
        (PinholeCamera::new(width, height, refined_fov), None)
    }
}
