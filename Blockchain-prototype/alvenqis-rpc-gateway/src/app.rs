use crate::config::RpcConfig;
use crate::error::{RpcError, RpcResult};
use crate::models::{
    address_account_response, address_balance_response, address_response, block_response,
    state_response, supply_response, transaction_response, AddressAccountResponse,
    AddressBalanceResponse, AddressResponse, BlockResponse, ChainHeightResponse, ChainTipResponse,
    HealthResponse, IndexedAddressesPageResponse, IndexedBlocksPageResponse,
    IndexedTransactionsPageResponse, IndexerOverviewResponse, MempoolResponse,
    MempoolStatusResponse, MiningSubmitRequest, MiningSubmitResponse, MiningTemplateResponse,
    NetworkResponse, StateResponse, StatusResponse, SubmitTransactionResponse, SupplyResponse,
    SyncStatusResponse, TransactionResponse,
};
use alvenqis_core::{hash_to_hex, next_base_fee, Block, Chain, Transaction};
use alvenqis_indexer::{
    load_index as load_index_snapshot, AddressActivity, IndexData, IndexedBlock,
    IndexedTransaction, IndexerStatus, INDEXER_MODE,
};
use alvenqis_node::{
    create_block_template, load_p2p_status, load_pending_transactions,
    mempool_status as load_mempool_status, runtime_dir_for_data_dir, storage, submit_mined_block,
    submit_transaction as submit_pending_transaction, NetworkConfig, P2pStatus,
};
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use http::header::CONTENT_TYPE;
use http::{HeaderValue, Method};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::Path as FsPath;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::{ServeDir, ServeFile};

const MINING_PROTOCOL: &str = "alvenqis-mining-v1";
const MINING_TEMPLATE_TTL_SECONDS: u64 = 90;
/// Prefer reusing an existing template while it has at least this much life left.
/// Prevents miners from thrashing nonce search on a new timestamp every poll.
const MINING_TEMPLATE_REUSE_MIN_REMAINING_SECONDS: u64 = 25;
const MAX_ACTIVE_MINING_TEMPLATES: usize = 256;

#[derive(Clone, Debug)]
struct StoredMiningTemplate {
    expires_at_unix_seconds: u64,
    block: Block,
    miner_address: String,
    chain_fingerprint: storage::ChainStorageFingerprint,
}

#[derive(Clone, Debug)]
struct CachedChain {
    /// Fingerprint of the SQLite database and WAL when loaded.
    fingerprint: storage::ChainStorageFingerprint,
    blocks: Arc<Vec<Block>>,
    chain: Arc<Chain>,
    height: Option<u64>,
    tip_hash: Option<String>,
    emitted_supply_atomic: u64,
    cumulative_work: Option<String>,
}

#[derive(Clone, Debug)]
struct CachedIndex {
    fingerprint: (u64, u64),
    data: Arc<IndexData>,
    transactions: Arc<Vec<IndexedTransaction>>,
    addresses: Arc<Vec<AddressActivity>>,
}

#[derive(Clone, Debug, Default)]
struct RateBucket {
    window_started_at: u64,
    write_count: u32,
    template_count: u32,
}

#[derive(Clone, Debug)]
pub struct RpcState {
    pub config: RpcConfig,
    node_config_path: PathBuf,
    mining_templates: Arc<Mutex<HashMap<String, StoredMiningTemplate>>>,
    /// Serialize expensive full-chain template builds without blocking the async runtime.
    mining_template_build_lock: Arc<tokio::sync::Mutex<()>>,
    /// Process-local chain cache invalidated by file fingerprint (maturity: multi-client load).
    chain_cache: Arc<Mutex<Option<CachedChain>>>,
    /// Read-only index cache. The dedicated indexer service is the sole index writer.
    index_cache: Arc<Mutex<Option<CachedIndex>>>,
    /// Per-client write rate limits (audit CR-H04).
    rate_limits: Arc<Mutex<HashMap<String, RateBucket>>>,
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

fn client_key(headers: &HeaderMap, peer: Option<&ConnectInfo<SocketAddr>>) -> String {
    if let Some(forwarded) = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return format!("xff:{forwarded}");
    }
    if let Some(ConnectInfo(addr)) = peer {
        return format!("ip:{}", addr.ip());
    }
    "ip:unknown".to_owned()
}

fn require_write_auth(state: &RpcState, headers: &HeaderMap) -> Result<(), RpcError> {
    let Some(expected) = state.config.effective_api_token() else {
        return Ok(());
    };
    let provided = headers
        .get("x-alvenqis-api-token")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            headers
                .get(http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|value| {
                    let value = value.trim();
                    value
                        .strip_prefix("Bearer ")
                        .or_else(|| value.strip_prefix("bearer "))
                        .map(str::trim)
                        .map(str::to_owned)
                })
        });
    match provided {
        Some(token) if token == expected => Ok(()),
        _ => Err(RpcError::Unauthorized(
            "submit/mining requires Authorization: Bearer <token> or X-Alvenqis-Api-Token"
                .to_owned(),
        )),
    }
}

#[derive(Clone, Copy)]
enum WriteKind {
    Submit,
    MiningTemplate,
    MiningSubmit,
}

fn enforce_write_rate_limit(
    state: &RpcState,
    headers: &HeaderMap,
    peer: Option<&ConnectInfo<SocketAddr>>,
    kind: WriteKind,
) -> Result<(), RpcError> {
    let write_limit = state.config.write_rate_limit_per_minute;
    let template_limit = state.config.mining_template_rate_limit_per_minute;
    if write_limit == 0 && template_limit == 0 {
        return Ok(());
    }
    let key = client_key(headers, peer);
    let now = unix_seconds();
    let mut map = state
        .rate_limits
        .lock()
        .map_err(|_| RpcError::Config("rate limit lock poisoned".to_owned()))?;
    let bucket = map.entry(key).or_default();
    if now.saturating_sub(bucket.window_started_at) >= 60 {
        bucket.window_started_at = now;
        bucket.write_count = 0;
        bucket.template_count = 0;
    }
    match kind {
        WriteKind::Submit | WriteKind::MiningSubmit => {
            if write_limit > 0 {
                bucket.write_count = bucket.write_count.saturating_add(1);
                if bucket.write_count > write_limit {
                    return Err(RpcError::RateLimited(format!(
                        "write rate limit exceeded ({write_limit}/min)"
                    )));
                }
            }
        }
        WriteKind::MiningTemplate => {
            if template_limit > 0 {
                bucket.template_count = bucket.template_count.saturating_add(1);
                if bucket.template_count > template_limit {
                    return Err(RpcError::RateLimited(format!(
                        "mining template rate limit exceeded ({template_limit}/min)"
                    )));
                }
            }
            if write_limit > 0 {
                bucket.write_count = bucket.write_count.saturating_add(1);
                if bucket.write_count > write_limit {
                    return Err(RpcError::RateLimited(format!(
                        "write rate limit exceeded ({write_limit}/min)"
                    )));
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct LoadedChain {
    pub blocks: Arc<Vec<Block>>,
    pub chain: Arc<Chain>,
    pub height: Option<u64>,
    pub tip_hash: Option<String>,
    pub emitted_supply_atomic: u64,
    pub cumulative_work: Option<String>,
}

fn file_fingerprint(path: &FsPath) -> (u64, u64) {
    match fs::metadata(path) {
        Ok(meta) => {
            let len = meta.len();
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            (len, mtime)
        }
        Err(_) => (0, 0),
    }
}

fn index_file_fingerprint(index_data_path: &FsPath) -> (u64, u64) {
    file_fingerprint(&alvenqis_indexer::storage::index_file_path(index_data_path))
}

pub fn router(state: RpcState) -> Router {
    let configured_origins = state.config.effective_cors_origins();
    let cors = if configured_origins.contains(&"*") {
        CorsLayer::new().allow_origin(Any)
    } else {
        // Invalid origins are skipped rather than panicking the gateway process.
        let origins: Vec<HeaderValue> = configured_origins
            .into_iter()
            .filter_map(|origin| HeaderValue::from_str(origin).ok())
            .collect();
        if origins.is_empty() {
            // Restrictive default: same-origin / non-browser clients only.
            CorsLayer::new()
        } else {
            CorsLayer::new().allow_origin(origins)
        }
    }
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([CONTENT_TYPE]);

    let mut router = Router::new()
        .route("/health", get(health))
        .route("/network", get(network))
        .route("/status", get(status))
        .route("/sync/status", get(sync_status))
        .route("/chain/tip", get(chain_tip))
        .route("/chain/height", get(chain_height))
        .route("/addresses/:address", get(addresses))
        .route("/addresses/:address/balance", get(address_balance))
        .route("/addresses/:address/account", get(address_account))
        .route("/state", get(state_snapshot))
        .route("/supply", get(supply))
        .route("/blocks/latest", get(blocks_latest))
        .route("/blocks/:height", get(blocks_by_height))
        .route("/blocks/hash/:hash", get(blocks_by_hash))
        .route("/transactions/:hash", get(transactions_by_hash))
        .route("/mempool", get(mempool))
        .route("/mempool/status", get(mempool_status))
        .route("/indexer/status", get(indexer_status))
        .route("/indexer/overview", get(indexer_overview))
        .route("/indexer/summary", get(indexer_summary))
        .route("/indexer/blocks", get(indexer_blocks_page))
        .route("/indexer/blocks/latest", get(indexer_blocks_latest))
        .route("/indexer/blocks/hash/:hash", get(indexer_blocks_by_hash))
        .route("/indexer/blocks/:height", get(indexer_blocks_by_height))
        .route("/indexer/transactions", get(indexer_transactions_page))
        .route("/indexer/addresses", get(indexer_addresses_page))
        .route("/indexer/tx/:hash", get(indexer_transaction_by_hash))
        .route("/indexer/address/:address", get(indexer_address))
        // Read-only network telemetry; public nodes expose it so clients can
        // render peer and sync state regardless of the configured endpoint.
        .route("/p2p/status", get(p2p_status));
    if state.config.access_mode.allows_transaction_submission() {
        router = router.route("/transactions", post(submit_transaction));
    }
    if state.config.mining_endpoints_enabled() {
        router = router
            .route("/mining/template", get(mining_template))
            .route("/mining/submit", post(mining_submit));
    }
    if !state.config.explorer_static_path.trim().is_empty() {
        let root = PathBuf::from(&state.config.explorer_static_path);
        router = router.fallback_service(
            ServeDir::new(&root).fallback(ServeFile::new(root.join("index.html"))),
        );
    }
    router
        .layer(RequestBodyLimitLayer::new(
            state.config.max_request_body_bytes,
        ))
        .layer(cors)
        .with_state(state)
}

async fn p2p_status(State(state): State<RpcState>) -> Result<Json<P2pStatus>, RpcError> {
    let node_config = NetworkConfig::load_from_path(&state.node_config_path)?;
    let status = load_p2p_status(
        &runtime_dir_for_data_dir(FsPath::new(&state.config.chain_data_path)),
        &node_config,
    )?;
    Ok(Json(status))
}

#[derive(Debug, Deserialize)]
struct MiningTemplateQuery {
    miner_address: String,
}

async fn mining_template(
    State(state): State<RpcState>,
    headers: HeaderMap,
    peer: Option<ConnectInfo<SocketAddr>>,
    Query(query): Query<MiningTemplateQuery>,
) -> Result<Json<MiningTemplateResponse>, RpcError> {
    require_write_auth(&state, &headers)?;
    enforce_write_rate_limit(&state, &headers, peer.as_ref(), WriteKind::MiningTemplate)?;
    let chain_fingerprint =
        storage::chain_storage_fingerprint(FsPath::new(&state.config.chain_data_path));
    if let Some(response) = reusable_mining_template(
        &state,
        &query.miner_address,
        chain_fingerprint,
        unix_seconds(),
    )? {
        return Ok(Json(response));
    }

    // Different miners poll together after a new block. Parallel full-chain validation here
    // saturated the public mining RPC and made otherwise unrelated HTTP requests time out.
    let _build_guard = state.mining_template_build_lock.lock().await;
    let now = unix_seconds();
    let chain_fingerprint =
        storage::chain_storage_fingerprint(FsPath::new(&state.config.chain_data_path));
    if let Some(response) =
        reusable_mining_template(&state, &query.miner_address, chain_fingerprint, now)?
    {
        return Ok(Json(response));
    }

    // Disk/chain work is synchronous and can stall the async runtime if run inline
    // (observed: public /health 504 while a template request held the process).
    let node_config_path = state.node_config_path.clone();
    let chain_data_path = state.config.chain_data_path.clone();
    let mempool_data_path = state.config.mempool_data_path.clone();
    let miner_address = query.miner_address.clone();
    let max_txs = state.config.max_mempool_transactions;
    let template = tokio::task::spawn_blocking(move || {
        create_block_template(
            &node_config_path,
            FsPath::new(&chain_data_path),
            FsPath::new(&mempool_data_path),
            &miner_address,
            max_txs,
        )
    })
    .await
    .map_err(|error| RpcError::Config(format!("mining template task failed: {error}")))?
    .map_err(map_submission_error)?;
    let fresh = template.block;
    // Opening SQLite can create/checkpoint the WAL even when the logical tip is unchanged. Store
    // the post-build fingerprint; a real concurrent tip advance will simply make this work stale.
    let chain_fingerprint =
        storage::chain_storage_fingerprint(FsPath::new(&state.config.chain_data_path));
    let tip_hash = hash_to_hex(&fresh.header.previous_hash);
    let merkle = hash_to_hex(&fresh.header.merkle_root);

    // Store one stable nonce space per miner and chain tip.
    {
        let mut templates = state
            .mining_templates
            .lock()
            .map_err(|_| RpcError::Config("mining template store lock poisoned".to_owned()))?;
        templates.retain(|_, stored| stored.expires_at_unix_seconds > now);

        let expires_at_unix_seconds = now.saturating_add(MINING_TEMPLATE_TTL_SECONDS);
        let template_id = random_template_id();
        // Prefer per-miner template cap under load (audit CR-H05).
        let miner_templates = templates
            .values()
            .filter(|stored| {
                stored.miner_address == query.miner_address
                    && stored.chain_fingerprint == chain_fingerprint
            })
            .count();
        if miner_templates >= 8 {
            return Err(RpcError::BadRequest(
                "too many active mining templates for this miner_address; reuse or wait for expiry"
                    .to_owned(),
            ));
        }
        if templates.len() >= MAX_ACTIVE_MINING_TEMPLATES {
            return Err(RpcError::BadRequest(
                "too many active mining templates; retry after expiration".to_owned(),
            ));
        }
        templates.insert(
            template_id.clone(),
            StoredMiningTemplate {
                expires_at_unix_seconds,
                block: fresh.clone(),
                miner_address: query.miner_address.clone(),
                chain_fingerprint,
            },
        );

        Ok(Json(MiningTemplateResponse {
            protocol: MINING_PROTOCOL,
            template_id,
            expires_at_unix_seconds,
            version: fresh.header.version,
            network_id: fresh.header.network_id.clone(),
            height: fresh.header.height,
            previous_hash: tip_hash,
            merkle_root: merkle,
            base_fee_atomic: fresh.header.base_fee_atomic,
            timestamp: fresh.header.timestamp,
            difficulty_leading_zero_bits: fresh.header.difficulty_leading_zero_bits,
            nonce_start: 0,
            transactions: fresh.transactions,
        }))
    }
}

fn reusable_mining_template(
    state: &RpcState,
    miner_address: &str,
    chain_fingerprint: storage::ChainStorageFingerprint,
    now: u64,
) -> Result<Option<MiningTemplateResponse>, RpcError> {
    let mut templates = state
        .mining_templates
        .lock()
        .map_err(|_| RpcError::Config("mining template store lock poisoned".to_owned()))?;
    templates.retain(|_, stored| stored.expires_at_unix_seconds > now);
    let Some((template_id, stored)) = templates.iter().find(|(_, stored)| {
        stored.expires_at_unix_seconds
            > now.saturating_add(MINING_TEMPLATE_REUSE_MIN_REMAINING_SECONDS)
            && stored.miner_address == miner_address
            && stored.chain_fingerprint == chain_fingerprint
    }) else {
        return Ok(None);
    };

    let block = stored.block.clone();
    Ok(Some(MiningTemplateResponse {
        protocol: MINING_PROTOCOL,
        template_id: template_id.clone(),
        expires_at_unix_seconds: stored.expires_at_unix_seconds,
        version: block.header.version,
        network_id: block.header.network_id.clone(),
        height: block.header.height,
        previous_hash: hash_to_hex(&block.header.previous_hash),
        merkle_root: hash_to_hex(&block.header.merkle_root),
        base_fee_atomic: block.header.base_fee_atomic,
        timestamp: block.header.timestamp,
        difficulty_leading_zero_bits: block.header.difficulty_leading_zero_bits,
        nonce_start: 0,
        transactions: block.transactions,
    }))
}

async fn mining_submit(
    State(state): State<RpcState>,
    headers: HeaderMap,
    peer: Option<ConnectInfo<SocketAddr>>,
    Json(request): Json<MiningSubmitRequest>,
) -> Result<Json<MiningSubmitResponse>, RpcError> {
    require_write_auth(&state, &headers)?;
    enforce_write_rate_limit(&state, &headers, peer.as_ref(), WriteKind::MiningSubmit)?;
    if request.protocol != MINING_PROTOCOL {
        return Err(RpcError::BadRequest(format!(
            "unsupported mining protocol {}; expected {MINING_PROTOCOL}",
            request.protocol
        )));
    }
    let now = unix_seconds();
    let stored = {
        let mut templates = state
            .mining_templates
            .lock()
            .map_err(|_| RpcError::Config("mining template store lock poisoned".to_owned()))?;
        templates.retain(|_, item| item.expires_at_unix_seconds > now);
        templates.remove(&request.template_id)
    };
    let Some(stored) = stored else {
        return Ok(Json(MiningSubmitResponse {
            protocol: MINING_PROTOCOL,
            status: "stale",
            template_id: request.template_id,
            block_hash: request.block_hash,
            height: None,
            reason: Some("template is unknown or expired".to_owned()),
        }));
    };

    let mut candidate = stored.block;
    candidate.header.nonce = request.nonce;
    if !request.mix_hash.trim().is_empty() {
        candidate.header.mix_hash = alvenqis_core::Hash::from_hex(request.mix_hash.trim())
            .map_err(|e| RpcError::BadRequest(format!("invalid mix_hash: {e}")))?;
    }
    let computed_hash = hash_to_hex(&candidate.hash()?);
    if computed_hash != request.block_hash.to_ascii_lowercase() {
        return Ok(Json(MiningSubmitResponse {
            protocol: MINING_PROTOCOL,
            status: "rejected",
            template_id: request.template_id,
            block_hash: computed_hash,
            height: None,
            reason: Some(
                "submitted block_hash does not match FiroPoW final hash for nonce/mix_hash"
                    .to_owned(),
            ),
        }));
    }

    // Validation reads the full chain and may wait on SQLite. Keep it off the async executor so a
    // submitted block cannot make every template and health request time out.
    let node_config_path = state.node_config_path.clone();
    let chain_data_path = state.config.chain_data_path.clone();
    let mempool_data_path = state.config.mempool_data_path.clone();
    let submission = tokio::task::spawn_blocking(move || {
        submit_mined_block(
            &node_config_path,
            FsPath::new(&chain_data_path),
            FsPath::new(&mempool_data_path),
            &candidate,
        )
    })
    .await
    .map_err(|error| RpcError::Config(format!("mining submission task failed: {error}")))?;

    match submission {
        Ok(summary) => Ok(Json(MiningSubmitResponse {
            protocol: MINING_PROTOCOL,
            status: "accepted",
            template_id: request.template_id,
            block_hash: summary.block_hash,
            height: Some(summary.block_height),
            reason: None,
        })),
        Err(alvenqis_node::NodeError::Core(
            alvenqis_core::AlvenqisError::InvalidPreviousHash { .. },
        )) => Ok(Json(MiningSubmitResponse {
            protocol: MINING_PROTOCOL,
            status: "stale",
            template_id: request.template_id,
            block_hash: computed_hash,
            height: None,
            reason: Some("chain tip changed before submission".to_owned()),
        })),
        Err(alvenqis_node::NodeError::Core(
            alvenqis_core::AlvenqisError::InvalidHeight { expected, actual },
        )) if actual < expected => Ok(Json(MiningSubmitResponse {
            protocol: MINING_PROTOCOL,
            status: "stale",
            template_id: request.template_id,
            block_hash: computed_hash,
            height: None,
            reason: Some(format!(
                "chain tip advanced before submission (expected height {expected}, template height {actual})"
            )),
        })),
        Err(error) => Ok(Json(MiningSubmitResponse {
            protocol: MINING_PROTOCOL,
            status: "rejected",
            template_id: request.template_id,
            block_hash: computed_hash,
            height: None,
            reason: Some(error.to_string()),
        })),
    }
}

fn random_template_id() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

pub fn load_chain(state: &RpcState) -> RpcResult<LoadedChain> {
    let chain_path = FsPath::new(&state.config.chain_data_path);
    let fingerprint = storage::chain_storage_fingerprint(chain_path);

    // Hold the lock through a cache miss so concurrent dashboard requests do
    // not all replay and validate the complete chain after the same new block.
    let mut guard = state
        .chain_cache
        .lock()
        .map_err(|_| RpcError::Config("chain cache lock poisoned".to_owned()))?;
    if let Some(cached) = guard.as_ref() {
        if cached.fingerprint == fingerprint {
            return Ok(LoadedChain {
                blocks: Arc::clone(&cached.blocks),
                chain: Arc::clone(&cached.chain),
                height: cached.height,
                tip_hash: cached.tip_hash.clone(),
                emitted_supply_atomic: cached.emitted_supply_atomic,
                cumulative_work: cached.cumulative_work.clone(),
            });
        }
    }

    let blocks = storage::load_blocks(chain_path)?;
    if blocks.is_empty() {
        *guard = None;
        return Err(RpcError::Node(
            alvenqis_node::NodeError::ChainNotInitialized(storage::chain_file_path(chain_path)),
        ));
    }
    let chain = Chain::from_blocks(state.config.network, blocks.iter().cloned())?;
    let height = chain.height();
    let tip_hash = chain.tip_hash()?.map(|hash| hash_to_hex(&hash));
    let emitted_supply_atomic = chain.emitted_supply().as_atomic();
    let cumulative_work = chain.cumulative_work().ok().map(|work| work.to_string());
    let blocks = Arc::new(blocks);
    let chain = Arc::new(chain);
    *guard = Some(CachedChain {
        fingerprint,
        blocks: Arc::clone(&blocks),
        chain: Arc::clone(&chain),
        height,
        tip_hash: tip_hash.clone(),
        emitted_supply_atomic,
        cumulative_work: cumulative_work.clone(),
    });
    Ok(LoadedChain {
        blocks,
        chain,
        height,
        tip_hash,
        emitted_supply_atomic,
        cumulative_work,
    })
}

fn load_cached_index(state: &RpcState) -> RpcResult<CachedIndex> {
    // The dedicated indexer writes atomically; request handlers only read/cache.
    let index_path = FsPath::new(&state.config.indexer_data_path);
    let fingerprint = index_file_fingerprint(index_path);
    let mut guard = state
        .index_cache
        .lock()
        .map_err(|_| RpcError::Config("index cache lock poisoned".to_owned()))?;
    if let Some(cached) = guard.as_ref() {
        if cached.fingerprint == fingerprint {
            return Ok(cached.clone());
        }
    }

    let data = Arc::new(load_index_snapshot(index_path)?);
    let mut transactions: Vec<IndexedTransaction> =
        data.transactions_by_hash.values().cloned().collect();
    transactions.sort_by(|left, right| {
        right
            .block_height
            .cmp(&left.block_height)
            .then_with(|| right.transaction_index.cmp(&left.transaction_index))
            .then_with(|| right.hash.cmp(&left.hash))
    });
    let mut addresses: Vec<AddressActivity> = data.addresses.values().cloned().collect();
    addresses.sort_by(|left, right| {
        right
            .balance_atomic
            .cmp(&left.balance_atomic)
            .then_with(|| left.address.cmp(&right.address))
    });
    let cached = CachedIndex {
        fingerprint,
        data: Arc::clone(&data),
        transactions: Arc::new(transactions),
        addresses: Arc::new(addresses),
    };
    *guard = Some(cached.clone());
    Ok(cached)
}

pub fn load_index_data(state: &RpcState) -> RpcResult<Arc<IndexData>> {
    Ok(load_cached_index(state)?.data)
}

fn cached_indexer_status(
    state: &RpcState,
    chain_height: Option<u64>,
    chain_tip_hash: Option<String>,
) -> RpcResult<IndexerStatus> {
    let index_dir = FsPath::new(&state.config.indexer_data_path);
    let mut status = match load_index_data(state) {
        Ok(index) => {
            let in_sync = chain_tip_hash.is_some()
                && index.summary.tip_hash == chain_tip_hash
                && index.summary.indexed_height == chain_height;
            let lag_blocks = match (chain_height, index.summary.indexed_height) {
                (Some(chain), Some(indexed)) if chain > indexed => chain - indexed,
                (Some(chain), None) => chain.saturating_add(1),
                _ => 0,
            };
            IndexerStatus {
                mode: INDEXER_MODE.to_owned(),
                network_id: Some(index.summary.network.clone()),
                status_label: Some(index.summary.status.clone()),
                initialized: true,
                index_dir: index_dir.display().to_string(),
                indexed_height: index.summary.indexed_height,
                indexed_block_count: index.summary.indexed_block_count,
                transaction_count: index.summary.transaction_count,
                address_count: index.summary.address_count,
                tip_hash: index.summary.tip_hash.clone(),
                chain_height,
                chain_tip_hash,
                in_sync,
                lag_blocks,
            }
        }
        Err(RpcError::Indexer(alvenqis_indexer::IndexerError::IndexNotInitialized(_))) => {
            IndexerStatus {
                mode: INDEXER_MODE.to_owned(),
                network_id: None,
                status_label: None,
                initialized: false,
                index_dir: index_dir.display().to_string(),
                indexed_height: None,
                indexed_block_count: 0,
                transaction_count: 0,
                address_count: 0,
                tip_hash: None,
                chain_height,
                chain_tip_hash,
                in_sync: false,
                lag_blocks: chain_height
                    .map(|height| height.saturating_add(1))
                    .unwrap_or(0),
            }
        }
        Err(error) => return Err(error),
    };
    if state.config.access_mode != crate::config::RpcAccessMode::Local {
        status.index_dir = "redacted".to_owned();
    }
    Ok(status)
}

fn load_mempool_transactions(
    state: &RpcState,
) -> RpcResult<Vec<alvenqis_node::PendingTransactionRecord>> {
    load_pending_transactions(FsPath::new(&state.config.mempool_data_path)).map_err(Into::into)
}

async fn health(State(state): State<RpcState>) -> Json<HealthResponse> {
    let exposure = match state.config.access_mode {
        crate::config::RpcAccessMode::Local => "Local only",
        crate::config::RpcAccessMode::PublicRead => "Public read",
        crate::config::RpcAccessMode::PublicSubmit => {
            if state.config.mining_endpoints_enabled() {
                "Public submit + mining (loopback/proxy-deny required)"
            } else {
                "Public submit (mining disabled)"
            }
        }
        crate::config::RpcAccessMode::PrivateMining => {
            "Private container-network mining (no published host port)"
        }
    };
    Json(HealthResponse {
        ok: true,
        service: "alvenqis-rpc-gateway",
        mode: format!("{} / {exposure}", state.config.status_label),
        network_id: state.config.network_id.clone(),
        network_name: state.config.human_name.clone(),
        status_label: state.config.status_label.clone(),
    })
}

async fn network(State(state): State<RpcState>) -> Json<NetworkResponse> {
    let protocol = alvenqis_core::launch_protocol_parameters(state.config.network);
    Json(NetworkResponse {
        protocol_parameters_id: protocol.parameters_id,
        protocol_version: protocol.protocol_version,
        block_version: protocol.block_version,
        network_id: state.config.network_id.clone(),
        network_name: state.config.human_name.clone(),
        status_label: state.config.status_label.clone(),
        ticker: protocol.ticker,
        address_prefix: protocol.address_prefix.to_owned(),
        address_standard_id: alvenqis_core::launch_address_standard(state.config.network)
            .standard_id,
        address_encoding: alvenqis_core::launch_address_standard(state.config.network).encoding,
        address_checksum_rule: alvenqis_core::launch_address_standard(state.config.network)
            .checksum_rule,
        address_payload_version: alvenqis_core::launch_address_standard(state.config.network)
            .payload_version,
        public_key_scheme: alvenqis_core::launch_signing_standard().public_key_scheme,
        signature_standard_id: alvenqis_core::launch_signing_standard().standard_id,
        signature_scheme: alvenqis_core::launch_signing_standard().signature_scheme,
        tx_signing_domain: alvenqis_core::launch_signing_standard().tx_signing_domain,
        key_derivation_policy_id: alvenqis_core::launch_key_derivation_policy().policy_id,
        block_time_seconds: protocol.block_time_seconds,
        decimals: protocol.decimals,
        atomic_units_per_alve: protocol.atomic_units_per_alve,
        max_supply_atomic: protocol.max_supply_atomic,
        halving_interval_blocks: protocol.halving_interval_blocks,
        initial_block_reward_atomic: protocol.initial_block_reward_atomic,
        pow_hash_algorithm: protocol.pow_hash_algorithm,
        difficulty_adjustment_algorithm: protocol.difficulty_adjustment_algorithm,
        fee_policy: protocol.fee_policy,
        default_rpc_port: protocol.default_rpc_port,
        default_p2p_port: protocol.default_p2p_port,
        max_transactions_per_block: protocol.max_transactions_per_block,
        max_transaction_wire_bytes: protocol.max_transaction_wire_bytes,
        median_time_past_window: protocol.median_time_past_window,
        max_future_block_drift_seconds: protocol.max_future_block_drift_seconds,
        first_account_nonce: alvenqis_core::FIRST_ACCOUNT_NONCE,
    })
}

async fn status(State(state): State<RpcState>) -> Result<Json<StatusResponse>, RpcError> {
    match load_chain(&state) {
        Ok(loaded) => {
            let idx = cached_indexer_status(&state, loaded.height, loaded.tip_hash.clone()).ok();
            Ok(Json(StatusResponse {
                network_id: state.config.network_id.clone(),
                network_name: state.config.human_name.clone(),
                status_label: state.config.status_label.clone(),
                initialized: true,
                block_count: loaded.blocks.len(),
                height: loaded.height,
                tip_hash: loaded.tip_hash,
                emitted_supply_atomic: Some(loaded.emitted_supply_atomic),
                index_tip_hash: idx.as_ref().and_then(|s| s.tip_hash.clone()),
                index_height: idx.as_ref().and_then(|s| s.indexed_height),
                index_in_sync: idx.as_ref().map(|s| s.in_sync).unwrap_or(false),
                index_lag_blocks: idx.as_ref().map(|s| s.lag_blocks).unwrap_or(0),
                cumulative_work: loaded.cumulative_work,
            }))
        }
        Err(RpcError::Node(alvenqis_node::NodeError::ChainNotInitialized(_))) => {
            Ok(Json(StatusResponse {
                network_id: state.config.network_id.clone(),
                network_name: state.config.human_name.clone(),
                status_label: state.config.status_label.clone(),
                initialized: false,
                block_count: 0,
                height: None,
                tip_hash: None,
                emitted_supply_atomic: None,
                index_tip_hash: None,
                index_height: None,
                index_in_sync: false,
                index_lag_blocks: 0,
                cumulative_work: None,
            }))
        }
        Err(error) => Err(error),
    }
}

async fn sync_status(State(state): State<RpcState>) -> Result<Json<SyncStatusResponse>, RpcError> {
    let local_height = match load_chain(&state) {
        Ok(loaded) => loaded.height,
        Err(RpcError::Node(alvenqis_node::NodeError::ChainNotInitialized(_))) => None,
        Err(error) => return Err(error),
    };
    let node_config = NetworkConfig::load_from_path(&state.node_config_path)?;
    let p2p = load_p2p_status(
        &runtime_dir_for_data_dir(FsPath::new(&state.config.chain_data_path)),
        &node_config,
    )?;
    let peer_height = p2p
        .peers
        .iter()
        .filter(|peer| peer.handshake_validated)
        .filter_map(|peer| peer.best_height)
        .max();

    let Some(local_height) = local_height else {
        return Ok(Json(SyncStatusResponse {
            network_id: state.config.network_id.clone(),
            sync_state: "uninitialized",
            local_height: None,
            network_height: peer_height,
            remaining_blocks: None,
            progress_percent: None,
            connected_peer_count: p2p.connected_peer_count,
            validated_peer_count: p2p.validated_peer_count,
            detail: "Local chain is not initialized",
        }));
    };

    let Some(peer_height) = peer_height else {
        return Ok(Json(SyncStatusResponse {
            network_id: state.config.network_id.clone(),
            sync_state: "discovering",
            local_height: Some(local_height),
            network_height: None,
            remaining_blocks: None,
            progress_percent: None,
            connected_peer_count: p2p.connected_peer_count,
            validated_peer_count: p2p.validated_peer_count,
            detail: "Waiting for a validated peer to report network height",
        }));
    };

    let network_height = local_height.max(peer_height);
    let remaining_blocks = network_height.saturating_sub(local_height);
    let progress_percent = if network_height == 0 {
        100.0
    } else {
        local_height as f64 / network_height as f64 * 100.0
    };
    let (sync_state, detail) = if remaining_blocks == 0 {
        ("synced", "Local chain matches validated peers")
    } else {
        ("syncing", "Downloading and validating blocks")
    };

    Ok(Json(SyncStatusResponse {
        network_id: state.config.network_id.clone(),
        sync_state,
        local_height: Some(local_height),
        network_height: Some(network_height),
        remaining_blocks: Some(remaining_blocks),
        progress_percent: Some(progress_percent),
        connected_peer_count: p2p.connected_peer_count,
        validated_peer_count: p2p.validated_peer_count,
        detail,
    }))
}

async fn chain_tip(State(state): State<RpcState>) -> Result<Json<ChainTipResponse>, RpcError> {
    let loaded = load_chain(&state)?;
    let height = loaded
        .height
        .ok_or_else(|| RpcError::NotFound("no chain tip available".to_owned()))?;
    let hash = loaded
        .tip_hash
        .ok_or_else(|| RpcError::NotFound("no chain tip available".to_owned()))?;
    Ok(Json(ChainTipResponse { height, hash }))
}

async fn chain_height(
    State(state): State<RpcState>,
) -> Result<Json<ChainHeightResponse>, RpcError> {
    let loaded = load_chain(&state)?;
    let height = loaded
        .height
        .ok_or_else(|| RpcError::NotFound("no chain height available".to_owned()))?;
    Ok(Json(ChainHeightResponse { height }))
}

async fn blocks_latest(State(state): State<RpcState>) -> Result<Json<BlockResponse>, RpcError> {
    let loaded = load_chain(&state)?;
    let block = loaded
        .blocks
        .last()
        .ok_or_else(|| RpcError::NotFound("latest block not found".to_owned()))?;
    Ok(Json(block_response(block)?))
}

async fn addresses(
    State(state): State<RpcState>,
    Path(address): Path<String>,
) -> Result<Json<AddressResponse>, RpcError> {
    let loaded = load_chain(&state)?;
    Ok(Json(address_response(&loaded.chain, &address)))
}

async fn address_balance(
    State(state): State<RpcState>,
    Path(address): Path<String>,
) -> Result<Json<AddressBalanceResponse>, RpcError> {
    let loaded = load_chain(&state)?;
    Ok(Json(address_balance_response(&loaded.chain, &address)))
}

async fn address_account(
    State(state): State<RpcState>,
    Path(address): Path<String>,
) -> Result<Json<AddressAccountResponse>, RpcError> {
    let loaded = load_chain(&state)?;
    let pending = load_mempool_transactions(&state).unwrap_or_default();
    let mempool_nonces = pending.iter().filter_map(|record| {
        let from = record.transaction.from.as_deref()?;
        if from == address.as_str() {
            Some(record.transaction.nonce)
        } else {
            None
        }
    });
    let anticipated_base_fee = next_base_fee(loaded.blocks.last()).as_atomic();
    Ok(Json(address_account_response(
        &loaded.chain,
        &address,
        mempool_nonces,
        anticipated_base_fee,
    )?))
}

async fn state_snapshot(State(state): State<RpcState>) -> Result<Json<StateResponse>, RpcError> {
    let loaded = load_chain(&state)?;
    Ok(Json(state_response(&loaded.chain)?))
}

async fn supply(State(state): State<RpcState>) -> Result<Json<SupplyResponse>, RpcError> {
    let loaded = load_chain(&state)?;
    Ok(Json(supply_response(&loaded.chain)))
}

async fn blocks_by_height(
    State(state): State<RpcState>,
    Path(height): Path<u64>,
) -> Result<Json<BlockResponse>, RpcError> {
    let loaded = load_chain(&state)?;
    let block = loaded
        .blocks
        .iter()
        .find(|block| block.header.height == height)
        .ok_or_else(|| RpcError::NotFound(format!("block at height {height} not found")))?;
    Ok(Json(block_response(block)?))
}

async fn blocks_by_hash(
    State(state): State<RpcState>,
    Path(hash): Path<String>,
) -> Result<Json<BlockResponse>, RpcError> {
    let loaded = load_chain(&state)?;
    let block = loaded
        .blocks
        .iter()
        .find(|block| block.hash().ok().is_some_and(|h| hash_to_hex(&h) == hash))
        .ok_or_else(|| RpcError::NotFound(format!("block with hash {hash} not found")))?;
    Ok(Json(block_response(block)?))
}

async fn transactions_by_hash(
    State(state): State<RpcState>,
    Path(hash): Path<String>,
) -> Result<Json<TransactionResponse>, RpcError> {
    let loaded = load_chain(&state)?;
    let anticipated_base_fee = next_base_fee(loaded.blocks.last());
    let pending_transactions = load_mempool_transactions(&state)?;
    if let Some(record) = pending_transactions
        .iter()
        .find(|record| record.tx_hash == hash)
    {
        return Ok(Json(transaction_response(
            &record.transaction,
            "pending",
            None,
            None,
            anticipated_base_fee,
        )));
    }

    for block in loaded.blocks.iter() {
        let block_hash = hash_to_hex(&block.hash()?);
        if let Some(transaction) = block
            .transactions
            .iter()
            .find(|transaction| hash_to_hex(&transaction.tx_hash()) == hash)
        {
            return Ok(Json(transaction_response(
                transaction,
                "mined",
                Some(block.header.height),
                Some(&block_hash),
                alvenqis_core::Amount::from_atomic(block.header.base_fee_atomic),
            )));
        }
    }

    Err(RpcError::NotFound(format!(
        "transaction with hash {hash} not found"
    )))
}

async fn submit_transaction(
    State(state): State<RpcState>,
    headers: HeaderMap,
    peer: Option<ConnectInfo<SocketAddr>>,
    Json(transaction): Json<Transaction>,
) -> Result<Json<SubmitTransactionResponse>, RpcError> {
    require_write_auth(&state, &headers)?;
    enforce_write_rate_limit(&state, &headers, peer.as_ref(), WriteKind::Submit)?;
    let summary = submit_pending_transaction(
        FsPath::new(&state.config.chain_data_path),
        FsPath::new(&state.config.mempool_data_path),
        state.config.max_mempool_transactions,
        &transaction,
    )
    .map_err(map_submission_error)?;
    Ok(Json(SubmitTransactionResponse {
        status: summary.status,
        tx_hash: summary.tx_hash,
        lifecycle_status: summary.lifecycle_status,
        mempool_size: summary.mempool_size,
    }))
}

async fn mempool(State(state): State<RpcState>) -> Result<Json<MempoolResponse>, RpcError> {
    let loaded = load_chain(&state)?;
    let anticipated_base_fee = next_base_fee(loaded.blocks.last());
    let summary = load_mempool_status(
        FsPath::new(&state.config.chain_data_path),
        FsPath::new(&state.config.mempool_data_path),
    )
    .map_err(map_submission_error)?;
    let transactions = load_mempool_transactions(&state)?
        .into_iter()
        .map(|record| {
            transaction_response(
                &record.transaction,
                "pending",
                None,
                None,
                anticipated_base_fee,
            )
        })
        .collect();
    Ok(Json(MempoolResponse {
        status: summary.status,
        pending_count: summary.pending_count,
        anticipated_base_fee_atomic: summary.anticipated_base_fee_atomic,
        total_fees_atomic: summary.total_fees_atomic,
        total_burned_fees_atomic: summary.total_burned_fees_atomic,
        total_priority_fees_atomic: summary.total_priority_fees_atomic,
        highest_priority_fee_atomic: summary.highest_priority_fee_atomic,
        highest_max_fee_atomic: summary.highest_max_fee_atomic,
        transactions,
    }))
}

async fn mempool_status(
    State(state): State<RpcState>,
) -> Result<Json<MempoolStatusResponse>, RpcError> {
    let summary = load_mempool_status(
        FsPath::new(&state.config.chain_data_path),
        FsPath::new(&state.config.mempool_data_path),
    )
    .map_err(map_submission_error)?;
    Ok(Json(MempoolStatusResponse {
        status: summary.status,
        pending_count: summary.pending_count,
        anticipated_base_fee_atomic: summary.anticipated_base_fee_atomic,
        total_fees_atomic: summary.total_fees_atomic,
        total_burned_fees_atomic: summary.total_burned_fees_atomic,
        total_priority_fees_atomic: summary.total_priority_fees_atomic,
        highest_priority_fee_atomic: summary.highest_priority_fee_atomic,
        highest_max_fee_atomic: summary.highest_max_fee_atomic,
    }))
}

async fn indexer_status(State(state): State<RpcState>) -> Result<Json<IndexerStatus>, RpcError> {
    let (chain_height, chain_tip_hash) = match load_chain(&state) {
        Ok(loaded) => (loaded.height, loaded.tip_hash),
        Err(RpcError::Node(alvenqis_node::NodeError::ChainNotInitialized(_))) => (None, None),
        Err(error) => return Err(error),
    };
    Ok(Json(cached_indexer_status(
        &state,
        chain_height,
        chain_tip_hash,
    )?))
}

async fn indexer_summary(State(state): State<RpcState>) -> Result<Json<IndexData>, RpcError> {
    Ok(Json((*load_index_data(&state)?).clone()))
}

#[derive(Debug, Deserialize)]
struct IndexerOverviewQuery {
    blocks: Option<usize>,
    transactions: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct IndexerPageQuery {
    offset: Option<usize>,
    limit: Option<usize>,
}

fn page_bounds(query: IndexerPageQuery) -> (usize, usize) {
    (
        query.offset.unwrap_or(0),
        query.limit.unwrap_or(20).clamp(1, 100),
    )
}

async fn indexer_overview(
    State(state): State<RpcState>,
    Query(query): Query<IndexerOverviewQuery>,
) -> Result<Json<IndexerOverviewResponse>, RpcError> {
    let cached = load_cached_index(&state)?;
    let index = &cached.data;
    let block_limit = query.blocks.unwrap_or(12).clamp(1, 100);
    let transaction_limit = query.transactions.unwrap_or(20).clamp(1, 200);
    let recent_blocks = index
        .blocks_by_height
        .values()
        .rev()
        .take(block_limit)
        .cloned()
        .collect();
    let recent_transactions = cached
        .transactions
        .iter()
        .take(transaction_limit)
        .cloned()
        .collect();
    Ok(Json(IndexerOverviewResponse {
        summary: index.summary.clone(),
        recent_blocks,
        recent_transactions,
    }))
}

async fn indexer_blocks_page(
    State(state): State<RpcState>,
    Query(query): Query<IndexerPageQuery>,
) -> Result<Json<IndexedBlocksPageResponse>, RpcError> {
    let index = load_index_data(&state)?;
    let (offset, limit) = page_bounds(query);
    let items = index
        .blocks_by_height
        .values()
        .rev()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();
    Ok(Json(IndexedBlocksPageResponse {
        total: index.blocks_by_height.len(),
        offset,
        limit,
        items,
    }))
}

async fn indexer_transactions_page(
    State(state): State<RpcState>,
    Query(query): Query<IndexerPageQuery>,
) -> Result<Json<IndexedTransactionsPageResponse>, RpcError> {
    let cached = load_cached_index(&state)?;
    let (offset, limit) = page_bounds(query);
    let total = cached.transactions.len();
    let items = cached
        .transactions
        .iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();
    Ok(Json(IndexedTransactionsPageResponse {
        total,
        offset,
        limit,
        items,
    }))
}

async fn indexer_addresses_page(
    State(state): State<RpcState>,
    Query(query): Query<IndexerPageQuery>,
) -> Result<Json<IndexedAddressesPageResponse>, RpcError> {
    let cached = load_cached_index(&state)?;
    let (offset, limit) = page_bounds(query);
    let total = cached.addresses.len();
    let items = cached
        .addresses
        .iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();
    Ok(Json(IndexedAddressesPageResponse {
        total,
        offset,
        limit,
        items,
    }))
}

async fn indexer_blocks_latest(
    State(state): State<RpcState>,
) -> Result<Json<IndexedBlock>, RpcError> {
    let index = load_index_data(&state)?;
    let block = index
        .summary
        .indexed_height
        .and_then(|height| index.blocks_by_height.get(&height))
        .cloned()
        .ok_or_else(|| RpcError::NotFound("no indexed block available".to_owned()))?;
    Ok(Json(block))
}

async fn indexer_blocks_by_height(
    State(state): State<RpcState>,
    Path(height): Path<u64>,
) -> Result<Json<IndexedBlock>, RpcError> {
    let index = load_index_data(&state)?;
    let block = index
        .blocks_by_height
        .get(&height)
        .cloned()
        .ok_or_else(|| RpcError::NotFound(format!("block at height {height} not found")))?;
    Ok(Json(block))
}

async fn indexer_blocks_by_hash(
    State(state): State<RpcState>,
    Path(hash): Path<String>,
) -> Result<Json<IndexedBlock>, RpcError> {
    let index = load_index_data(&state)?;
    let block = index
        .blocks_by_hash
        .get(&hash)
        .cloned()
        .ok_or_else(|| RpcError::NotFound(format!("block with hash {hash} not found")))?;
    Ok(Json(block))
}

async fn indexer_transaction_by_hash(
    State(state): State<RpcState>,
    Path(hash): Path<String>,
) -> Result<Json<IndexedTransaction>, RpcError> {
    let index = load_index_data(&state)?;
    let transaction = index
        .transactions_by_hash
        .get(&hash)
        .cloned()
        .ok_or_else(|| RpcError::NotFound(format!("transaction with hash {hash} not found")))?;
    Ok(Json(transaction))
}

async fn indexer_address(
    State(state): State<RpcState>,
    Path(address): Path<String>,
) -> Result<Json<AddressActivity>, RpcError> {
    let index = load_index_data(&state)?;
    let activity = index
        .addresses
        .get(&address)
        .cloned()
        .ok_or_else(|| RpcError::NotFound(format!("address {address} not found in index")))?;
    Ok(Json(activity))
}

fn map_submission_error(error: alvenqis_node::NodeError) -> RpcError {
    match error {
        alvenqis_node::NodeError::Input(message) => RpcError::BadRequest(message),
        alvenqis_node::NodeError::Core(core_error) => RpcError::BadRequest(core_error.to_string()),
        other => RpcError::Node(other),
    }
}
