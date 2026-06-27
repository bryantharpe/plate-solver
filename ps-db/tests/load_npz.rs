use ps_db::importer;
use ps_db::loader;
#[cfg(feature = "kd-tree")]
use ps_db::nearby_stars;

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

#[cfg(feature = "kd-tree")]
#[test]
fn test_nearby_stars() {
    let db_path = npz_path();
    let mut db = importer::import_npz(&db_path).unwrap();
    db.build_kd_tree();

    let fixture_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/nearby_stars.json");
    let fixture: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&fixture_path).unwrap()).unwrap();

    let queries = fixture["queries"].as_array().expect("queries should be an array");
    for query in queries {
        let label = query["label"].as_str().expect("missing label");
        let q_vec: Vec<f64> = query["query_vector"]
            .as_array()
            .expect(&format!("{}: missing query_vector", label))
            .iter()
            .map(|v| v.as_f64().expect("not f64"))
            .collect();
        assert_eq!(q_vec.len(), 3, "{}: query_vector should have 3 elements", label);
        let vector = [q_vec[0] as f32, q_vec[1] as f32, q_vec[2] as f32];

        let radius = query["radius_rad"]
            .as_f64()
            .expect(&format!("{}: missing radius_rad", label)) as f32;

        let expected_num = query["num_nearby"]
            .as_u64()
            .expect(&format!("{}: missing num_nearby", label)) as usize;

        let expected_indices: Vec<usize> = query["indices"]
            .as_array()
            .expect(&format!("{}: missing indices", label))
            .iter()
            .map(|v| v.as_u64().expect("not u64") as usize)
            .collect();

        let result = nearby_stars(&db, &vector, radius);

        assert_eq!(
            result.len(),
            expected_num,
            "{}: expected {} nearby stars, got {}",
            label,
            expected_num,
            result.len()
        );

        assert_eq!(
            result, expected_indices,
            "{}: indices mismatch",
            label
        );
    }
}

#[test]
fn test_hash_lookup_parity() {
    let fixture_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/hash_lookup.json");
    let fixture: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&fixture_path).unwrap()).unwrap();

    let db = importer::import_npz(&npz_path()).unwrap();

    for entry in fixture.as_array().expect("fixture should be an array") {
        let key: [u32; 5] = entry["pattern_key"]
            .as_array()
            .expect("missing pattern_key")
            .iter()
            .map(|v| v.as_u64().expect("not u64") as u32)
            .collect::<Vec<_>>()
            .try_into()
            .expect("pattern_key should have 5 elements");

        let largest_edge_rad: f64 = entry["largest_edge_rad"]
            .as_f64()
            .expect("missing largest_edge_rad");

        let expected_no_fov: Vec<usize> = entry["candidates_no_fov"]
            .as_array()
            .expect("missing candidates_no_fov")
            .iter()
            .map(|v| v.as_u64().expect("not u64") as usize)
            .collect();

        let expected_with_fov: Vec<usize> = entry["candidates_with_fov"]
            .as_array()
            .expect("missing candidates_with_fov")
            .iter()
            .map(|v| v.as_u64().expect("not u64") as usize)
            .collect();

        let fov_estimate_rad: f64 = entry["fov_estimate_rad"]
            .as_f64()
            .expect("missing fov_estimate_rad");

        // Test without FOV filter (coarse_fov_rad = None)
        let got_no_fov = ps_db::lookup_pattern(&db, &key, largest_edge_rad, None);
        assert_eq!(
            got_no_fov, expected_no_fov,
            "slot {}: candidates_no_fov mismatch: expected {:?}, got {:?}",
            entry["slot"], expected_no_fov, got_no_fov
        );

        // Test with FOV filter (coarse_fov_rad = Some(fov_estimate))
        let got_with_fov = ps_db::lookup_pattern(&db, &key, largest_edge_rad, Some(fov_estimate_rad));
        assert_eq!(
            got_with_fov, expected_with_fov,
            "slot {}: candidates_with_fov mismatch: expected {:?}, got {:?}",
            entry["slot"], expected_with_fov, got_with_fov
        );
    }
}
