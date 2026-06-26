//! MC7 parity tests for false-alarm probability.
//! Golden values captured from the reference into
//! `tests/fixtures/false_alarm.json` (via `tools/parity/capture_core.py`).

use ps_core::false_alarm::{
    effective_match_threshold, false_alarm_probability, reported_probability,
};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    num_extracted_stars: usize,
    num_nearby_catalog_stars: usize,
    num_star_matches: usize,
    match_radius: f64,
    prob_single_star_mismatch: f64,
    cdf_k: i64,
    prob_mismatch: f64,
    num_patterns: usize,
    effective_threshold: f64,
    reported_probability: f64,
    accepted: bool,
}

fn load() -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/false_alarm.json");
    let body =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&body).expect("parse false_alarm.json")
}

#[test]
fn false_alarm_probability_matches_reference() {
    for c in load().cases.iter() {
        let got = false_alarm_probability(
            c.num_extracted_stars,
            c.num_nearby_catalog_stars,
            c.num_star_matches,
            c.match_radius,
        );
        assert!(
            (got - c.prob_mismatch).abs() < 1e-6 * c.prob_mismatch.max(1e-15),
            "n={}, nc={}, m={}, mr={}: got={}, expected={}",
            c.num_extracted_stars,
            c.num_nearby_catalog_stars,
            c.num_star_matches,
            c.match_radius,
            got,
            c.prob_mismatch
        );
    }
}

#[test]
fn effective_match_threshold_matches_reference() {
    for c in load().cases.iter() {
        let user_threshold = 1e-5;
        let got = effective_match_threshold(user_threshold, c.num_patterns);
        assert!(
            (got - c.effective_threshold).abs() < 1e-15,
            "effective_threshold: got={}, expected={}",
            got,
            c.effective_threshold
        );
    }
}

#[test]
fn reported_probability_matches_reference() {
    for c in load().cases.iter() {
        let got = reported_probability(c.prob_mismatch, c.num_patterns);
        assert!(
            (got - c.reported_probability).abs() < 1e-6 * c.reported_probability.max(1e-15),
            "reported_prob: got={}, expected={}",
            got,
            c.reported_probability
        );
    }
}

#[test]
fn acceptance_decision_matches_reference() {
    for c in load().cases.iter() {
        let user_threshold = 1e-5;
        let eff = effective_match_threshold(user_threshold, c.num_patterns);
        let prob = false_alarm_probability(
            c.num_extracted_stars,
            c.num_nearby_catalog_stars,
            c.num_star_matches,
            c.match_radius,
        );
        let accepted = prob < eff;
        assert!(
            accepted == c.accepted,
            "n={}, m={}: accepted={} (expected {}), prob={}, threshold={}",
            c.num_extracted_stars,
            c.num_star_matches,
            accepted,
            c.accepted,
            prob,
            eff
        );
    }
}
