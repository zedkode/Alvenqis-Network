use crate::cache::{
    load_cached_index_async, load_chain_async, load_index_data, load_index_data_async,
};
use crate::error::{RpcError, RpcResult};
use crate::models::{
    IndexedAddressesPageResponse, IndexedBlocksPageResponse, IndexedTransactionsPageResponse,
    IndexerOverviewResponse,
};
use crate::state::RpcState;
use alvenqis_indexer::{
    AddressActivity, IndexData, IndexedBlock, IndexedTransaction, IndexerStatus, INDEXER_MODE,
};
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use std::path::Path as FsPath;

pub(crate) async fn cached_indexer_status_async(
    state: &RpcState,
    chain_height: Option<u64>,
    chain_tip_hash: Option<String>,
) -> RpcResult<IndexerStatus> {
    let state = state.clone();
    tokio::task::spawn_blocking(move || cached_indexer_status(&state, chain_height, chain_tip_hash))
        .await
        .map_err(|error| RpcError::Config(format!("index status task failed: {error}")))?
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

pub(crate) async fn indexer_status(
    State(state): State<RpcState>,
) -> Result<Json<IndexerStatus>, RpcError> {
    let (chain_height, chain_tip_hash) = match load_chain_async(&state).await {
        Ok(loaded) => (loaded.height, loaded.tip_hash),
        Err(RpcError::Node(alvenqis_node::NodeError::ChainNotInitialized(_))) => (None, None),
        Err(error) => return Err(error),
    };
    Ok(Json(
        cached_indexer_status_async(&state, chain_height, chain_tip_hash).await?,
    ))
}

pub(crate) async fn indexer_summary(
    State(state): State<RpcState>,
) -> Result<Json<IndexData>, RpcError> {
    Ok(Json((*load_index_data_async(&state).await?).clone()))
}

#[derive(Debug, Deserialize)]
pub(crate) struct IndexerOverviewQuery {
    blocks: Option<usize>,
    transactions: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct IndexerPageQuery {
    offset: Option<usize>,
    limit: Option<usize>,
}

fn page_bounds(query: IndexerPageQuery) -> (usize, usize) {
    (
        query.offset.unwrap_or(0),
        query.limit.unwrap_or(20).clamp(1, 100),
    )
}

pub(crate) async fn indexer_overview(
    State(state): State<RpcState>,
    Query(query): Query<IndexerOverviewQuery>,
) -> Result<Json<IndexerOverviewResponse>, RpcError> {
    let cached = load_cached_index_async(&state).await?;
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

pub(crate) async fn indexer_blocks_page(
    State(state): State<RpcState>,
    Query(query): Query<IndexerPageQuery>,
) -> Result<Json<IndexedBlocksPageResponse>, RpcError> {
    let index = load_index_data_async(&state).await?;
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

pub(crate) async fn indexer_transactions_page(
    State(state): State<RpcState>,
    Query(query): Query<IndexerPageQuery>,
) -> Result<Json<IndexedTransactionsPageResponse>, RpcError> {
    let cached = load_cached_index_async(&state).await?;
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

pub(crate) async fn indexer_addresses_page(
    State(state): State<RpcState>,
    Query(query): Query<IndexerPageQuery>,
) -> Result<Json<IndexedAddressesPageResponse>, RpcError> {
    let cached = load_cached_index_async(&state).await?;
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

pub(crate) async fn indexer_blocks_latest(
    State(state): State<RpcState>,
) -> Result<Json<IndexedBlock>, RpcError> {
    let index = load_index_data_async(&state).await?;
    let block = index
        .summary
        .indexed_height
        .and_then(|height| index.blocks_by_height.get(&height))
        .cloned()
        .ok_or_else(|| RpcError::NotFound("no indexed block available".to_owned()))?;
    Ok(Json(block))
}

pub(crate) async fn indexer_blocks_by_height(
    State(state): State<RpcState>,
    Path(height): Path<u64>,
) -> Result<Json<IndexedBlock>, RpcError> {
    let index = load_index_data_async(&state).await?;
    let block = index
        .blocks_by_height
        .get(&height)
        .cloned()
        .ok_or_else(|| RpcError::NotFound(format!("block at height {height} not found")))?;
    Ok(Json(block))
}

pub(crate) async fn indexer_blocks_by_hash(
    State(state): State<RpcState>,
    Path(hash): Path<String>,
) -> Result<Json<IndexedBlock>, RpcError> {
    let index = load_index_data_async(&state).await?;
    let block = index
        .blocks_by_hash
        .get(&hash)
        .cloned()
        .ok_or_else(|| RpcError::NotFound(format!("block with hash {hash} not found")))?;
    Ok(Json(block))
}

pub(crate) async fn indexer_transaction_by_hash(
    State(state): State<RpcState>,
    Path(hash): Path<String>,
) -> Result<Json<IndexedTransaction>, RpcError> {
    let index = load_index_data_async(&state).await?;
    let transaction = index
        .transactions_by_hash
        .get(&hash)
        .cloned()
        .ok_or_else(|| RpcError::NotFound(format!("transaction with hash {hash} not found")))?;
    Ok(Json(transaction))
}

pub(crate) async fn indexer_address(
    State(state): State<RpcState>,
    Path(address): Path<String>,
) -> Result<Json<AddressActivity>, RpcError> {
    let index = load_index_data_async(&state).await?;
    let activity = index
        .addresses
        .get(&address)
        .cloned()
        .ok_or_else(|| RpcError::NotFound(format!("address {address} not found in index")))?;
    Ok(Json(activity))
}
