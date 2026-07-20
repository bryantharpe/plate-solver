//! Shared status and solution types for the plate solver.

use math_core::{PinholeCamera, UnitVector};
use pattern_database::Candidate;

/// Final status of a solve attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveStatus {
    /// A match was found and verified.
    MatchFound,
    /// No match was found within the search budget.
    NoMatch,
    /// The solve timed out.
    Timeout,
    /// The solve was cancelled by the caller.
    Cancelled,
    /// Too few centroids to attempt a solve.
    TooFew,
}

/// Result of verifying one candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchResult {
    /// Candidate accepted as the solution.
    Accepted,
    /// Candidate rejected; continue searching.
    Rejected,
}

/// A solved (or failed) solution.
#[derive(Debug, Clone, Default)]
pub struct Solution {
    pub status: Option<SolveStatus>,
    pub camera: Option<PinholeCamera>,
    pub matched_centroids: Vec<(f64, f64)>,
    pub matched_stars: Vec<UnitVector>,
    pub matched_catalog_ids: Vec<usize>,
    pub match_probability: Option<f64>,
    pub fov_used: Option<f64>,
    pub pattern_candidates: Vec<Candidate>,
}

/// Context carried through the solve pipeline.
#[derive(Debug, Clone)]
pub struct SolveContext {
    pub db: pattern_database::PatternDatabase,
    pub props: pattern_database::DatabaseProperties,
    pub fov_initial: f64,
    pub match_threshold: f64,
    pub match_radius: f64,
    pub match_max_error: f64,
    pub distortion: f64,
    pub solve_timeout_ms: u64,
    pub start_instant: std::time::Instant,
    pub cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub verification_stars_per_fov: usize,
}

impl SolveContext {
    /// Returns true if the caller has requested cancellation.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Returns true if the timeout has elapsed.
    pub fn is_timed_out(&self) -> bool {
        self.start_instant.elapsed().as_millis() as u64 >= self.solve_timeout_ms
    }

    /// Returns true if either cancellation or timeout has fired.
    pub fn should_stop(&self) -> bool {
        self.is_cancelled() || self.is_timed_out()
    }
}
