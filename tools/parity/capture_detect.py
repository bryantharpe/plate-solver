#!/usr/bin/env python3
"""Capture golden star centroids from cedar-detect's reference implementation.

Builds a temporary Cargo project that depends on cedar-detect (path dep),
loads test images, calls get_stars_from_image(sigma=8), and writes the
results to ps-detect/tests/fixtures/golden_centroids.json.
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]  # plate-solver root
CEDAR_DETECT = ROOT / "reference-solutions" / "cedar-detect"
TEST_DATA = CEDAR_DETECT / "test_data"
OUTPUT = ROOT / "ps-detect" / "tests" / "fixtures" / "golden_centroids.json"

IMAGES = [
    "2019-07-29T204726_Alt40_Azi-135_Try1.jpg",
    "hale_bopp.jpg",
]


def main():
    env = os.environ.copy()
    env["PATH"] = f"{Path.home()}/.cargo/bin:{env.get('PATH', '')}"

    # Create a temp directory outside the workspace to avoid workspace conflicts.
    tmp_root = tempfile.mkdtemp(prefix="capture_detect_")
    build_dir = Path(tmp_root) / "capture_detect"
    print(f"Build dir: {build_dir}")

    try:
        # Create the temp Cargo project.
        result = subprocess.run(
            ["cargo", "new", "--lib", "capture_detect"],
            cwd=Path(tmp_root),
            env=env,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            print(f"cargo new failed:\n{result.stderr}", file=sys.stderr)
            sys.exit(1)

        # Write Cargo.toml with cedar-detect as path dep.
        cargo_toml = build_dir / "Cargo.toml"
        cargo_toml.write_text(
            f"""[package]
name = "capture_detect"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "capture_detect"
path = "main.rs"

[dependencies]
cedar_detect = {{ path = "{CEDAR_DETECT}" }}
image = "0.25"
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
"""
        )

        # Write main.rs.
        main_rs = build_dir / "main.rs"
        main_rs.write_text(
            r'''use std::collections::BTreeMap;
use std::path::PathBuf;

use cedar_detect::algorithm::{get_stars_from_image, estimate_noise_from_image};

fn main() {
    let test_data: PathBuf = std::env::var("TEST_DATA_DIR")
        .expect("TEST_DATA_DIR must be set")
        .into();

    let image_list = std::env::var("IMAGE_LIST")
        .expect("IMAGE_LIST must be set");
    let filenames: Vec<&str> = image_list.split(',').collect();

    let mut result = BTreeMap::new();

    for &filename in &filenames {
        let image_path = test_data.join(filename);
        println!("Processing: {}", image_path.display());
        let img = image::open(&image_path)
            .unwrap_or_else(|e| panic!("failed to open {}: {}", image_path.display(), e));
        let gray = img.to_luma8();
        let (w, h) = gray.dimensions();
        println!("  size: {}x{}", w, h);

        let noise = estimate_noise_from_image(&gray);
        println!("  noise_estimate: {:.6}", noise);

        let (stars, hot_pixel_count, _binned, _histogram) =
            get_stars_from_image(&gray, noise, 8.0, false, 1, true, false);

        println!("  stars: {}, hot_pixels: {}", stars.len(), hot_pixel_count);

        let centroids: Vec<serde_json::Value> = stars.iter().map(|s| {
            serde_json::json!({
                "centroid_x": s.centroid_x,
                "centroid_y": s.centroid_y,
                "peak_value": s.peak_value as u8,
                "brightness": s.brightness,
                "num_saturated": s.num_saturated as u16,
            })
        }).collect();

        result.insert(filename.to_string(), serde_json::json!({
            "sigma": 8.0,
            "noise_estimate": noise,
            "hot_pixel_count": hot_pixel_count,
            "centroids": centroids,
        }));
    }

    let output_path = std::env::var("OUTPUT_PATH").expect("OUTPUT_PATH must be set");
    let json_str = serde_json::to_string_pretty(&result).unwrap();
    std::fs::write(&output_path, &json_str).unwrap_or_else(|e| {
        eprintln!("Failed to write {}: {}", output_path, e);
        println!("{}", json_str);
    });
    println!("Wrote {} entries to {}", result.len(), output_path);
}
'''
        )

        # Build and run.
        print("Building capture binary...")
        result = subprocess.run(
            ["cargo", "run", "--release"],
            cwd=build_dir,
            env={**env,
                 "TEST_DATA_DIR": str(TEST_DATA),
                 "IMAGE_LIST": ",".join(IMAGES),
                 "OUTPUT_PATH": str(OUTPUT)},
            capture_output=True,
            text=True,
        )
        print(result.stdout)
        if result.stderr:
            print(result.stderr, file=sys.stderr)
        if result.returncode != 0:
            print(f"capture_detect failed with code {result.returncode}", file=sys.stderr)
            sys.exit(1)

        # Verify output.
        if not OUTPUT.exists():
            print(f"ERROR: output file not found at {OUTPUT}", file=sys.stderr)
            sys.exit(1)

        data = json.loads(OUTPUT.read_text())
        for fname in IMAGES:
            entry = data.get(fname)
            if entry is None:
                print(f"ERROR: missing entry for {fname}", file=sys.stderr)
                sys.exit(1)
            count = len(entry.get("centroids", []))
            print(f"  {fname}: {count} centroids, noise={entry['noise_estimate']:.4f}")

        print(f"Golden data written to {OUTPUT}")
    finally:
        # Clean up temp dir.
        print(f"Cleaning up {tmp_root}")
        shutil.rmtree(tmp_root)


if __name__ == "__main__":
    main()
