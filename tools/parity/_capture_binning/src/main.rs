//! Capture binning parity fixture from a test image.
//!
//! Loads the named JPEG, bins it 2x2, and writes the top-left 8x8 pixel
//! values as JSON to stdout (redirect to fixture file).

use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: capture-binning <image-path>");
        std::process::exit(1);
    }

    let image_path = Path::new(&args[1]);
    let img = image::open(image_path)
        .expect("failed to open image")
        .to_luma8();

    // Use ps-detect's binning implementation.
    let binned = ps_detect::binning::bin_2x2(&img);

    let (bw, bh) = binned.dimensions();
    let region_w = (8.min(bw)) as usize;
    let region_h = (8.min(bh)) as usize;

    let mut pixels: Vec<Vec<u8>> = Vec::with_capacity(region_h);
    for y in 0..region_h {
        let mut row = Vec::with_capacity(region_w);
        for x in 0..region_w {
            row.push(binned.get_pixel(x as u32, y as u32)[0]);
        }
        pixels.push(row);
    }

    let fixture = serde_json::json!({
        "image": image_path.file_name().unwrap().to_string_lossy(),
        "binning": 2,
        "region": {"x": 0, "y": 0, "w": region_w, "h": region_h},
        "pixels": pixels
    });

    println!("{}", serde_json::to_string_pretty(&fixture).unwrap());
}
