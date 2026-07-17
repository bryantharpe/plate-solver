//! Differential ("parity") test: the Rust implementation must match a set of
//! values computed by an *independent* reference (see `fixtures/golden.json`,
//! generated with NumPy) within a fixed tolerance. This is the miniature of the
//! real rig's parity contract against tetra3/cedar — the strongest kind of
//! evidence, because the oracle was not produced by the code under test.

use canary_core::angular_separation;
use serde_json::Value;

fn arr3(v: &Value) -> [f64; 3] {
    let a = v.as_array().expect("vector must be an array");
    [
        a[0].as_f64().unwrap(),
        a[1].as_f64().unwrap(),
        a[2].as_f64().unwrap(),
    ]
}

#[test]
fn matches_independent_reference() {
    let raw = include_str!("../fixtures/golden.json");
    let doc: Value = serde_json::from_str(raw).expect("golden.json must be valid JSON");
    let tol = doc["tolerance_rad"].as_f64().expect("tolerance_rad");

    let cases = doc["cases"].as_array().expect("cases array");
    assert!(!cases.is_empty(), "fixture must contain cases");

    for case in cases {
        let a = arr3(&case["a"]);
        let b = arr3(&case["b"]);
        let expected = case["sep"].as_f64().expect("sep");
        let got = angular_separation(a, b);
        assert!(
            (got - expected).abs() <= tol,
            "parity failure for {a:?} -> {b:?}: got {got}, reference {expected} (tol {tol})"
        );
    }
}
