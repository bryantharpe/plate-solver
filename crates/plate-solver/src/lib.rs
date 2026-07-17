//! Lost-in-space plate solver: front of the solve pipeline.
//!
//! This crate owns the solve entry points, preparation, and image-pattern
//! iteration. Downstream beads fill in candidate-key generation, verification,
//! and refinement via the `candidates`, `verify`, and `refine` modules.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use math_core::{PinholeCamera, UnitVector};
use pattern_database::{DatabaseProperties, PatternDatabase};

pub mod candidates;
pub mod input;
pub mod iteration;
pub mod preparation;
pub mod refine;
pub mod solve;
pub mod verify;

pub use input::{DetectParams, SolveOptions};
pub use solve::{solve_from_centroids, solve_from_image};

/// Status of a solve attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolveStatus {
    /// A match was found and accepted.
    MatchFound,
    /// No match was found after exhausting the search.
    NoMatch,
    /// The solve timed out.
    Timeout,
    /// The solve was cancelled by the caller.
    Cancelled,
    /// Too few centroids were available to form a pattern.
    TooFew,
}

/// A single matched star pairing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Match {
    /// Image centroid index in the input centroid list.
    pub centroid_index: usize,
    /// Catalog star index in the database star table.
    pub star_index: usize,
    /// Source catalog ID, if available.
    pub catalog_id: Option<pattern_database::CatalogId>,
}

/// Result returned by the solver.
#[derive(Clone, Debug, PartialEq)]
pub struct Solution {
    /// Solve status.
    pub status: SolveStatus,
    /// Right ascension in degrees, if a match was found.
    pub ra: Option<f64>,
    /// Declination in degrees, if a match was found.
    pub dec: Option<f64>,
    /// Roll in degrees, if a match was found.
    pub roll: Option<f64>,
    /// Horizontal field of view in degrees, if a match was found.
    pub fov: Option<f64>,
    /// Radial distortion coefficient, if estimated or supplied.
    pub distortion: Option<f64>,
    /// Root-mean-square residual in arcseconds, if a match was found.
    pub rmse: Option<f64>,
    /// 90th-percentile residual in arcseconds, if a match was found.
    pub p90e: Option<f64>,
    /// Maximum residual in arcseconds, if a match was found.
    pub maxe: Option<f64>,
    /// Number of matched stars, if a match was found.
    pub matches: Option<usize>,
    /// False-alarm probability (Bonferroni-corrected), if a match was found.
    pub prob: Option<f64>,
    /// Extraction time in milliseconds, if the solve started from an image.
    pub t_extract_ms: Option<f64>,
    /// Solve time in milliseconds.
    pub t_solve_ms: f64,
    /// Matched star pairings, if requested and a match was found.
    pub matched: Vec<Match>,
}

impl Solution {
    /// Create a non-match solution with the given status and solve time.
    pub fn failure(status: SolveStatus, t_solve_ms: f64) -> Self {
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
            t_extract_ms: None,
            t_solve_ms,
            matched: Vec::new(),
        }
    }
}

/// A cancellation token that can be shared across threads.
#[derive(Clone, Debug)]
pub struct CancellationToken {
    flag: std::sync::Arc<AtomicBool>,
}

impl CancellationToken {
    /// Create a new token that is not cancelled.
    pub fn new() -> Self {
        Self {
            flag: std::sync::Arc::new(AtomicBool::new(false)),
        }
    }

    /// Request cancellation.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }

    /// Check whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal prepared solve state.
#[derive(Clone, Debug)]
pub(crate) struct PreparedSolve {
    pub camera: PinholeCamera,
    pub fov_initial_rad: f64,
    pub working_threshold: f64,
    pub centroids: Vec<(f64, f64)>,
    pub vectors: Vec<Option<UnitVector>>,
    pub pattern_centroid_indices: Vec<usize>,
    pub db: PatternDatabase,
    pub options: SolveOptions,
}

/// Check whether the solve has exceeded its timeout or been cancelled.
pub(crate) fn should_stop(
    start: Instant,
    timeout: Duration,
    cancel: Option<&CancellationToken>,
) -> Option<SolveStatus> {
    if start.elapsed() >= timeout {
        return Some(SolveStatus::Timeout);
    }
    if cancel.map(|c| c.is_cancelled()).unwrap_or(false) {
        return Some(SolveStatus::Cancelled);
    }
    None
}
