//! MC7 parity tests for residual statistics.
//! Golden values captured from the reference into
//! `tests/fixtures/residuals.json` (via `tools/parity/capture_core.py`).

use ps_core::residuals::compute_residuals;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    final_match_vectors: Vec<Vec<f64>>,
    matched_catalog_vectors: Vec<Vec<f64>>,
    rmse_arcsec: f64,
    p90e_arcsec: f64,
    maxe_arcsec: f64,
}

fn load() -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/residuals.json");
    let body =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&body).expect("parse residuals.json")
}

#[test]
fn residual_stats_match_reference() {
    for c in load().cases.iter() {
        let final_vecs: Vec<[f64; 3]> = c
            .final_match_vectors
            .iter()
            .map(|v| [v[0], v[1], v[2]])
            .collect();
        let cat_vecs: Vec<[f64; 3]> = c
            .matched_catalog_vectors
            .iter()
            .map(|v| [v[0], v[1], v[2]])
            .collect();

        let stats = compute_residuals(&final_vecs, &cat_vecs);

        assert!(
            (stats.rmse_arcsec - c.rmse_arcsec).abs() < 1e-9,
            "{} rmse: got={}, expected={}",
            c.name,
            stats.rmse_arcsec,
            c.rmse_arcsec
        );
        assert!(
            (stats.p90e_arcsec - c.p90e_arcsec).abs() < 1e-9,
            "{} p90e: got={}, expected={}",
            c.name,
            stats.p90e_arcsec,
            c.p90e_arcsec
        );
        assert!(
            (stats.maxe_arcsec - c.maxe_arcsec).abs() < 1e-9,
            "{} maxe: got={}, expected={}",
            c.name,
            stats.maxe_arcsec,
            c.maxe_arcsec
        );

        // Invariant: p90e <= maxe
        assert!(
            stats.p90e_arcsec <= stats.maxe_arcsec,
            "{} invariant violated: p90e={} > maxe={}",
            c.name,
            stats.p90e_arcsec,
            stats.maxe_arcsec
        );
    }
}
