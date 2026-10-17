//! End-to-end parity gate against the cedar reference solver.
//!
//! Solves the 21 sample-image centroid lists in `data/cedar_parity.json` with the
//! real cedar database that ships in `reference-solutions/` and compares every
//! result to what cedar-solve's `solve_from_centroids` returned for the same
//! centroids: same status, RA/Dec and roll within a few arcseconds, FOV within
//! a few arcseconds, and identical matched Hipparcos IDs (the PRD parity contract).
//!
//! Unlike the unit tests (empty databases, symmetric coordinates), this is the
//! check that fails when the stages are individually right but wrong together.

use std::path::PathBuf;

use pattern_database::{load_from_path, CatalogId};
use plate_solver::{solve_from_centroids, SolveStatus};

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn sep_arcsec(ra1: f64, dec1: f64, ra2: f64, dec2: f64) -> f64 {
    let (ra1, dec1, ra2, dec2) = (
        ra1.to_radians(),
        dec1.to_radians(),
        ra2.to_radians(),
        dec2.to_radians(),
    );
    let c = dec1.sin() * dec2.sin() + dec1.cos() * dec2.cos() * (ra1 - ra2).cos();
    c.clamp(-1.0, 1.0).acos().to_degrees() * 3600.0
}

#[test]
fn reference_images_solve_to_cedar_parity() {
    let db = load_from_path(&repo_path(
        "reference-solutions/cedar-solve/tetra3/data/default_database.npz",
    ))
    .expect("the cedar default database must load");
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("data/cedar_parity.json")).expect("fixture parses");

    let mut failures = Vec::new();
    let mut solved = 0;
    for case in fixture["cases"].as_array().expect("cases") {
        let name = case["image"].as_str().unwrap();
        let width = case["width"].as_u64().unwrap() as usize;
        let height = case["height"].as_u64().unwrap() as usize;
        let centroids: Vec<(f64, f64)> = case["centroids_yx"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (p[0].as_f64().unwrap(), p[1].as_f64().unwrap()))
            .collect();

        let sol = solve_from_centroids(
            &centroids,
            (width, height),
            None,
            0.0,
            0.01,
            1e-5,
            5000,
            0.0,
            0.002,
            db.clone(),
        );
        let got = match sol.status {
            Some(SolveStatus::MatchFound) => "MATCH_FOUND",
            Some(SolveStatus::NoMatch) => "NO_MATCH",
            Some(SolveStatus::Timeout) => "TIMEOUT",
            Some(SolveStatus::Cancelled) => "CANCELLED",
            Some(SolveStatus::TooFew) => "TOO_FEW",
            None => "NONE",
        };
        let cedar = &case["cedar"];
        let expected = cedar["status"].as_str().unwrap();
        // cedar's NO_MATCH cases are exhaustive searches; a Rust timeout there is fine.
        let equivalent = got == expected || (expected == "NO_MATCH" && got == "TIMEOUT");
        if !equivalent {
            failures.push(format!("{name}: status {got}, cedar {expected}"));
            continue;
        }
        if expected != "MATCH_FOUND" {
            continue;
        }
        solved += 1;

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
        if d_pos > 5.0 || d_roll > 5.0 || d_fov > 5.0 {
            failures.push(format!(
                "{name}: off by {d_pos:.2}\" position, {d_roll:.2}\" roll, {d_fov:.2}\" FOV"
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
        if hip != want {
            failures.push(format!("{name}: matched HIP IDs differ from cedar"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} parity failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(solved, 19, "cedar solves 19 of the 21 reference images");
}
