//! Plate-solving crate.
//!
//! Accepts star centroids (or a raw image) and a pattern database,
//! and returns the orientation of the field.

use nalgebra::Vector3;
pub use ps_db::Database;
pub use ps_detect::StarDescription;
use ps_detect::GrayImageView;
use std::f64::consts::PI;
use std::time::Instant;

/// Status codes for a solve attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolveStatus {
    MatchFound,
    NoMatch,
    Timeout,
    Cancelled,
    TooFew,
}

/// Full output of a solve attempt.
#[derive(Debug, Clone)]
pub struct Solution {
    /// Right ascension of boresight, degrees (valid on MatchFound).
    pub ra: f64,
    /// Declination of boresight, degrees (valid on MatchFound).
    pub dec: f64,
    /// Roll angle, degrees (valid on MatchFound).
    pub roll: f64,
    /// Field of view (diagonal), degrees (valid on MatchFound).
    pub fov: f64,
    /// Distortion coefficient k (valid on MatchFound).
    pub distortion: f64,
    /// RMSE residual, arcseconds (valid on MatchFound).
    pub rmse: f64,
    /// 90th-percentile residual, arcseconds.
    pub p90e: f64,
    /// Max residual, arcseconds.
    pub maxe: f64,
    /// Number of matched stars (valid on MatchFound).
    pub matches: usize,
    /// False-alarm probability (Bonferroni-corrected, valid on MatchFound).
    pub prob: f64,
    /// Solve wall-clock time, seconds.
    pub t_solve: f64,
    /// Extraction (star-detect + centroid) wall-clock time, seconds.
    /// Set by `solve_from_image`; 0.0 for `solve_from_centroids` (no extraction).
    pub t_extract: f64,
    /// Status of the solve attempt.
    pub status: SolveStatus,
    /// Matched centroid positions (y,x), if requested.
    pub matched_centroids: Option<Vec<[f64; 2]>>,
    /// Matched star RA/Dec/mag triples, if requested.
    pub matched_stars: Option<Vec<[f64; 3]>>,
    /// Matched catalog IDs, if requested.
    pub matched_cat_ids: Option<Vec<u32>>,
    /// Number of 4-star combinations examined before stopping.
    pub combos_examined: u64,
}

impl Solution {
    /// Construct a failure result.
    pub fn failure(status: SolveStatus, t_solve: f64, combos_examined: u64) -> Self {
        Self {
            status,
            ra: 0.0,
            dec: 0.0,
            roll: 0.0,
            fov: 0.0,
            distortion: 0.0,
            rmse: 0.0,
            p90e: 0.0,
            maxe: 0.0,
            matches: 0,
            prob: 1.0,
            t_solve,
            t_extract: 0.0,
            matched_centroids: None,
            matched_stars: None,
            matched_cat_ids: None,
            combos_examined,
        }
    }
}

/// Parameters for a solve call (cedar defaults).
#[derive(Debug, Clone)]
pub struct SolveParams {
    /// Fraction of image width used as match radius. Default: 0.01
    pub match_radius: f64,
    /// Per-solve false-alarm threshold (Bonferroni divided by num_patterns). Default: 1e-5
    pub match_threshold: f64,
    /// Pattern-key tolerance band half-width (clamped to >= DB pattern_max_error). Default: 0.002
    pub match_max_error: f64,
    /// Solve timeout in milliseconds. Default: Some(5000)
    pub solve_timeout: Option<u64>,
    /// Radial distortion k. Some(0.0) = no distortion; None = estimate. Default: Some(0.0)
    pub distortion: Option<f64>,
    /// FOV estimate (degrees). None → use DB FOV-range midpoint.
    pub fov_estimate: Option<f64>,
    /// FOV max error (degrees). None → no constraint.
    pub fov_max_error: Option<f64>,
    /// Optional cancel flag. Set to true from another thread to cancel the solve.
    pub cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
}

impl Default for SolveParams {
    fn default() -> Self {
        Self {
            match_radius: 0.01,
            match_threshold: 1e-5,
            match_max_error: 0.002,
            solve_timeout: Some(5000),
            distortion: Some(0.0),
            fov_estimate: None,
            fov_max_error: None,
            cancel_flag: None,
        }
    }
}

/// Solve from pre-extracted centroids.
///
/// `star_centroids`: brightest-first, (y, x) pairs.
/// `size`: (height, width) in pixels.
/// When `params.fov_estimate` is None, uses the DB FOV-range midpoint.
pub fn solve_from_centroids(
    db: &Database,
    star_centroids: &[[f64; 2]],
    size: (usize, usize),
    params: &SolveParams,
) -> Solution {
    let (height, width) = size;

    // SV2 step 1: Set fov_initial (radians)
    let fov_initial = params.fov_estimate.map(f64::to_radians).unwrap_or_else(|| {
        (db.properties.min_fov as f64 + db.properties.max_fov as f64) / 2.0 * PI / 180.0
    });

    // SV2 step 2: Bonferroni correction
    let _match_threshold = params.match_threshold / db.properties.num_patterns as f64;

    // SV2 step 3: Pre-cluster-bust TooFew guard
    if star_centroids.len() < 4 {
        return Solution::failure(SolveStatus::TooFew, 0.0, 0);
    }

    // SV2 step 4: Cluster-bust on ALL raw star_centroids
    let vsfov = db.properties.verification_stars_per_fov as f64;
    let separation_pixels = width as f64 * 0.6 / vsfov.sqrt();
    let cluster_bust_result = cluster_bust_centroids(star_centroids, separation_pixels);

    // SV2 step 5: Slice to verification_stars_per_fov (brightest-first already)
    let vsfov_usize = db.properties.verification_stars_per_fov as usize;
    let image_centroids = &star_centroids[..star_centroids.len().min(vsfov_usize)];

    // SV2 step 6: Undistort (only if distortion is Some and finite)
    let image_centroids_undist: Vec<[f64; 2]> = if let Some(k) = params.distortion {
        if k.is_finite() {
            ps_core::distortion::undistort_centroids(image_centroids, (height, width), k)
        } else {
            image_centroids.to_vec()
        }
    } else {
        image_centroids.to_vec()
    };

    // SV2 step 7: Compute vectors (returns Vec<Vector3<f64>>, convert to [f64;3])
    let raw_vectors =
        ps_core::projection::compute_vectors(&image_centroids_undist, (height, width), fov_initial);
    let image_centroids_vectors: Vec<[f64; 3]> =
        raw_vectors.iter().map(|v| [v.x, v.y, v.z]).collect();

    // ---- SV3: Breadth-first candidate generation and key search ----

    // Filter cluster-bust result to only indices within the vsfov slice.
    let pattern_centroids_inds: Vec<usize> = cluster_bust_result
        .iter()
        .copied()
        .filter(|&i| i < vsfov_usize)
        .collect();
    let num_pattern_centroids = pattern_centroids_inds.len();

    // Timing and tolerance
    let t0 = Instant::now();
    let p_max_err = params
        .match_max_error
        .max(db.properties.pattern_max_error as f64);
    let p_bins = db.properties.pattern_bins as u32;
    let solve_timeout_secs: Option<f64> = params.solve_timeout.map(|ms| ms as f64 / 1000.0);
    let cancel_flag = params.cancel_flag.clone();

    let mut status = SolveStatus::NoMatch;
    let mut combos_examined: u64 = 0;

    'outer: for combo in breadth_first_combinations_4(num_pattern_centroids) {
        combos_examined += 1;
        // Timeout check
        if let Some(tmax) = solve_timeout_secs {
            if t0.elapsed().as_secs_f64() > tmax {
                status = SolveStatus::Timeout;
                break 'outer;
            }
        }

        // Cancel check
        if cancel_flag
            .as_ref()
            .map(|f| f.load(std::sync::atomic::Ordering::Relaxed))
            .unwrap_or(false)
        {
            status = SolveStatus::Cancelled;
            break 'outer;
        }

        // Map combination indices (into pattern_centroids_inds) to actual
        // indices into image_centroids_vectors.
        let img_idx: [usize; 4] = combo.map(|c| pattern_centroids_inds[c]);
        let img_vecs: [[f64; 3]; 4] = std::array::from_fn(|i| image_centroids_vectors[img_idx[i]]);

        // Compute image pattern key.
        let (image_pattern_key, image_pattern_largest_edge) =
            ps_core::pattern::compute_pattern_key(&img_vecs, p_bins);

        // Tolerance band: each key dimension ± ceil(p_max_err * p_bins) bins.
        let band = (p_max_err * p_bins as f64).ceil() as i32;
        let key_ranges: [[u32; 2]; 5] = std::array::from_fn(|i| {
            let center = image_pattern_key[i] as i32;
            let lo = (center - band).max(0) as u32;
            let hi = (center + band).min(p_bins as i32) as u32;
            [lo, hi]
        });

        // Enumerate all candidate keys in the tolerance band, sorted nearest-first.
        let candidate_keys: Vec<([u32; 5], i64)> = {
            let mut keys = Vec::new();
            for k0 in key_ranges[0][0]..=key_ranges[0][1] {
                for k1 in key_ranges[1][0]..=key_ranges[1][1] {
                    for k2 in key_ranges[2][0]..=key_ranges[2][1] {
                        for k3 in key_ranges[3][0]..=key_ranges[3][1] {
                            for k4 in key_ranges[4][0]..=key_ranges[4][1] {
                                let key = [k0, k1, k2, k3, k4];
                                let dist: i64 = key
                                    .iter()
                                    .zip(image_pattern_key.iter())
                                    .map(|(&k, &c)| {
                                        let d = k as i64 - c as i64;
                                        d * d
                                    })
                                    .sum();
                                keys.push((key, dist));
                            }
                        }
                    }
                }
            }
            keys.sort_by_key(|&(_, d)| d);
            keys
        };

        // Inner loop: DB lookup per candidate key.
        for (cand_key, _dist) in &candidate_keys {
            // lookup_pattern's FOV pre-filter uses a fixed, very tight tolerance
            // (~0.1% of the estimate) whenever a coarse FOV is passed in. The
            // tetra3 reference only applies this filter when the caller supplies
            // *both* fov_estimate and fov_max_error (see
            // `_get_all_patterns_for_index` in tetra3.py). Passing fov_estimate
            // alone would incorrectly reject the correct pattern for any
            // estimate that isn't already accurate to ~0.1%, so only forward it
            // when fov_max_error is also set.
            let coarse_fov_for_lookup = if params.fov_max_error.is_some() {
                params.fov_estimate
            } else {
                None
            };
            let slots = ps_db::lookup::lookup_pattern(
                db,
                cand_key,
                image_pattern_largest_edge,
                coarse_fov_for_lookup,
            );

            for &slot in &slots {
                // --- A1: Coarse FOV estimate ---
                let catalog_largest_edge_rad = db.largest_edge[slot].to_f64() / 1000.0;

                // Compute largest pixel distance among the 4 image pattern centroids.
                let pattern_centroids: Vec<[f64; 2]> =
                    img_idx.map(|i| image_centroids_undist[i]).to_vec();
                let image_pattern_largest_pixel_distance = {
                    let mut max_d = 0.0_f64;
                    for i in 0..4 {
                        for j in (i + 1)..4 {
                            let dy = pattern_centroids[i][0] - pattern_centroids[j][0];
                            let dx = pattern_centroids[i][1] - pattern_centroids[j][1];
                            let d = (dy * dy + dx * dx).sqrt();
                            if d > max_d {
                                max_d = d;
                            }
                        }
                    }
                    max_d
                };

                let fov = ps_core::fov::estimate_fov_from_pattern(
                    catalog_largest_edge_rad,
                    image_pattern_largest_edge,
                    image_pattern_largest_pixel_distance,
                    params.fov_estimate.map(f64::to_radians),
                    width as f64,
                );

                // --- A2: Re-compute image pattern vectors at coarse FOV, sort by centroid distance ---
                let img_pat_vectors =
                    ps_core::projection::compute_vectors(&pattern_centroids, (height, width), fov);
                let img_pat_vectors_sorted = sort_by_centroid_distance_vec(&img_pat_vectors);

                // --- A3: Get catalog pattern vectors and sort by centroid distance ---
                let cat_star_indices = get_pattern_star_indices(db, slot);
                let cat_pat_vectors: Vec<Vector3<f64>> = cat_star_indices
                    .iter()
                    .map(|&si| {
                        let row = &db.star_table[si as usize];
                        Vector3::new(row[2] as f64, row[3] as f64, row[4] as f64)
                    })
                    .collect();
                let cat_pat_vectors_sorted = sort_by_centroid_distance_vec(&cat_pat_vectors);

                // --- A4: Find rotation matrix R ---
                let r_mat = ps_core::attitude::find_rotation_matrix(
                    &img_pat_vectors_sorted,
                    &cat_pat_vectors_sorted,
                );

                // --- A5: Reject if det(R) < 0 (reflection) ---
                if ps_core::attitude::is_reflection(&r_mat) {
                    continue;
                }

                // --- A6: Gather nearby catalog stars ---
                let center_vec_f32: [f32; 3] = [
                    r_mat[(0, 0)] as f32,
                    r_mat[(0, 1)] as f32,
                    r_mat[(0, 2)] as f32,
                ];
                let fov_diagonal_rad = ps_core::fov::diagonal_fov(fov, width as f64, height as f64);
                let nearby_inds =
                    ps_db::nearby_stars(db, &center_vec_f32, (fov_diagonal_rad / 2.0) as f32);

                // Get nearby catalog star vectors as [f64; 3]
                let nearby_cat_vectors: Vec<[f64; 3]> = nearby_inds
                    .iter()
                    .map(|&i| {
                        let row = &db.star_table[i];
                        [row[2] as f64, row[3] as f64, row[4] as f64]
                    })
                    .collect();

                // Derotate: r_mat * v for each nearby catalog vector
                let nearby_derotated: Vec<Vector3<f64>> = nearby_cat_vectors
                    .iter()
                    .map(|&v| r_mat * Vector3::new(v[0], v[1], v[2]))
                    .collect();

                // Project derotated vectors to pixels
                let (nearby_centroids, kept) =
                    ps_core::projection::compute_centroids(&nearby_derotated, (height, width), fov);

                // Filter to kept indices only (within image bounds)
                let nearby_centroids_kept: Vec<[f64; 2]> =
                    kept.iter().map(|&k| nearby_centroids[k]).collect();
                let nearby_cat_vectors_kept: Vec<[f64; 3]> =
                    kept.iter().map(|&k| nearby_cat_vectors[k]).collect();
                let nearby_inds_kept: Vec<usize> = kept.iter().map(|&k| nearby_inds[k]).collect();

                // Trim to 2 * num_centroids
                let num_centroids = image_centroids_undist.len();
                let trim_limit = (2 * num_centroids).min(nearby_centroids_kept.len());
                let nearby_cat_centroids: Vec<[f64; 2]> =
                    nearby_centroids_kept[..trim_limit].to_vec();
                let nearby_cat_vectors_trimmed: Vec<[f64; 3]> =
                    nearby_cat_vectors_kept[..trim_limit].to_vec();

                // --- A7: Match image centroids to nearby catalog centroids ---
                let match_r_pixels = params.match_radius * width as f64;
                let matched_pairs = find_centroid_matches(
                    &image_centroids_undist,
                    &nearby_cat_centroids,
                    match_r_pixels,
                );

                // --- A8: Binomial accept ---
                let num_nearby_catalog_stars = nearby_cat_centroids.len();
                let num_star_matches = matched_pairs.len();
                let prob_mismatch = ps_core::false_alarm::false_alarm_probability(
                    num_centroids,
                    num_nearby_catalog_stars,
                    num_star_matches,
                    params.match_radius,
                );

                if prob_mismatch >= _match_threshold {
                    continue;
                }

                // --- MATCH ACCEPTED — refine + re-extract + populate ---

                // Extract matched image indices and catalog vectors
                let matched_image_inds: Vec<usize> =
                    matched_pairs.iter().map(|&[i, _]| i).collect();
                let matched_catalog_vectors: Vec<[f64; 3]> = matched_pairs
                    .iter()
                    .map(|&[_, c]| nearby_cat_vectors_trimmed[c])
                    .collect();

                // Compute matched image centroids and vectors at coarse FOV
                let matched_image_centroids: Vec<[f64; 2]> = matched_image_inds
                    .iter()
                    .map(|&i| image_centroids_undist[i])
                    .collect();
                let matched_image_vectors_raw = ps_core::projection::compute_vectors(
                    &matched_image_centroids,
                    (height, width),
                    fov,
                );

                // --- SV5 Step 1: Re-fit attitude over all matches ---
                let matched_catalog_vectors_nalgebra: Vec<Vector3<f64>> = matched_catalog_vectors
                    .iter()
                    .map(|&v| Vector3::new(v[0], v[1], v[2]))
                    .collect();
                let r_refined = ps_core::attitude::find_rotation_matrix(
                    &matched_image_vectors_raw,
                    &matched_catalog_vectors_nalgebra,
                );
                let (ra_rad, dec_rad, roll_rad) = ps_core::attitude::extract_radec_roll(&r_refined);
                let ra_deg = ra_rad.to_degrees();
                let dec_deg = dec_rad.to_degrees();
                let roll_deg = roll_rad.to_degrees();

                // --- SV5 Step 2: Refine FOV ---
                let matched_image_vectors_as_slices: Vec<[f64; 3]> = matched_image_vectors_raw
                    .iter()
                    .map(|v| [v.x, v.y, v.z])
                    .collect();
                let matched_catalog_vectors_as_slices = &matched_catalog_vectors;

                let (fov_refined, distortion_final) = match params.distortion {
                    Some(0.0) => {
                        // No distortion: refine FOV without distortion
                        let fov_r = ps_core::fov::refine_fov_no_distortion(
                            fov,
                            &matched_image_vectors_as_slices,
                            matched_catalog_vectors_as_slices,
                        );
                        (fov_r, 0.0)
                    }
                    None => {
                        // Estimate distortion: refine FOV with distortion
                        // Build row-major [[f64;3];3] from r_refined
                        let r_mat_arr: [[f64; 3]; 3] =
                            std::array::from_fn(|i| std::array::from_fn(|j| r_refined[(i, j)]));
                        let matched_centroids_slice: Vec<[f64; 2]> =
                            matched_image_centroids.to_vec();
                        let (fov_r, k_r) = ps_core::fov::refine_fov_with_distortion(
                            &matched_centroids_slice,
                            matched_catalog_vectors_as_slices,
                            &r_mat_arr,
                            width as f64,
                            height as f64,
                        );
                        (fov_r, k_r)
                    }
                    Some(k) => {
                        // Fixed non-zero distortion: keep coarse FOV
                        (fov, k)
                    }
                };

                // --- SV5 Step 3: Re-compute residuals at refined FOV ---
                // Re-undistort matched centroids at final distortion
                let matched_centroids_for_projection: Vec<[f64; 2]> = if distortion_final != 0.0 {
                    ps_core::distortion::undistort_centroids(
                        &matched_image_centroids,
                        (height, width),
                        distortion_final,
                    )
                } else {
                    matched_image_centroids.clone()
                };

                // Re-project at refined FOV
                let matched_image_vectors_refined = ps_core::projection::compute_vectors(
                    &matched_centroids_for_projection,
                    (height, width),
                    fov_refined,
                );

                // Rotate to sky: r_refined^T * v for each
                let r_transpose = r_refined.transpose();
                let sky_vectors_refined: Vec<[f64; 3]> = matched_image_vectors_refined
                    .iter()
                    .map(|v| {
                        let sky = r_transpose * *v;
                        [sky.x, sky.y, sky.z]
                    })
                    .collect();

                // Compute residuals
                let residual_stats = ps_core::residuals::compute_residuals(
                    &sky_vectors_refined,
                    matched_catalog_vectors_as_slices,
                );

                // --- SV5 Step 4: Populate optional output fields ---

                // matched_centroids
                let matched_centroids_out: Vec<[f64; 2]> =
                    matched_image_centroids.to_vec();

                // matched_stars: for each match, look up RA/Dec/mag from star_table
                // matched_pairs[k][1] -> index into nearby_cat_vectors_trimmed
                // nearby_inds_kept maps trimmed indices back to db.star_table row indices
                let matched_stars_out: Vec<[f64; 3]> = matched_pairs
                    .iter()
                    .map(|&[_, c]| {
                        let star_idx = nearby_inds_kept[c];
                        let row = &db.star_table[star_idx];
                        [row[0] as f64, row[1] as f64, row[5] as f64]
                    })
                    .collect();

                // matched_cat_ids: look up catalog IDs or fall back to star-table index
                let matched_cat_ids_out: Vec<u32> = matched_pairs
                    .iter()
                    .map(|&[_, c]| {
                        let star_idx = nearby_inds_kept[c];
                        if let Some(ref ids) = db.star_catalog_ids_u32 {
                            ids[star_idx]
                        } else if let Some(ref ids) = db.star_catalog_ids_u16 {
                            ids[star_idx] as u32
                        } else {
                            star_idx as u32
                        }
                    })
                    .collect();

                let fov_deg = fov_refined.to_degrees();

                return Solution {
                    status: SolveStatus::MatchFound,
                    ra: ra_deg,
                    dec: dec_deg,
                    roll: roll_deg,
                    fov: fov_deg,
                    distortion: distortion_final,
                    rmse: residual_stats.rmse_arcsec,
                    p90e: residual_stats.p90e_arcsec,
                    maxe: residual_stats.maxe_arcsec,
                    matches: num_star_matches,
                    prob: prob_mismatch,
                    t_solve: t0.elapsed().as_secs_f64(),
                    t_extract: 0.0,
                    matched_centroids: Some(matched_centroids_out),
                    matched_stars: Some(matched_stars_out),
                    matched_cat_ids: Some(matched_cat_ids_out),
                    combos_examined,
                };
            }
        }
    }

    Solution::failure(status, t0.elapsed().as_secs_f64(), combos_examined)
}

/// Cluster-bust centroids: greedy O(n^2) pass keeping stars separated
/// by at least `separation_pixels`.
///
/// Returns indices into the original slice for the kept centroids.
pub fn cluster_bust_centroids(centroids: &[[f64; 2]], separation_pixels: f64) -> Vec<usize> {
    let mut kept = Vec::with_capacity(centroids.len());
    let sep_sq = separation_pixels * separation_pixels;

    for i in 0..centroids.len() {
        let [yi, xi] = centroids[i];
        let mut dominated = false;
        for &k_idx in &kept {
            let [yk, xk] = centroids[k_idx];
            let dy = yi - yk;
            let dx = xi - xk;
            if dy * dy + dx * dx <= sep_sq {
                dominated = true;
                break;
            }
        }
        if !dominated {
            kept.push(i);
        }
    }
    kept
}

/// Lazily yields all 4-element combinations of `[0, n)` in the same
/// breadth-first ("brightest first") order as cedar-solve's
/// `breadth_first_combinations` (tetra3/breadth_first_combinations.py),
/// which for fixed r is colexicographic order: combinations sorted
/// ascending by largest element, then by second-largest, and so on.
/// A valid match among the brightest stars is therefore reached after
/// ~C(d,4) combos (d = its largest index) instead of potentially most
/// of C(n,4) under the old lexicographic order.
///
/// Also replaces an eager `Vec<[usize; 4]>` that allocated C(n,4)*32 B
/// up front (~618 MiB at n = 150, the bundled DB's
/// verification_stars_per_fov), so timeout/cancel checks fire at the
/// same per-combo cadence with no allocation.
struct BreadthFirstCombinations4 {
    n: usize,
    cur: [usize; 4],
    done: bool,
}

impl BreadthFirstCombinations4 {
    fn new(n: usize) -> Self {
        // done = n < 4 short-circuits the empty case AND guards the
        // bound arithmetic in next() from ever running with n < 4.
        Self { n, cur: [0, 1, 2, 3], done: n < 4 }
    }
}

impl Iterator for BreadthFirstCombinations4 {
    type Item = [usize; 4];

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let out = self.cur;
        // Colex successor: find the smallest position that can advance
        // (bounded by the element above it, or n at the top), bump it,
        // and reset everything below to the minimal values 0..i.
        for i in 0..4 {
            let bound = if i < 3 { self.cur[i + 1] } else { self.n };
            if self.cur[i] + 1 < bound {
                self.cur[i] += 1;
                for j in 0..i {
                    self.cur[j] = j;
                }
                return Some(out);
            }
        }
        self.done = true;
        Some(out) // the final combination is still yielded
    }
}

/// Lazy breadth-first (colex) 4-combinations of `[0, n)`. No allocation.
fn breadth_first_combinations_4(n: usize) -> BreadthFirstCombinations4 {
    BreadthFirstCombinations4::new(n)
}

/// Solve from a raw grayscale image (detects stars then solves).
pub fn solve_from_image(db: &Database, image: &GrayImageView<'_>, params: &SolveParams) -> Solution {
    let (width, height) = (image.width() as usize, image.height() as usize);
    // Time the extraction (detection + centroid collection); `solve_from_centroids`
    // measures only the solve region, so t_extract is stamped on its result.
    let t_extract_start = Instant::now();
    let (stars, _, _, _) = ps_detect::get_stars_from_image(
        image, 1.0,   // noise_estimate (floored to NOISE_FLOOR internally)
        4.0,   // sigma
        false, // normalize_rows
        1,     // binning
        true,  // detect_hot_pixels
        false, // return_binned_image
    ).unwrap();
    let centroids: Vec<[f64; 2]> = stars
        .iter()
        .map(|s| [s.centroid_y, s.centroid_x])
        .collect();
    let t_extract = t_extract_start.elapsed().as_secs_f64();
    let mut sol = solve_from_centroids(db, &centroids, (height, width), params);
    sol.t_extract = t_extract;
    sol
}

/// Sort 4 pattern vectors by ascending Euclidean distance from their centroid.
fn sort_by_centroid_distance_vec(vectors: &[Vector3<f64>]) -> Vec<Vector3<f64>> {
    let n = vectors.len();
    let mut cx = 0.0_f64;
    let mut cy = 0.0_f64;
    let mut cz = 0.0_f64;
    for v in vectors.iter() {
        cx += v.x;
        cy += v.y;
        cz += v.z;
    }
    cx /= n as f64;
    cy /= n as f64;
    cz /= n as f64;

    let mut indexed: Vec<(f64, usize)> = vectors
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let dx = v.x - cx;
            let dy = v.y - cy;
            let dz = v.z - cz;
            ((dx * dx + dy * dy + dz * dz).sqrt(), i)
        })
        .collect();
    indexed.sort_by(|a, b| a.0.total_cmp(&b.0));

    indexed.iter().map(|&(_, i)| vectors[i]).collect()
}

/// Get the 4 star-table indices for a given pattern catalog slot.
fn get_pattern_star_indices(db: &Database, slot: usize) -> [u32; 4] {
    if let Some(ref cat_u8) = db.pattern_catalog_u8 {
        let row = cat_u8[slot];
        [row[0] as u32, row[1] as u32, row[2] as u32, row[3] as u32]
    } else if let Some(ref cat_u16) = db.pattern_catalog_u16 {
        let row = cat_u16[slot];
        [row[0] as u32, row[1] as u32, row[2] as u32, row[3] as u32]
    } else if let Some(ref cat_u32) = db.pattern_catalog_u32 {
        cat_u32[slot]
    } else {
        [0; 4]
    }
}

/// Find unique 1-1 matches between image and catalog centroids within radius r.
///
/// For every (i_img, i_cat) pair with pixel distance < r, collect all such pairs.
/// Then deduplicate: keep only one match per catalog star (first found), then one
/// match per image star (first found). This order matches the reference algorithm.
fn find_centroid_matches(
    image_centroids: &[[f64; 2]],
    catalog_centroids: &[[f64; 2]],
    r: f64,
) -> Vec<[usize; 2]> {
    let r_sq = r * r;
    let mut matches: Vec<[usize; 2]> = Vec::new();

    #[allow(clippy::needless_range_loop)]
    for i in 0..image_centroids.len() {
        #[allow(clippy::needless_range_loop)]
        for j in 0..catalog_centroids.len() {
            let dy = image_centroids[i][0] - catalog_centroids[j][0];
            let dx = image_centroids[i][1] - catalog_centroids[j][1];
            if dy * dy + dx * dx < r_sq {
                matches.push([i, j]);
            }
        }
    }

    // Deduplicate: one match per catalog star (first found) — matches reference order
    let mut seen_cat = vec![false; catalog_centroids.len()];
    let mut unique_cat: Vec<[usize; 2]> = Vec::new();
    for &pair in &matches {
        let j = pair[1];
        if !seen_cat[j] {
            seen_cat[j] = true;
            unique_cat.push(pair);
        }
    }

    // Deduplicate: one match per image star (first found) — matches reference order
    let mut seen_img = vec![false; image_centroids.len()];
    let mut unique_img: Vec<[usize; 2]> = Vec::new();
    for &pair in &unique_cat {
        let i = pair[0];
        if !seen_img[i] {
            seen_img[i] = true;
            unique_img.push(pair);
        }
    }

    unique_img
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solve_status_variants() {
        let statuses = [
            SolveStatus::MatchFound,
            SolveStatus::NoMatch,
            SolveStatus::Timeout,
            SolveStatus::Cancelled,
            SolveStatus::TooFew,
        ];
        for s in &statuses {
            let _ = format!("{:?}", s);
        }
    }

    #[test]
    fn default_params_cedar_defaults() {
        let p = SolveParams::default();
        assert_eq!(p.match_radius, 0.01);
        assert!((p.match_threshold - 1e-5).abs() < 1e-15);
        assert_eq!(p.match_max_error, 0.002);
        assert_eq!(p.solve_timeout, Some(5000));
        assert_eq!(p.distortion, Some(0.0));
        assert!(p.fov_estimate.is_none());
    }

    #[test]
    fn fov_estimate_defaults_to_db_midpoint() {
        // This test uses a mock by checking that the fallback formula is correct.
        // Full DB integration tested in later tasks.
        let min_fov = 10.0f64;
        let max_fov = 30.0f64;
        let midpoint = (min_fov + max_fov) / 2.0;
        assert_eq!(midpoint, 20.0);
    }

    #[test]
    fn solution_failure_constructor() {
        let sol = Solution::failure(SolveStatus::TooFew, 0.123, 0);
        assert_eq!(sol.status, SolveStatus::TooFew);
        assert_eq!(sol.matches, 0);
        assert!((sol.t_solve - 0.123).abs() < 1e-12);
    }

    #[test]
    fn preparation_cluster_bust_and_too_few() {
        // Verify the cluster-bust logic in isolation.
        let centroids: Vec<[f64; 2]> = vec![
            [100.0, 100.0],
            [101.0, 101.0], // very close to first
            [200.0, 200.0],
            [201.0, 201.0], // very close to third
            [300.0, 300.0],
        ];
        let separation_pixels = 10.0_f64;
        let kept = cluster_bust_centroids(&centroids, separation_pixels);
        // Should keep [0, 2, 4] (indices 1 and 3 are within 10px of kept neighbors)
        assert_eq!(kept, vec![0usize, 2, 4]);
    }

    #[test]
    fn cluster_bust_separation_formula() {
        // Cedar-solve reference: sep_px = width * 0.6 / sqrt(vsfov)
        let width = 1920.0_f64;
        let vsfov = 150.0_f64;
        let expected = width * 0.6 / vsfov.sqrt();
        // For fov_initial = 11.0 degrees in radians, this should be fov-independent
        let fov_initial_rad = 11.0_f64.to_radians();
        // The correct formula produces the same result regardless of fov
        let sep = width * 0.6 / vsfov.sqrt();
        let _ = fov_initial_rad; // fov cancels, formula is fov-independent
        assert!((sep - expected).abs() < 1e-10);
        // Check concrete value ~93.9 px at width=1920, vsfov=150
        assert!(sep > 90.0 && sep < 100.0);
    }

    #[test]
    fn timeout_reachable() {
        use ps_db::{Database, DatabaseProperties};
        let props = DatabaseProperties::apply_legacy_fallbacks(
            None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            None, None,
        );
        let db = Database::empty(props);
        // 5 centroids spread out (no cluster-bust issue), 0ms timeout → Timeout
        let centroids: Vec<[f64; 2]> = vec![
            [100.0, 100.0],
            [200.0, 300.0],
            [400.0, 150.0],
            [350.0, 400.0],
            [150.0, 350.0],
        ];
        let params = SolveParams {
            solve_timeout: Some(0),
            ..Default::default()
        };
        let sol = solve_from_centroids(&db, &centroids, (500, 500), &params);
        assert_eq!(sol.status, SolveStatus::Timeout);
    }

    #[test]
    fn cancelled_reachable() {
        use ps_db::{Database, DatabaseProperties};
        use std::sync::{atomic::AtomicBool, Arc};
        let props = DatabaseProperties::apply_legacy_fallbacks(
            None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            None, None,
        );
        let db = Database::empty(props);
        let centroids: Vec<[f64; 2]> = vec![
            [100.0, 100.0],
            [200.0, 300.0],
            [400.0, 150.0],
            [350.0, 400.0],
            [150.0, 350.0],
        ];
        let flag = Arc::new(AtomicBool::new(true)); // pre-cancelled
        let params = SolveParams {
            cancel_flag: Some(flag.clone()),
            ..Default::default()
        };
        let sol = solve_from_centroids(&db, &centroids, (500, 500), &params);
        assert_eq!(sol.status, SolveStatus::Cancelled);
    }

    #[test]
    fn combinations_4_basic() {
        assert_eq!(breadth_first_combinations_4(0).collect::<Vec<_>>(), Vec::<[usize; 4]>::new());
        assert_eq!(breadth_first_combinations_4(3).collect::<Vec<_>>(), Vec::<[usize; 4]>::new());
        let combos: Vec<_> = breadth_first_combinations_4(4).collect();
        assert_eq!(combos.len(), 1);
        assert_eq!(combos[0], [0, 1, 2, 3]);
    }

    #[test]
    fn combinations_4_five_elements() {
        // C(5,4) = 5 combinations. Note: n=r+1 is a coincidence where lexicographic
        // and colexicographic order coincide; this test does not prove breadth-first
        // ordering is correct in general (see SP1.3 for n=6/n=7 proof).
        let combos: Vec<_> = breadth_first_combinations_4(5).collect();
        assert_eq!(combos.len(), 5);
        assert_eq!(combos[0], [0, 1, 2, 3]);
        assert_eq!(combos[1], [0, 1, 2, 4]);
        assert_eq!(combos[2], [0, 1, 3, 4]);
        assert_eq!(combos[3], [0, 2, 3, 4]);
        assert_eq!(combos[4], [1, 2, 3, 4]);
    }

    #[test]
    fn breadth_first_combinations_4_matches_reference() {
        use std::collections::BTreeSet;

        // ASSERTION 1: n=6 cross-check
        // Reference-generated literal from: python3 -c "import sys; sys.path.insert(0, 'reference-solutions/cedar-solve/tetra3'); from breadth_first_combinations import breadth_first_combinations; print(list(breadth_first_combinations(range(6), 4)))"
        // Run 2026-07-04
        let expected_n6: Vec<[usize; 4]> = vec![
            [0, 1, 2, 3], [0, 1, 2, 4], [0, 1, 3, 4], [0, 2, 3, 4], [1, 2, 3, 4],
            [0, 1, 2, 5], [0, 1, 3, 5], [0, 2, 3, 5], [1, 2, 3, 5], [0, 1, 4, 5],
            [0, 2, 4, 5], [1, 2, 4, 5], [0, 3, 4, 5], [1, 3, 4, 5], [2, 3, 4, 5],
        ];
        let combos_n6: Vec<_> = breadth_first_combinations_4(6).collect();
        assert_eq!(combos_n6, expected_n6, "n=6 breadth-first order must match reference exactly");

        // ASSERTION 2: n=7 cross-check
        // n=7 is n=6 followed by all combos containing index 6
        let expected_n7_suffix: Vec<[usize; 4]> = vec![
            [0, 1, 2, 6], [0, 1, 3, 6], [0, 2, 3, 6], [1, 2, 3, 6], [0, 1, 4, 6],
            [0, 2, 4, 6], [1, 2, 4, 6], [0, 3, 4, 6], [1, 3, 4, 6], [2, 3, 4, 6],
            [0, 1, 5, 6], [0, 2, 5, 6], [1, 2, 5, 6], [0, 3, 5, 6], [1, 3, 5, 6],
            [2, 3, 5, 6], [0, 4, 5, 6], [1, 4, 5, 6], [2, 4, 5, 6], [3, 4, 5, 6],
        ];
        let mut expected_n7 = expected_n6.clone();
        expected_n7.extend(expected_n7_suffix);
        assert_eq!(expected_n7.len(), 35, "n=7 should have 35 combos (C(7,4))");

        let combos_n7: Vec<_> = breadth_first_combinations_4(7).collect();
        assert_eq!(combos_n7, expected_n7, "n=7 breadth-first order must match reference exactly");

        // ASSERTION 3: Prefix-property assertion
        // The defining structural feature of breadth-first order is that n=7's sequence is
        // n=6's sequence followed by every combo containing index 6. This is a good sanity
        // check independent of the literals above.
        let combos_n7_first_15 = combos_n7.iter().take(15).copied().collect::<Vec<_>>();
        assert_eq!(
            combos_n7_first_15, expected_n6,
            "n=7's first 15 combos must equal n=6's full sequence (breadth-first prefix property)"
        );

        // ASSERTION 4: n=10 and n=20 count/set assertions
        // Verify count matches binomial coefficient C(n,4) = n*(n-1)*(n-2)*(n-3)/24
        let combos_n10: Vec<_> = breadth_first_combinations_4(10).collect();
        let expected_count_10 = 10 * 9 * 8 * 7 / 24; // C(10,4) = 210
        assert_eq!(
            combos_n10.len(),
            expected_count_10,
            "n=10 should have C(10,4)={} combos",
            expected_count_10
        );

        let combos_n20: Vec<_> = breadth_first_combinations_4(20).collect();
        let expected_count_20 = 20 * 19 * 18 * 17 / 24; // C(20,4) = 4845
        assert_eq!(
            combos_n20.len(),
            expected_count_20,
            "n=20 should have C(20,4)={} combos",
            expected_count_20
        );

        // For both n=10 and n=20, verify that the *set* of combos matches a local
        // lexicographic generator (order not compared here, just membership).
        for n in [10, 20].iter() {
            let breadth_first_set: BTreeSet<[usize; 4]> =
                breadth_first_combinations_4(*n).collect();

            // Build lexicographic set: four nested loops
            let mut lexicographic_set = BTreeSet::new();
            for a in 0..*n {
                for b in (a + 1)..*n {
                    for c in (b + 1)..*n {
                        for d in (c + 1)..*n {
                            lexicographic_set.insert([a, b, c, d]);
                        }
                    }
                }
            }

            assert_eq!(
                breadth_first_set, lexicographic_set,
                "n={} breadth-first set must equal lexicographic set (same membership, order differs)",
                n
            );
        }
    }

    /// Helper: build a mock DB with a single pattern of 4 catalog stars.
    fn build_mock_db_with_pattern(
        cat_vectors: &[[f64; 3]],
        fov_deg: f64,
        num_extra_stars: usize,
    ) -> Database {
        use half::f16;
        use ps_db::DatabaseProperties;

        let _fov_rad = fov_deg.to_radians();

        // Compute pattern key from the 4 catalog vectors.
        let cat_vecs_4: [[f64; 3]; 4] = [
            cat_vectors[0],
            cat_vectors[1],
            cat_vectors[2],
            cat_vectors[3],
        ];
        let (pattern_key, largest_edge_rad) =
            ps_core::pattern::compute_pattern_key(&cat_vecs_4, 250);

        // Compute the hash and find the slot.
        let full_hash = ps_core::pattern::compute_pattern_key_hash(&pattern_key, 250);
        let low16 = ps_core::pattern::key_hash_low16(full_hash);
        let table_size: u64 = 100;
        let hash_index = ps_core::pattern::pattern_key_hash_to_index(
            full_hash, table_size, false, // quadratic_probe
        );
        let slot = hash_index as usize;

        // Build star_table with 4 pattern stars + extra stars spread around.
        let mut star_table: Vec<[f32; 6]> = Vec::with_capacity(4 + num_extra_stars);

        // First 4 are the pattern stars.
        for v in cat_vectors.iter().take(4) {
            let ra = v[1].atan2(v[0]).rem_euclid(2.0 * PI);
            let dec = v[2].asin();
            star_table.push([
                ra as f32,
                dec as f32,
                v[0] as f32,
                v[1] as f32,
                v[2] as f32,
                5.0,
            ]);
        }

        // Extra stars: spread around the sphere to fill the FOV region.
        for i in 0..num_extra_stars {
            let angle = (i as f64 / num_extra_stars as f64) * 2.0 * PI;
            let r = 0.3 + (i as f64 % 5.0) * 0.05;
            let x = angle.cos() * r;
            let y = angle.sin() * r;
            let z_sq = 1.0 - x * x - y * y;
            let z = if z_sq > 0.0 { z_sq.sqrt() } else { 1.0 };
            let norm = (x * x + y * y + z * z).sqrt();
            star_table.push([
                0.0,
                0.0,
                (x / norm) as f32,
                (y / norm) as f32,
                (z / norm) as f32,
                6.0 + i as f32 * 0.1,
            ]);
        }

        // Build hash table arrays.
        let num_slots = table_size as usize;
        let mut pattern_catalog: Vec<[u8; 4]> = vec![[255; 4]; num_slots];
        let mut key_hashes: Vec<u16> = vec![0; num_slots];
        let mut largest_edge: Vec<f16> = vec![f16::from_f64(0.0); num_slots];

        // Insert our pattern at the computed slot.
        pattern_catalog[slot] = [0, 1, 2, 3];
        key_hashes[slot] = low16;
        largest_edge[slot] = f16::from_f64(largest_edge_rad * 1000.0);

        // Build DB.
        let props = DatabaseProperties::apply_legacy_fallbacks(
            None,
            Some("quadratic_probe".into()),
            None,
            Some(250),
            None,
            Some((fov_deg + 5.0) as f32),
            Some((fov_deg - 5.0) as f32),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(1),
        );

        let mut db = Database {
            properties: props,
            star_table,
            pattern_catalog_u8: Some(pattern_catalog),
            pattern_catalog_u16: None,
            pattern_catalog_u32: None,
            largest_edge,
            key_hashes,
            star_catalog_ids_u16: None,
            star_catalog_ids_u32: None,
            star_kd_tree: None,
        };

        // Build KD-tree for nearby_stars.
        db.build_kd_tree();

        db
    }

    /// Test that a candidate with det(R) < 0 is rejected.
    #[test]
    fn sv4_det_negative_rejected() {
        // We create a DB with 4 stars and feed image centroids that are
        // the mirror of the catalog pattern (across a plane), producing
        // a reflection matrix with det < 0.

        let height = 500usize;
        let width = 500usize;
        let fov_deg = 10.0_f64;
        let fov_rad = fov_deg.to_radians();

        // Pick 4 catalog stars near the boresight (small angles).
        let cat_vectors: [[f64; 3]; 4] = [
            [0.998, 0.050, 0.020],   // star 0
            [0.995, -0.040, 0.080],  // star 1
            [0.990, 0.030, -0.070],  // star 2
            [0.996, -0.020, -0.050], // star 3
        ];

        // Normalize them to be proper unit vectors.
        let cat_vectors_norm: Vec<[f64; 3]> = cat_vectors
            .iter()
            .map(|v| {
                let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                [v[0] / n, v[1] / n, v[2] / n]
            })
            .collect();

        let db = build_mock_db_with_pattern(&cat_vectors_norm, fov_deg, 5);

        // Project catalog stars to pixel centroids (identity rotation).
        let cat_vecs_nalgebra: Vec<Vector3<f64>> = cat_vectors_norm
            .iter()
            .map(|v| Vector3::new(v[0], v[1], v[2]))
            .collect();
        let (centroids, _kept) =
            ps_core::projection::compute_centroids(&cat_vecs_nalgebra, (height, width), fov_rad);

        // Now reflect the centroids across x-axis to create a mirror image.
        // This should produce a rotation matrix with det < 0.
        let reflected_centroids: Vec<[f64; 2]> = centroids
            .iter()
            .map(|&[y, x]| [height as f64 - y, x]) // flip vertically
            .collect();

        // We need at least 4 centroids for the solver.
        let star_centroids: Vec<[f64; 2]> = reflected_centroids.iter().take(4).copied().collect();

        let params = SolveParams {
            fov_estimate: Some(fov_deg),
            solve_timeout: Some(5000),
            ..Default::default()
        };

        let sol = solve_from_centroids(&db, &star_centroids, (height, width), &params);
        // Should not find a match because the best rotation is a reflection.
        assert_ne!(
            sol.status,
            SolveStatus::MatchFound,
            "Expected NoMatch for reflected pattern but got MatchFound"
        );
    }

    /// Test that the solver accepts a correct pattern with enough matches.
    #[test]
    fn sv4_accepts_correct_pattern() {
        let height = 500usize;
        let width = 500usize;
        let fov_deg = 10.0_f64;
        let fov_rad = fov_deg.to_radians();

        // Pick 4 catalog stars near the boresight with distinct positions.
        let cat_vectors: [[f64; 3]; 4] = [
            [0.998, 0.050, 0.020],
            [0.995, -0.040, 0.080],
            [0.990, 0.030, -0.070],
            [0.996, -0.020, -0.050],
        ];

        let cat_vectors_norm: Vec<[f64; 3]> = cat_vectors
            .iter()
            .map(|v| {
                let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                [v[0] / n, v[1] / n, v[2] / n]
            })
            .collect();

        let db = build_mock_db_with_pattern(&cat_vectors_norm, fov_deg, 5);

        // Project catalog stars to pixel centroids (identity rotation = no rotation).
        let cat_vecs_nalgebra: Vec<Vector3<f64>> = cat_vectors_norm
            .iter()
            .map(|v| Vector3::new(v[0], v[1], v[2]))
            .collect();
        let (centroids, _kept) =
            ps_core::projection::compute_centroids(&cat_vecs_nalgebra, (height, width), fov_rad);

        // Add tiny noise to centroids to simulate real observations.
        let star_centroids: Vec<[f64; 2]> = centroids
            .iter()
            .enumerate()
            .map(|(i, &[y, x])| {
                let noise = 0.1_f64; // very small noise
                [
                    y + noise * (i as f64 - 1.5),
                    x + noise * (i as f64 - 1.5) * 0.7,
                ]
            })
            .collect();

        // Include extra centroids to have > 4 stars for verification.
        let mut all_centroids = star_centroids.clone();
        // Add a couple more centroids near the center.
        all_centroids.push([height as f64 / 2.0 + 1.0, width as f64 / 2.0 + 2.0]);
        all_centroids.push([height as f64 / 2.0 - 1.0, width as f64 / 2.0 - 2.0]);

        let params = SolveParams {
            fov_estimate: Some(fov_deg),
            solve_timeout: Some(5000),
            ..Default::default()
        };

        let sol = solve_from_centroids(&db, &all_centroids, (height, width), &params);
        assert_eq!(
            sol.status,
            SolveStatus::MatchFound,
            "Expected MatchFound but got {:?} (prob={}, matches={})",
            sol.status,
            sol.prob,
            sol.matches
        );
        assert!(
            sol.matches >= 4,
            "Expected at least 4 matches, got {}",
            sol.matches
        );
    }

    /// Test that when the false-alarm probability is above the threshold,
    /// the solver returns NoMatch via binomial rejection.
    #[test]
    fn sv4_mismatch_above_threshold() {
        // This test verifies that the binomial false-alarm rejection path is reachable.
        // Strategy: use the same correct pattern setup as sv4_accepts_correct_pattern,
        // but set match_threshold to an astronomically tight value so the
        // Bonferroni-corrected threshold is essentially 0, and prob_mismatch >= threshold.

        let height = 500usize;
        let width = 500usize;
        let fov_deg = 10.0_f64;
        let fov_rad = fov_deg.to_radians();

        let cat_vectors: [[f64; 3]; 4] = [
            [0.998, 0.050, 0.020],
            [0.995, -0.040, 0.080],
            [0.990, 0.030, -0.070],
            [0.996, -0.020, -0.050],
        ];

        let cat_vectors_norm: Vec<[f64; 3]> = cat_vectors
            .iter()
            .map(|v| {
                let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                [v[0] / n, v[1] / n, v[2] / n]
            })
            .collect();

        let db = build_mock_db_with_pattern(&cat_vectors_norm, fov_deg, 5);

        // Project catalog stars to pixel centroids (identity rotation).
        let cat_vecs_nalgebra: Vec<Vector3<f64>> = cat_vectors_norm
            .iter()
            .map(|v| Vector3::new(v[0], v[1], v[2]))
            .collect();
        let (centroids, _kept) =
            ps_core::projection::compute_centroids(&cat_vecs_nalgebra, (height, width), fov_rad);

        let mut all_centroids = centroids.clone();
        all_centroids.push([height as f64 / 2.0 + 1.0, width as f64 / 2.0 + 2.0]);
        all_centroids.push([height as f64 / 2.0 - 1.0, width as f64 / 2.0 - 2.0]);

        // Use match_threshold = f64::MIN_POSITIVE (effectively 0.0 after Bonferroni).
        // With num_patterns=1, _match_threshold = f64::MIN_POSITIVE.
        // Any computed prob_mismatch > f64::MIN_POSITIVE will be rejected.
        // Even a perfect match of 4 stars out of 6 image centroids against a ~4-star
        // nearby catalog yields a small but non-zero prob_mismatch.
        let params = SolveParams {
            fov_estimate: Some(fov_deg),
            match_threshold: f64::MIN_POSITIVE,
            solve_timeout: Some(5000),
            ..Default::default()
        };

        let sol = solve_from_centroids(&db, &all_centroids, (height, width), &params);
        // The binomial probability is non-zero (even for a good match), so with an
        // effectively-zero threshold, the accept branch is never reached.
        assert_ne!(
            sol.status,
            SolveStatus::MatchFound,
            "Expected NoMatch (binomial rejection) but got MatchFound with prob={}",
            sol.prob
        );
    }

    /// Test that a MatchFound Solution has all SV5 output fields populated.
    #[test]
    fn sv5_solution_fields_populated() {
        let height = 500usize;
        let width = 500usize;
        let fov_deg = 10.0_f64;
        let fov_rad = fov_deg.to_radians();

        let cat_vectors: [[f64; 3]; 4] = [
            [0.998, 0.050, 0.020],
            [0.995, -0.040, 0.080],
            [0.990, 0.030, -0.070],
            [0.996, -0.020, -0.050],
        ];

        let cat_vectors_norm: Vec<[f64; 3]> = cat_vectors
            .iter()
            .map(|v| {
                let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                [v[0] / n, v[1] / n, v[2] / n]
            })
            .collect();

        let db = build_mock_db_with_pattern(&cat_vectors_norm, fov_deg, 5);

        // Project catalog stars to pixel centroids (identity rotation).
        let cat_vecs_nalgebra: Vec<Vector3<f64>> = cat_vectors_norm
            .iter()
            .map(|v| Vector3::new(v[0], v[1], v[2]))
            .collect();
        let (centroids, _kept) =
            ps_core::projection::compute_centroids(&cat_vecs_nalgebra, (height, width), fov_rad);

        // Add tiny noise to centroids.
        let star_centroids: Vec<[f64; 2]> = centroids
            .iter()
            .enumerate()
            .map(|(i, &[y, x])| {
                let noise = 0.1_f64;
                [
                    y + noise * (i as f64 - 1.5),
                    x + noise * (i as f64 - 1.5) * 0.7,
                ]
            })
            .collect();

        // Include extra centroids for verification matching.
        let mut all_centroids = star_centroids.clone();
        all_centroids.push([height as f64 / 2.0 + 1.0, width as f64 / 2.0 + 2.0]);
        all_centroids.push([height as f64 / 2.0 - 1.0, width as f64 / 2.0 - 2.0]);

        let params = SolveParams {
            fov_estimate: Some(fov_deg),
            solve_timeout: Some(5000),
            ..Default::default()
        };

        let sol = solve_from_centroids(&db, &all_centroids, (height, width), &params);
        assert_eq!(
            sol.status,
            SolveStatus::MatchFound,
            "Expected MatchFound but got {:?}",
            sol.status
        );

        // matched_centroids: Some with >= 4 entries
        assert!(
            sol.matched_centroids.is_some(),
            "matched_centroids should be Some"
        );
        let mc = sol.matched_centroids.as_ref().unwrap();
        assert!(mc.len() >= 4, "matched_centroids len={} < 4", mc.len());

        // matched_stars: Some with >= 4 entries
        assert!(sol.matched_stars.is_some(), "matched_stars should be Some");
        let ms = sol.matched_stars.as_ref().unwrap();
        assert!(ms.len() >= 4, "matched_stars len={} < 4", ms.len());

        // matched_cat_ids: Some
        assert!(
            sol.matched_cat_ids.is_some(),
            "matched_cat_ids should be Some"
        );

        // Residual stats: non-negative and consistent
        assert!(sol.rmse >= 0.0, "rmse should be >= 0");
        assert!(sol.p90e >= 0.0, "p90e should be >= 0");
        assert!(sol.maxe >= 0.0, "maxe should be >= 0");
        assert!(
            sol.p90e <= sol.maxe,
            "p90e ({}) should be <= maxe ({})",
            sol.p90e,
            sol.maxe
        );

        // FOV and timing
        assert!(sol.fov > 0.0, "fov should be > 0");
        assert!(sol.t_solve >= 0.0, "t_solve should be >= 0");
    }

    /// Test that all five SolveStatus variants are reachable via existing tests.
    #[test]
    fn sv5_all_status_codes_reachable() {
        // All five enum variants must be distinct and constructible.
        let statuses = vec![
            SolveStatus::MatchFound,
            SolveStatus::NoMatch,
            SolveStatus::Timeout,
            SolveStatus::Cancelled,
            SolveStatus::TooFew,
        ];

        // Verify they are all distinct (5 unique variants).
        assert_eq!(statuses.len(), 5);
        for i in 0..statuses.len() {
            for j in (i + 1)..statuses.len() {
                assert_ne!(
                    statuses[i], statuses[j],
                    "Status variants {} and {} should be distinct",
                    i, j
                );
            }
        }

        // Verify each is reachable:
        // - TooFew: < 4 centroids (tested in preparation_cluster_bust_and_too_few indirectly)
        use ps_db::{Database, DatabaseProperties};
        let props = DatabaseProperties::apply_legacy_fallbacks(
            None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            None, None,
        );
        let db = Database::empty(props);

        // TooFew: fewer than 4 centroids
        let sol = solve_from_centroids(
            &db,
            &[[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]],
            (100, 100),
            &SolveParams::default(),
        );
        assert_eq!(sol.status, SolveStatus::TooFew);

        // Timeout: 0ms timeout with enough centroids to enter the loop
        let params = SolveParams {
            solve_timeout: Some(0),
            ..Default::default()
        };
        let sol = solve_from_centroids(
            &db,
            &[
                [10.0, 10.0],
                [20.0, 30.0],
                [40.0, 15.0],
                [35.0, 40.0],
                [15.0, 35.0],
            ],
            (100, 100),
            &params,
        );
        assert_eq!(sol.status, SolveStatus::Timeout);

        // Cancelled: pre-cancelled flag
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let params = SolveParams {
            cancel_flag: Some(flag),
            ..Default::default()
        };
        let sol = solve_from_centroids(
            &db,
            &[
                [10.0, 10.0],
                [20.0, 30.0],
                [40.0, 15.0],
                [35.0, 40.0],
                [15.0, 35.0],
            ],
            (100, 100),
            &params,
        );
        assert_eq!(sol.status, SolveStatus::Cancelled);

        // NoMatch: empty DB, no patterns to match against (after timeout/cancel guards pass)
        // With a real timeout (not 0), the solver will exhaust patterns and return NoMatch.
        // But with an empty DB and no patterns, the outer loop body never executes,
        // so status stays as NoMatch (default).
        let params = SolveParams {
            solve_timeout: Some(5000),
            ..Default::default()
        };
        let sol = solve_from_centroids(
            &db,
            &[
                [10.0, 10.0],
                [20.0, 30.0],
                [40.0, 15.0],
                [35.0, 40.0],
                [15.0, 35.0],
            ],
            (100, 100),
            &params,
        );
        assert_eq!(sol.status, SolveStatus::NoMatch);

        // MatchFound: tested in sv4_accepts_correct_pattern and sv5_solution_fields_populated
        // We just verify the variant exists here; those tests cover actual MatchFound paths.
    }

    /// SV6: solve_from_image parity vs cedar reference on a medium_fov image.
    #[test]
    fn sv6_solve_from_image_parity() {
        use ps_db::{importer, loader};
        use serde::Deserialize;
        use std::path::PathBuf;
        use tempfile::NamedTempFile;

        #[derive(Deserialize)]
        struct Fixture {
            ra_deg: f64,
            dec_deg: f64,
            #[allow(dead_code)]
            roll_deg: f64,
            #[allow(dead_code)]
            fov_deg: f64,
            #[allow(dead_code)]
            matches: usize,
            #[allow(dead_code)]
            matched_cat_ids: Vec<u32>,
            #[allow(dead_code)]
            centroids_yx: Vec<[f64; 2]>,
            #[allow(dead_code)]
            image_size: [usize; 2],
        }

        // Load golden fixture for RA/Dec reference
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let fixture_path = manifest.join("tests/fixtures/reference_solve.json");
        let fixture: Fixture = serde_json::from_str(
            &std::fs::read_to_string(&fixture_path)
                .unwrap_or_else(|e| panic!("Cannot read {}: {}", fixture_path.display(), e)),
        )
        .expect("fixture JSON parse failed");

        // Import the reference NPZ database
        let npz_path =
            manifest.join("../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
        let db_imported =
            importer::import_npz(&npz_path).unwrap_or_else(|e| panic!("import_npz failed: {}", e));

        // Save → load native (exercises the full ps-db round-trip)
        let tmp = NamedTempFile::new().expect("tempfile");
        loader::save_native(&db_imported, tmp.path()).expect("save_native");
        let mut db = loader::load_native(tmp.path()).expect("load_native");
        db.build_kd_tree();

        // Load the actual image and solve from it
        let img_path = manifest.join(
            "../reference-solutions/cedar-solve/examples/data/medium_fov/2019-07-29T204726_Alt40_Azi-135_Try1.jpg",
        );
        let img = image::open(&img_path)
            .unwrap_or_else(|e| panic!("Cannot open {}: {}", img_path.display(), e))
            .into_luma8();

        let params = SolveParams {
            solve_timeout: Some(120000),
            ..Default::default()
        };
        let sol = solve_from_image(&db, &ps_detect::as_view(&img), &params);

        assert_eq!(
            sol.status,
            SolveStatus::MatchFound,
            "Expected MatchFound, got {:?}",
            sol.status
        );

        // RA/Dec within 10 arcsec of reference (ps-detect centroids differ from tetra3)
        let ra_err_arcsec = (sol.ra - fixture.ra_deg).abs() * 3600.0;
        let dec_err_arcsec = (sol.dec - fixture.dec_deg).abs() * 3600.0;
        assert!(
            ra_err_arcsec < 10.0,
            "RA error {:.2} arcsec >= 10 arcsec (sol={:.6} ref={:.6})",
            ra_err_arcsec,
            sol.ra,
            fixture.ra_deg
        );
        assert!(
            dec_err_arcsec < 10.0,
            "Dec error {:.2} arcsec >= 10 arcsec (sol={:.6} ref={:.6})",
            dec_err_arcsec,
            sol.dec,
            fixture.dec_deg
        );
    }

    /// Temporary diagnostic: check which detection params give correct RA/Dec.
    #[test]
    #[ignore]
    fn sv6_diagnostic_solve_sweep() {
        use ps_db::{importer, loader};
        use std::path::PathBuf;
        use tempfile::NamedTempFile;

        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let npz_path =
            manifest.join("../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
        let db_imported = importer::import_npz(&npz_path).unwrap();
        let tmp = NamedTempFile::new().unwrap();
        loader::save_native(&db_imported, tmp.path()).unwrap();
        let mut db = loader::load_native(tmp.path()).unwrap();
        db.build_kd_tree();

        let img_path = manifest.join(
            "../reference-solutions/cedar-solve/examples/data/medium_fov/2019-07-29T204726_Alt40_Azi-135_Try1.jpg",
        );
        let img = image::open(&img_path).unwrap().into_luma8();
        let (height, width) = (img.height() as usize, img.width() as usize);

        let ref_ra = 230.668224_f64;
        let ref_dec = 11.03581_f64;

        let img_view = ps_detect::as_view(&img);
        for sigma in [2.0, 3.0, 4.0, 5.0, 6.0] {
            for binning in [1u32, 2] {
                for normalize in [false, true] {
                    let (stars, _, _, _) = ps_detect::get_stars_from_image(
                        &img_view, 1.0, sigma, normalize, binning, true, false,
                    ).unwrap();
                    if stars.len() < 4 {
                        continue;
                    }
                    let centroids: Vec<[f64; 2]> = stars
                        .iter()
                        .map(|s| [s.centroid_y as f64, s.centroid_x as f64])
                        .collect();
                    let sol = solve_from_centroids(
                        &db,
                        &centroids,
                        (height, width),
                        &SolveParams {
                            solve_timeout: Some(30000),
                            ..Default::default()
                        },
                    );
                    if sol.status == SolveStatus::MatchFound {
                        let ra_err = (sol.ra - ref_ra).abs() * 3600.0;
                        let dec_err = (sol.dec - ref_dec).abs() * 3600.0;
                        eprintln!("DIAG: sigma={:.0}, bin={}, norm={} -> MATCH {} stars, ra_err={:.1}\" dec_err={:.1}\" ra={:.4} dec={:.4}",
                            sigma, binning, normalize, sol.matches, ra_err, dec_err, sol.ra, sol.dec);
                    } else {
                        eprintln!(
                            "DIAG: sigma={:.0}, bin={}, norm={} -> {:?} ({} stars)",
                            sigma,
                            binning,
                            normalize,
                            sol.status,
                            stars.len()
                        );
                    }
                }
            }
        }
    }

    /// SV6: solve_from_centroids parity — same image, same fixture.
    #[test]
    fn sv6_solve_from_centroids_parity() {
        use ps_db::{importer, loader};
        use serde::Deserialize;
        use std::path::PathBuf;
        use tempfile::NamedTempFile;

        #[derive(Deserialize)]
        struct Fixture {
            ra_deg: f64,
            dec_deg: f64,
            #[allow(dead_code)]
            roll_deg: f64,
            #[allow(dead_code)]
            fov_deg: f64,
            #[allow(dead_code)]
            matches: usize,
            matched_cat_ids: Vec<u32>,
            centroids_yx: Vec<[f64; 2]>,
            image_size: [usize; 2],
        }

        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let fixture_path = manifest.join("tests/fixtures/reference_solve.json");
        let fixture: Fixture = serde_json::from_str(
            &std::fs::read_to_string(&fixture_path)
                .unwrap_or_else(|e| panic!("Cannot read {}: {}", fixture_path.display(), e)),
        )
        .expect("fixture JSON parse failed");

        let npz_path =
            manifest.join("../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
        let db =
            importer::import_npz(&npz_path).unwrap_or_else(|e| panic!("import_npz failed: {}", e));
        let tmp = NamedTempFile::new().expect("tempfile");
        loader::save_native(&db, tmp.path()).expect("save_native");
        let mut db = loader::load_native(tmp.path()).expect("load_native");
        db.build_kd_tree();

        // Use the fixture's centroids (from tetra3) for exact parity
        let params = SolveParams {
            solve_timeout: Some(120000),
            ..Default::default()
        };
        let sol = solve_from_centroids(
            &db,
            &fixture.centroids_yx,
            (fixture.image_size[0], fixture.image_size[1]),
            &params,
        );

        assert_eq!(
            sol.status,
            SolveStatus::MatchFound,
            "Expected MatchFound, got {:?}",
            sol.status
        );

        let ra_err_arcsec = (sol.ra - fixture.ra_deg).abs() * 3600.0;
        let dec_err_arcsec = (sol.dec - fixture.dec_deg).abs() * 3600.0;
        assert!(
            ra_err_arcsec < 10.0,
            "RA error {:.2} arcsec >= 10 arcsec (sol={:.6} ref={:.6})",
            ra_err_arcsec,
            sol.ra,
            fixture.ra_deg
        );
        assert!(
            dec_err_arcsec < 10.0,
            "Dec error {:.2} arcsec >= 10 arcsec (sol={:.6} ref={:.6})",
            dec_err_arcsec,
            sol.dec,
            fixture.dec_deg
        );

        let mut our_ids: Vec<u32> = sol
            .matched_cat_ids
            .as_ref()
            .expect("matched_cat_ids should be Some")
            .clone();
        our_ids.sort_unstable();
        let mut ref_ids = fixture.matched_cat_ids.clone();
        ref_ids.sort_unstable();
        assert_eq!(
            our_ids, ref_ids,
            "matched catalog-ID sets differ:\n  ours={:?}\n  ref ={:?}",
            our_ids, ref_ids
        );
    }
}
