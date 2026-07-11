//! The zero-copy mmap accessors reinterpret raw bytes as typed slices, so a
//! misaligned offset would be UB. They must report misalignment as `Err`
//! rather than relying on a `debug_assert` that vanishes in release builds.

use ps_db::{importer, loader};

fn npz_path() -> std::path::PathBuf {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../reference-solutions/cedar-solve/tetra3/data/default_database.npz")
}

/// import_npz -> save_native -> load_native_mmap, the path every caller takes.
fn roundtrip_mmap() -> ps_db::MmappedDatabase {
    let db = importer::import_npz(&npz_path()).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    loader::save_native(&db, tmp.path()).unwrap();
    ps_db::load_native_mmap(tmp.path()).unwrap()
}

#[test]
fn mmap_key_hashes_returns_result_not_panic() {
    // The native format does not guarantee u16 alignment for key_hashes (a
    // catalog elem_size of 1 can produce an odd offset), so this may be Err.
    // The invariant under test is that it *returns* either way — never panics.
    let db_mmap = roundtrip_mmap();
    match db_mmap.key_hashes() {
        Err(e) => assert!(e.to_string().contains("not u16-aligned")),
        Ok(kh) => assert!(!kh.is_empty()),
    }
}

#[test]
fn mmap_largest_edge_returns_result_not_panic() {
    let db_mmap = roundtrip_mmap();
    match db_mmap.largest_edge() {
        Err(e) => assert!(e.to_string().contains("not f16-aligned")),
        Ok(le) => assert!(!le.is_empty()),
    }
}

#[test]
fn mmap_star_table_aligned_returns_ok() {
    // A normal roundtrip always lands star_table on a 4-aligned offset.
    let db_mmap = roundtrip_mmap();
    let st = db_mmap.star_table();
    assert!(st.is_ok(), "expected Ok, got: {:?}", st.err());
    assert_eq!(st.unwrap().len(), db_mmap.num_stars());
}

#[test]
fn mmap_star_table_misaligned_returns_err() {
    let mut db_mmap = roundtrip_mmap();
    // The OS-provided mmap base is page-aligned (4096), so forcing offset=1
    // puts the pointer at base+1 → align_offset(4) == 3. count=0 keeps the
    // slice bounds valid (&data[1..1] is an in-bounds empty range), isolating
    // the alignment check as the only thing that can fail.
    db_mmap.set_star_table_layout_for_test(1, 0);
    let result = db_mmap.star_table();
    assert!(result.is_err(), "expected Err for misaligned offset, got Ok");
    assert!(
        result.unwrap_err().to_string().contains("not [f32;6]-aligned"),
        "unexpected error message"
    );
}
