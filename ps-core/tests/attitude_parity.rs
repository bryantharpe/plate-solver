//! MC5 parity tests for Wahba/SVD attitude estimation and RA/Dec/Roll extraction.
//! Golden values captured from the reference into `tests/fixtures/rotation.json`
//! (via `tools/parity/capture_core.py`).

use nalgebra::{Matrix3, Vector3};
use ps_core::attitude::{extract_radec_roll, find_rotation_matrix, is_reflection};
use ps_core::celestial::vector_to_radec;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct Fixture {
    #[serde(rename = "R0")]
    r0: [[f64; 3]; 3],
    #[serde(rename = "R_recovered")]
    r_recovered: [[f64; 3]; 3],
    catalog_vectors: Vec<[f64; 3]>,
    image_vectors: Vec<[f64; 3]>,
    ra_deg: f64,
    dec_deg: f64,
    roll_deg: f64,
    reflection_catalog_vectors: Vec<[f64; 3]>,
    reflection_image_vectors: Vec<[f64; 3]>,
    #[serde(rename = "reflection_R")]
    reflection_r: [[f64; 3]; 3],
    reflection_det: f64,
}

fn m3_from_arr(m: &[[f64; 3]; 3]) -> Matrix3<f64> {
    Matrix3::new(
        m[0][0], m[0][1], m[0][2], m[1][0], m[1][1], m[1][2], m[2][0], m[2][1], m[2][2],
    )
}

fn vecs_from_arr(arr: &[[f64; 3]]) -> Vec<Vector3<f64>> {
    arr.iter().map(|&[x, y, z]| Vector3::new(x, y, z)).collect()
}

fn load() -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rotation.json");
    let body =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&body).expect("parse rotation.json")
}

#[test]
fn find_rotation_matrix_recovers_known_rotation() {
    let fixture = load();
    let image = vecs_from_arr(&fixture.image_vectors);
    let catalog = vecs_from_arr(&fixture.catalog_vectors);
    let r = find_rotation_matrix(&image, &catalog);

    // Every element matches R0 within 1e-9
    for i in 0..3 {
        for j in 0..3 {
            assert!(
                (r[(i, j)] - fixture.r0[i][j]).abs() < 1e-9,
                "R0[{},{}] mismatch: got {}, expected {}",
                i,
                j,
                r[(i, j)],
                fixture.r0[i][j]
            );
        }
    }

    // Proper rotation: det > 0 and not a reflection
    assert!(r.determinant() > 0.0, "det(R) = {}", r.determinant());
    assert!(!is_reflection(&r), "recovered R should not be a reflection");
}

#[test]
fn extract_radec_roll_matches_reference() {
    let fixture = load();
    let r = m3_from_arr(&fixture.r_recovered);
    let (ra, dec, roll) = extract_radec_roll(&r);

    let ra_deg = ra.to_degrees();
    let dec_deg = dec.to_degrees();
    let roll_deg = roll.to_degrees();

    assert!(
        (ra_deg - fixture.ra_deg).abs() < 1e-9,
        "RA mismatch: got {}, expected {}",
        ra_deg,
        fixture.ra_deg
    );
    assert!(
        (dec_deg - fixture.dec_deg).abs() < 1e-9,
        "Dec mismatch: got {}, expected {}",
        dec_deg,
        fixture.dec_deg
    );
    assert!(
        (roll_deg - fixture.roll_deg).abs() < 1e-9,
        "Roll mismatch: got {}, expected {}",
        roll_deg,
        fixture.roll_deg
    );
}

#[test]
fn extract_boresight_matches_vector_to_radec() {
    let fixture = load();
    let r = m3_from_arr(&fixture.r_recovered);
    let (ra, dec, _) = extract_radec_roll(&r);

    // Row 0 of R is the boresight in celestial frame
    let boresight = Vector3::new(r[(0, 0)], r[(0, 1)], r[(0, 2)]);
    let (ra2, dec2) = vector_to_radec(&boresight);

    assert!(
        (ra - ra2).abs() < 1e-12,
        "boresight RA mismatch: got {}, expected {}",
        ra,
        ra2
    );
    assert!(
        (dec - dec2).abs() < 1e-12,
        "boresight Dec mismatch: got {}, expected {}",
        dec,
        dec2
    );
}

#[test]
fn reflection_is_rejected() {
    let fixture = load();
    let refl_image = vecs_from_arr(&fixture.reflection_image_vectors);
    let refl_catalog = vecs_from_arr(&fixture.reflection_catalog_vectors);
    let r = find_rotation_matrix(&refl_image, &refl_catalog);

    // Must be detected as a reflection
    assert!(
        is_reflection(&r),
        "reflection R should be detected as reflection"
    );
    assert!(r.determinant() < 0.0, "det(R) = {}", r.determinant());

    // Matches the reference's reflection_R within 1e-9
    for i in 0..3 {
        for j in 0..3 {
            assert!(
                (r[(i, j)] - fixture.reflection_r[i][j]).abs() < 1e-9,
                "reflection_R[{},{}] mismatch: got {}, expected {}",
                i,
                j,
                r[(i, j)],
                fixture.reflection_r[i][j]
            );
        }
    }

    // Sanity: the captured golden det is negative
    assert!(
        fixture.reflection_det < 0.0,
        "reflection_det = {}",
        fixture.reflection_det
    );
}
