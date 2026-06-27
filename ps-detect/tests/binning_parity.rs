//! Parity test for 2x2 binning against golden pixel values.
//!
//! The fixture was captured by running our own implementation on the same
//! JPEG data (deterministic integer arithmetic), providing a regression
//! anchor for the binning code.

use std::path::Path;

use ps_detect::binning::bin_2x2;
use ps_detect::io::load_grayscale;

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
fn binning_2x2_parity() {
    let body = std::fs::read_to_string(fixture_path("binning_parity.json"))
        .expect("read binning_parity.json");
    let data: serde_json::Value = serde_json::from_str(&body).expect("parse binning_parity.json");

    let filename = data
        .get("image")
        .expect("missing image field")
        .as_str()
        .expect("image is a string");

    let region = data.get("region").expect("missing region field");
    let rw = region
        .get("w")
        .expect("missing w")
        .as_u64()
        .expect("w is u64") as usize;
    let rh = region
        .get("h")
        .expect("missing h")
        .as_u64()
        .expect("h is u64") as usize;

    let golden: Vec<Vec<u8>> = data
        .get("pixels")
        .expect("missing pixels field")
        .as_array()
        .expect("pixels is an array")
        .iter()
        .map(|row| {
            row.as_array()
                .expect("each row is an array")
                .iter()
                .map(|v| v.as_u64().expect("pixel is u64") as u8)
                .collect()
        })
        .collect();

    assert_eq!(golden.len(), rh, "row count mismatch");
    for (i, row) in golden.iter().enumerate() {
        assert_eq!(row.len(), rw, "col count mismatch at row {}", i);
    }

    let test_data = test_data_path();
    let img_path = test_data.join(filename);
    let img =
        load_grayscale(&img_path).unwrap_or_else(|e| panic!("load {}: {}", img_path.display(), e));

    let binned = bin_2x2(&img);
    let (bw, bh) = binned.dimensions();
    assert!(bw >= rw as u32, "binned width {} < region width {}", bw, rw);
    assert!(
        bh >= rh as u32,
        "binned height {} < region height {}",
        bh,
        rh
    );

    for y in 0..rh {
        for x in 0..rw {
            let actual = binned.get_pixel(x as u32, y as u32)[0];
            assert_eq!(
                actual, golden[y][x],
                "pixel mismatch at ({}, {}): got {}, expected {}",
                x, y, actual, golden[y][x]
            );
        }
    }
}
