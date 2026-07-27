use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthResponse {
    pub ok: bool,
    #[serde(default)]
    pub service: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub network_id: Option<String>,
    #[serde(default)]
    pub network_name: Option<String>,
    #[serde(default)]
    pub status_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkResponse {
    pub network_id: String,
    pub network_name: String,
    pub status_label: String,
    #[serde(default)]
    pub ticker: Option<String>,
    #[serde(default)]
    pub address_prefix: Option<String>,
    #[serde(default)]
    pub decimals: Option<u32>,
    #[serde(default)]
    pub default_rpc_port: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusResponse {
    pub network_id: String,
    pub network_name: String,
    pub status_label: String,
    pub initialized: bool,
    pub block_count: usize,
    pub height: Option<u64>,
    pub tip_hash: Option<String>,
    pub emitted_supply_atomic: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainTipResponse {
    pub height: u64,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainHeightResponse {
    pub height: u64,
}

/// Remote / mobile-friendly sync summary (`GET /sync/status`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncStatusResponse {
    pub network_id: String,
    pub sync_state: String,
    pub local_height: Option<u64>,
    pub network_height: Option<u64>,
    pub remaining_blocks: Option<u64>,
    pub progress_percent: Option<f64>,
    #[serde(default)]
    pub connected_peer_count: usize,
    #[serde(default)]
    pub validated_peer_count: usize,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplyResponse {
    pub emitted_supply_atomic: u64,
    pub max_supply_atomic: u64,
    pub remaining_supply_atomic: u64,
}

/// Lightweight mempool summary (`GET /mempool/status`) — no tx bodies, no mining.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MempoolStatusResponse {
    pub status: String,
    pub pending_count: usize,
    pub anticipated_base_fee_atomic: u64,
    #[serde(default)]
    pub total_fees_atomic: u64,
    #[serde(default)]
    pub total_burned_fees_atomic: u64,
    #[serde(default)]
    pub total_priority_fees_atomic: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressBalanceResponse {
    pub address: String,
    pub balance_atomic: u64,
    #[serde(default)]
    pub next_nonce: u64,
    #[serde(default)]
    pub exists: bool,
}

/// Account snapshot used for remote wallet prepare (balance + next nonce + tip).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressAccountResponse {
    pub address: String,
    pub exists: bool,
    pub balance_atomic: u64,
    pub next_nonce: u64,
    pub tip_hash: Option<String>,
    pub tip_height: Option<u64>,
    #[serde(default)]
    pub anticipated_base_fee_atomic: u64,
}

/// Transaction as returned by RPC (pending or mined). Extra fields are optional for forward-compat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub lifecycle_status: String,
    pub hash: String,
    pub block_height: Option<u64>,
    pub block_hash: Option<String>,
    pub version: u32,
    pub nonce: u64,
    pub from: Option<String>,
    pub to: String,
    pub amount_atomic: u64,
    #[serde(default)]
    pub fee_atomic: u64,
    #[serde(default)]
    pub max_fee_atomic: u64,
    #[serde(default)]
    pub priority_fee_atomic: u64,
    #[serde(default)]
    pub effective_fee_atomic: u64,
    #[serde(default)]
    pub burned_fee_atomic: u64,
    #[serde(default)]
    pub effective_priority_fee_atomic: u64,
    #[serde(default)]
    pub base_fee_atomic: u64,
    #[serde(default)]
    pub memo_hash: Option<String>,
    #[serde(default)]
    pub sender_public_key_hex: Option<String>,
    #[serde(default)]
    pub signature_hex: Option<String>,
    #[serde(default)]
    pub authorization_state: Option<String>,
    #[serde(default)]
    pub signature_standard_id: Option<String>,
    #[serde(default)]
    pub signatures_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitTransactionResponse {
    pub status: String,
    pub tx_hash: String,
    pub lifecycle_status: String,
    #[serde(default)]
    pub mempool_size: usize,
}

/// Full block payload from `GET /blocks/*`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockResponse {
    #[serde(default)]
    pub network_id: Option<String>,
    pub height: u64,
    pub hash: String,
    pub previous_hash: String,
    pub merkle_root: String,
    #[serde(default)]
    pub base_fee_atomic: u64,
    pub timestamp: u64,
    pub nonce: u64,
    pub difficulty_leading_zero_bits: u8,
    pub transaction_count: usize,
    #[serde(default)]
    pub transactions: Vec<TransactionResponse>,
}

impl BlockResponse {
    /// Coinbase-like first tx amount when present (best-effort explorer helper).
    pub fn coinbase_amount_atomic(&self) -> Option<u64> {
        self.transactions.iter().find_map(|tx| {
            if tx.from.is_none() || tx.authorization_state.as_deref() == Some("coinbase") {
                Some(tx.amount_atomic)
            } else {
                None
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplySummary {
    pub emitted_supply_atomic: u64,
    pub max_supply_atomic: u64,
    pub remaining_supply_atomic: u64,
}

/// Compact index summary embedded in `/indexer/summary` and related payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexSummary {
    pub mode: String,
    pub network: String,
    pub status: String,
    pub indexed_height: Option<u64>,
    pub indexed_block_count: usize,
    pub transaction_count: usize,
    pub address_count: usize,
    pub tip_hash: Option<String>,
    pub latest_block_hash: Option<String>,
    pub latest_block_timestamp: Option<u64>,
    pub supply: SupplySummary,
}

/// Only the `summary` object is required — large maps in the full JSON are ignored by serde.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexerSummaryResponse {
    pub summary: IndexSummary,
}

/// `GET /indexer/status` — includes optional VPS sync-lag fields when present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexerStatusResponse {
    pub mode: String,
    #[serde(default)]
    pub network_id: Option<String>,
    #[serde(default)]
    pub status_label: Option<String>,
    pub initialized: bool,
    #[serde(default)]
    pub index_dir: Option<String>,
    pub indexed_height: Option<u64>,
    pub indexed_block_count: usize,
    pub transaction_count: usize,
    pub address_count: usize,
    pub tip_hash: Option<String>,
    #[serde(default)]
    pub chain_height: Option<u64>,
    #[serde(default)]
    pub chain_tip_hash: Option<String>,
    #[serde(default)]
    pub in_sync: Option<bool>,
    #[serde(default)]
    pub lag_blocks: Option<u64>,
}

/// Indexed block view (`GET /indexer/blocks/*`) — hash lists, not full tx bodies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedBlockResponse {
    pub height: u64,
    pub hash: String,
    pub previous_hash: String,
    pub merkle_root: String,
    pub timestamp: u64,
    pub nonce: u64,
    pub difficulty_leading_zero_bits: u8,
    pub transaction_count: usize,
    pub miner_address: String,
    pub coinbase_payout_atomic: u64,
    pub miner_reward_atomic: u64,
    pub fees_atomic: u64,
    pub burned_fees_atomic: u64,
    pub priority_fees_atomic: u64,
    pub base_fee_atomic: u64,
    #[serde(default)]
    pub transaction_hashes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedTransactionResponse {
    pub lifecycle_status: String,
    pub hash: String,
    pub block_height: u64,
    pub block_hash: String,
    #[serde(default)]
    pub block_transaction_count: usize,
    #[serde(default)]
    pub transaction_index: usize,
    pub version: u32,
    pub nonce: u64,
    pub from: Option<String>,
    pub to: String,
    pub amount_atomic: u64,
    #[serde(default)]
    pub fee_atomic: u64,
    #[serde(default)]
    pub max_fee_atomic: u64,
    #[serde(default)]
    pub priority_fee_atomic: u64,
    #[serde(default)]
    pub effective_fee_atomic: u64,
    #[serde(default)]
    pub burned_fee_atomic: u64,
    #[serde(default)]
    pub effective_priority_fee_atomic: u64,
    #[serde(default)]
    pub base_fee_atomic: u64,
    #[serde(default)]
    pub memo_hash: Option<String>,
    #[serde(default)]
    pub authorization_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedAddressResponse {
    pub address: String,
    pub exists_in_ledger: bool,
    pub balance_atomic: u64,
    pub total_received_atomic: u64,
    pub total_sent_atomic: u64,
    pub mined_reward_atomic: u64,
    #[serde(default)]
    pub transaction_hashes: Vec<String>,
    #[serde(default)]
    pub sent_tx_hashes: Vec<String>,
    #[serde(default)]
    pub received_tx_hashes: Vec<String>,
    #[serde(default)]
    pub mined_block_heights: Vec<u64>,
}

/// Lightweight P2P view from `GET /p2p/status` (fields optional for gateway variance).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct P2pStatusResponse {
    #[serde(default)]
    pub local_peer_id: Option<String>,
    #[serde(default)]
    pub listen_addresses: Vec<String>,
    #[serde(default)]
    pub connected_peer_count: Option<u64>,
    #[serde(default)]
    pub validated_peer_count: Option<u64>,
    #[serde(default)]
    pub mining_peer_count: Option<u64>,
    #[serde(default)]
    pub peers: Vec<serde_json::Value>,
    #[serde(default)]
    pub syncing: Option<bool>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
}

/// Atomic amount as decimal string or number (pool/gateway variance).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AtomicValue {
    U64(u64),
    I64(i64),
    String(String),
}

impl AtomicValue {
    pub fn as_display(&self) -> String {
        match self {
            Self::U64(v) => v.to_string(),
            Self::I64(v) => v.to_string(),
            Self::String(v) => v.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PoolWorker {
    #[serde(default)]
    pub miner_address: String,
    #[serde(default)]
    pub worker_name: String,
    #[serde(default)]
    pub accepted_shares: u64,
    #[serde(default)]
    pub blocks_found: u64,
    #[serde(default)]
    pub estimated_hashrate_hs: f64,
    #[serde(default)]
    pub assigned_difficulty_leading_zero_bits: u64,
    #[serde(default)]
    pub last_share_unix_seconds: u64,
    #[serde(default)]
    pub online: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PoolBlock {
    pub height: u64,
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub reward_atomic: Option<AtomicValue>,
    #[serde(default)]
    pub distributable_atomic: Option<AtomicValue>,
    #[serde(default)]
    pub pool_fee_atomic: Option<AtomicValue>,
    #[serde(default)]
    pub found_at_unix_seconds: Option<u64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub allocations: Option<serde_json::Value>,
}

/// Public pool status (`GET {pool}/api/v1/pool/status`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PoolStatusResponse {
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub status_label: Option<String>,
    #[serde(default)]
    pub pool_name: Option<String>,
    #[serde(default)]
    pub network_id: Option<String>,
    #[serde(default)]
    pub pool_address: Option<String>,
    #[serde(default)]
    pub upstream_status: Option<String>,
    #[serde(default)]
    pub upstream_error: Option<String>,
    #[serde(default)]
    pub pool_fee_basis_points: Option<u64>,
    #[serde(default)]
    pub payout_scheme: Option<String>,
    #[serde(default)]
    pub minimum_payout_atomic: Option<AtomicValue>,
    #[serde(default)]
    pub block_maturity_confirmations: Option<u64>,
    #[serde(default)]
    pub vardiff_enabled: Option<bool>,
    #[serde(default)]
    pub target_share_seconds: Option<u64>,
    #[serde(default)]
    pub accepted_shares: Option<u64>,
    #[serde(default)]
    pub connected_workers: Option<u64>,
    #[serde(default)]
    pub estimated_hashrate_hs: Option<f64>,
    #[serde(default)]
    pub blocks_found: Option<u64>,
    #[serde(default)]
    pub matured_blocks: Option<u64>,
    #[serde(default)]
    pub rejected_requests: Option<u64>,
    #[serde(default)]
    pub rate_limited_requests: Option<u64>,
    #[serde(default)]
    pub active_bans: Option<u64>,
    #[serde(default)]
    pub workers: Vec<PoolWorker>,
    #[serde(default)]
    pub recent_blocks: Vec<PoolBlock>,
}

/// Public pool history (`GET {pool}/api/v1/pool/history`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PoolHistoryResponse {
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub pool_name: Option<String>,
    #[serde(default)]
    pub network_id: Option<String>,
    #[serde(default)]
    pub pool_address: Option<String>,
    #[serde(default)]
    pub status_label: Option<String>,
    #[serde(default)]
    pub accepted_shares_counter: Option<u64>,
    #[serde(default)]
    pub connected_workers: Option<u64>,
    #[serde(default)]
    pub estimated_hashrate_hs: Option<f64>,
    #[serde(default)]
    pub blocks_found: Option<u64>,
    #[serde(default)]
    pub matured_blocks: Option<u64>,
    #[serde(default)]
    pub workers: Vec<PoolWorker>,
    #[serde(default)]
    pub blocks: Vec<PoolBlock>,
    #[serde(default)]
    pub shares: Vec<serde_json::Value>,
    #[serde(default)]
    pub payouts: Vec<serde_json::Value>,
    #[serde(default)]
    pub accounts: Vec<serde_json::Value>,
}

/// Pool block with computed maturity (convenience type).
#[derive(Debug, Clone, PartialEq)]
pub struct PoolBlockWithMaturity {
    pub height: u64,
    pub hash: String,
    pub status: String,
    pub reward_atomic: Option<AtomicValue>,
    pub maturity: crate::maturity::MaturityProgress,
}
