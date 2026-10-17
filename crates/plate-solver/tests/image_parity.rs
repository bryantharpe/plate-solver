//! End-to-end parity through the image entry point (eval tier E2).
//!
//! `reference_parity` starts from centroids. This starts from the JPEGs the
//! goldens came from and goes through `solve_from_image`: the noise estimate,
//! detection, the `Star { x, y }` → solver `(y, x)` convention, and the solve.
//! Two claims, checked on all 21 reference images:
//!
//! 1. `solve_from_image` is exactly `detect_stars` → `(y, x)` → `solve_from_centroids`
//!    on the same pixels — the library form of grpc-service's "Consistent with the
//!    composed ExtractCentroids + SolveFromCentroids path". Same code, same inputs,
//!    so status, attitude, FOV, distortion and matched stars must be identical.
//! 2. Every image lands where cedar put it: same status, RA/Dec within 10″, roll
//!    within 60″ (under 10″ of motion at the corners of these 10–15° fields), FOV
//!    within 15″, and at least 90% of cedar's matched stars matched again. Looser
//!    than E1's 5″ on purpose: this path solves with every detected star, while the
//!    goldens were solved from the brightest 30. Measured worst on these images:
//!    6.4″, 46″, 11.7″, and 100% of cedar's stars.
//!
//! Gray conversion matches PIL's `convert('L')`, which is how the goldens' centroids
//! were produced; `image`'s own `to_luma8` uses Rec. 709 weights and moves faint stars.
//! The fixture is read at run time so `e2e-canary` can probe the checks without a rebuild.

use std::path::PathBuf;
use std::time::Instant;

use image::ImageReader;
use pattern_database::{load_from_path, CatalogId, PatternDatabase};
use plate_solver::{solve_from_centroids, solve_from_image, DetectParams, Solution, SolveStatus};
use serde_json::{json, Value};

const TOL_POS_ARCSEC: f64 = 10.0;
const TOL_ROLL_ARCSEC: f64 = 60.0;
const TOL_FOV_ARCSEC: f64 = 15.0;
const MIN_CEDAR_RECALL: f64 = 0.9;
const EXPECTED_SOLVED: usize = 19;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// Angular separation in arcseconds (Vincenty's formula on the sphere): accurate for
/// tiny separations, where the `acos` of a dot product bottoms out near 0.003″.
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

/// Decode a reference JPEG to 8-bit gray exactly as PIL's `convert('L')` does.
fn pil_gray(name: &str) -> (Vec<u8>, usize, usize) {
    let path = repo_path(&format!(
        "reference-solutions/cedar-solve/examples/data/medium_fov/{name}.jpg"
    ));
    let img = ImageReader::open(&path)
        .unwrap_or_else(|e| panic!("open {}: {e}", path.display()))
        .decode()
        .unwrap_or_else(|e| panic!("decode {}: {e}", path.display()));
    let (w, h) = (img.width() as usize, img.height() as usize);
    let pixels = if img.color().has_color() {
        img.to_rgb8()
            .pixels()
            .map(|p| {
                // PIL's L24: ITU-R 601-2 luma in 16-bit fixed point, rounded.
                let l = p[0] as u32 * 19595 + p[1] as u32 * 38470 + p[2] as u32 * 7471 + 0x8000;
                (l >> 16) as u8
            })
            .collect()
    } else {
        img.to_luma8().into_raw()
    };
    (pixels, w, h)
}

fn hip_ids(sol: &Solution, db: &PatternDatabase) -> Vec<u64> {
    let mut ids: Vec<u64> = sol
        .matched_catalog_ids
        .iter()
        .map(|&i| match db.star_catalog_ids[i] {
            CatalogId::Hip(n) => n as u64,
            other => panic!("unexpected catalog id {other:?}"),
        })
        .collect();
    ids.sort_unstable();
    ids
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

#[test]
fn reference_images_solve_to_cedar_parity_from_pixels() {
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
    // The detection settings the goldens' centroids were produced with.
    let params = DetectParams {
        sigma: 8.0,
        binning: 2,
        normalize_rows: true,
        detect_hot_pixels: true,
        ..DetectParams::default()
    };

    let mut failures = Vec::new();
    let mut per_case = Vec::new();
    let (mut solved, mut composed_identical) = (0, 0);
    let (mut worst_pos, mut worst_roll, mut worst_fov) = (0.0f64, 0.0f64, 0.0f64);
    let mut worst_recall = 1.0f64;
    let mut solve_ms = Vec::new();
    for case in cases {
        let name = case["image"].as_str().unwrap();
        let (pixels, w, h) = pil_gray(name);
        assert_eq!(
            (w as u64, h as u64),
            (
                case["width"].as_u64().unwrap(),
                case["height"].as_u64().unwrap()
            ),
            "{name}: image size differs from the fixture"
        );

        // The solver takes the database by value; copy it outside the timed region.
        let db_copy = db.clone();
        let start = Instant::now();
        let sol = solve_from_image(
            &pixels, w, h, None, 0.0, 0.01, 1e-5, 5000, 0.0, 0.002, db_copy, params,
        );
        solve_ms.push(start.elapsed().as_secs_f64() * 1e3);

        // Claim 1: identical to the composed path on the same pixels.
        let stars = star_detection::detect_stars(&pixels, w, h, 8.0, 2, true, true);
        let centroids: Vec<(f64, f64)> = stars.iter().map(|s| (s.y, s.x)).collect();
        let composed = solve_from_centroids(
            &centroids,
            (w, h),
            None,
            0.0,
            0.01,
            1e-5,
            5000,
            0.0,
            0.002,
            db.clone(),
        );
        let same_status = sol.status == composed.status
            || matches!(
                (sol.status, composed.status),
                (Some(SolveStatus::NoMatch), Some(SolveStatus::Timeout))
                    | (Some(SolveStatus::Timeout), Some(SolveStatus::NoMatch))
            );
        if same_status
            && sol.ra == composed.ra
            && sol.dec == composed.dec
            && sol.roll == composed.roll
            && sol.camera.map(|c| c.fov) == composed.camera.map(|c| c.fov)
            && sol.distortion == composed.distortion
            && sol.matched_catalog_ids == composed.matched_catalog_ids
        {
            composed_identical += 1;
        } else {
            failures.push(format!(
                "{name}: solve_from_image ({:?}) differs from detect_stars + solve_from_centroids ({:?})",
                sol.status, composed.status
            ));
        }

        // Claim 2: lands where cedar put it.
        let cedar = &case["cedar"];
        let expected = cedar["status"].as_str().unwrap();
        let found = sol.status == Some(SolveStatus::MatchFound);
        if expected == "MATCH_FOUND" && !found {
            failures.push(format!("{name}: {:?}, cedar MATCH_FOUND", sol.status));
            per_case.push(json!({ "image": name, "status": format!("{:?}", sol.status) }));
            continue;
        }
        if expected != "MATCH_FOUND" {
            if !matches!(
                sol.status,
                Some(SolveStatus::NoMatch) | Some(SolveStatus::Timeout)
            ) {
                failures.push(format!("{name}: {:?}, cedar {expected}", sol.status));
            }
            per_case.push(json!({ "image": name, "status": format!("{:?}", sol.status) }));
            continue;
        }
        solved += 1;
        let d_pos = sep_arcsec(
            sol.ra.unwrap().to_degrees(),
            sol.dec.unwrap().to_degrees(),
            cedar["ra"].as_f64().unwrap(),
            cedar["dec"].as_f64().unwrap(),
        );
        let d_roll = (((sol.roll.unwrap().to_degrees() - cedar["roll"].as_f64().unwrap()) + 180.0)
            .rem_euclid(360.0)
            - 180.0)
            .abs()
            * 3600.0;
        let d_fov = (sol.camera.expect("camera on success").fov.to_degrees()
            - cedar["fov"].as_f64().unwrap())
        .abs()
            * 3600.0;
        let ours = hip_ids(&sol, &db);
        let want: Vec<u64> = cedar["hip"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect();
        let recall = want
            .iter()
            .filter(|id| ours.binary_search(id).is_ok())
            .count() as f64
            / want.len() as f64;
        worst_pos = worst_pos.max(d_pos);
        worst_roll = worst_roll.max(d_roll);
        worst_fov = worst_fov.max(d_fov);
        worst_recall = worst_recall.min(recall);
        // Written as "not within", so a NaN anywhere fails instead of slipping through.
        if !(d_pos <= TOL_POS_ARCSEC
            && d_roll <= TOL_ROLL_ARCSEC
            && d_fov <= TOL_FOV_ARCSEC
            && recall >= MIN_CEDAR_RECALL)
        {
            failures.push(format!(
                "{name}: {d_pos:.2}\" position, {d_roll:.2}\" roll, {d_fov:.2}\" FOV, {:.0}% of cedar's stars",
                recall * 100.0
            ));
        }
        per_case.push(json!({
            "image": name, "status": "MATCH_FOUND", "d_pos_arcsec": d_pos,
            "d_roll_arcsec": d_roll, "d_fov_arcsec": d_fov, "cedar_recall": recall,
        }));
    }

    solve_ms.sort_by(f64::total_cmp);
    write_report(
        "E2",
        &json!({
            "tier": "E2",
            "name": "image entry point",
            "claim": "JPEG → solve_from_image: identical to the composed path; within 10\" of cedar",
            "spec": "openspec/specs/plate-solver/spec.md — Requirement: Reference parity (from the image)",
            "cases": cases.len(),
            "expected_solved": EXPECTED_SOLVED,
            "solved": solved,
            "composed_identical": composed_identical,
            "tolerance_arcsec": { "position": TOL_POS_ARCSEC, "roll": TOL_ROLL_ARCSEC, "fov": TOL_FOV_ARCSEC },
            "worst_pos_arcsec": worst_pos,
            "worst_roll_arcsec": worst_roll,
            "worst_fov_arcsec": worst_fov,
            "worst_cedar_recall": worst_recall,
            "solve_ms_median": solve_ms[solve_ms.len() / 2],
            "solve_ms_max": solve_ms[solve_ms.len() - 1],
            "per_case": per_case,
            "failures": failures,
        }),
    );
    assert!(
        failures.is_empty(),
        "{} image-path failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(
        solved, EXPECTED_SOLVED,
        "cedar solves 19 of the 21 reference images"
    );
    assert_eq!(composed_identical, cases.len());
}
