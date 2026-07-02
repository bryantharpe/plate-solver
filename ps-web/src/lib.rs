//! HTTP web API for the Plate Solver.

use axum::extract::State;
use axum::response::Html;
use axum::routing::get;
use axum::{Json, Router};
use ps_db::Database;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Semaphore;

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

async fn index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

/// Build the axum router for the plate solver web API.
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/", get(index))
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

    #[tokio::test]
    async fn healthz_returns_ok_with_db_info() {
        let app = app(make_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

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
        let app = app(make_state());

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get("content-type")
            .expect("content-type header present")
            .to_str()
            .unwrap();
        assert!(
            content_type.starts_with("text/html"),
            "expected text/html, got {}",
            content_type
        );
    }
}
