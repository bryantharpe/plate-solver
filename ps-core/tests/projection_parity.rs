//! MC3 parity + round-trip tests for pinhole projection: pixel centroids
//! ↔ camera-frame unit vectors. Golden values captured from the reference into
//! `tests/fixtures/projection.json` (via `tools/parity/capture_core.py`).

use nalgebra::Vector3;
use ps_core::projection::{compute_centroids, compute_vectors};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct Case {
    centroid: [f64; 2],
    vector: [f64; 3],
    centroid_back: [f64; 2],
}

#[derive(Deserialize)]
struct Fixture {
    size: [usize; 2],
    fov: f64,
    cases: Vec<Case>,
    keep: Vec<usize>,
}

fn load() -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/projection.json");
    let body =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&body).expect("parse projection.json")
}

#[test]
fn compute_vectors_matches_reference_and_is_unit() {
    let fixture = load();
    let size = (fixture.size[0], fixture.size[1]);
    let centroids: Vec<[f64; 2]> = fixture.cases.iter().map(|c| c.centroid).collect();
    let vectors = compute_vectors(&centroids, size, fixture.fov);
    for (i, (v, c)) in vectors.iter().zip(fixture.cases.iter()).enumerate() {
        assert!(
            (v.x - c.vector[0]).abs() < 1e-9,
            "x case={} centroid={:?}",
            i,
            c.centroid
        );
        assert!(
            (v.y - c.vector[1]).abs() < 1e-9,
            "y case={} centroid={:?}",
            i,
            c.centroid
        );
        assert!(
            (v.z - c.vector[2]).abs() < 1e-9,
            "z case={} centroid={:?}",
            i,
            c.centroid
        );
        assert!(
            (v.norm() - 1.0).abs() < 1e-12,
            "unit norm case={} centroid={:?}",
            i,
            c.centroid
        );
    }
}

#[test]
fn compute_centroids_matches_reference_and_keep() {
    let fixture = load();
    let size = (fixture.size[0], fixture.size[1]);
    let vectors: Vec<Vector3<f64>> = fixture
        .cases
        .iter()
        .map(|c| Vector3::new(c.vector[0], c.vector[1], c.vector[2]))
        .collect();
    let (centroids, keep) = compute_centroids(&vectors, size, fixture.fov);
    for (i, (back, c)) in centroids.iter().zip(fixture.cases.iter()).enumerate() {
        assert!(
            (back[0] - c.centroid_back[0]).abs() < 1e-9,
            "y case={} vector={:?}",
            i,
            c.vector
        );
        assert!(
            (back[1] - c.centroid_back[1]).abs() < 1e-9,
            "x case={} vector={:?}",
            i,
            c.vector
        );
    }
    assert_eq!(keep, fixture.keep);
}

#[test]
fn projection_round_trip_identity() {
    let fixture = load();
    let size = (fixture.size[0], fixture.size[1]);
    let centroids: Vec<[f64; 2]> = fixture.cases.iter().map(|c| c.centroid).collect();
    let vectors = compute_vectors(&centroids, size, fixture.fov);
    let (back, _) = compute_centroids(&vectors, size, fixture.fov);
    for (i, (b, orig)) in back.iter().zip(centroids.iter()).enumerate() {
        assert!(
            (b[0] - orig[0]).abs() < 1e-9,
            "y case={} centroid={:?}",
            i,
            orig
        );
        assert!(
            (b[1] - orig[1]).abs() < 1e-9,
            "x case={} centroid={:?}",
            i,
            orig
        );
    }
}

#[test]
fn vector_behind_camera_is_excluded() {
    let size = (480, 640);
    let fov = 20f64.to_radians();
    let vectors = vec![
        Vector3::new(-1.0, 0.0, 0.0).normalize(),
        Vector3::new(-0.9, 0.05, 0.05).normalize(),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
    ];
    let (_centroids, keep) = compute_centroids(&vectors, size, fov);
    assert_eq!(keep, vec![3]);
}
