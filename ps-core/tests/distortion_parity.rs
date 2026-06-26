//! MC4 parity + round-trip tests for single-parameter radial distortion.
//! Golden values captured from the reference (tetra3._undistort_centroids /
//! _distort_centroids, which compute in float32) into
//! `tests/fixtures/distortion.json` via `tools/parity/capture_core.py`.

use ps_core::distortion::{distort_centroids, undistort_centroids};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Deserialize)]
struct Case {
    k: f64,
    input: Vec<[f64; 2]>,
    undistorted: Vec<[f64; 2]>,
    distorted_roundtrip: Vec<[f64; 2]>,
}

#[derive(Deserialize)]
struct Fixture {
    size: [usize; 2],
    centre: [f64; 2],
    centre_undistorted_by_k: BTreeMap<String, [f64; 2]>,
    cases: Vec<Case>,
}

fn load() -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/distortion.json");
    let body =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&body).expect("parse distortion.json")
}

// Reference goldens are f32-quantised (the reference casts to float32 before
// computing); f64 compute reproduces them to ~1e-4 px. 1e-3 leaves headroom.
const PARITY_TOL: f64 = 1e-3;

#[test]
fn undistort_matches_reference() {
    let fx = load();
    let size = (fx.size[0], fx.size[1]);
    for case in &fx.cases {
        let got = undistort_centroids(&case.input, size, case.k);
        assert_eq!(got.len(), case.undistorted.len());
        for (i, (g, want)) in got.iter().zip(case.undistorted.iter()).enumerate() {
            assert!(
                (g[0] - want[0]).abs() < PARITY_TOL && (g[1] - want[1]).abs() < PARITY_TOL,
                "undistort k={} case={i} got={g:?} want={want:?}",
                case.k
            );
        }
    }
}

#[test]
fn distort_reproduces_reference_roundtrip() {
    // The fixture's `distorted_roundtrip` is the reference's
    // distort(undistort(input)); reproduce it (including the reference's own
    // non-convergence at corner points beyond the half-width radius).
    let fx = load();
    let size = (fx.size[0], fx.size[1]);
    for case in &fx.cases {
        let undist = undistort_centroids(&case.input, size, case.k);
        let got = distort_centroids(&undist, size, case.k);
        for (i, (g, want)) in got.iter().zip(case.distorted_roundtrip.iter()).enumerate() {
            assert!(
                (g[0] - want[0]).abs() < PARITY_TOL && (g[1] - want[1]).abs() < PARITY_TOL,
                "distort-roundtrip k={} case={i} got={g:?} want={want:?}",
                case.k
            );
        }
    }
}

#[test]
fn zero_distortion_is_identity_and_centre_is_fixed() {
    let fx = load();
    let size = (fx.size[0], fx.size[1]);
    // k = 0 → undistort is the exact identity.
    let case0 = fx.cases.iter().find(|c| c.k == 0.0).expect("k=0 case");
    let id = undistort_centroids(&case0.input, size, 0.0);
    for (g, want) in id.iter().zip(case0.input.iter()) {
        assert_eq!(g, want, "k=0 must be exact identity");
    }
    // Image centre is a fixed point of undistort for every k.
    for (k_str, want) in &fx.centre_undistorted_by_k {
        let k: f64 = k_str.parse().unwrap();
        let got = undistort_centroids(&[fx.centre], size, k);
        assert!(
            (got[0][0] - want[0]).abs() < 1e-9 && (got[0][1] - want[1]).abs() < 1e-9,
            "centre must be fixed for k={k}: got={:?} want={want:?}",
            got[0]
        );
    }
}

#[test]
fn round_trip_within_tol_on_invertible_points() {
    // Spec scenarios "Distort/undistort round-trip" + "Convergence bound":
    // on non-centre points inside the invertible radius, both round-trip
    // orders close to < tol (1e-6) in f64.
    let size = (480usize, 640usize);
    let pts: Vec<[f64; 2]> = vec![
        [200.0, 360.0],
        [300.0, 260.0],
        [180.0, 400.0],
        [260.0, 240.0],
        [260.0, 360.0],
    ];
    const TOL: f64 = 1e-6;
    for &k in &[-0.2f64, -0.05, 0.0, 0.05, 0.2] {
        let ud = undistort_centroids(&distort_centroids(&pts, size, k), size, k);
        let du = distort_centroids(&undistort_centroids(&pts, size, k), size, k);
        for (i, p) in pts.iter().enumerate() {
            assert!(
                (ud[i][0] - p[0]).abs() < TOL && (ud[i][1] - p[1]).abs() < TOL,
                "undistort(distort) k={k} i={i} got={:?} want={p:?}",
                ud[i]
            );
            assert!(
                (du[i][0] - p[0]).abs() < TOL && (du[i][1] - p[1]).abs() < TOL,
                "distort(undistort) k={k} i={i} got={:?} want={p:?}",
                du[i]
            );
        }
    }
}
