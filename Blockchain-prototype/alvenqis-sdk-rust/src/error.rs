use crate::protocol::AlvenqisError;
use thiserror::Error;

/// SDK result type.
pub type Result<T> = std::result::Result<T, SdkError>;

/// Errors returned by the Alvenqis client SDK.
#[derive(Debug, Error)]
pub enum SdkError {
    #[error("protocol error: {0}")]
    Protocol(#[from] AlvenqisError),

    #[error("invalid input: {0}")]
    Input(String),

    #[error("RPC request failed: {0}")]
    Rpc(String),

    #[error("RPC returned HTTP {status} for {url}: {body}")]
    RpcHttp {
        status: u16,
        url: String,
        body: String,
    },

    #[error("RPC response decode failed: {0}")]
    RpcDecode(String),

    #[error("transaction lifecycle timeout after {elapsed_ms}ms (last status: {last_status})")]
    TxTimeout {
        elapsed_ms: u64,
        last_status: String,
    },

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl SdkError {
    pub fn input(message: impl Into<String>) -> Self {
        Self::Input(message.into())
    }

    pub fn rpc(message: impl Into<String>) -> Self {
        Self::Rpc(message.into())
    }
}
