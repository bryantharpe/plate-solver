//! HTTP web API for the Plate Solver.

mod solve;

use axum::extract::{DefaultBodyLimit, State};
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use ps_db::Database;
use rust_embed::RustEmbed;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Max accepted request body size for `/api/solve` (bytes).
const SOLVE_BODY_LIMIT: usize = 32 * 1024 * 1024;

/// Shared application state passed to every handler.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    /// Serializes access to the (future) solve endpoint: only one solve at a time.
    pub solve_gate: Arc<Semaphore>,
}

impl AppState {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            solve_gate: Arc::new(Semaphore::new(1)),
        }
    }
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    star_catalog: String,
    min_fov: f32,
    max_fov: f32,
    num_patterns: u32,
}

async fn healthz(State(state): State<AppState>) -> Json<HealthResponse> {
    let p = &state.db.properties;
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        star_catalog: p.star_catalog.clone(),
        min_fov: p.min_fov,
        max_fov: p.max_fov,
        num_patterns: p.num_patterns,
    })
}

/// The built React SPA (`frontend/dist`), embedded into the binary so the
/// server stays a single self-contained executable. `dist` is committed to
/// git; rebuilding it requires node (`cd ps-web/frontend && npm run build`)
/// but cargo never does — see ps-web/README.md.
#[derive(RustEmbed)]
#[folder = "frontend/dist"]
struct FrontendAssets;

fn serve_embedded(path: &str) -> Response {
    match FrontendAssets::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], file.data).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Fallback handler: serve embedded frontend files by path, 404 for missing
/// assets (a dotted path is a real file request — don't mask a broken build),
/// and SPA-fallback everything else to `index.html`.
async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        serve_embedded("index.html")
    } else if FrontendAssets::get(path).is_some() {
        serve_embedded(path)
    } else if path.starts_with("assets/") || path.contains('.') {
        StatusCode::NOT_FOUND.into_response()
    } else {
        serve_embedded("index.html")
    }
}

/// Build the axum router for the plate solver web API.
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route(
            "/api/solve",
            post(solve::solve_handler).layer(DefaultBodyLimit::max(SOLVE_BODY_LIMIT)),
        )
        .fallback(get(static_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use ps_db::DatabaseProperties;
    use tower::ServiceExt;

    fn make_empty_db() -> Database {
        let props = DatabaseProperties::apply_legacy_fallbacks(
            None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            None, None,
        );
        Database::empty(props)
    }

    fn make_state() -> AppState {
        AppState::new(Arc::new(make_empty_db()))
    }

    async fn get_path(path: &str) -> axum::response::Response {
        app(make_state())
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    async fn body_string(response: axum::response::Response) -> String {
        let body = response.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(body.to_vec()).expect("body is valid UTF-8")
    }

    fn content_type(response: &axum::response::Response) -> String {
        response
            .headers()
            .get("content-type")
            .expect("content-type header present")
            .to_str()
            .unwrap()
            .to_string()
    }

    /// `AppState::solve_gate` is the single-permit semaphore that serializes
    /// decode+solve so at most one heavy operation runs at a time. This pins
    /// that architecture directly against the `Semaphore`, independent of
    /// timing-sensitive concurrent-HTTP-request behavior.
    #[test]
    fn solve_gate_allows_only_one_permit_at_a_time() {
        let state = make_state();
        assert_eq!(state.solve_gate.available_permits(), 1);

        let first = state.solve_gate.clone().try_acquire_owned().unwrap();
        assert_eq!(state.solve_gate.available_permits(), 0);
        assert!(
            state.solve_gate.clone().try_acquire_owned().is_err(),
            "a second permit must not be available while the first is held"
        );

        drop(first);
        assert_eq!(state.solve_gate.available_permits(), 1);
    }

    #[tokio::test]
    async fn healthz_returns_ok_with_db_info() {
        let response = get_path("/healthz").await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["status"], "ok");
        assert!(json["min_fov"].is_number());
        assert!(json["max_fov"].is_number());
        assert_eq!(json["min_fov"], 10.0);
        assert_eq!(json["max_fov"], 30.0);
    }

    #[tokio::test]
    async fn index_returns_html() {
        let response = get_path("/").await;
        assert_eq!(response.status(), StatusCode::OK);
        let ct = content_type(&response);
        assert!(ct.starts_with("text/html"), "expected text/html, got {ct}");
    }

    #[tokio::test]
    async fn index_serves_spa_shell() {
        let response = get_path("/").await;
        assert_eq!(response.status(), StatusCode::OK);
        let html = body_string(response).await;
        assert!(html.contains(r#"<div id="root">"#), "missing SPA root div");
        assert!(
            html.contains(r#"<script type="module""#),
            "missing module script tag"
        );
        assert!(html.contains("/assets/"), "missing hashed asset reference");
    }

    /// Every /assets/… path referenced by the embedded index.html must exist
    /// in the embed and serve with a sensible mime type. This doubles as a
    /// stale-dist guard: a half-committed frontend build fails here.
    #[tokio::test]
    async fn assets_serve_with_correct_mime_and_exist() {
        let html = body_string(get_path("/").await).await;

        let mut asset_paths = Vec::new();
        for (i, _) in html.match_indices("/assets/") {
            let rest = &html[i..];
            let end = rest
                .find(|c| c == '"' || c == '\'')
                .expect("asset path terminated by a quote");
            asset_paths.push(rest[..end].to_string());
        }
        assert!(
            !asset_paths.is_empty(),
            "index.html references no /assets/ files"
        );

        for path in asset_paths {
            let response = get_path(&path).await;
            assert_eq!(response.status(), StatusCode::OK, "missing asset {path}");
            let ct = content_type(&response);
            if path.ends_with(".js") {
                assert!(ct.contains("javascript"), "bad mime for {path}: {ct}");
            } else if path.ends_with(".css") {
                assert!(ct.starts_with("text/css"), "bad mime for {path}: {ct}");
            }
        }
    }

    #[tokio::test]
    async fn unknown_asset_404s() {
        let response = get_path("/assets/nope.js").await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn spa_fallback_serves_index() {
        let response = get_path("/some/client/route").await;
        assert_eq!(response.status(), StatusCode::OK);
        let ct = content_type(&response);
        assert!(ct.starts_with("text/html"), "expected text/html, got {ct}");
        let html = body_string(response).await;
        assert!(html.contains(r#"<div id="root">"#), "missing SPA root div");
    }
}
