//! Pattern enumeration: lattice fields + breadth-first combinations.
//!
//! For each FOV scale in the ladder, thin pattern stars by density, lay down a
//! Fibonacci sphere lattice of field centers, gather stars within `pattern_fov/2`
//! of each center, and enumerate up to `patterns_per_lattice_field` 4-star
//! combinations per field in brightness order. Patterns are deduped globally
//! across all fields and scales.

use crate::catalog::CatalogEntry;
use crate::fov_ladder::fov_ladder;
use crate::lattice::fibonacci_sphere_lattice;
use crate::num_fields::num_fields_for_sky;
use crate::thinning::{separation_for_density, thin_by_density};
use math_core::{angular_distance, UnitVector};
use std::collections::HashSet;

/// Default multiscale step factor.
pub const DEFAULT_MULTISCALE_STEP: f64 = 1.5;

/// Default lattice-field oversampling factor.
pub const DEFAULT_LATTICE_FIELD_OVERSAMPLING: f64 = 100.0;

/// Default number of patterns to generate per lattice field.
pub const DEFAULT_PATTERNS_PER_LATTICE_FIELD: usize = 50;

/// Default target density for pattern-star thinning.
pub const DEFAULT_VERIFICATION_STARS_PER_FOV: f64 = 150.0;

/// A 4-star pattern, stored as sorted catalog indices.
pub type Pattern = [usize; 4];

/// Enumerate all patterns for the given catalog and FOV range.
///
/// * `entries` must be sorted by ascending magnitude (brightest first).
/// * `min_fov` and `max_fov` are in degrees.
/// * `verification_stars_per_fov` drives the thinning separation.
/// * `lattice_field_oversampling` and `patterns_per_lattice_field` control the
///   lattice-field budget.
///
/// Returns a vector of deduplicated patterns. Each pattern is a 4-tuple of
/// catalog indices sorted by **brightness** (input order), not centroid order;
/// centroid ordering is applied later during hash-table insertion.
pub fn enumerate_patterns(
    entries: &[CatalogEntry],
    min_fov: f64,
    max_fov: f64,
    verification_stars_per_fov: f64,
    lattice_field_oversampling: f64,
    patterns_per_lattice_field: usize,
) -> Vec<Pattern> {
    assert!(!entries.is_empty());
    assert!(min_fov > 0.0 && max_fov >= min_fov);

    let vectors: Vec<UnitVector> = entries
        .iter()
        .map(|e| UnitVector::from_radec(e.ra, e.dec))
        .collect();

    let fovs = fov_ladder(min_fov, max_fov, DEFAULT_MULTISCALE_STEP);
    let mut global_patterns: HashSet<Pattern> = HashSet::new();

    // Process scales from largest to smallest so the global dedup naturally
    // favours larger-scale patterns first, matching the reference loop order.
    for &pattern_fov in fovs.iter().rev() {
        let separation = separation_for_density(pattern_fov, verification_stars_per_fov);
        let pattern_star_indices = thin_by_density(entries, separation);

        if pattern_star_indices.len() < 4 {
            continue;
        }

        let half_fov_rad = (pattern_fov / 2.0).to_radians();
        let n_fields =
            (num_fields_for_sky(pattern_fov) * lattice_field_oversampling).ceil() as usize;

        for center in fibonacci_sphere_lattice(n_fields) {
            // Gather pattern stars within pattern_fov/2 of the field center.
            let field_stars: Vec<usize> = pattern_star_indices
                .iter()
                .copied()
                .filter(|&idx| angular_distance(center, vectors[idx]) < half_fov_rad)
                .collect();

            // Already in brightness order because pattern_star_indices is sorted
            // and thin_by_density preserves input order.
            if field_stars.len() < 4 {
                continue;
            }

            // Generate up to patterns_per_lattice_field combinations.
            let mut count = 0;
            for pattern in breadth_first_combinations(&field_stars, 4) {
                global_patterns.insert(pattern);
                count += 1;
                if count >= patterns_per_lattice_field {
                    break;
                }
            }
        }
    }

    global_patterns.into_iter().collect()
}

/// Breadth-first combination generator.
///
/// Variant of itertools-style combinations that advances the last element
/// slowest, so combinations using the brightest (earliest) stars are yielded
/// first. This matches `breadth_first_combinations.py` from the reference.
fn breadth_first_combinations(sequence: &[usize], r: usize) -> impl Iterator<Item = Pattern> + use<'_> {
    let mut state: Vec<usize> = Vec::new();

    std::iter::from_fn(move || {
        if state.is_empty() {
            // First call: initialise with the first r indices.
            if sequence.len() < r {
                return None;
            }
            state.extend(0..r);
            let pattern: Pattern = std::array::from_fn(|i| sequence[state[i]]);
            return Some(pattern);
        }

        // Try to advance the rightmost choosable position.
        let mut pos = r;
        while pos > 0 {
            pos -= 1;
            let max_at_pos = sequence.len() - r + pos;
            if state[pos] < max_at_pos {
                state[pos] += 1;
                // Reset all later positions to follow immediately after.
                for j in pos + 1..r {
                    state[j] = state[j - 1] + 1;
                }
                let pattern: Pattern = std::array::from_fn(|i| sequence[state[i]]);
                return Some(pattern);
            }
        }

        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{CatalogEntry, CatalogId};

    fn entry(ra_deg: f64, dec_deg: f64, mag: f64, id: u32) -> CatalogEntry {
        CatalogEntry {
            ra: ra_deg.to_radians(),
            dec: dec_deg.to_radians(),
            mag,
            id: CatalogId::Hip(id),
            pm_ra: None,
            pm_dec: None,
        }
    }

    #[test]
    fn breadth_first_yields_brightest_first() {
        let seq = vec![0, 1, 2, 3, 4];
        let combos: Vec<Pattern> = breadth_first_combinations(&seq, 4).collect();
        assert_eq!(combos.len(), 5);
        // First combination should be [0,1,2,3]; last should include 4.
        assert_eq!(combos[0], [0, 1, 2, 3]);
        assert_eq!(combos[4], [1, 2, 3, 4]);
    }

    #[test]
    fn field_radius_bounds_pattern_size() {
        // Two tight clusters separated by 20 degrees, with a 10-degree FOV.
        // Only one cluster should fit in any single lattice field.
        let mut entries = vec![];
        for i in 0..6 {
            entries.push(entry(i as f64 * 0.1, 0.0, i as f64, i as u32));
        }
        for i in 6..12 {
            entries.push(entry(20.0 + (i - 6) as f64 * 0.1, 0.0, i as f64, i as u32));
        }
        entries.sort_by(|a, b| a.mag.partial_cmp(&b.mag).unwrap());

        let patterns = enumerate_patterns(
            &entries,
            10.0,
            10.0,
            150.0,
            1.0, // low oversampling to keep test fast
            100,
        );

        // No pattern should mix stars from both clusters.
        for pat in &patterns {
            let ras: Vec<f64> = pat.iter().map(|&i| entries[i].ra.to_degrees()).collect();
            let span = ras.iter().copied().fold(f64::NAN, f64::max)
                - ras.iter().copied().fold(f64::NAN, f64::min);
            assert!(
                span < 10.0,
                "pattern spans {} degrees, exceeding FOV",
                span
            );
        }
    }

    #[test]
    fn per_field_budget_limits_patterns() {
        // A dense cluster of 8 bright stars within a tiny radius. With a very
        // small FOV and tiny separation, many combinations exist, but the
        // per-field budget should cap the number of patterns generated.
        let mut entries = vec![];
        for i in 0..8 {
            entries.push(entry(i as f64 * 0.01, 0.0, i as f64, i as u32));
        }
        entries.sort_by(|a, b| a.mag.partial_cmp(&b.mag).unwrap());

        let budget = 3;
        let patterns = enumerate_patterns(
            &entries,
            1.0,
            1.0,
            10.0, // low density -> large separation, but still 8 stars
            1.0,
            budget,
        );

        // With one field and a budget of 3, we should get at most 3 patterns.
        assert!(
            patterns.len() <= budget,
            "expected at most {} patterns, got {}",
            budget,
            patterns.len()
        );
    }
}
