//! Verification of a candidate catalog pattern.
//!
//! Owned by the current bead as a stub; full implementation is downstream work.

use crate::{candidates::Candidate, Preparation, Solution, SolveOptions, SolveStatus};
use math_core::UnitVector;
use pattern_database::PatternDatabase;

/// Try to verify a candidate pattern. Returns `Some(Solution)` on acceptance.
pub(crate) fn try_verify(
    _prep: &Preparation,
    _image_vectors: &[UnitVector; 4],
    _candidate: Candidate,
    _options: &SolveOptions,
    _database: &PatternDatabase,
) -> Option<Solution> {
    // TODO(ps-plate-03): implement full verification pipeline.
    None
}

/// Build a non-match solution with the given status.
pub fn failure(status: SolveStatus, t_solve: f64) -> Solution {
    Solution::empty(t_solve, status)
}
