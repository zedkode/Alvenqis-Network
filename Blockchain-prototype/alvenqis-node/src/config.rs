use crate::error::{NodeError, NodeResult};
use alvenqis_core::{launch_protocol_parameters, Network};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const MAX_P2P_PEERS: usize = 128;
pub const MAX_SEED_NODES: usize = 32;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkConfig {
    pub network: Network,
    pub network_id: String,
    pub human_name: String,
    pub status_label: String,
    pub block_time_seconds: u64,
    pub difficulty_leading_zero_bits: u8,
    pub ticker: String,
    pub address_prefix: String,
    pub max_supply: String,
    pub halving_interval: u64,
    pub initial_block_reward: String,
    pub default_rpc_port: u16,
    pub default_p2p_port: u16,
    #[serde(default = "default_p2p_bind_host")]
    pub p2p_bind_host: String,
    #[serde(default)]
    pub p2p_listen_port: Option<u16>,
    #[serde(default = "default_max_peers")]
    pub max_peers: usize,
    #[serde(default)]
    pub seed_nodes: Vec<String>,
    #[serde(default = "default_max_mempool_transactions")]
    pub max_mempool_transactions: usize,
    pub genesis_config_path: String,
    /// Immutable human-readable value covered by the published genesis review.
    ///
    /// This may differ from `human_name` after a product-brand migration. When
    /// omitted, new networks use the active human name.
    #[serde(default)]
    pub genesis_review_human_name: Option<String>,
    #[serde(default)]
    pub genesis_approval_path: Option<String>,
    pub chain_magic_hex: String,
    #[serde(default)]
    pub allow_mainnet_candidate: bool,
}

fn default_max_mempool_transactions() -> usize {
    1024
}

fn default_p2p_bind_host() -> String {
    "127.0.0.1".to_owned()
}

fn default_max_peers() -> usize {
    64
}

impl NetworkConfig {
    pub fn load_from_path(path: &Path) -> NodeResult<Self> {
        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> NodeResult<()> {
        let protocol = launch_protocol_parameters(self.network);
        if self.network_id != protocol.network_id {
            return Err(NodeError::ConfigMismatch(format!(
                "network_id must be {}",
                protocol.network_id
            )));
        }
        if self.human_name != protocol.network_name {
            return Err(NodeError::ConfigMismatch(format!(
                "human_name must be {}",
                protocol.network_name
            )));
        }
        if self.status_label != protocol.status_label {
            return Err(NodeError::ConfigMismatch(format!(
                "status_label must be {}",
                protocol.status_label
            )));
        }
        if self.block_time_seconds != protocol.block_time_seconds {
            return Err(NodeError::ConfigMismatch(format!(
                "block_time_seconds must be {}",
                protocol.block_time_seconds
            )));
        }
        if self.ticker != protocol.ticker {
            return Err(NodeError::ConfigMismatch(format!(
                "ticker must be {}",
                protocol.ticker
            )));
        }
        if self.address_prefix != protocol.address_prefix {
            return Err(NodeError::ConfigMismatch(format!(
                "address_prefix must be {}",
                protocol.address_prefix
            )));
        }
        if self.max_supply != protocol.max_supply_alve.to_string() {
            return Err(NodeError::ConfigMismatch(format!(
                "max_supply must be {}",
                protocol.max_supply_alve
            )));
        }
        if self.halving_interval != protocol.halving_interval_blocks {
            return Err(NodeError::ConfigMismatch(format!(
                "halving_interval must be {}",
                protocol.halving_interval_blocks
            )));
        }
        if self.initial_block_reward != protocol.initial_block_reward_alve {
            return Err(NodeError::ConfigMismatch(format!(
                "initial_block_reward must be {}",
                protocol.initial_block_reward_alve
            )));
        }
        if self.default_rpc_port != protocol.default_rpc_port {
            return Err(NodeError::ConfigMismatch(format!(
                "default_rpc_port must be {}",
                protocol.default_rpc_port
            )));
        }
        if self.default_p2p_port != protocol.default_p2p_port {
            return Err(NodeError::ConfigMismatch(format!(
                "default_p2p_port must be {}",
                protocol.default_p2p_port
            )));
        }
        if self.p2p_bind_host.trim().is_empty() {
            return Err(NodeError::ConfigMismatch(
                "p2p_bind_host cannot be empty".to_owned(),
            ));
        }
        if self.p2p_listen_port == Some(0) {
            return Err(NodeError::ConfigMismatch(
                "p2p_listen_port must be greater than zero".to_owned(),
            ));
        }
        if self.max_peers == 0 || self.max_peers > MAX_P2P_PEERS {
            return Err(NodeError::ConfigMismatch(format!(
                "max_peers must be between 1 and {MAX_P2P_PEERS}"
            )));
        }
        if self.seed_nodes.len() > MAX_SEED_NODES {
            return Err(NodeError::ConfigMismatch(format!(
                "seed_nodes cannot contain more than {MAX_SEED_NODES} entries"
            )));
        }
        let mut unique_seeds = std::collections::BTreeSet::new();
        for seed in &self.seed_nodes {
            let seed = seed.trim();
            if seed.is_empty() {
                return Err(NodeError::ConfigMismatch(
                    "seed_nodes cannot contain empty entries".to_owned(),
                ));
            }
            if seed.len() > 512 || seed.chars().any(char::is_control) {
                return Err(NodeError::ConfigMismatch(
                    "seed_nodes contains an invalid or oversized entry".to_owned(),
                ));
            }
            if !unique_seeds.insert(seed.to_ascii_lowercase()) {
                return Err(NodeError::ConfigMismatch(format!(
                    "seed_nodes contains duplicate entry {seed}"
                )));
            }
        }
        if self.max_mempool_transactions == 0 {
            return Err(NodeError::ConfigMismatch(
                "max_mempool_transactions must be greater than zero".to_owned(),
            ));
        }
        if self.genesis_config_path != self.network.genesis_config_path() {
            return Err(NodeError::ConfigMismatch(format!(
                "genesis_config_path must be {}",
                self.network.genesis_config_path()
            )));
        }
        if self
            .genesis_review_human_name
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(NodeError::ConfigMismatch(
                "genesis_review_human_name cannot be empty when provided".to_owned(),
            ));
        }
        if self
            .genesis_approval_path
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(NodeError::ConfigMismatch(
                "genesis_approval_path cannot be empty when provided".to_owned(),
            ));
        }
        if self.chain_magic_hex != chain_magic_hex(self.network) {
            return Err(NodeError::ConfigMismatch(format!(
                "chain_magic_hex must be {}",
                chain_magic_hex(self.network)
            )));
        }
        if self.network.requires_explicit_allow() && !self.allow_mainnet_candidate {
            return Err(NodeError::ConfigMismatch(
                "mainnet candidate requires allow_mainnet_candidate = true".to_owned(),
            ));
        }
        if self.network.requires_explicit_allow() && self.genesis_approval_path.is_none() {
            return Err(NodeError::ConfigMismatch(
                "mainnet candidate requires genesis_approval_path".to_owned(),
            ));
        }
        if !self.network.allows_low_difficulty_defaults() && self.difficulty_leading_zero_bits <= 8
        {
            return Err(NodeError::ConfigMismatch(
                "non-devnet networks must not use low difficulty defaults".to_owned(),
            ));
        }
        if self.difficulty_leading_zero_bits < self.network.minimum_difficulty_leading_zero_bits()
            || self.difficulty_leading_zero_bits
                > self.network.maximum_difficulty_leading_zero_bits()
        {
            return Err(NodeError::ConfigMismatch(format!(
                "difficulty_leading_zero_bits must stay between {} and {} for {}",
                self.network.minimum_difficulty_leading_zero_bits(),
                self.network.maximum_difficulty_leading_zero_bits(),
                self.network.network_id()
            )));
        }
        Ok(())
    }

    pub fn p2p_listen_port(&self) -> u16 {
        self.p2p_listen_port.unwrap_or(self.default_p2p_port)
    }

    pub fn genesis_review_human_name(&self) -> &str {
        self.genesis_review_human_name
            .as_deref()
            .unwrap_or(&self.human_name)
    }
}

fn chain_magic_hex(network: Network) -> String {
    launch_protocol_parameters(network)
        .chain_magic_bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
