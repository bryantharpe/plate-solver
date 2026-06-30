//! Hash table builder for pattern databases.
//!
//! Takes a set of enumerated patterns (4-star index tuples) and inserts them
//! into an open-addressed hash table using the same probing strategy as the
//! reference Python implementation.

use std::collections::HashSet;

use half::f16;
use ps_core::pattern::{
    compute_pattern_key, compute_pattern_key_hash, key_hash_low16, order_by_centroid_distance,
    pattern_key_hash_to_index,
};

use crate::prime::next_prime;

/// Result of building the hash table.
pub struct HashTableResult {
    /// Pattern catalog — exactly one variant is Some
    pub catalog_u8: Option<Vec<[u8; 4]>>,
    pub catalog_u16: Option<Vec<[u16; 4]>>,
    pub catalog_u32: Option<Vec<[u32; 4]>>,
    /// Largest edge in milliradians (f16 per slot)
    pub largest_edge: Vec<f16>,
    /// Low 16 bits of pattern key hash (u16 per slot)
    pub key_hashes: Vec<u16>,
    /// Number of valid patterns inserted
    pub num_patterns: u32,
}

/// Build the hash table from enumerated patterns.
///
/// `patterns`: deduplicated set of 4-star index tuples (indices into `star_vectors`).
/// `star_vectors`: unit vectors for all stars, in brightness order.
/// `pattern_bins`: quantisation bins per edge-ratio dimension.
/// `linear_probe`: true for linear probing, false for quadratic probing.
pub fn build_hash_table(
    patterns: &HashSet<[usize; 4]>,
    star_vectors: &[[f32; 3]],
    pattern_bins: u32,
    linear_probe: bool,
) -> HashTableResult {
    // Table sizing: next_prime(2N) for quadratic, next_prime(3N) for linear
    let table_size = if linear_probe {
        next_prime(3 * patterns.len() as u64) as usize
    } else {
        next_prime(2 * patterns.len() as u64) as usize
    };

    // Determine element type by max star index
    let max_idx = patterns
        .iter()
        .map(|p| *p.iter().max().unwrap())
        .max()
        .unwrap_or(0);

    // Initialize catalogs with sentinel values (empty slots)
    let mut key_hashes = vec![0u16; table_size];
    let mut largest_edge = vec![f16::ZERO; table_size];

    let (mut catalog_u8, mut catalog_u16, mut catalog_u32) = if max_idx <= 255 {
        let cat = vec![[u8::MAX; 4]; table_size];
        (Some(cat), None, None)
    } else if max_idx <= 65534 {
        let cat = vec![[u16::MAX; 4]; table_size];
        (None, Some(cat), None)
    } else {
        let cat = vec![[u32::MAX; 4]; table_size];
        (None, None, Some(cat))
    };

    let mut num_patterns: u32 = 0;

    // Sort patterns for deterministic insertion order
    let mut sorted_patterns: Vec<_> = patterns.iter().copied().collect();
    sorted_patterns.sort();

    for pattern in &sorted_patterns {
        // Look up 4 unit vectors -> f64
        let vectors_f64: [[f64; 3]; 4] = pattern.map(|idx| {
            let v = star_vectors[idx];
            [v[0] as f64, v[1] as f64, v[2] as f64]
        });

        // Compute pattern key and largest edge
        let (key, largest_edge_rad) = compute_pattern_key(&vectors_f64, pattern_bins);

        // Compute hash
        let full_hash = compute_pattern_key_hash(&key, pattern_bins);
        let low16 = key_hash_low16(full_hash);

        // Map to initial table index
        let hash_index =
            pattern_key_hash_to_index(full_hash, table_size as u64, linear_probe) as usize;

        // Presort pattern indices by centroid distance
        let ordered = order_by_centroid_distance(&vectors_f64);
        let sorted_pattern: [usize; 4] = [
            pattern[ordered[0]],
            pattern[ordered[1]],
            pattern[ordered[2]],
            pattern[ordered[3]],
        ];

        // Open-address insert: probe until we find an empty slot
        let mut c = 0u64;
        loop {
            let slot = if linear_probe {
                (hash_index + c as usize) % table_size
            } else {
                (hash_index + (c * c) as usize) % table_size
            };

            // Check if empty: key_hashes[slot] == 0 && largest_edge[slot].to_bits() == 0
            if key_hashes[slot] == 0 && largest_edge[slot].to_bits() == 0 {
                // Insert here!
                let star_indices = [
                    sorted_pattern[0],
                    sorted_pattern[1],
                    sorted_pattern[2],
                    sorted_pattern[3],
                ];

                match (&mut catalog_u8, &mut catalog_u16, &mut catalog_u32) {
                    (Some(cat), None, None) => {
                        cat[slot] = [
                            star_indices[0] as u8,
                            star_indices[1] as u8,
                            star_indices[2] as u8,
                            star_indices[3] as u8,
                        ];
                    }
                    (None, Some(cat), None) => {
                        cat[slot] = [
                            star_indices[0] as u16,
                            star_indices[1] as u16,
                            star_indices[2] as u16,
                            star_indices[3] as u16,
                        ];
                    }
                    (None, None, Some(cat)) => {
                        cat[slot] = [
                            star_indices[0] as u32,
                            star_indices[1] as u32,
                            star_indices[2] as u32,
                            star_indices[3] as u32,
                        ];
                    }
                    _ => panic!("exactly one catalog variant must be Some"),
                }

                largest_edge[slot] = f16::from_f64(largest_edge_rad * 1000.0);
                key_hashes[slot] = low16;
                num_patterns += 1;
                break;
            }

            c += 1;
            if c as usize >= table_size {
                panic!(
                    "hash table full — table_size={}, patterns={}",
                    table_size,
                    patterns.len()
                );
            }
        }
    }

    HashTableResult {
        catalog_u8,
        catalog_u16,
        catalog_u32,
        largest_edge,
        key_hashes,
        num_patterns,
    }
}
