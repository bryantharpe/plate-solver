//! gRPC service implementation for the Plate Solver.

use crate::plate_solver::plate_solver_server::PlateSolver;
use crate::plate_solver::{
    CentroidsRequest, CentroidsResult, Image, ImageCoord, InfoRequest, ServerInfo, Solution,
    SolveFromCentroidsRequest, SolveFromImageRequest, StarCentroid,
};
use memmap2::MmapOptions;
use ps_detect::noise::estimate_noise_from_image;
use ps_detect::{get_stars_from_image, GrayImage};
use std::fs::File;
use std::time::Instant;
use tonic::{Request, Response, Status};

pub struct PlateSolverService;

#[async_trait::async_trait]
impl PlateSolver for PlateSolverService {
    async fn extract_centroids(
        &self,
        request: Request<CentroidsRequest>,
    ) -> Result<Response<CentroidsResult>, Status> {
        let req = request.into_inner();

        // Extract input image.
        let input_image = req
            .input_image
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("missing input_image"))?;

        // reopen_shmem: we open fresh per request, so reopen is implicit.
        let _reopen = input_image.reopen_shmem;

        // Resolve image bytes (shmem or inline).
        let image_bytes = if let Some(ref shmem_name) = input_image.shmem_name {
            let path = format!("/dev/shm/{}", shmem_name);
            let file = File::open(&path).map_err(|e| {
                Status::internal(format!("shmem open failed: {}: {}", path, e))
            })?;
            let mmap = unsafe { MmapOptions::new().map(&file) }
                .map_err(|e| {
                    Status::internal(format!("shmem mmap failed: {}: {}", path, e))
                })?;
            mmap.to_vec()
        } else {
            input_image.image_data.clone()
        };

        let width = input_image.width as u32;
        let height = input_image.height as u32;

        // Validate dimensions.
        let expected_len = (width * height) as usize;
        if image_bytes.len() != expected_len {
            return Err(Status::invalid_argument(format!(
                "image_data length {} does not match width*height {}*{}={}",
                image_bytes.len(),
                width,
                height,
                expected_len
            )));
        }

        // Build GrayImage.
        let image = GrayImage::from_raw(width, height, image_bytes)
            .ok_or_else(|| Status::invalid_argument("failed to construct GrayImage"))?;

        // Estimate noise.
        let noise_estimate = estimate_noise_from_image(&image);

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
            &image,
            noise_estimate,
            sigma,
            normalize_rows,
            effective_binning,
            detect_hot_pixels,
            return_binned,
        );
        let algorithm_time_us = start.elapsed().as_micros() as i64;

        // Peak star pixel value: average of the NUM_PEAKS brightest stars, fallback 255.
        const NUM_PEAKS: usize = 10;
        let (sum_peak, num_peak) =
            stars.iter().take(NUM_PEAKS).fold((0i32, 0i32), |(s, n), star| {
                (s + star.peak_value as i32, n + 1)
            });
        let peak_star_pixel = if num_peak > 0 { sum_peak / num_peak } else { 255 };

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
            algorithm_time_us,
        }))
    }

    async fn solve_from_centroids(
        &self,
        _request: Request<SolveFromCentroidsRequest>,
    ) -> Result<Response<Solution>, Status> {
        Err(Status::unimplemented("solve_from_centroids not yet implemented"))
    }

    async fn solve_from_image(
        &self,
        _request: Request<SolveFromImageRequest>,
    ) -> Result<Response<Solution>, Status> {
        Err(Status::unimplemented("solve_from_image not yet implemented"))
    }

    async fn get_info(
        &self,
        _request: Request<InfoRequest>,
    ) -> Result<Response<ServerInfo>, Status> {
        Err(Status::unimplemented("get_info not yet implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plate_solver::{Image, CentroidsRequest};
    use std::path::Path;

    /// Helper: create a CentroidsRequest with inline image data.
    fn make_inline_request(image_data: Vec<u8>, width: i32, height: i32, sigma: f64) -> CentroidsRequest {
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
                width: 0,
                height: 0,
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

        let service = PlateSolverService;
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
        assert!(
            resp.algorithm_time_us > 0,
            "algorithm_time_us should be > 0, got {}",
            resp.algorithm_time_us
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

        let service = PlateSolverService;
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

        let service = PlateSolverService;
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

        let service = PlateSolverService;
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
}
