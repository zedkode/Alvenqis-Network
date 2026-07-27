use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum MinerError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("invalid mining template: {0}")]
    InvalidTemplate(String),
    #[error("mining RPC error: {0}")]
    Rpc(String),
    #[error("work file does not exist: {0}")]
    WorkFileMissing(PathBuf),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("core error: {0}")]
    Core(#[from] alvenqis_core::AlvenqisError),
    #[error("worker thread panicked")]
    WorkerPanicked,
    #[error("GPU mining error: {0}")]
    Gpu(String),
}

pub type Result<T> = std::result::Result<T, MinerError>;
