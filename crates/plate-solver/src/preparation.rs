//! Preparation stage of the solve.
//!
//! Sets the initial FOV, applies the Bonferroni correction, limits centroids to
//! the brightest verification budget, undistorts, cluster-busts, and precomputes
//! centroid vectors.

use database_generation::thinning::separation_for_density;
use math_core::{undistort_centroids, PinholeCamera, UnitVector};
use pattern_database::{DatabaseProperties, PatternDatabase};

use crate::input::ResolvedDetectParams;
use crate::{PreparedSolve, SolveOptions, SolveStatus};

/// Prepare the solve from inputs and a database.
pub(crate) fn prepare(
    centroids: Vec<(f64, f64)>,
    size: (usize, usize),
    db: PatternDatabase,
    options: SolveOptions,
    _detect: ResolvedDetectParams,
) -> Result<PreparedSolve, SolveStatus> {
    let (height, width) = size;
    let width_f = width as f64;
    let height_f = height as f64;

    // Default FOV from database range midpoint.
    let fov_initial_deg = options
        .fov_estimate
        .unwrap_or_else(|| (db.properties.min_fov + db.properties.max_fov) as f64 / 2.0);
    let fov_initial_rad = fov_initial_deg.to_radians();

    // Bonferroni correction.
    let num_patterns = db.properties.num_patterns.max(1) as f64;
    let working_threshold = options.match_threshold / num_patterns;

    // Brightest-N limit.
    let max_centroids = db.properties.verification_stars_per_fov as usize;
    let centroids: Vec<(f64, f64)> = centroids.into_iter().take(max_centroids).collect();

    // Undistort if a scalar distortion is supplied.
    let centroids = if let Some(k) = options.distortion {
        undistort_centroids(&centroids, width_f, height_f, k)
    } else {
        centroids
    };

    // Cluster-busting via database density rule.
    let separation_px = width_f
        * separation_for_density(fov_initial_deg, db.properties.verification_stars_per_fov as f64)
        / fov_initial_deg;
    let pattern_centroid_indices = cluster_bust(&centroids, width_f, height_f, separation_px);

    // Too few centroids to form a 4-star pattern.
    if pattern_centroid_indices.len() < 4 {
        return Err(SolveStatus::TooFew);
    }

    // Precompute centroid vectors once.
    let camera = PinholeCamera::new(width_f, height_f, fov_initial_rad);
    let vectors = camera.unproject(&centroids);

    Ok(PreparedSolve {
        camera,
        fov_initial_rad,
        working_threshold,
        centroids,
        vectors,
        pattern_centroid_indices,
        db,
        options,
    })
}

/// Greedy brightest-first density thinning in pixel space.
///
/// Keeps a centroid only if no already-kept centroid lies within
/// `separation_px` pixels. This prevents dense clusters from dominating the
/// pattern budget.
fn cluster_bust(
    centroids: &[(f64, f64)],
    _width: f64,
    _height: f64,
    separation_px: f64,
) -> Vec<usize> {
    let sep2 = separation_px * separation_px;
    let mut kept: Vec<usize> = Vec::new();

    for (i, &(y, x)) in centroids.iter().enumerate() {
        let mut too_close = false;
        for &k in &kept {
            let dy = y - centroids[k].0;
            let dx = x - centroids[k].1;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DetectParams;

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
    fn default_fov_from_db_midpoint() {
        let mut db = default_db();
        db.properties.min_fov = 20.0;
        db.properties.max_fov = 40.0;
        db.properties.num_patterns = 1000;
        db.properties.verification_stars_per_fov = 150;

        let centroids: Vec<(f64, f64)> = (0..10).map(|i| (i as f64 * 10.0, i as f64 * 10.0)).collect();
        let options = SolveOptions::default();
        let prepared = prepare(centroids, (100, 100), db, options, DetectParams::default().resolve())
            .expect("should prepare");

        let expected_fov = ((20.0_f64 + 40.0_f64) / 2.0).to_radians();
        assert!((prepared.fov_initial_rad - expected_fov).abs() < 1e-12);
    }

    #[test]
    fn bonferroni_threshold() {
        let mut db = default_db();
        db.properties.num_patterns = 1000;
        db.properties.verification_stars_per_fov = 150;

        let centroids: Vec<(f64, f64)> = (0..10).map(|i| (i as f64 * 10.0, i as f64 * 10.0)).collect();
        let mut options = SolveOptions::default();
        options.match_threshold = 1e-5;
        let prepared = prepare(centroids, (100, 100), db, options, DetectParams::default().resolve())
            .expect("should prepare");

        assert!((prepared.working_threshold - 1e-8).abs() < 1e-15);
    }

    #[test]
    fn too_few_centroids() {
        let mut db = default_db();
        db.properties.num_patterns = 1;
        db.properties.verification_stars_per_fov = 150;

        let centroids = vec![(10.0, 10.0), (20.0, 20.0), (30.0, 30.0)];
        let result = prepare(centroids, (100, 100), db, SolveOptions::default(), DetectParams::default().resolve());
        assert!(matches!(result, Err(SolveStatus::TooFew)));
    }

    #[test]
    fn cluster_busting_thins_tight_cluster() {
        let mut db = default_db();
        db.properties.min_fov = 30.0;
        db.properties.max_fov = 30.0;
        db.properties.num_patterns = 1;
        db.properties.verification_stars_per_fov = 150;

        // 10 centroids packed within a 5-pixel radius.
        let mut centroids = Vec::new();
        for i in 0..10 {
            let angle = i as f64 * 0.1;
            centroids.push((50.0 + angle.cos() * 2.0, 50.0 + angle.sin() * 2.0));
        }
        // 4 well-separated centroids outside the cluster.
        centroids.push((10.0, 10.0));
        centroids.push((10.0, 90.0));
        centroids.push((90.0, 10.0));
        centroids.push((90.0, 90.0));

        let prepared = prepare(
            centroids,
            (100, 100),
            db,
            SolveOptions::default(),
            DetectParams::default().resolve(),
        )
        .expect("should prepare");

        // The tight cluster should be reduced to a single centroid.
        assert!(prepared.pattern_centroid_indices.len() >= 4);
        assert!(prepared.pattern_centroid_indices.len() <= 8);
    }
}
