use crate::error::RpcError;
use crate::models::{
    IndexedAddressesPageResponse, IndexedBlocksPageResponse, IndexedTransactionsPageResponse,
    IndexerOverviewResponse,
};
use crate::services::{self, IndexerOverviewQuery, IndexerPageQuery};
use crate::state::RpcState;
use alvenqis_indexer::{
    AddressActivity, IndexData, IndexedBlock, IndexedTransaction, IndexerStatus,
};
use axum::extract::{Path, Query, State};
use axum::Json;

pub(crate) async fn indexer_status(
    State(state): State<RpcState>,
) -> Result<Json<IndexerStatus>, RpcError> {
    services::indexer_status(State(state)).await
}

pub(crate) async fn indexer_summary(
    State(state): State<RpcState>,
) -> Result<Json<IndexData>, RpcError> {
    services::indexer_summary(State(state)).await
}

pub(crate) async fn indexer_overview(
    State(state): State<RpcState>,
    Query(query): Query<IndexerOverviewQuery>,
) -> Result<Json<IndexerOverviewResponse>, RpcError> {
    services::indexer_overview(State(state), Query(query)).await
}

pub(crate) async fn indexer_blocks_page(
    State(state): State<RpcState>,
    Query(query): Query<IndexerPageQuery>,
) -> Result<Json<IndexedBlocksPageResponse>, RpcError> {
    services::indexer_blocks_page(State(state), Query(query)).await
}

pub(crate) async fn indexer_transactions_page(
    State(state): State<RpcState>,
    Query(query): Query<IndexerPageQuery>,
) -> Result<Json<IndexedTransactionsPageResponse>, RpcError> {
    services::indexer_transactions_page(State(state), Query(query)).await
}

pub(crate) async fn indexer_addresses_page(
    State(state): State<RpcState>,
    Query(query): Query<IndexerPageQuery>,
) -> Result<Json<IndexedAddressesPageResponse>, RpcError> {
    services::indexer_addresses_page(State(state), Query(query)).await
}

pub(crate) async fn indexer_blocks_latest(
    State(state): State<RpcState>,
) -> Result<Json<IndexedBlock>, RpcError> {
    services::indexer_blocks_latest(State(state)).await
}

pub(crate) async fn indexer_blocks_by_height(
    State(state): State<RpcState>,
    Path(height): Path<u64>,
) -> Result<Json<IndexedBlock>, RpcError> {
    services::indexer_blocks_by_height(State(state), Path(height)).await
}

pub(crate) async fn indexer_blocks_by_hash(
    State(state): State<RpcState>,
    Path(hash): Path<String>,
) -> Result<Json<IndexedBlock>, RpcError> {
    services::indexer_blocks_by_hash(State(state), Path(hash)).await
}

pub(crate) async fn indexer_transaction_by_hash(
    State(state): State<RpcState>,
    Path(hash): Path<String>,
) -> Result<Json<IndexedTransaction>, RpcError> {
    services::indexer_transaction_by_hash(State(state), Path(hash)).await
}

pub(crate) async fn indexer_address(
    State(state): State<RpcState>,
    Path(address): Path<String>,
) -> Result<Json<AddressActivity>, RpcError> {
    services::indexer_address(State(state), Path(address)).await
}
