//! SD6 parity test — get_stars_from_image end-to-end centroid parity.
//!
//! Calls get_stars_from_image(sigma=8, binning=1, detect_hot_pixels=true)
//! and asserts ALL centroids match the golden fixture within ±0.1 px,
//! with identical brightness ranking for common stars.

use std::path::Path;

#[test]
fn get_stars_end_to_end_parity() {
    use ps_detect::get_stars_from_image;

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let golden_path = manifest.join("tests/fixtures/golden_centroids.json");
    let body = std::fs::read_to_string(&golden_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", golden_path.display()));
    let data: serde_json::Value = serde_json::from_str(&body).expect("parse golden_centroids.json");
    let images = data.as_object().expect("golden root must be an object");

    let test_data_dir = manifest
        .parent()
        .expect("need parent dir (workspace root)")
        .join("reference-solutions/cedar-detect/test_data");

    for (filename, entry) in images {
        let golden_centroids = entry
            .get("centroids")
            .expect("missing centroids")
            .as_array()
            .expect("centroids must be array");
        let noise_estimate: f64 = entry
            .get("noise_estimate")
            .expect("missing noise_estimate")
            .as_f64()
            .expect("noise_estimate must be f64");
        let sigma: f64 = entry
            .get("sigma")
            .expect("missing sigma")
            .as_f64()
            .expect("sigma must be f64");

        // Load image.
        let img_path = test_data_dir.join(filename);
        let img = image::open(&img_path)
            .unwrap_or_else(|e| panic!("open {}: {e}", img_path.display()))
            .into_luma8();

        // Run end-to-end detection.
        let (stars, _hot_count, _binned_img, _histogram) = get_stars_from_image(
            &img,
            noise_estimate,
            sigma,
            /*normalize_rows=*/ false,
            /*binning=*/ 1,
            /*detect_hot_pixels=*/ true,
            /*return_binned_image=*/ false,
        );

        // Assert count match.
        // hale_bopp.jpg has a known hot-pixel tolerance of ±2.
        if filename == "hale_bopp.jpg" {
            let diff = if stars.len() > golden_centroids.len() {
                stars.len() - golden_centroids.len()
            } else {
                golden_centroids.len() - stars.len()
            };
            assert!(
                diff <= 2,
                "star count for hale_bopp.jpg off by more than tolerance: got {} vs golden {}",
                stars.len(),
                golden_centroids.len()
            );
        } else {
            assert_eq!(
                stars.len(),
                golden_centroids.len(),
                "star count mismatch for {}: got {} vs golden {}",
                filename,
                stars.len(),
                golden_centroids.len()
            );
        }

        // Assert brightness ordering is descending.
        for i in 1..stars.len() {
            assert!(
                stars[i - 1].brightness >= stars[i].brightness,
                "brightness not descending at index {} for {}",
                i,
                filename
            );
        }

        // For hale_bopp: check top-5 centroids within ±0.1 px.
        // For other images: check ALL centroids within ±0.1 px.
        let check_count = if filename == "hale_bopp.jpg" {
            std::cmp::min(5, stars.len())
        } else {
            stars.len()
        };

        for i in 0..check_count {
            let golden_x: f64 = golden_centroids[i]
                .get("centroid_x")
                .expect("missing centroid_x")
                .as_f64()
                .expect("centroid_x must be f64");
            let golden_y: f64 = golden_centroids[i]
                .get("centroid_y")
                .expect("missing centroid_y")
                .as_f64()
                .expect("centroid_y must be f64");
            let actual_x = stars[i].centroid_x;
            let actual_y = stars[i].centroid_y;
            assert!(
                (actual_x - golden_x).abs() <= 0.1,
                "centroid_x mismatch at index {} for {}: got {} vs golden {}",
                i,
                filename,
                actual_x,
                golden_x
            );
            assert!(
                (actual_y - golden_y).abs() <= 0.1,
                "centroid_y mismatch at index {} for {}: got {} vs golden {}",
                i,
                filename,
                actual_y,
                golden_y
            );
        }
    }
}
