use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("SQLite storage error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("node error: {0}")]
    Node(#[from] alvenqis_node::NodeError),
    #[error("core error: {0}")]
    Core(#[from] alvenqis_core::AlvenqisError),
    #[error("chain index is not initialized at {0}")]
    IndexNotInitialized(PathBuf),
    #[error("invalid index file at {path}: {message}")]
    InvalidIndexFile { path: PathBuf, message: String },
    #[error("resource not found: {0}")]
    NotFound(String),
}

pub type IndexerResult<T> = Result<T, IndexerError>;
