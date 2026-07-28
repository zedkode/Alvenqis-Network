use crate::error::RpcError;
use crate::models::{
    BlockResponse, ChainHeightResponse, ChainTipResponse, StateResponse, StatusResponse,
    SupplyResponse, SyncStatusResponse, TransactionResponse,
};
use crate::services;
use crate::state::RpcState;
use axum::extract::{Path, State};
use axum::Json;

pub(crate) async fn status(
    State(state): State<RpcState>,
) -> Result<Json<StatusResponse>, RpcError> {
    services::status(State(state)).await
}

pub(crate) async fn sync_status(
    State(state): State<RpcState>,
) -> Result<Json<SyncStatusResponse>, RpcError> {
    services::sync_status(State(state)).await
}

pub(crate) async fn chain_tip(
    State(state): State<RpcState>,
) -> Result<Json<ChainTipResponse>, RpcError> {
    services::chain_tip(State(state)).await
}

pub(crate) async fn chain_height(
    State(state): State<RpcState>,
) -> Result<Json<ChainHeightResponse>, RpcError> {
    services::chain_height(State(state)).await
}

pub(crate) async fn blocks_latest(
    State(state): State<RpcState>,
) -> Result<Json<BlockResponse>, RpcError> {
    services::blocks_latest(State(state)).await
}

pub(crate) async fn state_snapshot(
    State(state): State<RpcState>,
) -> Result<Json<StateResponse>, RpcError> {
    services::state_snapshot(State(state)).await
}

pub(crate) async fn supply(
    State(state): State<RpcState>,
) -> Result<Json<SupplyResponse>, RpcError> {
    services::supply(State(state)).await
}

pub(crate) async fn blocks_by_height(
    State(state): State<RpcState>,
    Path(height): Path<u64>,
) -> Result<Json<BlockResponse>, RpcError> {
    services::blocks_by_height(State(state), Path(height)).await
}

pub(crate) async fn blocks_by_hash(
    State(state): State<RpcState>,
    Path(hash): Path<String>,
) -> Result<Json<BlockResponse>, RpcError> {
    services::blocks_by_hash(State(state), Path(hash)).await
}

pub(crate) async fn transactions_by_hash(
    State(state): State<RpcState>,
    Path(hash): Path<String>,
) -> Result<Json<TransactionResponse>, RpcError> {
    services::transactions_by_hash(State(state), Path(hash)).await
}
