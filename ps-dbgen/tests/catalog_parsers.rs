use byteorder::{LittleEndian, WriteBytesExt};
use ps_dbgen::catalog::bsc5::parse_bsc5;
use ps_dbgen::catalog::hip::parse_hip;
use ps_dbgen::catalog::tyc::parse_tyc;
use ps_dbgen::catalog::{CatalogId, ParseParams};
use std::io::Cursor;

/* ------------------------------------------------------------------ */
/*  BSC5 helpers                                                       */
/* ------------------------------------------------------------------ */

fn build_bsc5_fixture(stars: &[(f32, f64, f64, i16, f32, f32)], j2000: bool) -> Vec<u8> {
    let mut buf = Vec::new();
    let n = stars.len() as i32;
    // STARN negative = J2000
    let starn = if j2000 { -(n) } else { n };
    buf.write_i32::<LittleEndian>(0).unwrap(); // STAR0
    buf.write_i32::<LittleEndian>(0).unwrap(); // STAR1
    buf.write_i32::<LittleEndian>(starn).unwrap(); // STARN
    buf.write_i32::<LittleEndian>(1).unwrap(); // STNUM
    buf.write_i32::<LittleEndian>(1).unwrap(); // MPROP
    buf.write_i32::<LittleEndian>(1).unwrap(); // NMAG
    buf.write_i32::<LittleEndian>(32).unwrap(); // NBENT
    for (id, ra, dec, mag, ra_pm, dec_pm) in stars {
        buf.write_f32::<LittleEndian>(*id).unwrap();
        buf.write_f64::<LittleEndian>(*ra).unwrap();
        buf.write_f64::<LittleEndian>(*dec).unwrap();
        buf.write_i16::<LittleEndian>(0).unwrap(); // type unused
        buf.write_i16::<LittleEndian>(*mag).unwrap();
        buf.write_f32::<LittleEndian>(*ra_pm).unwrap();
        buf.write_f32::<LittleEndian>(*dec_pm).unwrap();
    }
    buf
}

#[test]
fn test_bsc5_parse_basic() {
    // Two stars, J2000 equinox (negative STARN), no PM to propagate.
    let stars = [
        (
            1.0_f32,
            std::f64::consts::FRAC_PI_2,
            0.5_f64,
            345,
            0.0_f32,
            0.0_f32,
        ),
        (2.0_f32, 1.0_f64, -0.3_f64, 510, 0.0_f32, 0.0_f32),
    ];
    let buf = build_bsc5_fixture(&stars, true /* j2000 */);
    let mut cursor = Cursor::new(buf);
    let params = ParseParams {
        epoch_proper_motion: 2000.0,
    };
    let records = parse_bsc5(&mut cursor, &params).expect("parse should succeed");

    // Both stars have non-zero positions, so no drops.
    assert_eq!(records.len(), 2);

    // Sorted by mag ascending: 3.45 (id=1) before 5.10 (id=2)
    assert!((records[0].mag - 3.45).abs() < 1e-9);
    assert_eq!(records[0].cat_id, CatalogId::Bsc(1));
    assert!((records[0].ra - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    assert!((records[0].dec - 0.5).abs() < 1e-9);

    assert!((records[1].mag - 5.10).abs() < 1e-9);
    assert_eq!(records[1].cat_id, CatalogId::Bsc(2));
    assert!((records[1].ra - 1.0).abs() < 1e-9);
    assert!((records[1].dec - (-0.3)).abs() < 1e-9);
}

#[test]
fn test_bsc5_equinox_sign_rule() {
    // B1950 equinox (positive STARN): pm_origin = 1950.0
    // With params.epoch_proper_motion = 2000.0, dt = 50 years.
    // Use a star with non-zero PM to verify propagation.
    let stars = [
        // id=10, ra=1.0, dec=0.5 rad, mag=400 (4.00), pm_ra=0.001 rad/yr, pm_dec=0.0005 rad/yr
        (10.0_f32, 1.0_f64, 0.5_f64, 400, 0.001_f32, 0.0005_f32),
    ];
    let buf = build_bsc5_fixture(&stars, false /* b1950 */);
    let mut cursor = Cursor::new(buf);
    let params = ParseParams {
        epoch_proper_motion: 2000.0,
    };
    let records = parse_bsc5(&mut cursor, &params).expect("parse should succeed");

    assert_eq!(records.len(), 1);

    // cos(0.5) > 0.05, so PM applies:
    // mu_alpha = 0.001 / cos(0.5)
    // ra_final = 1.0 + mu_alpha * (2000.0 - 1950.0) = 1.0 + 0.001/cos(0.5)*50
    let cos_d = (0.5_f64).cos();
    let mu_alpha = 0.001_f64 / cos_d;
    let expected_ra = 1.0 + mu_alpha * 50.0;
    let expected_dec = 0.5 + 0.0005_f64 * 50.0;

    assert!(
        (records[0].ra - expected_ra).abs() < 1e-6,
        "ra {} vs expected {}",
        records[0].ra,
        expected_ra
    );
    assert!(
        (records[0].dec - expected_dec).abs() < 1e-6,
        "dec {} vs expected {}",
        records[0].dec,
        expected_dec
    );
}

#[test]
fn test_bsc5_drops_zero_position() {
    // One star with RA=0, Dec=0 (should be dropped), one valid star.
    let stars = [
        (99.0_f32, 0.0_f64, 0.0_f64, 100, 0.0_f32, 0.0_f32),
        (5.0_f32, 0.7_f64, 0.3_f64, 200, 0.0_f32, 0.0_f32),
    ];
    let buf = build_bsc5_fixture(&stars, true);
    let mut cursor = Cursor::new(buf);
    let params = ParseParams {
        epoch_proper_motion: 2000.0,
    };
    let records = parse_bsc5(&mut cursor, &params).expect("parse should succeed");

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].cat_id, CatalogId::Bsc(5));
}

#[test]
fn test_bsc5_pole_guard() {
    // Star near the pole: dec close to pi/2 => cos(delta) ~ 0 < 0.05 => PM zeroed.
    let stars = [(
        7.0_f32,
        0.5_f64,
        std::f64::consts::FRAC_PI_2 - 0.01_f64,
        300,
        100.0_f32,
        100.0_f32,
    )];
    let buf = build_bsc5_fixture(&stars, true);
    let mut cursor = Cursor::new(buf);
    let params = ParseParams {
        epoch_proper_motion: 2000.0,
    };
    let records = parse_bsc5(&mut cursor, &params).expect("parse should succeed");

    assert_eq!(records.len(), 1);
    // PM should be zeroed because cos(delta) < 0.05
    // (FRAC_PI_2 - 0.01).cos() is very small (~0.01), definitely < 0.05
    assert!(
        (records[0].ra - 0.5).abs() < 1e-9,
        "RA should be unchanged near pole"
    );
    assert!(
        (records[0].dec - (std::f64::consts::FRAC_PI_2 - 0.01)).abs() < 1e-9,
        "Dec should be unchanged near pole"
    );
}

/* ------------------------------------------------------------------ */
/*  HIP tests                                                          */
/* ------------------------------------------------------------------ */

#[test]
fn test_hip_parse_basic() {
    let data = include_str!("fixtures/hip_sample.dat");
    let cursor = Cursor::new(data.as_bytes());
    let params = ParseParams {
        epoch_proper_motion: 2000.0,
    };
    let records = parse_hip(cursor, &params).expect("parse should succeed");

    // Row 3 has empty mag -> skipped. Only 2 valid records.
    assert_eq!(records.len(), 2);

    // Sorted by mag ascending: 3.45 before 5.10
    assert!((records[0].mag - 3.45).abs() < 1e-6);
    assert_eq!(records[0].cat_id, CatalogId::Hip(1));

    // RA = 90 deg = pi/2, Dec = 45 deg = pi/4 (with PM propagation from 1991.25 to 2000.0)
    // dt = 8.75 years
    // pm_ra = 10 mas/yr = 10/1000/3600 deg/yr, pm_dec = 5 mas/yr
    let dt = 2000.0 - 1991.25; // 8.75
    let pm_ra_deg = 10.0 / 1000.0 / 3600.0;
    let pm_dec_deg = 5.0 / 1000.0 / 3600.0;
    let delta_0 = 45.0_f64;
    let cos_delta = delta_0.to_radians().cos();
    let mu_alpha = pm_ra_deg / cos_delta;
    let ra_expected = (90.0 + mu_alpha * dt).to_radians();
    let dec_expected = (delta_0 + pm_dec_deg * dt).to_radians();

    assert!(
        (records[0].ra - ra_expected).abs() < 1e-6,
        "HIP ra {} vs expected {}",
        records[0].ra,
        ra_expected
    );
    assert!(
        (records[0].dec - dec_expected).abs() < 1e-6,
        "HIP dec {} vs expected {}",
        records[0].dec,
        dec_expected
    );

    // Second star
    assert!((records[1].mag - 5.10).abs() < 1e-6);
    assert_eq!(records[1].cat_id, CatalogId::Hip(2));
}

/* ------------------------------------------------------------------ */
/*  TYC tests                                                          */
/* ------------------------------------------------------------------ */

#[test]
fn test_tyc_parse_basic() {
    let data = include_str!("fixtures/tyc_sample.dat");
    let cursor = Cursor::new(data.as_bytes());
    let params = ParseParams {
        epoch_proper_motion: 2000.0,
    };
    let records = parse_tyc(cursor, &params).expect("parse should succeed");

    assert_eq!(records.len(), 2);

    // Sorted by mag: 4.50 before 6.20
    assert!((records[0].mag - 4.50).abs() < 1e-6);
    assert_eq!(records[0].cat_id, CatalogId::Tyc([1, 2, 3]));

    assert!((records[1].mag - 6.20).abs() < 1e-6);
    assert_eq!(records[1].cat_id, CatalogId::Tyc([4, 5, 6]));
}
