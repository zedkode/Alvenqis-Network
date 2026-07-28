use crate::cache::{CachedChain, CachedIndex};
use crate::config::RpcConfig;
use crate::middleware::RateBucket;
use crate::services::StoredMiningTemplate;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct RpcState {
    pub config: RpcConfig,
    pub(crate) node_config_path: PathBuf,
    pub(crate) mining_templates: Arc<Mutex<HashMap<String, StoredMiningTemplate>>>,
    /// Serialize expensive full-chain template builds without blocking the async runtime.
    pub(crate) mining_template_build_lock: Arc<tokio::sync::Mutex<()>>,
    /// Process-local chain cache invalidated by file fingerprint (maturity: multi-client load).
    pub(crate) chain_cache: Arc<Mutex<Option<CachedChain>>>,
    /// Read-only index cache. The dedicated indexer service is the sole index writer.
    pub(crate) index_cache: Arc<Mutex<Option<CachedIndex>>>,
    /// Per-client write rate limits (audit CR-H04).
    pub(crate) rate_limits: Arc<Mutex<HashMap<String, RateBucket>>>,
}

impl RpcState {
    pub fn new(config: RpcConfig) -> Self {
        Self {
            config,
            node_config_path: PathBuf::from("configs/mainnet-candidate.toml"),
            mining_templates: Arc::new(Mutex::new(HashMap::new())),
            mining_template_build_lock: Arc::new(tokio::sync::Mutex::new(())),
            chain_cache: Arc::new(Mutex::new(None)),
            index_cache: Arc::new(Mutex::new(None)),
            rate_limits: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_node_config_path(mut self, path: PathBuf) -> Self {
        self.node_config_path = path;
        self
    }
}
