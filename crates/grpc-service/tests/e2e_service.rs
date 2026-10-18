//! End-to-end over gRPC with the real cedar database (eval tier E3).
//!
//! `service_acceptance.rs` runs against an empty database and blank images, so
//! every solve there is TOO_FEW by construction: it proves the RPCs exist, not
//! that they solve. This runs the grpc-service spec's end-to-end scenarios for
//! real, on all 21 reference images, through an in-process tonic server:
//!
//! - `SolveFromImage` returns what the library's `solve_from_image` returns for the
//!   same pixels: status, RA/Dec/roll, FOV (degrees on the wire), distortion and
//!   matched stars;
//! - "Consistent with the composed ExtractCentroids + SolveFromCentroids path":
//!   RA/Dec within 10″ and identical matched catalog IDs;
//! - "Hardcoded-noise regression guard": `hale_bopp` at sigma 8 is MATCH_FOUND;
//! - every image lands where cedar put it: same status, RA/Dec within 10″, roll
//!   within 60″, FOV within 15″ (E2's tolerances, for the same reason).
//!
//! CI runs this in the `e2e-eval` job; with `E2E_REPORT_DIR` set it writes `E3.json`.
//! The fixture is read at run time so `e2e-canary` can probe the checks without a rebuild.

use std::path::PathBuf;

use grpc_service::proto::plate_solver_client::PlateSolverClient;
use grpc_service::proto::plate_solver_server::PlateSolverServer as GeneratedServer;
use grpc_service::proto::{
    CentroidsRequest, Image, SolveFromCentroidsRequest, SolveFromImageRequest, SolveParams,
    SolveStatus,
};
use grpc_service::PlateSolverServer;
use hyper_util::rt::TokioIo;
use image::ImageReader;
use pattern_database::{load_from_path, CatalogId, PatternDatabase};
use plate_solver::{solve_from_image, DetectParams};
use serde_json::{json, Value};

const TOL_POS_ARCSEC: f64 = 10.0;
const TOL_ROLL_ARCSEC: f64 = 60.0;
const TOL_FOV_ARCSEC: f64 = 15.0;
const EXPECTED_SOLVED: usize = 20;
const SOLVE_TIMEOUT_MS: i32 = 5000;

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

/// Decode a reference JPEG to 8-bit gray exactly as PIL's `convert('L')` does
/// (how the goldens' centroids were produced).
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
                let l = p[0] as u32 * 19595 + p[1] as u32 * 38470 + p[2] as u32 * 7471 + 0x8000;
                (l >> 16) as u8
            })
            .collect()
    } else {
        img.to_luma8().into_raw()
    };
    (pixels, w, h)
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

async fn client_for(db: PatternDatabase) -> PlateSolverClient<tonic::transport::Channel> {
    let (client_io, server_io) = tokio::io::duplex(64 * 1024);
    let client_io = std::sync::Arc::new(tokio::sync::Mutex::new(Some(client_io)));
    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                GeneratedServer::new(PlateSolverServer::new(db))
                    .max_decoding_message_size(16 * 1024 * 1024),
            )
            .serve_with_incoming(tokio_stream::iter(vec![Ok::<_, std::io::Error>(server_io)]))
            .await
            .unwrap()
    });
    let endpoint = tonic::transport::Endpoint::from_static("http://localhost:50051");
    PlateSolverClient::new(
        endpoint
            .connect_with_connector(tower::service_fn(move |_| {
                let client_io = client_io.clone();
                async move {
                    let mut guard = client_io.lock().await;
                    Ok::<_, std::convert::Infallible>(TokioIo::new(guard.take().unwrap()))
                }
            }))
            .await
            .unwrap(),
    )
}

/// The detection settings the goldens' centroids were produced with.
fn extract(pixels: &[u8], w: usize, h: usize) -> CentroidsRequest {
    CentroidsRequest {
        input_image: Some(Image {
            width: w as i32,
            height: h as i32,
            image_data: pixels.to_vec(),
            shmem_name: None,
            reopen_shmem: false,
        }),
        sigma: 8.0,
        binning: Some(2),
        return_binned: false,
        use_binned_for_star_candidates: false,
        detect_hot_pixels: true,
        normalize_rows: true,
        estimate_background_region: None,
    }
}

fn params() -> SolveParams {
    SolveParams {
        fov_estimate: None,
        fov_max_error: None,
        match_radius: Some(0.01),
        match_threshold: Some(1e-5),
        solve_timeout_ms: Some(SOLVE_TIMEOUT_MS),
        distortion: Some(0.0),
        return_matches: true,
        return_catalog: false,
    }
}

fn wire_ids(sol: &grpc_service::proto::Solution) -> Vec<i64> {
    // A missing ID becomes -1, so it can never compare equal to a real one.
    let mut ids: Vec<i64> = sol.matched.iter().map(|m| m.cat_id.unwrap_or(-1)).collect();
    ids.sort_unstable();
    ids
}

fn unsolved(status: i32) -> bool {
    status == SolveStatus::NoMatch as i32 || status == SolveStatus::Timeout as i32
}

#[tokio::test]
async fn reference_images_solve_end_to_end_over_grpc() {
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
    let mut client = client_for(db.clone()).await;

    let mut failures = Vec::new();
    let mut per_case = Vec::new();
    let (mut solved, mut matches_library, mut matches_composed) = (0, 0, 0);
    let (mut worst_pos, mut worst_roll, mut worst_fov) = (0.0f64, 0.0f64, 0.0f64);
    let mut hale_bopp = "missing";
    for case in cases {
        let name = case["image"].as_str().unwrap();
        let (pixels, w, h) = pil_gray(name);

        let wire = client
            .solve_from_image(SolveFromImageRequest {
                extract: Some(extract(&pixels, w, h)),
                params: Some(params()),
            })
            .await
            .unwrap_or_else(|e| panic!("{name}: SolveFromImage failed: {e}"))
            .into_inner();
        let found = wire.status == SolveStatus::MatchFound as i32;
        if name == "hale_bopp" {
            hale_bopp = if found { "MATCH_FOUND" } else { "NOT_SOLVED" };
        }

        // The wire returns what the library returns for the same pixels.
        let lib = solve_from_image(
            &pixels,
            w,
            h,
            None,
            5.0,
            0.01,
            1e-5,
            SOLVE_TIMEOUT_MS as u64,
            0.0,
            0.002,
            db.clone(),
            DetectParams {
                sigma: 8.0,
                binning: 2,
                normalize_rows: true,
                detect_hot_pixels: true,
                ..DetectParams::default()
            },
        );
        let mut lib_ids: Vec<i64> = lib
            .matched_catalog_ids
            .iter()
            .map(|&i| match db.star_catalog_ids[i] {
                CatalogId::Hip(n) => n as i64,
                other => panic!("unexpected catalog id {other:?}"),
            })
            .collect();
        lib_ids.sort_unstable();
        let lib_found = lib.status == Some(plate_solver::SolveStatus::MatchFound);
        // On success the wire carries the FOV in degrees (the contract); the library
        // keeps radians on its camera.
        let lib_fov_deg = lib.camera.map(|c| c.fov.to_degrees());
        if found == lib_found
            && wire.ra == lib.ra.map(f64::to_degrees)
            && wire.dec == lib.dec.map(f64::to_degrees)
            && wire.roll == lib.roll.map(f64::to_degrees)
            && (!found || wire.fov == lib_fov_deg)
            && wire.distortion == lib.distortion
            && wire_ids(&wire) == lib_ids
        {
            matches_library += 1;
        } else {
            failures.push(format!(
                "{name}: SolveFromImage differs from the library's solve_from_image"
            ));
        }

        // The composed path over the wire: ExtractCentroids -> SolveFromCentroids.
        let centroids = client
            .extract_centroids(extract(&pixels, w, h))
            .await
            .unwrap_or_else(|e| panic!("{name}: ExtractCentroids failed: {e}"))
            .into_inner();
        let composed = client
            .solve_from_centroids(SolveFromCentroidsRequest {
                centroids: centroids
                    .star_candidates
                    .iter()
                    .map(|c| c.centroid_position.expect("centroid position"))
                    .collect(),
                width: w as i32,
                height: h as i32,
                params: Some(params()),
            })
            .await
            .unwrap_or_else(|e| panic!("{name}: SolveFromCentroids failed: {e}"))
            .into_inner();
        let composed_ok = if found {
            composed.status == SolveStatus::MatchFound as i32
                && sep_arcsec(
                    wire.ra.unwrap(),
                    wire.dec.unwrap(),
                    composed.ra.unwrap(),
                    composed.dec.unwrap(),
                ) <= TOL_POS_ARCSEC
                && wire_ids(&wire) == wire_ids(&composed)
        } else {
            unsolved(wire.status) && unsolved(composed.status)
        };
        if composed_ok {
            matches_composed += 1;
        } else {
            failures.push(format!(
                "{name}: SolveFromImage ({}) disagrees with ExtractCentroids + SolveFromCentroids ({})",
                wire.status, composed.status
            ));
        }

        // Lands where cedar put it.
        let cedar = &case["cedar"];
        let expected = cedar["status"].as_str().unwrap();
        if expected == "MATCH_FOUND" {
            if !found {
                failures.push(format!("{name}: status {}, cedar MATCH_FOUND", wire.status));
                per_case.push(json!({ "image": name, "status": wire.status }));
                continue;
            }
            solved += 1;
            let d_pos = sep_arcsec(
                wire.ra.unwrap(),
                wire.dec.unwrap(),
                cedar["ra"].as_f64().unwrap(),
                cedar["dec"].as_f64().unwrap(),
            );
            let d_roll = (((wire.roll.unwrap_or(f64::NAN) - cedar["roll"].as_f64().unwrap())
                + 180.0)
                .rem_euclid(360.0)
                - 180.0)
                .abs()
                * 3600.0;
            let d_fov =
                (wire.fov.unwrap_or(f64::NAN) - cedar["fov"].as_f64().unwrap()).abs() * 3600.0;
            worst_pos = worst_pos.max(d_pos);
            worst_roll = worst_roll.max(d_roll);
            worst_fov = worst_fov.max(d_fov);
            // Written as "not within", so a NaN anywhere fails instead of slipping through.
            if !(d_pos <= TOL_POS_ARCSEC && d_roll <= TOL_ROLL_ARCSEC && d_fov <= TOL_FOV_ARCSEC) {
                failures.push(format!(
                    "{name}: over gRPC {d_pos:.2}\" position, {d_roll:.2}\" roll, {d_fov:.2}\" FOV from cedar"
                ));
            }
            per_case.push(json!({
                "image": name, "status": wire.status, "d_pos_arcsec": d_pos,
                "d_roll_arcsec": d_roll, "d_fov_arcsec": d_fov,
            }));
        } else {
            if !unsolved(wire.status) {
                failures.push(format!("{name}: status {}, cedar {expected}", wire.status));
            }
            per_case.push(json!({ "image": name, "status": wire.status }));
        }
    }
    if hale_bopp != "MATCH_FOUND" {
        failures.push(format!(
            "hale_bopp at sigma 8: {hale_bopp}, spec requires MATCH_FOUND"
        ));
    }

    write_report(
        "E3",
        &json!({
            "tier": "E3",
            "name": "gRPC service",
            "claim": "SolveFromImage over the wire = library = ExtractCentroids+SolveFromCentroids; within 10\" of cedar",
            "spec": "openspec/specs/grpc-service/spec.md — SolveFromImage scenarios",
            "cases": cases.len(),
            "expected_solved": EXPECTED_SOLVED,
            "solved": solved,
            "matches_library": matches_library,
            "matches_composed": matches_composed,
            "hale_bopp": hale_bopp,
            "tolerance_arcsec": { "position": TOL_POS_ARCSEC, "roll": TOL_ROLL_ARCSEC, "fov": TOL_FOV_ARCSEC },
            "worst_pos_arcsec": worst_pos,
            "worst_roll_arcsec": worst_roll,
            "worst_fov_arcsec": worst_fov,
            "per_case": per_case,
            "failures": failures,
        }),
    );
    assert!(
        failures.is_empty(),
        "{} gRPC end-to-end failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(
        solved, EXPECTED_SOLVED,
        "cedar solves 20 of the 21 reference images"
    );
}
