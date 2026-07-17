//! Round-trip test for the `.npz` serialization format.
//!
//! Generates a tiny pattern database, writes it to disk with
//! `database_generation::write_database`, and reads it back with
//! `pattern_database::load_from_path`. All arrays and properties must match.

use database_generation::catalog::{CatalogEntry, CatalogId};
use database_generation::pattern_catalog::build_pattern_catalog;
use database_generation::serialize::write_database;
use pattern_database::{load_from_path, DatabaseProperties};
use std::path::PathBuf;

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

fn make_properties(num_patterns: u32) -> DatabaseProperties {
    DatabaseProperties {
        pattern_mode: "edge_ratio".to_string(),
        hash_table_type: "quadratic_probe".to_string(),
        pattern_size: 4,
        pattern_bins: 250,
        pattern_max_error: 0.001,
        max_fov: 30.0,
        min_fov: 10.0,
        star_catalog: "hip_main".to_string(),
        epoch_equinox: 2000,
        epoch_proper_motion: 2026.0,
        verification_stars_per_fov: 150,
        star_max_magnitude: 7.0,
        num_patterns,
    }
}

#[test]
fn round_trip_preserves_arrays_and_properties() {
    // A small catalog with two patterns, enough to exercise empty slots.
    let entries = vec![
        entry(0.0, 0.0, 1.0, 1),
        entry(1.0, 0.0, 2.0, 2),
        entry(0.0, 1.0, 3.0, 3),
        entry(1.0, 1.0, 4.0, 4),
        entry(2.0, 0.0, 5.0, 5),
    ];
    let patterns = vec![[0, 1, 2, 3], [0, 1, 2, 4]];

    let catalog = build_pattern_catalog(&entries, &patterns, 0.001, false);
    let properties = make_properties(catalog.num_patterns as u32);

    let mut path = PathBuf::from(std::env::temp_dir());
    path.push("dbgen_round_trip_test.npz");

    write_database(&path, &catalog, &entries, &properties).unwrap();

    let loaded = load_from_path(&path).unwrap();

    assert_eq!(loaded.num_stars, entries.len());

    // star_table: compare the written f32 values exactly.
    let mut expected_star_table = Vec::with_capacity(entries.len() * 6);
    for entry in &entries {
        let v = math_core::UnitVector::from_radec(entry.ra, entry.dec);
        expected_star_table.push(entry.ra as f32);
        expected_star_table.push(entry.dec as f32);
        expected_star_table.push(v.x as f32);
        expected_star_table.push(v.y as f32);
        expected_star_table.push(v.z as f32);
        expected_star_table.push(entry.mag as f32);
    }
    assert_eq!(loaded.star_table, expected_star_table);

    // pattern_catalog: empty slots are recognized by zero largest-edge.
    assert_eq!(loaded.pattern_catalog.len(), catalog.table_size);
    for slot in 0..catalog.table_size {
        if catalog.pattern_largest_edge[slot] == 0.0 {
            assert_eq!(loaded.pattern_catalog[slot], [usize::MAX; 4]);
        } else {
            assert_eq!(loaded.pattern_catalog[slot], catalog.pattern_catalog[slot]);
        }
    }

    // pattern_largest_edge: f16 round-trip may introduce tiny differences.
    assert_eq!(loaded.pattern_largest_edge.len(), catalog.table_size);
    for slot in 0..catalog.table_size {
        let expected = catalog.pattern_largest_edge[slot];
        let actual = loaded.pattern_largest_edge[slot];
        assert!(
            (actual - expected).abs() < 0.02,
            "largest_edge mismatch at slot {}: {} vs {}",
            slot,
            actual,
            expected
        );
    }

    // pattern_key_hashes: exact match for occupied slots; zero for empty.
    assert_eq!(loaded.pattern_key_hashes.len(), catalog.table_size);
    for slot in 0..catalog.table_size {
        if catalog.pattern_largest_edge[slot] == 0.0 {
            assert_eq!(loaded.pattern_key_hashes[slot], 0);
        } else {
            assert_eq!(loaded.pattern_key_hashes[slot], catalog.pattern_key_hashes[slot]);
        }
    }

    // star_catalog_IDs: all Hip in this test.
    assert_eq!(loaded.star_catalog_ids.len(), entries.len());
    for (i, entry) in entries.iter().enumerate() {
        match entry.id {
            CatalogId::Hip(h) => assert_eq!(loaded.star_catalog_ids[i], pattern_database::CatalogId::Hip(h)),
            _ => panic!("unexpected catalog id variety"),
        }
    }

    // properties: exact match.
    assert_eq!(loaded.properties, properties);

    std::fs::remove_file(&path).ok();
}
