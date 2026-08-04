//! Solve a single image against a pattern database and print the result.
//!
//! Usage:
//!   cargo run --release -p plate-solver --example solve_image -- <db.npz> <image> <fov_estimate_deg> [timeout_ms]

use plate_solver::{solve_from_image, DetectParams};
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: solve_image <db.npz> <image> <fov_estimate_deg> [timeout_ms]");
        std::process::exit(2);
    }
    let db_path = &args[1];
    let img_path = &args[2];
    let fov_estimate: f64 = args[3].parse().expect("fov_estimate_deg");
    let timeout_ms: u64 = args
        .get(4)
        .map(|s| s.parse().expect("timeout_ms"))
        .unwrap_or(10_000);

    let t0 = Instant::now();
    let db = pattern_database::load_from_path(std::path::Path::new(db_path)).expect("load db");
    eprintln!(
        "db loaded in {:.0} ms: catalog={} stars={} patterns={}",
        t0.elapsed().as_secs_f64() * 1000.0,
        db.properties.star_catalog,
        db.num_stars,
        db.properties.num_patterns
    );

    let img = image::open(img_path).expect("open image").to_luma8();
    let (w, h) = img.dimensions();

    let detect_started = Instant::now();
    let stars =
        star_detection::detect_stars(img.as_raw(), w as usize, h as usize, 8.0, 1, false, true);
    eprintln!(
        "detected {} stars in {:.0} ms",
        stars.len(),
        detect_started.elapsed().as_secs_f64() * 1000.0
    );

    let solve_started = Instant::now();
    let sol = solve_from_image(
        img.as_raw(),
        w as usize,
        h as usize,
        Some(fov_estimate),
        0.0,
        0.01,
        1e-5,
        timeout_ms,
        0.0,
        0.002,
        db,
        DetectParams::default(),
    );
    let t_solve = solve_started.elapsed().as_secs_f64() * 1000.0;

    println!(
        "status={:?} t_solve={:.0}ms ra={:?} dec={:?} roll={:?} fov={:?} matches={} prob={:?} rmse={:?}",
        sol.status,
        t_solve,
        sol.ra.map(|v| v.to_degrees()),
        sol.dec.map(|v| v.to_degrees()),
        sol.roll.map(|v| v.to_degrees()),
        sol.fov_used.map(|v| v.to_degrees()),
        sol.matched_centroids.len(),
        sol.match_probability,
        sol.rmse,
    );
}
