use crate::cache::{load_chain_async, load_tip_block_async};
use crate::error::RpcError;
use crate::models::{
    address_account_response, address_balance_response, address_response, block_response,
    state_response, supply_response, transaction_response, AddressAccountResponse,
    AddressBalanceResponse, AddressResponse, BlockResponse, ChainHeightResponse, ChainTipResponse,
    StateResponse, StatusResponse, SupplyResponse, SyncStatusResponse, TransactionResponse,
};
use crate::services::{cached_indexer_status_async, load_mempool_transactions};
use crate::state::RpcState;
use alvenqis_core::{hash_to_hex, next_base_fee};
use alvenqis_node::{load_p2p_status, runtime_dir_for_data_dir, NetworkConfig};
use axum::extract::{Path, State};
use axum::Json;
use std::path::Path as FsPath;

pub(crate) async fn status(
    State(state): State<RpcState>,
) -> Result<Json<StatusResponse>, RpcError> {
    match load_chain_async(&state).await {
        Ok(loaded) => {
            let indexer_status =
                cached_indexer_status_async(&state, loaded.height, loaded.tip_hash.clone())
                    .await
                    .ok();
            Ok(Json(StatusResponse {
                network_id: state.config.network_id.clone(),
                network_name: state.config.human_name.clone(),
                status_label: state.config.status_label.clone(),
                initialized: true,
                block_count: loaded.blocks.len(),
                height: loaded.height,
                tip_hash: loaded.tip_hash,
                emitted_supply_atomic: Some(loaded.emitted_supply_atomic),
                index_tip_hash: indexer_status
                    .as_ref()
                    .and_then(|status| status.tip_hash.clone()),
                index_height: indexer_status
                    .as_ref()
                    .and_then(|status| status.indexed_height),
                index_in_sync: indexer_status
                    .as_ref()
                    .map(|status| status.in_sync)
                    .unwrap_or(false),
                index_lag_blocks: indexer_status
                    .as_ref()
                    .map(|status| status.lag_blocks)
                    .unwrap_or(0),
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

pub(crate) async fn sync_status(
    State(state): State<RpcState>,
) -> Result<Json<SyncStatusResponse>, RpcError> {
    let local_height = match load_chain_async(&state).await {
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

pub(crate) async fn chain_tip(
    State(state): State<RpcState>,
) -> Result<Json<ChainTipResponse>, RpcError> {
    let block = load_tip_block_async(&state)
        .await?
        .ok_or_else(|| RpcError::NotFound("no chain tip available".to_owned()))?;
    let height = block.header.height;
    let hash = hash_to_hex(&block.hash()?);
    Ok(Json(ChainTipResponse { height, hash }))
}

pub(crate) async fn chain_height(
    State(state): State<RpcState>,
) -> Result<Json<ChainHeightResponse>, RpcError> {
    let height = load_tip_block_async(&state)
        .await?
        .map(|block| block.header.height)
        .ok_or_else(|| RpcError::NotFound("no chain height available".to_owned()))?;
    Ok(Json(ChainHeightResponse { height }))
}

pub(crate) async fn blocks_latest(
    State(state): State<RpcState>,
) -> Result<Json<BlockResponse>, RpcError> {
    let block = load_tip_block_async(&state)
        .await?
        .ok_or_else(|| RpcError::NotFound("latest block not found".to_owned()))?;
    Ok(Json(block_response(&block)?))
}

pub(crate) async fn addresses(
    State(state): State<RpcState>,
    Path(address): Path<String>,
) -> Result<Json<AddressResponse>, RpcError> {
    let loaded = load_chain_async(&state).await?;
    Ok(Json(address_response(&loaded.chain, &address)))
}

pub(crate) async fn address_balance(
    State(state): State<RpcState>,
    Path(address): Path<String>,
) -> Result<Json<AddressBalanceResponse>, RpcError> {
    let loaded = load_chain_async(&state).await?;
    Ok(Json(address_balance_response(&loaded.chain, &address)))
}

pub(crate) async fn address_account(
    State(state): State<RpcState>,
    Path(address): Path<String>,
) -> Result<Json<AddressAccountResponse>, RpcError> {
    let loaded = load_chain_async(&state).await?;
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

pub(crate) async fn state_snapshot(
    State(state): State<RpcState>,
) -> Result<Json<StateResponse>, RpcError> {
    let loaded = load_chain_async(&state).await?;
    Ok(Json(state_response(&loaded.chain)?))
}

pub(crate) async fn supply(
    State(state): State<RpcState>,
) -> Result<Json<SupplyResponse>, RpcError> {
    let loaded = load_chain_async(&state).await?;
    Ok(Json(supply_response(&loaded.chain)))
}

pub(crate) async fn blocks_by_height(
    State(state): State<RpcState>,
    Path(height): Path<u64>,
) -> Result<Json<BlockResponse>, RpcError> {
    let loaded = load_chain_async(&state).await?;
    let block = loaded
        .blocks
        .iter()
        .find(|block| block.header.height == height)
        .ok_or_else(|| RpcError::NotFound(format!("block at height {height} not found")))?;
    Ok(Json(block_response(block)?))
}

pub(crate) async fn blocks_by_hash(
    State(state): State<RpcState>,
    Path(hash): Path<String>,
) -> Result<Json<BlockResponse>, RpcError> {
    let loaded = load_chain_async(&state).await?;
    let block = loaded
        .blocks
        .iter()
        .find(|block| {
            block
                .hash()
                .ok()
                .is_some_and(|block_hash| hash_to_hex(&block_hash) == hash)
        })
        .ok_or_else(|| RpcError::NotFound(format!("block with hash {hash} not found")))?;
    Ok(Json(block_response(block)?))
}

pub(crate) async fn transactions_by_hash(
    State(state): State<RpcState>,
    Path(hash): Path<String>,
) -> Result<Json<TransactionResponse>, RpcError> {
    let loaded = load_chain_async(&state).await?;
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
