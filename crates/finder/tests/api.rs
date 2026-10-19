//! The console's API end to end, on the real database and cedar's sample frames.

use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use finder::samples::Samples;
use finder::stars::NAMED_STARS;
use finder::{router, AppState};
use pattern_database::{CatalogId, PatternDatabase, StarId};
use serde_json::Value;
use tower::ServiceExt;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn samples_dir() -> PathBuf {
    repo().join("reference-solutions/cedar-solve/examples/data/medium_fov")
}

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    let path = repo().join("reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    Arc::new(pattern_database::load_from_path(&path).expect("the vendored cedar database"))
});

fn app() -> Router {
    router(AppState::shared(
        DB.clone(),
        Samples::scan(&samples_dir()).expect("the sample frames"),
    ))
}

async fn send(request: Request<Body>) -> (StatusCode, String, Vec<u8>) {
    let response = app().oneshot(request).await.unwrap();
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, content_type, body.to_vec())
}

async fn get(uri: &str) -> (StatusCode, String, Vec<u8>) {
    send(Request::get(uri).body(Body::empty()).unwrap()).await
}

async fn get_json(uri: &str) -> (StatusCode, Value) {
    let (status, _, body) = get(uri).await;
    (status, serde_json::from_slice(&body).expect("JSON body"))
}

/// Form text fields, `(name, value)`.
type Fields<'a> = &'a [(&'a str, &'a str)];
/// An uploaded frame, `(file name, bytes)`.
type Upload<'a> = Option<(&'a str, &'a [u8])>;

/// A `multipart/form-data` body from text fields and at most one file.
fn form(fields: Fields, file: Upload) -> Request<Body> {
    const BOUNDARY: &str = "finder-test-boundary";
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .bytes(),
        );
    }
    if let Some((file_name, bytes)) = file {
        body.extend(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"image\"; \
                 filename=\"{file_name}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .bytes(),
        );
        body.extend_from_slice(bytes);
        body.extend(b"\r\n");
    }
    body.extend(format!("--{BOUNDARY}--\r\n").bytes());
    Request::post("/api/solve")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::from(body))
        .unwrap()
}

async fn solve(fields: Fields<'_>, file: Upload<'_>) -> (StatusCode, Value) {
    let (status, _, body) = send(form(fields, file)).await;
    (status, serde_json::from_slice(&body).expect("JSON body"))
}

fn ids(list: &Value) -> Vec<String> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|o| o["id"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn health_reports_the_database() {
    let (status, json) = get_json("/api/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "ok");
    assert_eq!(json["database"]["stars"], 42212);
    assert_eq!(json["database"]["min_fov"], 10.0);
    assert_eq!(json["database"]["max_fov"], 30.0);
    assert_eq!(json["samples"], 21);
}

#[tokio::test]
async fn samples_are_listed_served_and_previewed() {
    let (status, json) = get_json("/api/samples").await;
    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 21, "the output/ folder is not a sample");
    assert!(names.contains(&"orion_belt.jpg"));
    assert!(json
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["label"] == "Orion belt"));

    let (status, content_type, body) = get("/api/samples/orion_belt.jpg").await;
    assert_eq!(
        (status, content_type.as_str()),
        (StatusCode::OK, "image/jpeg")
    );
    let original = std::fs::read(samples_dir().join("orion_belt.jpg")).unwrap();
    assert_eq!(body, original);

    let (status, content_type, thumb) = get("/api/samples/orion_belt.jpg/thumb").await;
    assert_eq!(
        (status, content_type.as_str()),
        (StatusCode::OK, "image/jpeg")
    );
    let preview = image::load_from_memory(&thumb).unwrap();
    // Whole-block shrinking: at most 320 pixels, and never less than half that.
    let side = preview.width().max(preview.height());
    assert!((160..=320).contains(&side), "{side}");

    for uri in [
        "/api/samples/missing.jpg",
        "/api/samples/..%2F..%2FCargo.toml",
        "/api/samples/missing.jpg/thumb",
        "/api/nothing-here",
    ] {
        let (status, json) = get_json(uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        assert!(json["error"].is_string(), "{uri}");
    }
}

/// Solving the orion2 sample gives the `solve_image` example's answer, and every matched
/// star's catalog position projects back onto the star it was matched to: the overlay the
/// page draws is the solution, not a picture of it.
#[tokio::test]
async fn a_sample_solves_and_its_overlay_lands_on_its_stars() {
    let (status, r) = solve(&[("sample", "orion2.jpg")], None).await;
    assert_eq!(status, StatusCode::OK, "{r}");
    assert_eq!(r["status"], "match_found");
    assert_eq!(r["name"], "orion2.jpg");
    assert_eq!(
        (r["width"].as_u64(), r["height"].as_u64()),
        (Some(507), Some(507))
    );
    assert!(r["hint"].is_null());

    let p = &r["pointing"];
    // `solve_image --json` on this frame: ra 85.190964, dec -4.246105, fov 10.181431.
    assert!(
        (p["ra_deg"].as_f64().unwrap() - 85.190_964).abs() < 1e-4,
        "{p}"
    );
    assert!(
        (p["dec_deg"].as_f64().unwrap() + 4.246_105).abs() < 1e-4,
        "{p}"
    );
    assert!(
        (p["fov_deg"].as_f64().unwrap() - 10.181_431).abs() < 1e-4,
        "{p}"
    );
    assert_eq!(p["ra_text"], "05h 40m 46s");
    assert_eq!(p["dec_text"], "\u{2212}04° 14′ 46″");
    assert_eq!(p["constellation"]["name"], "Orion");
    assert_eq!(p["matched"], 30);

    let stars = r["stars"].as_array().unwrap();
    let matched: Vec<&Value> = stars.iter().filter(|s| s["matched"] == true).collect();
    assert_eq!(matched.len(), 30);
    // Allow the solution's worst residual plus half a pixel of centroid noise.
    let max_px = p["max_arcsec"].as_f64().unwrap() / p["arcsec_per_px"].as_f64().unwrap() + 0.5;
    for s in &matched {
        let error = s["error_px"].as_f64().unwrap();
        assert!(
            error < max_px,
            "{} is {error:.2} px off (limit {max_px:.2})",
            s["id"]
        );
    }
    let names: Vec<&str> = stars.iter().filter_map(|s| s["name"].as_str()).collect();
    for star in ["Alnitak", "Alnilam", "Mintaka"] {
        assert!(names.contains(&star), "{star} not labelled: {names:?}");
    }
    let (w, h) = (507.0, 507.0);
    for s in stars.iter().filter(|s| s["matched"] == false) {
        let (x, y) = (s["x"].as_f64().unwrap(), s["y"].as_f64().unwrap());
        assert!(
            (0.0..w).contains(&x) && (0.0..h).contains(&y),
            "expected star off frame"
        );
    }

    let objects = ids(&r["objects"]);
    assert_eq!(objects, ["M42", "M78", "M43"], "brightest first");
    let m42 = &r["objects"][0];
    assert_eq!(m42["name"], "Great Orion Nebula");
    assert_eq!(m42["inside"], true);
    // 90′ across is a 45′ radius: 2700″ at 72″ a pixel.
    let radius = m42["radius_px"].as_f64().unwrap();
    assert!((36.0..39.0).contains(&radius), "M42 radius {radius} px");

    assert!(r["detected"].as_array().unwrap().len() >= 30);
    assert_eq!(r["used"], 30);
    let t = &r["timing_ms"];
    assert!(t["decode"].as_f64().unwrap() > 0.0 && t["solve"].as_f64().unwrap() > 0.0);
}

#[tokio::test]
async fn an_upload_solves_and_finds_what_is_in_it() {
    let bytes = std::fs::read(samples_dir().join("pleiades.jpg")).unwrap();
    let (status, r) = solve(
        &[("fov", "10"), ("fov_max_error", "2")],
        Some(("mine.jpg", &bytes)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{r}");
    assert_eq!(r["status"], "match_found");
    assert_eq!(r["name"], "mine.jpg");
    assert_eq!(r["pointing"]["constellation"]["abbr"], "Tau");
    assert!(ids(&r["objects"]).contains(&"M45".to_string()));
    let names: Vec<&str> = r["stars"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["name"].as_str())
        .collect();
    assert!(names.contains(&"Alcyone"), "{names:?}");
}

#[tokio::test]
async fn a_frame_that_does_not_solve_says_what_to_try() {
    let (status, r) = solve(&[("sample", "roof.jpg")], None).await;
    assert_eq!(status, StatusCode::OK);
    assert_ne!(r["status"], "match_found");
    assert!(r["hint"].as_str().unwrap().len() > 20);
    assert!(r["pointing"].is_null());
    assert!(r["stars"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn bad_requests_are_refused_with_reasons() {
    let cases: [(Fields, Upload, StatusCode); 6] = [
        (&[], None, StatusCode::BAD_REQUEST),
        (
            &[("sample", "orion2.jpg"), ("bin", "3")],
            None,
            StatusCode::BAD_REQUEST,
        ),
        (
            &[("sample", "orion2.jpg"), ("sigma", "lots")],
            None,
            StatusCode::BAD_REQUEST,
        ),
        (
            &[("sample", "orion2.jpg"), ("fov", "200")],
            None,
            StatusCode::BAD_REQUEST,
        ),
        (&[("sample", "nope.jpg")], None, StatusCode::NOT_FOUND),
        (
            &[],
            Some(("x.jpg", b"not a jpeg")),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
    ];
    for (fields, file, expected) in cases {
        let (status, json) = solve(fields, file).await;
        assert_eq!(status, expected, "{fields:?}: {json}");
        assert!(json["error"].as_str().is_some_and(|e| !e.is_empty()));
    }
}

#[tokio::test]
async fn similar_objects_rank_for_an_observer() {
    let (status, json) = get_json("/api/similar?id=m42").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["reference"]["id"], "M42");
    assert_eq!(json["family"], "diffuse nebulae");
    assert_eq!(json["suggestions"][0]["id"], "M78");

    // 40° N, 75° W at 9 pm on 2026-01-15.
    let (status, json) =
        get_json("/api/similar?id=M42&lat=40&lon=-75&time=1768528800&limit=3").await;
    assert_eq!(status, StatusCode::OK);
    let s = json["suggestions"].as_array().unwrap();
    assert_eq!(s.len(), 3);
    assert_eq!(
        (s[0]["id"].as_str(), s[0]["up"].as_bool()),
        (Some("M78"), Some(true))
    );
    assert!(s[0]["alt_deg"].as_f64().unwrap() > 40.0);
    assert_eq!(json["observer"]["lat"], 40.0);

    for (uri, expected) in [
        ("/api/similar?id=M200", StatusCode::NOT_FOUND),
        ("/api/similar?id=M42&lat=95&lon=0", StatusCode::BAD_REQUEST),
        ("/api/similar?id=M42&lat=40", StatusCode::BAD_REQUEST),
        ("/api/similar", StatusCode::BAD_REQUEST),
    ] {
        let (status, _, _) = get(uri).await;
        assert_eq!(status, expected, "{uri}");
    }
}

#[tokio::test]
async fn the_page_is_served_and_routes_fall_back_to_it() {
    for uri in ["/", "/index.html", "/some/route"] {
        let (status, content_type, body) = get(uri).await;
        assert_eq!(status, StatusCode::OK, "{uri}");
        assert!(
            content_type.starts_with("text/html"),
            "{uri}: {content_type}"
        );
        assert!(String::from_utf8(body)
            .unwrap()
            .contains("<div id=\"root\">"));
    }
    let (status, _, _) = get("/assets/missing.js").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Every named star's HIP number is in the database at the position written beside it, so
/// a wrong number cannot put the wrong name on a frame.
#[test]
fn named_stars_match_the_database() {
    let db = &**DB;
    for &(hip, name, ra, dec) in NAMED_STARS {
        let index = db
            .star_catalog_ids
            .iter()
            .position(|id| *id == CatalogId::Hip(hip))
            .unwrap_or_else(|| panic!("{name} (HIP {hip}) is not in the database"));
        let star = db.star_vector(StarId(index)).unwrap();
        let listed = math_core::UnitVector::from_radec(ra.to_radians(), dec.to_radians());
        let off = math_core::angular_distance(star, listed).to_degrees();
        assert!(
            off < 0.05,
            "{name} (HIP {hip}) is {off:.3}° from its listed position"
        );
    }
}
