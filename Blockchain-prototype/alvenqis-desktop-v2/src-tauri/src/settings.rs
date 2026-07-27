use crate::error::{AppError, AppResult};
use crate::workspace::{settings_path, user_data_dir};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::OnceLock;

pub const DEFAULT_RPC_URL: &str = alvenqis_sdk_rust::DEFAULT_MAINNET_CANDIDATE_RPC;
pub const DEFAULT_MINING_RPC_URL: &str = alvenqis_sdk_rust::DEFAULT_MAINNET_CANDIDATE_MINING_RPC;
pub const DEFAULT_EXPLORER_URL: &str = "https://dohotstudio.com/explorer";

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
    pub default_gpu_batch_size: u64,
    #[serde(default = "default_template_refresh_seconds")]
    pub default_template_refresh_seconds: u64,
    #[serde(default = "default_status_interval_seconds")]
    pub default_status_interval_seconds: u64,
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
    #[serde(default = "default_stratum_timeout_seconds")]
    pub stratum_timeout_seconds: u64,
    #[serde(default = "default_miner_custom_commands")]
    pub miner_custom_commands: Vec<String>,
    pub default_page: String,
    #[serde(default = "default_explorer_url")]
    pub explorer_url: String,
    pub open_external_explorer: bool,
    pub keep_logs_days: u32,
}

fn default_stratum_port() -> u16 {
    alvenqis_sdk_rust::DEFAULT_MAINNET_CANDIDATE_STRATUM_PORT
}

fn default_template_refresh_seconds() -> u64 {
    3
}

fn default_status_interval_seconds() -> u64 {
    2
}

fn default_stratum_timeout_seconds() -> u64 {
    20
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

fn default_explorer_url() -> String {
    DEFAULT_EXPLORER_URL.to_owned()
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
            hide_balances: false,
            mask_addresses: false,
            show_advanced_metrics: true,
            show_technical_labels: true,
            // Pool uses lower share difficulty (VarDiff) so the miner shows progress;
            // solo RPC requires full network difficulty (often 30+ bits) and can look "broken".
            default_miner_mode: "stratum".into(),
            default_miner_backend: "cuda".into(),
            default_gpu_intensity: 90,
            default_gpu_batch_size: 0,
            default_template_refresh_seconds: default_template_refresh_seconds(),
            default_status_interval_seconds: default_status_interval_seconds(),
            default_gpu_devices: Vec::new(),
            default_pool_url: alvenqis_sdk_rust::DEFAULT_MAINNET_CANDIDATE_POOL.to_owned(),
            pool_urls: vec![alvenqis_sdk_rust::DEFAULT_MAINNET_CANDIDATE_POOL.to_owned()],
            default_worker_name: "desktop-01".into(),
            stratum_host: alvenqis_sdk_rust::DEFAULT_MAINNET_CANDIDATE_STRATUM_HOST.to_owned(),
            stratum_port: default_stratum_port(),
            stratum_use_tls: true,
            stratum_skip_tls_verify: false,
            stratum_password: String::new(),
            stratum_timeout_seconds: default_stratum_timeout_seconds(),
            miner_custom_commands: default_miner_custom_commands(),
            default_page: "overview".into(),
            explorer_url: default_explorer_url(),
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
            migrate_mining_settings(&mut settings);
            let _ = persist(&settings);
            settings
        }
        Err(_) => AppSettings::default(),
    }
}

fn migrate_mining_settings(settings: &mut AppSettings) {
    if settings
        .mining_rpc_url
        .eq_ignore_ascii_case(DEFAULT_RPC_URL)
    {
        settings.mining_rpc_url = DEFAULT_MINING_RPC_URL.to_owned();
    }
    let legacy_pool = "https://rpcnode.dohotstudio.com/pool";
    if settings.default_pool_url.eq_ignore_ascii_case(legacy_pool) {
        settings.default_pool_url = alvenqis_sdk_rust::DEFAULT_MAINNET_CANDIDATE_POOL.to_owned();
    }
    for pool_url in &mut settings.pool_urls {
        if pool_url.eq_ignore_ascii_case(legacy_pool) {
            *pool_url = alvenqis_sdk_rust::DEFAULT_MAINNET_CANDIDATE_POOL.to_owned();
        }
    }
    if settings.default_miner_mode == "pool" {
        settings.default_miner_mode = "stratum".into();
    }
    if settings.stratum_host.trim().is_empty() {
        settings.stratum_host =
            alvenqis_sdk_rust::DEFAULT_MAINNET_CANDIDATE_STRATUM_HOST.to_owned();
    }
    if settings.explorer_url.trim().is_empty() {
        settings.explorer_url = default_explorer_url();
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
    if let Some(explorer) = patch.get("explorer_url").and_then(|v| v.as_str()) {
        settings.explorer_url = normalize_explorer_url(explorer)?;
    }
    // Allow 3s local floor in settings UI; App.tsx raises remote polls to ≥10s.
    settings.refresh_interval_ms = settings.refresh_interval_ms.clamp(3_000, 60_000);
    settings.live_log_interval_ms = settings.live_log_interval_ms.clamp(2_000, 30_000);
    settings.keep_logs_days = settings.keep_logs_days.clamp(1, 365);
    settings.default_gpu_intensity = settings.default_gpu_intensity.clamp(1, 100);
    settings.default_gpu_batch_size = if settings.default_gpu_batch_size == 0 {
        0
    } else {
        settings.default_gpu_batch_size.clamp(256, 131_072)
    };
    settings.default_template_refresh_seconds =
        settings.default_template_refresh_seconds.clamp(1, 60);
    settings.default_status_interval_seconds =
        settings.default_status_interval_seconds.clamp(1, 60);
    settings.stratum_timeout_seconds = settings.stratum_timeout_seconds.clamp(5, 120);
    settings.default_gpu_devices = settings
        .default_gpu_devices
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .take(16)
        .collect();
    if settings.default_miner_mode == "pool" {
        settings.default_miner_mode = "stratum".into();
    } else if settings.default_miner_mode != "solo" && settings.default_miner_mode != "stratum" {
        settings.default_miner_mode = "solo".into();
    }
    let backend = settings.default_miner_backend.to_ascii_lowercase();
    // Continuous CPU modes removed from product — migrate to GPU auto.
    settings.default_miner_backend = match backend.as_str() {
        "gpu" | "cuda" | "auto" => "cuda".into(),
        _ => "cuda".into(),
    };
    settings.stratum_host = settings.stratum_host.trim().to_owned();
    if settings.stratum_host.is_empty()
        || settings.stratum_host.contains("://")
        || settings.stratum_host.chars().any(char::is_whitespace)
    {
        return Err(AppError::msg(
            "Stratum host must be a DNS name or IP address without a URL scheme.",
        ));
    }
    settings.default_worker_name = settings.default_worker_name.trim().to_owned();
    if settings.default_worker_name.is_empty()
        || settings.default_worker_name.len() > 64
        || settings
            .default_worker_name
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace())
    {
        return Err(AppError::msg(
            "Worker name must contain 1-64 non-whitespace characters.",
        ));
    }
    let local_stratum = matches!(
        settings.stratum_host.to_ascii_lowercase().as_str(),
        "127.0.0.1" | "localhost" | "::1" | "[::1]"
    );
    if !local_stratum && !settings.stratum_use_tls {
        return Err(AppError::msg("Remote Stratum requires TLS."));
    }
    if !local_stratum && settings.stratum_skip_tls_verify {
        return Err(AppError::msg(
            "Remote Stratum certificate verification cannot be disabled.",
        ));
    }
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

pub fn get_explorer_url() -> String {
    let value = get().explorer_url;
    if value.trim().is_empty() {
        DEFAULT_EXPLORER_URL.to_owned()
    } else {
        value
    }
}

pub fn normalize_explorer_url(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(AppError::msg("Enter a public Alvenqis Explorer URL."));
    }
    let url = url::Url::parse(trimmed)?;
    let local = matches!(
        url.host_str().map(str::to_ascii_lowercase).as_deref(),
        Some("127.0.0.1" | "localhost" | "::1")
    );
    if url.scheme() != "https" && !(local && url.scheme() == "http") {
        return Err(AppError::msg(
            "The public Explorer must use HTTPS; HTTP is allowed only for localhost.",
        ));
    }
    if url.username() != "" || url.password().is_some() {
        return Err(AppError::msg(
            "The Explorer URL cannot contain embedded credentials.",
        ));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(AppError::msg(
            "Enter the Explorer base URL without a query or fragment.",
        ));
    }
    Ok(trimmed.to_owned())
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

#[cfg(test)]
mod tests {
    use super::{normalize_explorer_url, DEFAULT_EXPLORER_URL};

    #[test]
    fn public_explorer_requires_https() {
        assert_eq!(
            normalize_explorer_url(DEFAULT_EXPLORER_URL).unwrap(),
            DEFAULT_EXPLORER_URL
        );
        assert!(normalize_explorer_url("http://explorer.example.com").is_err());
        assert!(normalize_explorer_url("http://127.0.0.1:4173").is_ok());
    }

    #[test]
    fn public_explorer_rejects_credentials_and_query() {
        assert!(normalize_explorer_url("https://user:secret@example.com/explorer").is_err());
        assert!(normalize_explorer_url("https://example.com/explorer?q=secret").is_err());
    }
}
