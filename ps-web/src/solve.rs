//! `POST /api/solve` — multipart image + FOV estimate to JSON plate solution.

use crate::AppState;
use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use ps_solve::{DetectParams, Solution, SolveParams, SolveStatus};
use serde::Serialize;

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

fn error_response(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(ErrorBody { error: msg.into() })).into_response()
}

/// Map a multipart read error to a response using axum's own `.status()`
/// classification (BAD_REQUEST for malformed bodies, PAYLOAD_TOO_LARGE when
/// the error wraps the `DefaultBodyLimit` being exceeded) rather than
/// collapsing every multipart error to 400.
fn multipart_error_response(
    context: &str,
    err: axum::extract::multipart::MultipartError,
) -> Response {
    error_response(err.status(), format!("{context}: {err}"))
}

#[derive(Serialize)]
struct MatchedStarJson {
    x: f64,
    y: f64,
    ra: f64,
    dec: f64,
    mag: f64,
    cat_id: u32,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum SolveResponse {
    MatchFound {
        ra_deg: f64,
        dec_deg: f64,
        roll_deg: f64,
        fov_deg: f64,
        ra_hms: String,
        dec_dms: String,
        rmse: f64,
        p90e: f64,
        maxe: f64,
        matches: usize,
        prob: f64,
        distortion: f64,
        t_solve_ms: f64,
        matched_stars: Vec<MatchedStarJson>,
    },
    NoMatch {
        hint: String,
    },
    Timeout {
        hint: String,
    },
    Cancelled {
        hint: String,
    },
    TooFew {
        hint: String,
    },
}

/// Map a solved [`Solution`] to the JSON response shape.
fn map_response(sol: &Solution) -> SolveResponse {
    match sol.status {
        SolveStatus::MatchFound => {
            let centroids = sol.matched_centroids.as_deref().unwrap_or(&[]);
            let stars = sol.matched_stars.as_deref().unwrap_or(&[]);
            let cat_ids = sol.matched_cat_ids.as_deref().unwrap_or(&[]);
            let matched_stars = centroids
                .iter()
                .zip(stars.iter())
                .zip(cat_ids.iter())
                .map(|((yx, rdm), cat_id)| MatchedStarJson {
                    x: yx[1], // swap: response uses (x, y)
                    y: yx[0],
                    // ps_solve::Solution.matched_stars stores ra/dec in radians
                    // (star_table columns); convert to degrees to match ra_deg/dec_deg.
                    ra: rdm[0].to_degrees(),
                    dec: rdm[1].to_degrees(),
                    mag: rdm[2],
                    cat_id: *cat_id,
                })
                .collect();

            SolveResponse::MatchFound {
                ra_deg: sol.ra,
                dec_deg: sol.dec,
                roll_deg: sol.roll,
                fov_deg: sol.fov,
                ra_hms: ra_to_hms(sol.ra),
                dec_dms: dec_to_dms(sol.dec),
                rmse: sol.rmse,
                p90e: sol.p90e,
                maxe: sol.maxe,
                matches: sol.matches,
                prob: sol.prob,
                distortion: sol.distortion,
                t_solve_ms: sol.t_solve * 1000.0,
                matched_stars,
            }
        }
        SolveStatus::NoMatch => SolveResponse::NoMatch {
            hint: "No matching pattern was found in the catalog. Try a different FOV estimate."
                .to_string(),
        },
        SolveStatus::Timeout => SolveResponse::Timeout {
            hint: "Solve timed out before finding a match. Try increasing timeout_ms.".to_string(),
        },
        SolveStatus::Cancelled => SolveResponse::Cancelled {
            hint: "Solve was cancelled.".to_string(),
        },
        SolveStatus::TooFew => SolveResponse::TooFew {
            hint: "Too few star centroids were detected to attempt a solve (need at least 4)."
                .to_string(),
        },
    }
}

const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const MAX_TIMEOUT_MS: u64 = 60_000;
const DEFAULT_MATCH_RADIUS: f64 = 0.01;
const DEFAULT_MATCH_THRESHOLD: f64 = 1e-5;
const MATCH_MAX_ERROR: f64 = 0.002;

/// Strict per-side cap on decoded image dimensions. Well above any real
/// astrophotography sensor (largest common consumer sensors are well under
/// 10k px/side); exists to reject a small, highly-compressible file that
/// declares absurd dimensions (decompression-bomb DoS).
const MAX_IMAGE_DIMENSION: u32 = 20_000;
/// Non-strict cap on total decoder allocation. Backstops the dimension cap
/// for formats/paths where width*height*channels can still blow past a sane
/// memory budget even under the dimension limit.
const MAX_DECODE_ALLOC_BYTES: u64 = 256 * 1024 * 1024;

fn image_decode_limits() -> image::io::Limits {
    // `image::io::Limits` is `#[non_exhaustive]`: build from `default()` and
    // mutate fields rather than using struct-literal syntax.
    let mut limits = image::io::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOC_BYTES);
    limits
}

/// Parsed multipart form fields for a solve request.
struct SolveForm {
    image_bytes: Vec<u8>,
    fov_estimate: f64,
    fov_max_error: Option<f64>,
    match_radius: f64,
    match_threshold: f64,
    timeout_ms: u64,
    distortion: Option<f64>,
}

async fn parse_form(multipart: &mut Multipart) -> Result<SolveForm, Response> {
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut fov_estimate: Option<f64> = None;
    let mut fov_max_error: Option<f64> = None;
    let mut match_radius = DEFAULT_MATCH_RADIUS;
    let mut match_threshold = DEFAULT_MATCH_THRESHOLD;
    let mut timeout_ms = DEFAULT_TIMEOUT_MS;
    let mut distortion: Option<f64> = None;

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(e) => return Err(multipart_error_response("invalid multipart body", e)),
        };
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "image" => {
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| multipart_error_response("failed to read image field", e))?;
                image_bytes = Some(bytes.to_vec());
            }
            "fov_estimate" => {
                fov_estimate = Some(parse_field_f64(field, "fov_estimate").await?);
            }
            "fov_max_error" => {
                fov_max_error = Some(parse_field_f64(field, "fov_max_error").await?);
            }
            "match_radius" => {
                match_radius = parse_field_f64(field, "match_radius").await?;
            }
            "match_threshold" => {
                match_threshold = parse_field_f64(field, "match_threshold").await?;
            }
            "timeout_ms" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| multipart_error_response("failed to read timeout_ms field", e))?;
                timeout_ms = text.trim().parse::<u64>().map_err(|_| {
                    error_response(
                        StatusCode::BAD_REQUEST,
                        format!("invalid timeout_ms: {text:?}"),
                    )
                })?;
            }
            "distortion" => {
                distortion = Some(parse_field_f64(field, "distortion").await?);
            }
            _ => {
                // Unknown field: drain it and ignore.
                let _ = field.bytes().await;
            }
        }
    }

    let image_bytes = image_bytes
        .ok_or_else(|| error_response(StatusCode::BAD_REQUEST, "missing required field: image"))?;
    let fov_estimate = fov_estimate.ok_or_else(|| {
        error_response(
            StatusCode::BAD_REQUEST,
            "missing required field: fov_estimate",
        )
    })?;
    if !fov_estimate.is_finite() || fov_estimate <= 0.0 {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "fov_estimate must be a positive, finite number",
        ));
    }

    let timeout_ms = timeout_ms.min(MAX_TIMEOUT_MS);

    Ok(SolveForm {
        image_bytes,
        fov_estimate,
        fov_max_error,
        match_radius,
        match_threshold,
        timeout_ms,
        distortion,
    })
}

async fn parse_field_f64(
    field: axum::extract::multipart::Field<'_>,
    field_name: &str,
) -> Result<f64, Response> {
    let text = field
        .text()
        .await
        .map_err(|e| multipart_error_response(&format!("failed to read {field_name} field"), e))?;
    text.trim().parse::<f64>().map_err(|_| {
        error_response(
            StatusCode::BAD_REQUEST,
            format!("invalid {field_name}: {text:?}"),
        )
    })
}

/// Decode an image with strict dimension/allocation limits, to guard against
/// decompression-bomb inputs (a small, highly-compressible file that decodes
/// to an enormous pixel buffer). Oversize images are rejected with 413
/// rather than allowed to exhaust process memory; malformed/unsupported
/// images are rejected with 415 as before.
fn decode_image_bounded(bytes: &[u8]) -> image::ImageResult<image::DynamicImage> {
    let mut reader = image::io::Reader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(image::ImageError::IoError)?;
    reader.limits(image_decode_limits());
    reader.decode()
}

/// Trips the wrapped flag when dropped. `spawn_blocking` tasks are not
/// aborted when their `JoinHandle` future is dropped, so a client disconnect
/// mid-solve would otherwise leave the blocking task (and the solve permit
/// it holds) running until it finishes on its own. Holding this guard across
/// the `spawn_blocking` await means that if the handler future is dropped
/// (request cancelled), the guard drops too and trips the flag, which
/// `ps_solve`'s solve loop polls to exit early.
struct CancelOnDrop(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

pub async fn solve_handler(State(state): State<AppState>, mut multipart: Multipart) -> Response {
    let form = match parse_form(&mut multipart).await {
        Ok(f) => f,
        Err(resp) => return resp,
    };

    let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let _cancel_guard = CancelOnDrop(cancel_flag.clone());

    let params = SolveParams {
        match_radius: form.match_radius,
        match_threshold: form.match_threshold,
        match_max_error: MATCH_MAX_ERROR,
        solve_timeout: Some(form.timeout_ms),
        distortion: form.distortion,
        fov_estimate: Some(form.fov_estimate),
        fov_max_error: form.fov_max_error,
        cancel_flag: Some(cancel_flag),
    };

    // Acquire the solve permit *before* touching the image: decode is
    // CPU-bound and allocation-heavy, so it must be throttled by the same
    // single-heavy-op-at-a-time gate as the solve itself, and run off the
    // async executor via spawn_blocking rather than blocking a tokio worker.
    let permit = match state.solve_gate.clone().acquire_owned().await {
        Ok(p) => p,
        Err(_) => {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "solve gate unavailable")
        }
    };

    let db = state.db.clone();
    let image_bytes = form.image_bytes;
    let result = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let img = decode_image_bounded(&image_bytes)?.to_luma8();
        Ok(ps_solve::solve_from_image(
            &db,
            &ps_detect::as_view(&img),
            &params,
            &DetectParams::default(),
        ))
    })
    .await;

    let sol = match result {
        Ok(Ok(sol)) => sol,
        Ok(Err(image::ImageError::Limits(e))) => {
            return error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                format!("image exceeds decode limits: {e}"),
            )
        }
        Ok(Err(e)) => {
            return error_response(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                format!("could not decode image: {e}"),
            )
        }
        Err(_) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "solver panicked"),
    };

    Json(map_response(&sol)).into_response()
}

/// Format right ascension (degrees) as sexagesimal hours: `HHhMMmSS.SSs`.
fn ra_to_hms(ra_deg: f64) -> String {
    let normalized = ra_deg.rem_euclid(360.0);
    let hours = normalized / 15.0;
    let (h, m, s) = split_sexagesimal(hours);
    let h = h.rem_euclid(24);
    format!("{h:02}h{m:02}m{s:05.2}s")
}

/// Format declination (degrees) as sexagesimal degrees: `+DDdMMmSS.Ss`.
fn dec_to_dms(dec_deg: f64) -> String {
    let sign = if dec_deg < 0.0 { '-' } else { '+' };
    let (d, m, s) = split_sexagesimal(dec_deg.abs());
    format!("{sign}{d:02}d{m:02}m{s:05.2}s")
}

/// Split a non-negative value (in "big units", e.g. hours or degrees) into
/// (whole units, minutes, seconds), rounding to the nearest hundredth of a
/// second and carrying overflow correctly.
fn split_sexagesimal(total_units: f64) -> (i64, i64, f64) {
    let total_seconds = total_units * 3600.0;
    let total_centiseconds = (total_seconds * 100.0).round() as i64;
    let cs = total_centiseconds % 100;
    let total_seconds_int = total_centiseconds / 100;
    let s_int = total_seconds_int % 60;
    let total_minutes = total_seconds_int / 60;
    let m = total_minutes % 60;
    let h = total_minutes / 60;
    let sec = s_int as f64 + cs as f64 / 100.0;
    (h, m, sec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_on_drop_trips_flag() {
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        {
            let _guard = CancelOnDrop(flag.clone());
            assert!(!flag.load(std::sync::atomic::Ordering::Relaxed));
        }
        assert!(flag.load(std::sync::atomic::Ordering::Relaxed));
    }

    /// `map_response` is the only place that turns a solver [`SolveStatus`]
    /// into the wire JSON shape; this pins its behavior for every non-match
    /// status (each of which must carry a non-empty, human-readable hint),
    /// independent of whatever actually drives the solver into that status.
    #[test]
    fn map_response_covers_all_non_match_statuses() {
        for status in [
            SolveStatus::NoMatch,
            SolveStatus::Timeout,
            SolveStatus::Cancelled,
            SolveStatus::TooFew,
        ] {
            let sol = Solution::failure(status.clone(), 0.0, 0);
            let resp = map_response(&sol);
            let hint = match resp {
                SolveResponse::NoMatch { hint } => hint,
                SolveResponse::Timeout { hint } => hint,
                SolveResponse::Cancelled { hint } => hint,
                SolveResponse::TooFew { hint } => hint,
                SolveResponse::MatchFound { .. } => {
                    panic!("failure status {status:?} mapped to MatchFound")
                }
            };
            assert!(!hint.is_empty(), "{status:?} produced an empty hint");
        }
    }

    #[test]
    fn timeout_ms_clamps_to_max() {
        assert_eq!(DEFAULT_TIMEOUT_MS.min(MAX_TIMEOUT_MS), DEFAULT_TIMEOUT_MS);
        assert_eq!(999_999u64.min(MAX_TIMEOUT_MS), MAX_TIMEOUT_MS);
    }

    #[test]
    fn ra_to_hms_zero() {
        assert_eq!(ra_to_hms(0.0), "00h00m00.00s");
    }

    #[test]
    fn ra_to_hms_half_day() {
        assert_eq!(ra_to_hms(180.0), "12h00m00.00s");
    }

    #[test]
    fn ra_to_hms_one_hour() {
        assert_eq!(ra_to_hms(15.0), "01h00m00.00s");
    }

    #[test]
    fn ra_to_hms_wraps_at_360() {
        assert_eq!(ra_to_hms(359.99999999), "00h00m00.00s");
    }

    #[test]
    fn ra_to_hms_reference_value() {
        assert_eq!(ra_to_hms(230.66822431880013), "15h22m40.37s");
    }

    #[test]
    fn dec_to_dms_zero() {
        assert_eq!(dec_to_dms(0.0), "+00d00m00.00s");
    }

    #[test]
    fn dec_to_dms_negative() {
        assert_eq!(dec_to_dms(-45.5), "-45d30m00.00s");
    }

    #[test]
    fn dec_to_dms_reference_value() {
        assert_eq!(dec_to_dms(11.035810666031358), "+11d02m08.92s");
    }
}
