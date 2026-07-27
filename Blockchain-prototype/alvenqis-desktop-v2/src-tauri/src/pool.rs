//! Public mining-pool snapshot for Control Center (multi-pool aware).
//! Fetches only public pool HTTP APIs — never admin tokens or wallet secrets.

use crate::availability::{self, ConnectionAvailability};
use crate::error::{AppError, AppResult};
use crate::settings;
use serde_json::{json, Value};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const TIMEOUT_MS: u64 = 8_000;
const MAX_ATTEMPTS: u32 = 2;

fn client() -> AppResult<reqwest::Client> {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client.clone());
    }
    let built = reqwest::Client::builder()
        .timeout(Duration::from_millis(TIMEOUT_MS))
        .connect_timeout(Duration::from_secs(4))
        .pool_max_idle_per_host(4)
        .pool_idle_timeout(Duration::from_secs(60))
        .tcp_keepalive(Duration::from_secs(30))
        .user_agent("Alvenqis-Control-Center/2.0")
        .build()
        .map_err(|e| AppError::msg(format!("HTTP client init failed: {e}")))?;
    let _ = CLIENT.set(built.clone());
    Ok(built)
}

/// Normalize a pool base URL (no trailing slash).
pub fn normalize_pool_url(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim().trim_end_matches('/').to_string();
    if trimmed.is_empty() {
        return Err(AppError::msg("Pool URL cannot be empty."));
    }
    let with_scheme = if trimmed.contains("://") {
        trimmed
    } else {
        format!("http://{trimmed}")
    };
    let parsed = url::Url::parse(&with_scheme)
        .map_err(|e| AppError::msg(format!("Invalid pool URL: {e}")))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(AppError::msg("Pool URL must use http or https."));
    }
    let host = parsed.host_str().unwrap_or("");
    if host.is_empty() {
        return Err(AppError::msg("Pool URL must include a host."));
    }
    // Drop path noise except when the pool is mounted under a subpath (e.g. /pool).
    let mut out = format!(
        "{}://{}",
        parsed.scheme(),
        parsed.host_str().unwrap_or_default()
    );
    if let Some(port) = parsed.port() {
        out.push(':');
        out.push_str(&port.to_string());
    }
    let path = parsed.path().trim_end_matches('/');
    if !path.is_empty() && path != "/" {
        out.push_str(path);
    }
    Ok(out)
}

async fn get_json_once(url: &str) -> AppResult<Value> {
    let response = client()?
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::msg(format!("Pool request failed ({url}): {e}")))?;
    if !response.status().is_success() {
        return Err(AppError::msg(format!(
            "Pool returned HTTP {} for {url}",
            response.status()
        )));
    }
    response
        .json()
        .await
        .map_err(|e| AppError::msg(format!("Invalid JSON from pool ({url}): {e}")))
}

fn retryable(error: &AppError) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    [
        "408",
        "425",
        "429",
        "500",
        "502",
        "503",
        "504",
        "timeout",
        "timed out",
        "connection",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}

async fn get_json(url: &str) -> AppResult<Value> {
    let mut last_error = None;
    for attempt in 0..MAX_ATTEMPTS {
        match get_json_once(url).await {
            Ok(value) => return Ok(value),
            Err(error) => {
                let should_retry = retryable(&error) && attempt + 1 < MAX_ATTEMPTS;
                last_error = Some(error);
                if !should_retry {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(300u64 << attempt)).await;
            }
        }
    }
    Err(last_error.unwrap_or_else(|| AppError::msg("Pool request failed.")))
}

async fn get_json_tracked(
    url: &str,
) -> Result<(Value, ConnectionAvailability), (AppError, ConnectionAvailability)> {
    let key = format!("pool:{url}");
    if let Err(connection) = availability::begin(&key, url) {
        return Err((
            AppError::msg(
                connection
                    .error
                    .clone()
                    .unwrap_or_else(|| "Pool connection circuit is open.".into()),
            ),
            *connection,
        ));
    }
    let started = Instant::now();
    match get_json(url).await {
        Ok(value) => Ok((
            value,
            availability::success(&key, url, started.elapsed().as_millis() as u64),
        )),
        Err(error) => {
            let connection = availability::failure(
                &key,
                url,
                started.elapsed().as_millis() as u64,
                error.to_string(),
            );
            Err((error, connection))
        }
    }
}

async fn get_json_optional(url: &str) -> Option<Value> {
    get_json(url).await.ok()
}

/// Full public snapshot for one pool base URL.
pub async fn pool_snapshot(
    pool_url: Option<String>,
    miner_address: Option<String>,
) -> AppResult<Value> {
    let base = match pool_url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => normalize_pool_url(raw)?,
        None => {
            let settings = settings::get();
            let preferred = if !settings.default_pool_url.trim().is_empty() {
                settings.default_pool_url.clone()
            } else {
                settings
                    .pool_urls
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "http://rpcnode.dohotstudio.com/pool".into())
            };
            normalize_pool_url(&preferred)?
        }
    };

    let status_url = format!("{base}/api/v1/pool/status");
    let history_url = format!("{base}/api/v1/pool/history");
    let payouts_url = format!("{base}/api/v1/payouts");
    let health_url = format!("{base}/health");

    let (status, connection) = match get_json_tracked(&status_url).await {
        Ok(result) => result,
        Err((error, connection)) => {
            let settings = settings::get();
            return Ok(json!({
                "online": false,
                "pool_url": base,
                "fetched_at_unix_seconds": unix_now(),
                "connection": connection,
                "error": error.to_string(),
                "health": null,
                "status": {},
                "history_available": false,
                "workers": [],
                "blocks": [],
                "shares": [],
                "payouts": [],
                "accounts": [],
                "miner": null,
                "configured_pools": settings.pool_urls,
                "default_pool_url": settings.default_pool_url,
            }));
        }
    };
    let history = get_json_optional(&history_url).await;
    let payouts = get_json_optional(&payouts_url)
        .await
        .unwrap_or_else(|| json!([]));
    let health = get_json_optional(&health_url).await;

    let mut miner: Option<Value> = None;
    if let Some(address) = miner_address
        .as_deref()
        .map(str::trim)
        .filter(|s| s.starts_with("alve1"))
    {
        let enc = urlencoding_lite(address);
        miner = get_json_optional(&format!("{base}/api/v1/miners/{enc}")).await;
    }

    // Prefer history payload for deep tables when available; fall back to status subsets.
    let blocks = history
        .as_ref()
        .and_then(|h| h.get("blocks"))
        .cloned()
        .or_else(|| status.get("recent_blocks").cloned())
        .unwrap_or_else(|| json!([]));
    let shares = history
        .as_ref()
        .and_then(|h| h.get("shares"))
        .cloned()
        .unwrap_or_else(|| json!([]));
    let accounts = history
        .as_ref()
        .and_then(|h| h.get("accounts"))
        .cloned()
        .unwrap_or_else(|| json!([]));
    let workers = history
        .as_ref()
        .and_then(|h| h.get("workers"))
        .cloned()
        .or_else(|| status.get("workers").cloned())
        .unwrap_or_else(|| json!([]));
    let history_payouts = history
        .as_ref()
        .and_then(|h| h.get("payouts"))
        .cloned()
        .unwrap_or(payouts);

    Ok(json!({
        "online": true,
        "pool_url": base,
        "fetched_at_unix_seconds": unix_now(),
        "connection": connection,
        "error": null,
        "health": health,
        "status": status,
        "history_available": history.is_some(),
        "workers": workers,
        "blocks": blocks,
        "shares": shares,
        "payouts": history_payouts,
        "accounts": accounts,
        "miner": miner,
        "configured_pools": settings::get().pool_urls,
        "default_pool_url": settings::get().default_pool_url,
    }))
}

/// Probe several configured pools for a quick multi-pool overview.
pub async fn pool_catalog() -> AppResult<Value> {
    let settings = settings::get();
    let mut urls = settings.pool_urls.clone();
    if urls.is_empty() && !settings.default_pool_url.trim().is_empty() {
        urls.push(settings.default_pool_url.clone());
    }
    if urls.is_empty() {
        urls.push("http://rpcnode.dohotstudio.com/pool".into());
    }
    // Dedup while preserving order
    let mut seen = std::collections::HashSet::new();
    urls.retain(|u| seen.insert(u.trim().trim_end_matches('/').to_ascii_lowercase()));

    let mut entries = Vec::new();
    for raw in urls {
        let base = match normalize_pool_url(&raw) {
            Ok(v) => v,
            Err(err) => {
                entries.push(json!({
                    "pool_url": raw,
                    "online": false,
                    "error": err.to_string(),
                }));
                continue;
            }
        };
        let status_url = format!("{base}/api/v1/pool/status");
        match get_json_tracked(&status_url).await {
            Ok((status, connection)) => {
                entries.push(json!({
                    "pool_url": base,
                    "online": true,
                    "connection": connection,
                    "pool_name": status.get("pool_name"),
                    "status_label": status.get("status_label"),
                    "network_id": status.get("network_id"),
                    "connected_workers": status.get("connected_workers"),
                    "estimated_hashrate_hs": status.get("estimated_hashrate_hs"),
                    "blocks_found": status.get("blocks_found"),
                    "accepted_shares": status.get("accepted_shares"),
                    "upstream_status": status.get("upstream_status"),
                    "pool_address": status.get("pool_address"),
                }));
            }
            Err((err, connection)) => {
                entries.push(json!({
                    "pool_url": base,
                    "online": false,
                    "error": err.to_string(),
                    "connection": connection,
                }));
            }
        }
    }

    Ok(json!({
        "default_pool_url": settings.default_pool_url,
        "pools": entries,
        "fetched_at_unix_seconds": unix_now(),
    }))
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn urlencoding_lite(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
