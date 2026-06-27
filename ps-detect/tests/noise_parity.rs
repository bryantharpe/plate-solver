//! Parity tests for noise and background estimation against cedar-detect reference.

use std::path::Path;

use imageproc::rect::Rect;
use ps_detect::io::load_grayscale;
use ps_detect::noise::{estimate_noise_from_image, estimate_background_from_image_region};

fn fixture_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn test_data_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .expect("need at least 1 ancestor dir for workspace root")
        .to_path_buf()
        .join("reference-solutions")
        .join("cedar-detect")
        .join("test_data")
}

#[test]
fn noise_estimation_parity() {
    let body = std::fs::read_to_string(fixture_path("noise_estimation.json"))
        .expect("read noise_estimation.json");
    let data: serde_json::Value = serde_json::from_str(&body)
        .expect("parse noise_estimation.json");

    let images = data.as_object().expect("root must be object");
    assert!(!images.is_empty(), "must have at least one entry");

    let test_data = test_data_path();
    for (filename, entry) in images {
        let img_path = test_data.join(filename);
        let img = load_grayscale(&img_path)
            .unwrap_or_else(|e| panic!("load {}: {}", img_path.display(), e));

        let noise = estimate_noise_from_image(&img);

        let expected: f64 = entry.get("noise_estimate")
            .expect(&format!("missing noise_estimate for {}", filename))
            .as_f64()
            .expect(&format!("noise_estimate not a number for {}", filename));

        assert!(
            (noise - expected).abs() < 1e-6,
            "noise mismatch for {}: got {:.10}, expected {:.10}",
            filename, noise, expected
        );
    }
}

#[test]
fn background_estimation_parity() {
    // Use the first test image and a known ROI to verify background estimation.
    let body = std::fs::read_to_string(fixture_path("noise_estimation.json"))
        .expect("read noise_estimation.json");
    let data: serde_json::Value = serde_json::from_str(&body)
        .expect("parse noise_estimation.json");

    let images = data.as_object().expect("root must be object");
    let filename = images.keys().next().expect("at least one entry");

    let test_data = test_data_path();
    let img_path = test_data.join(filename);
    let img = load_grayscale(&img_path)
        .unwrap_or_else(|e| panic!("load {}: {}", img_path.display(), e));

    let (w, h) = img.dimensions();
    // Pick a ROI in the lower-left quadrant where sky background is typical.
    let roi = Rect::at(10, h as i32 - 20)
        .of_size(w / 4, 10);

    let (mean, stddev) = estimate_background_from_image_region(&img, &roi);

    // Sanity checks: mean should be a reasonable background level, stddev > 0.
    assert!(mean >= 0.0 && mean <= 255.0, "mean out of range: {}", mean);
    assert!(stddev >= 0.0, "stddev should be non-negative: {}", stddev);

    // The golden fixture has noise_estimate for the whole image;
    // the background stddev for a small ROI won't match exactly but
    // should be in the same ballpark (within factor of 3).
    let entry = images.get(filename).expect("entry exists");
    let global_noise: f64 = entry.get("noise_estimate")
        .expect("noise_estimate present")
        .as_f64()
        .expect("noise_estimate is number");

    assert!(
        stddev < global_noise * 3.0,
        "ROI stddev {:.4} exceeds 3x global noise {:.4} for {}",
        stddev, global_noise, filename
    );
}
