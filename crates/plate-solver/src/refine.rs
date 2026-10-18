//! Refinement stage: re-fit attitude over all matches, extract RA/Dec/Roll,
//! refine FOV/distortion, compute residuals, and assemble the final solution.

use crate::status::{Solution, SolveStatus, VerificationOutcome};
use math_core::{
    attitude::{extract_radec_roll, solve_attitude},
    fov::{refine_fov, refine_fov_with_distortion},
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
pub fn refine_solution<D>(
    ctx: &crate::status::SolveContext<D>,
    candidate: &pattern_database::Candidate,
    outcome: &VerificationOutcome,
    prep: &crate::preparation::Prepared,
    width: f64,
    height: f64,
) -> Solution {
    let no_match = || Solution {
        status: Some(SolveStatus::NoMatch),
        fov_used: Some(ctx.fov_initial),
        ..Solution::default()
    };
    if !outcome.accepted || outcome.matched_image_vectors.is_empty() {
        return no_match();
    }

    // 1. Re-fit the attitude over all matches, image vectors at the coarse FOV.
    let Some((rotation, ra, dec, roll)) =
        fit_attitude(&outcome.matched_image_vectors, &outcome.matched_stars)
    else {
        return no_match();
    };

    // 2. FOV and distortion by least squares on (tan(theta_catalog), r_image), cedar's
    //    model r(1 - k r^2) = f tan(theta) with r in units of width/2. Catalog vectors
    //    are derotated into the camera frame first.
    let raw: Vec<(f64, f64)> = outcome
        .matched_image_indices
        .iter()
        .map(|&i| prep.raw_centroids[i])
        .collect();
    let (cy, cx) = (height / 2.0, width / 2.0);
    let (mut a00, mut a01, mut a11, mut b0, mut b1) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (c, cat) in raw.iter().zip(outcome.matched_stars.iter()) {
        let d = rotation.rotate(*cat);
        let t = (d.y * d.y + d.z * d.z).sqrt() / d.x;
        let r = ((c.0 - cy).powi(2) + (c.1 - cx).powi(2)).sqrt() / width * 2.0;
        let r3 = r * r * r;
        a00 += t * t;
        a01 += t * r3;
        a11 += r3 * r3;
        b0 += t * r;
        b1 += r3 * r;
    }
    let det = a00 * a11 - a01 * a01;
    let (mut fov, mut k) = (outcome.coarse_fov, 0.0);
    if det.abs() > 1e-300 {
        let f = (a11 * b0 - a01 * b1) / det;
        let k_est = (a00 * b1 - a01 * b0) / det;
        let f = f / (1.0 - k_est);
        let fov_est = 2.0 * (1.0 / f).atan();
        if fov_est.is_finite() && fov_est > 0.0 {
            fov = fov_est;
            k = k_est;
        }
    }

    // 3. Residuals: matched centroids re-undistorted with k, unprojected at the
    //    refined FOV, rotated to the sky (R^T v), compared to the catalog vectors.
    let undist = math_core::undistort_centroids(&raw, width, height, k);
    let camera = PinholeCamera::new(width, height, fov);
    let r = &rotation.rows;
    let mut angles: Vec<f64> = camera
        .unproject(&undist)
        .into_iter()
        .zip(outcome.matched_stars.iter())
        .map(|(v, cat)| {
            let v = v.expect("never zero-length");
            let sky = UnitVector {
                x: r[0][0] * v.x + r[1][0] * v.y + r[2][0] * v.z,
                y: r[0][1] * v.x + r[1][1] * v.y + r[2][1] * v.z,
                z: r[0][2] * v.x + r[1][2] * v.y + r[2][2] * v.z,
            };
            math_core::angular_distance(sky, *cat)
        })
        .collect();
    angles.sort_by(|a, b| a.total_cmp(b));
    let to_arcsec = 180.0 / std::f64::consts::PI * 3600.0;
    let rmse = (angles.iter().map(|a| a * a).sum::<f64>() / angles.len() as f64).sqrt() * to_arcsec;
    let p90e = angles[((angles.len() - 1) as f64 * 0.9) as usize] * to_arcsec;
    let maxe = angles[angles.len() - 1] * to_arcsec;

    Solution {
        status: Some(SolveStatus::MatchFound),
        camera: Some(camera),
        matched_centroids: outcome.matched_centroids.clone(),
        matched_stars: outcome.matched_stars.clone(),
        matched_catalog_ids: outcome.matched_catalog_ids.clone(),
        match_probability: outcome.match_probability,
        fov_used: Some(camera.fov),
        pattern_candidates: vec![*candidate],
        ra: Some(ra),
        dec: Some(dec),
        roll: Some(roll),
        rmse: Some(rmse),
        p90e: Some(p90e),
        maxe: Some(maxe),
        distortion: Some(k),
        noise_estimate: None,
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
#[allow(dead_code)]
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
