//! H3 tests — key_hashes() and largest_edge() return Result, not panic.

use ps_db::{importer, loader};

fn npz_path() -> std::path::PathBuf {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../reference-solutions/cedar-solve/tetra3/data/default_database.npz")
}

#[test]
fn test_mmap_key_hashes_returns_result() {
    // Roundtrip: import_npz -> save_native -> load_native_mmap.
    // The native format does NOT guarantee u16 alignment for key_hashes data
    // (catalog elem_size=1 causes odd offsets), so key_hashes() returns Err.
    // The important invariant: it returns Err (not panic).
    let db = importer::import_npz(&npz_path()).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    loader::save_native(&db, tmp.path()).unwrap();

    let db_mmap = ps_db::load_native_mmap(tmp.path()).unwrap();
    let kh = db_mmap.key_hashes();
    // key_hashes() must not panic — it returns Result.
    // Whether Ok or Err depends on the actual alignment of the file layout.
    if kh.is_err() {
        assert!(kh.unwrap_err().contains("not u16-aligned"));
    } else {
        assert_eq!(kh.unwrap().len(), 3032963);
    }
}

#[test]
fn test_mmap_largest_edge_returns_result() {
    // Same roundtrip: largest_edge() must return Result, not panic.
    let db = importer::import_npz(&npz_path()).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    loader::save_native(&db, tmp.path()).unwrap();

    let db_mmap = ps_db::load_native_mmap(tmp.path()).unwrap();
    let le = db_mmap.largest_edge();
    // largest_edge() must not panic — it returns Result.
    if le.is_err() {
        assert!(le.unwrap_err().contains("not f16-aligned"));
    } else {
        assert_eq!(le.unwrap().len(), 3032963);
    }
}

#[test]
fn test_mmap_star_table_aligned_returns_ok() {
    // Normal roundtrip always produces a 4-aligned offset → Ok.
    let db = importer::import_npz(&npz_path()).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    loader::save_native(&db, tmp.path()).unwrap();

    let db_mmap = ps_db::load_native_mmap(tmp.path()).unwrap();
    let st = db_mmap.star_table();
    // Normal layout is always 4-aligned → Ok.
    assert!(st.is_ok(), "expected Ok, got: {:?}", st.err());
    assert_eq!(st.unwrap().len(), db_mmap.num_stars());
}

#[test]
fn test_mmap_star_table_misaligned_returns_err() {
    // Load any valid database so we have a live Mmap backing store.
    let db = importer::import_npz(&npz_path()).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    loader::save_native(&db, tmp.path()).unwrap();

    let mut db_mmap = ps_db::load_native_mmap(tmp.path()).unwrap();
    // The OS-provided mmap base is page-aligned (4096).
    // Setting offset=1, count=0 makes ptr = mmap_base+1 → align_offset(4)=3.
    // count=0 makes end=1+0=1, so &data[1..1] is in-bounds (empty slice).
    db_mmap.set_star_table_layout_for_test(1, 0);
    let result = db_mmap.star_table();
    assert!(result.is_err(), "expected Err for misaligned offset, got Ok");
    assert!(
        result.unwrap_err().contains("not [f32;6]-aligned"),
        "unexpected error message"
    );
}
