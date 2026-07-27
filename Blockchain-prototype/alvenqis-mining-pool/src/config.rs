use crate::{PoolError, Result};
use alvenqis_core::Address;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PoolConfig {
    pub bind_host: String,
    pub bind_port: u16,
    pub network_id: String,
    pub status_label: String,
    pub pool_name: String,
    pub pool_address: String,
    pub upstream_rpc_url: String,
    pub public_url: String,
    pub data_dir: PathBuf,
    pub admin_token_file: PathBuf,
    pub share_difficulty_leading_zero_bits: u8,
    #[serde(default = "default_vardiff_enabled")]
    pub vardiff_enabled: bool,
    #[serde(default = "default_min_share_difficulty")]
    pub min_share_difficulty_leading_zero_bits: u8,
    #[serde(default = "default_max_share_difficulty")]
    pub max_share_difficulty_leading_zero_bits: u8,
    #[serde(default = "default_target_share_seconds")]
    pub target_share_seconds: u64,
    #[serde(default = "default_vardiff_window_shares")]
    pub vardiff_window_shares: usize,
    /// Keep share target at least this many leading-zero bits *easier* than network
    /// so multiple shares accumulate between blocks (fair multi-miner PPLNS).
    /// If share_bits == network_bits, the fastest GPU finds every block alone.
    #[serde(default = "default_share_network_gap")]
    pub share_network_gap_bits: u8,
    pub pool_fee_basis_points: u16,
    pub pplns_window_shares: usize,
    pub block_maturity_confirmations: u64,
    pub minimum_payout_atomic: u64,
    /// How often the pool polls upstream for a new tip/template while the current job is still live.
    #[serde(default = "default_job_cache_seconds")]
    pub job_cache_seconds: u64,
    /// Rolling window (seconds) used for live estimated hashrate (not cumulative lifetime work).
    #[serde(default = "default_hashrate_window_seconds")]
    pub hashrate_window_seconds: u64,
    #[serde(default = "default_worker_timeout_seconds")]
    pub worker_timeout_seconds: u64,
    #[serde(default = "default_max_shares")]
    pub max_stored_shares: usize,
    #[serde(default = "default_max_workers_per_address")]
    pub max_workers_per_address: usize,
    #[serde(default = "default_work_requests_per_minute")]
    pub max_work_requests_per_minute: u32,
    #[serde(default = "default_share_requests_per_minute")]
    pub max_share_requests_per_minute: u32,
    #[serde(default = "default_invalid_share_ban_threshold")]
    pub invalid_share_ban_threshold: u32,
    #[serde(default = "default_ban_seconds")]
    pub ban_seconds: u64,
    /// Fail closed when the admin token file is older than this many seconds
    /// (forces periodic manual rotation). Default: 90 days.
    #[serde(default = "default_admin_token_max_age_seconds")]
    pub admin_token_max_age_seconds: u64,
    /// Browser CORS origins. Empty = no wildcard; same-origin / non-browser clients only.
    /// Use exact origins such as `https://pool.example.org`. `*` is allowed only explicitly.
    #[serde(default)]
    pub cors_allowed_origins: Vec<String>,
    /// Must be true to bind non-loopback. Pool remains Prototype until HSM payout + multi-instance
    /// admission (audit A-H07).
    #[serde(default)]
    pub allow_public_pool_prototype: bool,
    /// Optional TLS-only Stratum listener. Plaintext Stratum is intentionally unsupported.
    #[serde(default)]
    pub stratum: Option<StratumConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StratumConfig {
    pub bind_host: String,
    pub bind_port: u16,
    pub tls_cert_file: PathBuf,
    pub tls_key_file: PathBuf,
    #[serde(default = "default_stratum_max_connections")]
    pub max_connections: usize,
    #[serde(default = "default_stratum_max_line_bytes")]
    pub max_line_bytes: usize,
    #[serde(default = "default_stratum_handshake_timeout_seconds")]
    pub handshake_timeout_seconds: u64,
    #[serde(default = "default_stratum_idle_timeout_seconds")]
    pub idle_timeout_seconds: u64,
}

impl PoolConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path).map_err(|error| {
            PoolError::Config(format!("cannot read {}: {error}", path.display()))
        })?;
        let config: Self = toml::from_str(&raw)
            .map_err(|error| PoolError::Config(format!("invalid {}: {error}", path.display())))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        let bind: IpAddr = self
            .bind_host
            .parse()
            .map_err(|_| PoolError::Config("bind_host must be a literal IP".to_owned()))?;
        let is_loopback = match bind {
            IpAddr::V4(v4) => v4.is_loopback(),
            IpAddr::V6(v6) => v6.is_loopback(),
        };
        if !is_loopback && !self.allow_public_pool_prototype {
            return Err(PoolError::Config(
                "non-loopback bind requires allow_public_pool_prototype = true (Prototype only; not a public production pool)".to_owned(),
            ));
        }
        if (bind.is_unspecified() || !is_loopback) && !self.public_url.starts_with("https://") {
            return Err(PoolError::Config(
                "public bind requires an HTTPS public_url".to_owned(),
            ));
        }
        if self.network_id != "alvenqis-mainnet-candidate" {
            return Err(PoolError::Config(
                "pool currently supports alvenqis-mainnet-candidate only".to_owned(),
            ));
        }
        let address = Address::parse(&self.pool_address)
            .map_err(|error| PoolError::Config(format!("invalid pool_address: {error}")))?;
        if address.network().network_id() != self.network_id {
            return Err(PoolError::Config(
                "pool_address belongs to another network".to_owned(),
            ));
        }
        let upstream = reqwest::Url::parse(&self.upstream_rpc_url)
            .map_err(|error| PoolError::Config(format!("invalid upstream_rpc_url: {error}")))?;
        if !matches!(upstream.scheme(), "http" | "https") {
            return Err(PoolError::Config(
                "upstream_rpc_url must use HTTP or HTTPS".to_owned(),
            ));
        }
        if self.pool_fee_basis_points > 1_000 {
            return Err(PoolError::Config(
                "pool fee cannot exceed 10 percent".to_owned(),
            ));
        }
        if self.min_share_difficulty_leading_zero_bits > self.share_difficulty_leading_zero_bits
            || self.share_difficulty_leading_zero_bits > self.max_share_difficulty_leading_zero_bits
            || self.max_share_difficulty_leading_zero_bits > 63
        {
            return Err(PoolError::Config(
                "share difficulty must be between the VarDiff minimum and maximum".to_owned(),
            ));
        }
        if self.share_network_gap_bits == 0 || self.share_network_gap_bits > 16 {
            return Err(PoolError::Config(
                "share_network_gap_bits must be 1..=16 (share target easier than network)"
                    .to_owned(),
            ));
        }
        if self.pplns_window_shares == 0
            || self.block_maturity_confirmations == 0
            || self.minimum_payout_atomic == 0
            || self.max_stored_shares < self.pplns_window_shares
            || self.target_share_seconds == 0
            || self.vardiff_window_shares < 4
            || self.max_workers_per_address == 0
            || self.max_work_requests_per_minute == 0
            || self.max_share_requests_per_minute == 0
            || self.invalid_share_ban_threshold == 0
            || self.ban_seconds == 0
            || self.hashrate_window_seconds == 0
            || self.job_cache_seconds == 0
            || self.admin_token_max_age_seconds == 0
        {
            return Err(PoolError::Config(
                "PPLNS, maturity, payout and storage limits must be positive and coherent"
                    .to_owned(),
            ));
        }
        if let Some(stratum) = &self.stratum {
            let stratum_bind: IpAddr = stratum.bind_host.parse().map_err(|_| {
                PoolError::Config("stratum.bind_host must be a literal IP".to_owned())
            })?;
            if stratum.bind_port == 0 {
                return Err(PoolError::Config(
                    "stratum.bind_port must be non-zero".to_owned(),
                ));
            }
            if stratum.tls_cert_file.as_os_str().is_empty()
                || stratum.tls_key_file.as_os_str().is_empty()
            {
                return Err(PoolError::Config(
                    "Stratum requires both tls_cert_file and tls_key_file".to_owned(),
                ));
            }
            if stratum.tls_cert_file == stratum.tls_key_file {
                return Err(PoolError::Config(
                    "Stratum certificate and private key must be different files".to_owned(),
                ));
            }
            if stratum.max_connections == 0
                || stratum.max_line_bytes < 1_024
                || stratum.max_line_bytes > 1_048_576
                || stratum.handshake_timeout_seconds == 0
                || stratum.idle_timeout_seconds < stratum.handshake_timeout_seconds
            {
                return Err(PoolError::Config(
                    "Stratum connection, line-size and timeout limits are invalid".to_owned(),
                ));
            }
            if !stratum_bind.is_loopback() && !self.allow_public_pool_prototype {
                return Err(PoolError::Config(
                    "public Stratum bind requires allow_public_pool_prototype = true".to_owned(),
                ));
            }
        }
        Ok(())
    }
}

const fn default_vardiff_enabled() -> bool {
    true
}
const fn default_min_share_difficulty() -> u8 {
    16
}
const fn default_max_share_difficulty() -> u8 {
    28
}
const fn default_target_share_seconds() -> u64 {
    15
}
const fn default_vardiff_window_shares() -> usize {
    16
}
const fn default_share_network_gap() -> u8 {
    // Share must be easier than network so both GPUs can submit many shares per block.
    4
}

const fn default_job_cache_seconds() -> u64 {
    // Keep the active job sticky; only re-poll upstream on this cadence while still valid.
    15
}
const fn default_hashrate_window_seconds() -> u64 {
    // Live rate over the last minute — not a long cumulative average.
    60
}
const fn default_worker_timeout_seconds() -> u64 {
    120
}
const fn default_max_shares() -> usize {
    100_000
}
const fn default_max_workers_per_address() -> usize {
    64
}
const fn default_work_requests_per_minute() -> u32 {
    240
}
const fn default_share_requests_per_minute() -> u32 {
    600
}
const fn default_invalid_share_ban_threshold() -> u32 {
    20
}
const fn default_ban_seconds() -> u64 {
    600
}
/// 90 days — operator must rotate admin_token_file (see Blockchain-scripts/operator/).
const fn default_admin_token_max_age_seconds() -> u64 {
    90 * 24 * 60 * 60
}

const fn default_stratum_max_connections() -> usize {
    1_024
}

const fn default_stratum_max_line_bytes() -> usize {
    64 * 1_024
}

const fn default_stratum_handshake_timeout_seconds() -> u64 {
    10
}

const fn default_stratum_idle_timeout_seconds() -> u64 {
    180
}
