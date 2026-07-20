//! Solve entry points: `solve_from_centroids` and `solve_from_image`.

use crate::candidates;
use crate::preparation::{build_context, prepare};
use crate::status::{MatchResult, Solution, SolveContext, SolveStatus};
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
    db: PatternDatabase,
) -> Solution {
    let ctx = build_context(
        db,
        fov_estimate,
        fov_max_error,
        match_radius,
        match_threshold,
        match_max_error,
        distortion,
        solve_timeout,
    );

    let (width, height) = (size.0 as f64, size.1 as f64);

    let (vectors, _centroids) = match prepare(&ctx, centroids, width, height) {
        Ok(v) => v,
        Err(solution) => return solution,
    };

    iterate_patterns(&ctx, &vectors)
}

/// Solve a lost-in-space plate from a raw grayscale image.
///
/// Extracts centroids via `star_detection::detect_stars`, using the supplied
/// `DetectParams`. Records the extraction time in the returned `Solution`.
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
    db: PatternDatabase,
    detect_params: DetectParams,
) -> Solution {
    let sigma = detect_params.sigma;
    let noise_estimate = detect_params.noise_estimate.unwrap_or_else(|| estimate_noise(image, width, height));
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

    let centroids: Vec<(f64, f64)> = stars.iter().map(|s| (s.x, s.y)).collect();

    let mut solution = solve_from_centroids(
        &centroids,
        (width as usize, height as usize),
        fov_estimate,
        fov_max_error,
        match_radius,
        match_threshold,
        solve_timeout,
        distortion,
        match_max_error,
        db,
    );

    solution.match_probability = Some(noise_estimate);
    solution
}

/// Breadth-first iteration over 4-star combinations.
///
/// For each combination, generate a pattern key, look up candidates, and verify.
/// Returns immediately on the first accepted candidate, timeout, or cancellation.
fn iterate_patterns(ctx: &SolveContext, vectors: &[UnitVector]) -> Solution {
    let n = vectors.len();
    if n < 4 {
        return Solution {
            status: Some(SolveStatus::TooFew),
            fov_used: Some(ctx.fov_initial),
            ..Solution::default()
        };
    }

    // Breadth-first: outer loop over pattern radius (max index distance), inner loops over i0..i3.
    for radius in 1..n {
        for i0 in 0..n {
            if ctx.should_stop() {
                return stopped_solution(ctx);
            }
            for i1 in (i0 + 1)..n.min(i0 + radius + 1) {
                for i2 in (i1 + 1)..n.min(i0 + radius + 1) {
                    for i3 in (i2 + 1)..n.min(i0 + radius + 1) {
                        if ctx.should_stop() {
                            return stopped_solution(ctx);
                        }
                        let pattern = [vectors[i0], vectors[i1], vectors[i2], vectors[i3]];
                        let cands = candidates::lookup_candidates(ctx, pattern);
                        for candidate in cands {
                            if candidates::verify_candidate(ctx, &candidate, [i0, i1, i2, i3])
                                == MatchResult::Accepted
                            {
                                return Solution {
                                    status: Some(SolveStatus::MatchFound),
                                    fov_used: Some(ctx.fov_initial),
                                    pattern_candidates: vec![candidate],
                                    ..Solution::default()
                                };
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

fn stopped_solution(ctx: &SolveContext) -> Solution {
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
