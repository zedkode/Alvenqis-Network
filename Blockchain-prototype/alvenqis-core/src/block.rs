use crate::crypto::{blake3_hash, Hash};
use crate::errors::{AlvenqisError, Result};
use crate::firopow::{self, FiroPowOutput};
use crate::network::Network;
use crate::transaction::Transaction;
use crate::upgrade::expected_block_version;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    pub version: u32,
    pub network_id: String,
    pub height: u64,
    pub previous_hash: Hash,
    pub merkle_root: Hash,
    pub base_fee_atomic: u64,
    pub timestamp: u64,
    pub nonce: u64,
    /// FiroPoW mix hash (set by miner; verified by nodes).
    #[serde(default = "Hash::zero")]
    pub mix_hash: Hash,
    pub difficulty_leading_zero_bits: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn new(
        network: Network,
        height: u64,
        previous_hash: Hash,
        base_fee_atomic: u64,
        timestamp: u64,
        difficulty_leading_zero_bits: u8,
        transactions: Vec<Transaction>,
    ) -> Result<Self> {
        Self::new_with_version(
            expected_block_version(network, height),
            network,
            height,
            previous_hash,
            base_fee_atomic,
            timestamp,
            difficulty_leading_zero_bits,
            transactions,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_version(
        version: u32,
        network: Network,
        height: u64,
        previous_hash: Hash,
        base_fee_atomic: u64,
        timestamp: u64,
        difficulty_leading_zero_bits: u8,
        transactions: Vec<Transaction>,
    ) -> Result<Self> {
        if transactions.is_empty() {
            return Err(AlvenqisError::EmptyTransactions);
        }

        let merkle_root = compute_merkle_root(&transactions)?;
        Ok(Self {
            header: BlockHeader {
                version,
                network_id: network.network_id().to_owned(),
                height,
                previous_hash,
                merkle_root,
                base_fee_atomic,
                timestamp,
                nonce: 0,
                mix_hash: Hash::zero(),
                difficulty_leading_zero_bits,
            },
            transactions,
        })
    }

    /// Canonical chain identity = FiroPoW final hash only.
    ///
    /// There is **no** Blake3 / domain-separated fallback identity. If the native
    /// FiroPoW evaluator cannot produce a final hash, this returns an error so
    /// validation, storage, P2P, and RPC stop with a diagnostic failure instead of
    /// minting a second, environment-dependent block id.
    pub fn hash(&self) -> Result<Hash> {
        Ok(self.pow_result()?.final_hash)
    }

    /// FiroPoW final hash for the stored nonce (alias of [`Self::hash`]).
    pub fn pow_hash(&self) -> Result<Hash> {
        self.hash()
    }

    /// Full FiroPoW evaluation for the stored nonce.
    pub fn pow_result(&self) -> Result<FiroPowOutput> {
        let seed = firopow::mining_header_hash(self);
        firopow::firopow_hash(self.header.height, &seed, self.header.nonce)
    }

    pub fn pow_hash_with_nonce(&self, nonce: u64) -> Result<FiroPowOutput> {
        let seed = firopow::mining_header_hash(self);
        firopow::firopow_hash(self.header.height, &seed, nonce)
    }

    pub fn network(&self) -> Result<Network> {
        Network::from_network_id(&self.header.network_id).ok_or_else(|| {
            AlvenqisError::InvalidNetwork {
                expected: "known network".to_owned(),
                actual: self.header.network_id.clone(),
            }
        })
    }

    /// Identity serialization (includes mix_hash + nonce) for wire formats.
    pub fn header_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.version.to_le_bytes());
        push_string(&mut bytes, &self.header.network_id);
        bytes.extend_from_slice(&self.header.height.to_le_bytes());
        bytes.extend_from_slice(self.header.previous_hash.as_bytes());
        bytes.extend_from_slice(self.header.merkle_root.as_bytes());
        bytes.extend_from_slice(&self.header.base_fee_atomic.to_le_bytes());
        bytes.extend_from_slice(&self.header.timestamp.to_le_bytes());
        bytes.extend_from_slice(&self.header.nonce.to_le_bytes());
        bytes.extend_from_slice(self.header.mix_hash.as_bytes());
        bytes.push(self.header.difficulty_leading_zero_bits);
        bytes
    }

    pub fn recompute_merkle_root(&self) -> Result<Hash> {
        compute_merkle_root(&self.transactions)
    }
}

pub fn compute_merkle_root(transactions: &[Transaction]) -> Result<Hash> {
    if transactions.is_empty() {
        return Err(AlvenqisError::EmptyTransactions);
    }

    let mut level: Vec<Hash> = transactions.iter().map(Transaction::tx_hash).collect();
    while level.len() > 1 {
        if level.len() % 2 == 1 {
            let Some(&last) = level.last() else {
                return Err(AlvenqisError::EmptyTransactions);
            };
            level.push(last);
        }
        let mut next = Vec::with_capacity(level.len() / 2);
        for pair in level.chunks(2) {
            let mut data = Vec::with_capacity(64);
            data.extend_from_slice(pair[0].as_bytes());
            data.extend_from_slice(pair[1].as_bytes());
            // Merkle tree still uses Blake3 (not PoW).
            next.push(blake3_hash(&data));
        }
        level = next;
    }
    Ok(level[0])
}

fn push_string(bytes: &mut Vec<u8>, value: &str) {
    let encoded = value.as_bytes();
    bytes.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
    bytes.extend_from_slice(encoded);
}
