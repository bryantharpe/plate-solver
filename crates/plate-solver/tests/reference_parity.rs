//! End-to-end parity gate against the cedar reference solver (eval tier E1).
//!
//! Solves the 21 sample-image centroid lists in `data/cedar_parity.json` with the
//! real cedar database that ships in `reference-solutions/` and compares every
//! result to what cedar-solve's `solve_from_centroids` returned for the same
//! centroids: same status, RA/Dec and roll within a few arcseconds, FOV within
//! a few arcseconds, and identical matched Hipparcos IDs (the PRD parity contract;
//! `openspec/specs/plate-solver/spec.md`, Requirement: Reference parity).
//!
//! Two passes, each against its own cedar answers: without an FOV estimate (`cedar`),
//! and with a user-style estimate — the field to the nearest degree, ±1° (`fov_hint`,
//! `cedar_hinted`). The second exists because the FOV filter only runs when an
//! estimate is given, and that is exactly where the worst bug on main lived (audit S1).
//! A hint moves cedar's own answer by up to ~10″, so each pass has its own goldens;
//! `scripts/ci/cedar-goldens.py --check` re-derives all of them from the vendored cedar.
//!
//! Unlike the unit tests (empty databases, symmetric coordinates), this is the
//! check that fails when the stages are individually right but wrong together.
//! CI runs it in the `e2e-eval` job; with `E2E_REPORT_DIR` set it also writes its
//! numbers to `E1.json` there, before asserting, so a red run keeps its evidence.
//! The fixture is read at run time, not compiled in, so `e2e-canary` can probe each
//! assertion by editing a golden without a rebuild.

use std::path::PathBuf;
use std::time::Instant;

use pattern_database::{load_from_path, CatalogId, PatternDatabase};
use plate_solver::{solve_from_centroids, SolveStatus};
use serde_json::{json, Value};

/// Position, roll and FOV tolerance, arcseconds ("a few arcseconds").
const TOL_ARCSEC: f64 = 5.0;
/// cedar solves 20 of the 21 sample images; the other (roof) is an exhaustive NO_MATCH.
const EXPECTED_SOLVED: usize = 20;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// Angular separation in arcseconds (Vincenty's formula on the sphere). Unlike the
/// `acos` of a dot product, it stays accurate for tiny separations: `acos` cannot
/// return anything between 0 and about 0.003″.
fn sep_arcsec(ra1: f64, dec1: f64, ra2: f64, dec2: f64) -> f64 {
    let (dra, (s1, c1), (s2, c2)) = (
        (ra2 - ra1).to_radians(),
        dec1.to_radians().sin_cos(),
        dec2.to_radians().sin_cos(),
    );
    let y = ((c2 * dra.sin()).powi(2) + (c1 * s2 - s1 * c2 * dra.cos()).powi(2)).sqrt();
    let x = s1 * s2 + c1 * c2 * dra.cos();
    y.atan2(x).to_degrees() * 3600.0
}

fn status_name(status: Option<SolveStatus>) -> &'static str {
    match status {
        Some(SolveStatus::MatchFound) => "MATCH_FOUND",
        Some(SolveStatus::NoMatch) => "NO_MATCH",
        Some(SolveStatus::Timeout) => "TIMEOUT",
        Some(SolveStatus::Cancelled) => "CANCELLED",
        Some(SolveStatus::TooFew) => "TOO_FEW",
        None => "NONE",
    }
}

/// Write this tier's scorecard when `E2E_REPORT_DIR` is set (CI's `e2e-eval` job).
fn write_report(tier: &str, report: &Value) {
    if let Ok(dir) = std::env::var("E2E_REPORT_DIR") {
        std::fs::create_dir_all(&dir).expect("create E2E_REPORT_DIR");
        let path = PathBuf::from(dir).join(format!("{tier}.json"));
        let text = serde_json::to_string_pretty(report).expect("serialize report");
        std::fs::write(&path, text).expect("write report");
    }
}

#[derive(Default)]
struct Pass {
    solved: usize,
    identical_ids: usize,
    worst_pos: f64,
    worst_roll: f64,
    worst_fov: f64,
    solve_ms: Vec<f64>,
    failures: Vec<String>,
    per_case: Vec<Value>,
}

/// Solve every case, with the case's FOV hint or without, and compare with `golden_key`.
fn run_pass(db: &PatternDatabase, cases: &[Value], hinted: bool, golden_key: &str) -> Pass {
    let label = if hinted { " [FOV hint]" } else { "" };
    let mut pass = Pass::default();
    for case in cases {
        let name = case["image"].as_str().unwrap();
        let width = case["width"].as_u64().unwrap() as usize;
        let height = case["height"].as_u64().unwrap() as usize;
        let centroids: Vec<(f64, f64)> = case["centroids_yx"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (p[0].as_f64().unwrap(), p[1].as_f64().unwrap()))
            .collect();
        let (fov_estimate, fov_max_error) = if hinted {
            let hint = &case["fov_hint"];
            (
                Some(
                    hint["fov_estimate"]
                        .as_f64()
                        .expect("fov_hint.fov_estimate"),
                ),
                hint["fov_max_error"]
                    .as_f64()
                    .expect("fov_hint.fov_max_error"),
            )
        } else {
            (None, 0.0)
        };

        // The solver takes the database by value; copy it outside the timed region.
        let db_copy = db.clone();
        let start = Instant::now();
        let sol = solve_from_centroids(
            &centroids,
            (width, height),
            fov_estimate,
            fov_max_error,
            0.01,
            1e-5,
            5000,
            0.0,
            0.002,
            db_copy,
        );
        pass.solve_ms.push(start.elapsed().as_secs_f64() * 1e3);
        let got = status_name(sol.status);
        let cedar = &case[golden_key];
        let expected = cedar["status"].as_str().unwrap();
        // cedar's NO_MATCH cases are exhaustive searches; a Rust timeout there is fine.
        let equivalent = got == expected || (expected == "NO_MATCH" && got == "TIMEOUT");
        if !equivalent {
            pass.failures
                .push(format!("{name}{label}: status {got}, cedar {expected}"));
            pass.per_case
                .push(json!({ "image": name, "status": got, "cedar_status": expected }));
            continue;
        }
        if expected != "MATCH_FOUND" {
            pass.per_case.push(json!({ "image": name, "status": got }));
            continue;
        }
        pass.solved += 1;

        let ra = sol.ra.unwrap().to_degrees();
        let dec = sol.dec.unwrap().to_degrees();
        let roll = sol.roll.unwrap().to_degrees();
        let fov = sol.camera.expect("camera on success").fov.to_degrees();
        let d_pos = sep_arcsec(
            ra,
            dec,
            cedar["ra"].as_f64().unwrap(),
            cedar["dec"].as_f64().unwrap(),
        );
        let d_roll = (((roll - cedar["roll"].as_f64().unwrap()) + 180.0).rem_euclid(360.0) - 180.0)
            .abs()
            * 3600.0;
        let d_fov = (fov - cedar["fov"].as_f64().unwrap()).abs() * 3600.0;
        pass.worst_pos = pass.worst_pos.max(d_pos);
        pass.worst_roll = pass.worst_roll.max(d_roll);
        pass.worst_fov = pass.worst_fov.max(d_fov);
        // Written as "not within", so a NaN anywhere fails instead of slipping through.
        if !(d_pos <= TOL_ARCSEC && d_roll <= TOL_ARCSEC && d_fov <= TOL_ARCSEC) {
            pass.failures.push(format!(
                "{name}{label}: off by {d_pos:.2}\" position, {d_roll:.2}\" roll, {d_fov:.2}\" FOV"
            ));
        }
        let mut hip: Vec<u64> = sol
            .matched_catalog_ids
            .iter()
            .map(|&i| match db.star_catalog_ids[i] {
                CatalogId::Hip(n) => n as u64,
                other => panic!("unexpected catalog id {other:?}"),
            })
            .collect();
        hip.sort_unstable();
        let mut want: Vec<u64> = cedar["hip"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect();
        want.sort_unstable();
        let identical = hip == want;
        if identical {
            pass.identical_ids += 1;
        } else {
            pass.failures
                .push(format!("{name}{label}: matched HIP IDs differ from cedar"));
        }
        pass.per_case.push(json!({
            "image": name, "status": got, "d_pos_arcsec": d_pos, "d_roll_arcsec": d_roll,
            "d_fov_arcsec": d_fov, "ids_identical": identical,
        }));
    }
    pass.solve_ms.sort_by(f64::total_cmp);
    pass
}

#[test]
fn reference_images_solve_to_cedar_parity() {
    let db = load_from_path(&repo_path(
        "reference-solutions/cedar-solve/tetra3/data/default_database.npz",
    ))
    .expect("the cedar default database must load");
    let text = std::fs::read_to_string(repo_path(
        "crates/plate-solver/tests/data/cedar_parity.json",
    ))
    .expect("read the cedar golden fixture");
    let fixture: Value = serde_json::from_str(&text).expect("fixture parses");
    let cases = fixture["cases"].as_array().expect("cases");

    let plain = run_pass(&db, cases, false, "cedar");
    let hinted = run_pass(&db, cases, true, "cedar_hinted");
    let failures: Vec<&String> = plain.failures.iter().chain(&hinted.failures).collect();

    write_report(
        "E1",
        &json!({
            "tier": "E1",
            "name": "centroid parity",
            "claim": "cedar's centroids, cedar's database, with and without an FOV estimate: same status, same matched stars, attitude within 5\"",
            "spec": "openspec/specs/plate-solver/spec.md — Requirement: Reference parity",
            "cases": cases.len(),
            "expected_solved": EXPECTED_SOLVED,
            "solved": plain.solved,
            "identical_ids": plain.identical_ids,
            "tolerance_arcsec": TOL_ARCSEC,
            "worst_pos_arcsec": plain.worst_pos.max(hinted.worst_pos),
            "worst_roll_arcsec": plain.worst_roll.max(hinted.worst_roll),
            "worst_fov_arcsec": plain.worst_fov.max(hinted.worst_fov),
            "solve_ms_median": plain.solve_ms[plain.solve_ms.len() / 2],
            "solve_ms_max": plain.solve_ms[plain.solve_ms.len() - 1],
            "hinted": {
                "fov_max_error_deg": 1.0,
                "solved": hinted.solved,
                "identical_ids": hinted.identical_ids,
                "worst_pos_arcsec": hinted.worst_pos,
                "solve_ms_median": hinted.solve_ms[hinted.solve_ms.len() / 2],
                "per_case": hinted.per_case,
            },
            "per_case": plain.per_case,
            "failures": failures,
        }),
    );
    assert!(
        failures.is_empty(),
        "{} parity failures:\n{}",
        failures.len(),
        failures
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(
        plain.solved, EXPECTED_SOLVED,
        "cedar solves 20 of the 21 reference images"
    );
    assert_eq!(
        hinted.solved, EXPECTED_SOLVED,
        "cedar solves 20 of the 21 reference images with an FOV estimate"
    );
}
