use crate::error::AppResult;
use crate::keystore::WalletMetadata;
use crate::process::{is_local_rpc_url, managed_process_running};
use crate::settings::get_rpc_url;
use crate::workspace::{find_workspace_root, local_root};
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

/// Local loopback RPC is usually fast.
const LOCAL_TIMEOUT_MS: u64 = 8_000;
/// Public VPS status can stall under gateway load; keep requests bounded.
const REMOTE_TIMEOUT_MS: u64 = 25_000;
const REMOTE_HEALTH_TIMEOUT_MS: u64 = 8_000;
/// Retries for transient proxy pressure (429/503/504/timeout).
const MAX_STATUS_ATTEMPTS: u32 = 3;

fn last_good_snapshot() -> &'static Mutex<Option<NetworkSnapshot>> {
    static LAST: OnceLock<Mutex<Option<NetworkSnapshot>>> = OnceLock::new();
    LAST.get_or_init(|| Mutex::new(None))
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkSnapshot {
    pub online: bool,
    /// True when the gateway is reachable but a poll was throttled/stalled; UI keeps last-known data.
    #[serde(default)]
    pub degraded: bool,
    pub status_label: String,
    pub height: Option<u64>,
    pub block_count: u64,
    pub mempool_count: u64,
    pub mempool_transactions: Vec<Value>,
    pub mempool_anticipated_base_fee_atomic: String,
    pub mempool_total_fees_atomic: String,
    pub mempool_total_burned_fees_atomic: String,
    pub mempool_total_priority_fees_atomic: String,
    pub balance_atomic: Option<String>,
    pub emitted_supply_atomic: Option<String>,
    pub max_supply_atomic: Option<String>,
    pub tip_hash: Option<String>,
    pub indexed_height: Option<u64>,
    pub indexed_blocks: u64,
    pub indexed_transactions: u64,
    pub indexed_addresses: u64,
    pub latest_block_timestamp: Option<u64>,
    pub latest_block_transactions: u64,
    pub latest_block_reward_atomic: Option<String>,
    pub latest_block_fees_atomic: Option<String>,
    pub node_running: bool,
    pub rpc_running: bool,
    pub indexer_ready: bool,
    pub miner_running: bool,
    pub miner_hashrate_hs: Option<f64>,
    pub miner_height: Option<u64>,
    pub miner_accepted_blocks: Option<u64>,
    pub miner_accepted_shares: Option<u64>,
    pub miner_status: Option<String>,
    /// Last miner error / warm-up message from metrics.json (DAG build, template, CUDA).
    pub miner_last_error: Option<String>,
    pub miner_template_id: Option<String>,
    pub miner_difficulty_leading_zero_bits: Option<u64>,
    pub miner_share_difficulty_leading_zero_bits: Option<u64>,
    pub miner_eta_block_seconds: Option<f64>,
    pub miner_eta_share_seconds: Option<f64>,
    pub miner_hashes_attempted: Option<String>,
    pub miner_updated_at_unix_seconds: Option<u64>,
    pub miner_backend_mode: Option<String>,
    pub miner_active_backend: Option<String>,
    pub miner_gpu_devices: Option<u64>,
    pub local_peer_id: Option<String>,
    pub p2p_listen_addresses: Vec<String>,
    pub configured_seed_count: u64,
    pub connected_peer_count: u64,
    pub validated_peer_count: u64,
    pub mining_peer_count: u64,
    pub observed_network_hashrate_hs: f64,
    pub miners: Vec<Value>,
    pub validating_peer_count: u64,
    pub banned_peer_count: u64,
    pub reputation_enabled: bool,
    pub p2p_syncing: bool,
    pub p2p_error: Option<String>,
    pub sync_status: String,
    pub sync_target_height: Option<u64>,
    pub sync_remaining_blocks: Option<u64>,
    pub sync_progress_percent: Option<f64>,
    pub sync_target_peer_count: u64,
    pub recent_blocks: Vec<Value>,
    pub recent_transactions: Vec<Value>,
    pub peers: Vec<Value>,
    pub fleet_nodes: Vec<Value>,
    pub fleet_registered_nodes: u64,
    pub fleet_online_nodes: u64,
    pub pool_online: bool,
    pub pool_name: Option<String>,
    pub pool_workers: u64,
    pub pool_hashrate_hs: f64,
    pub pool_blocks_found: u64,
    pub pool_vardiff_target_seconds: Option<u64>,
    pub pool_rejected_requests: u64,
    pub pool_rate_limited_requests: u64,
    pub pool_active_bans: u64,
    pub detail: String,
}

fn client_for(remote: bool) -> AppResult<reqwest::Client> {
    // Separate clients so local stays snappy while remote tolerates VPS stalls.
    static LOCAL: OnceLock<reqwest::Client> = OnceLock::new();
    static REMOTE: OnceLock<reqwest::Client> = OnceLock::new();
    let slot = if remote { &REMOTE } else { &LOCAL };
    if let Some(c) = slot.get() {
        return Ok(c.clone());
    }
    let timeout_ms = if remote {
        REMOTE_TIMEOUT_MS
    } else {
        LOCAL_TIMEOUT_MS
    };
    let built = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .connect_timeout(Duration::from_secs(if remote { 10 } else { 3 }))
        .pool_max_idle_per_host(8)
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(30))
        .user_agent("Alvenqis-Control-Center/1.0")
        .build()
        .map_err(|error| {
            crate::error::AppError::msg(format!("HTTP client init failed: {error}"))
        })?;
    let _ = slot.set(built.clone());
    Ok(built)
}

fn is_retryable_status(code: reqwest::StatusCode) -> bool {
    matches!(code.as_u16(), 408 | 425 | 429 | 500 | 502 | 503 | 504)
}

async fn request_once(base: &str, endpoint: &str, remote: bool) -> AppResult<Value> {
    let url = format!("{}{}", base.trim_end_matches('/'), endpoint);
    let response = client_for(remote)?
        .get(&url)
        .header("X-Alvenqis-Client", "control-center")
        .send()
        .await
        .map_err(|error| crate::error::AppError::msg(format!("RPC request failed: {error}")))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let snippet: String = body.chars().take(120).collect();
        return Err(crate::error::AppError::msg(format!(
            "RPC returned {status}{}{}",
            if snippet.is_empty() { "" } else { ": " },
            snippet
        )));
    }
    response
        .json()
        .await
        .map_err(|error| crate::error::AppError::msg(format!("RPC JSON decode failed: {error}")))
}

/// GET with exponential backoff for transient proxy or gateway pressure.
async fn request(base: &str, endpoint: &str) -> AppResult<Value> {
    let remote = !is_local_rpc_url(base);
    let attempts = if remote { MAX_STATUS_ATTEMPTS } else { 1 };
    let mut last_err = None;
    for attempt in 0..attempts {
        match request_once(base, endpoint, remote).await {
            Ok(value) => return Ok(value),
            Err(err) => {
                let msg = err.to_string();
                let retryable = msg.contains("429")
                    || msg.contains("503")
                    || msg.contains("504")
                    || msg.contains("502")
                    || msg.contains("408")
                    || msg.contains("timed out")
                    || msg.contains("timeout")
                    || msg.contains("connection")
                    || msg.contains("request failed");
                last_err = Some(err);
                if !retryable || attempt + 1 >= attempts {
                    break;
                }
                let backoff_ms = 400u64.saturating_mul(1u64 << attempt.min(3));
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }
        }
    }
    Err(last_err.unwrap_or_else(|| crate::error::AppError::msg("RPC request failed")))
}

async fn optional(base: &str, endpoint: &str, fallback: Value) -> Value {
    request(base, endpoint).await.unwrap_or(fallback)
}

async fn health_ok(base: &str) -> bool {
    let remote = !is_local_rpc_url(base);
    let url = format!("{}/health", base.trim_end_matches('/'));
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(if remote {
            REMOTE_HEALTH_TIMEOUT_MS
        } else {
            3_000
        }))
        .connect_timeout(Duration::from_secs(5))
        .user_agent("Alvenqis-Control-Center/1.0")
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    match client
        .get(url)
        .header("X-Alvenqis-Client", "control-center")
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

fn atomic(value: &Value) -> String {
    if value.is_null() {
        "0".into()
    } else {
        value
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| value.to_string().trim_matches('"').to_string())
    }
}

fn optional_atomic(value: &Value) -> Option<String> {
    if value.is_null() {
        None
    } else {
        Some(atomic(value))
    }
}

fn map_block(value: &Value) -> Value {
    json!({
        "height": value.get("height").and_then(|v| v.as_u64()).unwrap_or(0),
        "hash": value.get("hash").and_then(|v| v.as_str()).unwrap_or(""),
        "previous_hash": value.get("previous_hash").and_then(|v| v.as_str()).unwrap_or(""),
        "merkle_root": value.get("merkle_root").and_then(|v| v.as_str()).unwrap_or(""),
        "timestamp": value.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(0),
        "nonce": value.get("nonce").and_then(|v| v.as_u64()).unwrap_or(0),
        "difficulty_leading_zero_bits": value.get("difficulty_leading_zero_bits").and_then(|v| v.as_u64()).unwrap_or(0),
        "transaction_count": value.get("transaction_count").and_then(|v| v.as_u64()).unwrap_or(0),
        "miner_address": value.get("miner_address").and_then(|v| v.as_str()).unwrap_or(""),
        "coinbase_payout_atomic": atomic(value.get("coinbase_payout_atomic").unwrap_or(&Value::Null)),
        "miner_reward_atomic": atomic(value.get("miner_reward_atomic").unwrap_or(&Value::Null)),
        "fees_atomic": atomic(value.get("fees_atomic").unwrap_or(&Value::Null)),
        "burned_fees_atomic": atomic(value.get("burned_fees_atomic").unwrap_or(&Value::Null)),
        "priority_fees_atomic": atomic(value.get("priority_fees_atomic").unwrap_or(&Value::Null)),
        "base_fee_atomic": atomic(value.get("base_fee_atomic").unwrap_or(&Value::Null)),
        "transaction_hashes": value.get("transaction_hashes").cloned().unwrap_or_else(|| json!([])),
    })
}

fn map_tx(value: &Value) -> Value {
    json!({
        "lifecycle_status": value.get("lifecycle_status").and_then(|v| v.as_str()).unwrap_or(""),
        "hash": value.get("hash").and_then(|v| v.as_str()).unwrap_or(""),
        "block_height": value.get("block_height").and_then(|v| v.as_u64()).unwrap_or(0),
        "block_hash": value.get("block_hash").and_then(|v| v.as_str()).unwrap_or(""),
        "transaction_index": value
            .get("transaction_index")
            .and_then(|v| v.as_u64())
            .or_else(|| value.get("transaction_index").and_then(|v| v.as_u64()))
            .unwrap_or(0),
        "nonce": value.get("nonce").and_then(|v| v.as_u64()).unwrap_or(0),
        "version": value.get("version").and_then(|v| v.as_u64()).unwrap_or(0),
        "from": value.get("from").cloned().unwrap_or(Value::Null),
        "to": value.get("to").and_then(|v| v.as_str()).unwrap_or(""),
        "amount_atomic": atomic(value.get("amount_atomic").unwrap_or(&Value::Null)),
        "effective_fee_atomic": atomic(
            value
                .get("effective_fee_atomic")
                .or_else(|| value.get("fee_atomic"))
                .unwrap_or(&Value::Null),
        ),
        "burned_fee_atomic": atomic(value.get("burned_fee_atomic").unwrap_or(&Value::Null)),
        "effective_priority_fee_atomic": atomic(
            value
                .get("effective_priority_fee_atomic")
                .or_else(|| value.get("priority_fee_atomic"))
                .unwrap_or(&Value::Null),
        ),
        "max_fee_atomic": atomic(value.get("max_fee_atomic").unwrap_or(&Value::Null)),
        "base_fee_atomic": atomic(value.get("base_fee_atomic").unwrap_or(&Value::Null)),
        "memo_hash": value.get("memo_hash").cloned().unwrap_or(Value::Null),
        "sender_public_key_hex": value
            .get("sender_public_key_hex")
            .cloned()
            .unwrap_or(Value::Null),
        "signature_hex": value.get("signature_hex").cloned().unwrap_or(Value::Null),
        "authorization_state": value.get("authorization_state").and_then(|v| v.as_str()).unwrap_or(""),
        "block_transaction_count": value
            .get("block_transaction_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    })
}

async fn p2p_status(base: &str) -> Value {
    // Always read P2P from the configured gateway. Never fall back to a
    // hidden local stack ΓÇö Control Center is VPS/gateway-first.
    request(base, "/p2p/status")
        .await
        .unwrap_or_else(|_| json!({}))
}

fn read_miner_metrics() -> Option<Value> {
    // Prefer the same runtime root used when starting the miner (packaged user data
    // vs monorepo .alvenqis-local). Fall back across both so hashrate never goes blank
    // due to a path mismatch between process.rs and snapshot reads.
    let mut candidates = Vec::new();
    if let Ok(workspace) = find_workspace_root() {
        candidates.push(local_root(&workspace).join("miner").join("metrics.json"));
        candidates.push(
            workspace
                .join(".alvenqis-local")
                .join("miner")
                .join("metrics.json"),
        );
    }
    if let Ok(app_data) = std::env::var("LOCALAPPDATA") {
        candidates.push(
            PathBuf::from(app_data)
                .join("Alvenqis")
                .join("ControlCenter")
                .join(".alvenqis-local")
                .join("miner")
                .join("metrics.json"),
        );
    }
    // Pick the freshest metrics file that parses.
    let mut best: Option<(u64, Value)> = None;
    for path in candidates {
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        if best.as_ref().map(|(t, _)| mtime >= *t).unwrap_or(true) {
            best = Some((mtime, value));
        }
    }
    best.map(|(_, v)| v)
}

fn empty_snapshot(node_running: bool, miner_running: bool, detail: String) -> NetworkSnapshot {
    NetworkSnapshot {
        online: false,
        degraded: false,
        status_label: "Mainnet Candidate".into(),
        height: None,
        block_count: 0,
        mempool_count: 0,
        mempool_transactions: vec![],
        mempool_anticipated_base_fee_atomic: "0".into(),
        mempool_total_fees_atomic: "0".into(),
        mempool_total_burned_fees_atomic: "0".into(),
        mempool_total_priority_fees_atomic: "0".into(),
        balance_atomic: None,
        emitted_supply_atomic: None,
        max_supply_atomic: None,
        tip_hash: None,
        indexed_height: None,
        indexed_blocks: 0,
        indexed_transactions: 0,
        indexed_addresses: 0,
        latest_block_timestamp: None,
        latest_block_transactions: 0,
        latest_block_reward_atomic: None,
        latest_block_fees_atomic: None,
        node_running,
        rpc_running: false,
        indexer_ready: false,
        miner_running,
        miner_hashrate_hs: None,
        miner_height: None,
        miner_accepted_blocks: None,
        miner_accepted_shares: None,
        miner_share_difficulty_leading_zero_bits: None,
        miner_eta_block_seconds: None,
        miner_eta_share_seconds: None,
        miner_status: None,
        miner_last_error: None,
        miner_template_id: None,
        miner_difficulty_leading_zero_bits: None,
        miner_hashes_attempted: None,
        miner_updated_at_unix_seconds: None,
        miner_backend_mode: None,
        miner_active_backend: None,
        miner_gpu_devices: None,
        local_peer_id: None,
        p2p_listen_addresses: vec![],
        configured_seed_count: 0,
        connected_peer_count: 0,
        validated_peer_count: 0,
        mining_peer_count: 0,
        observed_network_hashrate_hs: 0.0,
        miners: vec![],
        validating_peer_count: 0,
        banned_peer_count: 0,
        reputation_enabled: true,
        p2p_syncing: false,
        p2p_error: None,
        sync_status: "offline".into(),
        sync_target_height: None,
        sync_remaining_blocks: None,
        sync_progress_percent: None,
        sync_target_peer_count: 0,
        recent_blocks: vec![],
        recent_transactions: vec![],
        peers: vec![],
        fleet_nodes: vec![],
        fleet_registered_nodes: 0,
        fleet_online_nodes: 0,
        pool_online: false,
        pool_name: None,
        pool_workers: 0,
        pool_hashrate_hs: 0.0,
        pool_blocks_found: 0,
        pool_vardiff_target_seconds: None,
        pool_rejected_requests: 0,
        pool_rate_limited_requests: 0,
        pool_active_bans: 0,
        detail,
    }
}

pub async fn network_snapshot(wallet: Option<WalletMetadata>) -> NetworkSnapshot {
    let node_managed = managed_process_running("node").await;
    let miner_managed = managed_process_running("miner").await;
    let base = get_rpc_url();
    let remote_gateway = !is_local_rpc_url(&base);
    let status = match request(&base, "/status").await {
        Ok(value) => value,
        Err(err) => {
            // Transient VPS pressure: if /health still answers, keep last-known snapshot
            // as degraded instead of flipping the UI offline every few seconds.
            let gateway_alive = health_ok(&base).await;
            if gateway_alive {
                if let Some(mut cached) = last_good_snapshot().lock().clone() {
                    cached.degraded = true;
                    cached.online = true;
                    cached.rpc_running = true;
                    cached.detail = format!(
                        "{} RPC degraded ({base}): {err}. Gateway /health OK — using last-known tip.",
                        if remote_gateway { "VPS" } else { "Local" }
                    );
                    return cached;
                }
                // No cache yet: report online-but-thin so the UI does not hard-fail.
                let mut thin = empty_snapshot(
                    node_managed || remote_gateway,
                    miner_managed,
                    format!(
                        "{} RPC degraded ({base}): {err}. Gateway alive; waiting for /status.",
                        if remote_gateway { "VPS" } else { "Local" }
                    ),
                );
                thin.online = true;
                thin.degraded = true;
                thin.rpc_running = true;
                thin.sync_status = "degraded".into();
                return thin;
            }
            return empty_snapshot(
                node_managed,
                miner_managed,
                format!(
                    "{} RPC is offline ({base}): {err}",
                    if remote_gateway { "VPS" } else { "Local" }
                ),
            );
        }
    };

    // Fan-out RPC reads in parallel. Sequential calls were ~2s+ wall time and
    // stacked under load so the UI sat on zeros while the system looked busy.
    let balance_path = wallet
        .as_ref()
        .map(|w| format!("/addresses/{}/balance", urlencoding_lite(&w.address)));
    let (mempool, indexer, index_data, sync_gateway, p2p, fleet, pool, balance_json) = tokio::join!(
        optional(&base, "/mempool", json!({})),
        optional(&base, "/indexer/status", json!({})),
        optional(
            &base,
            "/indexer/overview?blocks=12&transactions=20",
            json!({}),
        ),
        optional(&base, "/sync/status", json!({})),
        p2p_status(&base),
        optional(&base, "/fleet/status", json!({})),
        optional(&base, "/pool/api/v1/pool/status", json!({})),
        async {
            if let Some(path) = balance_path.as_deref() {
                optional(&base, path, json!({})).await
            } else {
                json!({})
            }
        },
    );
    let metrics = read_miner_metrics();

    let balance = if wallet.is_some() {
        optional_atomic(balance_json.get("balance_atomic").unwrap_or(&Value::Null))
    } else {
        None
    };

    let mut recent_blocks: Vec<Value> = index_data
        .get("recent_blocks")
        .and_then(|value| value.as_array())
        .map(|items| items.iter().map(map_block).collect())
        .unwrap_or_default();
    recent_blocks.sort_by(|a, b| {
        b.get("height")
            .and_then(|v| v.as_u64())
            .cmp(&a.get("height").and_then(|v| v.as_u64()))
    });
    recent_blocks.truncate(12);

    let mut recent_transactions: Vec<Value> = index_data
        .get("recent_transactions")
        .and_then(|value| value.as_array())
        .map(|items| items.iter().map(map_tx).collect())
        .unwrap_or_default();
    recent_transactions.sort_by(|a, b| {
        let ah = a.get("block_height").and_then(|v| v.as_u64()).unwrap_or(0);
        let bh = b.get("block_height").and_then(|v| v.as_u64()).unwrap_or(0);
        let ai = a
            .get("transaction_index")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let bi = b
            .get("transaction_index")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        bh.cmp(&ah).then(bi.cmp(&ai))
    });
    recent_transactions.truncate(20);

    let mempool_transactions: Vec<Value> = mempool
        .get("transactions")
        .and_then(|v| v.as_array())
        .map(|items| items.iter().map(map_tx).collect())
        .unwrap_or_default();

    let peers: Vec<Value> = p2p
        .get("peers")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .map(|peer| {
                    json!({
                        "peer_id": peer.get("peer_id").and_then(|v| v.as_str()).unwrap_or(""),
                        "address": peer.get("address").cloned().unwrap_or(Value::Null),
                        "handshake_validated": peer.get("handshake_validated").and_then(|v| v.as_bool()).unwrap_or(false),
                        "best_height": peer.get("best_height").cloned().unwrap_or(Value::Null),
                        "validating": peer.get("validating").and_then(|v| v.as_bool()).unwrap_or(false),
                        "mining": peer.get("mining").and_then(|v| v.as_bool()).unwrap_or(false),
                        "hashrate_hs": peer.get("hashrate_hs").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        "last_error": peer.get("last_error").cloned().unwrap_or(Value::Null),
                        "reputation_score": peer.get("reputation_score").and_then(|v| v.as_i64()).unwrap_or(50),
                        "banned": peer.get("banned").and_then(|v| v.as_bool()).unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let miners = p2p
        .get("miners")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let fleet_nodes = fleet
        .get("nodes")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let miner_updated = metrics
        .as_ref()
        .and_then(|m| m.get("updated_at_unix_seconds"))
        .and_then(|v| v.as_u64());
    let miner_status_raw = metrics
        .as_ref()
        .and_then(|m| m.get("status"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    // Explicit "stopped" must never keep the UI in mining state after stop.
    // "building_dag" / "starting" / "waiting_work" are active warm-up states.
    let miner_metrics_active = !matches!(miner_status_raw, "stopped" | "");
    // First epoch DAG build can exceed a minute on host export; keep a wide
    // freshness window so the UI does not flip to stopped mid warm-up.
    let miner_fresh = miner_metrics_active
        && miner_updated
            .map(|ts| now.saturating_sub(ts) <= 180)
            .unwrap_or(false);
    let p2p_updated = p2p
        .get("updated_at_unix_seconds")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let p2p_fresh = p2p_updated > 0 && now.saturating_sub(p2p_updated) <= 30;
    let mode = p2p
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let p2p_active = p2p_fresh && !mode.contains("stopped");

    let local_height = status.get("height").and_then(|v| v.as_u64());
    let validated_heights: Vec<u64> = peers
        .iter()
        .filter(|peer| {
            peer.get("handshake_validated")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                && peer.get("best_height").and_then(|v| v.as_u64()).is_some()
        })
        .filter_map(|peer| peer.get("best_height").and_then(|v| v.as_u64()))
        .collect();

    // Prefer gateway /sync/status when available.
    let gateway_sync_state = sync_gateway
        .get("sync_state")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let gateway_network_height = sync_gateway.get("network_height").and_then(|v| v.as_u64());
    let gateway_remaining = sync_gateway
        .get("remaining_blocks")
        .and_then(|v| v.as_u64());
    let gateway_progress = sync_gateway
        .get("progress_percent")
        .and_then(|v| v.as_f64());

    let peer_target = if !validated_heights.is_empty() {
        local_height.map(|h| validated_heights.iter().copied().fold(h, u64::max))
    } else {
        None
    };
    let sync_target_height = gateway_network_height.or(peer_target).or(local_height);
    let sync_remaining_blocks = if let Some(remaining) = gateway_remaining {
        Some(remaining)
    } else {
        match (sync_target_height, local_height) {
            (Some(target), Some(local)) => Some(target.saturating_sub(local)),
            _ => None,
        }
    };
    let sync_progress_percent = if let Some(progress) = gateway_progress {
        Some(progress)
    } else {
        match (sync_target_height, local_height) {
            (Some(0), Some(_)) => Some(100.0),
            (Some(target), Some(local)) => {
                Some(((local as f64 / target as f64) * 100.0).min(100.0))
            }
            _ => None,
        }
    };

    // Remote-gateway clients observe the VPS chain tip. When /status reports a
    // tip (including height 0 genesis), the desktop is synced to the gateway.
    let block_count = status
        .get("block_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let gateway_chain_ready = remote_gateway
        && (local_height.is_some() || block_count > 0)
        && status
            .get("initialized")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
    let p2p_syncing_flag = p2p
        .get("syncing")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    // Only treat remaining>0 as syncing when we also have an explicit syncing signal
    // or peer lag - avoid stuck "syncing" when the candidate chain is still at genesis.
    let peer_lag = match (sync_target_height, local_height) {
        (Some(target), Some(local)) => target > local,
        _ => false,
    };
    let actively_syncing = gateway_sync_state == "syncing"
        || p2p_syncing_flag
        || (peer_lag && sync_remaining_blocks.unwrap_or(0) > 0);
    let sync_status = if gateway_chain_ready {
        if actively_syncing {
            "syncing"
        } else {
            "synced"
        }
    } else if !p2p_active && !remote_gateway {
        "offline"
    } else if validated_heights.is_empty() && !gateway_chain_ready {
        if local_height.is_some() && !remote_gateway {
            "discovering"
        } else if local_height.is_some() {
            "synced"
        } else {
            "discovering"
        }
    } else if actively_syncing {
        "syncing"
    } else {
        "synced"
    };

    let index_summary = index_data.get("summary").cloned().unwrap_or(json!({}));
    let latest = recent_blocks.first().cloned().unwrap_or_else(|| json!({}));
    let index_supply = index_summary.get("supply").cloned().unwrap_or(json!({}));

    let host = url::Url::parse(&base)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_else(|| base.clone());

    let snapshot = NetworkSnapshot {
        online: true,
        degraded: false,
        status_label: status
            .get("status_label")
            .and_then(|v| v.as_str())
            .unwrap_or("Mainnet Candidate")
            .to_string(),
        height: local_height,
        block_count: status
            .get("block_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        mempool_count: mempool
            .get("pending_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(mempool_transactions.len() as u64),
        mempool_transactions,
        mempool_anticipated_base_fee_atomic: atomic(
            mempool
                .get("anticipated_base_fee_atomic")
                .unwrap_or(&Value::Null),
        ),
        mempool_total_fees_atomic: atomic(mempool.get("total_fees_atomic").unwrap_or(&Value::Null)),
        mempool_total_burned_fees_atomic: atomic(
            mempool
                .get("total_burned_fees_atomic")
                .unwrap_or(&Value::Null),
        ),
        mempool_total_priority_fees_atomic: atomic(
            mempool
                .get("total_priority_fees_atomic")
                .unwrap_or(&Value::Null),
        ),
        balance_atomic: balance,
        emitted_supply_atomic: optional_atomic(
            status
                .get("emitted_supply_atomic")
                .or_else(|| index_supply.get("emitted_supply_atomic"))
                .unwrap_or(&Value::Null),
        ),
        max_supply_atomic: optional_atomic(
            index_supply
                .get("max_supply_atomic")
                .unwrap_or(&Value::Null),
        ),
        tip_hash: status
            .get("tip_hash")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        indexed_height: indexer.get("indexed_height").and_then(|v| v.as_u64()),
        indexed_blocks: indexer
            .get("indexed_block_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        indexed_transactions: index_summary
            .get("transaction_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        indexed_addresses: index_summary
            .get("address_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        latest_block_timestamp: latest.get("timestamp").and_then(|v| v.as_u64()),
        latest_block_transactions: latest
            .get("transaction_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        latest_block_reward_atomic: optional_atomic(
            latest.get("miner_reward_atomic").unwrap_or(&Value::Null),
        ),
        latest_block_fees_atomic: optional_atomic(
            latest.get("fees_atomic").unwrap_or(&Value::Null),
        ),
        // For VPS gateway mode, the remote node is "running" when the gateway
        // reports a peer id or a known chain height - not when a local process
        // is managed on this PC.
        node_running: node_managed
            || (p2p
                .get("local_peer_id")
                .and_then(|v| v.as_str())
                .is_some_and(|s| !s.is_empty()))
            || (remote_gateway && local_height.is_some()),
        rpc_running: true,
        indexer_ready: indexer
            .get("initialized")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        miner_running: miner_managed || miner_fresh,
        miner_hashrate_hs: {
            let running = miner_managed || miner_fresh;
            if !running {
                Some(0.0)
            } else {
                metrics
                    .as_ref()
                    .and_then(|m| m.get("hashrate_hs"))
                    .and_then(|v| v.as_f64())
            }
        },
        miner_height: metrics
            .as_ref()
            .and_then(|m| m.get("height"))
            .and_then(|v| v.as_u64()),
        miner_accepted_blocks: metrics
            .as_ref()
            .and_then(|m| m.get("accepted_blocks"))
            .and_then(|v| v.as_u64()),
        miner_accepted_shares: metrics
            .as_ref()
            .and_then(|m| m.get("accepted_shares"))
            .and_then(|v| v.as_u64()),
        miner_status: {
            let running = miner_managed || miner_fresh;
            if !running {
                Some("stopped".to_string())
            } else {
                metrics
                    .as_ref()
                    .and_then(|m| m.get("status"))
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            }
        },
        miner_last_error: metrics
            .as_ref()
            .and_then(|m| m.get("last_error"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        miner_template_id: metrics
            .as_ref()
            .and_then(|m| m.get("template_id"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        miner_difficulty_leading_zero_bits: metrics
            .as_ref()
            .and_then(|m| m.get("difficulty_leading_zero_bits"))
            .and_then(|v| v.as_u64()),
        miner_share_difficulty_leading_zero_bits: metrics
            .as_ref()
            .and_then(|m| m.get("share_difficulty_leading_zero_bits"))
            .and_then(|v| v.as_u64()),
        miner_eta_block_seconds: metrics
            .as_ref()
            .and_then(|m| m.get("eta_block_seconds"))
            .and_then(|v| v.as_f64()),
        miner_eta_share_seconds: metrics
            .as_ref()
            .and_then(|m| m.get("eta_share_seconds"))
            .and_then(|v| v.as_f64()),
        miner_hashes_attempted: metrics
            .as_ref()
            .and_then(|m| m.get("hashes_attempted"))
            .map(|v| {
                v.as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| v.to_string())
            }),
        miner_updated_at_unix_seconds: miner_updated,
        miner_backend_mode: metrics
            .as_ref()
            .and_then(|m| m.get("backend_mode"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        miner_active_backend: metrics
            .as_ref()
            .and_then(|m| m.get("active_backend"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        miner_gpu_devices: metrics
            .as_ref()
            .and_then(|m| m.get("gpu_devices"))
            .and_then(|v| v.as_u64()),
        local_peer_id: p2p
            .get("local_peer_id")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        p2p_listen_addresses: p2p
            .get("listen_addresses")
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        configured_seed_count: p2p
            .get("configured_seed_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        connected_peer_count: p2p
            .get("connected_peer_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        validated_peer_count: p2p
            .get("validated_peer_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        mining_peer_count: {
            // P2P presence alone misses desktop RPC/pool miners. Fold in local + pool workers.
            let p2p_miners = p2p
                .get("mining_peer_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let pool_workers = pool
                .get("connected_workers")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let local_active = if miner_managed || miner_fresh { 1 } else { 0 };
            // Prefer max so we do not invent double-counts when the same host is in P2P + pool.
            p2p_miners
                .max(pool_workers.saturating_add(local_active))
                .max(local_active)
        },
        observed_network_hashrate_hs: {
            let p2p_hs = p2p
                .get("observed_network_hashrate_hs")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let pool_hs = pool
                .get("estimated_hashrate_hs")
                .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|n| n as f64)))
                .unwrap_or(0.0);
            let local_hs = if miner_managed || miner_fresh {
                metrics
                    .as_ref()
                    .and_then(|m| m.get("hashrate_hs"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            let fleet_hs = fleet_nodes
                .iter()
                .filter_map(|node| {
                    node.get("observed_hashrate_hs")
                        .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|n| n as f64)))
                })
                .fold(0.0_f64, |a, b| a + b);
            // Aggregate observed sources without inventing peers. Take the strongest signal
            // that is coherent (p2p often empty when miners are RPC/pool-only).
            p2p_hs
                .max(pool_hs + local_hs)
                .max(fleet_hs)
                .max(local_hs)
                .max(pool_hs)
        },
        miners,
        validating_peer_count: p2p
            .get("validating_peer_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        banned_peer_count: p2p
            .get("banned_peer_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        reputation_enabled: p2p
            .get("reputation_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        p2p_syncing: p2p
            .get("syncing")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        p2p_error: p2p
            .get("last_error")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        sync_status: sync_status.into(),
        sync_target_height,
        sync_remaining_blocks,
        sync_progress_percent,
        sync_target_peer_count: validated_heights.len() as u64,
        recent_blocks,
        recent_transactions,
        peers,
        fleet_nodes,
        fleet_registered_nodes: fleet
            .get("registered_node_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        fleet_online_nodes: fleet
            .get("online_node_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        pool_online: pool.get("protocol").is_some(),
        pool_name: pool
            .get("pool_name")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        pool_workers: pool
            .get("connected_workers")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        pool_hashrate_hs: pool
            .get("estimated_hashrate_hs")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        pool_blocks_found: pool
            .get("blocks_found")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        pool_vardiff_target_seconds: if pool
            .get("vardiff_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            pool.get("target_share_seconds").and_then(|v| v.as_u64())
        } else {
            None
        },
        pool_rejected_requests: pool
            .get("rejected_requests")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        pool_rate_limited_requests: pool
            .get("rate_limited_requests")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        pool_active_bans: pool
            .get("active_bans")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        detail: if remote_gateway {
            format!("VPS gateway verified at {host}")
        } else {
            format!("Local RPC verified at {host}")
        },
    };
    *last_good_snapshot().lock() = Some(snapshot.clone());
    snapshot
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

/// Classify a free-form explorer query into a safe lookup kind.
fn classify_explorer_query(raw: &str) -> (&'static str, String) {
    let q = raw.trim();
    if q.is_empty() {
        return ("empty", String::new());
    }
    if q.chars().all(|c| c.is_ascii_digit()) && q.len() <= 18 {
        return ("height", q.to_string());
    }
    if q.to_ascii_lowercase().starts_with("alve1") && q.len() >= 20 {
        return ("address", q.to_string());
    }
    let hex = q.strip_prefix("0x").unwrap_or(q);
    if hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return ("hash", hex.to_ascii_lowercase());
    }
    // libp2p peer ids commonly start with 12D3KooW / Qm / or appear as /p2p/<id>
    let peerish = q
        .trim_start_matches("/p2p/")
        .trim_start_matches("p2p/")
        .to_string();
    if peerish.starts_with("12D3KooW")
        || peerish.starts_with("Qm")
        || peerish.starts_with("12D3")
        || q.contains("/p2p/")
        || (peerish.len() >= 32
            && peerish
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '=' || c == '/' || c == '+'))
    {
        return ("peer_id", peerish);
    }
    // Worker names / loose pool tokens
    if q.len() <= 64 && !q.contains(' ') {
        return ("worker_or_partial", q.to_string());
    }
    ("unknown", q.to_string())
}

/// Safe in-app chain lookup: public RPC/indexer/pool/P2P data only (no secrets).
pub async fn explorer_lookup(query: &str) -> AppResult<Value> {
    let base = get_rpc_url();
    let (kind, normalized) = classify_explorer_query(query);
    if kind == "empty" {
        return Ok(json!({
            "kind": "not_found",
            "query": query,
            "query_kind": "empty",
            "message": "Enter a block height, tx/block hash, alve1 address, peer id, or pool worker name.",
            "data": null,
            "sources": [],
        }));
    }

    let mut sources: Vec<&str> = Vec::new();
    let mut notes: Vec<String> = Vec::new();

    match kind {
        "height" => {
            let height: u64 = normalized.parse().unwrap_or(0);
            if let Ok(block) = request(&base, &format!("/indexer/blocks/{height}")).await {
                sources.push("indexer");
                return Ok(json!({
                    "kind": "block",
                    "query": query,
                    "query_kind": "height",
                    "data": map_block(&block),
                    "raw": block,
                    "sources": sources,
                    "notes": notes,
                }));
            }
            if let Ok(block) = request(&base, &format!("/blocks/{height}")).await {
                sources.push("rpc");
                return Ok(json!({
                    "kind": "block",
                    "query": query,
                    "query_kind": "height",
                    "data": map_block(&block),
                    "raw": block,
                    "sources": sources,
                    "notes": notes,
                }));
            }
            Ok(json!({
                "kind": "not_found",
                "query": query,
                "query_kind": "height",
                "message": format!("No block found at height {height} on the configured gateway."),
                "data": null,
                "sources": sources,
            }))
        }
        "hash" => {
            if let Ok(tx) = request(&base, &format!("/indexer/tx/{normalized}")).await {
                sources.push("indexer");
                return Ok(json!({
                    "kind": "transaction",
                    "query": query,
                    "query_kind": "hash",
                    "data": map_tx(&tx),
                    "raw": tx,
                    "sources": sources,
                    "notes": notes,
                }));
            }
            if let Ok(tx) = request(&base, &format!("/transactions/{normalized}")).await {
                sources.push("rpc");
                return Ok(json!({
                    "kind": "transaction",
                    "query": query,
                    "query_kind": "hash",
                    "data": map_tx(&tx),
                    "raw": tx,
                    "sources": sources,
                    "notes": notes,
                }));
            }
            if let Ok(block) = request(&base, &format!("/blocks/hash/{normalized}")).await {
                sources.push("rpc");
                return Ok(json!({
                    "kind": "block",
                    "query": query,
                    "query_kind": "hash",
                    "data": map_block(&block),
                    "raw": block,
                    "sources": sources,
                    "notes": notes,
                }));
            }
            Ok(json!({
                "kind": "not_found",
                "query": query,
                "query_kind": "hash",
                "message": "No transaction or block matched this 64-char hash on the gateway.",
                "data": null,
                "sources": sources,
            }))
        }
        "address" => {
            let enc = urlencoding_lite(&normalized);
            let mut data = json!({ "address": normalized });
            if let Ok(activity) = request(&base, &format!("/indexer/address/{enc}")).await {
                sources.push("indexer");
                if let Some(obj) = activity.as_object() {
                    for (k, v) in obj {
                        data[k] = v.clone();
                    }
                }
            } else {
                notes.push(
                    "Indexer address activity unavailable; trying chain account endpoints.".into(),
                );
            }
            if let Ok(account) = request(&base, &format!("/addresses/{enc}/account")).await {
                sources.push("rpc-account");
                if let Some(obj) = account.as_object() {
                    for (k, v) in obj {
                        if data.get(k).is_none() || data.get(k) == Some(&Value::Null) {
                            data[k] = v.clone();
                        }
                    }
                    // Normalize atomic fields that may be numbers.
                    if let Some(bal) = account.get("balance_atomic") {
                        data["balance_atomic"] = json!(atomic(bal));
                    }
                    if let Some(fee) = account.get("anticipated_base_fee_atomic") {
                        data["anticipated_base_fee_atomic"] = json!(atomic(fee));
                    }
                }
            } else if let Ok(balance) = request(&base, &format!("/addresses/{enc}/balance")).await {
                sources.push("rpc-balance");
                if let Some(bal) = balance.get("balance_atomic") {
                    data["balance_atomic"] = json!(atomic(bal));
                }
                data["exists"] = json!(true);
            }

            // Pool workers bound to this payout address (public pool status).
            let pool = optional(&base, "/pool/api/v1/pool/status", json!({})).await;
            if pool.get("pool_name").is_some() || pool.get("connected_workers").is_some() {
                sources.push("pool");
                let workers = pool
                    .get("workers")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|w| {
                        w.get("miner_address")
                            .and_then(|v| v.as_str())
                            .map(|a| a.eq_ignore_ascii_case(&normalized))
                            .unwrap_or(false)
                    })
                    .collect::<Vec<_>>();
                data["pool_workers"] = json!(workers);
                data["pool_name"] = pool.get("pool_name").cloned().unwrap_or(Value::Null);
                data["pool_address"] = pool.get("pool_address").cloned().unwrap_or(Value::Null);
                data["pool_online"] = json!(pool.get("connected_workers").is_some());
            }

            let found = data.get("balance_atomic").is_some()
                || data.get("exists_in_ledger") == Some(&json!(true))
                || data.get("exists") == Some(&json!(true))
                || data
                    .get("transaction_hashes")
                    .and_then(|v| v.as_array())
                    .map(|a| !a.is_empty())
                    .unwrap_or(false)
                || data
                    .get("pool_workers")
                    .and_then(|v| v.as_array())
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);

            if !found {
                notes.push(
                    "Address not present in ledger yet (zero history is still a valid result)."
                        .into(),
                );
            }
            data["balance_atomic"] =
                json!(atomic(data.get("balance_atomic").unwrap_or(&json!("0"))));
            data["total_received_atomic"] = json!(atomic(
                data.get("total_received_atomic").unwrap_or(&json!("0"))
            ));
            data["total_sent_atomic"] =
                json!(atomic(data.get("total_sent_atomic").unwrap_or(&json!("0"))));
            data["mined_reward_atomic"] = json!(atomic(
                data.get("mined_reward_atomic").unwrap_or(&json!("0"))
            ));

            Ok(json!({
                "kind": "address",
                "query": query,
                "query_kind": "address",
                "data": data,
                "sources": sources,
                "notes": notes,
            }))
        }
        "peer_id" | "worker_or_partial" | "unknown" => {
            // Peers from P2P status
            let p2p = p2p_status(&base).await;
            if p2p.get("local_peer_id").is_some() || p2p.get("peers").is_some() {
                sources.push("p2p");
            }
            let needle = normalized.to_ascii_lowercase();
            let local_id = p2p
                .get("local_peer_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !local_id.is_empty() && local_id.to_ascii_lowercase().contains(&needle) {
                return Ok(json!({
                    "kind": "peer",
                    "query": query,
                    "query_kind": "peer_id",
                    "data": {
                        "peer_id": local_id,
                        "is_local": true,
                        "handshake_validated": true,
                        "validating": true,
                        "mining": false,
                        "listen_addresses": p2p.get("listen_addresses").cloned().unwrap_or(json!([])),
                        "best_height": null,
                        "hashrate_hs": 0,
                    },
                    "sources": sources,
                    "notes": notes,
                }));
            }
            if let Some(peers) = p2p.get("peers").and_then(|v| v.as_array()) {
                for peer in peers {
                    let pid = peer.get("peer_id").and_then(|v| v.as_str()).unwrap_or("");
                    let addr = peer.get("address").and_then(|v| v.as_str()).unwrap_or("");
                    if pid.to_ascii_lowercase().contains(&needle)
                        || addr.to_ascii_lowercase().contains(&needle)
                    {
                        return Ok(json!({
                            "kind": "peer",
                            "query": query,
                            "query_kind": "peer_id",
                            "data": peer,
                            "sources": sources,
                            "notes": notes,
                        }));
                    }
                }
            }
            if let Some(miners) = p2p.get("miners").and_then(|v| v.as_array()) {
                for miner in miners {
                    let pid = miner.get("peer_id").and_then(|v| v.as_str()).unwrap_or("");
                    if pid.to_ascii_lowercase().contains(&needle) {
                        let mut data = miner.clone();
                        if let Some(obj) = data.as_object_mut() {
                            obj.insert("mining".into(), json!(true));
                        }
                        return Ok(json!({
                            "kind": "peer",
                            "query": query,
                            "query_kind": "peer_id",
                            "data": data,
                            "sources": sources,
                            "notes": notes,
                        }));
                    }
                }
            }

            // Pool workers by worker name or miner address substring
            let pool = optional(&base, "/pool/api/v1/pool/status", json!({})).await;
            if let Some(workers) = pool.get("workers").and_then(|v| v.as_array()) {
                sources.push("pool");
                for worker in workers {
                    let name = worker
                        .get("worker_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let addr = worker
                        .get("miner_address")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if name.to_ascii_lowercase().contains(&needle)
                        || addr.to_ascii_lowercase().contains(&needle)
                    {
                        let mut data = worker.clone();
                        if let Some(obj) = data.as_object_mut() {
                            obj.insert(
                                "pool_name".into(),
                                pool.get("pool_name").cloned().unwrap_or(Value::Null),
                            );
                            obj.insert(
                                "pool_address".into(),
                                pool.get("pool_address").cloned().unwrap_or(Value::Null),
                            );
                        }
                        return Ok(json!({
                            "kind": "pool_worker",
                            "query": query,
                            "query_kind": "worker",
                            "data": data,
                            "sources": sources,
                            "notes": notes,
                        }));
                    }
                }
                // Exact pool_address match
                if let Some(pool_addr) = pool.get("pool_address").and_then(|v| v.as_str()) {
                    if pool_addr.to_ascii_lowercase().contains(&needle)
                        || needle.contains(&pool_addr.to_ascii_lowercase())
                    {
                        return Ok(json!({
                            "kind": "pool",
                            "query": query,
                            "query_kind": "pool",
                            "data": {
                                "pool_name": pool.get("pool_name"),
                                "pool_address": pool_addr,
                                "connected_workers": pool.get("connected_workers"),
                                "estimated_hashrate_hs": pool.get("estimated_hashrate_hs"),
                                "blocks_found": pool.get("blocks_found"),
                                "accepted_shares": pool.get("accepted_shares"),
                                "payout_scheme": pool.get("payout_scheme"),
                                "upstream_status": pool.get("upstream_status"),
                                "status_label": pool.get("status_label"),
                            },
                            "sources": sources,
                            "notes": notes,
                        }));
                    }
                }
            }

            Ok(json!({
                "kind": "not_found",
                "query": query,
                "query_kind": kind,
                "message": "No peer, pool worker, address, or chain object matched this query on the public gateway views.",
                "data": null,
                "sources": sources,
                "notes": notes,
            }))
        }
        _ => Ok(json!({
            "kind": "not_found",
            "query": query,
            "query_kind": kind,
            "message": "Unrecognized query.",
            "data": null,
            "sources": [],
        })),
    }
}
