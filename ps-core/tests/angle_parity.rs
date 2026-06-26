//! MC2 parity + round-trip tests for the `2·asin(d/2)` angular-distance helpers.
//! Golden values captured from the reference into
//! `tests/fixtures/angle_distance.json` (via `tools/parity/capture_core.py`).

use ps_core::angle::{angle_from_distance, distance_from_angle};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct Case {
    angle: f64,
    distance: f64,
    angle_back: f64,
}

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}

fn load() -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/angle_distance.json");
    let body =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&body).expect("parse angle_distance.json")
}

#[test]
fn distance_from_angle_matches_reference() {
    for c in load().cases {
        assert!(
            (distance_from_angle(c.angle) - c.distance).abs() < 1e-9,
            "angle={}",
            c.angle
        );
    }
}

#[test]
fn angle_from_distance_matches_reference() {
    for c in load().cases {
        assert!(
            (angle_from_distance(c.distance) - c.angle_back).abs() < 1e-9,
            "angle={}",
            c.angle
        );
    }
}

#[test]
fn round_trip_identity() {
    for c in load().cases {
        let d = distance_from_angle(c.angle);
        let a = angle_from_distance(d);
        assert!((a - c.angle).abs() < 1e-12, "round-trip angle={}", c.angle);
    }
}

#[test]
fn small_angle_conditioning() {
    // Sub-arcsecond (~0.02") angle round-trips without precision loss.
    let angle = 1e-7_f64;
    let d = distance_from_angle(angle);
    let a = angle_from_distance(d);
    assert!((a - angle).abs() < 1e-12, "small-angle conditioning");
}
