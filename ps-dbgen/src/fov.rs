use kiddo::{KdTree, SquaredEuclidean};

/// Compute the multiscale FOV pattern ladder.
///
/// Reference: tetra3.py lines 1167-1175:
///   fov_ratio = max_fov / min_fov
///   fov_divisions = ceil(log(fov_ratio) / log(multiscale_step)) + 1
///   if fov_ratio < sqrt(multiscale_step):
///       pattern_fovs = [max_fov]
///   else:
///       pattern_fovs = 2^linspace(log2(min_fov), log2(max_fov), fov_divisions)
///
/// Adjacent-equal dedup is applied to the result.
pub fn compute_fov_ladder(min_fov: f64, max_fov: f64, multiscale_step: f64) -> Vec<f64> {
    let fov_ratio = max_fov / min_fov;
    if fov_ratio < multiscale_step.sqrt() {
        return vec![max_fov];
    }

    let fov_divisions = (fov_ratio.ln() / multiscale_step.ln()).ceil() as usize + 1;

    let log2_min = min_fov.log2();
    let log2_max = max_fov.log2();

    let mut foos: Vec<f64> = if fov_divisions == 1 {
        vec![log2_min]
    } else {
        (0..fov_divisions)
            .map(|i| {
                let t = i as f64 / (fov_divisions - 1) as f64;
                log2_min + t * (log2_max - log2_min)
            })
            .collect()
    };

    // Adjacent-equal dedup
    foos.dedup();

    foos.into_iter().map(|log2_val| 2.0_f64.powf(log2_val)).collect()
}

/// Greedy brightest-first density thinning.
///
/// Reference: tetra3.py lines 1241-1251 + fov_util.py:
///   separation = 0.6 * fov / sqrt(stars_per_fov)  [same angular units as fov]
///   For each star (already sorted brightest-first by the caller):
///     if no kept star within pattern_stars_dist (chord of separation):
///       keep this star
///
/// `vectors` must be ordered brightest-first (same order as the star catalog).
/// `fov_rad` is the pattern FOV in radians.
/// `stars_per_fov` is the target density (e.g. pattern stars per FOV).
///
/// Returns indices into `vectors` of kept stars (in brightness order).
pub fn thin_stars_for_fov(vectors: &[[f32; 3]], fov_rad: f64, stars_per_fov: usize) -> Vec<usize> {
    if vectors.is_empty() {
        return vec![];
    }

    let separation = 0.6 * fov_rad / (stars_per_fov as f64).sqrt();
    // Convert angular separation to chord distance on the unit sphere
    let chord = 2.0 * (separation / 2.0).sin();
    let sq_chord = chord * chord;

    let mut tree: KdTree<f32, 3> = KdTree::with_capacity(vectors.len());
    let mut kept = Vec::new();

    for (idx, &vector) in vectors.iter().enumerate() {
        let results: Vec<_> = tree.within_unsorted::<SquaredEuclidean>(&vector, sq_chord as f32);
        if results.is_empty() {
            // No kept star within exclusion zone — keep this one
            tree.add(&vector, idx as u64);
            kept.push(idx);
        }
    }

    kept
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;

    /// Helper to create a unit vector at (ra, dec) in radians.
    fn vec_at(ra: f64, dec: f64) -> [f32; 3] {
        let cos_dec = dec.cos();
        [
            (ra.cos() * cos_dec) as f32,
            (ra.sin() * cos_dec) as f32,
            dec.sin() as f32,
        ]
    }

    #[test]
    fn test_fov_ladder_single_scale() {
        // fov_ratio = 1.1 / 1.0 = 1.1 < sqrt(1.5) ≈ 1.225 → single entry [max_fov]
        let ladder = compute_fov_ladder(1.0, 1.1, 1.5);
        assert_eq!(ladder.len(), 1);
        assert!((ladder[0] - 1.1).abs() < 1e-10);
    }

    #[test]
    fn test_fov_ladder_two_scale() {
        // min=1.0, max=1.5, step=1.5
        // fov_ratio = 1.5, sqrt(1.5) ≈ 1.225, ratio > sqrt(step) → multi-scale path
        // fov_divisions = ceil(log(1.5)/log(1.5)) + 1 = ceil(1.0) + 1 = 2
        // 2 entries: [1.0, 1.5]
        let ladder = compute_fov_ladder(1.0, 1.5, 1.5);
        assert_eq!(ladder.len(), 2);
        assert!((ladder[0] - 1.0).abs() < 1e-10, "first entry should be ~1.0");
        assert!((ladder[1] - 1.5).abs() < 1e-10, "second entry should be ~1.5");
    }

    #[test]
    fn test_fov_ladder_three_scale() {
        // min=1.0, max=2.0, step=1.5
        // fov_ratio = 2.0, sqrt(1.5) ≈ 1.225, ratio > sqrt(step) → multi-scale path
        // fov_divisions = ceil(log(2)/log(1.5)) + 1 = ceil(1.709) + 1 = 2 + 1 = 3
        // 3 entries: [1.0, sqrt(2), 2.0] ≈ [1.0, 1.414, 2.0]
        let ladder = compute_fov_ladder(1.0, 2.0, 1.5);
        assert_eq!(ladder.len(), 3);
        assert!((ladder[0] - 1.0).abs() < 1e-10, "first entry should be ~1.0");
        assert!(
            (ladder[1] - 2.0_f64.sqrt()).abs() < 1e-10,
            "middle entry should be ~sqrt(2) ≈ 1.414, got {}",
            ladder[1]
        );
        assert!((ladder[2] - 2.0).abs() < 1e-10, "last entry should be ~2.0");
    }

    #[test]
    fn test_thin_stars_keeps_all_sparse() {
        // 3 stars very far apart (120° separation on equator), fov=PI/6, stars_per_fov=4
        // separation = 0.6 * PI/6 / sqrt(4) = 0.6 * PI/6 / 2 = PI/20 ≈ 9°
        // Stars are 120° apart, well beyond exclusion zone → all kept
        let vectors = vec![
            vec_at(0.0, 0.0),
            vec_at(2.0 * PI / 3.0, 0.0),
            vec_at(4.0 * PI / 3.0, 0.0),
        ];
        let kept = thin_stars_for_fov(&vectors, PI / 6.0, 4);
        assert_eq!(kept.len(), 3, "all 3 sparse stars should be kept");
        assert_eq!(kept, vec![0, 1, 2]);
    }

    #[test]
    fn test_thin_stars_cluster_bust() {
        // 5 stars tightly clustered (within exclusion zone), fov=PI/6, stars_per_fov=4
        // separation = 0.6 * PI/6 / sqrt(4) = PI/20 ≈ 0.157 rad
        // chord ≈ 0.156 — place all stars within a tiny angular distance
        let vectors: Vec<[f32; 3]> = (0..5)
            .map(|i| vec_at(0.001 * i as f64, 0.001 * i as f64))
            .collect();
        let kept = thin_stars_for_fov(&vectors, PI / 6.0, 4);
        assert_eq!(kept.len(), 1, "only the brightest star should be kept");
        assert_eq!(kept[0], 0);
    }

    #[test]
    fn test_thin_stars_brightness_priority() {
        // 3 stars: star 0 and star 2 are very close, star 1 is far away
        // All in brightness order (brightest first)
        // Star 0 kept (first). Star 1 kept (far from star 0). Star 2 excluded (close to star 0).
        let vectors = vec![
            vec_at(0.0, 0.0),       // star 0 — kept
            vec_at(PI / 2.0, 0.0), // star 1 — kept (far from star 0)
            vec_at(0.001, 0.001),  // star 2 — excluded (close to star 0)
        ];
        let kept = thin_stars_for_fov(&vectors, PI / 6.0, 4);
        assert_eq!(kept.len(), 2, "stars 0 and 1 should be kept");
        assert_eq!(kept, vec![0, 1]);
    }
}
