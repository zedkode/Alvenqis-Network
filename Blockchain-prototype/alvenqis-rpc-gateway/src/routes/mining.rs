use crate::error::RpcError;
use crate::models::{MiningSubmitRequest, MiningSubmitResponse, MiningTemplateResponse};
use crate::services::{self, MiningTemplateQuery};
use crate::state::RpcState;
use axum::extract::{ConnectInfo, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use std::net::SocketAddr;

pub(crate) async fn mining_template(
    State(state): State<RpcState>,
    headers: HeaderMap,
    peer: Option<ConnectInfo<SocketAddr>>,
    Query(query): Query<MiningTemplateQuery>,
) -> Result<Json<MiningTemplateResponse>, RpcError> {
    services::mining_template(State(state), headers, peer, Query(query)).await
}

pub(crate) async fn mining_submit(
    State(state): State<RpcState>,
    headers: HeaderMap,
    peer: Option<ConnectInfo<SocketAddr>>,
    Json(request): Json<MiningSubmitRequest>,
) -> Result<Json<MiningSubmitResponse>, RpcError> {
    services::mining_submit(State(state), headers, peer, Json(request)).await
}
