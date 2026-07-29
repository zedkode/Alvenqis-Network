use crate::protocol::Network;

/// Default public Mainnet Candidate RPC base URL (no trailing slash).
pub const DEFAULT_MAINNET_CANDIDATE_RPC: &str = "https://rpcnode.dohotstudio.com";

/// Local-only mining RPC default. Public VPS mining HTTP is intentionally retired.
pub const DEFAULT_MAINNET_CANDIDATE_MINING_RPC: &str = DEFAULT_LOCAL_MAINNET_CANDIDATE_RPC;

/// Default local Mainnet Candidate RPC base URL (no trailing slash).
pub const DEFAULT_LOCAL_MAINNET_CANDIDATE_RPC: &str = "http://127.0.0.1:10787";

/// Default public Mainnet Candidate mining pool base URL (no trailing slash).
///
/// Aligned with TypeScript `@alvenqis/sdk` (`ALVENQIS_DEFAULT_POOL_URL`).
pub const DEFAULT_MAINNET_CANDIDATE_POOL: &str = "https://pool.dohotstudio.com";

/// Official TLS-only Stratum endpoint for remote GPU mining.
pub const DEFAULT_MAINNET_CANDIDATE_STRATUM_HOST: &str = "stratum.dohotstudio.com";
pub const DEFAULT_MAINNET_CANDIDATE_STRATUM_PORT: u16 = 3333;

/// Network + RPC (+ optional pool) endpoint configuration for SDK consumers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkConfig {
    pub network: Network,
    pub rpc_base_url: String,
    /// Public pool coordinator base (read APIs only). Empty disables pool helpers.
    pub pool_base_url: String,
}

impl NetworkConfig {
    /// Product default: Mainnet Candidate against the public RPC + pool hosts.
    ///
    /// This is **not** Mainnet Live. Status remains candidate / prototype.
    pub fn mainnet_candidate() -> Self {
        Self {
            network: Network::MainnetCandidate,
            rpc_base_url: DEFAULT_MAINNET_CANDIDATE_RPC.to_owned(),
            pool_base_url: DEFAULT_MAINNET_CANDIDATE_POOL.to_owned(),
        }
    }

    /// Mainnet Candidate against a local node/RPC on loopback (pool empty).
    pub fn mainnet_candidate_local() -> Self {
        Self {
            network: Network::MainnetCandidate,
            rpc_base_url: DEFAULT_LOCAL_MAINNET_CANDIDATE_RPC.to_owned(),
            pool_base_url: String::new(),
        }
    }

    /// Custom RPC base URL for the given network (pool defaulted empty).
    pub fn with_rpc(network: Network, rpc_base_url: impl Into<String>) -> Self {
        Self {
            network,
            rpc_base_url: trim_slash(rpc_base_url.into()),
            pool_base_url: String::new(),
        }
    }

    /// Custom RPC + pool bases.
    pub fn with_rpc_and_pool(
        network: Network,
        rpc_base_url: impl Into<String>,
        pool_base_url: impl Into<String>,
    ) -> Self {
        Self {
            network,
            rpc_base_url: trim_slash(rpc_base_url.into()),
            pool_base_url: trim_slash(pool_base_url.into()),
        }
    }

    /// Set / override the public pool base URL.
    pub fn with_pool_url(mut self, pool_base_url: impl Into<String>) -> Self {
        self.pool_base_url = trim_slash(pool_base_url.into());
        self
    }

    pub fn network_id(&self) -> &'static str {
        self.network.network_id()
    }

    pub fn status_label(&self) -> &'static str {
        self.network.status_label()
    }

    pub fn address_prefix(&self) -> &'static str {
        self.network.address_prefix()
    }

    pub fn rpc_url(&self, path: &str) -> String {
        join_base(&self.rpc_base_url, path)
    }

    pub fn pool_url(&self, path: &str) -> crate::error::Result<String> {
        if self.pool_base_url.is_empty() {
            return Err(crate::error::SdkError::input(
                "pool_base_url is not configured (use NetworkConfig::with_pool_url or mainnet_candidate())",
            ));
        }
        Ok(join_base(&self.pool_base_url, path))
    }
}

fn trim_slash(mut url: String) -> String {
    while url.ends_with('/') {
        url.pop();
    }
    url
}

fn join_base(base: &str, path: &str) -> String {
    let path = if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("/{path}")
    };
    format!("{base}{path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mainnet_candidate_defaults_are_honest() {
        let config = NetworkConfig::mainnet_candidate();
        assert_eq!(config.network_id(), "alvenqis-mainnet-candidate");
        assert!(config.status_label().contains("Mainnet Candidate"));
        assert_eq!(config.address_prefix(), "alve");
        assert!(!config.rpc_base_url.ends_with('/'));
        assert_eq!(config.pool_base_url, DEFAULT_MAINNET_CANDIDATE_POOL);
    }

    #[test]
    fn rpc_url_joins_paths() {
        let config = NetworkConfig::with_rpc(Network::MainnetCandidate, "http://127.0.0.1:10787/");
        assert_eq!(config.rpc_url("/status"), "http://127.0.0.1:10787/status");
        assert_eq!(config.rpc_url("health"), "http://127.0.0.1:10787/health");
    }

    #[test]
    fn pool_url_requires_config() {
        let config = NetworkConfig::with_rpc(Network::MainnetCandidate, "http://127.0.0.1:10787");
        assert!(config.pool_url("/api/v1/pool/status").is_err());
        let config = config.with_pool_url("http://127.0.0.1:10787/pool/");
        assert_eq!(
            config.pool_url("/api/v1/pool/status").unwrap(),
            "http://127.0.0.1:10787/pool/api/v1/pool/status"
        );
    }
}
