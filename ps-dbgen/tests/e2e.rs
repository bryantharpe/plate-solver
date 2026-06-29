use std::io::Write;
use byteorder::WriteBytesExt;
use tempfile::{NamedTempFile, tempdir};

/// Build a valid BSC5 binary in memory.
///
/// Format (per ps-dbgen/src/catalog/bsc5.rs):
///   Header: 7 x i32 LE  (STAR0, STAR1, STARN, STNUM, MPROP, NMAG, NBENT)
///   Per-star: f32(id), f64(ra_rad), f64(dec_rad), i16(type), i16(mag_x100),
///             f32(pm_ra), f32(pm_dec)
///
/// STARN negative = J2000 equinox (no PM propagation when epoch_proper_motion=2000).
fn build_bsc5_fixture(stars: &[(f32, f64, f64, i16, f32, f32)]) -> Vec<u8> {
    let mut buf = Vec::new();
    let n = stars.len() as i32;
    let starn = -(n); // negative = J2000
    buf.write_i32::<byteorder::LittleEndian>(0).unwrap();  // STAR0
    buf.write_i32::<byteorder::LittleEndian>(0).unwrap();  // STAR1
    buf.write_i32::<byteorder::LittleEndian>(starn).unwrap(); // STARN (negative = J2000)
    buf.write_i32::<byteorder::LittleEndian>(1).unwrap();  // STNUM
    buf.write_i32::<byteorder::LittleEndian>(1).unwrap();  // MPROP
    buf.write_i32::<byteorder::LittleEndian>(1).unwrap();  // NMAG
    buf.write_i32::<byteorder::LittleEndian>(32).unwrap(); // NBENT
    for (id, ra, dec, mag, ra_pm, dec_pm) in stars {
        buf.write_f32::<byteorder::LittleEndian>(*id).unwrap();
        buf.write_f64::<byteorder::LittleEndian>(*ra).unwrap();
        buf.write_f64::<byteorder::LittleEndian>(*dec).unwrap();
        buf.write_i16::<byteorder::LittleEndian>(0).unwrap(); // type (unused)
        buf.write_i16::<byteorder::LittleEndian>(*mag).unwrap();
        buf.write_f32::<byteorder::LittleEndian>(*ra_pm).unwrap();
        buf.write_f32::<byteorder::LittleEndian>(*dec_pm).unwrap();
    }
    buf
}

#[test]
fn test_e2e_build_validate_and_determinism() {
    /* ------------------------------------------------------------------ */
    /* Step 1 — build a valid BSC5 binary with 6 stars near the NCP       */
    /* ------------------------------------------------------------------ */
    // Stars positioned near the north celestial pole, spread ~2 deg apart.
    // Coordinates: (id, ra_rad, dec_rad, mag_x100, pm_ra, pm_dec)
    // RA in hours converted to radians: h * 15 deg * pi/180 = h * pi/12
    // Dec in degrees converted to radians
    let stars: &[(f32, f64, f64, i16, f32, f32)] = &[
        (1.0, 0.0_f64 * std::f64::consts::PI / 12.0,   88.0_f64.to_radians(), 300, 0.0, 0.0),
        (2.0, 0.5_f64 * std::f64::consts::PI / 12.0,   86.5_f64.to_radians(), 350, 0.0, 0.0),
        (3.0, 1.0_f64 * std::f64::consts::PI / 12.0,   87.0_f64.to_radians(), 400, 0.0, 0.0),
        (4.0, 1.5_f64 * std::f64::consts::PI / 12.0,   85.5_f64.to_radians(), 450, 0.0, 0.0),
        (5.0, 2.0_f64 * std::f64::consts::PI / 12.0,   86.0_f64.to_radians(), 500, 0.0, 0.0),
        (6.0, 2.5_f64 * std::f64::consts::PI / 12.0,   87.5_f64.to_radians(), 550, 0.0, 0.0),
    ];

    let data = build_bsc5_fixture(stars);

    // Write to a temp file with "bsc5" in the name so the CLI detects the format.
    let catalog_tempfile = NamedTempFile::new().expect("create temp catalog");
    std::fs::write(&catalog_tempfile, &data).expect("write catalog bytes");

    // The CLI detects BSC5 by filename containing "bsc5" or ".bsc5" extension.
    // NamedTempFile won't have that, so we copy to a file with the right name.
    let tmp_dir = tempdir().expect("create temp dir");
    let catalog_path = tmp_dir.path().join("test.bsc5");
    std::fs::write(&catalog_path, &data).expect("write catalog to named file");

    /* ------------------------------------------------------------------ */
    /* Step 2 — run the CLI binary                                         */
    /* ------------------------------------------------------------------ */
    let bin = env!("CARGO_BIN_EXE_ps-dbgen");
    let out_path = tmp_dir.path().join("test.psdb");

    let status = std::process::Command::new(bin)
        .args([
            catalog_path.to_str().unwrap(),
            out_path.to_str().unwrap(),
            "--max-fov", "10",
        ])
        .status()
        .expect("ps-dbgen CLI should start");
    assert!(status.success(), "ps-dbgen CLI exited non-zero: {:?}", status);
    assert!(out_path.exists(), "output DB file must exist");

    /* ------------------------------------------------------------------ */
    /* Step 3 — load the DB and validate structure                         */
    /* ------------------------------------------------------------------ */
    let db = ps_db::loader::load_native(&out_path).expect("must load DB");
    let props = &db.properties;

    assert!(props.num_patterns > 0,
        "DB must contain at least 1 pattern; got 0 (pipeline may have failed)");
    assert!(props.max_fov > 0.0 && props.max_fov <= 10.5,
        "max_fov should be near 10, got {}", props.max_fov);

    /* ------------------------------------------------------------------ */
    /* Step 4 — answer a sample lookup                                     */
    /* ------------------------------------------------------------------ */
    // Re-parse the 6 stars' RA/Dec and compute unit vectors.
    let stars_radec: &[(f64, f64)] = &[
        (0.0_f64 * std::f64::consts::PI / 12.0,   88.0_f64.to_radians()),
        (0.5_f64 * std::f64::consts::PI / 12.0,   86.5_f64.to_radians()),
        (1.0_f64 * std::f64::consts::PI / 12.0,   87.0_f64.to_radians()),
        (1.5_f64 * std::f64::consts::PI / 12.0,   85.5_f64.to_radians()),
        (2.0_f64 * std::f64::consts::PI / 12.0,   86.0_f64.to_radians()),
        (2.5_f64 * std::f64::consts::PI / 12.0,   87.5_f64.to_radians()),
    ];

    // Convert to [f64; 3] unit vectors via radec_to_vector.
    let vecs: Vec<[f64; 3]> = stars_radec
        .iter()
        .map(|(ra, dec)| {
            let v = ps_core::celestial::radec_to_vector(*ra, *dec);
            [v.x, v.y, v.z]
        })
        .collect();

    // Take the first 4 stars as a candidate pattern; compute their key.
    let four_vecs: [[f64; 3]; 4] = [vecs[0], vecs[1], vecs[2], vecs[3]];
    let (key, largest_edge) = ps_core::pattern::compute_pattern_key(&four_vecs, props.pattern_bins as u32);

    // Lookup in the database.
    let candidates = ps_db::lookup_pattern(
        &db,
        &key,
        largest_edge,
        None, // no coarse FOV filter for this simple lookup
    );

    assert!(!candidates.is_empty(),
        "lookup_pattern must return at least 1 candidate for an inserted pattern");

    /* ------------------------------------------------------------------ */
    /* Step 5 — determinism check                                          */
    /* ------------------------------------------------------------------ */
    let out_path2 = tmp_dir.path().join("test2.psdb");
    let status2 = std::process::Command::new(bin)
        .args([
            catalog_path.to_str().unwrap(),
            out_path2.to_str().unwrap(),
            "--max-fov", "10",
        ])
        .status()
        .expect("ps-dbgen CLI should start (second run)");
    assert!(status2.success(), "second run should succeed");

    let bytes1 = std::fs::read(&out_path).unwrap();
    let bytes2 = std::fs::read(&out_path2).unwrap();
    assert_eq!(bytes1, bytes2, "two runs must produce byte-identical output");

    /* ------------------------------------------------------------------ */
    /* Step 6 — parity log                                                 */
    /* ------------------------------------------------------------------ */
    eprintln!(
        "PARITY NOTE: Generated {} patterns from {} BSC5 fixture stars with max_fov=10\u{b0}. \
         Full count-parity vs default_database.npz (HIP/TYC source) is DEFERRED — \
         source catalog not in-repo. Structural validity and determinism verified.",
        props.num_patterns,
        db.star_table.len(),
    );
}
