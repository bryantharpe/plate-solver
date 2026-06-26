//! MC6 — edge-ratio pattern key, hashing, and probing.
//!
//! Implements the side-of-triangle (edge-ratio) pattern key used by tetra3's
//! lost-in-space solver. The key is rotation-invariant: it normalises the six
//! pairwise edge angles of a 4-star pattern by its largest angle, then
//! quantises the five smaller ratios into integer bins.
//!
//! Reference: `reference-solutions/tetra3/tetra3.py`
//! (`_compute_pattern_key`, `_compute_pattern_key_hash`,
//!  `_pattern_key_hash_to_index`, `_get_order`).

use crate::angle::angle_from_distance;

/// Magic random constant for quadratic probe: floor(2^32 / phi) = 2654435761.
pub const MAGIC_RAND: u64 = 2654435761;

/// Compute `pattern_bins` from the maximum allowed fractional edge error.
///
/// Formula: `round(1 / (4 * pattern_max_error))`.
pub fn compute_pattern_bins(pattern_max_error: f64) -> u32 {
    (1.0 / (4.0 * pattern_max_error)).round() as u32
}

/// Order 4 pattern stars by ascending distance from their centroid.
///
/// Centroid = mean of the 4 unit vectors. Returns permutation indices
/// `[i0, i1, i2, i3]` sorted by ascending Euclidean distance to the centroid.
pub fn order_by_centroid_distance(vectors: &[[f64; 3]; 4]) -> [usize; 4] {
    // Compute centroid (mean of the 4 vectors).
    let mut cx: f64 = 0.0;
    let mut cy: f64 = 0.0;
    let mut cz: f64 = 0.0;
    for v in vectors.iter() {
        cx += v[0];
        cy += v[1];
        cz += v[2];
    }
    cx /= 4.0;
    cy /= 4.0;
    cz /= 4.0;

    // Distance squared from each vector to the centroid (no sqrt needed for sorting).
    let mut dists: [(f64, usize); 4] = [
        (
            (vectors[0][0] - cx).powi(2)
                + (vectors[0][1] - cy).powi(2)
                + (vectors[0][2] - cz).powi(2),
            0,
        ),
        (
            (vectors[1][0] - cx).powi(2)
                + (vectors[1][1] - cy).powi(2)
                + (vectors[1][2] - cz).powi(2),
            1,
        ),
        (
            (vectors[2][0] - cx).powi(2)
                + (vectors[2][1] - cy).powi(2)
                + (vectors[2][2] - cz).powi(2),
            2,
        ),
        (
            (vectors[3][0] - cx).powi(2)
                + (vectors[3][1] - cy).powi(2)
                + (vectors[3][2] - cz).powi(2),
            3,
        ),
    ];
    dists.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    [dists[0].1, dists[1].1, dists[2].1, dists[3].1]
}

/// Compute the 5-element integer pattern key from 4 unit vectors.
///
/// Algorithm:
/// 1. Order the 4 vectors by ascending distance from their centroid (deterministic).
/// 2. Form all C(4,2) = 6 pairwise edge angles using `angle_from_distance`.
/// 3. Sort the 6 angles ascending.
/// 4. Largest edge `L = edges[5]` is the normaliser.
/// 5. Ratios: `edges[m] / L` for `m = 0..4` (the five smaller edges).
/// 6. Quantise: `key[m] = (ratio * pattern_bins) as u32`.
///
/// Returns `([key0..key4], largest_edge_radians)`.
pub fn compute_pattern_key(vectors: &[[f64; 3]; 4], pattern_bins: u32) -> ([u32; 5], f64) {
    // Step 1: order vectors deterministically.
    let order = order_by_centroid_distance(vectors);
    let ordered: [[f64; 3]; 4] = [
        vectors[order[0]],
        vectors[order[1]],
        vectors[order[2]],
        vectors[order[3]],
    ];

    // Step 2: compute all 6 pairwise chord distances, then convert to angles.
    let mut edges: [f64; 6] = [0.0; 6];
    let mut idx: usize = 0;
    for i in 0..4 {
        for j in (i + 1)..4 {
            let dx = ordered[i][0] - ordered[j][0];
            let dy = ordered[i][1] - ordered[j][1];
            let dz = ordered[i][2] - ordered[j][2];
            let d = (dx * dx + dy * dy + dz * dz).sqrt();
            edges[idx] = angle_from_distance(d);
            idx += 1;
        }
    }

    // Step 3: sort ascending.
    edges.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Step 4: largest edge is the normaliser.
    let largest = edges[5];

    // Steps 5-6: ratios and quantisation.
    let mut key = [0u32; 5];
    for m in 0..5 {
        let ratio = edges[m] / largest;
        key[m] = (ratio * pattern_bins as f64) as u32;
    }

    (key, largest)
}

/// Pack the 5-element key into a 64-bit positional code.
///
/// Formula: `sum(key[m] * bins^m)` for `m = 0..4`, computed in `u64`.
/// Overflow wraps intentionally (mod 2^64), matching numpy uint64 semantics.
pub fn compute_pattern_key_hash(key: &[u32; 5], bins: u32) -> u64 {
    let bins = bins as u64;
    let mut hash: u64 = 0;
    let mut power: u64 = 1; // bins^0
    for &k in key.iter() {
        hash = hash.wrapping_add(k as u64 * power);
        power = power.wrapping_mul(bins);
    }
    hash
}

/// Map a key hash to a table index.
///
/// Quadratic probe (`linear_probe = false`): `(hash * MAGIC_RAND) % table_size`
/// Linear probe  (`linear_probe = true`):  `hash % table_size`
///
/// Multiplication overflow wraps mod 2^64 intentionally (matching numpy uint64).
pub fn pattern_key_hash_to_index(hash: u64, table_size: u64, linear_probe: bool) -> u64 {
    if linear_probe {
        hash % table_size
    } else {
        hash.wrapping_mul(MAGIC_RAND) % table_size
    }
}

/// Low 16 bits of key hash for pre-filtering.
pub fn key_hash_low16(hash: u64) -> u16 {
    (hash & 0xFFFF) as u16
}

/// Generate probe slot indices for a given hash index.
///
/// Returns `max_probes` slot indices.
/// Offset: `c` (linear) or `c*c` (quadratic), for `c = 0, 1, 2, ...`.
/// Slot: `(hash_index + offset(c)) % table_size`.
pub fn probe_slots(
    hash_index: u64,
    table_size: u64,
    linear_probe: bool,
    max_probes: usize,
) -> Vec<u64> {
    (0..max_probes)
        .map(|c| {
            let offset = if linear_probe {
                c as u64
            } else {
                (c as u64).wrapping_mul(c as u64)
            };
            (hash_index + offset) % table_size
        })
        .collect()
}
