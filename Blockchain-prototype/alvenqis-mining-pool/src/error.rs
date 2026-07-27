use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("invalid share: {0}")]
    InvalidShare(String),
    #[error("stale share: {0}")]
    Stale(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("request limit exceeded; retry after {0} seconds")]
    RateLimited(u64),
    #[error("client temporarily banned; retry after {0} seconds")]
    Banned(u64),
    #[error("upstream error: {0}")]
    Upstream(String),
    #[error("storage error: {0}")]
    Storage(String),
}

pub type Result<T> = std::result::Result<T, PoolError>;

impl IntoResponse for PoolError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::InvalidShare(_) | Self::Config(_) => StatusCode::BAD_REQUEST,
            Self::Stale(_) => StatusCode::CONFLICT,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::RateLimited(_) | Self::Banned(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::Upstream(_) => StatusCode::BAD_GATEWAY,
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({ "error": self.to_string() }))).into_response()
    }
}
