use alvenqis_core::{hash_to_hex, Amount, Block, Chain, Transaction, MAX_SUPPLY_ATOMIC};
use alvenqis_indexer::{AddressActivity, IndexSummary, IndexedBlock, IndexedTransaction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub ok: bool,
    pub service: &'static str,
    pub mode: String,
    pub network_id: String,
    pub network_name: String,
    pub status_label: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct NetworkResponse {
    pub protocol_parameters_id: &'static str,
    pub protocol_version: u32,
    pub block_version: u32,
    pub network_id: String,
    pub network_name: String,
    pub status_label: String,
    pub ticker: &'static str,
    pub address_prefix: String,
    pub address_standard_id: &'static str,
    pub address_encoding: &'static str,
    pub address_checksum_rule: &'static str,
    pub address_payload_version: u8,
    pub public_key_scheme: &'static str,
    pub signature_standard_id: &'static str,
    pub signature_scheme: &'static str,
    pub tx_signing_domain: &'static str,
    pub key_derivation_policy_id: &'static str,
    pub block_time_seconds: u64,
    pub decimals: u32,
    pub atomic_units_per_alve: u64,
    pub max_supply_atomic: u64,
    pub halving_interval_blocks: u64,
    pub initial_block_reward_atomic: u64,
    pub pow_hash_algorithm: &'static str,
    pub difficulty_adjustment_algorithm: &'static str,
    pub fee_policy: &'static str,
    pub default_rpc_port: u16,
    pub default_p2p_port: u16,
    /// Consensus DoS / timing bounds (clients and miners should honor these).
    #[serde(default)]
    pub max_transactions_per_block: usize,
    #[serde(default)]
    pub max_transaction_wire_bytes: usize,
    #[serde(default)]
    pub median_time_past_window: usize,
    #[serde(default)]
    pub max_future_block_drift_seconds: u64,
    /// First non-coinbase spend nonce for a new account.
    #[serde(default)]
    pub first_account_nonce: u64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct StatusResponse {
    pub network_id: String,
    pub network_name: String,
    pub status_label: String,
    pub initialized: bool,
    pub block_count: usize,
    pub height: Option<u64>,
    pub tip_hash: Option<String>,
    pub emitted_supply_atomic: Option<u64>,
    /// Indexer tip hash when available (None if index missing / unreadable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_tip_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_height: Option<u64>,
    /// True when index tip matches chain tip (or index not configured).
    #[serde(default)]
    pub index_in_sync: bool,
    #[serde(default)]
    pub index_lag_blocks: u64,
    /// Cumulative proof-of-work of the local canonical chain (decimal string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cumulative_work: Option<String>,
}

/// Bounded index data for frequently-polled clients. The full `/indexer/summary`
/// payload grows with chain history and must not be used as a live dashboard poll.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct IndexerOverviewResponse {
    pub summary: IndexSummary,
    pub recent_blocks: Vec<IndexedBlock>,
    pub recent_transactions: Vec<IndexedTransaction>,
}

/// Server-side pagination keeps explorer responses bounded as chain history grows.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct IndexerPageResponse<T> {
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    pub items: Vec<T>,
}

pub type IndexedBlocksPageResponse = IndexerPageResponse<IndexedBlock>;
pub type IndexedTransactionsPageResponse = IndexerPageResponse<IndexedTransaction>;
pub type IndexedAddressesPageResponse = IndexerPageResponse<AddressActivity>;

#[derive(Debug, Serialize, PartialEq)]
pub struct SyncStatusResponse {
    pub network_id: String,
    pub sync_state: &'static str,
    pub local_height: Option<u64>,
    pub network_height: Option<u64>,
    pub remaining_blocks: Option<u64>,
    pub progress_percent: Option<f64>,
    pub connected_peer_count: usize,
    pub validated_peer_count: usize,
    pub detail: &'static str,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ChainTipResponse {
    pub height: u64,
    pub hash: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ChainHeightResponse {
    pub height: u64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct AddressResponse {
    pub address: String,
    pub exists: bool,
    pub balance_atomic: u64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct AddressBalanceResponse {
    pub address: String,
    pub balance_atomic: u64,
    /// Ledger-backed next sequential spend nonce (same source as /account).
    #[serde(default)]
    pub next_nonce: u64,
    #[serde(default)]
    pub exists: bool,
}

/// Account snapshot for remote wallets (desktop keystore signing without a local chain).
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct AddressAccountResponse {
    pub address: String,
    pub exists: bool,
    pub balance_atomic: u64,
    /// Next transaction nonce for this address (1 if the account has never spent).
    pub next_nonce: u64,
    pub tip_hash: Option<String>,
    pub tip_height: Option<u64>,
    pub anticipated_base_fee_atomic: u64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct StateBalanceEntryResponse {
    pub address: String,
    pub balance_atomic: u64,
    #[serde(default)]
    pub next_nonce: u64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct StateResponse {
    pub height: Option<u64>,
    pub tip_hash: Option<String>,
    pub emitted_supply_atomic: u64,
    pub tracked_addresses: usize,
    pub latest_block_base_fee_atomic: Option<u64>,
    pub latest_block_fees_atomic: Option<u64>,
    pub latest_block_burned_fees_atomic: Option<u64>,
    pub latest_block_priority_fees_atomic: Option<u64>,
    pub latest_coinbase_reward_atomic: Option<u64>,
    pub balances: Vec<StateBalanceEntryResponse>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct SupplyResponse {
    pub emitted_supply_atomic: u64,
    pub max_supply_atomic: u64,
    pub remaining_supply_atomic: u64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
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
    pub fee_atomic: u64,
    pub max_fee_atomic: u64,
    pub priority_fee_atomic: u64,
    pub effective_fee_atomic: u64,
    pub burned_fee_atomic: u64,
    pub effective_priority_fee_atomic: u64,
    pub base_fee_atomic: u64,
    pub memo_hash: Option<String>,
    pub sender_public_key_hex: Option<String>,
    pub signature_hex: Option<String>,
    pub authorization_state: String,
    pub signature_standard_id: &'static str,
    pub signatures_status: &'static str,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct SubmitTransactionResponse {
    pub status: String,
    pub tx_hash: String,
    pub lifecycle_status: String,
    pub mempool_size: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct MempoolStatusResponse {
    pub status: String,
    pub pending_count: usize,
    pub anticipated_base_fee_atomic: u64,
    pub total_fees_atomic: u64,
    pub total_burned_fees_atomic: u64,
    pub total_priority_fees_atomic: u64,
    #[serde(default)]
    pub highest_priority_fee_atomic: u64,
    #[serde(default)]
    pub highest_max_fee_atomic: u64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct MempoolResponse {
    pub status: String,
    pub pending_count: usize,
    pub anticipated_base_fee_atomic: u64,
    pub total_fees_atomic: u64,
    pub total_burned_fees_atomic: u64,
    pub total_priority_fees_atomic: u64,
    #[serde(default)]
    pub highest_priority_fee_atomic: u64,
    #[serde(default)]
    pub highest_max_fee_atomic: u64,
    pub transactions: Vec<TransactionResponse>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct MiningTemplateResponse {
    pub protocol: &'static str,
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
    pub nonce_start: u64,
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct MiningSubmitRequest {
    pub protocol: String,
    pub template_id: String,
    pub nonce: u64,
    pub block_hash: String,
    /// FiroPoW mix hash (hex). Empty/default is rejected by validation if required.
    #[serde(default)]
    pub mix_hash: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct MiningSubmitResponse {
    pub protocol: &'static str,
    pub status: &'static str,
    pub template_id: String,
    pub block_hash: String,
    pub height: Option<u64>,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct BlockResponse {
    pub network_id: String,
    pub height: u64,
    pub hash: String,
    pub previous_hash: String,
    pub merkle_root: String,
    pub base_fee_atomic: u64,
    pub timestamp: u64,
    pub nonce: u64,
    pub difficulty_leading_zero_bits: u8,
    pub transaction_count: usize,
    pub transactions: Vec<TransactionResponse>,
}

pub fn block_response(block: &Block) -> Result<BlockResponse, alvenqis_core::AlvenqisError> {
    let block_hash = hash_to_hex(&block.hash()?);
    let base_fee = Amount::from_atomic(block.header.base_fee_atomic);
    Ok(BlockResponse {
        network_id: block.header.network_id.clone(),
        height: block.header.height,
        hash: block_hash.clone(),
        previous_hash: hash_to_hex(&block.header.previous_hash),
        merkle_root: hash_to_hex(&block.header.merkle_root),
        base_fee_atomic: block.header.base_fee_atomic,
        timestamp: block.header.timestamp,
        nonce: block.header.nonce,
        difficulty_leading_zero_bits: block.header.difficulty_leading_zero_bits,
        transaction_count: block.transactions.len(),
        transactions: block
            .transactions
            .iter()
            .map(|transaction| {
                transaction_response(
                    transaction,
                    "mined",
                    Some(block.header.height),
                    Some(&block_hash),
                    base_fee,
                )
            })
            .collect(),
    })
}

pub fn address_response(chain: &Chain, address: &str) -> AddressResponse {
    AddressResponse {
        address: address.to_owned(),
        exists: chain.state().balances().contains_key(address),
        balance_atomic: chain.state().balance_of(address).as_atomic(),
    }
}

pub fn address_balance_response(chain: &Chain, address: &str) -> AddressBalanceResponse {
    let next_nonce = chain.state().next_nonce_of(address);
    AddressBalanceResponse {
        address: address.to_owned(),
        balance_atomic: chain.state().balance_of(address).as_atomic(),
        next_nonce,
        exists: chain.state().balances().contains_key(address)
            || next_nonce > alvenqis_core::FIRST_ACCOUNT_NONCE,
    }
}

pub fn address_account_response(
    chain: &Chain,
    address: &str,
    mempool_nonces: impl IntoIterator<Item = u64>,
    anticipated_base_fee_atomic: u64,
) -> Result<AddressAccountResponse, alvenqis_core::AlvenqisError> {
    // Canonical next nonce comes from ledger sequential state (FIRST_ACCOUNT_NONCE = 1).
    let mut next = chain.state().next_nonce_of(address);
    // Pending mempool spends reserve nonces; clients must continue after the highest pending.
    for nonce in mempool_nonces {
        next = next.max(nonce.saturating_add(1));
    }
    Ok(AddressAccountResponse {
        address: address.to_owned(),
        exists: chain.state().balances().contains_key(address)
            || chain.state().next_nonce_of(address) > alvenqis_core::FIRST_ACCOUNT_NONCE,
        balance_atomic: chain.state().balance_of(address).as_atomic(),
        next_nonce: next,
        tip_hash: chain.tip_hash()?.map(|hash| hash_to_hex(&hash)),
        tip_height: chain.height(),
        anticipated_base_fee_atomic,
    })
}

pub fn state_response(chain: &Chain) -> Result<StateResponse, alvenqis_core::AlvenqisError> {
    let latest_height = chain.height();
    let latest_block_base_fee_atomic = latest_height.and_then(|height| {
        chain
            .blocks()
            .iter()
            .find(|block| block.header.height == height)
            .map(|block| block.header.base_fee_atomic)
    });
    let latest_block_fees_atomic = latest_height.and_then(|height| {
        chain
            .state()
            .block_fees()
            .get(&height)
            .copied()
            .map(|amount| amount.as_atomic())
    });
    let latest_block_burned_fees_atomic = latest_height.and_then(|height| {
        chain
            .state()
            .block_burned_fees()
            .get(&height)
            .copied()
            .map(|amount| amount.as_atomic())
    });
    let latest_block_priority_fees_atomic = latest_height.and_then(|height| {
        chain
            .state()
            .block_priority_fees()
            .get(&height)
            .copied()
            .map(|amount| amount.as_atomic())
    });
    let latest_coinbase_reward_atomic = latest_height.and_then(|height| {
        chain
            .state()
            .coinbase_rewards()
            .get(&height)
            .copied()
            .map(|amount| amount.as_atomic())
    });

    Ok(StateResponse {
        height: latest_height,
        tip_hash: chain.tip_hash()?.map(|hash| hash_to_hex(&hash)),
        emitted_supply_atomic: chain.emitted_supply().as_atomic(),
        tracked_addresses: chain.state().balances().len(),
        latest_block_base_fee_atomic,
        latest_block_fees_atomic,
        latest_block_burned_fees_atomic,
        latest_block_priority_fees_atomic,
        latest_coinbase_reward_atomic,
        balances: chain
            .state()
            .balances()
            .iter()
            .map(|(address, balance)| StateBalanceEntryResponse {
                address: address.clone(),
                balance_atomic: balance.as_atomic(),
                next_nonce: chain.state().next_nonce_of(address),
            })
            .collect(),
    })
}

pub fn supply_response(chain: &Chain) -> SupplyResponse {
    let emitted_supply_atomic = chain.emitted_supply().as_atomic();
    SupplyResponse {
        emitted_supply_atomic,
        max_supply_atomic: MAX_SUPPLY_ATOMIC,
        remaining_supply_atomic: MAX_SUPPLY_ATOMIC - emitted_supply_atomic,
    }
}

pub fn transaction_response(
    transaction: &Transaction,
    lifecycle_status: &str,
    block_height: Option<u64>,
    block_hash: Option<&str>,
    base_fee: Amount,
) -> TransactionResponse {
    let effective_priority_fee_atomic = transaction
        .effective_priority_fee(base_fee)
        .map(|amount| amount.as_atomic())
        .unwrap_or(0);
    let effective_fee_atomic = transaction
        .effective_fee(base_fee)
        .map(|amount| amount.as_atomic())
        .unwrap_or(0);
    let burned_fee_atomic = if transaction.is_coinbase() {
        0
    } else {
        base_fee.as_atomic()
    };
    TransactionResponse {
        lifecycle_status: lifecycle_status.to_owned(),
        hash: hash_to_hex(&transaction.tx_hash()),
        block_height,
        block_hash: block_hash.map(ToOwned::to_owned),
        version: transaction.version,
        nonce: transaction.nonce,
        from: transaction.from.clone(),
        to: transaction.to.clone(),
        amount_atomic: transaction.amount.as_atomic(),
        fee_atomic: effective_fee_atomic,
        max_fee_atomic: transaction.max_fee.as_atomic(),
        priority_fee_atomic: transaction.priority_fee.as_atomic(),
        effective_fee_atomic,
        burned_fee_atomic,
        effective_priority_fee_atomic,
        base_fee_atomic: base_fee.as_atomic(),
        memo_hash: transaction.memo_hash.map(|hash| hash_to_hex(&hash)),
        sender_public_key_hex: transaction
            .sender_public_key
            .as_ref()
            .map(|public_key| public_key.to_hex()),
        signature_hex: transaction
            .signature
            .as_ref()
            .map(|signature| signature.to_hex()),
        authorization_state: if transaction.is_coinbase() {
            "coinbase".to_owned()
        } else if transaction.signature.is_some() {
            "signed".to_owned()
        } else {
            "unsigned".to_owned()
        },
        signature_standard_id: alvenqis_core::launch_signing_standard().standard_id,
        signatures_status: alvenqis_core::SIGNATURES_STATUS,
    }
}
