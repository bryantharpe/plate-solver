//! Benchmark the solver over a corpus of images, emitting JSON.
//!
//! Usage:
//!   cargo run --release -p plate-solver --example bench_corpus -- \
//!       <db.npz> <image_dir> <fov_estimate_deg> > out.json
//!
//! Each image is solved once as warmup, then five timed runs; wall-clock per
//! call covers star detection + solve, mirroring the Python benchmark.

use plate_solver::{solve_from_image, DetectParams, SolveStatus};
use std::time::Instant;

const TIMED_RUNS: usize = 5;
const TIMEOUT_MS: u64 = 10_000;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: bench_corpus <db.npz> <image_dir> <fov_estimate_deg>");
        std::process::exit(2);
    }
    let db = pattern_database::load_from_path(std::path::Path::new(&args[1])).expect("load db");
    let fov_estimate: f64 = args[3].parse().expect("fov_estimate_deg");

    let mut images: Vec<_> = std::fs::read_dir(&args[2])
        .expect("read image dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("jpg") | Some("jpeg") | Some("png") | Some("tiff")
            ) && p
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("2019-07-29T"))
        })
        .collect();
    images.sort();

    println!("[");
    for (i, img_path) in images.iter().enumerate() {
        let img = image::open(img_path).expect("open image").to_luma8();
        let (w, h) = (img.width() as usize, img.height() as usize);

        let solve = || {
            solve_from_image(
                img.as_raw(),
                w,
                h,
                Some(fov_estimate),
                0.0,
                0.01,
                1e-5,
                TIMEOUT_MS,
                0.0,
                0.002,
                db.clone(),
                DetectParams::default(),
            )
        };

        let _warmup = solve();
        let mut walls = Vec::new();
        let mut sol = None;
        for _ in 0..TIMED_RUNS {
            let t0 = Instant::now();
            sol = Some(solve());
            walls.push(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let sol = sol.unwrap();

        let name = img_path.file_name().unwrap().to_string_lossy();
        let solved = sol.status == Some(SolveStatus::MatchFound);
        eprintln!("rust {name}: solved={solved}");
        let fmt_opt = |v: Option<f64>| v.map(|x| x.to_string()).unwrap_or("null".into());
        println!(
            " {{\"solver\": \"plate-solver-rs\", \"image\": \"{}\", \"ra\": {}, \"dec\": {}, \"roll\": {}, \"fov\": {}, \"rmse\": {}, \"matches\": {}, \"prob\": {}, \"wall_ms\": {:?}}}{}",
            name,
            fmt_opt(sol.ra.map(|v| v.to_degrees())),
            fmt_opt(sol.dec.map(|v| v.to_degrees())),
            fmt_opt(sol.roll.map(|v| v.to_degrees())),
            fmt_opt(sol.fov_used.filter(|_| solved).map(|v| v.to_degrees())),
            fmt_opt(sol.rmse),
            sol.matched_centroids.len(),
            fmt_opt(sol.match_probability),
            walls,
            if i + 1 == images.len() { "" } else { "," }
        );
    }
    println!("]");
}
