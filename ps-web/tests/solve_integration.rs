//! Integration tests for `POST /api/solve`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use ps_db::{importer, loader};
use ps_web::{app, AppState};
use serde::Deserialize;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tower::ServiceExt;

const BOUNDARY: &str = "ps-web-test-boundary-fixed";

enum PartField<'a> {
    Text {
        name: &'a str,
        value: &'a str,
    },
    File {
        name: &'a str,
        filename: &'a str,
        content_type: &'a str,
        data: &'a [u8],
    },
}

/// Hand-build a multipart/form-data body with a fixed boundary (no reqwest dependency).
fn build_multipart_body(fields: &[PartField]) -> Vec<u8> {
    let mut body = Vec::new();
    for field in fields {
        body.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
        match field {
            PartField::Text { name, value } => {
                body.extend_from_slice(
                    format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
                );
                body.extend_from_slice(value.as_bytes());
                body.extend_from_slice(b"\r\n");
            }
            PartField::File {
                name,
                filename,
                content_type,
                data,
            } => {
                body.extend_from_slice(
                    format!(
                        "Content-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n"
                    )
                    .as_bytes(),
                );
                body.extend_from_slice(data);
                body.extend_from_slice(b"\r\n");
            }
        }
    }
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    body
}

fn multipart_content_type() -> String {
    format!("multipart/form-data; boundary={BOUNDARY}")
}

fn make_state() -> AppState {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let npz_path =
        manifest.join("../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    let db_imported =
        importer::import_npz(&npz_path).unwrap_or_else(|e| panic!("import_npz failed: {}", e));

    // Save -> load native round-trip, matching solve_from_image_parity in ps-grpc/src/service.rs.
    let tmp = NamedTempFile::new().expect("tempfile");
    loader::save_native(&db_imported, tmp.path()).expect("save_native");
    let mut db = loader::load_native(tmp.path()).expect("load_native");
    db.build_kd_tree();

    AppState::new(Arc::new(db))
}

#[derive(Deserialize)]
struct Fixture {
    ra_deg: f64,
    dec_deg: f64,
    fov_deg: f64,
}

fn load_fixture() -> Fixture {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_path = manifest.join("../ps-solve/tests/fixtures/reference_solve.json");
    let raw = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", fixture_path.display(), e));
    serde_json::from_str(&raw).expect("fixture JSON parse failed")
}

fn reference_image_bytes() -> Vec<u8> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let img_path = manifest.join(
        "../reference-solutions/cedar-solve/examples/data/medium_fov/2019-07-29T204726_Alt40_Azi-135_Try1.jpg",
    );
    std::fs::read(&img_path).unwrap_or_else(|e| panic!("cannot read {}: {}", img_path.display(), e))
}

#[tokio::test]
async fn solve_reference_image_returns_match_found() {
    let fixture = load_fixture();
    let image_bytes = reference_image_bytes();

    let body = build_multipart_body(&[
        PartField::File {
            name: "image",
            filename: "reference.jpg",
            content_type: "image/jpeg",
            data: &image_bytes,
        },
        PartField::Text {
            name: "fov_estimate",
            value: "11",
        },
        PartField::Text {
            name: "timeout_ms",
            value: "60000",
        },
    ]);

    let app = app(make_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/solve")
                .header("content-type", multipart_content_type())
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json["status"], "match_found", "response body: {json}");

    let ra = json["ra_deg"].as_f64().expect("ra_deg present");
    let dec = json["dec_deg"].as_f64().expect("dec_deg present");
    let fov = json["fov_deg"].as_f64().expect("fov_deg present");
    let matches = json["matches"].as_u64().expect("matches present");

    assert!(
        (ra - fixture.ra_deg).abs() < 0.1,
        "ra_deg {} not within 0.1 of fixture {}",
        ra,
        fixture.ra_deg
    );
    assert!(
        (dec - fixture.dec_deg).abs() < 0.1,
        "dec_deg {} not within 0.1 of fixture {}",
        dec,
        fixture.dec_deg
    );
    assert!(
        (fov - fixture.fov_deg).abs() < 0.2,
        "fov_deg {} not within 0.2 of fixture {}",
        fov,
        fixture.fov_deg
    );
    assert!(matches >= 10, "expected matches >= 10, got {}", matches);

    assert!(json["ra_hms"].is_string());
    assert!(json["dec_dms"].is_string());
    let matched_stars = json["matched_stars"].as_array().unwrap();
    assert!(matched_stars.len() as u64 == matches);

    // Pin matched_stars ra/dec units: they must be degrees, consistent with
    // ra_deg/dec_deg (regression test for radians-vs-degrees mismatch). A
    // matched star must lie within the solved field of view of the
    // boresight; if ra/dec were still in radians this would be off by ~57x.
    let first_star = &matched_stars[0];
    let star_ra = first_star["ra"].as_f64().expect("star ra present");
    let star_dec = first_star["dec"].as_f64().expect("star dec present");
    let sep_deg = angular_separation_deg(star_ra, star_dec, ra, dec);
    assert!(
        sep_deg < fov,
        "matched star ({}, {}) is {} deg from solved boresight ({}, {}), expected < fov {} (units mismatch?)",
        star_ra,
        star_dec,
        sep_deg,
        ra,
        dec,
        fov
    );
}

/// Great-circle angular separation between two ra/dec points, in degrees.
fn angular_separation_deg(ra1_deg: f64, dec1_deg: f64, ra2_deg: f64, dec2_deg: f64) -> f64 {
    let (ra1, dec1) = (ra1_deg.to_radians(), dec1_deg.to_radians());
    let (ra2, dec2) = (ra2_deg.to_radians(), dec2_deg.to_radians());
    let cos_sep = dec1.sin() * dec2.sin() + dec1.cos() * dec2.cos() * (ra1 - ra2).cos();
    cos_sep.clamp(-1.0, 1.0).acos().to_degrees()
}

#[tokio::test]
async fn missing_image_field_returns_400() {
    let body = build_multipart_body(&[PartField::Text {
        name: "fov_estimate",
        value: "11",
    }]);

    let app = app(make_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/solve")
                .header("content-type", multipart_content_type())
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json["error"].is_string());
}

#[tokio::test]
async fn undecodable_image_returns_415() {
    let garbage = vec![0x00u8, 0x01, 0x02, 0x03, 0xffu8, 0xd8, 0x00, 0x00];

    let body = build_multipart_body(&[
        PartField::File {
            name: "image",
            filename: "garbage.bin",
            content_type: "application/octet-stream",
            data: &garbage,
        },
        PartField::Text {
            name: "fov_estimate",
            value: "11",
        },
    ]);

    let app = app(make_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/solve")
                .header("content-type", multipart_content_type())
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json["error"].is_string());
}

/// Regression test for decompression-bomb DoS protection: a valid, cheap-to-encode
/// image (1px tall) that declares a width past the decoder's strict dimension
/// limit must be rejected before any large pixel buffer is allocated.
#[tokio::test]
async fn oversize_image_returns_413() {
    // One dimension past the server's configured limit; height=1 keeps the
    // actual encoded/decoded buffer tiny even though the image is "rejected
    // for being too big".
    let gray = image::GrayImage::new(20_001, 1);
    let dynamic = image::DynamicImage::from(gray);
    let mut png_bytes = Cursor::new(Vec::new());
    dynamic
        .write_to(&mut png_bytes, image::ImageFormat::Png)
        .expect("encode png");

    let body = build_multipart_body(&[
        PartField::File {
            name: "image",
            filename: "oversize.png",
            content_type: "image/png",
            data: png_bytes.get_ref(),
        },
        PartField::Text {
            name: "fov_estimate",
            value: "11",
        },
    ]);

    let app = app(make_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/solve")
                .header("content-type", multipart_content_type())
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json["error"].is_string());
}

#[tokio::test]
async fn invalid_fov_estimate_returns_400() {
    let gray = image::GrayImage::new(64, 64);
    let dynamic = image::DynamicImage::from(gray);
    let mut png_bytes = Cursor::new(Vec::new());
    dynamic
        .write_to(&mut png_bytes, image::ImageFormat::Png)
        .expect("encode png");

    for bad_fov in ["-1", "0", "not-a-number", "NaN"] {
        let body = build_multipart_body(&[
            PartField::File {
                name: "image",
                filename: "black.png",
                content_type: "image/png",
                data: png_bytes.get_ref(),
            },
            PartField::Text {
                name: "fov_estimate",
                value: bad_fov,
            },
        ]);

        let app = app(make_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/solve")
                    .header("content-type", multipart_content_type())
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "fov_estimate={bad_fov:?} should be rejected"
        );
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["error"].is_string());
    }
}

#[tokio::test]
async fn unknown_field_is_ignored() {
    let gray = image::GrayImage::new(64, 64);
    let dynamic = image::DynamicImage::from(gray);
    let mut png_bytes = Cursor::new(Vec::new());
    dynamic
        .write_to(&mut png_bytes, image::ImageFormat::Png)
        .expect("encode png");

    let body = build_multipart_body(&[
        PartField::File {
            name: "image",
            filename: "black.png",
            content_type: "image/png",
            data: png_bytes.get_ref(),
        },
        PartField::Text {
            name: "fov_estimate",
            value: "11",
        },
        PartField::Text {
            name: "not_a_real_field",
            value: "should be silently ignored",
        },
    ]);

    let app = app(make_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/solve")
                .header("content-type", multipart_content_type())
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    // An unrecognized field must not cause a 400 — the request is processed
    // exactly as if the field were absent (all-black image -> too_few).
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["status"], "too_few", "response body: {json}");
}

/// Regression test for the raw request body size limit (distinct from the
/// decode dimension/alloc limits below, which only apply to the `image`
/// field's contents once parsed): a request whose total body exceeds
/// `SOLVE_BODY_LIMIT` must be rejected before multipart parsing even runs.
#[tokio::test]
async fn oversize_body_returns_413() {
    let filler = "a".repeat(34 * 1024 * 1024); // > 32 MiB SOLVE_BODY_LIMIT
    let body = build_multipart_body(&[
        PartField::Text {
            name: "filler",
            value: &filler,
        },
        PartField::Text {
            name: "fov_estimate",
            value: "11",
        },
    ]);

    let app = app(make_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/solve")
                .header("content-type", multipart_content_type())
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

/// Regression test distinguishing the decoder's allocation cap from its
/// dimension cap (`oversize_image_returns_413` above only trips the
/// dimension cap): an image within the per-side dimension limit but whose
/// total pixel buffer exceeds the allocation limit must also be rejected
/// with 413, and the error message must name the memory limit specifically.
#[tokio::test]
async fn decode_alloc_cap_returns_413() {
    // 16400 x 16400 = 268,960,000 bytes > 256 MiB alloc cap, while both
    // dimensions stay under the 20,000px dimension cap.
    let gray = image::GrayImage::new(16_400, 16_400);
    let dynamic = image::DynamicImage::from(gray);
    let mut png_bytes = Cursor::new(Vec::new());
    dynamic
        .write_to(&mut png_bytes, image::ImageFormat::Png)
        .expect("encode png");

    let body = build_multipart_body(&[
        PartField::File {
            name: "image",
            filename: "alloc_cap.png",
            content_type: "image/png",
            data: png_bytes.get_ref(),
        },
        PartField::Text {
            name: "fov_estimate",
            value: "11",
        },
    ]);

    let app = app(make_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/solve")
                .header("content-type", multipart_content_type())
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let error = json["error"].as_str().expect("error is a string");
    assert!(
        error.to_lowercase().contains("memory"),
        "expected a memory-limit error, got: {error}"
    );
}

#[tokio::test]
async fn all_black_image_returns_too_few() {
    let gray = image::GrayImage::new(64, 64);
    let dynamic = image::DynamicImage::from(gray);
    let mut png_bytes = Cursor::new(Vec::new());
    dynamic
        .write_to(&mut png_bytes, image::ImageFormat::Png)
        .expect("encode png");

    let body = build_multipart_body(&[
        PartField::File {
            name: "image",
            filename: "black.png",
            content_type: "image/png",
            data: png_bytes.get_ref(),
        },
        PartField::Text {
            name: "fov_estimate",
            value: "11",
        },
    ]);

    let app = app(make_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/solve")
                .header("content-type", multipart_content_type())
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["status"], "too_few", "response body: {json}");
    assert!(json["hint"].is_string());
}
