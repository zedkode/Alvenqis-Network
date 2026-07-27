use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const POOL_PROTOCOL_VERSION: &str = "alvenqis-pool-v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShareRecord {
    pub share_id: u64,
    pub job_id: String,
    pub miner_address: String,
    pub worker_name: String,
    pub nonce: u64,
    pub hash: String,
    pub share_difficulty_leading_zero_bits: u8,
    pub network_difficulty_leading_zero_bits: u8,
    pub accepted_at_unix_seconds: u64,
    pub block_candidate: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AccountBalance {
    pub immature_atomic: u64,
    pub mature_atomic: u64,
    pub pending_payout_atomic: u64,
    pub paid_atomic: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PoolBlockStatus {
    Immature,
    Mature,
    Orphaned,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PoolBlock {
    pub height: u64,
    pub hash: String,
    pub reward_atomic: u64,
    pub distributable_atomic: u64,
    pub pool_fee_atomic: u64,
    pub found_at_unix_seconds: u64,
    pub status: PoolBlockStatus,
    pub allocations: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PayoutStatus {
    Prepared,
    Submitted,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PayoutItem {
    pub address: String,
    pub amount_atomic: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PayoutBatch {
    pub payout_id: String,
    pub created_at_unix_seconds: u64,
    pub status: PayoutStatus,
    pub items: Vec<PayoutItem>,
    pub transaction_hashes: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PoolData {
    pub next_share_id: u64,
    pub shares: Vec<ShareRecord>,
    pub accounts: BTreeMap<String, AccountBalance>,
    pub blocks: Vec<PoolBlock>,
    pub payouts: Vec<PayoutBatch>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WorkQuery {
    pub miner_address: String,
    pub worker_name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkerView {
    pub miner_address: String,
    pub worker_name: String,
    pub accepted_shares: u64,
    pub blocks_found: u64,
    pub estimated_hashrate_hs: u64,
    pub assigned_difficulty_leading_zero_bits: u8,
    pub last_share_unix_seconds: u64,
    pub online: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolStatusView {
    pub protocol: &'static str,
    pub mode: &'static str,
    pub status_label: String,
    pub pool_name: String,
    pub network_id: String,
    pub pool_address: String,
    pub upstream_status: String,
    pub upstream_error: Option<String>,
    pub pool_fee_basis_points: u16,
    pub payout_scheme: &'static str,
    pub minimum_payout_atomic: u64,
    pub block_maturity_confirmations: u64,
    pub vardiff_enabled: bool,
    pub target_share_seconds: u64,
    pub accepted_shares: u64,
    pub connected_workers: usize,
    pub estimated_hashrate_hs: u64,
    pub blocks_found: usize,
    pub matured_blocks: usize,
    pub rejected_requests: u64,
    pub rate_limited_requests: u64,
    pub active_bans: usize,
    pub workers: Vec<WorkerView>,
    pub recent_blocks: Vec<PoolBlock>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MinerView {
    pub address: String,
    pub balance: AccountBalance,
    pub workers: Vec<WorkerView>,
    pub payouts: Vec<PayoutBatch>,
}

/// Public read-only history bundle for Control Center / explorers.
#[derive(Clone, Debug, Serialize)]
pub struct PoolHistoryView {
    pub protocol: &'static str,
    pub pool_name: String,
    pub network_id: String,
    pub pool_address: String,
    pub status_label: String,
    pub accepted_shares_counter: u64,
    pub connected_workers: usize,
    pub estimated_hashrate_hs: u64,
    pub blocks_found: usize,
    pub matured_blocks: usize,
    pub workers: Vec<WorkerView>,
    /// Full stored block history (newest first, capped).
    pub blocks: Vec<PoolBlock>,
    /// Recent accepted shares (newest first, capped).
    pub shares: Vec<ShareRecord>,
    /// Payout batches (newest first, capped).
    pub payouts: Vec<PayoutBatch>,
    pub accounts: Vec<PoolAccountView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolAccountView {
    pub address: String,
    pub immature_atomic: u64,
    pub mature_atomic: u64,
    pub pending_payout_atomic: u64,
    pub paid_atomic: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfirmPayoutRequest {
    pub transaction_hashes: Vec<String>,
}
