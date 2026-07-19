//! Round-trip test for the pattern-database serializer.
//!
//! Generates a tiny pattern catalog, writes it to disk with
//! `database_generation::serialize_to_path`, loads it back with
//! `pattern_database::load_from_path`, and asserts every array and property
//! matches the source data.

use std::path::PathBuf;

use database_generation::{build_pattern_catalog, serialize_to_path, CatalogEntry, CatalogId};
use pattern_database::{CatalogId as LoadedCatalogId, DatabaseProperties};

fn make_entry(ra_deg: f64, dec_deg: f64, mag: f64, id: u32, _index: usize) -> CatalogEntry {
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
fn hip_catalog_round_trips() {
    let entries = vec![
        make_entry(0.0, 0.0, 1.0, 1001, 0),
        make_entry(1.0, 0.0, 2.0, 1002, 1),
        make_entry(0.0, 1.0, 3.0, 1003, 2),
        make_entry(1.0, 1.0, 4.0, 1004, 3),
        make_entry(2.0, 0.5, 5.0, 1005, 4),
    ];

    let patterns = vec![[0usize, 1, 2, 3], [1, 2, 3, 4]];

    let catalog = build_pattern_catalog(&entries, &patterns, 0.001, false);

    let properties = DatabaseProperties {
        pattern_mode: "edge_ratio".to_string(),
        // The on-disk field is |S14, so a 15-character string is truncated by
        // NumPy to 14 bytes. Use a value that fits for a true round-trip.
        hash_table_type: "linear_probe".to_string(),
        pattern_size: 4,
        pattern_bins: catalog.pattern_bins as u16,
        pattern_max_error: 0.001,
        max_fov: 30.0,
        min_fov: 0.5,
        star_catalog: "hip_main".to_string(),
        epoch_equinox: 2000,
        epoch_proper_motion: 2026.0,
        verification_stars_per_fov: 150,
        star_max_magnitude: 7.0,
        num_patterns: catalog.num_patterns as u32,
    };

    let dir = tempfile::tempdir().unwrap();
    let path: PathBuf = dir.path().join("roundtrip.npz");

    serialize_to_path(&path, &entries, &catalog, &properties).unwrap();

    let loaded = pattern_database::load_from_path(&path).unwrap();

    assert_eq!(loaded.num_stars, entries.len());

    // Star table: RA, Dec, x, y, z, mag.
    for (i, entry) in entries.iter().enumerate() {
        let base = i * 6;
        let v = math_core::UnitVector::from_radec(entry.ra, entry.dec);
        assert!((loaded.star_table[base] - entry.ra as f32).abs() < 1e-6);
        assert!((loaded.star_table[base + 1] - entry.dec as f32).abs() < 1e-6);
        assert!((loaded.star_table[base + 2] - v.x as f32).abs() < 1e-6);
        assert!((loaded.star_table[base + 3] - v.y as f32).abs() < 1e-6);
        assert!((loaded.star_table[base + 4] - v.z as f32).abs() < 1e-6);
        assert!((loaded.star_table[base + 5] - entry.mag as f32).abs() < 1e-6);
    }

    // Pattern catalog: same length and matching occupied rows.
    assert_eq!(loaded.pattern_catalog.len(), catalog.pattern_catalog.len());
    for (loaded_row, source_row) in loaded
        .pattern_catalog
        .iter()
        .zip(catalog.pattern_catalog.iter())
    {
        assert_eq!(loaded_row, source_row);
    }

    // Largest edge and key hashes: f16 round trip is approximate for edge,
    // exact for hashes.
    assert_eq!(
        loaded.pattern_largest_edge.len(),
        catalog.pattern_largest_edge.len()
    );
    for (loaded_edge, source_edge) in loaded
        .pattern_largest_edge
        .iter()
        .zip(catalog.pattern_largest_edge.iter())
    {
        // f16 has ~3e-4 relative precision; absolute tolerance in milliradians
        // is generous enough for these small test angles.
        assert!((loaded_edge - source_edge).abs() < 1e-2);
    }

    assert_eq!(
        loaded.pattern_key_hashes.len(),
        catalog.pattern_key_hashes.len()
    );
    for (loaded_hash, source_hash) in loaded
        .pattern_key_hashes
        .iter()
        .zip(catalog.pattern_key_hashes.iter())
    {
        assert_eq!(loaded_hash, source_hash);
    }

    // Catalog IDs.
    assert_eq!(loaded.star_catalog_ids.len(), entries.len());
    for (loaded_id, entry) in loaded.star_catalog_ids.iter().zip(entries.iter()) {
        match (loaded_id, &entry.id) {
            (LoadedCatalogId::Hip(l), CatalogId::Hip(r)) => assert_eq!(*l, *r),
            _ => panic!("catalog ID mismatch"),
        }
    }

    // Properties.
    assert_eq!(loaded.properties.pattern_mode, properties.pattern_mode);
    assert_eq!(
        loaded.properties.hash_table_type,
        properties.hash_table_type
    );
    assert!(loaded.properties.linear_probe());
    assert_eq!(loaded.properties.pattern_size, properties.pattern_size);
    assert_eq!(loaded.properties.pattern_bins, properties.pattern_bins);
    assert!((loaded.properties.pattern_max_error - properties.pattern_max_error).abs() < 1e-6);
    assert!((loaded.properties.max_fov - properties.max_fov).abs() < 1e-6);
    assert!((loaded.properties.min_fov - properties.min_fov).abs() < 1e-6);
    assert_eq!(loaded.properties.star_catalog, properties.star_catalog);
    assert_eq!(loaded.properties.epoch_equinox, properties.epoch_equinox);
    assert!((loaded.properties.epoch_proper_motion - properties.epoch_proper_motion).abs() < 1e-6);
    assert_eq!(
        loaded.properties.verification_stars_per_fov,
        properties.verification_stars_per_fov
    );
    assert!((loaded.properties.star_max_magnitude - properties.star_max_magnitude).abs() < 1e-6);
    assert_eq!(loaded.properties.num_patterns, properties.num_patterns);
}

#[test]
fn bsc_catalog_round_trips() {
    let entries = vec![
        CatalogEntry {
            ra: 0.0,
            dec: 0.0,
            mag: 1.0,
            id: CatalogId::Bsc(1),
            pm_ra: None,
            pm_dec: None,
        },
        CatalogEntry {
            ra: 1.0f64.to_radians(),
            dec: 0.0,
            mag: 2.0,
            id: CatalogId::Bsc(2),
            pm_ra: None,
            pm_dec: None,
        },
        CatalogEntry {
            ra: 0.0,
            dec: 1.0f64.to_radians(),
            mag: 3.0,
            id: CatalogId::Bsc(3),
            pm_ra: None,
            pm_dec: None,
        },
        CatalogEntry {
            ra: 1.0f64.to_radians(),
            dec: 1.0f64.to_radians(),
            mag: 4.0,
            id: CatalogId::Bsc(4),
            pm_ra: None,
            pm_dec: None,
        },
    ];

    let patterns = vec![[0usize, 1, 2, 3]];
    let catalog = build_pattern_catalog(&entries, &patterns, 0.001, false);

    let properties = DatabaseProperties {
        pattern_mode: "edge_ratio".to_string(),
        hash_table_type: "linear_probe".to_string(),
        pattern_size: 4,
        pattern_bins: catalog.pattern_bins as u16,
        pattern_max_error: 0.001,
        max_fov: 20.0,
        min_fov: 1.0,
        star_catalog: "bsc5".to_string(),
        epoch_equinox: 2000,
        epoch_proper_motion: 2000.0,
        verification_stars_per_fov: 100,
        star_max_magnitude: 6.5,
        num_patterns: catalog.num_patterns as u32,
    };

    let dir = tempfile::tempdir().unwrap();
    let path: PathBuf = dir.path().join("bsc_roundtrip.npz");

    serialize_to_path(&path, &entries, &catalog, &properties).unwrap();

    let loaded = pattern_database::load_from_path(&path).unwrap();

    assert_eq!(loaded.star_catalog_ids.len(), entries.len());
    for (loaded_id, entry) in loaded.star_catalog_ids.iter().zip(entries.iter()) {
        match (loaded_id, &entry.id) {
            (LoadedCatalogId::Bsc(l), CatalogId::Bsc(r)) => assert_eq!(*l as u32, *r),
            _ => panic!("catalog ID mismatch"),
        }
    }

    assert!(loaded.properties.linear_probe());
}

#[test]
fn tyc_catalog_round_trips() {
    let entries = vec![
        CatalogEntry {
            ra: 0.0,
            dec: 0.0,
            mag: 1.0,
            id: CatalogId::Tyc(1, 1, 1),
            pm_ra: None,
            pm_dec: None,
        },
        CatalogEntry {
            ra: 1.0f64.to_radians(),
            dec: 0.0,
            mag: 2.0,
            id: CatalogId::Tyc(2, 2, 2),
            pm_ra: None,
            pm_dec: None,
        },
        CatalogEntry {
            ra: 0.0,
            dec: 1.0f64.to_radians(),
            mag: 3.0,
            id: CatalogId::Tyc(3, 3, 3),
            pm_ra: None,
            pm_dec: None,
        },
        CatalogEntry {
            ra: 1.0f64.to_radians(),
            dec: 1.0f64.to_radians(),
            mag: 4.0,
            id: CatalogId::Tyc(4, 4, 4),
            pm_ra: None,
            pm_dec: None,
        },
    ];

    let patterns = vec![[0usize, 1, 2, 3]];
    let catalog = build_pattern_catalog(&entries, &patterns, 0.001, false);

    let properties = DatabaseProperties {
        pattern_mode: "edge_ratio".to_string(),
        // Use a value that fits in the |S14 field so the round-trip is exact.
        hash_table_type: "linear_probe".to_string(),
        pattern_size: 4,
        pattern_bins: catalog.pattern_bins as u16,
        pattern_max_error: 0.001,
        max_fov: 15.0,
        min_fov: 1.0,
        star_catalog: "tyc_main".to_string(),
        epoch_equinox: 2000,
        epoch_proper_motion: 2000.0,
        verification_stars_per_fov: 120,
        star_max_magnitude: 8.0,
        num_patterns: catalog.num_patterns as u32,
    };

    let dir = tempfile::tempdir().unwrap();
    let path: PathBuf = dir.path().join("tyc_roundtrip.npz");

    serialize_to_path(&path, &entries, &catalog, &properties).unwrap();

    let loaded = pattern_database::load_from_path(&path).unwrap();

    assert_eq!(loaded.star_catalog_ids.len(), entries.len());
    for (loaded_id, entry) in loaded.star_catalog_ids.iter().zip(entries.iter()) {
        match (loaded_id, &entry.id) {
            (LoadedCatalogId::Tyc(l1, l2, l3), CatalogId::Tyc(r1, r2, r3)) => {
                assert_eq!(*l1 as u32, *r1);
                assert_eq!(*l2 as u32, *r2);
                assert_eq!(*l3 as u32, *r3);
            }
            _ => panic!("catalog ID mismatch"),
        }
    }
}
