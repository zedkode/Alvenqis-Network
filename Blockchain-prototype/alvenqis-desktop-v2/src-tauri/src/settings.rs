use crate::error::{AppError, AppResult};
use crate::workspace::{settings_path, user_data_dir};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::OnceLock;

pub const DEFAULT_RPC_URL: &str = alvenqis_sdk_rust::DEFAULT_MAINNET_CANDIDATE_RPC;
pub const DEFAULT_MINING_RPC_URL: &str =
    alvenqis_sdk_rust::DEFAULT_MAINNET_CANDIDATE_MINING_RPC;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub rpc_url: String,
    pub mining_rpc_url: String,
    pub language: String,
    pub theme: String,
    pub density: String,
    pub accent: String,
    pub refresh_interval_ms: u64,
    pub live_log_interval_ms: u64,
    pub reduce_motion: bool,
    pub confirm_before_operator: bool,
    pub auto_start_services: bool,
    pub start_minimized: bool,
    pub notify_block_mined: bool,
    pub notify_sound: bool,
    pub notify_updates: bool,
    /// When true (default), GitHub Releases are applied automatically without approval.
    #[serde(default = "default_true")]
    pub auto_update: bool,
    /// Poll interval for GitHub auto-update (seconds). Minimum 60.
    #[serde(default = "default_auto_update_interval")]
    pub auto_update_interval_secs: u64,
    pub hide_balances: bool,
    pub mask_addresses: bool,
    pub show_advanced_metrics: bool,
    pub show_technical_labels: bool,
    /// Retained for schema compatibility; product mining is GPU-only.
    pub default_miner_mode: String,
    /// Product compute backend. Legacy values migrate to CUDA.
    pub default_miner_backend: String,
    pub default_gpu_intensity: u8,
    #[serde(default)]
    pub default_gpu_devices: Vec<String>,
    pub default_pool_url: String,
    /// Saved pool endpoints for multi-pool Control Center selection (HTTP or HTTPS base URLs).
    #[serde(default)]
    pub pool_urls: Vec<String>,
    pub default_worker_name: String,
    #[serde(default)]
    pub stratum_host: String,
    #[serde(default = "default_stratum_port")]
    pub stratum_port: u16,
    #[serde(default = "default_true")]
    pub stratum_use_tls: bool,
    #[serde(default)]
    pub stratum_skip_tls_verify: bool,
    #[serde(default)]
    pub stratum_password: String,
    #[serde(default = "default_miner_custom_commands")]
    pub miner_custom_commands: Vec<String>,
    pub default_page: String,
    pub open_external_explorer: bool,
    pub keep_logs_days: u32,
}

fn default_stratum_port() -> u16 {
    3333
}

fn default_miner_custom_commands() -> Vec<String> {
    vec![
        "status".into(),
        "devices".into(),
        "config validate".into(),
        "benchmark --seconds 3".into(),
    ]
}

fn default_true() -> bool {
    true
}

fn default_auto_update_interval() -> u64 {
    900
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            rpc_url: DEFAULT_RPC_URL.to_string(),
            mining_rpc_url: DEFAULT_MINING_RPC_URL.to_string(),
            language: "en".into(),
            theme: "dark".into(),
            density: "comfortable".into(),
            accent: "cyan".into(),
            // V2: snappier local UX; remote floor still enforced in the UI poller.
            refresh_interval_ms: 6_000,
            live_log_interval_ms: 2_000,
            reduce_motion: false,
            confirm_before_operator: true,
            auto_start_services: false,
            start_minimized: false,
            notify_block_mined: true,
            notify_sound: true,
            notify_updates: true,
            auto_update: true,
            auto_update_interval_secs: 900,
            hide_balances: false,
            mask_addresses: false,
            show_advanced_metrics: true,
            show_technical_labels: true,
            // Pool uses lower share difficulty (VarDiff) so the miner shows progress;
            // solo RPC requires full network difficulty (often 30+ bits) and can look "broken".
            default_miner_mode: "pool".into(),
            default_miner_backend: "cuda".into(),
            default_gpu_intensity: 90,
            default_gpu_devices: Vec::new(),
            default_pool_url: alvenqis_sdk_rust::DEFAULT_MAINNET_CANDIDATE_POOL.to_owned(),
            pool_urls: vec![alvenqis_sdk_rust::DEFAULT_MAINNET_CANDIDATE_POOL.to_owned()],
            default_worker_name: "desktop-01".into(),
            stratum_host: String::new(),
            stratum_port: default_stratum_port(),
            stratum_use_tls: true,
            stratum_skip_tls_verify: false,
            stratum_password: String::new(),
            miner_custom_commands: default_miner_custom_commands(),
            default_page: "overview".into(),
            open_external_explorer: true,
            keep_logs_days: 14,
        }
    }
}

static SETTINGS: OnceLock<RwLock<AppSettings>> = OnceLock::new();

fn store() -> &'static RwLock<AppSettings> {
    SETTINGS.get_or_init(|| RwLock::new(load_from_disk()))
}

fn load_from_disk() -> AppSettings {
    let path = settings_path();
    match fs::read_to_string(&path) {
        Ok(raw) => {
            let mut settings: AppSettings = serde_json::from_str(&raw).unwrap_or_default();
            migrate_official_mining_endpoints_to_http(&mut settings);
            let _ = persist(&settings);
            settings
        }
        Err(_) => AppSettings::default(),
    }
}

fn migrate_official_mining_endpoints_to_http(settings: &mut AppSettings) {
    if settings.mining_rpc_url.eq_ignore_ascii_case(DEFAULT_RPC_URL) {
        settings.mining_rpc_url = DEFAULT_MINING_RPC_URL.to_owned();
    }
    let legacy_pool = "https://rpcnode.dohotstudio.com/pool";
    if settings.default_pool_url.eq_ignore_ascii_case(legacy_pool) {
        settings.default_pool_url =
            alvenqis_sdk_rust::DEFAULT_MAINNET_CANDIDATE_POOL.to_owned();
    }
    for pool_url in &mut settings.pool_urls {
        if pool_url.eq_ignore_ascii_case(legacy_pool) {
            *pool_url = alvenqis_sdk_rust::DEFAULT_MAINNET_CANDIDATE_POOL.to_owned();
        }
    }
}

fn persist(settings: &AppSettings) -> AppResult<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(settings)?;
    fs::write(path, raw)?;
    let _ = user_data_dir();
    Ok(())
}

pub fn get() -> AppSettings {
    store().read().clone()
}

pub fn defaults() -> AppSettings {
    AppSettings::default()
}

pub fn reset() -> AppResult<AppSettings> {
    let settings = AppSettings::default();
    persist(&settings)?;
    *store().write() = settings.clone();
    Ok(settings)
}

pub fn update(patch: serde_json::Value) -> AppResult<AppSettings> {
    let mut current = serde_json::to_value(get())?;
    if let (Some(obj), Some(patch_obj)) = (current.as_object_mut(), patch.as_object()) {
        for (key, value) in patch_obj {
            obj.insert(key.clone(), value.clone());
        }
    }
    let mut settings: AppSettings = serde_json::from_value(current)?;
    if let Some(rpc) = patch.get("rpc_url").and_then(|v| v.as_str()) {
        settings.rpc_url = normalize_rpc_url(rpc)?;
    }
    if let Some(rpc) = patch.get("mining_rpc_url").and_then(|v| v.as_str()) {
        settings.mining_rpc_url = normalize_mining_rpc_url(rpc)?;
    }
    // Allow 3s local floor in settings UI; App.tsx raises remote polls to ≥10s.
    settings.refresh_interval_ms = settings.refresh_interval_ms.clamp(3_000, 60_000);
    settings.live_log_interval_ms = settings.live_log_interval_ms.clamp(2_000, 30_000);
    settings.auto_update_interval_secs = settings.auto_update_interval_secs.clamp(60, 86_400);
    settings.keep_logs_days = settings.keep_logs_days.clamp(1, 365);
    settings.default_gpu_intensity = settings.default_gpu_intensity.clamp(1, 100);
    settings.default_gpu_devices = settings
        .default_gpu_devices
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .take(16)
        .collect();
    if settings.default_miner_mode != "solo"
        && settings.default_miner_mode != "pool"
        && settings.default_miner_mode != "stratum"
    {
        settings.default_miner_mode = "solo".into();
    }
    let backend = settings.default_miner_backend.to_ascii_lowercase();
    // Continuous CPU modes removed from product — migrate to GPU auto.
    settings.default_miner_backend = match backend.as_str() {
        "gpu" | "cuda" | "auto" => "cuda".into(),
        _ => "cuda".into(),
    };
    // Multi-pool list: normalize, dedupe, keep default_pool_url first when present.
    let mut pool_urls: Vec<String> = settings
        .pool_urls
        .into_iter()
        .filter_map(|u| {
            let t = u.trim().trim_end_matches('/').to_string();
            if t.is_empty() {
                None
            } else if t.contains("://") {
                Some(t)
            } else {
                Some(format!("http://{t}"))
            }
        })
        .take(16)
        .collect();
    let default_pool = settings
        .default_pool_url
        .trim()
        .trim_end_matches('/')
        .to_string();
    if !default_pool.is_empty()
        && !pool_urls
            .iter()
            .any(|u| u.eq_ignore_ascii_case(&default_pool))
    {
        pool_urls.insert(0, default_pool.clone());
    }
    if pool_urls.is_empty() && !default_pool.is_empty() {
        pool_urls.push(default_pool.clone());
    }
    // Dedupe case-insensitively preserving order
    let mut seen = std::collections::HashSet::new();
    pool_urls.retain(|u| seen.insert(u.to_ascii_lowercase()));
    settings.pool_urls = pool_urls;
    if settings.default_pool_url.trim().is_empty() {
        if let Some(first) = settings.pool_urls.first() {
            settings.default_pool_url = first.clone();
        }
    } else {
        settings.default_pool_url = default_pool;
    }
    persist(&settings)?;
    *store().write() = settings.clone();
    Ok(settings)
}

pub fn get_rpc_url() -> String {
    let value = get().rpc_url;
    if value.trim().is_empty() {
        DEFAULT_RPC_URL.to_string()
    } else {
        value
    }
}

pub fn get_mining_rpc_url() -> String {
    let value = get().mining_rpc_url;
    if value.trim().is_empty() {
        DEFAULT_MINING_RPC_URL.to_string()
    } else {
        value
    }
}

pub fn set_rpc_url(raw: &str) -> AppResult<String> {
    let normalized = normalize_rpc_url(raw)?;
    let mut settings = get();
    settings.rpc_url = normalized.clone();
    persist(&settings)?;
    *store().write() = settings;
    Ok(normalized)
}

pub fn normalize_rpc_url(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim().trim_end_matches('/').to_string();
    if trimmed.is_empty() {
        return Err(AppError::msg("Enter a Alvenqis RPC endpoint URL."));
    }
    // Static patterns: compile once; fall back to literal checks if regex build fails.
    let local_host = match regex::Regex::new(r"^(localhost|127\.0\.0\.1|\[::1\])(?::\d+)?$") {
        Ok(re) => re.is_match(&trimmed),
        Err(_) => {
            let host = trimmed.split('/').next().unwrap_or(&trimmed);
            host == "localhost" || host.starts_with("127.0.0.1") || host.starts_with("[::1]")
        }
    };
    let has_scheme = match regex::Regex::new(r"^[a-z][a-z0-9+.-]*://") {
        Ok(re) => re.is_match(&trimmed),
        Err(_) => trimmed.contains("://"),
    };
    let with_scheme = if has_scheme {
        trimmed
    } else if local_host {
        format!("http://{trimmed}")
    } else {
        format!("https://{trimmed}")
    };
    let url = url::Url::parse(&with_scheme)?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(AppError::msg(
            "The RPC endpoint must use http:// or https://.",
        ));
    }
    if url.port() == Some(20787) {
        return Err(AppError::msg(
            "Port 20787 is Alvenqis P2P, not HTTP RPC. Use the RPC gateway without the P2P port.",
        ));
    }
    if url.path() != "/" && !url.path().is_empty() {
        return Err(AppError::msg("Enter the RPC base URL without an API path."));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(AppError::msg(
            "The RPC endpoint cannot contain a query or fragment.",
        ));
    }
    Ok(url.origin().ascii_serialization())
}

fn normalize_mining_rpc_url(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.contains("://") {
        normalize_rpc_url(trimmed)
    } else {
        normalize_rpc_url(&format!("http://{trimmed}"))
    }
}
