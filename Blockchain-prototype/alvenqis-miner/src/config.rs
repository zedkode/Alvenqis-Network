use crate::{MinerError, Result};
use alvenqis_core::Address;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Versioned miner configuration (schema v4: CUDA-only, no host search fields).
pub const MINER_CONFIG_SCHEMA_VERSION: u32 = 4;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MinerConfig {
    /// Optional schema version for migrations (default 1 → treat as CPU-only legacy).
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub miner_address: String,
    #[serde(default = "default_nonce_batch_size")]
    pub nonce_batch_size: u64,
    #[serde(default = "default_template_refresh_seconds")]
    pub template_refresh_seconds: u64,
    #[serde(default = "default_status_interval_seconds")]
    pub status_interval_seconds: u64,
    #[serde(default)]
    pub metrics_path: Option<PathBuf>,
    /// Full activity log (every fetch/batch/share/submit). Defaults next to metrics as activity.log.
    #[serde(default)]
    pub activity_log_path: Option<PathBuf>,
    /// Mining compute backend. `auto`/`gpu` migrate to CUDA for compatibility.
    #[serde(default = "default_backend_mode")]
    pub backend_mode: String,
    /// GPU intensity 1-100 (scales batch size). 0 = use default 75.
    #[serde(default = "default_gpu_intensity")]
    pub gpu_intensity: u8,
    /// GPU batch size (work-items per dispatch). 0 = auto.
    #[serde(default)]
    pub gpu_batch_size: u64,
    /// Selected CUDA device ids (`cuda:0:...`) or indexes. Empty = all CUDA GPUs.
    #[serde(default)]
    pub gpu_devices: Vec<String>,
    /// Re-validate every GPU candidate with alvenqis-core (always recommended).
    #[serde(default = "default_true")]
    pub kernel_validation: bool,
    pub source: WorkSourceConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum WorkSourceConfig {
    Rpc {
        url: String,
        #[serde(default = "default_timeout_seconds")]
        timeout_seconds: u64,
    },
    Pool {
        url: String,
        worker_name: String,
        #[serde(default = "default_timeout_seconds")]
        timeout_seconds: u64,
    },
    /// Alvenqis Stratum v1 over TCP or TLS (see `stratum` module + docs).
    Stratum {
        host: String,
        port: u16,
        #[serde(default)]
        use_tls: bool,
        #[serde(default)]
        skip_tls_verify: bool,
        worker_name: String,
        #[serde(default)]
        password: String,
        #[serde(default = "default_timeout_seconds")]
        timeout_seconds: u64,
    },
    LocalFile {
        template_path: PathBuf,
        submission_path: PathBuf,
    },
}

impl MinerConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(|error| {
            MinerError::Config(format!("cannot read {}: {error}", path.display()))
        })?;
        let mut config: Self = toml::from_str(&content)?;
        if config.schema_version < MINER_CONFIG_SCHEMA_VERSION {
            // Soft migration: fill GPU defaults, bump version in-memory only.
            if config.backend_mode.is_empty() {
                config.backend_mode = default_backend_mode();
            }
            if config.gpu_intensity == 0 {
                config.gpu_intensity = default_gpu_intensity();
            }
            config.schema_version = MINER_CONFIG_SCHEMA_VERSION;
        }
        // Keep dispatch latency bounded so telemetry and work refresh stay responsive.
        config.nonce_batch_size = config.nonce_batch_size.clamp(1, MAX_NONCE_BATCH);
        if config.gpu_batch_size > 0 {
            config.gpu_batch_size = config.gpu_batch_size.clamp(1, MAX_GPU_BATCH);
        }
        config.validate()?;
        Ok(config)
    }

    /// Effective per-dispatch work size after clamping (used by mine loop + backends).
    pub fn effective_nonce_batch(&self) -> u64 {
        self.nonce_batch_size.clamp(1, MAX_NONCE_BATCH)
    }

    pub fn effective_gpu_batch(&self) -> u64 {
        let raw = if self.gpu_batch_size > 0 {
            self.gpu_batch_size
        } else {
            self.nonce_batch_size
        };
        raw.clamp(256, MAX_GPU_BATCH)
    }

    pub fn validate(&self) -> Result<()> {
        let address = Address::parse(&self.miner_address).map_err(|error| {
            MinerError::Config(format!(
                "invalid miner_address '{}': {error} (Mainnet Candidate uses alve1… Bech32m; reject foreign HRPs)",
                self.miner_address
            ))
        })?;
        // Product mining targets Alvenqis only — refuse unknown/legacy HRPs at config time.
        let hrp = address.network().address_prefix();
        if !matches!(hrp, "alve" | "talve" | "dalve") {
            return Err(MinerError::Config(format!(
                "miner_address HRP '{hrp}' is not an Alvenqis network (expected alve / talve / dalve)"
            )));
        }
        if self.nonce_batch_size == 0 {
            return Err(MinerError::Config(
                "nonce_batch_size must be at least 1".to_owned(),
            ));
        }
        if self.template_refresh_seconds == 0 || self.status_interval_seconds == 0 {
            return Err(MinerError::Config(
                "refresh and status intervals must be at least 1 second".to_owned(),
            ));
        }
        crate::backends::MiningMode::parse(&self.backend_mode)?;
        if self.gpu_intensity > 100 {
            return Err(MinerError::Config(
                "gpu_intensity must be between 0 and 100".into(),
            ));
        }
        if let WorkSourceConfig::Stratum {
            host,
            port,
            use_tls,
            skip_tls_verify,
            worker_name,
            ..
        } = &self.source
        {
            if host.trim().is_empty() {
                return Err(MinerError::Config("stratum host cannot be empty".into()));
            }
            if *port == 0 {
                return Err(MinerError::Config("stratum port must be non-zero".into()));
            }
            if worker_name.trim().is_empty() {
                return Err(MinerError::Config(
                    "stratum worker_name cannot be empty".into(),
                ));
            }
            let local = matches!(
                host.trim().to_ascii_lowercase().as_str(),
                "127.0.0.1" | "localhost" | "::1" | "[::1]"
            );
            if !*use_tls && !local {
                return Err(MinerError::Config(
                    "remote Stratum requires TLS; plaintext is limited to loopback".into(),
                ));
            }
            if *skip_tls_verify && !local {
                return Err(MinerError::Config(
                    "remote Stratum certificate verification cannot be disabled".into(),
                ));
            }
        }
        if let WorkSourceConfig::Rpc { url, .. } = &self.source {
            let parsed = reqwest::Url::parse(url)
                .map_err(|error| MinerError::Config(format!("invalid RPC URL: {error}")))?;
            if parsed.scheme() != "http" && parsed.scheme() != "https" {
                return Err(MinerError::Config(
                    "RPC URL scheme must be http or https".to_owned(),
                ));
            }
        }
        if let WorkSourceConfig::Pool { worker_name, .. } = &self.source {
            return Err(MinerError::Config(format!(
                "HTTP pool mining is retired (worker {worker_name}); configure kind = \"stratum\" with verified TLS"
            )));
        }
        Ok(())
    }

    pub fn effective_gpu_intensity(&self) -> u8 {
        if self.gpu_intensity == 0 {
            75
        } else {
            self.gpu_intensity.min(100)
        }
    }
}

fn default_schema_version() -> u32 {
    MINER_CONFIG_SCHEMA_VERSION
}

/// Max nonce lease accepted from config (solo/pool). Keeps telemetry live.
const MAX_NONCE_BATCH: u64 = 131_072;
/// Max GPU work-items per dispatch.
const MAX_GPU_BATCH: u64 = 131_072;

const fn default_nonce_batch_size() -> u64 {
    65_536
}

const fn default_template_refresh_seconds() -> u64 {
    5
}

const fn default_status_interval_seconds() -> u64 {
    10
}

const fn default_timeout_seconds() -> u64 {
    10
}

fn default_backend_mode() -> String {
    "cuda".into()
}

const fn default_gpu_intensity() -> u8 {
    75
}

const fn default_true() -> bool {
    true
}
