//! Scaffold integrity check: the golden parity fixtures captured from the
//! reference (via `tools/parity/capture_core.py`) are present and well-formed
//! enough for the MC2–MC7 parity tests to consume. This does not yet validate
//! any math — it only proves the parity harness is wired into the crate.

use std::path::Path;

/// Every fixture file the core parity suite expects to exist.
const FIXTURES: &[&str] = &[
    "celestial_vectors.json",
    "angle_distance.json",
    "projection.json",
    "distortion.json",
    "rotation.json",
    "pattern_hash.json",
];

#[test]
fn golden_fixtures_present_and_nonempty() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for name in FIXTURES {
        let path = dir.join(name);
        let body = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("missing fixture {}: {e}", path.display()));
        let trimmed = body.trim_start();
        assert!(
            trimmed.starts_with('{'),
            "fixture {} is not a JSON object",
            path.display()
        );
        assert!(body.len() > 2, "fixture {} is empty", path.display());
    }
}
