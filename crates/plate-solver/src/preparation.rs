//! Preparation stage: FOV initial, threshold, brightest-N limit, undistortion,
//! cluster-busting, centroid vectors, and TOO_FEW detection.

use crate::status::{Solution, SolveContext, SolveStatus};
use math_core::{undistort_centroids, PinholeCamera, UnitVector};
use pattern_database::{DatabaseProperties, PatternDatabase};
use std::borrow::Borrow;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

/// Minimum number of centroids required to attempt a solve.
pub const MIN_CENTROIDS: usize = 4;

/// Compute the initial FOV estimate from the database FOV range and caller bounds.
///
/// If the caller supplies a finite `fov_estimate`, it is clamped to the database
/// `[min_fov, max_fov]` range. If no estimate is supplied, the midpoint of the
/// database range is used. The returned value is the single FOV used for
/// unprojection and reported as `fov_used`.
pub fn initial_fov(
    props: &DatabaseProperties,
    fov_estimate: Option<f64>,
    _fov_max_error: f64,
) -> f64 {
    let mid = ((props.min_fov as f64) + (props.max_fov as f64)) / 2.0;
    let base = fov_estimate.unwrap_or(mid);
    let min_fov = props.min_fov as f64;
    let max_fov = props.max_fov as f64;
    base.clamp(min_fov, max_fov)
}

/// Compute the working match threshold with the Bonferroni correction.
pub fn bonferroni_threshold(match_threshold: f64, num_patterns: u32) -> f64 {
    if num_patterns == 0 {
        return match_threshold;
    }
    match_threshold / num_patterns as f64
}

/// Limit the centroid list to the brightest N stars for verification.
pub fn limit_centroids(centroids: &[(f64, f64)], limit: usize) -> Vec<(f64, f64)> {
    centroids.iter().copied().take(limit).collect()
}

/// Compute pixel separation for cluster-busting from the database density rule.
///
/// `separation_for_density(fov, n) = 0.6 * fov / sqrt(n)`.
pub fn separation_for_density(fov: f64, stars_per_fov: usize) -> f64 {
    if stars_per_fov == 0 {
        return 0.0;
    }
    0.6 * fov / (stars_per_fov as f64).sqrt()
}

/// Apply scalar radial distortion correction to centroids.
pub fn undistort(centroids: &[(f64, f64)], width: f64, height: f64, k: f64) -> Vec<(f64, f64)> {
    if k == 0.0 || centroids.is_empty() {
        return centroids.to_vec();
    }
    undistort_centroids(centroids, width, height, k)
}

/// Remove centroids that are too close to a brighter centroid.
///
/// The separation is computed from the initial FOV and the database density rule,
/// projected into pixel space: `width * separation_for_density(fov_initial, n) / fov_initial`.
pub fn cluster_bust(
    centroids: &[(f64, f64)],
    fov_initial: f64,
    stars_per_fov: usize,
    width: f64,
) -> Vec<(f64, f64)> {
    if centroids.len() <= MIN_CENTROIDS {
        return centroids.to_vec();
    }
    let sep_rad = separation_for_density(fov_initial, stars_per_fov);
    let sep_px = width * sep_rad / fov_initial;
    let mut kept: Vec<(f64, f64)> = Vec::with_capacity(centroids.len());
    for &c in centroids {
        let too_close = kept.iter().any(|k| {
            let dx = c.0 - k.0;
            let dy = c.1 - k.1;
            (dx * dx + dy * dy).sqrt() < sep_px
        });
        if !too_close {
            kept.push(c);
        }
    }
    kept
}

/// Convert 2D centroids into unit vectors using a pinhole camera with the given FOV.
pub fn centroid_vectors(
    centroids: &[(f64, f64)],
    width: f64,
    height: f64,
    fov: f64,
) -> Vec<UnitVector> {
    let camera = PinholeCamera::new(width, height, fov.to_radians());
    camera.unproject(centroids).into_iter().flatten().collect()
}

/// Build the solve context from inputs and the loaded pattern database.
///
/// `db` is held as given: pass `&PatternDatabase` to borrow it, or an owned database.
#[allow(clippy::too_many_arguments)]
pub fn build_context<D: Borrow<PatternDatabase>>(
    db: D,
    fov_estimate: Option<f64>,
    fov_max_error: f64,
    match_radius: f64,
    match_threshold: f64,
    match_max_error: f64,
    distortion: f64,
    solve_timeout_ms: u64,
) -> SolveContext<D> {
    let props = db.borrow().properties.clone();
    let fov_initial = initial_fov(&props, fov_estimate, fov_max_error);
    // Cedar applies the largest-edge/FOV pre-filter only when the caller supplied
    // both an estimate and a tolerance. The tolerance is in degrees.
    let fov_estimate = fov_estimate.filter(|f| f.is_finite() && *f > 0.0);
    let fov_max_error = fov_estimate
        .and(Some(fov_max_error))
        .filter(|e| e.is_finite() && *e > 0.0);
    SolveContext {
        db,
        props: props.clone(),
        fov_initial,
        fov_estimate,
        fov_max_error,
        match_threshold: bonferroni_threshold(match_threshold, props.num_patterns),
        match_radius,
        match_max_error,
        distortion,
        solve_timeout_ms,
        start_instant: Instant::now(),
        cancelled: Arc::new(AtomicBool::new(false)),
        verification_stars_per_fov: props.verification_stars_per_fov as usize,
    }
}

/// Indices of the centroids kept by cluster-busting (brightest wins), in input order.
pub fn cluster_bust_indices(
    centroids: &[(f64, f64)],
    fov_initial: f64,
    stars_per_fov: usize,
    width: f64,
) -> Vec<usize> {
    let sep_px = width * separation_for_density(fov_initial, stars_per_fov) / fov_initial;
    let sep2 = sep_px * sep_px;
    let mut kept: Vec<usize> = Vec::with_capacity(centroids.len());
    for (i, c) in centroids.iter().enumerate() {
        let occupied = kept.iter().any(|&k| {
            let dy = c.0 - centroids[k].0;
            let dx = c.1 - centroids[k].1;
            dy * dy + dx * dx <= sep2
        });
        if !occupied {
            kept.push(i);
        }
    }
    kept
}

/// The prepared solve inputs.
///
/// Cedar forms patterns from the cluster-busted subset but verifies against the full
/// (brightest-`verification_stars_per_fov`) centroid list, so both are carried here.
#[derive(Debug, Clone)]
pub struct Prepared {
    /// Brightest-N centroids as supplied (before undistortion), `(y, x)`.
    pub raw_centroids: Vec<(f64, f64)>,
    /// Brightest-N centroids after undistortion, `(y, x)`. Verification set.
    pub centroids: Vec<(f64, f64)>,
    /// Unit vectors for `centroids` at the initial FOV.
    pub vectors: Vec<UnitVector>,
    /// Indices into `centroids` of the cluster-busted pattern stars, brightest first.
    pub pattern_indices: Vec<usize>,
}

/// Prepared inputs, or an early `TOO_FEW` solution.
pub type PrepareResult = Result<Prepared, Box<Solution>>;

/// Run the preparation stage and return either the prepared inputs or a solution
/// already terminated with `TOO_FEW`.
pub fn prepare<D>(
    ctx: &SolveContext<D>,
    centroids: &[(f64, f64)],
    width: f64,
    height: f64,
) -> PrepareResult {
    let raw = limit_centroids(centroids, ctx.verification_stars_per_fov);
    let undistorted = undistort(&raw, width, height, ctx.distortion);
    let pattern_indices = cluster_bust_indices(
        &undistorted,
        ctx.fov_initial,
        ctx.verification_stars_per_fov,
        width,
    );

    if undistorted.len() < MIN_CENTROIDS || pattern_indices.len() < MIN_CENTROIDS {
        return Err(Box::new(Solution {
            status: Some(SolveStatus::TooFew),
            fov_used: Some(ctx.fov_initial),
            ..Solution::default()
        }));
    }

    let camera = PinholeCamera::new(width, height, ctx.fov_initial.to_radians());
    let vectors: Vec<UnitVector> = camera
        .unproject(&undistorted)
        .into_iter()
        .map(|v| v.expect("pinhole unprojection has i = 1, never zero-length"))
        .collect();
    Ok(Prepared {
        raw_centroids: raw,
        centroids: undistorted,
        vectors,
        pattern_indices,
    })
}
