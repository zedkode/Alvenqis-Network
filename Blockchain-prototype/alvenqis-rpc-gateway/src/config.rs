use crate::error::{RpcError, RpcResult};
use alvenqis_core::Network;
use http::HeaderValue;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RpcAccessMode {
    #[default]
    Local,
    PublicRead,
    PublicSubmit,
    InternalEdge,
    PrivateMining,
}

impl RpcAccessMode {
    pub const fn allows_transaction_submission(self) -> bool {
        matches!(self, Self::Local | Self::PublicSubmit | Self::InternalEdge)
    }

    pub const fn allows_operator_endpoints(self) -> bool {
        matches!(self, Self::Local)
    }

    /// Mining routes are enabled only for local or isolated Docker-network
    /// profiles. These listeners must never publish a host port.
    pub const fn default_exposes_mining(self) -> bool {
        matches!(self, Self::Local | Self::InternalEdge | Self::PrivateMining)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RpcConfig {
    pub bind_host: String,
    pub bind_port: u16,
    pub network: Network,
    pub network_id: String,
    pub human_name: String,
    pub status_label: String,
    pub address_prefix: String,
    pub chain_data_path: String,
    #[serde(default)]
    pub indexer_data_path: String,
    #[serde(default)]
    pub mempool_data_path: String,
    #[serde(default)]
    pub public_rpc_allowed: bool,
    #[serde(default)]
    pub access_mode: RpcAccessMode,
    /// When set, may disable mining for a local, internal-edge, or
    /// private-mining profile.
    /// Public profiles cannot opt in to mining.
    #[serde(default)]
    pub expose_mining_endpoints: Option<bool>,
    #[serde(default = "default_max_mempool_transactions")]
    pub max_mempool_transactions: usize,
    #[serde(default = "default_max_request_body_bytes")]
    pub max_request_body_bytes: usize,
    #[serde(default = "default_cors_allowed_origin")]
    pub cors_allowed_origin: String,
    #[serde(default)]
    pub cors_allowed_origins: Vec<String>,
    #[serde(default)]
    pub explorer_static_path: String,
    #[serde(default)]
    pub allow_mainnet_candidate: bool,
    /// Optional shared secret for submit/mining routes (audit CR-H03).
    /// When non-empty, clients must send `Authorization: Bearer <token>` or
    /// `X-Alvenqis-Api-Token: <token>`. Prefer env `ALVENQIS_RPC_API_TOKEN` over committing secrets.
    #[serde(default)]
    pub api_token: String,
    /// Per-IP write budget for submit + mining template/submit (audit CR-H04).
    /// `0` disables in-process limits (rely on reverse proxy only). Default: 120/min.
    #[serde(default = "default_write_rate_limit_per_minute")]
    pub write_rate_limit_per_minute: u32,
    /// Per-IP cap on mining template requests within the rate-limit window.
    #[serde(default = "default_mining_template_rate_limit_per_minute")]
    pub mining_template_rate_limit_per_minute: u32,
}

impl RpcConfig {
    pub fn load_from_path(path: &Path) -> RpcResult<Self> {
        let content = fs::read_to_string(path)?;
        let mut config: Self = toml::from_str(&content)?;
        config.chain_data_path = normalize_configured_path(&config.chain_data_path);
        if config.indexer_data_path.trim().is_empty() {
            config.indexer_data_path =
                alvenqis_indexer::default_index_dir_for_network(config.network)
                    .display()
                    .to_string();
        } else {
            config.indexer_data_path = normalize_configured_path(&config.indexer_data_path);
        }
        if config.mempool_data_path.trim().is_empty() {
            config.mempool_data_path = alvenqis_node::default_mempool_dir(config.network)
                .display()
                .to_string();
        } else {
            config.mempool_data_path = normalize_configured_path(&config.mempool_data_path);
        }
        if !config.explorer_static_path.trim().is_empty() {
            config.explorer_static_path = normalize_configured_path(&config.explorer_static_path);
        }
        if let Some(local_root) =
            std::env::var_os("ALVENQIS_LOCAL_ROOT").filter(|value| !value.is_empty())
        {
            let local_root = PathBuf::from(local_root);
            config.chain_data_path = local_root.join("chain").display().to_string();
            config.indexer_data_path = local_root.join("indexer").display().to_string();
            config.mempool_data_path = local_root.join("mempool").display().to_string();
        }
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> RpcResult<()> {
        if self.bind_host.trim().is_empty() {
            return Err(RpcError::Config("bind_host cannot be empty".to_owned()));
        }
        if self.bind_port == 0 {
            return Err(RpcError::Config(
                "bind_port must be greater than zero".to_owned(),
            ));
        }
        if self.network_id != self.network.network_id() {
            return Err(RpcError::Config(format!(
                "network_id must be {}",
                self.network.network_id()
            )));
        }
        if self.human_name != self.network.human_name() {
            return Err(RpcError::Config(format!(
                "human_name must be {}",
                self.network.human_name()
            )));
        }
        if self.status_label != self.network.status_label() {
            return Err(RpcError::Config(format!(
                "status_label must be {}",
                self.network.status_label()
            )));
        }
        if self.address_prefix != self.network.address_prefix() {
            return Err(RpcError::Config(format!(
                "address_prefix must be {}",
                self.network.address_prefix()
            )));
        }
        if self.bind_port != self.network.default_rpc_port() {
            return Err(RpcError::Config(format!(
                "bind_port must be {}",
                self.network.default_rpc_port()
            )));
        }
        if self.bind_host == "0.0.0.0" && !self.public_rpc_allowed {
            return Err(RpcError::Config(
                "bind_host 0.0.0.0 requires public_rpc_allowed = true".to_owned(),
            ));
        }
        if self.bind_host == "0.0.0.0" && self.access_mode == RpcAccessMode::Local {
            return Err(RpcError::Config(
                "local RPC access mode cannot bind to 0.0.0.0".to_owned(),
            ));
        }
        if self.network.requires_explicit_allow() && !self.allow_mainnet_candidate {
            return Err(RpcError::Config(
                "mainnet candidate RPC requires allow_mainnet_candidate = true".to_owned(),
            ));
        }
        if self.chain_data_path.trim().is_empty() {
            return Err(RpcError::Config(
                "chain_data_path cannot be empty".to_owned(),
            ));
        }
        if self.indexer_data_path.trim().is_empty() {
            return Err(RpcError::Config(
                "indexer_data_path cannot be empty".to_owned(),
            ));
        }
        if self.mempool_data_path.trim().is_empty() {
            return Err(RpcError::Config(
                "mempool_data_path cannot be empty".to_owned(),
            ));
        }
        if self.max_mempool_transactions == 0 {
            return Err(RpcError::Config(
                "max_mempool_transactions must be greater than zero".to_owned(),
            ));
        }
        if self.max_request_body_bytes == 0 {
            return Err(RpcError::Config(
                "max_request_body_bytes must be greater than zero".to_owned(),
            ));
        }
        if self.cors_allowed_origin.trim().is_empty() {
            return Err(RpcError::Config(
                "cors_allowed_origin cannot be empty".to_owned(),
            ));
        }
        for origin in self.effective_cors_origins() {
            // Reject wildcard CORS outside pure Local mode (audit A-M09 / A-H02).
            if origin.trim() == "*" && self.access_mode != RpcAccessMode::Local {
                return Err(RpcError::Config(
                    "CORS origin '*' is only allowed when access_mode = local".to_owned(),
                ));
            }
            HeaderValue::from_str(origin).map_err(|error| {
                RpcError::Config(format!("invalid CORS origin {origin:?}: {error}"))
            })?;
        }
        if self.expose_mining_endpoints == Some(true)
            && !matches!(
                self.access_mode,
                RpcAccessMode::Local | RpcAccessMode::InternalEdge | RpcAccessMode::PrivateMining
            )
        {
            return Err(RpcError::Config(
                "public RPC profiles cannot expose mining endpoints; use local loopback RPC, internal-edge, or private-mining on an unpublished container network".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn effective_api_token(&self) -> Option<String> {
        if let Ok(from_env) = std::env::var("ALVENQIS_RPC_API_TOKEN") {
            let trimmed = from_env.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_owned());
            }
        }
        let configured = self.api_token.trim();
        if configured.is_empty() {
            None
        } else {
            Some(configured.to_owned())
        }
    }

    pub fn effective_cors_origins(&self) -> Vec<&str> {
        if self.cors_allowed_origins.is_empty() {
            vec![self.cors_allowed_origin.as_str()]
        } else {
            self.cors_allowed_origins
                .iter()
                .map(String::as_str)
                .collect()
        }
    }

    /// Whether `/mining/template` and `/mining/submit` serve mining work.
    pub fn mining_endpoints_enabled(&self) -> bool {
        self.access_mode.default_exposes_mining() && self.expose_mining_endpoints.unwrap_or(true)
    }
}

fn default_max_request_body_bytes() -> usize {
    64 * 1024
}

fn default_max_mempool_transactions() -> usize {
    1024
}

fn default_write_rate_limit_per_minute() -> u32 {
    120
}

fn default_mining_template_rate_limit_per_minute() -> u32 {
    60
}

fn default_cors_allowed_origin() -> String {
    "http://127.0.0.1:4173".to_owned()
}

fn normalize_configured_path(configured_path: &str) -> String {
    let path = PathBuf::from(configured_path);
    if path.is_absolute() {
        return configured_path.to_owned();
    }

    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(path)
        .display()
        .to_string()
}
