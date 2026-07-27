use thiserror::Error;

#[derive(Debug, Error)]
pub enum WalletError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("core error: {0}")]
    Core(#[from] alvenqis_core::AlvenqisError),
    #[error("node error: {0}")]
    Node(#[from] alvenqis_node::NodeError),
    #[error("wallet not found at {0}")]
    WalletNotFound(String),
    #[error("wallet storage path is unavailable")]
    StorageUnavailable,
    #[error("RPC unavailable: {0}")]
    RpcUnavailable(String),
    #[error("wallet input error: {0}")]
    Input(String),
}

pub type WalletResult<T> = Result<T, WalletError>;
