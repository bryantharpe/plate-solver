//! Key-to-candidates lookup and rejection filters.
//!
//! Implements the open-addressing probe chain, 16-bit key pre-filter,
//! largest-edge/FOV pre-filter, and edge-ratio band test described in the spec.

use math_core::pattern::{
    get_table_indices_from_hash, pattern_key, pattern_key_hash, pattern_key_hash16,
    pattern_key_hash_index, PATTERN_SIZE,
};
use math_core::{angular_distance, UnitVector};

use crate::format::PatternDatabase;

/// Query describing an image pattern.
#[derive(Debug, Clone, Copy)]
pub struct LookupQuery {
    /// The four image star unit vectors, already ordered by centroid distance.
    pub vectors: [UnitVector; PATTERN_SIZE],
    /// Optional FOV estimate in degrees.
    pub fov_estimate: Option<f64>,
    /// Optional FOV tolerance in degrees.
    pub fov_max_error: Option<f64>,
    /// Lower bound of the edge-ratio tolerance band.
    pub ratio_min: [f64; 5],
    /// Upper bound of the edge-ratio tolerance band.
    pub ratio_max: [f64; 5],
}

/// A candidate pattern from the catalog that survived the cheap filters.
#[derive(Debug, Clone, Copy)]
pub struct Candidate {
    /// Index of the occupied slot in the pattern catalog table.
    pub table_index: usize,
    /// The four catalog star indices in centroid-distance order.
    pub star_indices: [usize; PATTERN_SIZE],
    /// The six sorted catalog edge angles in radians.
    pub edges: [f64; 6],
}

impl PatternDatabase {
    /// Compute the 64-bit key hash and largest edge for an image pattern.
    fn image_key_hash(&self, vectors: &[UnitVector; PATTERN_SIZE]) -> (u64, f64) {
        let (key, largest) = pattern_key(vectors, self.properties.pattern_bins as u32);
        (pattern_key_hash(&key, self.properties.pattern_bins as u32), largest)
    }

    /// Look up candidate patterns for an image pattern.
    ///
    /// Applies the 16-bit key pre-filter, the largest-edge/FOV pre-filter when
    /// both  and  are present, and the edge-ratio
    /// band test. Returns candidates in probe order.
    pub fn lookup_candidates(&self, query: &LookupQuery) -> Vec<Candidate> {
        let (key_hash, image_largest_edge) = self.image_key_hash(&query.vectors);
        let hash_index = pattern_key_hash_index(
            key_hash,
            self.pattern_catalog.len(),
            self.properties.linear_probe(),
        );
        let key_hash16 = pattern_key_hash16(key_hash);

        let table = &self.pattern_catalog;
        let is_empty = |row: &[usize; PATTERN_SIZE]| row[0] == usize::MAX;
        let occupied =
            get_table_indices_from_hash(hash_index, table, self.properties.linear_probe(), is_empty);

        let mut candidates = Vec::new();
        for table_index in occupied {
            // 16-bit key pre-filter.
            if self.pattern_key_hashes[table_index] != key_hash16 {
                continue;
            }

            // Largest-edge / FOV pre-filter.
            if let (Some(fov_estimate), Some(fov_max_error)) =
                (query.fov_estimate, query.fov_max_error)
            {
                let catalog_largest = self.pattern_largest_edge[table_index] as f64 / 1000.0;
                let implied_fov = catalog_largest / image_largest_edge * fov_estimate;
                if (implied_fov - fov_estimate).abs() > fov_max_error {
                    continue;
                }
            }

            let star_indices = self.pattern_catalog[table_index];
            let vectors: [UnitVector; PATTERN_SIZE] =
                std::array::from_fn(|k| self.star_vector(crate::format::StarId(star_indices[k])).unwrap());

            let (_key, catalog_largest) =
                pattern_key(&vectors, self.properties.pattern_bins as u32);
            let mut edges: [f64; 6] = [0.0; 6];
            let mut idx = 0;
            for i in 0..PATTERN_SIZE {
                for j in (i + 1)..PATTERN_SIZE {
                    edges[idx] = angular_distance(vectors[i], vectors[j]);
                    idx += 1;
                }
            }
            edges.sort_by(|a, b| a.partial_cmp(b).expect("edge angle must be comparable"));
            assert!(
                (edges[5] - catalog_largest).abs() < 1e-12,
                "largest edge mismatch"
            );

            // Edge-ratio band test.
            let mut pass = true;
            for (m, edge) in edges.iter().enumerate().take(5) {
                let ratio = edge / catalog_largest;
                if ratio <= query.ratio_min[m] || ratio >= query.ratio_max[m] {
                    pass = false;
                    break;
                }
            }
            if !pass {
                continue;
            }

            candidates.push(Candidate {
                table_index,
                star_indices,
                edges,
            });
        }
        candidates
    }
}
