//! ZCS.3 measurement tool: benchmark ExtractCentroids over the shmem path
//! at a given resolution, in-process (no real network round-trip), to
//! isolate the image-handling cost (copy vs zero-copy view) from
//! serialization/transport overhead.
//!
//! Usage: `shmem_bench <width> <height> <iterations>`

use ps_db::{Database, DatabaseProperties};
use ps_grpc::plate_solver::plate_solver_server::PlateSolver;
use ps_grpc::plate_solver::{CentroidsRequest, Image};
use ps_grpc::PlateSolverService;
use std::time::Instant;
use tonic::Request;

fn make_empty_db() -> Database {
    let props = DatabaseProperties::apply_legacy_fallbacks(
        None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
        None,
    );
    Database::empty(props)
}

/// RAII guard to remove the shmem file on drop.
struct ShmemGuard(String);

impl Drop for ShmemGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let width: i32 = args
        .get(1)
        .map(|s| s.parse().expect("width"))
        .unwrap_or(1024);
    let height: i32 = args
        .get(2)
        .map(|s| s.parse().expect("height"))
        .unwrap_or(768);
    let iterations: usize = args
        .get(3)
        .map(|s| s.parse().expect("iterations"))
        .unwrap_or(50);

    let pixel_count = (width as usize) * (height as usize);
    // Uniform background with no bright spots: detection finds nothing, so
    // the measured cost is dominated by the image-resolution path itself
    // (mmap/copy + GrayImage/GrayImageView construction + one noise-estimate
    // + one scan_image_for_candidates pass), not by star-detection work.
    let pixels = vec![64u8; pixel_count];

    let shmem_name = format!("zcs3_bench_{}x{}_{}", width, height, std::process::id());
    let shmem_path = format!("/dev/shm/{}", shmem_name);
    std::fs::write(&shmem_path, &pixels).expect("write shmem file");
    let _guard = ShmemGuard(shmem_path);

    let service = PlateSolverService::new(make_empty_db());

    // Warm up (page faults, allocator warm-up) before timing.
    for _ in 0..3 {
        let req = CentroidsRequest {
            input_image: Some(Image {
                width,
                height,
                image_data: vec![],
                shmem_name: Some(shmem_name.clone()),
                reopen_shmem: false,
            }),
            sigma: 10.0,
            binning: None,
            return_binned: false,
            use_binned_for_star_candidates: false,
            detect_hot_pixels: false,
            normalize_rows: false,
            estimate_background_region: None,
        };
        service
            .extract_centroids(Request::new(req))
            .await
            .expect("shmem extract_centroids should succeed");
    }

    let mut wall_times_s = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let req = CentroidsRequest {
            input_image: Some(Image {
                width,
                height,
                image_data: vec![],
                shmem_name: Some(shmem_name.clone()),
                reopen_shmem: false,
            }),
            sigma: 10.0,
            binning: None,
            return_binned: false,
            use_binned_for_star_candidates: false,
            detect_hot_pixels: false,
            normalize_rows: false,
            estimate_background_region: None,
        };
        let t0 = Instant::now();
        service
            .extract_centroids(Request::new(req))
            .await
            .expect("shmem extract_centroids should succeed");
        wall_times_s.push(t0.elapsed().as_secs_f64());
    }

    let n = wall_times_s.len() as f64;
    let mean = wall_times_s.iter().sum::<f64>() / n;
    let mut sorted = wall_times_s.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = sorted[sorted.len() / 2];
    let variance = wall_times_s.iter().map(|t| (t - mean).powi(2)).sum::<f64>() / n;
    let stdev = variance.sqrt();

    println!(
        "{}x{} ({} px): n={} mean={:.6}s median={:.6}s stdev={:.6}s mean_ms={:.4} median_ms={:.4}",
        width,
        height,
        pixel_count,
        wall_times_s.len(),
        mean,
        median,
        stdev,
        mean * 1000.0,
        median * 1000.0,
    );
}
