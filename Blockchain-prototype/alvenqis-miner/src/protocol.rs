use crate::{MinerError, Result};
use alvenqis_core::{hash_to_hex, Address, Block, BlockHeader, Hash, Transaction};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MINING_PROTOCOL_VERSION: &str = "alvenqis-mining-v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MiningTemplate {
    pub protocol: String,
    pub template_id: String,
    pub expires_at_unix_seconds: u64,
    pub version: u32,
    pub network_id: String,
    pub height: u64,
    pub previous_hash: String,
    pub merkle_root: String,
    pub base_fee_atomic: u64,
    pub timestamp: u64,
    pub difficulty_leading_zero_bits: u8,
    #[serde(default)]
    pub share_difficulty_leading_zero_bits: Option<u8>,
    #[serde(default)]
    pub nonce_start: u64,
    pub transactions: Vec<Transaction>,
}

impl MiningTemplate {
    pub fn validate_and_build(&self, miner_address: &str) -> Result<Block> {
        if self.protocol != MINING_PROTOCOL_VERSION {
            let hint = if self.protocol.contains("veiron")
                || self.protocol.contains("vireon")
                || self.protocol.contains("vire")
            {
                " (legacy foreign identity — Alvenqis requires alvenqis-mining-v1)"
            } else {
                ""
            };
            return Err(MinerError::InvalidTemplate(format!(
                "unsupported protocol {}; expected {MINING_PROTOCOL_VERSION}{hint}",
                self.protocol
            )));
        }
        if self.template_id.trim().is_empty() {
            return Err(MinerError::InvalidTemplate(
                "template_id cannot be empty".to_owned(),
            ));
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        if self.expires_at_unix_seconds <= now {
            return Err(MinerError::InvalidTemplate(format!(
                "template expired at {} (now={now}); refresh work source",
                self.expires_at_unix_seconds
            )));
        }
        if alvenqis_core::Network::is_legacy_identity_token(&self.network_id) {
            return Err(MinerError::InvalidTemplate(format!(
                "rejected legacy/foreign network_id '{}'; Alvenqis accepts only alvenqis-mainnet-candidate / alvenqis-testnet / alvenqis-devnet",
                self.network_id
            )));
        }
        let network = alvenqis_core::Network::from_network_id(&self.network_id).ok_or_else(|| {
            MinerError::InvalidTemplate(format!(
                "unknown network_id '{}'; expected alvenqis-mainnet-candidate (or alvenqis-testnet / alvenqis-devnet)",
                self.network_id
            ))
        })?;
        let address = Address::parse(miner_address).map_err(|error| {
            MinerError::InvalidTemplate(format!(
                "invalid miner_address '{miner_address}': {error} (Mainnet Candidate uses alve1… Bech32m)"
            ))
        })?;
        if address.network() != network {
            return Err(MinerError::InvalidTemplate(format!(
                "miner address belongs to {}, template belongs to {} (use an {} address for this work)",
                address.network().network_id(),
                network.network_id(),
                network.address_prefix()
            )));
        }
        let coinbase = self.transactions.first().ok_or_else(|| {
            MinerError::InvalidTemplate("transactions must include coinbase".to_owned())
        })?;
        // Solo: coinbase must pay the configured miner. Pool path overrides via WorkSource.
        if !coinbase.is_coinbase() || coinbase.to != miner_address {
            return Err(MinerError::InvalidTemplate(format!(
                "first transaction must be coinbase paying miner_address (coinbase.to={}, miner_address={miner_address})",
                coinbase.to
            )));
        }

        let block = Block {
            header: BlockHeader {
                version: self.version,
                network_id: self.network_id.clone(),
                height: self.height,
                previous_hash: parse_hash("previous_hash", &self.previous_hash)?,
                merkle_root: parse_hash("merkle_root", &self.merkle_root)?,
                base_fee_atomic: self.base_fee_atomic,
                timestamp: self.timestamp,
                nonce: self.nonce_start,
                mix_hash: Hash::zero(),
                difficulty_leading_zero_bits: self.difficulty_leading_zero_bits,
            },
            transactions: self.transactions.clone(),
        };
        let computed_merkle = block.recompute_merkle_root()?;
        if computed_merkle != block.header.merkle_root {
            return Err(MinerError::InvalidTemplate(format!(
                "merkle_root mismatch: expected {}, computed {}",
                self.merkle_root,
                hash_to_hex(&computed_merkle)
            )));
        }
        Ok(block)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MiningSubmitRequest {
    pub protocol: String,
    pub template_id: String,
    pub nonce: u64,
    /// FiroPoW final hash (hex).
    pub block_hash: String,
    /// FiroPoW mix hash (hex); required for node verification.
    #[serde(default)]
    pub mix_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub miner_address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_name: Option<String>,
}

impl MiningSubmitRequest {
    pub fn from_solution(
        template_id: String,
        nonce: u64,
        final_hash: Hash,
        mix_hash: Hash,
    ) -> Self {
        Self {
            protocol: MINING_PROTOCOL_VERSION.to_owned(),
            template_id,
            nonce,
            block_hash: hash_to_hex(&final_hash),
            mix_hash: hash_to_hex(&mix_hash),
            miner_address: None,
            worker_name: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MiningSubmitResponse {
    pub protocol: String,
    pub status: SubmitStatus,
    pub template_id: String,
    pub block_hash: String,
    pub height: Option<u64>,
    pub reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubmitStatus {
    Accepted,
    Stale,
    Rejected,
    PendingLocal,
}

fn parse_hash(field: &str, value: &str) -> Result<Hash> {
    Hash::from_hex(value)
        .map_err(|error| MinerError::InvalidTemplate(format!("invalid {field}: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_legacy_foreign_protocol_and_network_id() {
        let template = MiningTemplate {
            protocol: "veiron-mining-v1".into(),
            template_id: "t1".into(),
            expires_at_unix_seconds: u64::MAX,
            version: 1,
            network_id: "veiron-mainnet-candidate".into(),
            height: 1,
            previous_hash: "00".repeat(32),
            merkle_root: "00".repeat(32),
            base_fee_atomic: 0,
            timestamp: 1,
            difficulty_leading_zero_bits: 1,
            share_difficulty_leading_zero_bits: None,
            nonce_start: 0,
            transactions: vec![],
        };
        let err = template
            .validate_and_build("alve1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq")
            .expect_err("foreign protocol must fail");
        assert!(err.to_string().contains("alvenqis-mining-v1"));
    }

    #[test]
    fn rejects_legacy_network_id_with_alvenqis_protocol() {
        let template = MiningTemplate {
            protocol: MINING_PROTOCOL_VERSION.into(),
            template_id: "t1".into(),
            expires_at_unix_seconds: u64::MAX,
            version: 1,
            network_id: "veiron-mainnet-candidate".into(),
            height: 1,
            previous_hash: "00".repeat(32),
            merkle_root: "00".repeat(32),
            base_fee_atomic: 0,
            timestamp: 1,
            difficulty_leading_zero_bits: 1,
            share_difficulty_leading_zero_bits: None,
            nonce_start: 0,
            transactions: vec![],
        };
        let err = template
            .validate_and_build("alve1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq")
            .expect_err("legacy network_id must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("legacy")
                || msg.contains("unknown network_id")
                || msg.contains("rejected"),
            "unexpected error: {msg}"
        );
    }
}
