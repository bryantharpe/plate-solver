//! Candidate-key generation and database lookup (owned by a downstream bead).
//!
//! This module is intentionally a stub. The front of the solve loop declares it
//! so that the crate compiles and the downstream bead can replace the body without
//! touching `lib.rs`.

use crate::status::{MatchResult, SolveContext};
use math_core::UnitVector;
use pattern_database::Candidate;

/// Generate a pattern key from four unit vectors and look up candidates.
pub fn lookup_candidates(
    _ctx: &SolveContext,
    _vectors: [UnitVector; 4],
) -> Vec<Candidate> {
    Vec::new()
}

/// Verify a single candidate against the current context.
pub fn verify_candidate(
    _ctx: &SolveContext,
    _candidate: &Candidate,
    _pattern_indices: [usize; 4],
) -> MatchResult {
    MatchResult::Rejected
}
