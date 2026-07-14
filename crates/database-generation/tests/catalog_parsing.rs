use database_generation::catalog::{parse_bsc5, parse_hip, parse_tyc, CatalogId};
use database_generation::cleanup::{clean_and_limit, derive_magnitude_limit};
use database_generation::config::current_year;
use database_generation::num_fields::num_fields_for_sky;
use database_generation::proper_motion::{propagate, HIP_TYC_PM_ORIGIN};
use std::io::Cursor;

fn bsc5_header(starn: i32) -> Vec<u8> {
    let mut header = vec![0u8; 28];
    header[4..8].copy_from_slice(&starn.to_le_bytes());
    header
}

fn bsc5_entry(id: u32, ra: f64, dec: f64, mag_raw: i32, pm_ra: i32, pm_dec: i32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(32);
    buf.extend_from_slice(&id.to_le_bytes());
    buf.extend_from_slice(&ra.to_le_bytes());
    buf.extend_from_slice(&dec.to_le_bytes());
    buf.extend_from_slice(&mag_raw.to_le_bytes());
    buf.extend_from_slice(&pm_ra.to_le_bytes());
    buf.extend_from_slice(&pm_dec.to_le_bytes());
    buf
}

#[test]
fn bsc5_equinox_from_negative_starn() {
    let mut data = bsc5_header(-2);
    data.extend_from_slice(&bsc5_entry(1, 0.1, 0.2, 250, 0, 0));
    data.extend_from_slice(&bsc5_entry(2, 0.3, 0.4, 300, 0, 0));

    let entries = parse_bsc5(Cursor::new(&data)).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].id, CatalogId::Bsc(1));
    assert!((entries[0].ra - 0.1).abs() < 1e-12);
    assert!((entries[0].dec - 0.2).abs() < 1e-12);
    assert!((entries[0].mag - 2.5).abs() < 1e-9);
}

#[test]
fn bsc5_equinox_from_positive_starn() {
    let mut data = bsc5_header(1);
    data.extend_from_slice(&bsc5_entry(10, 1.0, 0.5, 100, 0, 0));

    let entries = parse_bsc5(Cursor::new(&data)).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, CatalogId::Bsc(10));
}

#[test]
fn hip_degrees_converted_to_radians() {
    let text = "# comment\n1|0.0|0.0|0|0|5.0\n2|90.0|45.0|0|0|4.0\n";
    let entries = parse_hip(Cursor::new(text)).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].id, CatalogId::Hip(1));
    assert!((entries[1].ra - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    assert!((entries[1].dec - std::f64::consts::FRAC_PI_4).abs() < 1e-12);
}

#[test]
fn hip_skips_empty_rows() {
    let text = "1| | |0|0|5.0\n2|10.0|20.0|0|0|6.0\n";
    let entries = parse_hip(Cursor::new(text)).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, CatalogId::Hip(2));
}

#[test]
fn tyc_parses_three_part_id() {
    let text = "123|456|7|10.0|-20.0|0|0|7.5\n";
    let entries = parse_tyc(Cursor::new(text)).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, CatalogId::Tyc(123, 456, 7));
    assert!((entries[0].ra - 10.0f64.to_radians()).abs() < 1e-12);
    assert!((entries[0].dec - (-20.0f64).to_radians()).abs() < 1e-12);
}

#[test]
fn proper_motion_pole_guard() {
    let mut entries = vec![
        database_generation::catalog::CatalogEntry {
            ra: 0.0,
            dec: 89.0f64.to_radians(),
            mag: 1.0,
            id: CatalogId::Hip(1),
            pm_ra: Some(100.0),
            pm_dec: Some(50.0),
        },
    ];
    propagate(&mut entries, HIP_TYC_PM_ORIGIN, HIP_TYC_PM_ORIGIN + 10.0);
    assert!((entries[0].ra - 0.0).abs() < 1e-12);
    assert!((entries[0].dec - 89.0f64.to_radians()).abs() < 1e-12);
}

#[test]
fn proper_motion_propagates_non_pole() {
    let mut entries = vec![
        database_generation::catalog::CatalogEntry {
            ra: 0.0,
            dec: 0.0,
            mag: 1.0,
            id: CatalogId::Hip(1),
            pm_ra: Some(1000.0), // mas/year
            pm_dec: Some(500.0),
        },
    ];
    propagate(&mut entries, HIP_TYC_PM_ORIGIN, HIP_TYC_PM_ORIGIN + 1.0);
    let mas_to_rad = std::f64::consts::PI / (180.0 * 3600.0 * 1000.0);
    assert!((entries[0].ra - 1000.0 * 1.0 * mas_to_rad).abs() < 1e-12);
    assert!((entries[0].dec - 500.0 * 1.0 * mas_to_rad).abs() < 1e-12);
}

#[test]
fn cleanup_drops_zero_zero_and_sorts() {
    use database_generation::catalog::CatalogEntry;
    let mut entries = vec![
        CatalogEntry { ra: 0.0, dec: 0.0, mag: 1.0, id: CatalogId::Hip(1), pm_ra: None, pm_dec: None },
        CatalogEntry { ra: 1.0, dec: 0.5, mag: 3.0, id: CatalogId::Hip(2), pm_ra: None, pm_dec: None },
        CatalogEntry { ra: 2.0, dec: 0.5, mag: 2.0, id: CatalogId::Hip(3), pm_ra: None, pm_dec: None },
    ];
    clean_and_limit(&mut entries, None);
    assert_eq!(entries.len(), 2);
    assert!((entries[0].mag - 2.0).abs() < 1e-9);
    assert!((entries[1].mag - 3.0).abs() < 1e-9);
}

#[test]
fn derive_auto_magnitude_limit() {
    use database_generation::catalog::CatalogEntry;
    let mut entries: Vec<CatalogEntry> = (0..10)
        .map(|i| CatalogEntry {
            ra: i as f64 * 0.1,
            dec: 0.0,
            mag: i as f64,
            id: CatalogId::Hip(i as u32),
            pm_ra: None,
            pm_dec: None,
        })
        .collect();
    entries.sort_by(|a, b| a.mag.partial_cmp(&b.mag).unwrap());
    let limit = derive_magnitude_limit(&entries, 5.5).unwrap();
    assert!((limit - 5.0).abs() < 1e-9);
}

#[test]
fn num_fields_for_sky_is_positive() {
    let n = num_fields_for_sky(1.0);
    assert!(n > 0.0);
    // A 1-degree FOV covers a tiny fraction of the sky; expect many fields.
    assert!(n > 1000.0);
}

#[test]
fn current_year_is_reasonable() {
    let year = current_year();
    assert!(year > 2020.0);
    assert!(year < 2100.0);
}
