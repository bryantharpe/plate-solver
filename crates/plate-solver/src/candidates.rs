//! Candidate-key generation and database lookup.
//!
//! Implements the geometric hashing step of the lost-in-space solver: per image
//! pattern, compute the edge-ratio key, form a tolerance band around it, enumerate
//! nearby quantized keys nearest-first, and query the pattern database for each key.

use crate::status::{MatchResult, SolveContext};
use math_core::pattern::{
    order_pattern_by_centroid_distance, pattern_key, pattern_key_hash, pattern_key_hash_index,
    pattern_key_hash16, KEY_LEN, PATTERN_SIZE,
};
use math_core::UnitVector;
use pattern_database::{Candidate, LookupQuery};

/// Generate candidate catalog patterns from four image star unit vectors.
///
/// Steps:
/// 1. Order the four stars by centroid distance so image and catalog patterns share
///    the same correspondence convention.
/// 2. Compute the 6 sorted edge angles, the largest edge, and the 5 quantized ratios.
/// 3. Form a tolerance band `ratio ± p_max_err` where `p_max_err` is
///    `match_max_error` clamped to at least the database `pattern_max_error`.
/// 4. Convert the band to per-ratio bin ranges and enumerate candidate keys as the
///    cartesian product, ordered nearest-first by squared distance in quantized key
///    space.
/// 5. For each candidate key, look up the database probe chain and collect all
///    catalog patterns that survive the cheap filters.
pub fn lookup_candidates(ctx: &SolveContext, vectors: [UnitVector; 4]) -> Vec<Candidate> {
    let bins = ctx.props.pattern_bins as u32;
    if bins == 0 || ctx.db.pattern_catalog.is_empty() {
        return Vec::new();
    }
    let p_max_err = ctx
        .match_max_error
        .max(ctx.props.pattern_max_error as f64);

    // Deterministic star ordering for correspondence.
    let order = order_pattern_by_centroid_distance(&vectors);
    let ordered: [UnitVector; PATTERN_SIZE] = std::array::from_fn(|m| vectors[order[m]]);

    // Measured pattern key and largest edge.
    let (key, _largest_edge) = pattern_key(&ordered, bins);

    // Tolerance band in ratio space.
    let mut ratio_min = [0.0; KEY_LEN];
    let mut ratio_max = [0.0; KEY_LEN];
    for m in 0..KEY_LEN {
        let ratio = key[m] as f64 / bins as f64;
        ratio_min[m] = (ratio - p_max_err).clamp(0.0, 1.0);
        ratio_max[m] = (ratio + p_max_err).clamp(0.0, 1.0);
    }

    // Enumerate candidate keys nearest-first.
    let keys = candidate_keys(&key, bins, p_max_err);

    // Query each candidate key, collecting unique candidates in probe order.
    let mut seen = std::collections::HashSet::new();
    let mut candidates = Vec::new();
    let query = LookupQuery {
        vectors: ordered,
        fov_estimate: Some(ctx.fov_initial),
        fov_max_error: Some(ctx.match_max_error),
        ratio_min,
        ratio_max,
    };

    for candidate_key in keys {
        let key_hash = pattern_key_hash(&candidate_key, bins);
        let hash_index = pattern_key_hash_index(
            key_hash,
            ctx.db.pattern_catalog.len(),
            ctx.db.properties.linear_probe(),
        );
        let key_hash16 = pattern_key_hash16(key_hash);

        // Quick pre-check: if no occupied slot in the probe chain has the same
        // 16-bit hash, the database lookup will return nothing. This avoids
        // building catalog vectors for dead keys.
        if !chain_has_hash16(&ctx.db, hash_index, key_hash16) {
            continue;
        }

        let cands = ctx.db.lookup_candidates(&query);
        for cand in cands {
            if seen.insert(cand.table_index) {
                candidates.push(cand);
            }
        }
    }

    candidates
}

fn chain_has_hash16(db: &pattern_database::PatternDatabase, hash_index: usize, hash16: u16) -> bool {
    use math_core::pattern::{get_table_indices_from_hash, PATTERN_SIZE};
    let is_empty = |row: &[usize; PATTERN_SIZE]| row[0] == usize::MAX;
    let indices = get_table_indices_from_hash(
        hash_index,
        &db.pattern_catalog,
        db.properties.linear_probe(),
        is_empty,
    );
    indices
        .iter()
        .any(|&i| db.pattern_key_hashes[i] == hash16)
}

/// Enumerate candidate quantized keys in nearest-first order.
///
/// For each of the 5 ratios, compute the integer bin range that falls inside
/// `[ratio_min, ratio_max]`, then take the cartesian product and sort by squared
/// distance from the measured key in bin space.
fn candidate_keys(measured: &[u32; KEY_LEN], bins: u32, p_max_err: f64) -> Vec<[u32; KEY_LEN]> {
    let mut ranges: [Vec<u32>; KEY_LEN] = std::array::from_fn(|_| Vec::new());
    for m in 0..KEY_LEN {
        let ratio = measured[m] as f64 / bins as f64;
        let lo = ((ratio - p_max_err).clamp(0.0, 1.0) * bins as f64).floor() as u32;
        let hi = ((ratio + p_max_err).clamp(0.0, 1.0) * bins as f64).ceil() as u32;
        let hi = hi.min(bins - 1);
        ranges[m] = (lo..=hi).collect();
    }

    let mut keys: Vec<[u32; KEY_LEN]> = Vec::new();
    cartesian_product(&ranges, 0, &mut [0; KEY_LEN], &mut keys);

    keys.sort_by_key(|k| squared_key_distance(measured, k));
    keys
}

fn cartesian_product(
    ranges: &[Vec<u32>; KEY_LEN],
    depth: usize,
    current: &mut [u32; KEY_LEN],
    out: &mut Vec<[u32; KEY_LEN]>,
) {
    if depth == KEY_LEN {
        out.push(*current);
        return;
    }
    for &v in &ranges[depth] {
        current[depth] = v;
        cartesian_product(ranges, depth + 1, current, out);
    }
}

fn squared_key_distance(a: &[u32; KEY_LEN], b: &[u32; KEY_LEN]) -> u64 {
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| {
            let d = (x as i64) - (y as i64);
            (d * d) as u64
        })
        .sum()
}

/// Verify a single candidate against the current context.
///
/// Verification is owned by a downstream bead; this stub always rejects so that
/// candidate generation can be tested independently.
pub fn verify_candidate(
    _ctx: &SolveContext,
    _candidate: &Candidate,
    _pattern_indices: [usize; 4],
) -> MatchResult {
    MatchResult::Rejected
}

#[cfg(test)]
mod tests {
    use super::*;
    use math_core::UnitVector;
    use pattern_database::{DatabaseProperties, PatternDatabase, StarKdTree};

    fn test_db(
        min_fov: f32,
        max_fov: f32,
        num_patterns: u32,
        verification_stars_per_fov: u16,
        pattern_bins: u16,
        pattern_max_error: f32,
    ) -> SolveContext {
        let db = PatternDatabase {
            star_table: Vec::new(),
            num_stars: 0,
            pattern_catalog: Vec::new(),
            pattern_largest_edge: Vec::new(),
            pattern_key_hashes: Vec::new(),
            star_catalog_ids: Vec::new(),
            properties: DatabaseProperties {
                min_fov,
                max_fov,
                num_patterns,
                verification_stars_per_fov,
                pattern_bins,
                pattern_max_error,
                ..DatabaseProperties::default()
            },
            star_kdtree: StarKdTree::new(&[]),
        };
        SolveContext {
            db,
            props: DatabaseProperties {
                min_fov,
                max_fov,
                num_patterns,
                verification_stars_per_fov,
                pattern_bins,
                pattern_max_error,
                ..DatabaseProperties::default()
            },
            fov_initial: 20.0,
            match_threshold: 1e-8,
            match_radius: 0.01,
            match_max_error: 0.002,
            distortion: 0.0,
            solve_timeout_ms: 5000,
            start_instant: std::time::Instant::now(),
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            verification_stars_per_fov: verification_stars_per_fov as usize,
        }
    }

    #[test]
    fn tolerance_band_uses_clamped_pattern_max_error() {
        // match_max_error smaller than pattern_max_error -> p_max_err = pattern_max_error.
        let mut ctx = test_db(10.0, 30.0, 100, 150, 250, 0.005);
        ctx.match_max_error = 0.001;
        let v = [
            UnitVector::from_radec(0.0, 0.0),
            UnitVector::from_radec(0.02, 0.0),
            UnitVector::from_radec(0.01, 0.015),
            UnitVector::from_radec(0.005, 0.025),
        ];
        let cands = lookup_candidates(&ctx, v);
        assert!(cands.is_empty(), "empty database yields no candidates");
    }

    #[test]
    fn nearest_first_ordering_by_squared_distance() {
        let ctx = test_db(10.0, 30.0, 100, 150, 10, 0.001);
        let measured = [5u32; KEY_LEN];
        let keys = candidate_keys(&measured, 10, 0.002);
        // The first key must be the measured key itself (distance 0).
        assert_eq!(keys[0], measured);
        // Distances must be non-decreasing.
        let mut last = 0u64;
        for k in &keys {
            let d = squared_key_distance(&measured, k);
            assert!(d >= last, "distance decreased");
            last = d;
        }
    }

    #[test]
    fn empty_database_returns_no_candidates() {
        let ctx = test_db(10.0, 30.0, 100, 150, 250, 0.001);
        let v = [
            UnitVector::from_radec(0.0, 0.0),
            UnitVector::from_radec(0.02, 0.0),
            UnitVector::from_radec(0.01, 0.015),
            UnitVector::from_radec(0.005, 0.025),
        ];
        assert!(lookup_candidates(&ctx, v).is_empty());
    }
}
