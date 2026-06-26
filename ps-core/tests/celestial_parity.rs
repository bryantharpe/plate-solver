//! MC2 parity + round-trip tests for celestial RA/Dec ↔ unit vector.
//! Golden values captured from the reference into
//! `tests/fixtures/celestial_vectors.json` (via `tools/parity/capture_core.py`).

use nalgebra::Vector3;
use ps_core::celestial::{radec_to_vector, vector_to_radec};
use serde::Deserialize;
use std::f64::consts::TAU;
use std::path::Path;

#[derive(Deserialize)]
struct Case {
    ra: f64,
    dec: f64,
    vector: [f64; 3],
    ra_back: f64,
    dec_back: f64,
}

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}

fn load() -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/celestial_vectors.json");
    let body =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&body).expect("parse celestial_vectors.json")
}

#[test]
fn forward_matches_reference_and_is_unit() {
    for c in load().cases {
        let v = radec_to_vector(c.ra, c.dec);
        assert!(
            (v.x - c.vector[0]).abs() < 1e-9,
            "x ra={} dec={}",
            c.ra,
            c.dec
        );
        assert!(
            (v.y - c.vector[1]).abs() < 1e-9,
            "y ra={} dec={}",
            c.ra,
            c.dec
        );
        assert!(
            (v.z - c.vector[2]).abs() < 1e-9,
            "z ra={} dec={}",
            c.ra,
            c.dec
        );
        assert!(
            (v.norm() - 1.0).abs() < 1e-12,
            "unit norm ra={} dec={}",
            c.ra,
            c.dec
        );
    }
}

#[test]
fn inverse_matches_reference() {
    for c in load().cases {
        let v = Vector3::new(c.vector[0], c.vector[1], c.vector[2]);
        let (ra, dec) = vector_to_radec(&v);
        assert!((ra - c.ra_back).abs() < 1e-9, "ra_back ra={}", c.ra);
        assert!((dec - c.dec_back).abs() < 1e-9, "dec_back dec={}", c.dec);
    }
}

#[test]
fn round_trip_identity() {
    // All fixture probes have RA ∈ [0, 2π) and Dec ∈ (-π/2, π/2), so the
    // recovered (RA mod 2π, Dec) must equal the input within 1e-12.
    for c in load().cases {
        let v = radec_to_vector(c.ra, c.dec);
        let (ra, dec) = vector_to_radec(&v);
        let ra_in = c.ra.rem_euclid(TAU);
        assert!((ra - ra_in).abs() < 1e-12, "round-trip ra ra={}", c.ra);
        assert!((dec - c.dec).abs() < 1e-12, "round-trip dec dec={}", c.dec);
    }
}
