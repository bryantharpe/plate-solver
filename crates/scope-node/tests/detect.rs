use plate_solver::DetectParams;
use scope_node::{
    detect::{self, DetectSettings},
    GrayFrame,
};
use serde_json::Value;
use std::path::PathBuf;

#[test]
fn cedar_defaults_equal_every_console_field() {
    let node = DetectSettings::cedar();
    let console = DetectParams::cedar();
    assert_eq!(node.sigma, console.sigma);
    assert_eq!(node.noise_estimate, console.noise_estimate);
    assert_eq!(node.binning, console.binning);
    assert_eq!(node.normalize_rows, console.normalize_rows);
    assert_eq!(node.detect_hot_pixels, console.detect_hot_pixels);
    assert_eq!(node.return_binned, console.return_binned);
    assert_eq!(
        node.use_binned_for_star_candidates,
        console.use_binned_for_star_candidates
    );
}

#[test]
fn saved_finder_frame_has_identical_nonempty_centroids_in_console_order() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/scope-node");
    let fixture: Value =
        serde_json::from_slice(&std::fs::read(root.join("fixtures.json")).unwrap()).unwrap();
    let frame = &fixture["finder_frame"];
    let image = image::open(root.join(frame["file"].as_str().unwrap()))
        .unwrap()
        .to_luma8();
    assert_eq!(image.width() as u64, frame["width"].as_u64().unwrap());
    assert_eq!(image.height() as u64, frame["height"].as_u64().unwrap());
    let gray = GrayFrame {
        width: image.width(),
        height: image.height(),
        pixels: image.into_raw(),
        start_s: 0.0,
        end_s: 0.02,
        exposure_ms: 20.0,
        gain: 1.0,
    };
    let settings = DetectParams::cedar();
    let console = star_detection::detect_stars(
        &gray.pixels,
        gray.width as usize,
        gray.height as usize,
        settings.sigma,
        settings.binning,
        settings.normalize_rows,
        settings.detect_hot_pixels,
    );
    let node = detect::detect(&gray, &DetectSettings::cedar());
    assert!(
        !node.is_empty(),
        "the saved finder frame must exercise detection"
    );
    assert_eq!(node.len(), console.len());
    for (a, b) in node.iter().zip(console) {
        // Exact equality: both paths call the console's detector, without reordering.
        assert_eq!((a.x, a.y, a.brightness), (b.x, b.y, b.brightness));
    }
}
