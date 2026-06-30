//! Hash-table pattern lookup with pre-filters.
//!
//! Walks the probe chain for a given pattern key hash and applies the same
//! pre-filters as the reference Python implementation's `_get_all_patterns_for_index`:
//! 1. Empty-slot termination (key_hashes==0 AND largest_edge bits==0)
//! 2. 16-bit key hash pre-filter
//! 3. Largest-edge / FOV pre-filter (when coarse_fov_rad is provided)

use ps_core::pattern::{compute_pattern_key_hash, key_hash_low16, pattern_key_hash_to_index};

use crate::Database;

/// Walk the probe chain for the given key and return all candidate slot indices.
///
/// Candidates pass all pre-filters:
///   1. Not an empty slot (both key_hashes[slot]==0 AND largest_edge[slot]==0 stops the chain)
///   2. 16-bit key pre-filter: key_hashes[slot] == key_hash_low16(full_hash)
///   3. Largest-edge/FOV pre-filter (only when coarse_fov_rad is Some):
///      the slot's largest_edge (mrad, f16) must satisfy:
///        fov2 = largest_edge_mrad / image_largest_edge_rad * coarse_fov_rad / 1000
///        accept if abs(fov2 - coarse_fov_rad) < fov_max_error
///      where fov_max_error = coarse_fov_rad * pattern_max_error
///
/// The probe chain runs up to table_size probes. Stops on empty slot.
pub fn lookup_pattern(
    db: &Database,
    key: &[u32; 5],
    largest_edge_rad: f64,
    coarse_fov_rad: Option<f64>,
) -> Vec<usize> {
    let table_size = db.num_slots() as u64;
    if table_size == 0 {
        return Vec::new();
    }

    let pattern_bins = db.properties.pattern_bins as u32;
    let linear_probe = db.properties.hash_table_type == "linear_probe";
    let pattern_max_error = db.properties.pattern_max_error as f64;

    // Compute the full 64-bit hash from the pattern key.
    let full_hash = compute_pattern_key_hash(key, pattern_bins);
    let low16 = key_hash_low16(full_hash);

    // Map to initial table index.
    let hash_index = pattern_key_hash_to_index(full_hash, table_size, linear_probe);

    // Determine FOV pre-filter parameters.
    // The reference uses: fov2 = largest_edge / image_pattern_largest_edge * fov_estimate / 1000
    // keep if abs(fov2 - fov_estimate) < fov_max_error
    // In our API, coarse_fov_rad corresponds to fov_estimate.
    // The reference's fov_max_error is derived from the caller; we compute it as
    // coarse_fov_rad * pattern_max_error to match the tolerance band concept.
    let (has_fov_filter, fov_max_error) = match coarse_fov_rad {
        Some(fov) => (true, fov * pattern_max_error),
        None => (false, 0.0),
    };

    let mut candidates = Vec::new();

    // Generate probe slots lazily (stop at first empty slot).
    for c in 0..db.num_slots() {
        let offset = if linear_probe {
            c as u64
        } else {
            (c as u64).wrapping_mul(c as u64)
        };
        let slot = ((hash_index + offset) % table_size) as usize;

        // Empty-slot check: key_hashes[slot] == 0 AND largest_edge[slot] == 0
        // The reference checks `all(table[i, :] == 0)` on pattern_catalog.
        // We use the surrogate: key_hashes==0 AND largest_edge bits==0.
        if db.key_hashes[slot] == 0 && db.largest_edge[slot].to_bits() == 0 {
            // Stop at first empty slot.
            break;
        }

        // 16-bit key hash pre-filter.
        if db.key_hashes[slot] != low16 {
            continue;
        }

        // Largest-edge / FOV pre-filter (only when coarse_fov_rad is Some).
        if has_fov_filter {
            let largest_edge_mrad = db.largest_edge[slot].to_f64();
            let fov2 = largest_edge_mrad / largest_edge_rad * coarse_fov_rad.unwrap() / 1000.0;
            if (fov2 - coarse_fov_rad.unwrap()).abs() >= fov_max_error {
                continue;
            }
        }

        candidates.push(slot);
    }

    candidates
}
