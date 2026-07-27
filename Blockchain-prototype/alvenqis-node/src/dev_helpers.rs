use crate::error::{NodeError, NodeResult};
use alvenqis_core::{
    hash_to_hex, initial_base_fee, Address, Amount, Hash, Network, PrivateKey, Transaction,
};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct GeneratedDevAddress {
    pub status: String,
    pub warning: &'static str,
    pub network_id: String,
    pub address: String,
    pub public_key_hex: String,
    pub private_key_hex: String,
}

pub fn generate_dev_address(network: Network) -> GeneratedDevAddress {
    let private_key = PrivateKey::generate();
    let public_key = private_key.public_key();
    GeneratedDevAddress {
        status: format!("{} / Prototype", network.status_label()),
        warning: "Local development key material. Do not reuse for production or store in repo.",
        network_id: network.network_id().to_owned(),
        address: Address::from_public_key_for_network(&public_key, network).to_string(),
        public_key_hex: public_key.to_hex(),
        private_key_hex: private_key.to_hex(),
    }
}

pub fn default_miner_address(network: Network) -> String {
    let private_key = PrivateKey::from_bytes([7_u8; 32]);
    Address::from_public_key_for_network(&private_key.public_key(), network).to_string()
}

pub fn sign_dev_transaction(
    network: Network,
    private_key_hex: &str,
    to: &str,
    amount_atomic: u64,
    fee_atomic: u64,
    nonce: u64,
    memo_hash_hex: Option<&str>,
) -> NodeResult<Transaction> {
    let private_key = PrivateKey::from_hex(private_key_hex).map_err(node_input_error)?;
    Address::parse(to).map_err(node_input_error)?;
    let memo_hash = match memo_hash_hex {
        Some(value) => Some(Hash::from_hex(value).map_err(NodeError::Input)?),
        None => None,
    };

    Transaction::new_signed(
        1,
        nonce,
        network,
        &private_key,
        to.to_owned(),
        Amount::from_atomic(amount_atomic),
        Amount::from_atomic(initial_base_fee().as_atomic().saturating_add(fee_atomic)),
        Amount::from_atomic(fee_atomic),
        memo_hash,
    )
    .map_err(NodeError::from)
}

pub fn verify_dev_transaction(tx_file: &Path) -> NodeResult<Transaction> {
    let content = fs::read_to_string(tx_file)?;
    let transaction: Transaction = serde_json::from_str(&content)?;
    transaction.verify().map_err(NodeError::from)?;
    Ok(transaction)
}

pub fn format_verified_transaction(transaction: &Transaction) -> String {
    format!(
        "verified tx_hash={} from={} to={} signed={}",
        hash_to_hex(&transaction.tx_hash()),
        transaction.from.as_deref().unwrap_or("coinbase"),
        transaction.to,
        transaction.signature.is_some()
    )
}

fn node_input_error(error: alvenqis_core::AlvenqisError) -> NodeError {
    NodeError::Input(error.to_string())
}
