use pattern_database::{load_from_path, CatalogId};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures");
    path.push(name);
    path
}

#[test]
fn minimal_fixture_loads() {
    let db = load_from_path(&fixture("minimal.npz")).unwrap();
    assert_eq!(db.num_stars, 4);
    assert_eq!(db.star_table.len(), 4 * 6);
    assert_eq!(db.pattern_catalog.len(), 1);
    assert_eq!(db.pattern_catalog[0], [0, 1, 2, 3]);
    assert!(db.pattern_largest_edge[0] > 0.0);
    assert_eq!(db.pattern_key_hashes.len(), 1);
    assert_eq!(db.star_catalog_ids.len(), 4);
    assert_eq!(db.star_catalog_ids[0], CatalogId::Hip(1));
    assert_eq!(db.properties.pattern_mode, "edge_ratio");
    assert_eq!(db.properties.hash_table_type, "linear_probe");
}

#[test]
fn u16_dtype_loads() {
    let db = load_from_path(&fixture("u16.npz")).unwrap();
    assert_eq!(db.num_stars, 2);
    assert_eq!(db.pattern_catalog.len(), 1);
    assert_eq!(db.pattern_catalog[0], [0, 1, 0, 1]);
    assert_eq!(db.properties.hash_table_type, "quadratic_probe");
}

#[test]
fn u32_dtype_loads() {
    let db = load_from_path(&fixture("u32.npz")).unwrap();
    assert_eq!(db.num_stars, 2);
    assert_eq!(db.pattern_catalog.len(), 1);
    assert_eq!(db.pattern_catalog[0], [0, 1, 0, 1]);
}

#[test]
fn s64_string_fields_load() {
    let db = load_from_path(&fixture("s64.npz")).unwrap();
    assert_eq!(db.properties.star_catalog, "hip_main");
    assert_eq!(db.properties.hash_table_type, "quadratic_probe");
}

#[test]
fn empty_slots_are_reconstructed() {
    let db = load_from_path(&fixture("empty_slots.npz")).unwrap();
    assert_eq!(db.pattern_catalog.len(), 3);
    assert_eq!(db.pattern_catalog[0], [0, 1, 2, 3]);
    assert_eq!(db.pattern_catalog[1], [usize::MAX; 4]);
    assert_eq!(db.pattern_catalog[2], [usize::MAX; 4]);
}

#[test]
fn tycho_ids_load_as_triples() {
    let db = load_from_path(&fixture("tycho.npz")).unwrap();
    assert_eq!(db.num_stars, 4);
    assert_eq!(db.star_catalog_ids[0], CatalogId::Tyc(1, 2, 3));
    assert_eq!(db.star_catalog_ids[3], CatalogId::Tyc(10, 11, 12));
    assert_eq!(db.properties.star_catalog, "tyc_main");
}

#[test]
fn star_vector_and_radec_accessors_work() {
    let db = load_from_path(&fixture("minimal.npz")).unwrap();
    let v = db.star_vector(pattern_database::StarId(0)).unwrap();
    assert!((v.x - 1.0).abs() < 1e-6);
    let (ra, _dec, mag) = db.star_radec_mag(pattern_database::StarId(0)).unwrap();
    assert!((ra - 0.0).abs() < 1e-6);
    assert!((mag - 0.0).abs() < 1e-6);
}
