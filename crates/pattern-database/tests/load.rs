use pattern_database::{load_from_path, load_mmap, CatalogId};

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn loads_minimal_fixture() {
    let db = load_from_path(&fixture("minimal.npz")).expect("load minimal.npz");

    assert_eq!(db.num_stars, 4);
    assert_eq!(db.star_table.len(), 4 * 6);
    assert_eq!(db.pattern_catalog, vec![[0, 1, 2, 3]]);
    assert_eq!(db.pattern_key_hashes, vec![43981]);
    assert!((db.pattern_largest_edge[0] - 3141.59).abs() < 1.0);
    assert_eq!(
        db.star_catalog_ids,
        vec![
            CatalogId::Hip(1),
            CatalogId::Hip(2),
            CatalogId::Hip(3),
            CatalogId::Hip(4),
        ]
    );

    let props = db.properties;
    assert_eq!(props.pattern_mode, "edge_ratio");
    assert_eq!(props.hash_table_type, "linear_probe");
    assert_eq!(props.star_catalog, "hip_main");
    assert_eq!(props.pattern_bins, 250);
    assert_eq!(props.max_fov, 30.0);
    assert_eq!(props.min_fov, 30.0);
    assert_eq!(props.verification_stars_per_fov, 150);
    assert_eq!(props.star_max_magnitude, 7.0);
    assert_eq!(props.num_patterns, 1);
}

#[test]
fn legacy_fallbacks_are_applied() {
    // Missing num_patterns derived: pattern_catalog.shape[0] // 2.
    // Missing min_fov defaults to max_fov.
    // verification_stars_per_fov <- catalog_stars_per_fov (legacy field name).
    // star_max_magnitude <- star_min_magnitude (legacy field name).
    let db = load_from_path(&fixture("legacy_fallbacks.npz")).expect("load legacy_fallbacks.npz");

    let props = &db.properties;
    assert_eq!(props.min_fov, props.max_fov);
    assert_eq!(props.num_patterns, 1); // pattern_catalog.shape[0] (2) // 2
    assert_eq!(props.verification_stars_per_fov, 200);
    assert_eq!(props.star_max_magnitude, 6.5);

    // The on-disk all-zero row is the "unoccupied slot" sentinel; the loader must
    // translate it to the usize::MAX convention lookup.rs's is_empty predicate expects.
    assert_eq!(db.pattern_catalog[0], [0, 1, 2, 3]);
    assert_eq!(db.pattern_catalog[1], [usize::MAX; 4]);
}

#[test]
fn mmap_load_matches_in_memory_load() {
    let path = fixture("minimal.npz");
    let ram = load_from_path(&path).expect("load via read");
    let mapped = load_mmap(&path).expect("load via mmap");

    assert_eq!(ram.star_table, mapped.star_table);
    assert_eq!(ram.pattern_catalog, mapped.pattern_catalog);
    assert_eq!(ram.pattern_key_hashes, mapped.pattern_key_hashes);
    assert_eq!(ram.star_catalog_ids, mapped.star_catalog_ids);
    assert_eq!(ram.properties, mapped.properties);
}
