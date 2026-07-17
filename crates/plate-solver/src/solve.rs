//! Solve entry points.
//!
//! Implements `solve_from_centroids` and the `solve_from_image` wrapper.

use std::time::Instant;

use star_detection::detect_stars;

use crate::input::{DetectParams, SolveOptions};
use crate::iteration::iterate_patterns;
use crate::preparation::prepare;
use crate::Solution;

/// Solve from a list of brightest-first `(y, x)` centroids.
///
/// `size` is `(height, width)` in pixels. `db` is the loaded pattern database.
/// `options` controls FOV estimate, thresholds, timeout, and cancellation.
pub fn solve_from_centroids(
    centroids: Vec<(f64, f64)>,
    size: (usize, usize),
    db: pattern_database::PatternDatabase,
    options: SolveOptions,
) -> Solution {
    let start = Instant::now();

    let detect = DetectParams::default().resolve();
    let prepared = match prepare(centroids, size, db, options.clone(), detect) {
        Ok(p) => p,
        Err(status) => return Solution::failure(status, elapsed_ms(start)),
    };

    iterate_patterns(prepared, start, options.solve_timeout_ms)
}

/// Solve from a grayscale image.
///
/// Extracts centroids via `star_detection::detect_stars`, records the extraction
/// time, then delegates to `solve_from_centroids`. Detection parameters are taken
/// from `detect`; when `noise_estimate` is not supplied, noise is estimated from
/// the image.
pub fn solve_from_image(
    image: &[u8],
    size: (usize, usize),
    db: pattern_database::PatternDatabase,
    detect: DetectParams,
    options: SolveOptions,
) -> Solution {
    let (height, width) = size;
    let detect_resolved = detect.resolve();

    let t0 = Instant::now();
    let stars = detect_stars(
        image,
        width,
        height,
        detect_resolved.sigma,
        detect_resolved.binning,
        detect_resolved.normalize_rows,
        detect_resolved.detect_hot_pixels,
    );
    let t_extract_ms = elapsed_ms(t0);

    let centroids: Vec<(f64, f64)> = stars.into_iter().map(|s| (s.y, s.x)).collect();

    let mut solution = solve_from_centroids(centroids, size, db, options);
    solution.t_extract_ms = Some(t_extract_ms);
    solution
}

pub(crate) fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SolveStatus;
    use pattern_database::{DatabaseProperties, PatternDatabase};

    fn default_db() -> PatternDatabase {
        PatternDatabase {
            star_table: Vec::new(),
            num_stars: 0,
            pattern_catalog: Vec::new(),
            pattern_largest_edge: Vec::new(),
            pattern_key_hashes: Vec::new(),
            star_catalog_ids: Vec::new(),
            properties: DatabaseProperties::default(),
        }
    }

    #[test]
    fn solve_from_centroids_too_few() {
        let db = default_db();
        let centroids = vec![(10.0, 10.0), (20.0, 20.0), (30.0, 30.0)];
        let solution = solve_from_centroids(centroids, (100, 100), db, SolveOptions::default());
        assert_eq!(solution.status, SolveStatus::TooFew);
        assert!(solution.ra.is_none());
    }

    #[test]
    fn solve_from_image_estimates_noise() {
        let db = default_db();
        let width = 80;
        let height = 60;
        let mut image = vec![50u8; width * height];
        // Add small perturbations so noise is above the floor.
        for (i, p) in image.iter_mut().enumerate() {
            let delta = ((i % 7) as i32) - 3;
            *p = (*p as i32 + delta).clamp(0, 255) as u8;
        }

        let detect = DetectParams {
            sigma: Some(8.0),
            noise_estimate: None,
            binning: Some(1),
            normalize_rows: Some(false),
            detect_hot_pixels: Some(false),
            return_binned: Some(false),
            use_binned_for_star_candidates: Some(false),
        };

        let solution = solve_from_image(
            &image,
            (height, width),
            db,
            detect,
            SolveOptions::default(),
        );
        assert!(solution.t_extract_ms.is_some());
        assert_eq!(solution.status, SolveStatus::TooFew);
    }
}
