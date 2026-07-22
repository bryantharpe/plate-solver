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

/// Pattern-match tolerance handed to the solver as `match_max_error`.
///
/// The solver clamps this up to the database's own `pattern_max_error`, so it
/// is a floor rather than an override. `openspec/specs/plate-solver/spec.md`
/// specifies 0.002 as the default, and the gRPC `Request parameters`
/// requirement deliberately does not expose it to clients: it is a property of
/// how the database was built, not a per-request knob.
const MATCH_MAX_ERROR: f64 = 0.002;

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

        // ps-grpc-02 owns returning the binned image; the field stays unset here
        // regardless of `return_binned`.
        let binned_image = None;

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

        let solve_start = std::time::Instant::now();
        let core_solution = solve_from_centroids(
            &centroids,
            (width, height),
            fov_estimate,
            fov_max_error,
            match_radius,
            match_threshold,
            solve_timeout,
            distortion,
            MATCH_MAX_ERROR,
            // The solver takes the database by value, so each request pays a
            // full clone. The database is large and immutable; the fix is for
            // `plate_solver::solve_*` to accept `&PatternDatabase` (or the
            // `Arc` itself), which is ps-plate-04's API to change, not this
            // boundary's. Tracked rather than worked around here.
            (*self.db).clone(),
        );
        let t_solve_ms = elapsed_ms(solve_start);

        Ok(Response::new(to_proto_solution(
            core_solution,
            req.params
                .as_ref()
                .map(|p| p.return_matches)
                .unwrap_or(false),
            &self.db,
            // No extraction happens on this path: the caller supplied centroids.
            0.0,
            t_solve_ms,
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

        let solve_start = std::time::Instant::now();
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
            MATCH_MAX_ERROR,
            // The solver takes the database by value, so each request pays a
            // full clone. The database is large and immutable; the fix is for
            // `plate_solver::solve_*` to accept `&PatternDatabase` (or the
            // `Arc` itself), which is ps-plate-04's API to change, not this
            // boundary's. Tracked rather than worked around here.
            (*self.db).clone(),
            detect,
        );
        let total_ms = elapsed_ms(solve_start);

        Ok(Response::new(to_proto_solution(
            core_solution,
            req.params
                .as_ref()
                .map(|p| p.return_matches)
                .unwrap_or(false),
            &self.db,
            // `plate_solver::solve_from_image` fuses detection and solving into a
            // single call, so the two cannot be timed apart without duplicating
            // detection. Report the measured total under `t_solve_ms` rather than
            // inventing a split. ps-grpc-02 owns this RPC's detection path and can
            // report a true `t_extract_ms` once it drives detection itself.
            0.0,
            total_ms,
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

/// Render a `CatalogId` as the wire's `(cat_id, cat_id_str)` pair.
///
/// BSC and Hipparcos are single numbers and populate both. Tycho is a
/// (TYC1, TYC2, TYC3) triple that cannot be flattened into one integer without
/// inventing an encoding, so it populates only the string form.
fn catalog_id_fields(id: &pattern_database::CatalogId) -> (Option<i64>, Option<String>) {
    use pattern_database::CatalogId;
    match id {
        CatalogId::Bsc(n) => (Some(*n as i64), Some(format!("BSC {n}"))),
        CatalogId::Hip(n) => (Some(*n as i64), Some(format!("HIP {n}"))),
        CatalogId::Tyc(a, b, c) => (None, Some(format!("TYC {a}-{b}-{c}"))),
    }
}

/// Elapsed wall-clock time since `start`, in fractional milliseconds.
fn elapsed_ms(start: std::time::Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1_000.0
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
fn to_proto_solution(
    sol: plate_solver::status::Solution,
    return_matches: bool,
    db: &PatternDatabase,
    t_extract_ms: f64,
    t_solve_ms: f64,
) -> Solution {
    let status = match sol.status {
        Some(CoreStatus::MatchFound) => SolveStatus::MatchFound,
        Some(CoreStatus::NoMatch) => SolveStatus::NoMatch,
        Some(CoreStatus::Timeout) => SolveStatus::Timeout,
        Some(CoreStatus::Cancelled) => SolveStatus::Cancelled,
        Some(CoreStatus::TooFew) => SolveStatus::TooFew,
        None => SolveStatus::NoMatch,
    };

    // `fov_used` carries two different units depending on which path produced
    // the solution: the success path in `refine` copies `PinholeCamera.fov`
    // (radians), while every failure path copies `ctx.fov_initial`, which
    // `preparation::initial_fov` derives from the database FOV bounds (degrees).
    //
    // Discriminate on `camera`, which is set by exactly the one site that writes
    // radians. Do NOT discriminate on magnitude ("above pi must be degrees"): a
    // radian FOV never exceeds pi, so that test silently misclassifies the
    // failure path of any database narrower than 3.14 degrees, double-converting
    // 2.5 degrees into 143.2.
    let fov_deg = sol.fov_used.map(|f| {
        if sol.camera.is_some() {
            f.to_degrees()
        } else {
            f
        }
    });

    let mut matched = Vec::new();
    if return_matches {
        for ((centroid, star_index), cat_vec) in sol
            .matched_centroids
            .iter()
            .zip(sol.matched_catalog_ids.iter())
            .zip(sol.matched_stars.iter())
        {
            let (ra, dec) = cat_vec.to_radec();
            // `Solution::matched_catalog_ids` holds database star-table indices
            // (verification feeds them straight to `db.star_vector(StarId(..))`),
            // not source-catalog numbers. Resolve both the magnitude and the real
            // catalog identifier through the database rather than leaking the
            // internal index onto the wire as a catalog ID.
            let star_id = pattern_database::StarId(*star_index);
            let mag = db.star_radec_mag(star_id).map(|(_, _, mag)| mag);
            let (cat_id, cat_id_str) = db
                .star_catalog_ids
                .get(*star_index)
                .map(catalog_id_fields)
                .unwrap_or((None, None));

            matched.push(MatchedStar {
                centroid: Some(ImageCoord {
                    // Solver (y, x) -> wire (x, y).
                    x: centroid.1,
                    y: centroid.0,
                }),
                ra: ra.to_degrees(),
                dec: dec.to_degrees(),
                mag,
                cat_id,
                cat_id_str,
            });
        }
    }

    Solution {
        status: status as i32,
        // Core attitude is in radians; the wire contract is degrees. Absent
        // values stay absent rather than collapsing to 0.0.
        ra: sol.ra.map(f64::to_degrees),
        dec: sol.dec.map(f64::to_degrees),
        roll: sol.roll.map(f64::to_degrees),
        fov: fov_deg,
        distortion: sol.distortion,
        // Already arcseconds in the core solution.
        rmse: sol.rmse,
        p90e: sol.p90e,
        maxe: sol.maxe,
        matches: sol.matched_centroids.len() as i32,
        prob: sol.match_probability,
        t_extract_ms,
        t_solve_ms,
        matched,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use math_core::PinholeCamera;
    use pattern_database::{CatalogId, StarKdTree};
    use plate_solver::status::Solution as CoreSolution;

    /// A database with no stars: enough for the conversions that never touch the
    /// star table.
    fn test_db() -> PatternDatabase {
        PatternDatabase {
            star_table: Vec::new(),
            num_stars: 0,
            pattern_catalog: Vec::new(),
            pattern_largest_edge: Vec::new(),
            pattern_key_hashes: Vec::new(),
            star_catalog_ids: Vec::new(),
            properties: Default::default(),
            star_kdtree: StarKdTree::new(&[]),
        }
    }

    /// A database holding a single star, so matched-star resolution has
    /// something to look up. Star-table rows are `[ra, dec, x, y, z, mag]`.
    fn db_with_one_star(mag: f32, catalog_id: CatalogId) -> PatternDatabase {
        PatternDatabase {
            star_table: vec![0.0, 0.0, 1.0, 0.0, 0.0, mag],
            num_stars: 1,
            star_catalog_ids: vec![catalog_id],
            ..test_db()
        }
    }

    /// Spec: "WHEN a solve succeeds THEN the Solution has status = MATCH_FOUND
    /// with ra, dec, roll, fov populated."
    ///
    /// Also pins the FOV unit conversion: on the success path the core stores
    /// `PinholeCamera.fov` in radians and the wire contract is degrees.
    #[test]
    fn success_populates_attitude_in_degrees() {
        let fov_rad = 20.0f64.to_radians();
        let sol = CoreSolution {
            status: Some(CoreStatus::MatchFound),
            camera: Some(PinholeCamera::new(1024.0, 768.0, fov_rad)),
            fov_used: Some(fov_rad),
            ra: Some(1.0),
            dec: Some(0.5),
            roll: Some(-0.25),
            ..CoreSolution::default()
        };

        let out = to_proto_solution(sol, false, &test_db(), 0.0, 0.0);

        assert_eq!(out.status, SolveStatus::MatchFound as i32);
        assert!((out.ra.unwrap() - 1.0f64.to_degrees()).abs() < 1e-9);
        assert!((out.dec.unwrap() - 0.5f64.to_degrees()).abs() < 1e-9);
        assert!((out.roll.unwrap() - (-0.25f64).to_degrees()).abs() < 1e-9);
        assert!(
            (out.fov.unwrap() - 20.0).abs() < 1e-9,
            "fov must be degrees"
        );
    }

    /// Regression guard for the units bug.
    ///
    /// A narrow-field database (say 2.5 deg) that fails to solve reports
    /// `fov_used = 2.5` in DEGREES. Deciding units by magnitude — "values above
    /// pi must already be degrees" — misreads 2.5 as radians and reports 143.2
    /// deg. Discriminating on `camera`, which is set only where radians are
    /// written, gets it right. This is the failure path, not the success path:
    /// a radian FOV never exceeds pi, so the magnitude guess only ever misfires
    /// here.
    #[test]
    fn narrow_field_failure_fov_not_mistaken_for_radians() {
        let sol = CoreSolution {
            status: Some(CoreStatus::NoMatch),
            camera: None,
            fov_used: Some(2.5), // degrees, from a narrow-field database
            ..CoreSolution::default()
        };

        let out = to_proto_solution(sol, false, &test_db(), 0.0, 0.0);

        assert!(
            (out.fov.unwrap() - 2.5).abs() < 1e-9,
            "narrow-field FOV must stay 2.5 deg, got {:?}",
            out.fov
        );
    }

    /// Spec: "WHEN a solve fails or times out THEN the Solution carries the
    /// corresponding status and unset attitude fields."
    ///
    /// On failure paths the core copies `ctx.fov_initial`, which is already in
    /// degrees and must not be converted again.
    #[test]
    fn failure_leaves_attitude_unset_and_fov_in_degrees() {
        for status in [CoreStatus::NoMatch, CoreStatus::Timeout, CoreStatus::TooFew] {
            let sol = CoreSolution {
                status: Some(status),
                camera: None,
                fov_used: Some(20.0), // degrees, straight from the database bounds
                ..CoreSolution::default()
            };

            let out = to_proto_solution(sol, false, &test_db(), 0.0, 0.0);

            assert_ne!(out.status, SolveStatus::MatchFound as i32);
            assert!(out.ra.is_none(), "ra must be unset on failure");
            assert!(out.dec.is_none(), "dec must be unset on failure");
            assert!(out.roll.is_none(), "roll must be unset on failure");
            assert!(
                (out.fov.unwrap() - 20.0).abs() < 1e-9,
                "fov already degrees"
            );
        }
    }

    /// Spec: the Solution may carry matched-star data including magnitude and
    /// catalog IDs. `matched_catalog_ids` holds internal star-table indices, so
    /// both must be resolved through the database rather than echoed straight out.
    #[test]
    fn matched_stars_resolve_magnitude_and_catalog_id() {
        let db = db_with_one_star(4.25, CatalogId::Hip(91262));
        let sol = CoreSolution {
            status: Some(CoreStatus::MatchFound),
            camera: Some(PinholeCamera::new(1024.0, 768.0, 0.3)),
            fov_used: Some(0.3),
            matched_centroids: vec![(11.0, 22.0)],
            matched_catalog_ids: vec![0],
            matched_stars: vec![math_core::UnitVector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            }],
            ..CoreSolution::default()
        };

        let out = to_proto_solution(sol, true, &db, 0.0, 0.0);

        assert_eq!(out.matched.len(), 1);
        let m = &out.matched[0];
        assert!(
            (m.mag.unwrap() - 4.25).abs() < 1e-6,
            "magnitude from star table"
        );
        // The real Hipparcos number, not the star-table index 0.
        assert_eq!(m.cat_id, Some(91262));
        assert_eq!(m.cat_id_str.as_deref(), Some("HIP 91262"));
        // And the centroid still crosses the boundary swapped: solver (y, x)
        // (11, 22) becomes wire (x, y) = (22, 11).
        let c = m.centroid.as_ref().unwrap();
        assert_eq!((c.x, c.y), (22.0, 11.0));
    }

    /// A Tycho identifier is a triple and cannot be flattened into `cat_id`, so
    /// only the string form is populated.
    #[test]
    fn tycho_catalog_id_uses_string_form_only() {
        let db = db_with_one_star(6.0, CatalogId::Tyc(1, 2, 3));
        let sol = CoreSolution {
            status: Some(CoreStatus::MatchFound),
            matched_centroids: vec![(1.0, 2.0)],
            matched_catalog_ids: vec![0],
            matched_stars: vec![math_core::UnitVector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            }],
            ..CoreSolution::default()
        };

        let out = to_proto_solution(sol, true, &db, 0.0, 0.0);

        assert_eq!(out.matched[0].cat_id, None);
        assert_eq!(out.matched[0].cat_id_str.as_deref(), Some("TYC 1-2-3"));
    }

    /// `return_matches = false` must suppress matched-star data entirely.
    #[test]
    fn matched_stars_omitted_unless_requested() {
        let db = db_with_one_star(4.25, CatalogId::Hip(91262));
        let sol = CoreSolution {
            status: Some(CoreStatus::MatchFound),
            matched_centroids: vec![(11.0, 22.0)],
            matched_catalog_ids: vec![0],
            matched_stars: vec![math_core::UnitVector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            }],
            ..CoreSolution::default()
        };

        let out = to_proto_solution(sol, false, &db, 0.0, 0.0);

        assert!(out.matched.is_empty());
        // `matches` still reports the count even when the data is withheld.
        assert_eq!(out.matches, 1);
    }

    /// proto3 uses the zero enum value for absent messages, so it must not mean
    /// MATCH_FOUND: a default-constructed Solution would otherwise claim success.
    #[test]
    fn default_status_is_not_match_found() {
        assert_eq!(SolveStatus::default(), SolveStatus::Unspecified);
        assert_ne!(SolveStatus::MatchFound as i32, 0);
    }
}
