//! Image-pattern iteration.
//!
//! Enumerates 4-star combinations over cluster-busted centroids in breadth-first
//! order, checking timeout and cancellation on each iteration.

use std::time::{Duration, Instant};

use crate::{PreparedSolve, Solution, SolveStatus};

/// Iterate over image patterns until a match is found, timeout, cancellation, or
/// exhaustion.
pub(crate) fn iterate_patterns(
    prepared: PreparedSolve,
    start: Instant,
    timeout_ms: u64,
) -> Solution {
    let timeout = Duration::from_millis(timeout_ms);
    let indices = &prepared.pattern_centroid_indices;
    let n = indices.len();

    // Breadth-first 4-star combinations: advance the last index slowest so the
    // brightest combinations are tried first.
    let mut state: [usize; 4] = [0, 1, 2, 3];
    let mut first = true;

    loop {
        if let Some(status) = crate::should_stop(start, timeout, prepared.options.cancel.as_ref()) {
            return Solution::failure(status, elapsed_ms(start));
        }

        if first {
            if n < 4 {
                return Solution::failure(SolveStatus::TooFew, elapsed_ms(start));
            }
            first = false;
        } else {
            let mut advanced = false;
            let mut pos = 4usize;
            while pos > 0 {
                pos -= 1;
                let max_at_pos = n - 4 + pos;
                if state[pos] < max_at_pos {
                    state[pos] += 1;
                    for j in pos + 1..4 {
                        state[j] = state[j - 1] + 1;
                    }
                    advanced = true;
                    break;
                }
            }
            if !advanced {
                return Solution::failure(SolveStatus::NoMatch, elapsed_ms(start));
            }
        }

        // Placeholder: downstream beads will generate candidates and verify them.
        // For now, every pattern is a non-match so the loop exhausts.
    }
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::SolveOptions;
    use math_core::PinholeCamera;
    use pattern_database::{DatabaseProperties, PatternDatabase};

    fn prepared(n: usize) -> PreparedSolve {
        let centroids: Vec<(f64, f64)> = (0..n).map(|i| (i as f64 * 10.0, i as f64 * 10.0)).collect();
        let camera = PinholeCamera::new(100.0, 100.0, 0.5);
        let vectors = camera.unproject(&centroids);
        PreparedSolve {
            camera,
            fov_initial_rad: 0.5,
            working_threshold: 1e-8,
            centroids,
            vectors,
            pattern_centroid_indices: (0..n).collect(),
            db: PatternDatabase {
                star_table: Vec::new(),
                num_stars: 0,
                pattern_catalog: Vec::new(),
                pattern_largest_edge: Vec::new(),
                pattern_key_hashes: Vec::new(),
                star_catalog_ids: Vec::new(),
                properties: DatabaseProperties::default(),
            },
            options: SolveOptions::default(),
        }
    }

    #[test]
    fn timeout_returns_timeout() {
        let mut prep = prepared(10);
        prep.options.solve_timeout_ms = 0;
        let solution = iterate_patterns(prep, Instant::now(), 0);
        assert_eq!(solution.status, SolveStatus::Timeout);
    }

    #[test]
    fn cancellation_returns_cancelled() {
        let mut prep = prepared(10);
        let cancel = crate::CancellationToken::new();
        cancel.cancel();
        prep.options.cancel = Some(cancel);
        let solution = iterate_patterns(prep, Instant::now(), 5000);
        assert_eq!(solution.status, SolveStatus::Cancelled);
    }

    #[test]
    fn exhaustion_returns_no_match() {
        let prep = prepared(6);
        let solution = iterate_patterns(prep, Instant::now(), 5000);
        assert_eq!(solution.status, SolveStatus::NoMatch);
    }
}
