//! Time the Rust pipeline (star-detection + plate-solver) against cedar-detect run
//! in-process, on the same decoded frames, with cedar's example settings: sigma 8,
//! binning 2, normalize_rows, hot pixels, brightest 30, no FOV estimate, distortion 0.
//!
//! usage: cedar-compare DB.npz IMAGE_DIR REPS NAME...   (one JSON object per line on stdout)
//!
//! Reads IMAGE_DIR/NAME.jpg, or IMAGE_DIR/NAME.pgm with BENCH_EXT=pgm (the gray frames
//! cedar_bench.py saved from PIL, so both pipelines see identical pixels).
//! See README.md; run.sh drives the whole comparison.

use std::path::Path;
use std::time::Instant;

use plate_solver::{solve_from_centroids, solve_from_image, DetectParams, Solution, SolveStatus};
use serde_json::{json, Value};

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

fn stats(mut v: Vec<f64>) -> Value {
    v.sort_by(f64::total_cmp);
    json!({ "median": v[v.len() / 2], "min": v[0], "max": v[v.len() - 1] })
}

fn sol_json(s: &Solution, db: &pattern_database::PatternDatabase) -> Value {
    let status = match s.status {
        Some(SolveStatus::MatchFound) => "MATCH_FOUND",
        Some(SolveStatus::NoMatch) => "NO_MATCH",
        Some(SolveStatus::Timeout) => "TIMEOUT",
        Some(SolveStatus::Cancelled) => "CANCELLED",
        Some(SolveStatus::TooFew) => "TOO_FEW",
        None => "UNKNOWN",
    };
    // matched_catalog_ids are star-table rows; map them to HIP numbers the way
    // tests/reference_parity.rs does.
    let mut ids: Vec<u64> = s
        .matched_catalog_ids
        .iter()
        .map(|&i| match db.star_catalog_ids[i] {
            pattern_database::CatalogId::Hip(n) => n as u64,
            other => panic!("unexpected catalog id {other:?}"),
        })
        .collect();
    ids.sort_unstable();
    json!({
        "status": status,
        "ra": s.ra.map(f64::to_degrees),
        "dec": s.dec.map(f64::to_degrees),
        "roll": s.roll.map(f64::to_degrees),
        "fov": s.camera.map(|c| c.fov.to_degrees()),
        "matches": s.matched_centroids.len(),
        "rmse": s.rmse,
        "hip": ids,
    })
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (db_path, dir, reps) = (&a[1], &a[2], a[3].parse::<usize>().unwrap());

    let t = Instant::now();
    let db = pattern_database::load_from_path(Path::new(db_path)).unwrap();
    let load_ms = ms(t);
    println!("{}", json!({ "kind": "setup", "db_load_ms": load_ms }));

    let params = DetectParams {
        sigma: 8.0,
        binning: 2,
        normalize_rows: true,
        detect_hot_pixels: true,
        ..DetectParams::default()
    };
    let ext = std::env::var("BENCH_EXT").unwrap_or_else(|_| "jpg".into());
    for name in &a[4..] {
        let path = format!("{dir}/{name}.{ext}");
        let t = Instant::now();
        let gray = image::ImageReader::open(&path)
            .unwrap()
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap()
            .to_luma8();
        let decode_ms = ms(t);
        let (w, h) = (gray.width() as usize, gray.height() as usize);
        let px = gray.as_raw();

        let (mut d_ours, mut d_cedar, mut s_ours, mut s_hyb, mut s_img) =
            (vec![], vec![], vec![], vec![], vec![]);
        let mut last = None;
        for _ in 0..reps {
            let t = Instant::now();
            let stars = star_detection::detect_stars(px, w, h, 8.0, 2, true, true);
            d_ours.push(ms(t));

            let t = Instant::now();
            let noise = cedar_detect::algorithm::estimate_noise_from_image(&gray);
            let (cstars, _, _, _) = cedar_detect::algorithm::get_stars_from_image(
                &gray, noise, 8.0, true, 2, true, false,
            );
            d_cedar.push(ms(t));

            let ours30: Vec<(f64, f64)> = stars.iter().take(30).map(|s| (s.y, s.x)).collect();
            let ced30: Vec<(f64, f64)> = cstars
                .iter()
                .take(30)
                .map(|s| (s.centroid_y, s.centroid_x))
                .collect();

            // The solver borrows the database (since #96), as the gRPC service does.
            let t = Instant::now();
            let so = solve_from_centroids(
                &ours30,
                (w, h),
                None,
                0.0,
                0.01,
                1e-5,
                5000,
                0.0,
                0.002,
                &db,
            );
            s_ours.push(ms(t));

            let t = Instant::now();
            let sh =
                solve_from_centroids(&ced30, (w, h), None, 0.0, 0.01, 1e-5, 5000, 0.0, 0.002, &db);
            s_hyb.push(ms(t));

            let t = Instant::now();
            let si = solve_from_image(
                px, w, h, None, 0.0, 0.01, 1e-5, 5000, 0.0, 0.002, &db, params,
            );
            s_img.push(ms(t));

            last = Some((stars.len(), cstars.len(), ours30, ced30, so, sh, si));
        }
        let (n_ours, n_cedar, ours30, ced30, so, sh, si) = last.unwrap();

        // Detection agreement: each of our top-30 centroids vs the nearest of cedar-detect's top 30.
        let nearest: Vec<f64> = ours30
            .iter()
            .map(|o| {
                ced30
                    .iter()
                    .map(|c| ((o.0 - c.0).powi(2) + (o.1 - c.1).powi(2)).sqrt())
                    .fold(f64::INFINITY, f64::min)
            })
            .collect();
        let within_01 = nearest.iter().filter(|d| **d <= 0.1).count();
        let within_1 = nearest.iter().filter(|d| **d <= 1.0).count();
        let max_off = nearest
            .iter()
            .copied()
            .filter(|d| *d <= 1.0)
            .fold(0.0, f64::max);
        let same_order = ours30.len() == ced30.len()
            && ours30
                .iter()
                .zip(&ced30)
                .all(|(o, c)| ((o.0 - c.0).powi(2) + (o.1 - c.1).powi(2)).sqrt() <= 0.1);

        println!(
            "{}",
            json!({
                "kind": "image", "image": name, "width": w, "height": h, "decode_ms": decode_ms,
                "stars_ours": n_ours, "stars_cedar_detect": n_cedar,
                "top30_within_0.1px": within_01, "top30_within_1px": within_1,
                "top30_max_offset_px": max_off, "top30_same_order": same_order,
                "detect_ours_ms": stats(d_ours), "detect_cedar_detect_ms": stats(d_cedar),
                "solve_ours_ms": stats(s_ours), "solve_on_cedar_centroids_ms": stats(s_hyb),
                "solve_from_image_ms": stats(s_img),
                "sol_ours": sol_json(&so, &db), "sol_on_cedar_centroids": sol_json(&sh, &db),
                "sol_from_image": sol_json(&si, &db),
                "ced30": ced30,
            })
        );
    }
}
