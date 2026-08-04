//! Lost-in-space plate solver — solve entry points, preparation, and image-pattern iteration.
//!
//! This crate owns the front of the solve: `solve_from_centroids` / `solve_from_image`,
//! preparation (FOV initial, Bonferroni threshold, brightest-N limit, undistortion,
//! cluster-busting, centroid vectors, TOO_FEW), and the breadth-first image-pattern loop
//! with timeout and cancellation checks. Candidate-key generation, candidate
//! gathering, verification, and refinement live in the `candidates`, `verify`,
//! and `refine` modules.

pub mod candidates;
pub mod preparation;
pub mod refine;
pub mod solve;
pub mod status;
pub mod verify;

pub use solve::{solve_from_centroids, solve_from_image, DetectParams};
pub use status::{Solution, SolveStatus};
