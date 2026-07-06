//! Parity tests for 1-D gate scanning and hot-pixel rejection against cedar-detect reference.
//!
//! Verifies that our scan_image_for_candidates + reject_hot_pixels pipeline
//! produces the same hot_pixel_count as the golden centroids fixture.
//!
//! Note: noise estimates come from noise_estimation.json (our implementation's
//! output), while hot_pixel_count comes from golden_centroids.json (cedar-detect's
//! output). The two fixtures may use slightly different noise estimates for some
//! images, so we verify the gate logic independently.

use std::path::Path;

use ps_detect::gate::{reject_hot_pixels, scan_image_for_candidates};
use ps_detect::io::load_grayscale;
use ps_detect::as_view;

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

/// Load noise estimates from the noise_estimation fixture.
fn load_noise_estimates() -> serde_json::Map<String, serde_json::Value> {
    let body = std::fs::read_to_string(fixture_path("noise_estimation.json"))
        .expect("read noise_estimation.json");
    let data: serde_json::Value = serde_json::from_str(&body).expect("parse noise_estimation.json");
    data.as_object()
        .expect("noise_estimation.json root must be object")
        .clone()
}

/// Load golden centroids fixture.
fn load_golden_centroids() -> serde_json::Map<String, serde_json::Value> {
    let body = std::fs::read_to_string(fixture_path("golden_centroids.json"))
        .expect("read golden_centroids.json");
    let data: serde_json::Value = serde_json::from_str(&body).expect("parse golden_centroids.json");
    data.as_object()
        .expect("golden_centroids.json root must be object")
        .clone()
}

#[test]
fn gate_hot_pixel_count_parity() {
    let noise_data = load_noise_estimates();
    let golden_data = load_golden_centroids();

    // Only test images that appear in both fixtures
    let common: Vec<_> = noise_data
        .keys()
        .filter(|k| golden_data.contains_key(*k))
        .collect();
    assert!(
        !common.is_empty(),
        "need at least one image in both fixtures"
    );

    let test_data = test_data_path();

    for filename in &common {
        let img_path = test_data.join(filename);
        let img = load_grayscale(&img_path)
            .unwrap_or_else(|e| panic!("load {}: {}", img_path.display(), e));

        // Use noise estimate from our noise_estimation fixture (matches our implementation)
        let noise_entry = noise_data.get(*filename).expect("noise entry exists");
        let noise_estimate: f64 = noise_entry
            .get("noise_estimate")
            .expect(&format!("missing noise_estimate for {}", filename))
            .as_f64()
            .expect(&format!("noise_estimate not a number for {}", filename));

        // Use sigma from golden centroids fixture
        let golden_entry = golden_data.get(*filename).expect("golden entry exists");
        let sigma: f64 = golden_entry
            .get("sigma")
            .expect(&format!("missing sigma for {}", filename))
            .as_f64()
            .expect(&format!("sigma not a number for {}", filename));

        // Scan for 1-D candidates (binning=1, using full-res image)
        let view = as_view(&img);
        let candidates = scan_image_for_candidates(&view, noise_estimate, sigma);

        // Compute sigma_noise_2 for hot-pixel rejection (must match scan thresholds)
        let sigma_noise_2 = std::cmp::max((2.0 * sigma * noise_estimate + 0.5) as i16, 2);

        // Reject hot pixels
        let (_filtered, hot_pixel_count) = reject_hot_pixels(&candidates, &img, 1, sigma_noise_2);

        let expected_hot: usize = golden_entry
            .get("hot_pixel_count")
            .expect(&format!("missing hot_pixel_count for {}", filename))
            .as_u64()
            .expect(&format!("hot_pixel_count not a number for {}", filename))
            as usize;

        // The hot_pixel_count may differ slightly if the noise estimate differs between
        // our implementation and cedar-detect (which causes different candidate sets).
        // We verify the count is within a reasonable tolerance.
        let diff = if hot_pixel_count > expected_hot {
            hot_pixel_count - expected_hot
        } else {
            expected_hot - hot_pixel_count
        };

        // Allow up to 5% tolerance since noise estimate differences can shift
        // which pixels cross the significance threshold.
        let tolerance = (expected_hot as f64 * 0.05).max(5.0) as usize;
        assert!(
            diff <= tolerance,
            "hot_pixel_count for {}: got {} (golden {}) diff={} exceeds tolerance {}",
            filename,
            hot_pixel_count,
            expected_hot,
            diff,
            tolerance
        );
    }
}
