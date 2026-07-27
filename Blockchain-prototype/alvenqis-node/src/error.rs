use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("SQLite storage error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("core validation error: {0}")]
    Core(#[from] alvenqis_core::AlvenqisError),
    #[error("network config mismatch: {0}")]
    ConfigMismatch(String),
    #[error("invalid local input: {0}")]
    Input(String),
    #[error("local chain is not initialized at {0}")]
    ChainNotInitialized(PathBuf),
    #[error("invalid chain file at {path} on line {line}: {message}")]
    InvalidChainFile {
        path: PathBuf,
        line: usize,
        message: String,
    },
    #[error("invalid chain database at {path}: {message}")]
    InvalidChainDatabase { path: PathBuf, message: String },
    #[error("invalid mempool file at {path}: {message}")]
    InvalidMempoolFile { path: PathBuf, message: String },
    #[error("mempool is full: limit {limit} pending transactions reached")]
    MempoolFull { limit: usize },
    #[error("reset is not allowed for network {0}")]
    ResetNotAllowed(String),
    #[error("reset requires explicit confirmation with --confirm for network {0}")]
    ResetConfirmationRequired(String),
    #[error("network mismatch: expected {expected}, got {actual}")]
    NetworkMismatch { expected: String, actual: String },
    #[error("chain magic mismatch: expected {expected}, got {actual}")]
    ChainMagicMismatch { expected: String, actual: String },
    #[error("genesis mismatch: expected {expected}, got {actual}")]
    GenesisMismatch { expected: String, actual: String },
    #[error("P2P error: {0}")]
    P2p(String),
    #[error("stale chain tip: expected {expected}, got {actual}")]
    StaleChainTip { expected: String, actual: String },
    #[error("invalid storage path for network {network}: expected a path under {expected_root}, got {actual_path}")]
    InvalidDataPath {
        network: String,
        expected_root: String,
        actual_path: String,
    },
    #[error("node shutdown requested but no running node marker exists for network {0}")]
    ShutdownNotRunning(String),
    #[error("reset refused because the local node is still marked running for network {0}")]
    ResetWhileNodeRunning(String),
}

pub type NodeResult<T> = Result<T, NodeError>;
