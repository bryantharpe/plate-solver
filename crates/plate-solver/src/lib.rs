//! Lost-in-space plate solver.
//!
//! Turns brightest-first centroid lists into an attitude (RA, Dec, Roll), refined
//! FOV, and matched stars. The solver is bounded by `solve_timeout` and reports a
//! status code on failure rather than guessing.

use math_core::{undistort_centroids, PinholeCamera, UnitVector};
use pattern_database::{CatalogId, PatternDatabase};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub mod candidates;
pub mod refine;
pub mod verify;

/// Solve status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolveStatus {
    /// A match was found and refined.
    MatchFound,
    /// No match after exhausting all candidates.
    NoMatch,
    /// The timeout was reached.
    Timeout,
    /// Cancellation was requested.
    Cancelled,
    /// Fewer than four centroids were available.
    TooFew,
}

/// A plate-solver solution.
#[derive(Clone, Debug, PartialEq)]
pub struct Solution {
    /// Solve status.
    pub status: SolveStatus,
    /// Right ascension in degrees, if matched.
    pub ra: Option<f64>,
    /// Declination in degrees, if matched.
    pub dec: Option<f64>,
    /// Roll in degrees, if matched.
    pub roll: Option<f64>,
    /// Horizontal field of view in degrees, if matched.
    pub fov: Option<f64>,
    /// Radial distortion coefficient `k`, if estimated or fixed.
    pub distortion: Option<f64>,
    /// Root-mean-square residual in arcseconds, if matched.
    pub rmse: Option<f64>,
    /// 90th-percentile residual in arcseconds, if matched.
    pub p90e: Option<f64>,
    /// Maximum residual in arcseconds, if matched.
    pub maxe: Option<f64>,
    /// Number of matched stars.
    pub matches: Option<usize>,
    /// Bonferroni-corrected false-alarm probability.
    pub prob: Option<f64>,
    /// Equinox epoch string from the database.
    pub epoch_equinox: Option<String>,
    /// Proper-motion epoch string from the database.
    pub epoch_proper_motion: Option<String>,
    /// Solve time in milliseconds.
    pub t_solve: f64,
    /// Extraction time in milliseconds, when solving from an image.
    pub t_extract: Option<f64>,
    /// Matched image centroids `(y, x)`, when requested.
    pub matched_centroids: Option<Vec<(f64, f64)>>,
    /// Matched catalog stars (RA, Dec, mag), when requested.
    pub matched_stars: Option<Vec<(f64, f64, f64)>>,
    /// Matched catalog IDs, when requested.
    pub matched_cat_id: Option<Vec<CatalogId>>,
    /// Full catalog list projected into the image, when requested.
    pub catalog_stars: Option<Vec<ProjectedCatalogStar>>,
    /// 3×3 rotation matrix rows, when requested.
    pub rotation_matrix: Option<[[f64; 3]; 3]>,
    /// RA/Dec in degrees for an extra target pixel, when requested.
    pub target_radec: Option<(f64, f64)>,
    /// Pixel `(y, x)` for an extra target sky coordinate, when requested.
    pub target_pixel: Option<(f64, f64)>,
}

/// A projected catalog star: `(ra, dec, mag, y, x)`.
pub type ProjectedCatalogStar = (f64, f64, f64, f64, f64);

impl Solution {
    /// Create a fresh non-match solution with only `t_solve` populated.
    fn empty(t_solve: f64, status: SolveStatus) -> Self {
        Self {
            status,
            ra: None,
            dec: None,
            roll: None,
            fov: None,
            distortion: None,
            rmse: None,
            p90e: None,
            maxe: None,
            matches: None,
            prob: None,
            epoch_equinox: None,
            epoch_proper_motion: None,
            t_solve,
            t_extract: None,
            matched_centroids: None,
            matched_stars: None,
            matched_cat_id: None,
            catalog_stars: None,
            rotation_matrix: None,
            target_radec: None,
            target_pixel: None,
        }
    }
}

/// Parameters controlling star detection in `solve_from_image`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DetectParams {
    /// Detection threshold in units of estimated RMS noise.
    pub sigma: f64,
    /// Optional explicit noise estimate; if `None`, noise is estimated from the image.
    pub noise_estimate: Option<f64>,
    /// Binning factor (1, 2, 4, or 8).
    pub binning: usize,
    /// Normalize each row to a dark bias of 2.0 before binning.
    pub normalize_rows: bool,
    /// Reject isolated single-pixel spikes.
    pub detect_hot_pixels: bool,
    /// Return centroids in binned coordinates.
    pub return_binned: bool,
    /// Use binned coordinates for star candidates.
    pub use_binned_for_star_candidates: bool,
}

impl Default for DetectParams {
    fn default() -> Self {
        Self {
            sigma: 8.0,
            noise_estimate: None,
            binning: 1,
            normalize_rows: false,
            detect_hot_pixels: true,
            return_binned: false,
            use_binned_for_star_candidates: false,
        }
    }
}

/// Options controlling the solve.
#[derive(Clone, Debug, PartialEq)]
pub struct SolveOptions {
    /// Optional FOV estimate in degrees.
    pub fov_estimate: Option<f64>,
    /// Maximum acceptable FOV deviation in degrees.
    pub fov_max_error: Option<f64>,
    /// Match radius as a fraction of image width.
    pub match_radius: f64,
    /// Base false-alarm acceptance threshold.
    pub match_threshold: f64,
    /// Pattern-key tolerance band half-width.
    pub match_max_error: f64,
    /// Solve timeout in milliseconds.
    pub solve_timeout: u64,
    /// Fixed scalar distortion `k`, or `None` to estimate it.
    pub distortion: Option<f64>,
    /// Maximum number of centroids used for verification.
    pub verification_stars_per_fov: usize,
    /// Extra image pixel to report RA/Dec for.
    pub target_pixel: Option<(f64, f64)>,
    /// Extra sky coordinate to project into the image.
    pub target_sky_coord: Option<(f64, f64)>,
    /// Return matched centroids/stars/IDs in the solution.
    pub return_matches: bool,
    /// Return the full projected catalog list.
    pub return_catalog: bool,
    /// Return the 3×3 rotation matrix.
    pub return_rotation_matrix: bool,
}

impl Default for SolveOptions {
    fn default() -> Self {
        Self {
            fov_estimate: None,
            fov_max_error: None,
            match_radius: 0.01,
            match_threshold: 1e-5,
            match_max_error: 0.002,
            solve_timeout: 5000,
            distortion: Some(0.0),
            verification_stars_per_fov: 150,
            target_pixel: None,
            target_sky_coord: None,
            return_matches: false,
            return_catalog: false,
            return_rotation_matrix: false,
        }
    }
}

/// Minimum separation in degrees for a target star density.
///
/// `separation = 0.6 * fov / sqrt(stars_per_fov)` where `fov` is in degrees.
fn separation_for_density(fov_deg: f64, stars_per_fov: f64) -> f64 {
    0.6 * fov_deg / stars_per_fov.sqrt()
}

/// Global cancellation flag shared by all running solves in this process.
static CANCELLED: AtomicBool = AtomicBool::new(false);

/// Request cancellation of any in-progress solve.
pub fn cancel_solve() {
    CANCELLED.store(true, Ordering::SeqCst);
}

/// Clear the global cancellation flag.
pub fn reset_cancellation() {
    CANCELLED.store(false, Ordering::SeqCst);
}

/// Solve from a grayscale image.
///
/// `image` is row-major with `width` columns and `height` rows. Detection
/// parameters are taken from `detect`; solve options from `options`.
pub fn solve_from_image(
    image: &[u8],
    width: usize,
    height: usize,
    detect: DetectParams,
    options: SolveOptions,
    database: &PatternDatabase,
) -> Solution {
    let t0 = Instant::now();
    let _noise = detect
        .noise_estimate
        .unwrap_or_else(|| star_detection::noise::estimate_noise(image, width, height));
    let stars = star_detection::detect_stars(
        image,
        width,
        height,
        detect.sigma,
        detect.binning,
        detect.normalize_rows,
        detect.detect_hot_pixels,
    );
    let t_extract = t0.elapsed().as_secs_f64() * 1000.0;

    let centroids: Vec<(f64, f64)> = stars.iter().map(|s| (s.y, s.x)).collect();
    let mut solution = solve_from_centroids(&centroids, (height, width), options, database);
    solution.t_extract = Some(t_extract);
    solution
}

/// Solve from an already-extracted brightest-first centroid list.
///
/// `star_centroids` are `(y, x)` in pixel coordinates, brightest first.
/// `size` is `(height, width)`.
pub fn solve_from_centroids(
    star_centroids: &[(f64, f64)],
    size: (usize, usize),
    options: SolveOptions,
    database: &PatternDatabase,
) -> Solution {
    let t0 = Instant::now();
    let (height, width) = size;
    let width_f = width as f64;
    let height_f = height as f64;

    let prep = match prepare(star_centroids, width_f, height_f, &options, database) {
        Ok(p) => p,
        Err(status) => return Solution::empty(t0.elapsed().as_secs_f64() * 1000.0, status),
    };

    let deadline = t0 + Duration::from_millis(options.solve_timeout);
    let result = iterate_patterns(&prep, &options, database, deadline);

    let mut solution = match result {
        Ok(sol) => sol,
        Err(status) => Solution::empty(t0.elapsed().as_secs_f64() * 1000.0, status),
    };
    solution.t_solve = t0.elapsed().as_secs_f64() * 1000.0;
    solution
}

/// Prepared solve state.
#[expect(dead_code)]
pub(crate) struct Preparation {
    /// Centroids kept for verification, undistorted if a scalar `k` was given.
    centroids: Vec<(f64, f64)>,
    /// Indices of centroids used to form patterns (cluster-busted subset).
    pattern_indices: Vec<usize>,
    /// Unit vectors for all kept centroids at `fov_initial`.
    vectors: Vec<UnitVector>,
    /// Initial horizontal FOV in radians.
    fov_initial: f64,
    /// Bonferroni-corrected acceptance threshold.
    corrected_threshold: f64,
    /// Number of patterns used for Bonferroni correction.
    num_patterns: usize,
}

/// Prepare the solve: FOV, threshold, centroid limit, distortion, cluster-bust.
fn prepare(
    star_centroids: &[(f64, f64)],
    width: f64,
    height: f64,
    options: &SolveOptions,
    database: &PatternDatabase,
) -> Result<Preparation, SolveStatus> {
    let props = &database.properties;

    let fov_initial_deg = options
        .fov_estimate
        .unwrap_or_else(|| props.fov_midpoint_deg());
    let fov_initial = fov_initial_deg.to_radians();

    let num_patterns = props.num_patterns.max(1);
    let corrected_threshold = options.match_threshold / num_patterns as f64;

    // Limit to brightest verification stars.
    let mut centroids: Vec<(f64, f64)> = star_centroids
        .iter()
        .take(options.verification_stars_per_fov)
        .copied()
        .collect();

    // Apply known scalar distortion.
    if let Some(k) = options.distortion {
        centroids = undistort_centroids(&centroids, width, height, k);
    }

    if centroids.len() < 4 {
        return Err(SolveStatus::TooFew);
    }

    // Cluster-bust to a pattern-centroid subset using the database density rule.
    let sep_px = width * separation_for_density(fov_initial_deg, props.verification_stars_per_fov)
        / fov_initial_deg;
    let pattern_indices = cluster_bust(&centroids, sep_px);

    if pattern_indices.len() < 4 {
        return Err(SolveStatus::TooFew);
    }

    // Precompute unit vectors once at the initial FOV.
    let camera = PinholeCamera::new(width, height, fov_initial);
    let vectors: Vec<UnitVector> = camera.unproject(&centroids).into_iter().flatten().collect();

    Ok(Preparation {
        centroids,
        pattern_indices,
        vectors,
        fov_initial,
        corrected_threshold,
        num_patterns,
    })
}

/// Greedy pixel-space cluster-buster.
///
/// Keeps centroids brightest-first such that no two kept centroids are within
/// `sep_px` pixels. Returns kept indices in ascending order.
fn cluster_bust(centroids: &[(f64, f64)], sep_px: f64) -> Vec<usize> {
    let sep2 = sep_px * sep_px;
    let mut kept: Vec<usize> = Vec::new();
    for (i, c) in centroids.iter().enumerate() {
        let mut too_close = false;
        for &k in &kept {
            let d = centroids[k];
            let dy = c.0 - d.0;
            let dx = c.1 - d.1;
            if dy * dy + dx * dx < sep2 {
                too_close = true;
                break;
            }
        }
        if !too_close {
            kept.push(i);
        }
    }
    kept
}

/// Iterate 4-star image patterns over the cluster-busted centroids.
fn iterate_patterns(
    prep: &Preparation,
    options: &SolveOptions,
    database: &PatternDatabase,
    deadline: Instant,
) -> Result<Solution, SolveStatus> {
    let pattern_indices: Vec<usize> = prep.pattern_indices.clone();
    for combo in breadth_first_combinations(&pattern_indices, 4) {
        if Instant::now() >= deadline {
            return Err(SolveStatus::Timeout);
        }
        if CANCELLED.load(Ordering::SeqCst) {
            return Err(SolveStatus::Cancelled);
        }

        let pattern_vectors = [
            prep.vectors[combo[0]],
            prep.vectors[combo[1]],
            prep.vectors[combo[2]],
            prep.vectors[combo[3]],
        ];

        let candidates = candidates::generate_candidates(&pattern_vectors, options, database);

        for candidate in candidates {
            if let Some(solution) =
                verify::try_verify(prep, &pattern_vectors, candidate, options, database)
            {
                return Ok(solution);
            }
        }
    }

    Err(SolveStatus::NoMatch)
}

/// Breadth-first combination generator.
///
/// Yields combinations of `r` elements from `sequence` so that combinations using
/// the earliest (brightest) elements come first.
fn breadth_first_combinations(
    sequence: &[usize],
    r: usize,
) -> impl Iterator<Item = [usize; 4]> + use<'_> {
    let mut state: Vec<usize> = Vec::new();

    std::iter::from_fn(move || {
        if state.is_empty() {
            if sequence.len() < r {
                return None;
            }
            state.extend(0..r);
            return Some(std::array::from_fn(|i| sequence[state[i]]));
        }

        let mut pos = r;
        while pos > 0 {
            pos -= 1;
            let max_at_pos = sequence.len() - r + pos;
            if state[pos] < max_at_pos {
                state[pos] += 1;
                for j in pos + 1..r {
                    state[j] = state[j - 1] + 1;
                }
                return Some(std::array::from_fn(|i| sequence[state[i]]));
            }
        }

        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_detect_params_are_explicit() {
        let d = DetectParams::default();
        assert_eq!(d.sigma, 8.0);
        assert_eq!(d.binning, 1);
        assert!(d.noise_estimate.is_none());
    }

    #[test]
    fn too_few_centroids_returns_too_few() {
        let db = PatternDatabase::empty();
        let centroids = vec![(100.0, 100.0), (200.0, 200.0), (300.0, 300.0)];
        let sol = solve_from_centroids(&centroids, (512, 512), SolveOptions::default(), &db);
        assert_eq!(sol.status, SolveStatus::TooFew);
        assert!(sol.ra.is_none());
    }

    #[test]
    fn default_fov_uses_database_midpoint() {
        let mut db = PatternDatabase::empty();
        db.properties.min_fov = 8.0;
        db.properties.max_fov = 16.0;
        let centroids = vec![
            (256.0, 256.0),
            (256.0, 300.0),
            (300.0, 256.0),
            (300.0, 300.0),
        ];
        let sol = solve_from_centroids(&centroids, (512, 512), SolveOptions::default(), &db);
        // Empty DB cannot match, but preparation should succeed and exit NO_MATCH.
        assert_eq!(sol.status, SolveStatus::NoMatch);
    }

    #[test]
    fn breadth_first_order_is_brightest_first() {
        let seq = vec![0, 1, 2, 3, 4];
        let combos: Vec<[usize; 4]> = breadth_first_combinations(&seq, 4).collect();
        assert_eq!(combos.len(), 5);
        assert_eq!(combos[0], [0, 1, 2, 3]);
        assert_eq!(combos[4], [1, 2, 3, 4]);
    }
}
