//! Integration tests for hash table building, serialization, and round-trip.

use std::collections::HashSet;

use ps_core::pattern::{compute_pattern_key, order_by_centroid_distance};
use ps_dbgen::hash_insert::build_hash_table;
use ps_dbgen::prime::{is_prime, next_prime};

/// Helper to create a unit vector at (ra, dec) in radians.
fn vec_at(ra: f64, dec: f64) -> [f32; 3] {
    let cos_dec = dec.cos();
    [
        (ra.cos() * cos_dec) as f32,
        (ra.sin() * cos_dec) as f32,
        dec.sin() as f32,
    ]
}

// =============================================================================
// Prime utility tests
// =============================================================================

#[test]
fn test_is_prime_known_values() {
    assert!(!is_prime(0));
    assert!(!is_prime(1));
    assert!(is_prime(2));
    assert!(is_prime(3));
    assert!(!is_prime(4));
    assert!(is_prime(5));
    assert!(!is_prime(9));
    assert!(is_prime(17));
    assert!(!is_prime(100));
    assert!(is_prime(7919)); // largest 4-digit prime
    assert!(!is_prime(7920));
}

#[test]
fn test_next_prime_basic() {
    assert_eq!(next_prime(0), 2);
    assert_eq!(next_prime(1), 2);
    assert_eq!(next_prime(2), 3);
    assert_eq!(next_prime(14), 17);
    assert_eq!(next_prime(17), 19); // prime input -> next prime after it
    assert_eq!(next_prime(100), 101);
}

#[test]
fn test_next_prime_is_always_prime() {
    for n in 0..200u64 {
        let p = next_prime(n);
        assert!(is_prime(p), "next_prime({}) = {} is not prime", n, p);
        assert!(p > n, "next_prime({}) = {} should be > {}", n, p, n);
    }
}

// =============================================================================
// Hash table build tests
// =============================================================================

/// Build a small synthetic set of stars and patterns for testing.
fn make_test_data() -> (Vec<[f32; 3]>, HashSet<[usize; 4]>) {
    // 10 stars spread across a region, in brightness order
    let vectors: Vec<[f32; 3]> = (0..10)
        .map(|i| {
            let ra = i as f64 * 0.5;
            let dec = 0.2 * (i as f64 % 5.0);
            vec_at(ra, dec)
        })
        .collect();

    // Create a few patterns manually (indices into vectors)
    let mut patterns = HashSet::new();
    patterns.insert([0, 1, 2, 3]);
    patterns.insert([1, 2, 3, 4]);
    patterns.insert([2, 3, 4, 5]);
    patterns.insert([0, 2, 4, 6]);
    patterns.insert([3, 5, 7, 9]);
    patterns.insert([0, 1, 5, 8]);

    (vectors, patterns)
}

#[test]
fn test_build_hash_table_quadratic_basic() {
    let (vectors, patterns) = make_test_data();
    let pattern_bins = 250u32;

    let result = build_hash_table(&patterns, &vectors, pattern_bins, false); // quadratic

    assert_eq!(result.num_patterns as usize, patterns.len());
    assert!(
        result.catalog_u8.is_some() || result.catalog_u16.is_some() || result.catalog_u32.is_some()
    );

    // Table size should be next_prime(2*N) for quadratic
    let expected_size = next_prime(2 * patterns.len() as u64) as usize;
    assert_eq!(result.key_hashes.len(), expected_size);
    assert_eq!(result.largest_edge.len(), expected_size);

    if let Some(ref cat) = result.catalog_u8 {
        assert_eq!(cat.len(), expected_size);
    }
}

#[test]
fn test_build_hash_table_linear_basic() {
    let (vectors, patterns) = make_test_data();
    let pattern_bins = 250u32;

    let result = build_hash_table(&patterns, &vectors, pattern_bins, true); // linear

    assert_eq!(result.num_patterns as usize, patterns.len());

    // Table size should be next_prime(3*N) for linear
    let expected_size = next_prime(3 * patterns.len() as u64) as usize;
    assert_eq!(result.key_hashes.len(), expected_size);
}

#[test]
fn test_build_hash_table_element_type_u8() {
    // All indices <= 255 -> u8 catalog
    let (vectors, patterns) = make_test_data();
    let result = build_hash_table(&patterns, &vectors, 250u32, false);

    assert!(result.catalog_u8.is_some(), "should use u8 catalog");
    assert!(result.catalog_u16.is_none(), "should not use u16 catalog");
    assert!(result.catalog_u32.is_none(), "should not use u32 catalog");
}

// =============================================================================
// Empty slot handling tests
// =============================================================================

#[test]
fn test_empty_slot_sentinels() {
    let (vectors, patterns) = make_test_data();
    let result = build_hash_table(&patterns, &vectors, 250u32, false);

    let table_size = result.key_hashes.len();

    // Count non-empty slots
    let mut filled_count = 0;
    for slot in 0..table_size {
        let is_empty = result.key_hashes[slot] == 0 && result.largest_edge[slot].to_bits() == 0;
        if !is_empty {
            filled_count += 1;
            // Non-empty slots should have non-zero key_hashes (usually)
            // and non-zero largest_edge
            assert!(
                result.largest_edge[slot].to_bits() != 0,
                "filled slot {} has zero largest_edge",
                slot
            );
        } else {
            // Empty slots: catalog should have MAX sentinel
            match (&result.catalog_u8, &result.catalog_u16, &result.catalog_u32) {
                (Some(cat), None, None) => {
                    assert_eq!(
                        cat[slot],
                        [u8::MAX; 4],
                        "empty slot {} catalog not MAX",
                        slot
                    );
                }
                (None, Some(cat), None) => {
                    assert_eq!(
                        cat[slot],
                        [u16::MAX; 4],
                        "empty slot {} catalog not MAX",
                        slot
                    );
                }
                (None, None, Some(cat)) => {
                    assert_eq!(
                        cat[slot],
                        [u32::MAX; 4],
                        "empty slot {} catalog not MAX",
                        slot
                    );
                }
                _ => panic!("exactly one catalog variant must be Some"),
            }
        }
    }

    assert_eq!(filled_count, result.num_patterns as usize);
}

// =============================================================================
// Determinism test
// =============================================================================

#[test]
fn test_build_hash_table_determinism() {
    let (vectors, patterns) = make_test_data();
    let pattern_bins = 250u32;

    let result1 = build_hash_table(&patterns, &vectors, pattern_bins, false);
    let result2 = build_hash_table(&patterns, &vectors, pattern_bins, false);

    // key_hashes should be identical
    assert_eq!(
        result1.key_hashes, result2.key_hashes,
        "key_hashes should be deterministic"
    );

    // largest_edge should be identical (compare bits for exact float match)
    for (a, b) in result1.largest_edge.iter().zip(result2.largest_edge.iter()) {
        assert_eq!(
            a.to_bits(),
            b.to_bits(),
            "largest_edge should be deterministic"
        );
    }

    // catalog should be identical
    match (&result1.catalog_u8, &result2.catalog_u8) {
        (Some(c1), Some(c2)) => assert_eq!(c1, c2, "catalog_u8 should be deterministic"),
        _ => {}
    }
}

// =============================================================================
// Serialization round-trip test
// =============================================================================

#[test]
fn test_hash_table_roundtrip_save_load() {
    let (vectors, patterns) = make_test_data();
    let pattern_bins = 250u32;

    let result = build_hash_table(&patterns, &vectors, pattern_bins, false);

    // Build a Database from the result
    let num_stars = vectors.len();
    let mut star_table = Vec::with_capacity(num_stars);
    for i in 0..num_stars {
        star_table.push([
            0.0, // ra placeholder
            0.0, // dec placeholder
            vectors[i][0],
            vectors[i][1],
            vectors[i][2],
            (i as f32) * 0.5, // magnitude placeholder
        ]);
    }

    let properties = ps_db::DatabaseProperties {
        pattern_mode: "edge_ratio".into(),
        hash_table_type: "quadratic_probe".into(),
        pattern_size: 4,
        pattern_bins: pattern_bins as u16,
        pattern_max_error: 0.001,
        max_fov: 30.0,
        min_fov: 10.0,
        star_catalog: "test".into(),
        epoch_equinox: 2000,
        epoch_proper_motion: 2000.0,
        lattice_field_oversampling: 100,
        patterns_per_lattice_field: 50,
        verification_stars_per_fov: 30,
        star_max_magnitude: 10.0,
        presort_patterns: true,
        num_patterns: result.num_patterns,
    };

    let mut db = ps_db::Database::empty(properties);
    db.star_table = star_table;
    db.pattern_catalog_u8 = result.catalog_u8.clone();
    db.pattern_catalog_u16 = result.catalog_u16.clone();
    db.pattern_catalog_u32 = result.catalog_u32.clone();
    db.largest_edge = result.largest_edge.clone();
    db.key_hashes = result.key_hashes.clone();

    // Save to temp file
    let tmp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let path = tmp_dir.path().join("test_db.bin");

    ps_db::loader::save_native(&db, &path).expect("failed to save database");

    // Load back
    let db_loaded = ps_db::loader::load_native(&path).expect("failed to load database");

    // Verify key_hashes match
    assert_eq!(
        db.key_hashes, db_loaded.key_hashes,
        "key_hashes should match after round-trip"
    );

    // Verify largest_edge matches (compare bits)
    for (i, (a, b)) in db
        .largest_edge
        .iter()
        .zip(db_loaded.largest_edge.iter())
        .enumerate()
    {
        assert_eq!(
            a.to_bits(),
            b.to_bits(),
            "largest_edge[{}] mismatch: {:016x} vs {:016x}",
            i,
            a.to_bits(),
            b.to_bits()
        );
    }

    // Verify catalog matches
    match (&db.pattern_catalog_u8, &db_loaded.pattern_catalog_u8) {
        (Some(c1), Some(c2)) => assert_eq!(c1, c2, "catalog_u8 should match after round-trip"),
        _ => {}
    }

    // Verify star_table matches
    assert_eq!(db.star_table.len(), db_loaded.star_table.len());
    for (i, (a, b)) in db
        .star_table
        .iter()
        .zip(db_loaded.star_table.iter())
        .enumerate()
    {
        for j in 0..6 {
            assert_eq!(
                a[j], b[j],
                "star_table[{}][{}] mismatch: {} vs {}",
                i, j, a[j], b[j]
            );
        }
    }

    // Verify properties match
    assert_eq!(
        db.properties.pattern_bins,
        db_loaded.properties.pattern_bins
    );
    assert_eq!(
        db.properties.num_patterns,
        db_loaded.properties.num_patterns
    );
    assert_eq!(
        db.properties.hash_table_type,
        db_loaded.properties.hash_table_type
    );
}

// =============================================================================
// Lookup verification test
// =============================================================================

#[test]
fn test_lookup_finds_inserted_patterns() {
    let (vectors, patterns) = make_test_data();
    let pattern_bins = 250u32;

    let result = build_hash_table(&patterns, &vectors, pattern_bins, false);

    // Build a Database
    let num_stars = vectors.len();
    let mut star_table = Vec::with_capacity(num_stars);
    for i in 0..num_stars {
        star_table.push([
            0.0,
            0.0,
            vectors[i][0],
            vectors[i][1],
            vectors[i][2],
            (i as f32) * 0.5,
        ]);
    }

    let properties = ps_db::DatabaseProperties {
        pattern_mode: "edge_ratio".into(),
        hash_table_type: "quadratic_probe".into(),
        pattern_size: 4,
        pattern_bins: pattern_bins as u16,
        pattern_max_error: 0.001,
        max_fov: 30.0,
        min_fov: 10.0,
        star_catalog: "test".into(),
        epoch_equinox: 2000,
        epoch_proper_motion: 2000.0,
        lattice_field_oversampling: 100,
        patterns_per_lattice_field: 50,
        verification_stars_per_fov: 30,
        star_max_magnitude: 10.0,
        presort_patterns: true,
        num_patterns: result.num_patterns,
    };

    let mut db = ps_db::Database::empty(properties);
    db.star_table = star_table;
    db.pattern_catalog_u8 = result.catalog_u8.clone();
    db.pattern_catalog_u16 = result.catalog_u16.clone();
    db.pattern_catalog_u32 = result.catalog_u32.clone();
    db.largest_edge = result.largest_edge.clone();
    db.key_hashes = result.key_hashes.clone();

    // For each original pattern, compute its key and verify lookup finds at least one candidate
    let mut found_count = 0;
    for pattern in &patterns {
        let vectors_f64: [[f64; 3]; 4] = pattern.map(|idx| {
            let v = vectors[idx];
            [v[0] as f64, v[1] as f64, v[2] as f64]
        });

        let (key, largest_edge_rad) = compute_pattern_key(&vectors_f64, pattern_bins);

        // Lookup without FOV filter
        let candidates = ps_db::lookup_pattern(&db, &key, largest_edge_rad, None);
        assert!(
            !candidates.is_empty(),
            "lookup should find candidates for pattern {:?}",
            pattern
        );

        // Verify that at least one candidate slot contains our pattern (after centroid reordering)
        let ordered = order_by_centroid_distance(&vectors_f64);
        let sorted_pattern: [usize; 4] = [
            pattern[ordered[0]],
            pattern[ordered[1]],
            pattern[ordered[2]],
            pattern[ordered[3]],
        ];

        let mut found_in_slot = false;
        for &slot in &candidates {
            if let Some(ref cat) = db.pattern_catalog_u8 {
                let slot_pattern: [usize; 4] = cat[slot].map(|v| v as usize);
                if slot_pattern == sorted_pattern.map(|v| v as usize) {
                    found_in_slot = true;
                    break;
                }
            }
        }
        assert!(
            found_in_slot,
            "pattern {:?} not found in any candidate slot",
            pattern
        );
        found_count += 1;
    }

    assert_eq!(
        found_count,
        patterns.len(),
        "all patterns should be findable"
    );
}

// =============================================================================
// Byte-identical determinism test (full serialization)
// =============================================================================

#[test]
fn test_serialization_byte_identical() {
    let (vectors, patterns) = make_test_data();
    let pattern_bins = 250u32;

    fn build_and_serialize(
        vectors: &[[f32; 3]],
        patterns: &HashSet<[usize; 4]>,
        pattern_bins: u32,
    ) -> Vec<u8> {
        let result = build_hash_table(patterns, vectors, pattern_bins, false);

        let num_stars = vectors.len();
        let mut star_table = Vec::with_capacity(num_stars);
        for i in 0..num_stars {
            star_table.push([
                0.0,
                0.0,
                vectors[i][0],
                vectors[i][1],
                vectors[i][2],
                (i as f32) * 0.5,
            ]);
        }

        let properties = ps_db::DatabaseProperties {
            pattern_mode: "edge_ratio".into(),
            hash_table_type: "quadratic_probe".into(),
            pattern_size: 4,
            pattern_bins: pattern_bins as u16,
            pattern_max_error: 0.001,
            max_fov: 30.0,
            min_fov: 10.0,
            star_catalog: "test".into(),
            epoch_equinox: 2000,
            epoch_proper_motion: 2000.0,
            lattice_field_oversampling: 100,
            patterns_per_lattice_field: 50,
            verification_stars_per_fov: 30,
            star_max_magnitude: 10.0,
            presort_patterns: true,
            num_patterns: result.num_patterns,
        };

        let mut db = ps_db::Database::empty(properties);
        db.star_table = star_table;
        db.pattern_catalog_u8 = result.catalog_u8;
        db.pattern_catalog_u16 = result.catalog_u16;
        db.pattern_catalog_u32 = result.catalog_u32;
        db.largest_edge = result.largest_edge;
        db.key_hashes = result.key_hashes;

        // Serialize to temp file for byte comparison
        let tmp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let path1 = tmp_dir.path().join("db1.bin");
        ps_db::loader::save_native(&db, &path1).expect("failed to save db1");
        std::fs::read(&path1).expect("failed to read db1")
    }

    let bytes1 = build_and_serialize(&vectors, &patterns, pattern_bins);
    let bytes2 = build_and_serialize(&vectors, &patterns, pattern_bins);

    assert_eq!(
        bytes1, bytes2,
        "serialized output should be byte-identical across runs"
    );
}
