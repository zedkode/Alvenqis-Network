use crate::cache::load_chain_async;
use crate::error::{RpcError, RpcResult};
use crate::middleware::{enforce_write_rate_limit, require_write_auth, WriteKind};
use crate::models::{
    transaction_response, MempoolResponse, MempoolStatusResponse, SubmitTransactionResponse,
};
use crate::services::map_submission_error;
use crate::state::RpcState;
use alvenqis_core::{next_base_fee, Transaction};
use alvenqis_node::{
    load_pending_transactions, mempool_status as load_mempool_status,
    submit_transaction as submit_pending_transaction,
};
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::Json;
use std::net::SocketAddr;
use std::path::Path;

pub(crate) fn load_mempool_transactions(
    state: &RpcState,
) -> RpcResult<Vec<alvenqis_node::PendingTransactionRecord>> {
    load_pending_transactions(Path::new(&state.config.mempool_data_path)).map_err(Into::into)
}

pub(crate) async fn submit_transaction(
    State(state): State<RpcState>,
    headers: HeaderMap,
    peer: Option<ConnectInfo<SocketAddr>>,
    Json(transaction): Json<Transaction>,
) -> Result<Json<SubmitTransactionResponse>, RpcError> {
    require_write_auth(&state, &headers)?;
    enforce_write_rate_limit(&state, &headers, peer.as_ref(), WriteKind::Submit)?;
    let summary = submit_pending_transaction(
        Path::new(&state.config.chain_data_path),
        Path::new(&state.config.mempool_data_path),
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

pub(crate) async fn mempool(
    State(state): State<RpcState>,
) -> Result<Json<MempoolResponse>, RpcError> {
    let loaded = load_chain_async(&state).await?;
    let anticipated_base_fee = next_base_fee(loaded.blocks.last());
    let summary = load_mempool_status(
        Path::new(&state.config.chain_data_path),
        Path::new(&state.config.mempool_data_path),
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

pub(crate) async fn mempool_status(
    State(state): State<RpcState>,
) -> Result<Json<MempoolStatusResponse>, RpcError> {
    let summary = load_mempool_status(
        Path::new(&state.config.chain_data_path),
        Path::new(&state.config.mempool_data_path),
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
