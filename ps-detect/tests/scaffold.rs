//! SD1 scaffold test — verify golden centroid fixtures exist.

use std::path::Path;

#[test]
fn golden_fixtures_present_and_nonempty() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden_centroids.json");
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let data: serde_json::Value = serde_json::from_str(&body)
        .expect("parse golden_centroids.json");
    let images = data.as_object()
        .expect("golden_centroids.json root must be an object");
    assert!(!images.is_empty(), "must have at least one image entry");
    for (_filename, entry) in images {
        let centroids = entry.get("centroids")
            .expect("each image must have a 'centroids' array")
            .as_array()
            .expect("centroids must be an array");
        assert!(!centroids.is_empty(), "centroids for {} must be non-empty", _filename);
        for c in centroids {
            assert!(c.get("centroid_x").is_some(), "missing centroid_x");
            assert!(c.get("centroid_y").is_some(), "missing centroid_y");
            assert!(c.get("peak_value").is_some(), "missing peak_value");
            assert!(c.get("brightness").is_some(), "missing brightness");
            assert!(c.get("num_saturated").is_some(), "missing num_saturated");
        }
    }
}
