use crate::error::RpcError;
use crate::middleware::{enforce_write_rate_limit, require_write_auth, WriteKind};
use crate::models::{MiningSubmitRequest, MiningSubmitResponse, MiningTemplateResponse};
use crate::services::map_submission_error;
use crate::state::RpcState;
use alvenqis_core::{hash_to_hex, Block};
use alvenqis_node::{create_block_template, storage, submit_mined_block};
use axum::extract::{ConnectInfo, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const MINING_PROTOCOL: &str = "alvenqis-mining-v1";
const MINING_TEMPLATE_TTL_SECONDS: u64 = 90;
/// Prefer reusing an existing template while it has at least this much life left.
/// Prevents miners from thrashing nonce search on a new timestamp every poll.
const MINING_TEMPLATE_REUSE_MIN_REMAINING_SECONDS: u64 = 25;
const MAX_ACTIVE_MINING_TEMPLATES: usize = 256;

#[derive(Clone, Debug)]
pub(crate) struct StoredMiningTemplate {
    expires_at_unix_seconds: u64,
    block: Block,
    miner_address: String,
    chain_fingerprint: storage::ChainStorageFingerprint,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MiningTemplateQuery {
    miner_address: String,
}

pub(crate) async fn mining_template(
    State(state): State<RpcState>,
    headers: HeaderMap,
    peer: Option<ConnectInfo<SocketAddr>>,
    Query(query): Query<MiningTemplateQuery>,
) -> Result<Json<MiningTemplateResponse>, RpcError> {
    require_write_auth(&state, &headers)?;
    enforce_write_rate_limit(&state, &headers, peer.as_ref(), WriteKind::MiningTemplate)?;
    let chain_fingerprint =
        storage::chain_storage_fingerprint(Path::new(&state.config.chain_data_path));
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
        storage::chain_storage_fingerprint(Path::new(&state.config.chain_data_path));
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
    let max_transactions = state.config.max_mempool_transactions;
    let template = tokio::task::spawn_blocking(move || {
        create_block_template(
            &node_config_path,
            Path::new(&chain_data_path),
            Path::new(&mempool_data_path),
            &miner_address,
            max_transactions,
        )
    })
    .await
    .map_err(|error| RpcError::Config(format!("mining template task failed: {error}")))?
    .map_err(map_submission_error)?;
    let fresh = template.block;
    // Opening SQLite can create/checkpoint the WAL even when the logical tip is unchanged. Store
    // the post-build fingerprint; a real concurrent tip advance will simply make this work stale.
    let chain_fingerprint =
        storage::chain_storage_fingerprint(Path::new(&state.config.chain_data_path));
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

pub(crate) async fn mining_submit(
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
            .map_err(|error| RpcError::BadRequest(format!("invalid mix_hash: {error}")))?;
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
            Path::new(&chain_data_path),
            Path::new(&mempool_data_path),
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
