//! Pattern catalog hash-table construction.
//!
//! Takes a deduplicated list of 4-star patterns, computes the edge-ratio key
//! and 64-bit hash for each, presorts the four star indices by centroid
//! distance, and open-address inserts them into a hash table sized to a prime
//! `2·N` (quadratic) or `3·N` (linear). Auxiliary arrays store the largest edge
//! (milliradians, f16) and the low 16 bits of the key hash.

use crate::catalog::CatalogEntry;
use crate::patterns::Pattern;
use math_core::pattern::{
    insert_at_index, next_prime, order_pattern_by_centroid_distance, pattern_key, pattern_key_hash,
    pattern_key_hash16, pattern_key_hash_index, PATTERN_SIZE,
};
use math_core::{angular_distance, UnitVector};

/// A built pattern catalog with all arrays needed by the solver.
#[derive(Clone, Debug, PartialEq)]
pub struct PatternCatalog {
    /// Hash table rows: star indices in centroid-distance order. The row dtype
    /// is the smallest unsigned int that holds the max star index.
    pub pattern_catalog: Vec<[usize; PATTERN_SIZE]>,
    /// Largest edge angle per occupied slot, in milliradians.
    pub pattern_largest_edge: Vec<f32>,
    /// Low 16 bits of the key hash per occupied slot.
    pub pattern_key_hashes: Vec<u16>,
    /// Table size (prime).
    pub table_size: usize,
    /// Number of patterns actually inserted.
    pub num_patterns: usize,
    /// Whether the table uses linear probing.
    pub linear_probe: bool,
    /// Number of quantization bins used for the key.
    pub pattern_bins: u32,
}

/// Build a pattern catalog from a list of patterns.
///
/// * `entries` is the star catalog (sorted by brightness).
/// * `patterns` is the deduplicated list of 4-star patterns (brightness order).
/// * `pattern_max_error` determines `pattern_bins = round(1/(4*error))`.
/// * `linear_probe` selects linear (`next_prime(3*N)`) vs quadratic
///   (`next_prime(2*N)`) probing.
pub fn build_pattern_catalog(
    entries: &[CatalogEntry],
    patterns: &[Pattern],
    pattern_max_error: f64,
    linear_probe: bool,
) -> PatternCatalog {
    assert!(!entries.is_empty());
    assert!(!patterns.is_empty());

    let bins = math_core::pattern::pattern_bins(pattern_max_error);
    let table_size = if linear_probe {
        next_prime(3 * patterns.len() as u64) as usize
    } else {
        next_prime(2 * patterns.len() as u64) as usize
    };

    let empty = [usize::MAX; PATTERN_SIZE];
    let mut catalog = vec![empty; table_size];
    let mut largest_edge = vec![0.0f32; table_size];
    let mut key_hashes = vec![0u16; table_size];

    let vectors: Vec<UnitVector> = entries
        .iter()
        .map(|e| UnitVector::from_radec(e.ra, e.dec))
        .collect();

    for &pattern in patterns {
        let pat_vectors = pattern.map(|idx| vectors[idx]);
        let (key, largest) = pattern_key(&pat_vectors, bins);
        let key_hash = pattern_key_hash(&key, bins);
        let hash_index = pattern_key_hash_index(key_hash, table_size, linear_probe);

        // Presort the pattern's star indices by centroid distance.
        let order = order_pattern_by_centroid_distance(&pat_vectors);
        let sorted_pattern: [usize; PATTERN_SIZE] = std::array::from_fn(|i| pattern[order[i]]);

        let slot = insert_at_index(
            sorted_pattern,
            hash_index,
            &mut catalog,
            linear_probe,
            |row| row[0] == usize::MAX,
        );

        key_hashes[slot] = pattern_key_hash16(key_hash);
        largest_edge[slot] = (largest * 1000.0) as f32;
    }

    PatternCatalog {
        pattern_catalog: catalog,
        pattern_largest_edge: largest_edge,
        pattern_key_hashes: key_hashes,
        table_size,
        num_patterns: patterns.len(),
        linear_probe,
        pattern_bins: bins,
    }
}

/// Compute the largest pairwise angular distance (radians) inside a pattern.
#[allow(dead_code)]
fn pattern_largest_edge_angle(vectors: &[UnitVector; PATTERN_SIZE]) -> f64 {
    let mut max = 0.0;
    for i in 0..PATTERN_SIZE {
        for j in (i + 1)..PATTERN_SIZE {
            let d = angular_distance(vectors[i], vectors[j]);
            if d > max {
                max = d;
            }
        }
    }
    max
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
    fn quadratic_table_size_is_next_prime_of_2n() {
        let entries = vec![
            entry(0.0, 0.0, 1.0, 1),
            entry(1.0, 0.0, 2.0, 2),
            entry(0.0, 1.0, 3.0, 3),
            entry(1.0, 1.0, 4.0, 4),
        ];
        let patterns = vec![[0, 1, 2, 3]];
        let catalog = build_pattern_catalog(&entries, &patterns, 0.001, false);
        assert_eq!(catalog.table_size, next_prime(2) as usize);
        assert!(!catalog.linear_probe);
    }

    #[test]
    fn linear_table_size_is_next_prime_of_3n() {
        let entries = vec![
            entry(0.0, 0.0, 1.0, 1),
            entry(1.0, 0.0, 2.0, 2),
            entry(0.0, 1.0, 3.0, 3),
            entry(1.0, 1.0, 4.0, 4),
        ];
        let patterns = vec![[0, 1, 2, 3]];
        let catalog = build_pattern_catalog(&entries, &patterns, 0.001, true);
        assert_eq!(catalog.table_size, next_prime(3) as usize);
        assert!(catalog.linear_probe);
    }

    #[test]
    fn patterns_are_presorted_by_centroid_distance() {
        // Four stars at different distances from the centroid.
        let entries = vec![
            entry(0.0, 0.0, 1.0, 1),
            entry(0.3, 0.0, 2.0, 2),
            entry(0.1, 0.2, 3.0, 3),
            entry(0.05, 0.25, 4.0, 4),
        ];
        let pattern = [0, 1, 2, 3];
        let vectors = pattern.map(|i| UnitVector::from_radec(entries[i].ra, entries[i].dec));
        let expected_order = order_pattern_by_centroid_distance(&vectors);
        let expected_sorted: [usize; 4] = std::array::from_fn(|i| pattern[expected_order[i]]);

        let catalog = build_pattern_catalog(&entries, &[pattern], 0.001, false);
        let slot = catalog
            .pattern_catalog
            .iter()
            .position(|row| row[0] != usize::MAX)
            .unwrap();
        assert_eq!(catalog.pattern_catalog[slot], expected_sorted);
    }

    #[test]
    fn largest_edge_is_stored_in_milliradians() {
        let entries = vec![
            entry(0.0, 0.0, 1.0, 1),
            entry(1.0, 0.0, 2.0, 2),
            entry(0.0, 1.0, 3.0, 3),
            entry(1.0, 1.0, 4.0, 4),
        ];
        let pattern = [0, 1, 2, 3];
        let vectors = pattern.map(|i| UnitVector::from_radec(entries[i].ra, entries[i].dec));
        let expected_largest = pattern_largest_edge_angle(&vectors);

        let catalog = build_pattern_catalog(&entries, &[pattern], 0.001, false);
        let slot = catalog
            .pattern_catalog
            .iter()
            .position(|row| row[0] != usize::MAX)
            .unwrap();
        assert!(
            (catalog.pattern_largest_edge[slot] - (expected_largest * 1000.0) as f32).abs() < 1e-3
        );
    }
}
