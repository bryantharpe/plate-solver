//! Edge-ratio pattern key for 4-star geometric hashing.
//!
//! Implements the rotation- and scale-invariant fingerprint used by tetra3/cedar:
//! six pairwise edge angles, sorted, normalised by the largest, and quantized into
//! a 5-tuple key. The key is packed into a 64-bit hash, mapped to a table index, and
//! stored with a 16-bit pre-filter. Patterns are ordered by centroid distance for
//! deterministic star-to-star correspondence.

use crate::{angular_distance, UnitVector};

/// Knuth-style multiplicative hash constant `⌊2³²/φ⌋` used for quadratic probing.
pub const MAGIC_RAND: u64 = 2654435761;

/// Number of stars in a pattern.
pub const PATTERN_SIZE: usize = 4;

/// Number of pairwise edges in a 4-star pattern (`C(4,2)`).
pub const NUM_EDGES: usize = 6;

/// Number of ratios in the pattern key (all edges except the normalizer).
pub const KEY_LEN: usize = NUM_EDGES - 1;

/// Compute the number of quantization bins from the maximum allowed pattern error.
///
/// `pattern_bins = round(1 / (4 * pattern_max_error))`. This matches the reference
/// convention: `0.001` → `250`, `0.005` → `50`.
pub fn pattern_bins(pattern_max_error: f64) -> u32 {
    (1.0 / (4.0 * pattern_max_error)).round() as u32
}

/// Compute the edge-ratio pattern key for a 4-star pattern.
///
/// Forms all six pairwise central angles using the chord form `2·arcsin(d/2)`,
/// sorts them ascending, takes the largest as normalizer `L`, computes the five
/// ratios `e[m] / L`, and quantizes each as `int(ratio * pattern_bins)`.
///
/// Returns the five quantized ratios and the largest edge angle in radians.
///
/// # Panics
///
/// Panics if any two input vectors are identical (giving a zero largest edge),
/// because a degenerate pattern has no meaningful normalizer.
pub fn pattern_key(
    vectors: &[UnitVector; PATTERN_SIZE],
    pattern_bins: u32,
) -> ([u32; KEY_LEN], f64) {
    let mut edges: [f64; NUM_EDGES] = [0.0; NUM_EDGES];
    let mut idx = 0;
    for i in 0..PATTERN_SIZE {
        for j in (i + 1)..PATTERN_SIZE {
            edges[idx] = angular_distance(vectors[i], vectors[j]);
            idx += 1;
        }
    }
    edges.sort_by(|a, b| a.partial_cmp(b).expect("edge angle must be comparable"));

    let largest = edges[NUM_EDGES - 1];
    assert!(largest > 0.0, "degenerate pattern with zero largest edge");

    let mut key = [0u32; KEY_LEN];
    for m in 0..KEY_LEN {
        key[m] = ((edges[m] / largest) * (pattern_bins as f64)) as u32;
    }
    (key, largest)
}

/// Pack a pattern key into a 64-bit positional code.
///
/// `key_hash = Σ_m key[m] · pattern_bins^m`, computed with intentional wrapping
/// overflow in `u64` arithmetic to match the reference implementation.
pub fn pattern_key_hash(key: &[u32; KEY_LEN], pattern_bins: u32) -> u64 {
    let mut hash: u64 = 0;
    let mut factor: u64 = 1;
    let bins = pattern_bins as u64;
    for &k in key.iter() {
        hash = hash.wrapping_add((k as u64).wrapping_mul(factor));
        factor = factor.wrapping_mul(bins);
    }
    hash
}

/// Map a 64-bit pattern key hash to a table index.
///
/// * Quadratic probe: `(key_hash * MAGIC_RAND) mod table_size`
/// * Linear probe: `key_hash mod table_size`
///
/// `table_size` must be non-zero.
pub fn pattern_key_hash_index(key_hash: u64, table_size: usize, linear_probe: bool) -> usize {
    if linear_probe {
        (key_hash % table_size as u64) as usize
    } else {
        key_hash
            .wrapping_mul(MAGIC_RAND)
            .wrapping_rem(table_size as u64) as usize
    }
}

/// Return the low 16 bits of a pattern key hash for the pre-filter.
pub fn pattern_key_hash16(key_hash: u64) -> u16 {
    (key_hash & 0xFFFF) as u16
}

/// Order the four stars of a pattern by ascending squared Euclidean distance from
/// the pattern centroid (mean of the four unit vectors).
///
/// Returns the input indices in centroid-distance order. This ordering is
/// deterministic and rotation-invariant, so the m-th image star corresponds to
/// the m-th catalog star after both patterns are ordered the same way.
pub fn order_pattern_by_centroid_distance(
    vectors: &[UnitVector; PATTERN_SIZE],
) -> [usize; PATTERN_SIZE] {
    let centroid = UnitVector {
        x: (vectors[0].x + vectors[1].x + vectors[2].x + vectors[3].x) / 4.0,
        y: (vectors[0].y + vectors[1].y + vectors[2].y + vectors[3].y) / 4.0,
        z: (vectors[0].z + vectors[1].z + vectors[2].z + vectors[3].z) / 4.0,
    };

    let mut distances: [(f64, usize); PATTERN_SIZE] = [
        (squared_distance(vectors[0], centroid), 0),
        (squared_distance(vectors[1], centroid), 1),
        (squared_distance(vectors[2], centroid), 2),
        (squared_distance(vectors[3], centroid), 3),
    ];
    distances.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .expect("centroid distance must be comparable")
    });

    [
        distances[0].1,
        distances[1].1,
        distances[2].1,
        distances[3].1,
    ]
}

fn squared_distance(a: UnitVector, b: UnitVector) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    dx * dx + dy * dy + dz * dz
}

/// Iterator over open-addressing probe offsets.
///
/// Yields `(c, offset)` for `c = 0, 1, 2, …` where `offset` is `c` for linear
/// probing or `c·c` for quadratic probing.
pub fn probe_offsets(linear_probe: bool) -> impl Iterator<Item = (u64, u64)> {
    (0u64..).map(move |c| {
        let offset = if linear_probe { c } else { c * c };
        (c, offset)
    })
}

/// Compute the sequence of table indices visited when probing from `hash_index`.
///
/// Stops after `max_probes` iterations. The returned indices wrap modulo
/// `table_size`.
pub fn probe_sequence(
    hash_index: usize,
    table_size: usize,
    linear_probe: bool,
    max_probes: usize,
) -> Vec<usize> {
    let mut indices = Vec::with_capacity(max_probes);
    for (_, offset) in probe_offsets(linear_probe).take(max_probes) {
        let i = ((hash_index as u64 + offset) % table_size as u64) as usize;
        indices.push(i);
    }
    indices
}

/// Insert a pattern into a hash table using open addressing.
///
/// Returns the index of the first empty slot where the pattern was inserted.
/// The caller supplies an `is_empty` predicate that identifies unoccupied rows.
///
/// The probe sequence is bounded by `table.len()`; if the table is completely
/// occupied, the function panics. This matches the reference behaviour where a
/// full table is an unrecoverable error.
///
/// # Panics
///
/// Panics if every slot in the table is occupied.
pub fn insert_at_index<T>(
    pattern: T,
    hash_index: usize,
    table: &mut [T],
    linear_probe: bool,
    is_empty: impl Fn(&T) -> bool,
) -> usize
where
    T: Copy,
{
    let table_size = table.len();
    assert!(table_size > 0, "hash table must be non-empty");
    for (_, offset) in probe_offsets(linear_probe).take(table_size) {
        let i = ((hash_index as u64 + offset) % table_size as u64) as usize;
        if is_empty(&table[i]) {
            table[i] = pattern;
            return i;
        }
    }
    panic!("hash table is full")
}

/// Look up all occupied slots in a probe chain up to the first empty slot.
///
/// Returns the indices of occupied slots in probe order, stopping at the first
/// empty slot. The probe sequence is bounded by `table.len()` so a table that
/// contains no empty slot returns every index once.
pub fn get_table_indices_from_hash<T>(
    hash_index: usize,
    table: &[T],
    linear_probe: bool,
    is_empty: impl Fn(&T) -> bool,
) -> Vec<usize> {
    let table_size = table.len();
    assert!(table_size > 0, "hash table must be non-empty");
    let mut found = Vec::new();
    for (_, offset) in probe_offsets(linear_probe).take(table_size) {
        let i = ((hash_index as u64 + offset) % table_size as u64) as usize;
        if is_empty(&table[i]) {
            break;
        }
        found.push(i);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UnitVector;

    #[test]
    fn bins_follow_max_error_formula() {
        assert_eq!(pattern_bins(0.001), 250);
        assert_eq!(pattern_bins(0.005), 50);
    }

    #[test]
    fn key_is_rotation_and_scale_invariant() {
        // Build a simple asymmetric tetrahedron in the camera frame.
        let base = [
            UnitVector::from_radec(0.0, 0.0),
            UnitVector::from_radec(0.02, 0.0),
            UnitVector::from_radec(0.01, 0.015),
            UnitVector::from_radec(0.005, 0.025),
        ];
        let bins = pattern_bins(0.001);
        let (key_base, _) = pattern_key(&base, bins);

        // Rotate the pattern about the z-axis by 0.3 rad.
        let c = 0.3_f64.cos();
        let s = 0.3_f64.sin();
        let rotated: [UnitVector; PATTERN_SIZE] = base
            .iter()
            .map(|v| {
                UnitVector {
                    x: c * v.x - s * v.y,
                    y: s * v.x + c * v.y,
                    z: v.z,
                }
                .normalize()
                .expect("rotation preserves norm")
            })
            .collect::<Vec<_>>()
            .try_into()
            .expect("four rotated vectors");
        let (key_rotated, _) = pattern_key(&rotated, bins);
        assert_eq!(key_base, key_rotated, "rotation changed the pattern key");

        // Scale the pattern by moving all stars closer to the boresight while
        // preserving relative angles. We simulate scale invariance by reprojecting
        // the same angular configuration at a smaller camera scale.
        let scaled: [UnitVector; PATTERN_SIZE] = base
            .iter()
            .map(|v| {
                // Shrink the transverse components; renormalize.
                UnitVector {
                    x: 1.0,
                    y: v.y * 0.5,
                    z: v.z * 0.5,
                }
                .normalize()
                .expect("non-zero scaled vector")
            })
            .collect::<Vec<_>>()
            .try_into()
            .expect("four scaled vectors");
        let (key_scaled, largest_scaled) = pattern_key(&scaled, bins);
        assert_eq!(key_base, key_scaled, "scale changed the pattern key");
        // The largest edge should be smaller for the scaled pattern.
        let (_, largest_base) = pattern_key(&base, bins);
        assert!(largest_scaled < largest_base);
    }

    #[test]
    fn distinct_keys_get_distinct_64_bit_codes() {
        let bins = 250u32;
        let a = [1u32, 2, 3, 4, 5];
        let b = [5u32, 4, 3, 2, 1];
        assert_ne!(pattern_key_hash(&a, bins), pattern_key_hash(&b, bins));
    }

    #[test]
    fn magic_constant_value() {
        assert_eq!(MAGIC_RAND, 2654435761);
    }

    #[test]
    fn quadratic_index_uses_magic_constant() {
        let hash = pattern_key_hash(&[1, 2, 3, 4, 5], 250);
        let table_size = 10007usize;
        let idx = pattern_key_hash_index(hash, table_size, false);
        let expected = hash
            .wrapping_mul(MAGIC_RAND)
            .wrapping_rem(table_size as u64) as usize;
        assert_eq!(idx, expected);
    }

    #[test]
    fn linear_index_is_modulo_table_size() {
        let hash = pattern_key_hash(&[1, 2, 3, 4, 5], 250);
        let table_size = 10007usize;
        let idx = pattern_key_hash_index(hash, table_size, true);
        assert_eq!(idx, (hash % table_size as u64) as usize);
    }

    #[test]
    fn lookup_returns_full_probe_chain() {
        let mut table: [u32; 11] = [0; 11];
        let hash_index = 5usize;

        // Insert three patterns that collide into the same quadratic chain.
        let patterns = [101u32, 102u32, 103u32];
        for &pat in &patterns {
            let i = insert_at_index(pat, hash_index, &mut table, false, |row| *row == 0);
            assert!(table[i] == pat);
        }

        let found = get_table_indices_from_hash(hash_index, &table, false, |row| *row == 0);
        assert_eq!(found.len(), 3);
        assert_eq!(table[found[0]], 101);
        assert_eq!(table[found[1]], 102);
        assert_eq!(table[found[2]], 103);
    }

    #[test]
    fn insertion_mirrors_lookup_ordering() {
        let mut table: [u32; 7] = [0; 7];
        let hash_index = 3usize;

        let slot = insert_at_index(42u32, hash_index, &mut table, true, |row| *row == 0);
        let found = get_table_indices_from_hash(hash_index, &table, true, |row| *row == 0);
        assert_eq!(found, vec![slot]);
        assert_eq!(table[slot], 42);
    }

    #[test]
    fn mismatched_16_bit_hash_is_discarded() {
        let key = [1u32, 2, 3, 4, 5];
        let bins = 250u32;
        let hash = pattern_key_hash(&key, bins);
        let stored = pattern_key_hash16(hash);
        // Construct a different hash whose low 16 bits differ.
        let other_hash = hash.wrapping_add(1);
        let query = pattern_key_hash16(other_hash);
        assert_ne!(stored, query);
    }

    #[test]
    fn centroid_ordering_is_deterministic() {
        // Four stars where each has a unique distance from the centroid.
        let v = [
            UnitVector::from_radec(0.0, 0.0),
            UnitVector::from_radec(0.03, 0.0),
            UnitVector::from_radec(0.01, 0.02),
            UnitVector::from_radec(0.02, 0.03),
        ];
        let order = order_pattern_by_centroid_distance(&v);
        // The centroid is near the middle; verify the ordering is stable and
        // that applying it twice to a rotated copy yields the same correspondence.
        let c = 0.5_f64.cos();
        let s = 0.5_f64.sin();
        let rotated: [UnitVector; PATTERN_SIZE] = v
            .iter()
            .map(|vec| {
                UnitVector {
                    x: c * vec.x - s * vec.y,
                    y: s * vec.x + c * vec.y,
                    z: vec.z,
                }
                .normalize()
                .unwrap()
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let order_rotated = order_pattern_by_centroid_distance(&rotated);
        assert_eq!(
            order, order_rotated,
            "centroid ordering not rotation-invariant"
        );
    }

    #[test]
    fn probe_offsets_match_spec() {
        let linear: Vec<_> = probe_offsets(true).take(4).collect();
        assert_eq!(linear, vec![(0, 0), (1, 1), (2, 2), (3, 3)]);

        let quadratic: Vec<_> = probe_offsets(false).take(4).collect();
        assert_eq!(quadratic, vec![(0, 0), (1, 1), (2, 4), (3, 9)]);
    }

    #[test]
    fn probe_sequence_wraps_modulo_table_size() {
        let seq = probe_sequence(9, 11, false, 4);
        // Quadratic offsets: 0, 1, 4, 9 -> (9+offset) mod 11 = 9, 10, 2, 7.
        assert_eq!(seq, vec![9, 10, 2, 7]);
    }

    #[test]
    fn empty_lookup_returns_nothing() {
        let table: [u32; 5] = [0; 5];
        let found = get_table_indices_from_hash(2, &table, true, |row| *row == 0);
        assert!(found.is_empty());
    }

    #[test]
    fn lookup_stops_at_first_empty_slot() {
        let mut table: [u32; 7] = [0; 7];
        table[3] = 1;
        table[4] = 2;
        table[5] = 0; // empty slot terminates chain
        table[6] = 3; // should not be reached
        let found = get_table_indices_from_hash(3, &table, true, |row| *row == 0);
        assert_eq!(found, vec![3, 4]);
    }

    #[test]
    fn hash_overflow_wraps_intentionally() {
        // Choose a key and bin factor that overflow u64.
        let bins = 1_000_000_000u32;
        let key = [u32::MAX; 5];
        let _hash = pattern_key_hash(&key, bins);
        // The function must not panic; wrapping arithmetic is intentional.
    }
}
