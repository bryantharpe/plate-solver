//! Candidate-key generation and catalog-pattern filtering.
//!
//! Owned by the current bead as a stub; full implementation is downstream work.

use crate::SolveOptions;
use math_core::{
    pattern::{
        pattern_key, pattern_key_hash, pattern_key_hash16, pattern_key_hash_index, PATTERN_SIZE,
    },
    UnitVector,
};
use pattern_database::PatternDatabase;

/// A candidate catalog pattern ready for verification.
#[derive(Clone, Debug)]
pub struct Candidate {
    /// Star-table indices of the four catalog stars.
    pub star_indices: [usize; 4],
    /// Six sorted edge angles in radians.
    pub edges: [f64; 6],
    /// Unit vectors of the four catalog stars, ordered for correspondence.
    pub vectors: [UnitVector; PATTERN_SIZE],
}

/// Generate candidate catalog patterns for an image pattern.
pub fn generate_candidates(
    image_vectors: &[UnitVector; PATTERN_SIZE],
    options: &SolveOptions,
    database: &PatternDatabase,
) -> Vec<Candidate> {
    let props = &database.properties;
    let bins = props.pattern_bins as u32;
    let (key, largest_edge) = pattern_key(image_vectors, bins);
    let key_hash = pattern_key_hash(&key, bins);
    let hash16 = pattern_key_hash16(key_hash);

    let linear_probe = props.linear_probe();
    let table_size = database.pattern_catalog.len();
    if table_size == 0 {
        return Vec::new();
    }
    let hash_index = pattern_key_hash_index(key_hash, table_size, linear_probe);

    let mut candidates = Vec::new();
    for offset in 0..table_size {
        let idx = if linear_probe {
            (hash_index + offset) % table_size
        } else {
            let o = (offset * offset) as u64;
            ((hash_index as u64 + o) % table_size as u64) as usize
        };
        if database.pattern_catalog.is_empty() {
            break;
        }
        if idx >= database.pattern_catalog.len() {
            break;
        }
        // Empty-slot sentinel: a row of all zeros is treated as empty for the stub.
        let row = database.pattern_catalog[idx];
        if row == [0; 4] {
            break;
        }
        if database.pattern_key_hashes[idx] != hash16 {
            continue;
        }
        // TODO(ps-plate-02): largest-edge/FOV pre-filter, edge-ratio band test.
        let _ = (options, largest_edge);
        candidates.push(Candidate {
            star_indices: row,
            edges: [0.0; 6],
            vectors: [
                UnitVector::from_radec(0.0, 0.0),
                UnitVector::from_radec(0.0, 0.0),
                UnitVector::from_radec(0.0, 0.0),
                UnitVector::from_radec(0.0, 0.0),
            ],
        });
    }

    candidates
}
