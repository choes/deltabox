use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Error type for API handlers: renders as a JSON object with an `error`
/// message. Messages that clearly describe a missing resource map to 404,
/// everything else to 500.
pub struct ApiError(anyhow::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let message = self.0.to_string();
        let lower = message.to_lowercase();
        let status = if lower.contains("not found") || lower.contains("no such file") {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(error: E) -> Self {
        Self(error.into())
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
