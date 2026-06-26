//! MC6 parity tests for edge-ratio pattern key, hashing, and probing.
//! Golden values captured from the reference into
//! `tests/fixtures/pattern_hash.json` (via `tools/parity/capture_core.py`).

use ps_core::pattern::{
    compute_pattern_bins, compute_pattern_key, compute_pattern_key_hash, key_hash_low16,
    order_by_centroid_distance, pattern_key_hash_to_index, probe_slots, MAGIC_RAND,
};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct Case {
    key: Vec<u32>,
    key_hash: u64,
    key_hash_low16: u16,
    index_quadratic: u64,
    index_linear: u64,
}

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
    pattern_bins: u32,
    pattern_max_error: f64,
    magic_rand: u64,
    table_size: u64,
}

fn load() -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pattern_hash.json");
    let body =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&body).expect("parse pattern_hash.json")
}

// ---------------------------------------------------------------------------
// 1. compute_pattern_bins
// ---------------------------------------------------------------------------

#[test]
fn pattern_bins_from_max_error() {
    assert_eq!(compute_pattern_bins(0.001), 250);
    assert_eq!(compute_pattern_bins(0.005), 50);
}

// ---------------------------------------------------------------------------
// 2. MAGIC_RAND constant
// ---------------------------------------------------------------------------

#[test]
fn magic_rand_constant() {
    let f = load();
    assert_eq!(MAGIC_RAND, f.magic_rand as u64);
    assert_eq!(MAGIC_RAND, 2654435761);
}

// ---------------------------------------------------------------------------
// 3-6. Fixture-driven hash / index / low16 parity
// ---------------------------------------------------------------------------

#[test]
fn key_hash_matches_reference() {
    let f = load();
    for c in &f.cases {
        let key: [u32; 5] = c.key.clone().try_into().expect("key length 5");
        let got = compute_pattern_key_hash(&key, f.pattern_bins);
        assert_eq!(got, c.key_hash, "hash mismatch for key {:?}", c.key);
    }
}

#[test]
fn index_quadratic_matches_reference() {
    let f = load();
    for c in &f.cases {
        let got = pattern_key_hash_to_index(c.key_hash, f.table_size, false);
        assert_eq!(
            got, c.index_quadratic,
            "quad index mismatch for key {:?}",
            c.key
        );
    }
}

#[test]
fn index_linear_matches_reference() {
    let f = load();
    for c in &f.cases {
        let got = pattern_key_hash_to_index(c.key_hash, f.table_size, true);
        assert_eq!(
            got, c.index_linear,
            "linear index mismatch for key {:?}",
            c.key
        );
    }
}

#[test]
fn key_hash_low16_matches_reference() {
    let f = load();
    for c in &f.cases {
        let got = key_hash_low16(c.key_hash);
        assert_eq!(got, c.key_hash_low16, "low16 mismatch for key {:?}", c.key);
    }
}

// ---------------------------------------------------------------------------
// 7. order_by_centroid_distance — deterministic ordering
// ---------------------------------------------------------------------------

#[test]
fn order_by_centroid_distance_deterministic() {
    // Four vectors with clearly different distances from centroid.
    let vectors: [[f64; 3]; 4] = [
        [1.0, 0.0, 0.0], // near centroid (close to mean)
        [0.9, 0.1, 0.0], // near centroid
        [0.5, 0.5, 0.0], // farther
        [0.0, 0.0, 1.0], // farthest from centroid
    ];
    let order = order_by_centroid_distance(&vectors);

    // Centroid = (0.6, 0.15, 0.25)
    // Distances squared:
    //   v0: (0.4)^2 + (0.15)^2 + (0.25)^2 = 0.16 + 0.0225 + 0.0625 = 0.245
    //   v1: (0.3)^2 + (0.05)^2 + (0.25)^2 = 0.09 + 0.0025 + 0.0625 = 0.155
    //   v2: (0.1)^2 + (0.35)^2 + (0.25)^2 = 0.01 + 0.1225 + 0.0625 = 0.195
    //   v3: (0.6)^2 + (0.15)^2 + (0.75)^2 = 0.36 + 0.0225 + 0.5625 = 0.945
    // Sorted ascending: v1 (0.155), v2 (0.195), v0 (0.245), v3 (0.945)
    assert_eq!(order, [1, 2, 0, 3]);
}

// ---------------------------------------------------------------------------
// 8. compute_pattern_key rotation invariance
// ---------------------------------------------------------------------------

#[test]
fn pattern_key_rotation_invariance() {
    // Four asymmetric unit vectors in the xy-plane with distinct centroid
    // distances so ordering is deterministic and preserved under rotation.
    let angles: [f64; 4] = [0.0, 1.2, 2.5, 4.1]; // radians — not evenly spaced
    let vectors: [[f64; 3]; 4] = angles.map(|a| [a.cos(), a.sin(), 0.0]);

    // Rotate all vectors around the z-axis by 30 degrees.
    // Rotation in the xy-plane preserves inter-vector distances exactly.
    let theta = std::f64::consts::FRAC_PI_6;
    let c = theta.cos();
    let s = theta.sin();
    let rotated: [[f64; 3]; 4] = vectors.map(|[x, y, z]| [c * x - s * y, s * x + c * y, z]);

    let (key_orig, largest_orig) = compute_pattern_key(&vectors, 250);
    let (key_rot, largest_rot) = compute_pattern_key(&rotated, 250);

    // Keys should be identical after rotation.
    assert_eq!(key_orig, key_rot, "pattern key not rotation-invariant");

    // Largest edge should be the same (within float tolerance).
    assert!(
        (largest_orig - largest_rot).abs() < 1e-12,
        "largest edge changed after rotation: {} vs {}",
        largest_orig,
        largest_rot
    );
}

// ---------------------------------------------------------------------------
// 9. probe_slots sequence correctness
// ---------------------------------------------------------------------------

#[test]
fn probe_slots_linear() {
    let slots = probe_slots(100, 1000, true, 5);
    assert_eq!(slots, vec![100, 101, 102, 103, 104]);
}

#[test]
fn probe_slots_quadratic() {
    // offset(c) = c*c: 0, 1, 4, 9, 16
    let slots = probe_slots(100, 1000, false, 5);
    assert_eq!(slots, vec![100, 101, 104, 109, 116]);
}

#[test]
fn probe_slots_wraps_around() {
    // table_size = 10, hash_index = 8
    // linear: 8, 9, 0, 1, 2
    let slots = probe_slots(8, 10, true, 5);
    assert_eq!(slots, vec![8, 9, 0, 1, 2]);

    // quadratic: offset = 0,1,4,9,16 => slots = 8,9,2,7,4 (mod 10)
    let slots = probe_slots(8, 10, false, 5);
    assert_eq!(slots, vec![8, 9, 2, 7, 4]);
}
