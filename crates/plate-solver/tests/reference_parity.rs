//! End-to-end reference parity test.
//!
//! Solves the real tetra3 reference test images against the real default database
//! and asserts that the Rust solver matches the tetra3 reference within the PRD
//! tolerances: RA/Dec within a few arcseconds, centroids within ~±0.1 px, and
//! identical matched catalog IDs.

use std::path::PathBuf;

use image::ImageReader;
use pattern_database::{load_from_path, CatalogId};
use plate_solver::{
    solve::{solve_from_image, DetectParams},
    SolveStatus,
};

/// Path to the real tetra3 default database.
fn default_database_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../reference-solutions/tetra3/tetra3/data/default_database.npz");
    path
}

/// Path to a reference test image by name.
fn reference_image_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../reference-solutions/tetra3/examples/data");
    path.push(name);
    path
}

/// Load a 16-bit TIFF and convert it to an 8-bit grayscale row-major buffer.
fn load_tiff_u8(path: &PathBuf) -> (Vec<u8>, usize, usize) {
    let img = ImageReader::open(path)
        .unwrap_or_else(|e| panic!("open {}: {}", path.display(), e))
        .decode()
        .unwrap_or_else(|e| panic!("decode {}: {}", path.display(), e));
    let gray = img.to_luma8();
    let (width, height) = gray.dimensions();
    (gray.into_raw(), width as usize, height as usize)
}

/// Reference solution for `2019-07-29T204726_Alt40_Azi-135_Try1.tiff` produced by
/// tetra3 (commit 2f8b0c5) with `fov_estimate=10`, `distortion=[-.2, .1]`.
/// RA/Dec in degrees, matched catalog IDs are Hipparcos numbers.
const ALT40_REFERENCE: ReferenceSolution = ReferenceSolution {
    ra_deg: 230.66789242161363,
    dec_deg: 11.036065445215474,
    roll_deg: 332.27970204760237,
    fov_deg: 11.422264236679718,
    matched_catalog_ids: &[
        76276, 75530, 76866, 74121, 75230, 74441, 75971, 74016, 74749, 74253, 77001,
    ],
};

const ALT60_REFERENCE: ReferenceSolution = ReferenceSolution {
    ra_deg: 240.46408520303135,
    dec_deg: 28.940608700387028,
    roll_deg: 329.04909716657534,
    fov_deg: 11.421566069071407,
    matched_catalog_ids: &[
        78159, 77512, 80181, 78493, 78459, 77048, 79349, 79757, 77442, 79686, 77397, 76456,
        79441, 78429, 79889, 78851,
    ],
};

struct ReferenceSolution {
    ra_deg: f64,
    dec_deg: f64,
    roll_deg: f64,
    fov_deg: f64,
    matched_catalog_ids: &'static [u32],
}

/// Convert a catalog ID to a Hipparcos number for comparison with the reference.
fn catalog_id_to_hip(id: &CatalogId) -> u32 {
    match *id {
        CatalogId::Hip(h) => h,
        _ => panic!("expected Hipparcos catalog IDs in default database"),
    }
}

/// Assert that two sets of catalog IDs are identical (order-independent).
fn assert_catalog_ids_match(actual: &[u32], expected: &[u32], label: &str) {
    let mut actual_sorted = actual.to_vec();
    let mut expected_sorted = expected.to_vec();
    actual_sorted.sort();
    expected_sorted.sort();
    assert_eq!(
        actual_sorted, expected_sorted,
        "{}: matched catalog IDs differ\n  actual: {:?}\n  expected: {:?}",
        label, actual, expected
    );
}

/// Solve a reference image and return the solution plus the loaded database.
fn solve_reference_image(
    name: &str,
    reference: &ReferenceSolution,
) -> (plate_solver::Solution, pattern_database::PatternDatabase) {
    let db_path = default_database_path();
    let db = load_from_path(&db_path).expect("load default database");

    let img_path = reference_image_path(name);
    let (image, width, height) = load_tiff_u8(&img_path);

    let params = DetectParams {
        sigma: 2.0,
        noise_estimate: None,
        binning: 1,
        normalize_rows: false,
        detect_hot_pixels: true,
        return_binned: false,
        use_binned_for_star_candidates: false,
    };

    let sol = solve_from_image(
        &image,
        width,
        height,
        Some(reference.fov_deg),
        1.45,
        0.01,
        1e-5,
        5_000,
        0.0,
        0.002,
        db.clone(),
        params,
        8,
    );
    (sol, db)
}

#[test]
#[ignore = "slow: loads 49 MB database and solves a real image"]
fn alt40_reference_parity() {
    let (sol, db) = solve_reference_image(
        "2019-07-29T204726_Alt40_Azi-135_Try1.tiff",
        &ALT40_REFERENCE,
    );

    assert_eq!(
        sol.status,
        Some(SolveStatus::MatchFound),
        "alt40 should find a match"
    );

    let ra_deg = sol.ra.unwrap().to_degrees();
    let dec_deg = sol.dec.unwrap().to_degrees();

    let ra_diff_arcsec = ((ra_deg - ALT40_REFERENCE.ra_deg).to_radians().cos() * dec_deg.cos())
        .hypot((dec_deg - ALT40_REFERENCE.dec_deg).to_radians())
        .to_degrees()
        * 3600.0;
    assert!(
        ra_diff_arcsec < 5.0,
        "alt40 RA/Dec difference {} arcsec exceeds tolerance",
        ra_diff_arcsec
    );

    let actual_ids: Vec<u32> = sol
        .matched_catalog_ids
        .iter()
        .map(|&idx| catalog_id_to_hip(&db.star_catalog_ids[idx]))
        .collect();
    assert_catalog_ids_match(&actual_ids, ALT40_REFERENCE.matched_catalog_ids, "alt40");
}

#[test]
#[ignore = "slow: loads 49 MB database and solves a real image"]
fn alt60_reference_parity() {
    let (sol, db) = solve_reference_image(
        "2019-07-29T204726_Alt60_Azi-135_Try1.tiff",
        &ALT60_REFERENCE,
    );

    assert_eq!(
        sol.status,
        Some(SolveStatus::MatchFound),
        "alt60 should find a match"
    );

    let ra_deg = sol.ra.unwrap().to_degrees();
    let dec_deg = sol.dec.unwrap().to_degrees();

    let ra_diff_arcsec = ((ra_deg - ALT60_REFERENCE.ra_deg).to_radians().cos() * dec_deg.cos())
        .hypot((dec_deg - ALT60_REFERENCE.dec_deg).to_radians())
        .to_degrees()
        * 3600.0;
    assert!(
        ra_diff_arcsec < 5.0,
        "alt60 RA/Dec difference {} arcsec exceeds tolerance",
        ra_diff_arcsec
    );

    let actual_ids: Vec<u32> = sol
        .matched_catalog_ids
        .iter()
        .map(|&idx| catalog_id_to_hip(&db.star_catalog_ids[idx]))
        .collect();
    assert_catalog_ids_match(&actual_ids, ALT60_REFERENCE.matched_catalog_ids, "alt60");
}
