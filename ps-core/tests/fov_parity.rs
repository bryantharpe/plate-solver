//! MC7 parity tests for FOV estimation and refinement.
//! Golden values captured from the reference into
//! `tests/fixtures/fov_refinement.json` (via `tools/parity/capture_core.py`).

use ps_core::fov::{
    diagonal_fov, estimate_fov_from_pattern, refine_fov_no_distortion, refine_fov_with_distortion,
};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct FovFixture {
    cases: Vec<FovCase>,
}

#[derive(Deserialize)]
struct FovCase {
    mode: String,
    catalog_largest_edge: Option<f64>,
    image_pattern_largest_edge: Option<f64>,
    image_pattern_largest_pixel_distance: Option<f64>,
    fov_estimate: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
    fov: Option<f64>,
    fov_horizontal: Option<Vec<f64>>,
    fov_diagonal: Option<Vec<f64>>,
    fov_coarse: Option<f64>,
    fov_fine: Option<f64>,
    matched_image_vectors: Option<Vec<[f64; 3]>>,
    matched_catalog_vectors: Option<Vec<[f64; 3]>>,
    matched_image_centroids: Option<Vec<[f64; 2]>>,
    rotation_matrix: Option<[[f64; 3]; 3]>,
    k: Option<f64>,
}

fn load() -> FovFixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fov_refinement.json");
    let body =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&body).expect("parse fov_refinement.json")
}

#[test]
fn coarse_fov_with_estimate() {
    let fixture = load();
    let c = fixture
        .cases
        .iter()
        .find(|c| c.mode == "coarse_with_estimate")
        .unwrap();
    let got = estimate_fov_from_pattern(
        c.catalog_largest_edge.unwrap(),
        c.image_pattern_largest_edge.unwrap(),
        0.0, // unused
        Some(c.fov_estimate.unwrap()),
        c.width.unwrap(),
    );
    assert!(
        (got - c.fov.unwrap()).abs() < 1e-9,
        "coarse_with_estimate: got={}, expected={}",
        got,
        c.fov.unwrap()
    );
}

#[test]
fn coarse_fov_no_estimate() {
    let fixture = load();
    let c = fixture
        .cases
        .iter()
        .find(|c| c.mode == "coarse_no_estimate")
        .unwrap();
    let got = estimate_fov_from_pattern(
        c.catalog_largest_edge.unwrap(),
        0.0, // unused
        c.image_pattern_largest_pixel_distance.unwrap(),
        None,
        c.width.unwrap(),
    );
    assert!(
        (got - c.fov.unwrap()).abs() < 1e-9,
        "coarse_no_estimate: got={}, expected={}",
        got,
        c.fov.unwrap()
    );
}

#[test]
fn diagonal_fov_matches_reference() {
    let fixture = load();
    let c = fixture.cases.iter().find(|c| c.mode == "diagonal").unwrap();
    let width = c.width.unwrap();
    let height = c.height.unwrap();
    let fovs = c.fov_horizontal.as_ref().unwrap();
    let expected = c.fov_diagonal.as_ref().unwrap();

    for (i, &fov_h) in fovs.iter().enumerate() {
        let got = diagonal_fov(fov_h, width, height);
        assert!(
            (got - expected[i]).abs() < 1e-9,
            "diagonal[{}]: got={}, expected={}",
            i,
            got,
            expected[i]
        );
    }
}

#[test]
fn fine_fov_no_distortion() {
    let fixture = load();
    let c = fixture
        .cases
        .iter()
        .find(|c| c.mode == "fine_no_distortion")
        .unwrap();
    let fov_coarse = c.fov_coarse.unwrap();
    let image_vectors = c.matched_image_vectors.as_ref().unwrap();
    let catalog_vectors = c.matched_catalog_vectors.as_ref().unwrap();

    // Vectors are [[f64; 3]] from serde, each inner array is [y0, y1, y2] (Vec<f64>)
    // Deserialize as Vec<Vec<f64>> to be safe, then convert.
    let img: Vec<[f64; 3]> = image_vectors.iter().map(|v| [v[0], v[1], v[2]]).collect();
    let cat: Vec<[f64; 3]> = catalog_vectors.iter().map(|v| [v[0], v[1], v[2]]).collect();

    let got = refine_fov_no_distortion(fov_coarse, &img, &cat);
    assert!(
        (got - c.fov_fine.unwrap()).abs() < 1e-9,
        "fine_no_distortion: got={}, expected={}",
        got,
        c.fov_fine.unwrap()
    );
}

#[test]
fn fine_fov_with_distortion() {
    let fixture = load();
    let c = fixture
        .cases
        .iter()
        .find(|c| c.mode == "fine_with_distortion")
        .unwrap();
    let centroids = c.matched_image_centroids.as_ref().unwrap();
    let catalog_vectors = c.matched_catalog_vectors.as_ref().unwrap();
    let rot_matrix = c.rotation_matrix.as_ref().unwrap();
    let width = c.width.unwrap();
    let height = c.height.unwrap();

    let centroids_arr: Vec<[f64; 2]> = centroids.iter().map(|v| [v[0], v[1]]).collect();
    let cat_arr: Vec<[f64; 3]> = catalog_vectors.iter().map(|v| [v[0], v[1], v[2]]).collect();
    let rot: [[f64; 3]; 3] = [rot_matrix[0], rot_matrix[1], rot_matrix[2]];

    let (fov, k) = refine_fov_with_distortion(&centroids_arr, &cat_arr, &rot, width, height);
    assert!(
        (fov - c.fov.unwrap()).abs() < 1e-9,
        "fine_with_distortion fov: got={}, expected={}",
        fov,
        c.fov.unwrap()
    );
    assert!(
        (k - c.k.unwrap()).abs() < 1e-9,
        "fine_with_distortion k: got={}, expected={}",
        k,
        c.k.unwrap()
    );
}
