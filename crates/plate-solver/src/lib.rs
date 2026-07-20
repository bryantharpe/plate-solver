//! Lost-in-space plate solver — solve entry points, preparation, and image-pattern iteration.
//!
//! This crate owns the front of the solve: `solve_from_centroids` / `solve_from_image`,
//! preparation (FOV initial, Bonferroni threshold, brightest-N limit, undistortion,
//! cluster-busting, centroid vectors, TOO_FEW), and the breadth-first image-pattern loop
//! with timeout and cancellation checks. Downstream beads fill candidate-key generation,
//! candidate gathering, verification, and refinement; they are declared here as modules
//! with stub implementations so their PRs touch disjoint files.

pub mod candidates;
pub mod preparation;
pub mod solve;
pub mod status;

// Downstream beads will own these modules; stubbed here so the crate compiles and
// downstream PRs do not collide on lib.rs.
pub mod refine {
    //! Refinement and output assembly (owned by a downstream bead).
    use crate::status::Solution;

    /// Placeholder for the final solution assembly step.
    pub fn assemble_solution() -> Solution {
        Solution::default()
    }
}

pub mod verify {
    //! Verification — attitude, projection/match, false-alarm acceptance (owned by a downstream bead).
    use crate::status::{MatchResult, SolveContext};

    /// Placeholder for the verification step.
    pub fn verify_candidate(_ctx: &SolveContext, _pattern_indices: [usize; 4]) -> MatchResult {
        MatchResult::Rejected
    }
}

pub use solve::{solve_from_centroids, solve_from_image, DetectParams};
pub use status::{Solution, SolveStatus};
