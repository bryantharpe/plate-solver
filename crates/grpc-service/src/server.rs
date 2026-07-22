//! `PlateSolver` gRPC server implementation.
//!
//! Bridges the wire protocol to the `plate-solver` crate. The coordinate swap
//! between wire `(x, y)` and solver `(y, x)` is performed at every boundary
//! crossing in this file.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::proto::plate_solver_server::PlateSolver;
use crate::proto::{
    CentroidsRequest, CentroidsResult, Image, ImageCoord, InfoRequest, MatchedStar, ServerInfo,
    Solution, SolveFromCentroidsRequest, SolveFromImageRequest, SolveParams, SolveStatus,
    StarCentroid,
};
use pattern_database::PatternDatabase;
use plate_solver::status::SolveStatus as CoreStatus;
use plate_solver::{solve_from_centroids, solve_from_image, DetectParams};
use star_detection::detect_stars;
use star_detection::noise::estimate_noise;

/// Shared state for the gRPC server.
#[derive(Clone)]
pub struct PlateSolverServer {
    db: Arc<PatternDatabase>,
    version: String,
}

impl PlateSolverServer {
    /// Create a new server wrapping the given pattern database.
    pub fn new(db: PatternDatabase) -> Self {
        Self {
            db: Arc::new(db),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

#[tonic::async_trait]
impl PlateSolver for PlateSolverServer {
    async fn extract_centroids(
        &self,
        request: Request<CentroidsRequest>,
    ) -> Result<Response<CentroidsResult>, Status> {
        let req = request.into_inner();
        let (image, width, height) = read_image(&req.input_image)?;
        let params = extract_params(&req)?;

        let start = std::time::Instant::now();
        let stars = detect_stars(
            &image,
            width,
            height,
            params.sigma,
            params.binning,
            params.normalize_rows,
            params.detect_hot_pixels,
        );
        let elapsed = start.elapsed();

        let noise_estimate = estimate_noise(&image, width, height);
        let hot_pixel_count = 0i32; // ps-grpc-02 owns accurate hot-pixel counting.

        let mut candidates: Vec<StarCentroid> = stars
            .iter()
            .map(|s| StarCentroid {
                centroid_position: Some(ImageCoord {
                    // Solver (y, x) -> wire (x, y).
                    x: s.y,
                    y: s.x,
                }),
                brightness: s.brightness,
                num_saturated: s.num_saturated as i32,
            })
            .collect();

        // Brightest-first is already guaranteed by `detect_stars`, but re-sort to
        // make the contract explicit at the boundary.
        candidates.sort_by(|a, b| {
            b.brightness
                .partial_cmp(&a.brightness)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let binned_image = if params.return_binned {
            // ps-grpc-02 owns binned-image return.
            None
        } else {
            None
        };

        Ok(Response::new(CentroidsResult {
            noise_estimate,
            background_estimate: None,
            hot_pixel_count,
            peak_star_pixel: 0,
            star_candidates: candidates,
            binned_image,
            algorithm_time: Some(prost_types::Duration::try_from(elapsed).unwrap_or_default()),
        }))
    }

    async fn solve_from_centroids(
        &self,
        request: Request<SolveFromCentroidsRequest>,
    ) -> Result<Response<Solution>, Status> {
        let req = request.into_inner();
        let width = req.width as usize;
        let height = req.height as usize;

        // Wire (x, y) -> solver (y, x).
        let centroids: Vec<(f64, f64)> = req.centroids.into_iter().map(|c| (c.y, c.x)).collect();

        let (fov_estimate, fov_max_error, match_radius, match_threshold, solve_timeout, distortion) =
            decode_solve_params(&req.params);

        let core_solution = solve_from_centroids(
            &centroids,
            (width, height),
            fov_estimate,
            fov_max_error,
            match_radius,
            match_threshold,
            solve_timeout,
            distortion,
            0.002,
            (*self.db).clone(),
        );

        Ok(Response::new(to_proto_solution(
            core_solution,
            req.params
                .as_ref()
                .map(|p| p.return_matches)
                .unwrap_or(false),
        )))
    }

    async fn solve_from_image(
        &self,
        request: Request<SolveFromImageRequest>,
    ) -> Result<Response<Solution>, Status> {
        let req = request.into_inner();
        let extract = req
            .extract
            .ok_or_else(|| Status::invalid_argument("missing extract"))?;
        let (image, width, height) = read_image(&extract.input_image)?;
        let detect = extract_params(&extract)?;

        let (fov_estimate, fov_max_error, match_radius, match_threshold, solve_timeout, distortion) =
            decode_solve_params(&req.params);

        let core_solution = solve_from_image(
            &image,
            width,
            height,
            fov_estimate,
            fov_max_error,
            match_radius,
            match_threshold,
            solve_timeout,
            distortion,
            0.002,
            (*self.db).clone(),
            detect,
        );

        Ok(Response::new(to_proto_solution(
            core_solution,
            req.params
                .as_ref()
                .map(|p| p.return_matches)
                .unwrap_or(false),
        )))
    }

    async fn get_info(
        &self,
        _request: Request<InfoRequest>,
    ) -> Result<Response<ServerInfo>, Status> {
        let props = &self.db.properties;
        Ok(Response::new(ServerInfo {
            version: self.version.clone(),
            star_catalog: props.star_catalog.clone(),
            min_fov: props.min_fov as f64,
            max_fov: props.max_fov as f64,
            num_patterns: props.num_patterns as i64,
            epoch_equinox: props.epoch_equinox as f64,
            epoch_proper_motion: props.epoch_proper_motion as f64,
        }))
    }
}

/// Read an `Image` message into a row-major 8-bit buffer.
///
/// Shared-memory images are stubbed here; ps-grpc-02 implements the fast path.
fn read_image(image: &Option<Image>) -> Result<(Vec<u8>, usize, usize), Status> {
    let img = image
        .as_ref()
        .ok_or_else(|| Status::invalid_argument("missing input_image"))?;

    if img.shmem_name.is_some() {
        // ps-grpc-02 owns shared-memory handling. For now, signal the client to
        // fall back to inline image_data per the spec.
        return Err(Status::internal(
            "shared-memory fast path not implemented in this bead",
        ));
    }

    let width = img.width as usize;
    let height = img.height as usize;
    let expected = width * height;
    if img.image_data.len() != expected {
        return Err(Status::invalid_argument(format!(
            "image_data length {} does not match width*height {}",
            img.image_data.len(),
            expected
        )));
    }

    Ok((img.image_data.clone(), width, height))
}

/// Decode `SolveParams` into the positional arguments expected by the solver.
fn decode_solve_params(params: &Option<SolveParams>) -> (Option<f64>, f64, f64, f64, u64, f64) {
    let p = params.as_ref();
    (
        p.and_then(|p| p.fov_estimate),
        p.and_then(|p| p.fov_max_error).unwrap_or(5.0),
        p.and_then(|p| p.match_radius).unwrap_or(0.01),
        p.and_then(|p| p.match_threshold).unwrap_or(1e-5),
        p.and_then(|p| p.solve_timeout_ms.map(|v| v as u64))
            .unwrap_or(10_000),
        p.and_then(|p| p.distortion).unwrap_or(0.0),
    )
}

/// Convert `CentroidsRequest` fields into `DetectParams`.
///
/// Effective binning resolution: None -> 2 when binned behavior is requested,
/// Some(2)|Some(4) -> that value, other -> 1 unless binned behavior is requested
/// (in which case it is an error).
fn extract_params(req: &CentroidsRequest) -> Result<DetectParams, Status> {
    let binning = req.binning.unwrap_or(0);
    let wants_binned = req.return_binned || req.use_binned_for_star_candidates;

    let effective_binning: usize = if binning == 0 {
        if wants_binned {
            2
        } else {
            1
        }
    } else if binning == 2 || binning == 4 {
        binning as usize
    } else if binning == 1 {
        1
    } else if wants_binned {
        return Err(Status::invalid_argument(format!(
            "unsupported binning {} for binned operation",
            binning
        )));
    } else {
        1
    };

    Ok(DetectParams {
        sigma: req.sigma,
        noise_estimate: None,
        binning: effective_binning,
        normalize_rows: req.normalize_rows,
        detect_hot_pixels: req.detect_hot_pixels,
        return_binned: req.return_binned,
        use_binned_for_star_candidates: req.use_binned_for_star_candidates,
    })
}

/// Convert a core `Solution` into the protobuf `Solution` message.
///
/// When `return_matches` is true, matched-star data is populated from the core
/// solution's matched pairs.
fn to_proto_solution(sol: plate_solver::status::Solution, return_matches: bool) -> Solution {
    let status = match sol.status {
        Some(CoreStatus::MatchFound) => SolveStatus::MatchFound,
        Some(CoreStatus::NoMatch) => SolveStatus::NoMatch,
        Some(CoreStatus::Timeout) => SolveStatus::Timeout,
        Some(CoreStatus::Cancelled) => SolveStatus::Cancelled,
        Some(CoreStatus::TooFew) => SolveStatus::TooFew,
        None => SolveStatus::NoMatch,
    };

    let rad_to_deg = 180.0 / std::f64::consts::PI;
    // `fov_used` is stored in degrees on failure/timeout paths (it is copied
    // from `ctx.fov_initial`, which `preparation::initial_fov` returns in
    // degrees), but in radians on the success path (from `PinholeCamera.fov`).
    // Convert to degrees accordingly.
    let fov_deg = sol.fov_used.map(|f| {
        if f > std::f64::consts::PI {
            // Already in degrees on non-success paths.
            f
        } else {
            f.to_degrees()
        }
    });

    let mut matched = Vec::new();
    if return_matches {
        for ((centroid, cat_id), cat_vec) in sol
            .matched_centroids
            .iter()
            .zip(sol.matched_catalog_ids.iter())
            .zip(sol.matched_stars.iter())
        {
            let (ra, dec) = cat_vec.to_radec();
            matched.push(MatchedStar {
                centroid: Some(ImageCoord {
                    // Solver (y, x) -> wire (x, y).
                    x: centroid.1,
                    y: centroid.0,
                }),
                ra: ra.to_degrees(),
                dec: dec.to_degrees(),
                mag: 0.0,
                cat_id: *cat_id as i64,
            });
        }
    }

    Solution {
        status: status as i32,
        ra: sol.ra.unwrap_or(0.0) * rad_to_deg,
        dec: sol.dec.unwrap_or(0.0) * rad_to_deg,
        roll: sol.roll.unwrap_or(0.0) * rad_to_deg,
        fov: fov_deg.unwrap_or(0.0),
        distortion: sol.distortion.unwrap_or(0.0),
        rmse: sol.rmse.unwrap_or(0.0),
        p90e: sol.p90e.unwrap_or(0.0),
        maxe: sol.maxe.unwrap_or(0.0),
        matches: sol.matched_centroids.len() as i32,
        prob: sol.match_probability.unwrap_or(0.0),
        t_extract_ms: 0.0,
        t_solve_ms: 0.0,
        matched,
    }
}
