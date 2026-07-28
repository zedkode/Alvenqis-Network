use crate::error::RpcError;
use crate::models::{AddressAccountResponse, AddressBalanceResponse, AddressResponse};
use crate::services;
use crate::state::RpcState;
use axum::extract::{Path, State};
use axum::Json;

pub(crate) async fn addresses(
    State(state): State<RpcState>,
    Path(address): Path<String>,
) -> Result<Json<AddressResponse>, RpcError> {
    services::addresses(State(state), Path(address)).await
}

pub(crate) async fn address_balance(
    State(state): State<RpcState>,
    Path(address): Path<String>,
) -> Result<Json<AddressBalanceResponse>, RpcError> {
    services::address_balance(State(state), Path(address)).await
}

pub(crate) async fn address_account(
    State(state): State<RpcState>,
    Path(address): Path<String>,
) -> Result<Json<AddressAccountResponse>, RpcError> {
    services::address_account(State(state), Path(address)).await
}
