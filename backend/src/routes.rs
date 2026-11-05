use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
    version: &'static str,
    environment: String,
    uptime_seconds: u64,
}

#[derive(Serialize)]
pub struct ReadinessResponse {
    status: &'static str,
    database: &'static str,
}

/// Liveness probe. Deliberately does not touch the database.
pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        environment: state.environment.clone(),
        uptime_seconds: state.started_at.elapsed().as_secs(),
    })
}

/// Readiness probe. Verifies that PostgreSQL is reachable.
pub async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ReadinessResponse {
                status: "ok",
                database: "up",
            }),
        ),
        Err(error) => {
            tracing::error!(%error, "database readiness check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ReadinessResponse {
                    status: "degraded",
                    database: "down",
                }),
            )
        }
    }
}

/// JSON 404 for unknown API routes, so the SPA fallback never answers them.
pub async fn api_not_found() -> impl IntoResponse {
    ApiError::NotFound("not found".to_string())
}
