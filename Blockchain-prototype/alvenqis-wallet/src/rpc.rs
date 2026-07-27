use crate::error::{WalletError, WalletResult};
use alvenqis_core::Transaction;
use alvenqis_sdk_rust::{BlockingRpcClient, NetworkConfig, SdkError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcBalanceResponse {
    pub address: String,
    pub balance_atomic: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcSubmitTransactionResponse {
    pub status: String,
    pub tx_hash: String,
    pub lifecycle_status: String,
    pub mempool_size: usize,
}

pub fn fetch_balance(rpc_base_url: &str, address: &str) -> WalletResult<RpcBalanceResponse> {
    let client = client_for_url(rpc_base_url)?;
    let balance = client.balance(address).map_err(map_sdk_error)?;
    Ok(RpcBalanceResponse {
        address: balance.address,
        balance_atomic: balance.balance_atomic,
    })
}

pub fn submit_transaction(
    rpc_base_url: &str,
    transaction: &Transaction,
) -> WalletResult<RpcSubmitTransactionResponse> {
    let client = client_for_url(rpc_base_url)?;
    let response = client.submit(transaction).map_err(map_sdk_error)?;
    Ok(RpcSubmitTransactionResponse {
        status: response.status,
        tx_hash: response.tx_hash,
        lifecycle_status: response.lifecycle_status,
        mempool_size: response.mempool_size,
    })
}

fn client_for_url(rpc_base_url: &str) -> WalletResult<BlockingRpcClient> {
    // Wallet CLI may target any network port; Network tag only affects SDK labels on config.
    // SDK Network is independent of alvenqis-core (standalone crate); label only.
    let config =
        NetworkConfig::with_rpc(alvenqis_sdk_rust::Network::MainnetCandidate, rpc_base_url);
    BlockingRpcClient::new(config).map_err(map_sdk_error)
}

fn map_sdk_error(error: SdkError) -> WalletError {
    match error {
        SdkError::RpcHttp { status, url, body } => WalletError::RpcUnavailable(format!(
            "RPC gateway returned status {status} for {url}: {body}"
        )),
        SdkError::Rpc(message) | SdkError::RpcDecode(message) => {
            WalletError::RpcUnavailable(message)
        }
        SdkError::Protocol(error) => WalletError::RpcUnavailable(error.to_string()),
        SdkError::Json(error) => WalletError::Json(error),
        other => WalletError::RpcUnavailable(other.to_string()),
    }
}
