//! Solve entry points: `solve_from_centroids` and `solve_from_image`.

use std::borrow::Borrow;

use crate::candidates;
use crate::preparation::{build_context, prepare, Prepared};
use crate::refine::refine_solution;
use crate::status::{Solution, SolveContext, SolveStatus};
use math_core::UnitVector;
use pattern_database::PatternDatabase;
use star_detection::{detect_stars, noise::estimate_noise};

/// Parameters controlling star extraction.
#[derive(Debug, Clone, Copy)]
pub struct DetectParams {
    pub sigma: f64,
    pub noise_estimate: Option<f64>,
    pub binning: usize,
    pub normalize_rows: bool,
    pub detect_hot_pixels: bool,
    pub return_binned: bool,
    pub use_binned_for_star_candidates: bool,
}

impl Default for DetectParams {
    fn default() -> Self {
        Self {
            sigma: 8.0,
            noise_estimate: None,
            binning: 1,
            normalize_rows: false,
            detect_hot_pixels: false,
            return_binned: false,
            use_binned_for_star_candidates: false,
        }
    }
}

/// Solve a lost-in-space plate from pre-extracted centroids.
///
/// This function owns the front of the solve loop: context construction,
/// preparation, and breadth-first image-pattern iteration. Verification and
/// refinement are delegated to downstream beads via the `candidates` module.
///
/// The solver only reads `db`, so a caller that keeps its database passes `&db`
/// (or `&*arc`) and pays no copy per solve. An owned `PatternDatabase` or an
/// `Arc<PatternDatabase>` works too.
#[allow(clippy::too_many_arguments)]
pub fn solve_from_centroids(
    centroids: &[(f64, f64)],
    size: (usize, usize),
    fov_estimate: Option<f64>,
    fov_max_error: f64,
    match_radius: f64,
    match_threshold: f64,
    solve_timeout: u64,
    distortion: f64,
    match_max_error: f64,
    db: impl Borrow<PatternDatabase>,
) -> Solution {
    let ctx = build_context(
        db.borrow(),
        fov_estimate,
        fov_max_error,
        match_radius,
        match_threshold,
        match_max_error,
        distortion,
        solve_timeout,
    );

    let (width, height) = (size.0 as f64, size.1 as f64);

    let prep = match prepare(&ctx, centroids, width, height) {
        Ok(v) => v,
        Err(solution) => return *solution,
    };

    iterate_patterns(&ctx, &prep, width, height)
}

/// Solve a lost-in-space plate from a raw grayscale image.
///
/// Extracts centroids via `star_detection::detect_stars`, using the supplied
/// `DetectParams`. Records the extraction time in the returned `Solution`.
///
/// Takes `db` as [`solve_from_centroids`] does; pass `&db` to keep the database.
#[allow(clippy::too_many_arguments)]
pub fn solve_from_image(
    image: &[u8],
    width: usize,
    height: usize,
    fov_estimate: Option<f64>,
    fov_max_error: f64,
    match_radius: f64,
    match_threshold: f64,
    solve_timeout: u64,
    distortion: f64,
    match_max_error: f64,
    db: impl Borrow<PatternDatabase>,
    detect_params: DetectParams,
) -> Solution {
    let sigma = detect_params.sigma;
    let noise_estimate = detect_params
        .noise_estimate
        .unwrap_or_else(|| estimate_noise(image, width, height));
    let binning = detect_params.binning;
    let normalize_rows = detect_params.normalize_rows;
    let detect_hot_pixels = detect_params.detect_hot_pixels;

    let stars = detect_stars(
        image,
        width,
        height,
        sigma,
        binning,
        normalize_rows,
        detect_hot_pixels,
    );

    // The solver speaks (y, x); `Star` carries named x (column) and y (row) fields.
    let centroids: Vec<(f64, f64)> = stars.iter().map(|s| (s.y, s.x)).collect();
    let mut solution = solve_from_centroids(
        &centroids,
        (width, height),
        fov_estimate,
        fov_max_error,
        match_radius,
        match_threshold,
        solve_timeout,
        distortion,
        match_max_error,
        db.borrow(),
    );
    // Report the noise estimate in its own field; `match_probability` is the
    // false-alarm probability and must not be overwritten.
    solution.noise_estimate = Some(noise_estimate);
    solution
}

/// Breadth-first iteration over 4-star combinations of the pattern stars.
///
/// Matches cedar's `breadth_first_combinations`: every combination is visited
/// exactly once, ordered by its faintest member, so all patterns drawn from the
/// brightest k stars are tried before any pattern that needs star k+1.
fn iterate_patterns(
    ctx: &SolveContext<&PatternDatabase>,
    prep: &Prepared,
    width: f64,
    height: f64,
) -> Solution {
    let p = &prep.pattern_indices;
    let n = p.len();
    for i3 in 3..n {
        for i2 in 2..i3 {
            for i1 in 1..i2 {
                for i0 in 0..i1 {
                    if ctx.should_stop() {
                        return stopped_solution(ctx);
                    }
                    let idx = [p[i0], p[i1], p[i2], p[i3]];
                    let pattern: [UnitVector; 4] = idx.map(|i| prep.vectors[i]);
                    for candidate in candidates::lookup_candidates(ctx, pattern) {
                        if let Some(outcome) = crate::verify::verify_candidate_with_outcome(
                            ctx,
                            &candidate,
                            idx,
                            &prep.vectors,
                            &prep.centroids,
                            width,
                            height,
                        ) {
                            if outcome.accepted {
                                return refine_solution(
                                    ctx, &candidate, &outcome, prep, width, height,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    Solution {
        status: Some(SolveStatus::NoMatch),
        fov_used: Some(ctx.fov_initial),
        ..Solution::default()
    }
}

fn stopped_solution(ctx: &SolveContext<&PatternDatabase>) -> Solution {
    let status = if ctx.is_cancelled() {
        SolveStatus::Cancelled
    } else {
        SolveStatus::Timeout
    };
    Solution {
        status: Some(status),
        fov_used: Some(ctx.fov_initial),
        ..Solution::default()
    }
}
