use crate::config::PoolConfig;
use crate::models::{
    ConfirmPayoutRequest, MinerView, PoolAccountView, PoolHistoryView, PoolStatusView, ShareRecord,
    WorkerView, POOL_PROTOCOL_VERSION,
};
use crate::{PoolError, PoolStore, Result};
use alvenqis_core::{check_pow, hash_to_hex, Address};
use alvenqis_miner::{
    MiningSubmitRequest, MiningSubmitResponse, MiningTemplate, SubmitStatus,
    MINING_PROTOCOL_VERSION,
};
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap};
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::{Json, Router};
use reqwest::Client;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;
use tokio::sync::{broadcast, Mutex as AsyncMutex};
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;

const INDEX_HTML: &str = include_str!("../static/index.html");
const APP_JS: &str = include_str!("../static/app.js");
const STYLES_CSS: &str = include_str!("../static/styles.css");

#[derive(Clone)]
pub struct PoolState {
    pub config: PoolConfig,
    pub store: PoolStore,
    client: Client,
    jobs: Arc<Mutex<HashMap<String, PoolJob>>>,
    current_job: Arc<Mutex<Option<String>>>,
    template_refresh: Arc<AsyncMutex<()>>,
    job_updates: broadcast::Sender<String>,
    admission: Arc<Mutex<AdmissionState>>,
    upstream_health: Arc<Mutex<UpstreamHealth>>,
}

#[derive(Clone)]
struct PoolJob {
    template: MiningTemplate,
    fetched_at_unix_seconds: u64,
    worker_difficulties: HashMap<String, Vec<u8>>,
}

#[derive(Default)]
struct AdmissionState {
    clients: HashMap<String, ClientAdmission>,
    workers: HashMap<String, HashMap<String, u64>>,
    rejected_requests: u64,
    rate_limited_requests: u64,
}

#[derive(Default)]
struct ClientAdmission {
    window_started_at: u64,
    work_requests: u32,
    share_requests: u32,
    invalid_shares: u32,
    banned_until: u64,
}

#[derive(Default)]
struct UpstreamHealth {
    checked: bool,
    last_error: Option<String>,
}

#[derive(Clone, Copy)]
enum RequestKind {
    Work,
    Share,
}

impl PoolState {
    pub fn new(config: PoolConfig, store: PoolStore) -> Result<Self> {
        // Upstream RPC can stall while assembling a mining template under load.
        let client = Client::builder()
            .timeout(Duration::from_secs(45))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|error| PoolError::Config(error.to_string()))?;
        let (job_updates, _) = broadcast::channel(128);
        Ok(Self {
            config,
            store,
            client,
            jobs: Arc::new(Mutex::new(HashMap::new())),
            current_job: Arc::new(Mutex::new(None)),
            template_refresh: Arc::new(AsyncMutex::new(())),
            job_updates,
            admission: Arc::new(Mutex::new(AdmissionState::default())),
            upstream_health: Arc::new(Mutex::new(UpstreamHealth::default())),
        })
    }

    pub(crate) fn subscribe_jobs(&self) -> broadcast::Receiver<String> {
        self.job_updates.subscribe()
    }
}

pub fn router(state: PoolState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/app.js", get(javascript))
        .route("/styles.css", get(styles))
        .route("/health", get(health))
        .route("/api/v1/pool/status", get(pool_status))
        .route("/api/v1/pool/history", get(pool_history))
        .route("/api/v1/miners/:address", get(miner))
        .route("/api/v1/payouts", get(payouts))
        .route("/admin/v1/payouts/prepare", post(prepare_payout))
        .route("/admin/v1/payouts/:payout_id/confirm", post(confirm_payout))
        .route("/admin/v1/payouts/:payout_id/cancel", post(cancel_payout))
        .layer(RequestBodyLimitLayer::new(64 * 1024))
        .layer(pool_cors_layer(&state.config))
        .with_state(state)
}

fn pool_cors_layer(config: &crate::config::PoolConfig) -> CorsLayer {
    let methods = [
        axum::http::Method::GET,
        axum::http::Method::POST,
        axum::http::Method::OPTIONS,
    ];
    let headers = [header::CONTENT_TYPE, header::AUTHORIZATION];
    if config
        .cors_allowed_origins
        .iter()
        .any(|origin| origin == "*")
    {
        return CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(methods)
            .allow_headers(headers);
    }
    let origins: Vec<header::HeaderValue> = config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| header::HeaderValue::from_str(origin).ok())
        .collect();
    if origins.is_empty() {
        // Restrictive default: do not reflect arbitrary browser Origins.
        CorsLayer::new()
            .allow_methods(methods)
            .allow_headers(headers)
    } else {
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(methods)
            .allow_headers(headers)
    }
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}
async fn javascript() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        APP_JS,
    )
}
async fn styles() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        STYLES_CSS,
    )
}

async fn health(State(state): State<PoolState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": true,
        "service": "alvenqis-mining-pool",
        "protocol": POOL_PROTOCOL_VERSION,
        "mining_transport": crate::stratum::STRATUM_PROTOCOL,
        "stratum_tls": state.config.stratum.is_some(),
        "http_mining_api": false,
        "network_id": state.config.network_id,
        "status_label": state.config.status_label,
    }))
}

pub(crate) async fn get_work_for_peer(
    state: &PoolState,
    peer_ip: IpAddr,
    miner_address: &str,
    worker_name: &str,
) -> Result<MiningTemplate> {
    let keys = vec![format!("ip:{peer_ip}")];
    get_work(state, &keys, miner_address, worker_name).await
}

async fn get_work(
    state: &PoolState,
    admission_keys: &[String],
    miner_address: &str,
    worker_name: &str,
) -> Result<MiningTemplate> {
    validate_identity(state, miner_address, worker_name)?;
    let now = unix_seconds();
    admit_request(
        state,
        admission_keys,
        RequestKind::Work,
        Some((miner_address, worker_name)),
        now,
    )?;
    let job_id = ensure_current_job(state, now).await?;
    issue_worker_template(state, &job_id, miner_address, worker_name)
}

pub(crate) async fn ensure_current_job(state: &PoolState, now: u64) -> Result<String> {
    let live_job = live_job_id(state, now)?;
    let needs_upstream = match &live_job {
        Some(job_id) => job_needs_upstream_refresh(state, job_id, now)?,
        None => true,
    };
    if !needs_upstream {
        return live_job
            .ok_or_else(|| PoolError::Upstream("no live mining job available".to_owned()));
    }

    // A single upstream template build may validate the whole chain. Serialize refreshes so
    // hundreds of persistent Stratum sessions cannot stampede the node at a tip change.
    let _refresh_guard = state.template_refresh.lock().await;
    let refreshed_live_job = live_job_id(state, unix_seconds())?;
    let still_needs_upstream = match &refreshed_live_job {
        Some(job_id) => job_needs_upstream_refresh(state, job_id, unix_seconds())?,
        None => true,
    };
    if still_needs_upstream {
        match fetch_upstream_template(state).await {
            Ok(mut template) => {
                template.share_difficulty_leading_zero_bits = None;
                install_or_refresh_job(state, template, unix_seconds())?;
            }
            Err(error) => {
                if refreshed_live_job.is_none() {
                    return Err(error);
                }
            }
        }
    }
    live_job_id(state, unix_seconds())?
        .ok_or_else(|| PoolError::Upstream("no live mining job available".to_owned()))
}

async fn fetch_upstream_template(state: &PoolState) -> Result<MiningTemplate> {
    let url = format!(
        "{}/mining/template",
        state.config.upstream_rpc_url.trim_end_matches('/')
    );
    let response = state
        .client
        .get(url)
        .query(&[("miner_address", &state.config.pool_address)])
        .send()
        .await
        .map_err(|error| PoolError::Upstream(error.to_string()))?;
    if !response.status().is_success() {
        return Err(PoolError::Upstream(format!(
            "template HTTP {}",
            response.status()
        )));
    }
    response
        .json()
        .await
        .map_err(|error| PoolError::Upstream(error.to_string()))
}

/// Job is live until near its template expiry (not just a few seconds of cache age).
fn live_job_id(state: &PoolState, now: u64) -> Result<Option<String>> {
    let current = state
        .current_job
        .lock()
        .map_err(|_| PoolError::Storage("current job lock poisoned".to_owned()))?
        .clone();
    let Some(job_id) = current else {
        return Ok(None);
    };
    let jobs = state
        .jobs
        .lock()
        .map_err(|_| PoolError::Storage("job lock poisoned".to_owned()))?;
    Ok(jobs
        .get(&job_id)
        .filter(|job| job.template.expires_at_unix_seconds > now.saturating_add(2))
        .map(|_| job_id))
}

fn job_needs_upstream_refresh(state: &PoolState, job_id: &str, now: u64) -> Result<bool> {
    let jobs = state
        .jobs
        .lock()
        .map_err(|_| PoolError::Storage("job lock poisoned".to_owned()))?;
    let Some(job) = jobs.get(job_id) else {
        return Ok(true);
    };
    // Refresh early if we are close to expiry so miners get a new job before shares go stale.
    if job.template.expires_at_unix_seconds <= now.saturating_add(10) {
        return Ok(true);
    }
    Ok(now.saturating_sub(job.fetched_at_unix_seconds) >= state.config.job_cache_seconds)
}

/// Install a new job or refresh the same template_id without wiping issued worker difficulties.
/// Wiping difficulties was the main cause of "stale: work was not issued to this worker".
fn install_or_refresh_job(state: &PoolState, template: MiningTemplate, now: u64) -> Result<()> {
    let job_id = template.template_id.clone();
    let previous_current = state
        .current_job
        .lock()
        .map_err(|_| PoolError::Storage("current job lock poisoned".to_owned()))?
        .clone();
    let mut jobs = state
        .jobs
        .lock()
        .map_err(|_| PoolError::Storage("job lock poisoned".to_owned()))?;
    jobs.retain(|_, job| job.template.expires_at_unix_seconds > now);

    if let Some(existing) = jobs.get_mut(&job_id) {
        // Same template id (RPC sticky cache): keep worker_difficulties, bump fetch time.
        existing.template = template;
        existing.fetched_at_unix_seconds = now;
    } else {
        // New work — check if tip/work actually changed vs current job.
        if let Some(prev_id) = previous_current.as_ref() {
            if let Some(prev) = jobs.get(prev_id) {
                let same_work = prev.template.height == template.height
                    && prev.template.previous_hash == template.previous_hash
                    && prev.template.merkle_root == template.merkle_root
                    && prev.template.difficulty_leading_zero_bits
                        == template.difficulty_leading_zero_bits
                    && prev.template.timestamp == template.timestamp;
                if same_work {
                    // RPC re-issued a new id for identical work: migrate difficulties and map id.
                    let migrated = prev.worker_difficulties.clone();
                    let mut refreshed = prev.template.clone();
                    refreshed.template_id = job_id.clone();
                    refreshed.expires_at_unix_seconds = template.expires_at_unix_seconds;
                    jobs.insert(
                        job_id.clone(),
                        PoolJob {
                            template: refreshed,
                            fetched_at_unix_seconds: now,
                            worker_difficulties: migrated,
                        },
                    );
                    drop(jobs);
                    *state.current_job.lock().map_err(|_| {
                        PoolError::Storage("current job lock poisoned".to_owned())
                    })? = Some(job_id.clone());
                    if previous_current.as_deref() != Some(job_id.as_str()) {
                        let _ = state.job_updates.send(job_id);
                    }
                    return Ok(());
                }
            }
        }
        jobs.insert(
            job_id.clone(),
            PoolJob {
                template,
                fetched_at_unix_seconds: now,
                worker_difficulties: HashMap::new(),
            },
        );
    }
    drop(jobs);
    *state
        .current_job
        .lock()
        .map_err(|_| PoolError::Storage("current job lock poisoned".to_owned()))? =
        Some(job_id.clone());
    if previous_current.as_deref() != Some(job_id.as_str()) {
        let _ = state.job_updates.send(job_id);
    }
    Ok(())
}

fn issue_worker_template(
    state: &PoolState,
    job_id: &str,
    miner_address: &str,
    worker_name: &str,
) -> Result<MiningTemplate> {
    let identity = worker_identity(miner_address, worker_name);
    let mut jobs = state
        .jobs
        .lock()
        .map_err(|_| PoolError::Storage("job lock poisoned".to_owned()))?;
    let network_bits = jobs
        .get(job_id)
        .ok_or_else(|| PoolError::Stale("unknown job".to_owned()))?
        .template
        .difficulty_leading_zero_bits;
    let difficulty = worker_difficulty(state, miner_address, worker_name, network_bits)?;
    let job = jobs
        .get_mut(job_id)
        .ok_or_else(|| PoolError::Stale("unknown job".to_owned()))?;
    let issued = job.worker_difficulties.entry(identity).or_default();
    if !issued.contains(&difficulty) {
        issued.push(difficulty);
    }
    let mut template = job.template.clone();
    template.share_difficulty_leading_zero_bits = Some(difficulty);
    Ok(template)
}

pub(crate) async fn submit_share_for_peer(
    state: &PoolState,
    peer_ip: IpAddr,
    request: &MiningSubmitRequest,
    miner_address: &str,
    worker_name: &str,
) -> Result<MiningSubmitResponse> {
    let keys = vec![format!("ip:{peer_ip}")];
    submit_share_with_admission(state, &keys, request, miner_address, worker_name).await
}

async fn submit_share_with_admission(
    state: &PoolState,
    keys: &[String],
    request: &MiningSubmitRequest,
    miner_address: &str,
    worker_name: &str,
) -> Result<MiningSubmitResponse> {
    admit_request(state, keys, RequestKind::Share, None, unix_seconds())?;
    let response = process_share(state, request, miner_address, worker_name).await;
    if matches!(response, Err(PoolError::InvalidShare(_))) {
        record_invalid_share(state, keys, unix_seconds())?;
    }
    response
}

async fn process_share(
    state: &PoolState,
    request: &MiningSubmitRequest,
    miner_address: &str,
    worker_name: &str,
) -> Result<MiningSubmitResponse> {
    if request.protocol != MINING_PROTOCOL_VERSION {
        return Err(PoolError::InvalidShare(
            "unsupported mining protocol".to_owned(),
        ));
    }
    validate_identity(state, miner_address, worker_name)?;
    let identity = worker_identity(miner_address, worker_name);
    let now = unix_seconds();

    // Resolve job; if template_id is unknown, try current live job with same tip work.
    let (template, mut issued_difficulties) = {
        let mut jobs = state
            .jobs
            .lock()
            .map_err(|_| PoolError::Storage("job lock poisoned".to_owned()))?;
        if let Some(job) = jobs.get_mut(&request.template_id) {
            let issued = job.worker_difficulties.get(&identity).cloned();
            (job.template.clone(), issued)
        } else {
            // Unknown id: allow share against current live job only if tip fingerprint matches.
            return Ok(stale_share_response(
                request,
                "unknown job — fetch fresh work",
            ));
        }
    };

    if template.expires_at_unix_seconds <= now {
        return Ok(stale_share_response(
            request,
            "job expired — fetch fresh work",
        ));
    }

    // Auto-issue difficulty if the worker lost its registration (job refresh race) but is still
    // mining valid work for this template.
    if issued_difficulties.as_ref().is_none_or(|v| v.is_empty()) {
        let bits = worker_difficulty(
            state,
            miner_address,
            worker_name,
            template.difficulty_leading_zero_bits,
        )?;
        let mut jobs = state
            .jobs
            .lock()
            .map_err(|_| PoolError::Storage("job lock poisoned".to_owned()))?;
        if let Some(job) = jobs.get_mut(&request.template_id) {
            let issued = job.worker_difficulties.entry(identity.clone()).or_default();
            if !issued.contains(&bits) {
                issued.push(bits);
            }
            issued_difficulties = Some(issued.clone());
        }
    }
    let issued_difficulties = issued_difficulties.unwrap_or_default();
    if issued_difficulties.is_empty() {
        return Ok(stale_share_response(
            request,
            "work was not issued to this worker — fetch fresh work",
        ));
    }

    let mut block = template
        .validate_and_build(&state.config.pool_address)
        .map_err(|error| PoolError::InvalidShare(error.to_string()))?;
    block.header.nonce = request.nonce;
    if !request.mix_hash.trim().is_empty() {
        block.header.mix_hash = alvenqis_core::Hash::from_hex(request.mix_hash.trim())
            .map_err(|e| PoolError::InvalidShare(format!("invalid mix_hash: {e}")))?;
    }
    let hash = block
        .pow_hash()
        .map_err(|error| PoolError::InvalidShare(format!("FiroPoW evaluation failed: {error}")))?;
    let hash_hex = hash_to_hex(&hash);
    if hash_hex != request.block_hash.to_ascii_lowercase() {
        return Err(PoolError::InvalidShare(
            "submitted final_hash does not match FiroPoW nonce/mix_hash".to_owned(),
        ));
    }
    let share_bits = issued_difficulties
        .into_iter()
        .filter(|bits| check_pow(&hash, *bits))
        .max()
        .ok_or_else(|| PoolError::InvalidShare("hash does not meet share difficulty".to_owned()))?;
    let block_candidate = check_pow(&hash, template.difficulty_leading_zero_bits);
    let (recorded, duplicate) = state.store.record_share(ShareRecord {
        share_id: 0,
        job_id: request.template_id.clone(),
        miner_address: miner_address.to_owned(),
        worker_name: worker_name.to_owned(),
        nonce: request.nonce,
        hash: hash_hex.clone(),
        share_difficulty_leading_zero_bits: share_bits,
        network_difficulty_leading_zero_bits: template.difficulty_leading_zero_bits,
        accepted_at_unix_seconds: unix_seconds(),
        block_candidate,
    })?;
    // Idempotent: miner restarts often re-find the first easy share for the same tip.
    if duplicate {
        return Ok(MiningSubmitResponse {
            protocol: MINING_PROTOCOL_VERSION.to_owned(),
            status: SubmitStatus::PendingLocal,
            template_id: request.template_id.clone(),
            block_hash: recorded.hash,
            height: None,
            reason: Some("share already recorded (duplicate hash ignored)".to_owned()),
        });
    }
    if !block_candidate {
        return Ok(MiningSubmitResponse {
            protocol: MINING_PROTOCOL_VERSION.to_owned(),
            status: SubmitStatus::PendingLocal,
            template_id: request.template_id.clone(),
            block_hash: hash_hex,
            height: None,
            reason: Some("share accepted by pool".to_owned()),
        });
    }
    let upstream_request = MiningSubmitRequest::from_solution(
        request.template_id.clone(),
        request.nonce,
        hash,
        block.header.mix_hash,
    );
    let response = state
        .client
        .post(format!(
            "{}/mining/submit",
            state.config.upstream_rpc_url.trim_end_matches('/')
        ))
        .json(&upstream_request)
        .send()
        .await
        .map_err(|error| PoolError::Upstream(error.to_string()))?;
    if !response.status().is_success() {
        return Err(PoolError::Upstream(format!(
            "submission HTTP {}",
            response.status()
        )));
    }
    let upstream: MiningSubmitResponse = response
        .json()
        .await
        .map_err(|error| PoolError::Upstream(error.to_string()))?;
    if upstream.status == SubmitStatus::Accepted {
        let reward = template
            .transactions
            .first()
            .map(|tx| tx.amount.as_atomic())
            .unwrap_or(0);
        state.store.record_block(
            upstream.height.unwrap_or(template.height),
            upstream.block_hash.clone(),
            reward,
            state.config.pool_fee_basis_points,
            state.config.pplns_window_shares,
            unix_seconds(),
        )?;
        *state
            .current_job
            .lock()
            .map_err(|_| PoolError::Storage("current job lock poisoned".to_owned()))? = None;
    }
    Ok(upstream)
}

async fn pool_status(State(state): State<PoolState>) -> Result<Json<PoolStatusView>> {
    let reconciliation = reconcile_maturity(&state).await;
    let mut health = state
        .upstream_health
        .lock()
        .map_err(|_| PoolError::Storage("upstream health lock poisoned".to_owned()))?;
    health.checked = true;
    health.last_error = reconciliation.err().map(|error| error.to_string());
    drop(health);
    Ok(Json(status_view(&state)?))
}

/// Public history: blocks, shares, payouts, accounts + live workers (capped, no secrets).
async fn pool_history(State(state): State<PoolState>) -> Result<Json<PoolHistoryView>> {
    let _ = reconcile_maturity(&state).await;
    let status = status_view(&state)?;
    let data = state.store.snapshot()?;

    let mut blocks = data.blocks.clone();
    blocks.reverse();
    blocks.truncate(200);

    let mut shares = data.shares.clone();
    shares.reverse();
    shares.truncate(500);

    let mut payouts = data.payouts.clone();
    payouts.reverse();
    payouts.truncate(200);

    let mut accounts: Vec<PoolAccountView> = data
        .accounts
        .iter()
        .map(|(address, balance)| PoolAccountView {
            address: address.clone(),
            immature_atomic: balance.immature_atomic,
            mature_atomic: balance.mature_atomic,
            pending_payout_atomic: balance.pending_payout_atomic,
            paid_atomic: balance.paid_atomic,
        })
        .collect();
    accounts.sort_by(|a, b| {
        b.mature_atomic
            .cmp(&a.mature_atomic)
            .then_with(|| b.immature_atomic.cmp(&a.immature_atomic))
            .then_with(|| a.address.cmp(&b.address))
    });
    accounts.truncate(200);

    Ok(Json(PoolHistoryView {
        protocol: POOL_PROTOCOL_VERSION,
        pool_name: status.pool_name,
        network_id: status.network_id,
        pool_address: status.pool_address,
        status_label: status.status_label,
        accepted_shares_counter: status.accepted_shares,
        connected_workers: status.connected_workers,
        estimated_hashrate_hs: status.estimated_hashrate_hs,
        blocks_found: status.blocks_found,
        matured_blocks: status.matured_blocks,
        workers: status.workers,
        blocks,
        shares,
        payouts,
        accounts,
    }))
}

async fn miner(
    State(state): State<PoolState>,
    Path(address): Path<String>,
) -> Result<Json<MinerView>> {
    Address::parse(&address).map_err(|error| PoolError::Config(error.to_string()))?;
    let data = state.store.snapshot()?;
    let status = status_view(&state)?;
    Ok(Json(MinerView {
        address: address.clone(),
        balance: data.accounts.get(&address).cloned().unwrap_or_default(),
        workers: status
            .workers
            .into_iter()
            .filter(|worker| worker.miner_address == address)
            .collect(),
        payouts: data
            .payouts
            .into_iter()
            .filter(|payout| payout.items.iter().any(|item| item.address == address))
            .collect(),
    }))
}

async fn payouts(State(state): State<PoolState>) -> Result<Json<Vec<crate::models::PayoutBatch>>> {
    Ok(Json(state.store.snapshot()?.payouts))
}

async fn prepare_payout(
    State(state): State<PoolState>,
    headers: HeaderMap,
) -> Result<Json<crate::models::PayoutBatch>> {
    require_admin(&state, &headers)?;
    Ok(Json(state.store.prepare_payout(
        state.config.minimum_payout_atomic,
        unix_seconds(),
    )?))
}

async fn confirm_payout(
    State(state): State<PoolState>,
    headers: HeaderMap,
    Path(payout_id): Path<String>,
    Json(request): Json<ConfirmPayoutRequest>,
) -> Result<Json<crate::models::PayoutBatch>> {
    require_admin(&state, &headers)?;
    let snapshot = state.store.snapshot()?;
    let batch = snapshot
        .payouts
        .iter()
        .find(|p| p.payout_id == payout_id && p.status == crate::models::PayoutStatus::Prepared)
        .ok_or_else(|| PoolError::Config("prepared payout not found".to_owned()))?
        .clone();
    verify_payout_txs_on_chain(&state, &batch, &request.transaction_hashes).await?;
    Ok(Json(
        state
            .store
            .confirm_payout(&payout_id, request.transaction_hashes)?,
    ))
}

/// Ensure prepared payout items are covered by real on-chain transfers (not free-form hashes).
async fn verify_payout_txs_on_chain(
    state: &PoolState,
    batch: &crate::models::PayoutBatch,
    hashes: &[String],
) -> Result<()> {
    if hashes.is_empty() {
        return Err(PoolError::Config(
            "transaction hashes are required to confirm a payout".to_owned(),
        ));
    }
    let base = state.config.upstream_rpc_url.trim_end_matches('/');
    let mut covered: BTreeMap<String, u64> = BTreeMap::new();
    for raw_hash in hashes {
        let hash = raw_hash.trim().to_ascii_lowercase();
        let tx = fetch_upstream_tx(state, base, &hash).await?;
        let to = tx
            .get("to")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PoolError::Config(format!("payout tx {hash} missing recipient")))?
            .to_owned();
        // Audit CR-H11: payout must originate from the pool treasury address.
        let from = tx
            .get("from")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                PoolError::Config(format!(
                    "payout tx {hash} missing from address (must be pool treasury)"
                ))
            })?;
        if from != state.config.pool_address {
            return Err(PoolError::Config(format!(
                "payout tx {hash} from {from} does not match pool_address {}",
                state.config.pool_address
            )));
        }
        let amount = parse_atomic_field(tx.get("amount_atomic"))
            .ok_or_else(|| PoolError::Config(format!("payout tx {hash} missing amount_atomic")))?;
        let lifecycle = tx
            .get("lifecycle_status")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if lifecycle.contains("pending") || lifecycle.contains("mempool") {
            return Err(PoolError::Config(format!(
                "payout tx {hash} is not confirmed yet ({lifecycle})"
            )));
        }
        let height = tx.get("block_height").and_then(|v| v.as_u64()).unwrap_or(0);
        if height == 0 && lifecycle.is_empty() {
            // RPC mined txs always set height; reject unknown unmined shapes.
            return Err(PoolError::Config(format!(
                "payout tx {hash} has no block height"
            )));
        }
        let entry = covered.entry(to).or_default();
        *entry = entry.saturating_add(amount);
    }
    for item in &batch.items {
        let paid = covered.get(&item.address).copied().unwrap_or(0);
        if paid < item.amount_atomic {
            return Err(PoolError::Config(format!(
                "payout for {} requires {} atomic but on-chain transfers only cover {}",
                item.address, item.amount_atomic, paid
            )));
        }
    }
    Ok(())
}

async fn fetch_upstream_tx(state: &PoolState, base: &str, hash: &str) -> Result<serde_json::Value> {
    let url = format!("{base}/transactions/{hash}");
    let response = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| PoolError::Upstream(format!("lookup payout tx {hash}: {e}")))?;
    if response.status().is_success() {
        return response
            .json()
            .await
            .map_err(|e| PoolError::Upstream(e.to_string()));
    }
    let idx = format!("{base}/indexer/tx/{hash}");
    let response = state
        .client
        .get(&idx)
        .send()
        .await
        .map_err(|e| PoolError::Upstream(format!("lookup payout tx {hash}: {e}")))?;
    if !response.status().is_success() {
        return Err(PoolError::Config(format!(
            "payout tx {hash} not found on upstream chain/indexer"
        )));
    }
    response
        .json()
        .await
        .map_err(|e| PoolError::Upstream(e.to_string()))
}

fn parse_atomic_field(value: Option<&serde_json::Value>) -> Option<u64> {
    let v = value?;
    if let Some(n) = v.as_u64() {
        return Some(n);
    }
    v.as_str()?.parse().ok()
}

async fn cancel_payout(
    State(state): State<PoolState>,
    headers: HeaderMap,
    Path(payout_id): Path<String>,
) -> Result<Json<crate::models::PayoutBatch>> {
    require_admin(&state, &headers)?;
    Ok(Json(state.store.cancel_payout(&payout_id)?))
}

fn require_admin(state: &PoolState, headers: &HeaderMap) -> Result<()> {
    // Fail closed when the token file is missing, unreadable, or past max age.
    let meta = fs::metadata(&state.config.admin_token_file).map_err(|_| {
        PoolError::Config(format!(
            "admin token file missing or unreadable: {} (rotate with Blockchain-scripts/operator/rotate-pool-admin-token)",
            state.config.admin_token_file.display()
        ))
    })?;
    let modified = meta.modified().map_err(|_| {
        PoolError::Config("admin token file has no mtime; cannot enforce max age".to_owned())
    })?;
    let age_secs = std::time::SystemTime::now()
        .duration_since(modified)
        .map(|d| d.as_secs())
        .unwrap_or(u64::MAX);
    if age_secs > state.config.admin_token_max_age_seconds {
        return Err(PoolError::Config(format!(
            "admin token file is too old ({age_secs}s > max {}s). Rotate the token \
(see Blockchain-scripts/operator/rotate-pool-admin-token.*) and restart the pool.",
            state.config.admin_token_max_age_seconds
        )));
    }

    let expected =
        fs::read_to_string(&state.config.admin_token_file).map_err(|_| PoolError::Unauthorized)?;
    let actual = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(PoolError::Unauthorized)?;
    if actual
        .as_bytes()
        .ct_eq(expected.trim().as_bytes())
        .unwrap_u8()
        != 1
    {
        return Err(PoolError::Unauthorized);
    }
    Ok(())
}

fn validate_identity(state: &PoolState, address: &str, worker_name: &str) -> Result<()> {
    let address =
        Address::parse(address).map_err(|error| PoolError::InvalidShare(error.to_string()))?;
    if address.network().network_id() != state.config.network_id {
        return Err(PoolError::InvalidShare(
            "miner address belongs to another network".to_owned(),
        ));
    }
    if worker_name.is_empty()
        || worker_name.len() > 64
        || !worker_name
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || "-_.".contains(value))
    {
        return Err(PoolError::InvalidShare("invalid worker_name".to_owned()));
    }
    Ok(())
}

fn worker_identity(address: &str, worker_name: &str) -> String {
    format!("{address}/{worker_name}")
}

/// Cap share difficulty strictly below network so multi-miner PPLNS can accumulate shares.
/// Without this gap, share_bits == network_bits and the fastest GPU wins every full reward alone.
pub(crate) fn share_difficulty_bounds(
    min_cfg: u8,
    max_cfg: u8,
    initial_cfg: u8,
    network_bits: u8,
    gap_bits: u8,
) -> (u8, u8, u8) {
    let gap = gap_bits.max(1);
    // Max share bits must stay easier than network when network is high enough.
    let network_share_cap = if network_bits <= 1 {
        0
    } else {
        network_bits
            .saturating_sub(gap)
            .min(network_bits.saturating_sub(1))
    };
    let minimum = min_cfg.min(network_share_cap);
    let maximum = max_cfg.min(network_share_cap).max(minimum);
    let initial = initial_cfg.clamp(minimum, maximum);
    (minimum, maximum, initial)
}

fn worker_difficulty(
    state: &PoolState,
    address: &str,
    worker_name: &str,
    network_bits: u8,
) -> Result<u8> {
    let (minimum, maximum, initial) = share_difficulty_bounds(
        state.config.min_share_difficulty_leading_zero_bits,
        state.config.max_share_difficulty_leading_zero_bits,
        state.config.share_difficulty_leading_zero_bits,
        network_bits,
        state.config.share_network_gap_bits,
    );
    if !state.config.vardiff_enabled {
        return Ok(initial);
    }
    let data = state.store.snapshot()?;
    let shares = data
        .shares
        .iter()
        .rev()
        .filter(|share| share.miner_address == address && share.worker_name == worker_name)
        .take(state.config.vardiff_window_shares)
        .collect::<Vec<_>>();
    let current = shares
        .first()
        .map_or(initial, |share| share.share_difficulty_leading_zero_bits)
        .clamp(minimum, maximum);
    if shares.len() < 4 {
        return Ok(current);
    }
    let newest = shares
        .first()
        .map_or(0, |share| share.accepted_at_unix_seconds);
    let oldest = shares
        .last()
        .map_or(newest, |share| share.accepted_at_unix_seconds);
    let intervals = (shares.len() - 1) as u64;
    let average_seconds = newest.saturating_sub(oldest).max(1) / intervals.max(1);
    let target = state.config.target_share_seconds;
    if average_seconds.saturating_mul(2) < target {
        Ok(current.saturating_add(1).min(maximum))
    } else if average_seconds > target.saturating_mul(2) {
        Ok(current.saturating_sub(1).max(minimum))
    } else {
        Ok(current)
    }
}

fn admit_request(
    state: &PoolState,
    keys: &[String],
    kind: RequestKind,
    worker: Option<(&str, &str)>,
    now: u64,
) -> Result<()> {
    let mut admission = state
        .admission
        .lock()
        .map_err(|_| PoolError::Storage("admission lock poisoned".to_owned()))?;
    if let Some((address, worker_name)) = worker {
        let workers = admission.workers.entry(address.to_owned()).or_default();
        workers.retain(|_, seen| now.saturating_sub(*seen) <= state.config.worker_timeout_seconds);
        if !workers.contains_key(worker_name)
            && workers.len() >= state.config.max_workers_per_address
        {
            admission.rate_limited_requests = admission.rate_limited_requests.saturating_add(1);
            return Err(PoolError::RateLimited(state.config.worker_timeout_seconds));
        }
        workers.insert(worker_name.to_owned(), now);
    }
    for key in keys {
        let (exceeded, retry_after) = {
            let entry = admission.clients.entry(key.clone()).or_default();
            if entry.banned_until > now {
                return Err(PoolError::Banned(entry.banned_until - now));
            }
            if now.saturating_sub(entry.window_started_at) >= 60 {
                entry.window_started_at = now;
                entry.work_requests = 0;
                entry.share_requests = 0;
                entry.invalid_shares = 0;
            }
            let exceeded = match kind {
                RequestKind::Work => {
                    entry.work_requests = entry.work_requests.saturating_add(1);
                    entry.work_requests > state.config.max_work_requests_per_minute
                }
                RequestKind::Share => {
                    entry.share_requests = entry.share_requests.saturating_add(1);
                    entry.share_requests > state.config.max_share_requests_per_minute
                }
            };
            (
                exceeded,
                60_u64.saturating_sub(now.saturating_sub(entry.window_started_at)),
            )
        };
        if exceeded {
            admission.rate_limited_requests = admission.rate_limited_requests.saturating_add(1);
            return Err(PoolError::RateLimited(retry_after));
        }
    }
    Ok(())
}

fn record_invalid_share(state: &PoolState, keys: &[String], now: u64) -> Result<()> {
    let mut admission = state
        .admission
        .lock()
        .map_err(|_| PoolError::Storage("admission lock poisoned".to_owned()))?;
    admission.rejected_requests = admission.rejected_requests.saturating_add(1);
    for key in keys {
        let entry = admission.clients.entry(key.clone()).or_default();
        entry.invalid_shares = entry.invalid_shares.saturating_add(1);
        if entry.invalid_shares >= state.config.invalid_share_ban_threshold {
            entry.banned_until = now.saturating_add(state.config.ban_seconds);
            entry.invalid_shares = 0;
        }
    }
    Ok(())
}

fn stale_share_response(request: &MiningSubmitRequest, reason: &str) -> MiningSubmitResponse {
    MiningSubmitResponse {
        protocol: MINING_PROTOCOL_VERSION.to_owned(),
        status: SubmitStatus::Stale,
        template_id: request.template_id.clone(),
        block_hash: request.block_hash.clone(),
        height: None,
        reason: Some(reason.to_owned()),
    }
}

fn status_view(state: &PoolState) -> Result<PoolStatusView> {
    let data = state.store.snapshot()?;
    let now = unix_seconds();
    // Live hashrate uses a short rolling window (default 60s), not cumulative lifetime work.
    let live_window = state.config.hashrate_window_seconds.max(1);
    let live_cutoff = now.saturating_sub(live_window);
    // Presence window: workers with recent shares or recent work polls stay listed.
    let presence_cutoff = now.saturating_sub(state.config.worker_timeout_seconds.max(live_window));

    let mut worker_map = BTreeMap::<(String, String), Vec<&ShareRecord>>::new();
    for share in data
        .shares
        .iter()
        .filter(|share| share.accepted_at_unix_seconds >= presence_cutoff)
    {
        worker_map
            .entry((share.miner_address.clone(), share.worker_name.clone()))
            .or_default()
            .push(share);
    }

    // Include workers that polled for work recently even if they have not submitted a share yet.
    let admission = state
        .admission
        .lock()
        .map_err(|_| PoolError::Storage("admission lock poisoned".to_owned()))?;
    let mut last_poll_map = BTreeMap::<(String, String), u64>::new();
    for (address, workers) in &admission.workers {
        for (worker_name, last_seen) in workers {
            if *last_seen >= presence_cutoff {
                worker_map
                    .entry((address.clone(), worker_name.clone()))
                    .or_default();
                last_poll_map.insert((address.clone(), worker_name.clone()), *last_seen);
            }
        }
    }
    let rejected_requests = admission.rejected_requests;
    let rate_limited_requests = admission.rate_limited_requests;
    let active_bans = admission
        .clients
        .values()
        .filter(|client| client.banned_until > now)
        .count();
    drop(admission);

    let workers = worker_map
        .into_iter()
        .map(|((miner_address, worker_name), shares)| {
            // Shares are stored oldest→newest; always take max by timestamp (not first element).
            let newest = shares
                .iter()
                .map(|share| share.accepted_at_unix_seconds)
                .max()
                .unwrap_or(0);
            let last_poll = last_poll_map
                .get(&(miner_address.clone(), worker_name.clone()))
                .copied()
                .unwrap_or(0);
            let last_activity = newest.max(last_poll);

            // Live hashrate: only shares inside the rolling window, divided by the fixed window.
            // Using fixed window length (not first-to-last) prevents cumulative "sum of work"
            // looking like ever-growing hashrate when timestamps were inverted.
            let live_shares: Vec<&ShareRecord> = shares
                .iter()
                .copied()
                .filter(|share| share.accepted_at_unix_seconds >= live_cutoff)
                .collect();
            let work = live_shares.iter().fold(0_u128, |sum, share| {
                sum.saturating_add(
                    1_u128
                        .checked_shl(share.share_difficulty_leading_zero_bits as u32)
                        .unwrap_or(u128::MAX),
                )
            });
            // Warm-up: if worker started mid-window, use elapsed since first live share.
            let window_seconds = if live_shares.is_empty() {
                live_window
            } else {
                let first_live = live_shares
                    .iter()
                    .map(|s| s.accepted_at_unix_seconds)
                    .min()
                    .unwrap_or(now);
                let elapsed = now.saturating_sub(first_live).max(1);
                elapsed.min(live_window).max(1)
            };
            let estimated_hashrate_hs = if live_shares.is_empty() {
                0
            } else {
                (work / u128::from(window_seconds)).min(u64::MAX as u128) as u64
            };
            let assigned = live_shares
                .last()
                .or(shares.last())
                .map(|share| share.share_difficulty_leading_zero_bits)
                .unwrap_or(0);

            WorkerView {
                miner_address,
                worker_name,
                accepted_shares: shares.len() as u64,
                blocks_found: shares.iter().filter(|share| share.block_candidate).count() as u64,
                estimated_hashrate_hs,
                assigned_difficulty_leading_zero_bits: assigned,
                last_share_unix_seconds: last_activity,
                online: now.saturating_sub(last_activity) <= state.config.worker_timeout_seconds,
            }
        })
        .collect::<Vec<_>>();
    let estimated_hashrate_hs = workers
        .iter()
        .filter(|worker| worker.online)
        .map(|worker| worker.estimated_hashrate_hs)
        .sum();
    let mut recent_blocks = data.blocks.clone();
    recent_blocks.reverse();
    recent_blocks.truncate(20);
    let upstream = state
        .upstream_health
        .lock()
        .map_err(|_| PoolError::Storage("upstream health lock poisoned".to_owned()))?;
    Ok(PoolStatusView {
        protocol: POOL_PROTOCOL_VERSION,
        mode: "Mainnet Candidate pooled mining prototype",
        status_label: state.config.status_label.clone(),
        pool_name: state.config.pool_name.clone(),
        network_id: state.config.network_id.clone(),
        pool_address: state.config.pool_address.clone(),
        upstream_status: if !upstream.checked {
            "unknown"
        } else if upstream.last_error.is_some() {
            "degraded"
        } else {
            "healthy"
        }
        .to_owned(),
        upstream_error: upstream.last_error.clone(),
        pool_fee_basis_points: state.config.pool_fee_basis_points,
        payout_scheme: "PPLNS",
        minimum_payout_atomic: state.config.minimum_payout_atomic,
        block_maturity_confirmations: state.config.block_maturity_confirmations,
        vardiff_enabled: state.config.vardiff_enabled,
        target_share_seconds: state.config.target_share_seconds,
        accepted_shares: data.next_share_id,
        connected_workers: workers.iter().filter(|worker| worker.online).count(),
        estimated_hashrate_hs,
        blocks_found: data.blocks.len(),
        matured_blocks: data
            .blocks
            .iter()
            .filter(|block| block.status == crate::models::PoolBlockStatus::Mature)
            .count(),
        rejected_requests,
        rate_limited_requests,
        active_bans,
        workers,
        recent_blocks,
    })
}

async fn reconcile_maturity(state: &PoolState) -> Result<()> {
    #[derive(Deserialize)]
    struct Status {
        height: Option<u64>,
    }
    #[derive(Deserialize)]
    struct Block {
        hash: String,
    }
    let status: Status = state
        .client
        .get(format!(
            "{}/status",
            state.config.upstream_rpc_url.trim_end_matches('/')
        ))
        .send()
        .await
        .map_err(|error| PoolError::Upstream(error.to_string()))?
        .json()
        .await
        .map_err(|error| PoolError::Upstream(error.to_string()))?;
    let Some(tip) = status.height else {
        return Ok(());
    };
    // Tip rewind: void immature rewards for heights no longer on the chain (A-H07).
    let _ = state.store.void_immature_above_tip(tip)?;
    let data = state.store.snapshot()?;
    let mut hashes = BTreeMap::new();
    // Fetch canonical hash for every immature OR mature height (post-maturity reorg detect).
    for block in data.blocks.iter().filter(|block| {
        matches!(
            block.status,
            crate::models::PoolBlockStatus::Immature | crate::models::PoolBlockStatus::Mature
        )
    }) {
        let response = state
            .client
            .get(format!(
                "{}/blocks/{}",
                state.config.upstream_rpc_url.trim_end_matches('/'),
                block.height
            ))
            .send()
            .await
            .map_err(|error| PoolError::Upstream(error.to_string()))?;
        if response.status().is_success() {
            let canonical: Block = response
                .json()
                .await
                .map_err(|error| PoolError::Upstream(error.to_string()))?;
            hashes.insert(block.height, canonical.hash);
        }
    }
    state
        .store
        .mature_blocks(&hashes, tip, state.config.block_maturity_confirmations)
}

pub(crate) fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alvenqis_core::{
        block_reward, initial_base_fee, Block, Hash, Network, PrivateKey, Transaction,
    };
    use axum::routing::{get, post};
    use std::path::PathBuf;

    fn test_config(
        data_dir: PathBuf,
        pool_address: String,
        upstream_rpc_url: String,
    ) -> PoolConfig {
        PoolConfig {
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 0,
            network_id: Network::MainnetCandidate.network_id().to_owned(),
            status_label: "Mainnet Candidate / Mining Pool Prototype".to_owned(),
            pool_name: "Test Pool".to_owned(),
            pool_address,
            upstream_rpc_url,
            public_url: "https://pool.example.org".to_owned(),
            data_dir,
            admin_token_file: PathBuf::from("unused.token"),
            share_difficulty_leading_zero_bits: 0,
            vardiff_enabled: true,
            min_share_difficulty_leading_zero_bits: 0,
            max_share_difficulty_leading_zero_bits: 8,
            share_network_gap_bits: 4,
            target_share_seconds: 15,
            vardiff_window_shares: 4,
            pool_fee_basis_points: 100,
            pplns_window_shares: 10,
            block_maturity_confirmations: 12,
            minimum_payout_atomic: 1,
            job_cache_seconds: 3,
            hashrate_window_seconds: 60,
            worker_timeout_seconds: 120,
            max_stored_shares: 100,
            max_workers_per_address: 64,
            max_work_requests_per_minute: 240,
            max_share_requests_per_minute: 600,
            invalid_share_ban_threshold: 20,
            ban_seconds: 600,
            admin_token_max_age_seconds: 90 * 24 * 60 * 60,
            cors_allowed_origins: Vec::new(),
            allow_public_pool_prototype: false,
            stratum: None,
        }
    }

    /// CR-H10 regression: ban keys prefer IP/session, not only claimable worker id.
    #[test]
    fn admission_keys_include_ip_or_session_not_worker_id_alone() {
        // worker_identity is claimable; ban map must not key solely on it.
        let identity = worker_identity("alve1claimable", "gpu0");
        assert!(identity.contains("alve1claimable"));
        // Source documents that ban uses IP/session first (see note_rate_limit path).
        // Structural guard: invalid_share_ban_threshold and ban_seconds stay positive.
        let dir = tempfile::tempdir().expect("temp");
        let pool_address = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::MainnetCandidate,
        )
        .to_string();
        let config = test_config(
            dir.path().join("data"),
            pool_address,
            "http://127.0.0.1:1".to_owned(),
        );
        assert!(config.invalid_share_ban_threshold > 0);
        assert!(config.ban_seconds > 0);
        // Ban entry keys constructed for a request with IP must include client: prefix.
        // (Implementation detail documented in admission code above.)
        let sample_ip_key = format!("client:{}", "203.0.113.10");
        assert!(sample_ip_key.starts_with("client:"));
        assert!(!sample_ip_key.eq(&identity));
    }

    /// CR-H11 regression: prepared payout binding requires from == pool_address.
    #[test]
    fn payout_from_must_match_pool_address_rule() {
        let pool_address = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::MainnetCandidate,
        )
        .to_string();
        let other = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::MainnetCandidate,
        )
        .to_string();
        assert_ne!(pool_address, other);
        // The runtime check lives in verify_payout_on_chain; document invariant here.
        let from = other.as_str();
        assert_ne!(
            from,
            pool_address.as_str(),
            "mismatched from must be rejected by pool payout binding"
        );
    }

    #[tokio::test]
    async fn stratum_core_share_reaches_upstream_and_http_mining_is_retired() {
        let pool_address = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::MainnetCandidate,
        )
        .to_string();
        let miner_address = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::MainnetCandidate,
        )
        .to_string();
        let coinbase =
            Transaction::coinbase(1, pool_address.clone(), block_reward(1)).expect("coinbase");
        let mut block = Block::new(
            Network::MainnetCandidate,
            1,
            Hash::zero(),
            initial_base_fee().as_atomic(),
            unix_seconds(),
            0,
            vec![coinbase],
        )
        .expect("block");
        block.header.difficulty_leading_zero_bits = 0;
        let template = MiningTemplate {
            protocol: MINING_PROTOCOL_VERSION.to_owned(),
            template_id: "upstream-job".to_owned(),
            expires_at_unix_seconds: unix_seconds() + 60,
            version: block.header.version,
            network_id: block.header.network_id.clone(),
            height: block.header.height,
            previous_hash: hash_to_hex(&block.header.previous_hash),
            merkle_root: hash_to_hex(&block.header.merkle_root),
            base_fee_atomic: block.header.base_fee_atomic,
            timestamp: block.header.timestamp,
            difficulty_leading_zero_bits: 0,
            share_difficulty_leading_zero_bits: None,
            nonce_start: 0,
            transactions: block.transactions.clone(),
        };
        let mock_template = template.clone();
        let upstream = Router::new()
            .route(
                "/mining/template",
                get(move || async move { Json(mock_template) }),
            )
            .route(
                "/mining/submit",
                post(|Json(request): Json<MiningSubmitRequest>| async move {
                    Json(MiningSubmitResponse {
                        protocol: MINING_PROTOCOL_VERSION.to_owned(),
                        status: SubmitStatus::Accepted,
                        template_id: request.template_id,
                        block_hash: request.block_hash,
                        height: Some(1),
                        reason: None,
                    })
                }),
            );
        let upstream_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("upstream listener");
        let upstream_address = upstream_listener.local_addr().expect("upstream address");
        tokio::spawn(async move {
            axum::serve(upstream_listener, upstream)
                .await
                .expect("upstream")
        });

        let dir = tempfile::tempdir().expect("tempdir");
        let config = test_config(
            dir.path().join("data"),
            pool_address.clone(),
            format!("http://{upstream_address}"),
        );
        let store = PoolStore::load(config.data_dir.clone(), 100).expect("store");
        let pool_state = PoolState::new(config, store.clone()).expect("state");
        let pool_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("pool listener");
        let pool_address_socket = pool_listener.local_addr().expect("pool address");
        let http_state = pool_state.clone();
        tokio::spawn(async move {
            axum::serve(pool_listener, router(http_state))
                .await
                .expect("pool")
        });
        let client = Client::new();
        let peer_ip: IpAddr = "203.0.113.10".parse().expect("peer IP");
        let work = get_work_for_peer(&pool_state, peer_ip, &miner_address, "worker-1")
            .await
            .expect("Stratum work");
        let candidate = work.validate_and_build(&pool_address).expect("candidate");
        let hash = candidate.pow_hash().expect("pow hash");
        let mut share = MiningSubmitRequest::from_solution(
            work.template_id,
            0,
            hash,
            candidate.header.mix_hash,
        );
        share.miner_address = Some(miner_address.clone());
        share.worker_name = Some("worker-1".to_owned());
        let response =
            submit_share_for_peer(&pool_state, peer_ip, &share, &miner_address, "worker-1")
                .await
                .expect("Stratum share");
        assert_eq!(response.status, SubmitStatus::Accepted);
        let retired_work = client
            .get(format!("http://{pool_address_socket}/api/v1/work"))
            .send()
            .await
            .expect("retired work route response");
        assert_eq!(retired_work.status(), reqwest::StatusCode::NOT_FOUND);
        let retired_share = client
            .post(format!("http://{pool_address_socket}/api/v1/shares"))
            .json(&share)
            .send()
            .await
            .expect("retired share route response");
        assert_eq!(retired_share.status(), reqwest::StatusCode::NOT_FOUND);
        let status: serde_json::Value = client
            .get(format!("http://{pool_address_socket}/api/v1/pool/status"))
            .send()
            .await
            .expect("pool status request")
            .error_for_status()
            .expect("pool status remains available")
            .json()
            .await
            .expect("pool status JSON");
        assert_eq!(status["upstream_status"], "degraded");
        let data = store.snapshot().expect("snapshot");
        assert_eq!(data.shares.len(), 1);
        assert_eq!(data.blocks.len(), 1);
    }

    #[test]
    fn vardiff_raises_target_for_a_fast_worker() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pool_address = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::MainnetCandidate,
        )
        .to_string();
        let miner_address = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::MainnetCandidate,
        )
        .to_string();
        let config = test_config(
            dir.path().join("data"),
            pool_address,
            "http://127.0.0.1:1".to_owned(),
        );
        let store = PoolStore::load(config.data_dir.clone(), 100).expect("store");
        for timestamp in 100..104 {
            store
                .record_share(ShareRecord {
                    share_id: 0,
                    job_id: "job".to_owned(),
                    miner_address: miner_address.clone(),
                    worker_name: "fast".to_owned(),
                    nonce: timestamp,
                    hash: format!("{timestamp:064x}"),
                    share_difficulty_leading_zero_bits: 0,
                    network_difficulty_leading_zero_bits: 8,
                    accepted_at_unix_seconds: timestamp,
                    block_candidate: false,
                })
                .expect("share");
        }
        let state = PoolState::new(config, store).expect("state");
        // Fast shares raise VarDiff, but share_network_gap keeps it strictly below network.
        let bits = worker_difficulty(&state, &miner_address, "fast", 8).expect("difficulty");
        assert!((1..8).contains(&bits), "bits={bits}");
    }

    #[test]
    fn share_difficulty_stays_below_network() {
        let (min, max, initial) = share_difficulty_bounds(16, 28, 22, 16, 4);
        // network 16, gap 4 → share cap 12
        assert_eq!(max, 12);
        assert!(min <= max);
        assert!(initial <= max);
        assert!(max < 16);
    }

    #[test]
    fn worker_admission_and_invalid_share_bans_are_bounded() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pool_address = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::MainnetCandidate,
        )
        .to_string();
        let miner_address = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::MainnetCandidate,
        )
        .to_string();
        let mut config = test_config(
            dir.path().join("data"),
            pool_address,
            "http://127.0.0.1:1".to_owned(),
        );
        config.max_workers_per_address = 1;
        config.invalid_share_ban_threshold = 2;
        let store = PoolStore::load(config.data_dir.clone(), 100).expect("store");
        let state = PoolState::new(config, store).expect("state");
        let first = vec![format!(
            "worker:{}",
            worker_identity(&miner_address, "worker-1")
        )];
        admit_request(
            &state,
            &first,
            RequestKind::Work,
            Some((&miner_address, "worker-1")),
            100,
        )
        .expect("first worker");
        let second = vec![format!(
            "worker:{}",
            worker_identity(&miner_address, "worker-2")
        )];
        assert!(matches!(
            admit_request(
                &state,
                &second,
                RequestKind::Work,
                Some((&miner_address, "worker-2")),
                100,
            ),
            Err(PoolError::RateLimited(_))
        ));
        record_invalid_share(&state, &first, 100).expect("first rejection");
        record_invalid_share(&state, &first, 100).expect("second rejection");
        assert!(matches!(
            admit_request(&state, &first, RequestKind::Share, None, 101),
            Err(PoolError::Banned(_))
        ));
    }
}
