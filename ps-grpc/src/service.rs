//! gRPC service implementation for the Plate Solver.

use crate::plate_solver::plate_solver_server::PlateSolver;
use crate::plate_solver::{
    CentroidsRequest, CentroidsResult, Image, ImageCoord, InfoRequest, MatchedStar, ServerInfo,
    Solution, SolveFromCentroidsRequest, SolveFromImageRequest, SolveStatus as ProtoSolveStatus,
    StarCentroid,
};
use memmap2::MmapOptions;
use prost_types::Duration;
use ps_db::Database;
use ps_detect::noise::estimate_noise_from_image;
use ps_detect::{get_stars_from_image, GrayImage, as_view};
use ps_solve::{
    solve_from_centroids as ps_solve_centroids, solve_from_image as ps_solve_image, DetectParams,
    SolveParams as PsSolveParams, SolveStatus as PsSolveStatus,
};
use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};
use tokio::time::sleep;
use tonic::{Request, Response, Status};

/// Backing storage for a request's input image: either an owned buffer
/// (inline `image_data`, already zero-copy per FU-A) or a memory-mapped
/// shared-memory segment (ZCS.2: no longer copied into an owned buffer).
enum ImageBacking {
    Owned(GrayImage),
    Shmem(memmap2::Mmap),
}

impl ImageBacking {
    /// Borrow this backing as a `GrayImageView` of the given dimensions.
    /// Returns `None` if the backing's byte length doesn't match
    /// `width * height` (shmem case only — the owned case is validated
    /// before construction and `as_view` on a `GrayImage` is infallible).
    fn as_view(&self, width: u32, height: u32) -> Option<ps_detect::GrayImageView<'_>> {
        match self {
            ImageBacking::Owned(img) => Some(as_view(img)),
            ImageBacking::Shmem(mmap) => {
                ps_detect::GrayImageView::from_raw(width, height, &mmap[..])
            }
        }
    }
}

pub struct PlateSolverService {
    db: Arc<Database>,
}

impl PlateSolverService {
    pub fn new(db: Database) -> Self {
        Self { db: Arc::new(db) }
    }
}

/// Map a ps_solve SolveStatus to the proto SolveStatus i32 value.
fn map_status(status: PsSolveStatus) -> i32 {
    match status {
        PsSolveStatus::MatchFound => ProtoSolveStatus::MatchFound as i32,
        PsSolveStatus::NoMatch => ProtoSolveStatus::NoMatch as i32,
        PsSolveStatus::Timeout => ProtoSolveStatus::Timeout as i32,
        PsSolveStatus::Cancelled => ProtoSolveStatus::Cancelled as i32,
        PsSolveStatus::TooFew => ProtoSolveStatus::TooFew as i32,
    }
}

/// Map a ps_solve Solution to the proto Solution message.
fn map_solution(sol: &ps_solve::Solution, return_matches: bool, t_extract_ms: f64) -> Solution {
    let matched = if return_matches
        && sol.matched_centroids.is_some()
        && sol.matched_stars.is_some()
        && sol.matched_cat_ids.is_some()
    {
        #[allow(clippy::unnecessary_unwrap)]
        let centroids = sol.matched_centroids.as_ref().unwrap();
        #[allow(clippy::unnecessary_unwrap)]
        let stars = sol.matched_stars.as_ref().unwrap();
        #[allow(clippy::unnecessary_unwrap)]
        let cat_ids = sol.matched_cat_ids.as_ref().unwrap();
        centroids
            .iter()
            .zip(stars.iter())
            .zip(cat_ids.iter())
            .map(|((yx, rdm), cat_id)| MatchedStar {
                centroid: Some(ImageCoord {
                    x: yx[1], // swap: proto uses (x,y)
                    y: yx[0],
                }),
                ra: rdm[0],
                dec: rdm[1],
                mag: rdm[2],
                cat_id: *cat_id as i64,
            })
            .collect()
    } else {
        Vec::new()
    };

    Solution {
        status: map_status(sol.status.clone()),
        ra: sol.ra,
        dec: sol.dec,
        roll: sol.roll,
        fov: sol.fov,
        distortion: sol.distortion,
        rmse: sol.rmse,
        p90e: sol.p90e,
        maxe: sol.maxe,
        matches: sol.matches as i32,
        prob: sol.prob,
        t_extract_ms,
        t_solve_ms: sol.t_solve * 1000.0,
        matched,
    }
}

/// Parse the gRPC `grpc-timeout` metadata header from the request.
/// Returns the deadline as a `std::time::Duration`, or `None` if absent/unparseable.
/// Format: <up-to-8 digits><unit> where unit ∈ H/M/S/m/u/n
/// (gRPC spec §A6-client-retries, §grpc-timeout header definition)
fn rpc_deadline<T>(request: &tonic::Request<T>) -> Option<StdDuration> {
    let val = request.metadata().get("grpc-timeout")?;
    let s = val.to_str().ok()?;
    let (digits, unit) = s.split_at(s.len().checked_sub(1)?);
    let n: u64 = digits.parse().ok()?;
    match unit {
        "H" => Some(StdDuration::from_secs(n * 3600)),
        "M" => Some(StdDuration::from_secs(n * 60)),
        "S" => Some(StdDuration::from_secs(n)),
        "m" => Some(StdDuration::from_millis(n)),
        "u" => Some(StdDuration::from_micros(n)),
        "n" => Some(StdDuration::from_nanos(n)),
        _ => None,
    }
}

/// Map proto SolveParams to ps_solve SolveParams.
fn map_params(params: &crate::plate_solver::SolveParams) -> PsSolveParams {
    PsSolveParams {
        fov_estimate: params.fov_estimate,
        fov_max_error: params.fov_max_error,
        match_radius: params.match_radius.unwrap_or(0.01),
        match_threshold: params.match_threshold.unwrap_or(1e-5),
        match_max_error: 0.002,
        solve_timeout: params.solve_timeout_ms.map(|ms| ms as u64),
        distortion: params.distortion,
        cancel_flag: None,
    }
}

/// Validates a client-supplied width/height pair: both must be positive and
/// within a sane maximum (matches ps-web's MAX_IMAGE_DIMENSION convention).
const MAX_IMAGE_DIMENSION: i32 = 20_000;

#[allow(clippy::result_large_err)]
fn validate_dimensions(width: i32, height: i32) -> Result<(u32, u32), Status> {
    if width <= 0 || width > MAX_IMAGE_DIMENSION {
        return Err(Status::invalid_argument(format!(
            "width must be in 1..={}, got {}",
            MAX_IMAGE_DIMENSION, width
        )));
    }
    if height <= 0 || height > MAX_IMAGE_DIMENSION {
        return Err(Status::invalid_argument(format!(
            "height must be in 1..={}, got {}",
            MAX_IMAGE_DIMENSION, height
        )));
    }
    // Both are now known positive and <= MAX_IMAGE_DIMENSION, so these
    // conversions and the u32 multiply below cannot wrap.
    Ok((width as u32, height as u32))
}

#[async_trait::async_trait]
impl PlateSolver for PlateSolverService {
    async fn extract_centroids(
        &self,
        request: Request<CentroidsRequest>,
    ) -> Result<Response<CentroidsResult>, Status> {
        let req = request.into_inner();

        // Move the input image out of the request so its inline `image_data`
        // buffer can be moved into GrayImage without a full-frame clone
        // (~0.79 MB at 1024x768). `req` keeps the other CentroidsRequest fields
        // (sigma, binning, etc.) for use below.
        let input_image = req
            .input_image
            .ok_or_else(|| Status::invalid_argument("missing input_image"))?;

        // reopen_shmem: we open fresh per request, so reopen is implicit.
        let _reopen = input_image.reopen_shmem;

        let (width, height) = validate_dimensions(input_image.width, input_image.height)?;

        // Resolve image bytes (shmem or inline). The inline path moves
        // `image_data` directly (no clone, per FU-A). The shmem path now
        // uses a borrowed view over the mmap (ZCS.2: zero-copy via the
        // GrayImageView API from ps-detect).
        let expected_len = (width * height) as usize;
        let backing = if let Some(shmem_name) = input_image.shmem_name {
            let path = format!("/dev/shm/{}", shmem_name);
            let file = File::open(&path)
                .map_err(|e| Status::internal(format!("shmem open failed: {}: {}", path, e)))?;
            let mmap = unsafe { MmapOptions::new().map(&file) }
                .map_err(|e| Status::internal(format!("shmem mmap failed: {}: {}", path, e)))?;
            if mmap.len() != expected_len {
                return Err(Status::invalid_argument(format!(
                    "shmem region length {} does not match width*height {}*{}={}",
                    mmap.len(), width, height, expected_len
                )));
            }
            ImageBacking::Shmem(mmap)
        } else {
            let image_data = input_image.image_data;
            if image_data.len() != expected_len {
                return Err(Status::invalid_argument(format!(
                    "image_data length {} does not match width*height {}*{}={}",
                    image_data.len(), width, height, expected_len
                )));
            }
            let image = GrayImage::from_raw(width, height, image_data)
                .ok_or_else(|| Status::invalid_argument("failed to construct GrayImage"))?;
            ImageBacking::Owned(image)
        };
        let image_view = backing
            .as_view(width, height)
            .ok_or_else(|| Status::invalid_argument("failed to construct image view"))?;

        // Estimate noise.
        let noise_estimate = estimate_noise_from_image(&image_view);

        // Parameters from request.
        let sigma = req.sigma;
        let normalize_rows = req.normalize_rows;
        let detect_hot_pixels = req.detect_hot_pixels;
        let return_binned = req.return_binned;
        let use_binned = req.use_binned_for_star_candidates;

        // Determine effective binning per cedar-detect reference:
        //   if use_binned_for_star_candidates || return_binned:
        //     match binning: None -> 2, Some(2) -> 2, Some(4) -> 4, other -> INVALID_ARGUMENT
        //   else: 1
        let need_binning = use_binned || return_binned;
        let effective_binning: u32 = if need_binning {
            match req.binning {
                None => 2,
                Some(2) | Some(4) => req.binning.unwrap() as u32,
                Some(other) => {
                    return Err(Status::invalid_argument(format!(
                        "binning must be 2 or 4, got {}",
                        other
                    )))
                }
            }
        } else {
            1
        };

        // Run detection.
        let start = Instant::now();
        let (stars, hot_pixel_count, binned_image, _histogram) = get_stars_from_image(
            &image_view,
            noise_estimate,
            sigma,
            normalize_rows,
            effective_binning,
            detect_hot_pixels,
            return_binned,
        ).map_err(|e| Status::invalid_argument(e.to_string()))?;
        let elapsed = start.elapsed();
        let algorithm_time = Duration {
            seconds: elapsed.as_secs() as i64,
            nanos: elapsed.subsec_nanos() as i32,
        };

        // Peak star pixel value: average of the NUM_PEAKS brightest stars, fallback 255.
        const NUM_PEAKS: usize = 10;
        let (sum_peak, num_peak) = stars
            .iter()
            .take(NUM_PEAKS)
            .fold((0i32, 0i32), |(s, n), star| {
                (s + star.peak_value as i32, n + 1)
            });
        let peak_star_pixel = if num_peak > 0 {
            sum_peak / num_peak
        } else {
            255
        };

        // Map stars to StarCentroid proto messages.
        let star_candidates: Vec<StarCentroid> = stars
            .into_iter()
            .map(|star| StarCentroid {
                centroid_position: Some(ImageCoord {
                    x: star.centroid_x,
                    y: star.centroid_y,
                }),
                brightness: star.brightness,
                num_saturated: star.num_saturated as i32,
            })
            .collect();

        // Optional binned image.
        let binned_image_proto = if return_binned {
            binned_image.map(|bimg| Image {
                width: bimg.width() as i32,
                height: bimg.height() as i32,
                image_data: bimg.into_raw(),
                shmem_name: None,
                reopen_shmem: false,
            })
        } else {
            None
        };

        Ok(Response::new(CentroidsResult {
            noise_estimate,
            background_estimate: None,
            hot_pixel_count,
            peak_star_pixel,
            star_candidates,
            binned_image: binned_image_proto,
            algorithm_time: Some(algorithm_time),
        }))
    }

    async fn solve_from_centroids(
        &self,
        request: Request<SolveFromCentroidsRequest>,
    ) -> Result<Response<Solution>, Status> {
        let deadline = rpc_deadline(&request);
        let req = request.into_inner();

        // (x,y) → (y,x) swap at RPC boundary.
        let centroids_yx: Vec<[f64; 2]> = req.centroids.iter().map(|c| [c.y, c.x]).collect();

        // Step 2: Extract image dimensions.
        let (width_u32, height_u32) = validate_dimensions(req.width, req.height)?;
        let height = height_u32 as usize;
        let width = width_u32 as usize;

        let default_params = crate::plate_solver::SolveParams::default();
        let params_msg = req.params.as_ref().unwrap_or(&default_params);
        let mut solve_params = map_params(params_msg);
        let return_matches = params_msg.return_matches;

        let db = Arc::clone(&self.db);

        if let Some(dur) = deadline {
            let cancel_flag = Arc::new(AtomicBool::new(false));
            solve_params.cancel_flag = Some(Arc::clone(&cancel_flag));
            let flag_for_timer = Arc::clone(&cancel_flag);
            let handle = tokio::task::spawn_blocking(move || {
                ps_solve_centroids(&db, &centroids_yx, (height, width), &solve_params)
            });
            tokio::spawn(async move {
                sleep(dur).await;
                flag_for_timer.store(true, Ordering::Relaxed);
            });
            let sol = handle.await.map_err(|e| Status::internal(format!("solve task failed: {e}")))?;
            Ok(Response::new(map_solution(&sol, return_matches, 0.0)))
        } else {
            let sol = tokio::task::spawn_blocking(move || {
                ps_solve_centroids(&db, &centroids_yx, (height, width), &solve_params)
            }).await.map_err(|e| Status::internal(format!("solve task failed: {e}")))?;
            Ok(Response::new(map_solution(&sol, return_matches, 0.0)))
        }
    }

    async fn solve_from_image(
        &self,
        request: Request<SolveFromImageRequest>,
    ) -> Result<Response<Solution>, Status> {
        let deadline = rpc_deadline(&request);
        let req = request.into_inner();

        // Take the CentroidsRequest + its input image out of the request so the
        // inline buffer can be moved into GrayImage without a full-frame clone
        // (~0.79 MB at 1024x768). `req` keeps `params` for use below.
        let extract_req = req
            .extract
            .ok_or_else(|| Status::invalid_argument("missing extract"))?;
        let input_image = extract_req
            .input_image
            .ok_or_else(|| Status::invalid_argument("missing input_image in extract"))?;

        let (width, height) = validate_dimensions(input_image.width, input_image.height)?;

        // Resolve image bytes (shmem or inline). The inline path moves
        // `image_data` directly (no clone, per FU-A). The shmem path now
        // uses a borrowed view over the mmap (ZCS.2: zero-copy via the
        // GrayImageView API from ps-detect).
        let expected_len = (width * height) as usize;
        let backing = if let Some(shmem_name) = input_image.shmem_name {
            let path = format!("/dev/shm/{}", shmem_name);
            let file = File::open(&path)
                .map_err(|e| Status::internal(format!("shmem open failed: {}: {}", path, e)))?;
            let mmap = unsafe { MmapOptions::new().map(&file) }
                .map_err(|e| Status::internal(format!("shmem mmap failed: {}: {}", path, e)))?;
            if mmap.len() != expected_len {
                return Err(Status::invalid_argument(format!(
                    "shmem region length {} does not match width*height {}*{}={}",
                    mmap.len(), width, height, expected_len
                )));
            }
            ImageBacking::Shmem(mmap)
        } else {
            let image_data = input_image.image_data;
            if image_data.len() != expected_len {
                return Err(Status::invalid_argument(format!(
                    "image_data length {} does not match width*height {}*{}={}",
                    image_data.len(), width, height, expected_len
                )));
            }
            let image = GrayImage::from_raw(width, height, image_data)
                .ok_or_else(|| Status::invalid_argument("failed to construct GrayImage"))?;
            ImageBacking::Owned(image)
        };
        // Validate up front that the backing can produce a view, then drop the
        // borrow: the solve runs on a blocking thread, so `backing` (owned buffer
        // or mmap, both 'static) is moved in and the zero-copy view is rebuilt
        // inside the closure.
        backing
            .as_view(width, height)
            .ok_or_else(|| Status::invalid_argument("failed to construct image view"))?;

        // Map SolveParams.
        let default_params = crate::plate_solver::SolveParams::default();
        let params_msg = req.params.as_ref().unwrap_or(&default_params);
        let mut solve_params = map_params(params_msg);
        let return_matches = params_msg.return_matches;

        // Map detection params from the request (client-controlled sigma/binning/etc).
        let raw_sigma = extract_req.sigma;
        let sigma = if raw_sigma > 0.0 { raw_sigma } else { 4.0 };
        let effective_binning: u32 = if let Some(b) = extract_req.binning {
            match b {
                2 | 4 => b as u32,
                _ => {
                    return Err(Status::invalid_argument(format!(
                        "binning must be 2 or 4, got {}",
                        b
                    )));
                }
            }
        } else {
            1u32
        };
        let detect = DetectParams {
            sigma,
            binning: effective_binning,
            normalize_rows: extract_req.normalize_rows,
            detect_hot_pixels: extract_req.detect_hot_pixels,
        };
        // Solve on a blocking thread. `solve_from_image` self-reports the extraction
        // wall-clock in `t_extract` (seconds); convert to ms for the wire field.
        // When the RPC carries a deadline, a timer trips `cancel_flag` so the solver
        // bails out instead of running past the client's timeout.
        let db = Arc::clone(&self.db);

        if let Some(dur) = deadline {
            let cancel_flag = Arc::new(AtomicBool::new(false));
            solve_params.cancel_flag = Some(Arc::clone(&cancel_flag));
            let flag_for_timer = Arc::clone(&cancel_flag);
            let db2 = Arc::clone(&db);
            let handle = tokio::task::spawn_blocking(move || {
                let view = backing
                    .as_view(width, height)
                    .expect("image view validated before the solve task was spawned");
                ps_solve_image(&db2, &view, &solve_params, &detect)
            });
            tokio::spawn(async move {
                sleep(dur).await;
                flag_for_timer.store(true, Ordering::Relaxed);
            });
            let sol = handle.await.map_err(|e| Status::internal(format!("solve task failed: {e}")))?;
            Ok(Response::new(map_solution(&sol, return_matches, sol.t_extract * 1000.0)))
        } else {
            let sol = tokio::task::spawn_blocking(move || {
                let view = backing
                    .as_view(width, height)
                    .expect("image view validated before the solve task was spawned");
                ps_solve_image(&db, &view, &solve_params, &detect)
            }).await.map_err(|e| Status::internal(format!("solve task failed: {e}")))?;
            Ok(Response::new(map_solution(&sol, return_matches, sol.t_extract * 1000.0)))
        }
    }

    async fn get_info(
        &self,
        _request: Request<InfoRequest>,
    ) -> Result<Response<ServerInfo>, Status> {
        let p = &self.db.properties;
        Ok(Response::new(ServerInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            star_catalog: p.star_catalog.clone(),
            min_fov: p.min_fov as f64,
            max_fov: p.max_fov as f64,
            num_patterns: p.num_patterns as i64,
            epoch_equinox: p.epoch_equinox as f64,
            epoch_proper_motion: p.epoch_proper_motion as f64,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plate_solver::{CentroidsRequest, Image, SolveParams};
    use ps_db::DatabaseProperties;
    use std::path::Path;
    use std::time::Duration as StdDuration;

    /// Helper: build an empty database for testing.
    fn make_empty_db() -> Database {
        let props = DatabaseProperties::apply_legacy_fallbacks(
            None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            None, None,
        );
        Database::empty(props)
    }

    /// Helper: create a CentroidsRequest with inline image data.
    fn make_inline_request(
        image_data: Vec<u8>,
        width: i32,
        height: i32,
        sigma: f64,
    ) -> CentroidsRequest {
        CentroidsRequest {
            input_image: Some(Image {
                width,
                height,
                image_data,
                shmem_name: None,
                reopen_shmem: false,
            }),
            sigma,
            binning: None,
            return_binned: false,
            use_binned_for_star_candidates: false,
            detect_hot_pixels: false,
            normalize_rows: false,
            estimate_background_region: None,
        }
    }

    /// Helper: create a CentroidsRequest with shmem_name set.
    fn make_shmem_request(shmem_name: String) -> CentroidsRequest {
        CentroidsRequest {
            input_image: Some(Image {
                width: 64,
                height: 64,
                image_data: vec![],
                shmem_name: Some(shmem_name),
                reopen_shmem: false,
            }),
            sigma: 10.0,
            binning: None,
            return_binned: false,
            use_binned_for_star_candidates: false,
            detect_hot_pixels: false,
            normalize_rows: false,
            estimate_background_region: None,
        }
    }

    /// Test: extract_centroids with inline image data finds stars.
    #[tokio::test]
    async fn extract_centroids_inline_basic() {
        // Load a real test image from the reference data.
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let img_path = manifest
            .parent()
            .expect("need workspace root")
            .join("reference-solutions/cedar-detect/test_data/tree.jpg");

        let img = image::open(&img_path)
            .unwrap_or_else(|e| panic!("open {}: {e}", img_path.display()))
            .into_luma8();

        let width = img.width() as i32;
        let height = img.height() as i32;
        let data = img.into_raw();

        let request = make_inline_request(data, width, height, 10.0);

        let service = PlateSolverService::new(make_empty_db());
        let result = service
            .extract_centroids(Request::new(request))
            .await
            .expect("extract_centroids should succeed");
        let resp = result.into_inner();

        // Must find at least one star.
        assert!(
            !resp.star_candidates.is_empty(),
            "expected at least one star candidate, got {}",
            resp.star_candidates.len()
        );

        // Centroids should be brightest-first (brightness descending).
        for i in 1..resp.star_candidates.len() {
            assert!(
                resp.star_candidates[i - 1].brightness >= resp.star_candidates[i].brightness,
                "brightness not descending at index {}",
                i
            );
        }

        // Noise estimate should be positive.
        assert!(
            resp.noise_estimate > 0.0,
            "noise_estimate should be > 0, got {}",
            resp.noise_estimate
        );

        // Algorithm time should be recorded.
        let algo_time = resp.algorithm_time.expect("algorithm_time present");
        assert!(
            algo_time.nanos > 0 || algo_time.seconds > 0,
            "algorithm_time should be > 0, got {:?}",
            algo_time
        );

        // Peak star pixel should be positive for a real star field.
        assert!(
            resp.peak_star_pixel > 0,
            "peak_star_pixel should be > 0, got {}",
            resp.peak_star_pixel
        );
    }

    /// Test: invalid binning value returns INVALID_ARGUMENT when binning is needed.
    #[tokio::test]
    async fn extract_centroids_invalid_binning() {
        let data = vec![128u8; 64 * 64];

        let mut request = make_inline_request(data, 64, 64, 10.0);
        request.binning = Some(3); // invalid: must be 2 or 4 when binning is needed
        request.use_binned_for_star_candidates = true;

        let service = PlateSolverService::new(make_empty_db());
        let result = service.extract_centroids(Request::new(request)).await;

        assert!(
            result.is_err(),
            "expected an error for invalid binning, got Ok"
        );
        let err = result.unwrap_err();
        assert_eq!(
            err.code(),
            tonic::Code::InvalidArgument,
            "expected InvalidArgument status, got {:?}",
            err.code()
        );
    }

    /// Test: shmem_name pointing to nonexistent file returns INTERNAL.
    #[tokio::test]
    async fn extract_centroids_shmem_failure_returns_internal() {
        let request = make_shmem_request("nonexistent_shmem_xyzzy".to_string());

        let service = PlateSolverService::new(make_empty_db());
        let result = service.extract_centroids(Request::new(request)).await;

        assert!(
            result.is_err(),
            "expected an error for nonexistent shmem, got Ok"
        );
        let err = result.unwrap_err();
        assert_eq!(
            err.code(),
            tonic::Code::Internal,
            "expected Internal status, got {:?}",
            err.code()
        );
    }

    /// Test: mismatched image dimensions return INVALID_ARGUMENT.
    #[tokio::test]
    async fn extract_centroids_bad_dimensions() {
        // 100 bytes of data but claim width=20, height=20 (400 expected).
        let data = vec![128u8; 100];

        let request = make_inline_request(data, 20, 20, 10.0);

        let service = PlateSolverService::new(make_empty_db());
        let result = service.extract_centroids(Request::new(request)).await;

        assert!(
            result.is_err(),
            "expected an error for bad dimensions, got Ok"
        );
        let err = result.unwrap_err();
        assert_eq!(
            err.code(),
            tonic::Code::InvalidArgument,
            "expected InvalidArgument status, got {:?}",
            err.code()
        );
    }

    /// Test: solve_from_centroids with (x,y) -> (y,x) swap does not panic
    /// and returns a valid status (not UNIMPLEMENTED).
    #[tokio::test]
    async fn solve_from_centroids_swap_test() {
        let service = PlateSolverService::new(make_empty_db());

        // Create centroids with known (x, y) values.
        let centroids = vec![
            ImageCoord { x: 10.0, y: 20.0 },
            ImageCoord { x: 30.0, y: 40.0 },
            ImageCoord { x: 50.0, y: 60.0 },
            ImageCoord { x: 70.0, y: 80.0 },
            ImageCoord { x: 90.0, y: 10.0 },
        ];

        let request = SolveFromCentroidsRequest {
            centroids,
            width: 100,
            height: 100,
            params: Some(SolveParams {
                solve_timeout_ms: Some(5000),
                ..Default::default()
            }),
        };

        let result = service.solve_from_centroids(Request::new(request)).await;

        // Should succeed (not return UNIMPLEMENTED or INTERNAL error).
        assert!(
            result.is_ok(),
            "solve_from_centroids should return Ok, got {:?}",
            result.err()
        );

        let resp = result.unwrap().into_inner();

        // With an empty DB and no patterns, the solver will exhaust combinations
        // and return NoMatch (or Timeout with 0ms). Either way, status should be
        // a valid enum value, not an error.
        assert!(
            resp.status >= 0 && resp.status <= 4,
            "status={} should be a valid SolveStatus (0-4)",
            resp.status
        );

        // Specifically, with 5 centroids and no patterns in the DB:
        // The outer loop over combinations_4 runs but find no slots, so NoMatch.
        assert_eq!(
            resp.status,
            ProtoSolveStatus::NoMatch as i32,
            "expected NoMatch (1) for empty DB, got status={}",
            resp.status
        );

        // t_extract_ms should be 0.0 for SolveFromCentroids.
        assert_eq!(resp.t_extract_ms, 0.0);
    }

    /// Integration test: solve_from_image on a reference image returns MATCH_FOUND.
    #[tokio::test]
    async fn solve_from_image_parity() {
        use ps_db::{importer, loader};
        use tempfile::NamedTempFile;

        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));

        // Import the reference NPZ database.
        let npz_path =
            manifest.join("../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
        let db_imported =
            importer::import_npz(&npz_path).unwrap_or_else(|e| panic!("import_npz failed: {}", e));

        // Save -> load native round-trip.
        let tmp = NamedTempFile::new().expect("tempfile");
        loader::save_native(&db_imported, tmp.path()).expect("save_native");
        let mut db = loader::load_native(tmp.path()).expect("load_native");
        db.build_kd_tree();

        // Load the reference image.
        let img_path = manifest.join(
            "../reference-solutions/cedar-solve/examples/data/medium_fov/2019-07-29T204726_Alt40_Azi-135_Try1.jpg",
        );
        let img = image::open(&img_path)
            .unwrap_or_else(|e| panic!("Cannot open {:?}: {}", img_path, e))
            .into_luma8();

        let width = img.width() as i32;
        let height = img.height() as i32;
        let data = img.into_raw();

        // Build the request.
        let extract_req = CentroidsRequest {
            input_image: Some(Image {
                width,
                height,
                image_data: data,
                shmem_name: None,
                reopen_shmem: false,
            }),
            sigma: 4.0,
            binning: None,
            return_binned: false,
            use_binned_for_star_candidates: false,
            detect_hot_pixels: true,
            normalize_rows: false,
            estimate_background_region: None,
        };

        let params = SolveParams {
            solve_timeout_ms: Some(120000),
            ..Default::default()
        };

        let request = SolveFromImageRequest {
            extract: Some(extract_req),
            params: Some(params),
        };

        let service = PlateSolverService::new(db);
        let result = service
            .solve_from_image(Request::new(request))
            .await
            .expect("solve_from_image should succeed");
        let resp = result.into_inner();

        assert_eq!(
            resp.status,
            ProtoSolveStatus::MatchFound as i32,
            "expected MATCH_FOUND (0), got status={}",
            resp.status
        );

        // FUA.1: SolveFromImage self-reports real extraction time (not hard-coded 0.0).
        // SolveFromCentroids legitimately reports 0.0 (no extraction); SolveFromImage does not.
        assert!(
            resp.t_extract_ms > 0.0,
            "SolveFromImage should report t_extract_ms > 0, got {}",
            resp.t_extract_ms
        );
    }

    /// Regression guard: solve_from_centroids with empty centroids returns TOO_FEW.
    #[tokio::test]
    async fn solve_from_centroids_returns_too_few() {
        let service = PlateSolverService::new(make_empty_db());

        let request = SolveFromCentroidsRequest {
            centroids: vec![], // empty
            width: 100,
            height: 100,
            params: Some(SolveParams {
                ..Default::default()
            }),
        };

        let result = service
            .solve_from_centroids(Request::new(request))
            .await
            .expect("should not return an error");
        let resp = result.into_inner();

        assert_eq!(
            resp.status,
            ProtoSolveStatus::TooFew as i32,
            "expected TOO_FEW (4) for empty centroids, got status={}",
            resp.status
        );
    }

    /// Test: get_info returns database properties correctly.
    #[tokio::test]
    async fn get_info_returns_db_properties() {
        let props = DatabaseProperties::apply_legacy_fallbacks(
            None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            None, None,
        );
        let db = Database::empty(props);
        let service = PlateSolverService::new(db);

        let result = service
            .get_info(Request::new(InfoRequest::default()))
            .await
            .expect("get_info should succeed");
        let resp = result.into_inner();

        assert_eq!(resp.min_fov, 10.0);
        assert_eq!(resp.max_fov, 30.0);
        assert_eq!(resp.num_patterns, 0);
        assert_eq!(resp.epoch_equinox, 2000.0);
        assert!(!resp.star_catalog.is_empty());
        assert_eq!(resp.star_catalog, "hip_main");
        assert_eq!(resp.epoch_proper_motion, 2015.5);
        assert_eq!(resp.version, env!("CARGO_PKG_VERSION"));
    }

    /// Helper: RAII guard to clean up a file under `/dev/shm/` on drop.
    struct ShmemGuard {
        path: String,
    }

    impl Drop for ShmemGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    /// Test: extract_centroids with shmem_name pointing to real shared-memory
    /// file produces identical results to the inline image_data path.
    #[tokio::test]
    async fn extract_centroids_shmem_success() {
        // Use a small synthetic grayscale image: 64x64 of mostly background (128)
        // with three small bright spots (200) for stars.
        let width = 64u32;
        let height = 64u32;
        let mut pixels = vec![128u8; (width * height) as usize];

        // Place three bright spots at different positions
        for &(cx, cy) in &[(10u32, 10u32), (40u32, 20u32), (55u32, 50u32)] {
            for dy in 0..3 {
                for dx in 0..3 {
                    let x = cx + dx;
                    let y = cy + dy;
                    if x < width && y < height {
                        pixels[(y * width + x) as usize] = 200;
                    }
                }
            }
        }

        // Create the shmem file with a unique name using PID.
        let shmem_name = format!("test_extract_centroids_shmem_{}", std::process::id());
        let shmem_path = format!("/dev/shm/{}", shmem_name);
        let _guard = ShmemGuard {
            path: shmem_path.clone(),
        };

        // Write pixels to the shmem file.
        std::fs::write(&shmem_path, &pixels).expect("write shmem file");

        let service = PlateSolverService::new(make_empty_db());

        // Call extract_centroids with shmem_name.
        let shmem_req = CentroidsRequest {
            input_image: Some(Image {
                width: width as i32,
                height: height as i32,
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

        let shmem_result = service
            .extract_centroids(Request::new(shmem_req))
            .await
            .expect("extract_centroids with shmem should succeed");
        let shmem_resp = shmem_result.into_inner();

        // Call extract_centroids with inline image_data.
        let inline_req = make_inline_request(pixels, width as i32, height as i32, 10.0);
        let inline_result = service
            .extract_centroids(Request::new(inline_req))
            .await
            .expect("extract_centroids with inline should succeed");
        let inline_resp = inline_result.into_inner();

        // Results must be identical (same noise_estimate, same star_candidates count, etc.).
        assert_eq!(shmem_resp.noise_estimate, inline_resp.noise_estimate);
        assert_eq!(
            shmem_resp.star_candidates.len(),
            inline_resp.star_candidates.len(),
            "star candidate count must match between shmem and inline"
        );

        // Verify centroids are identical
        for (i, (shmem_star, inline_star)) in shmem_resp
            .star_candidates
            .iter()
            .zip(inline_resp.star_candidates.iter())
            .enumerate()
        {
            assert_eq!(
                shmem_star.brightness, inline_star.brightness,
                "brightness mismatch at star {}",
                i
            );
            let shmem_pos = shmem_star.centroid_position.as_ref().unwrap();
            let inline_pos = inline_star.centroid_position.as_ref().unwrap();
            assert_eq!(shmem_pos.x, inline_pos.x, "x position mismatch at star {}", i);
            assert_eq!(shmem_pos.y, inline_pos.y, "y position mismatch at star {}", i);
        }
    }

    /// Test: solve_from_image with shmem_name pointing to real shared-memory
    /// file produces identical results to the inline image_data path.
    #[tokio::test]
    async fn solve_from_image_shmem_success() {
        use ps_db::{importer, loader};
        use tempfile::NamedTempFile;

        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));

        // Import the reference NPZ database for a real solve test.
        let npz_path =
            manifest.join("../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
        let db_imported =
            importer::import_npz(&npz_path).unwrap_or_else(|e| panic!("import_npz failed: {}", e));

        // Save -> load native round-trip.
        let tmp = NamedTempFile::new().expect("tempfile");
        loader::save_native(&db_imported, tmp.path()).expect("save_native");
        let mut db = loader::load_native(tmp.path()).expect("load_native");
        db.build_kd_tree();

        // Load the reference image.
        let img_path = manifest.join(
            "../reference-solutions/cedar-solve/examples/data/medium_fov/2019-07-29T204726_Alt40_Azi-135_Try1.jpg",
        );
        let img = image::open(&img_path)
            .unwrap_or_else(|e| panic!("Cannot open {:?}: {}", img_path, e))
            .into_luma8();

        let width = img.width() as i32;
        let height = img.height() as i32;
        let data = img.into_raw();

        // Create shmem file with a unique name.
        let shmem_name = format!("test_solve_from_image_shmem_{}", std::process::id());
        let shmem_path = format!("/dev/shm/{}", shmem_name);
        let _guard = ShmemGuard {
            path: shmem_path.clone(),
        };

        // Write image data to shmem file.
        std::fs::write(&shmem_path, &data).expect("write shmem file");

        let service = PlateSolverService::new(db);

        // Build shmem-backed request.
        let extract_req_shmem = CentroidsRequest {
            input_image: Some(Image {
                width,
                height,
                image_data: vec![],
                shmem_name: Some(shmem_name),
                reopen_shmem: false,
            }),
            sigma: 4.0,
            binning: None,
            return_binned: false,
            use_binned_for_star_candidates: false,
            detect_hot_pixels: true,
            normalize_rows: false,
            estimate_background_region: None,
        };

        let params = SolveParams {
            solve_timeout_ms: Some(120000),
            ..Default::default()
        };

        let request_shmem = SolveFromImageRequest {
            extract: Some(extract_req_shmem),
            params: Some(params.clone()),
        };

        let result_shmem = service
            .solve_from_image(Request::new(request_shmem))
            .await
            .expect("solve_from_image with shmem should succeed");
        let resp_shmem = result_shmem.into_inner();

        // Build inline-backed request with the same data.
        let extract_req_inline = CentroidsRequest {
            input_image: Some(Image {
                width,
                height,
                image_data: data,
                shmem_name: None,
                reopen_shmem: false,
            }),
            sigma: 4.0,
            binning: None,
            return_binned: false,
            use_binned_for_star_candidates: false,
            detect_hot_pixels: true,
            normalize_rows: false,
            estimate_background_region: None,
        };

        let request_inline = SolveFromImageRequest {
            extract: Some(extract_req_inline),
            params: Some(params),
        };

        // Need a new service instance for the inline test (old db was moved).
        // Re-import for inline test.
        let db_imported_2 =
            importer::import_npz(&npz_path).unwrap_or_else(|e| panic!("import_npz failed: {}", e));
        let tmp2 = NamedTempFile::new().expect("tempfile");
        loader::save_native(&db_imported_2, tmp2.path()).expect("save_native");
        let mut db2 = loader::load_native(tmp2.path()).expect("load_native");
        db2.build_kd_tree();
        let service2 = PlateSolverService::new(db2);

        let result_inline = service2
            .solve_from_image(Request::new(request_inline))
            .await
            .expect("solve_from_image with inline should succeed");
        let resp_inline = result_inline.into_inner();

        // Results must be identical: same status, same solution parameters.
        assert_eq!(
            resp_shmem.status, resp_inline.status,
            "solve status must match between shmem and inline"
        );
        assert_eq!(
            resp_shmem.ra, resp_inline.ra,
            "RA must match between shmem and inline"
        );
        assert_eq!(
            resp_shmem.dec, resp_inline.dec,
            "Dec must match between shmem and inline"
        );
        assert_eq!(
            resp_shmem.roll, resp_inline.roll,
            "Roll must match between shmem and inline"
        );
    }

    /// Test: shmem file with wrong size returns INVALID_ARGUMENT.
    #[tokio::test]
    async fn extract_centroids_shmem_bad_dimensions() {
        // Create a shmem file with 100 bytes.
        let shmem_name = format!("test_extract_shmem_bad_dims_{}", std::process::id());
        let shmem_path = format!("/dev/shm/{}", shmem_name);
        let _guard = ShmemGuard {
            path: shmem_path.clone(),
        };

        let bad_data = vec![128u8; 100];
        std::fs::write(&shmem_path, &bad_data).expect("write shmem file");

        // Request dimensions 20x20 (400 bytes expected), but shmem has 100.
        let request = CentroidsRequest {
            input_image: Some(Image {
                width: 20,
                height: 20,
                image_data: vec![],
                shmem_name: Some(shmem_name),
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

        let service = PlateSolverService::new(make_empty_db());
        let result = service.extract_centroids(Request::new(request)).await;

        assert!(
            result.is_err(),
            "expected an error for shmem bad dimensions, got Ok"
        );
        let err = result.unwrap_err();
        assert_eq!(
            err.code(),
            tonic::Code::InvalidArgument,
            "expected InvalidArgument status for bad dimensions, got {:?}",
            err.code()
        );
    }

    /// Cedar-detect wire interop: encode a cedar_detect::CentroidsRequest,
    /// decode as plate_solver::CentroidsRequest, and verify fields match.
    #[test]
    fn cedar_detect_interop() {
        use crate::cedar_detect::{CentroidsRequest as CedarRequest, Image as CedarImage};
        use crate::plate_solver::CentroidsRequest as OurRequest;
        use prost::Message;

        // Build a cedar_detect-shaped request
        let cedar_req = CedarRequest {
            input_image: Some(CedarImage {
                width: 640,
                height: 480,
                image_data: vec![42u8; 640 * 480],
                shmem_name: None,
                reopen_shmem: false,
            }),
            sigma: 8.0,
            max_size: 0,
            binning: Some(2),
            return_binned: false,
            use_binned_for_star_candidates: false,
            detect_hot_pixels: true,
            normalize_rows: false,
            estimate_background_region: None,
        };

        // Encode as cedar_detect bytes
        let mut bytes = Vec::new();
        cedar_req.encode(&mut bytes).expect("encode cedar request");

        // Decode as plate_solver::CentroidsRequest
        let our_req = OurRequest::decode(bytes.as_slice()).expect("decode as plate_solver request");

        // Fields must match
        let img = our_req.input_image.expect("image present");
        assert_eq!(img.width, 640);
        assert_eq!(img.height, 480);
        assert_eq!(img.image_data.len(), 640 * 480);
        assert_eq!(our_req.sigma, 8.0);
        assert_eq!(our_req.binning, Some(2));
        assert!(our_req.detect_hot_pixels);

        // Response direction: encode plate_solver::CentroidsResult → decode as cedar_detect::CentroidsResult
        use crate::cedar_detect::CentroidsResult as CedarResult;
        use crate::plate_solver::CentroidsResult as OurResult;

        let our_result = OurResult {
            noise_estimate: 3.14,
            background_estimate: None,
            hot_pixel_count: 5,
            peak_star_pixel: 200,
            star_candidates: vec![],
            binned_image: None,
            algorithm_time: Some(Duration {
                seconds: 0,
                nanos: 500_000,
            }),
        };

        let mut result_bytes = Vec::new();
        our_result
            .encode(&mut result_bytes)
            .expect("encode plate_solver result");

        let cedar_result =
            CedarResult::decode(result_bytes.as_slice()).expect("decode as cedar_detect result");

        assert_eq!(cedar_result.noise_estimate, 3.14);
        assert_eq!(cedar_result.hot_pixel_count, 5);
        assert_eq!(cedar_result.peak_star_pixel, 200);
        // algorithm_time field 5 should decode correctly as Duration in both protos now
        let algo_time = cedar_result.algorithm_time.expect("algorithm_time present");
        assert_eq!(algo_time.nanos, 500_000);
    }

    /// Test: extract_centroids with negative width returns INVALID_ARGUMENT.
    #[tokio::test]
    async fn extract_centroids_negative_width() {
        let data = vec![128u8; 64 * 64];
        let request = make_inline_request(data, -1, 64, 10.0);

        let service = PlateSolverService::new(make_empty_db());
        let result = service.extract_centroids(Request::new(request)).await;

        assert!(
            result.is_err(),
            "expected an error for negative width, got Ok"
        );
        let err = result.unwrap_err();
        assert_eq!(
            err.code(),
            tonic::Code::InvalidArgument,
            "expected InvalidArgument status, got {:?}",
            err.code()
        );
        assert!(err.message().contains("width"));
    }

    /// Test: extract_centroids with zero width returns INVALID_ARGUMENT.
    #[tokio::test]
    async fn extract_centroids_zero_width() {
        let data = vec![128u8; 64 * 64];
        let request = make_inline_request(data, 0, 64, 10.0);

        let service = PlateSolverService::new(make_empty_db());
        let result = service.extract_centroids(Request::new(request)).await;

        assert!(
            result.is_err(),
            "expected an error for zero width, got Ok"
        );
        let err = result.unwrap_err();
        assert_eq!(
            err.code(),
            tonic::Code::InvalidArgument,
            "expected InvalidArgument status, got {:?}",
            err.code()
        );
        assert!(err.message().contains("width"));
    }

    /// Test: extract_centroids with width > MAX_IMAGE_DIMENSION returns INVALID_ARGUMENT.
    #[tokio::test]
    async fn extract_centroids_oversized_width() {
        let data = vec![128u8; 100];
        let request = make_inline_request(data, 30_000, 64, 10.0);

        let service = PlateSolverService::new(make_empty_db());
        let result = service.extract_centroids(Request::new(request)).await;

        assert!(
            result.is_err(),
            "expected an error for oversized width, got Ok"
        );
        let err = result.unwrap_err();
        assert_eq!(
            err.code(),
            tonic::Code::InvalidArgument,
            "expected InvalidArgument status, got {:?}",
            err.code()
        );
        assert!(err.message().contains("width"));
    }

    /// Test: solve_from_centroids with negative width returns INVALID_ARGUMENT.
    #[tokio::test]
    async fn solve_from_centroids_negative_width() {
        let service = PlateSolverService::new(make_empty_db());

        let centroids = vec![
            ImageCoord { x: 10.0, y: 20.0 },
            ImageCoord { x: 30.0, y: 40.0 },
            ImageCoord { x: 50.0, y: 60.0 },
            ImageCoord { x: 70.0, y: 80.0 },
            ImageCoord { x: 90.0, y: 10.0 },
        ];

        let request = SolveFromCentroidsRequest {
            centroids,
            width: -1,
            height: 100,
            params: None,
        };

        let result = service.solve_from_centroids(Request::new(request)).await;

        assert!(
            result.is_err(),
            "expected an error for negative width, got Ok"
        );
        let err = result.unwrap_err();
        assert_eq!(
            err.code(),
            tonic::Code::InvalidArgument,
            "expected InvalidArgument status, got {:?}",
            err.code()
        );
        assert!(err.message().contains("width"));
    }

    /// Test: solve_from_image with zero height returns INVALID_ARGUMENT.
    #[tokio::test]
    async fn solve_from_image_zero_height() {
        let data = vec![128u8; 100];

        let extract_req = CentroidsRequest {
            input_image: Some(Image {
                width: 100,
                height: 0,
                image_data: data,
                shmem_name: None,
                reopen_shmem: false,
            }),
            sigma: 4.0,
            binning: None,
            return_binned: false,
            use_binned_for_star_candidates: false,
            detect_hot_pixels: false,
            normalize_rows: false,
            estimate_background_region: None,
        };

        let request = SolveFromImageRequest {
            extract: Some(extract_req),
            params: None,
        };

        let service = PlateSolverService::new(make_empty_db());
        let result = service.solve_from_image(Request::new(request)).await;

        assert!(
            result.is_err(),
            "expected an error for zero height, got Ok"
        );
        let err = result.unwrap_err();
        assert_eq!(
            err.code(),
            tonic::Code::InvalidArgument,
            "expected InvalidArgument status, got {:?}",
            err.code()
        );
        assert!(err.message().contains("height"));
    }

    /// H6: a tight RPC deadline cancels the solve via cancel_flag before solve_timeout.
    /// Uses the reference DB + many centroids to ensure the solve takes >200ms.
    #[tokio::test]
    async fn h6_rpc_deadline_cancels_solve() {
        use ps_db::{importer, loader};
        use std::time::Instant;
        use tempfile::NamedTempFile;

        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));

        // Load the reference DB (has real patterns — solve takes seconds without a deadline).
        let npz_path =
            manifest.join("../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
        let db_imported = importer::import_npz(&npz_path)
            .unwrap_or_else(|e| panic!("import_npz failed: {}", e));
        let tmp = NamedTempFile::new().expect("tempfile");
        loader::save_native(&db_imported, tmp.path()).expect("save_native");
        let mut db = loader::load_native(tmp.path()).expect("load_native");
        db.build_kd_tree();

        // 15 centroids → C(15,4) = 1365 combos with real pattern lookup = slow.
        let centroids: Vec<ImageCoord> = (0..15).map(|i| ImageCoord {
            x: 50.0 + (i as f64) * 30.0,
            y: 50.0 + (i as f64) * 20.0,
        }).collect();

        let mut request = Request::new(SolveFromCentroidsRequest {
            centroids,
            width: 640,
            height: 480,
            params: Some(SolveParams {
                solve_timeout_ms: Some(60_000), // 60s — proves the stop is from RPC deadline, not this
                ..Default::default()
            }),
        });
        // Set a 200ms RPC deadline via the grpc-timeout header.
        request.set_timeout(StdDuration::from_millis(200));

        let service = PlateSolverService::new(db);
        let t0 = Instant::now();
        let result = service.solve_from_centroids(request).await.expect("should return Ok");
        let elapsed = t0.elapsed();

        let resp = result.into_inner();
        // The solve must have been cancelled (not timed out by its own 60s budget).
        assert_eq!(
            resp.status,
            ProtoSolveStatus::Cancelled as i32,
            "expected Cancelled (3) but got status={}",
            resp.status
        );
        // Must complete well before solve_timeout_ms (60s).
        assert!(
            elapsed.as_secs() < 5,
            "deadline should have cancelled solve within 5s, took {:?}",
            elapsed
        );
    }
}
