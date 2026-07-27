use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RpcError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("node error: {0}")]
    Node(#[from] alvenqis_node::NodeError),
    #[error("indexer error: {0}")]
    Indexer(#[from] alvenqis_indexer::IndexerError),
    #[error("core error: {0}")]
    Core(#[from] alvenqis_core::AlvenqisError),
    #[error("config error: {0}")]
    Config(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("rate limited: {0}")]
    RateLimited(String),
}

pub type RpcResult<T> = Result<T, RpcError>;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub status: u16,
}

impl IntoResponse for RpcError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::Node(alvenqis_node::NodeError::ChainNotInitialized(_)) => StatusCode::NOT_FOUND,
            Self::Indexer(alvenqis_indexer::IndexerError::IndexNotInitialized(_))
            | Self::Indexer(alvenqis_indexer::IndexerError::NotFound(_)) => StatusCode::NOT_FOUND,
            Self::Node(alvenqis_node::NodeError::InvalidChainFile { .. })
            | Self::Indexer(alvenqis_indexer::IndexerError::InvalidIndexFile { .. })
            | Self::Node(_)
            | Self::Indexer(_)
            | Self::Core(_)
            | Self::Io(_)
            | Self::Json(_)
            | Self::Toml(_)
            | Self::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(ErrorResponse {
            error: match status {
                StatusCode::BAD_REQUEST
                | StatusCode::NOT_FOUND
                | StatusCode::UNAUTHORIZED
                | StatusCode::TOO_MANY_REQUESTS => self.to_string(),
                _ => "internal server error".to_owned(),
            },
            status: status.as_u16(),
        });
        (status, body).into_response()
    }
}
