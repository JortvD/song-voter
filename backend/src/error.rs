use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// Every handler error becomes a JSON `{"error": "..."}` response.
#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    TooManyRequests(String),
    ServiceUnavailable(String),
    Internal(anyhow::Error),
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized(message.into())
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    pub fn internal(error: impl Into<anyhow::Error>) -> Self {
        Self::Internal(error.into())
    }

    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::ServiceUnavailable(message.into())
    }

    pub fn too_many_requests(message: impl Into<String>) -> Self {
        Self::TooManyRequests(message.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message),
            Self::Forbidden(message) => (StatusCode::FORBIDDEN, message),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message),
            Self::Conflict(message) => (StatusCode::CONFLICT, message),
            Self::TooManyRequests(message) => (StatusCode::TOO_MANY_REQUESTS, message),
            Self::ServiceUnavailable(message) => (StatusCode::SERVICE_UNAVAILABLE, message),
            Self::Internal(error) => {
                // Full details go to the logs, never to the client.
                tracing::error!(error = ?error, "internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(message)
            | Self::Unauthorized(message)
            | Self::Forbidden(message)
            | Self::NotFound(message)
            | Self::Conflict(message)
            | Self::TooManyRequests(message)
            | Self::ServiceUnavailable(message) => formatter.write_str(message),
            Self::Internal(error) => write!(formatter, "{error:#}"),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        if matches!(error, sqlx::Error::PoolTimedOut) {
            tracing::warn!("database connection pool timed out");
            return Self::ServiceUnavailable(
                "the server is busy right now; please try again".to_string(),
            );
        }

        Self::Internal(error.into())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}

/// Reject control characters before they reach PostgreSQL. NUL bytes in
/// particular cause a database encoding error, which would otherwise become a
/// 500 for a user-supplied search or filter value.
pub fn validate_text_input(value: &str, label: &str) -> Result<(), ApiError> {
    if value.chars().any(|character| character.is_control()) {
        return Err(ApiError::bad_request(format!(
            "{label} contains invalid characters"
        )));
    }
    Ok(())
}

/// PostgreSQL unique-violation (23505) detection.
pub fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("23505")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    async fn parts(error: ApiError) -> (StatusCode, serde_json::Value) {
        let response = error.into_response();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap())
    }

    #[tokio::test]
    async fn bad_request_maps_to_400_with_the_envelope() {
        let (status, body) = parts(ApiError::bad_request("bad input")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "bad input");
    }

    #[tokio::test]
    async fn unauthorized_maps_to_401() {
        let (status, body) = parts(ApiError::unauthorized("nope")).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"], "nope");
    }

    #[tokio::test]
    async fn forbidden_maps_to_403() {
        let (status, _) = parts(ApiError::forbidden("no")).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn not_found_maps_to_404() {
        let (status, _) = parts(ApiError::NotFound("gone".into())).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn conflict_maps_to_409() {
        let (status, _) = parts(ApiError::conflict("taken")).await;
        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn too_many_requests_maps_to_429() {
        let (status, _) = parts(ApiError::too_many_requests("slow down")).await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn service_unavailable_maps_to_503() {
        let (status, _) = parts(ApiError::service_unavailable("busy")).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn internal_errors_hide_details_from_the_client() {
        let (status, body) = parts(ApiError::internal(anyhow::anyhow!("secret SQL text"))).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["error"], "internal server error");
        assert!(!body["error"].as_str().unwrap().contains("secret"));
    }

    #[tokio::test]
    async fn internal_sqlx_errors_hide_details() {
        let (status, body) = parts(ApiError::from(sqlx::Error::RowNotFound)).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["error"], "internal server error");
    }

    #[tokio::test]
    async fn anyhow_errors_convert_to_internal_errors() {
        let error: ApiError = anyhow::anyhow!("database exploded").into();
        let (status, body) = parts(error).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["error"], "internal server error");
    }

    #[test]
    fn display_exposes_the_safe_message() {
        assert_eq!(ApiError::bad_request("bad").to_string(), "bad");
        assert_eq!(
            ApiError::internal(anyhow::anyhow!("inner detail")).to_string(),
            "inner detail"
        );
    }

    #[test]
    fn text_validation_accepts_normal_text() {
        for value in ["", "hello", "café", "line with spaces", "emoji 🎸"] {
            assert!(validate_text_input(value, "search").is_ok(), "{value:?}");
        }
    }

    #[test]
    fn text_validation_rejects_control_characters() {
        for value in ["nul\0byte", "new\nline", "tab\there", "escape\u{1b}[31m"] {
            let error = validate_text_input(value, "search").unwrap_err();
            assert!(
                matches!(error, ApiError::BadRequest(message) if message.contains("search")),
                "{value:?} should be rejected"
            );
        }
    }

    #[test]
    fn text_validation_names_the_offending_field() {
        let error = validate_text_input("bad\0", "genre").unwrap_err();
        assert_eq!(error.to_string(), "genre contains invalid characters");
    }

    #[test]
    fn non_unique_sqlx_errors_are_not_unique_violations() {
        assert!(!is_unique_violation(&sqlx::Error::RowNotFound));
        assert!(!is_unique_violation(&sqlx::Error::PoolClosed));
    }
}
