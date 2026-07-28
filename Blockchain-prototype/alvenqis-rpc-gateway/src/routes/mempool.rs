use crate::error::RpcError;
use crate::models::{MempoolResponse, MempoolStatusResponse, SubmitTransactionResponse};
use crate::services;
use crate::state::RpcState;
use alvenqis_core::Transaction;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::Json;
use std::net::SocketAddr;

pub(crate) async fn submit_transaction(
    State(state): State<RpcState>,
    headers: HeaderMap,
    peer: Option<ConnectInfo<SocketAddr>>,
    Json(transaction): Json<Transaction>,
) -> Result<Json<SubmitTransactionResponse>, RpcError> {
    services::submit_transaction(State(state), headers, peer, Json(transaction)).await
}

pub(crate) async fn mempool(
    State(state): State<RpcState>,
) -> Result<Json<MempoolResponse>, RpcError> {
    services::mempool(State(state)).await
}

pub(crate) async fn mempool_status(
    State(state): State<RpcState>,
) -> Result<Json<MempoolStatusResponse>, RpcError> {
    services::mempool_status(State(state)).await
}
