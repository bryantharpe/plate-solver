use ps_db::importer;
use ps_db::loader;

fn npz_path() -> std::path::PathBuf {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../reference-solutions/cedar-solve/tetra3/data/default_database.npz")
}

#[test]
fn test_import_npz_shapes() {
    let db = importer::import_npz(&npz_path()).unwrap();
    assert_eq!(db.num_stars(), 42212);
    assert_eq!(db.num_slots(), 3032963);
    // 42212 < 65534 -> u16 catalog
    assert!(db.pattern_catalog_u16.is_some());
    assert_eq!(db.pattern_catalog_u16.as_ref().unwrap().len(), 3032963);
    assert_eq!(db.largest_edge.len(), 3032963);
    assert_eq!(db.key_hashes.len(), 3032963);
    assert!(db.star_catalog_ids_u32.is_some());
    assert_eq!(db.star_catalog_ids_u32.as_ref().unwrap().len(), 42212);
}

#[test]
fn test_import_npz_properties() {
    let db = importer::import_npz(&npz_path()).unwrap();
    let p = &db.properties;
    assert_eq!(p.pattern_mode, "edge_ratio");
    assert_eq!(p.hash_table_type, "linear_probe");
    assert_eq!(p.pattern_size, 4);
    assert_eq!(p.pattern_bins, 250);
    assert_eq!(p.max_fov, 30.0f32);
    assert_eq!(p.min_fov, 10.0f32);
    assert_eq!(p.star_catalog, "hip_main");
    assert_eq!(p.epoch_equinox, 2000u16);
    assert_eq!(p.lattice_field_oversampling, 100u16);
    assert_eq!(p.patterns_per_lattice_field, 40u16);
    assert_eq!(p.verification_stars_per_fov, 150u16);
    assert_eq!(p.num_patterns, 1010981u32);
    assert_eq!(p.presort_patterns, true);
}

#[test]
fn test_roundtrip_native() {
    let db = importer::import_npz(&npz_path()).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    loader::save_native(&db, tmp.path()).unwrap();
    let db2 = loader::load_native(tmp.path()).unwrap();
    assert_eq!(db2.num_stars(), 42212);
    assert_eq!(db2.num_slots(), 3032963);
    assert_eq!(db2.properties.num_patterns, 1010981u32);
    assert_eq!(db2.properties.hash_table_type, "linear_probe");
    assert!(db2.pattern_catalog_u16.is_some());
    // Spot-check first valid slot
    let cat = db.pattern_catalog_u16.as_ref().unwrap();
    let cat2 = db2.pattern_catalog_u16.as_ref().unwrap();
    assert_eq!(cat[0], cat2[0]); // first valid pattern preserved
}
