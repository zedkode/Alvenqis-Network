use crate::config::NetworkConfig;
use crate::dev_helpers::default_miner_address;
use crate::error::{NodeError, NodeResult};
use crate::mempool::{
    clear_mempool, current_unix_seconds, default_network_root, load_pending_transactions,
    lowest_fee_sender_package, mempool_file_path, reconcile_after_reorg,
    sanitize_pending_transactions, tx_hash_string, validate_pending_transaction,
    PendingTransactionRecord, MAX_PENDING_TXS_PER_SENDER,
};
use crate::p2p::{load_p2p_status, run_p2p_service};
use crate::storage::{self, BlockStore, SqliteBlockStore};
use alvenqis_core::{
    apply_transaction, blake3_hash, block_reward, child_block_with_consensus_difficulty,
    common_ancestor_height, genesis_with_difficulty_for_network,
    genesis_with_timestamp_for_network, hash_to_hex, median_time_past,
    mine_block as mine_core_block, next_base_fee, next_difficulty_for_network, select_fork,
    Address, Amount, Block, Chain, ForkChoice, Network, PrivateKey, Transaction,
    MAX_TRANSACTIONS_PER_BLOCK,
};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

pub const DEFAULT_MAINNET_CANDIDATE_CONFIG_PATH: &str = "configs/mainnet-candidate.toml";
pub const DEFAULT_CONFIG_PATH: &str = DEFAULT_MAINNET_CANDIDATE_CONFIG_PATH;
pub const DEFAULT_DATA_DIR: &str = ".alvenqis-mainnet/chain";
pub const LOCAL_OPERATOR_ROOT: &str = ".alvenqis-local";
pub const GENESIS_REVIEW_STANDARD_ID: &str = "alvenqis-genesis-review-v1";
pub const GENESIS_APPROVAL_STANDARD_ID: &str = "alvenqis-genesis-approval-v1";
const NODE_RUNTIME_DIR_NAME: &str = "node";
const NODE_RUNTIME_FILE_NAME: &str = "runtime.json";
const NODE_SHUTDOWN_FILE_NAME: &str = "shutdown.signal";
const GENESIS_MARKER_FILE_NAME: &str = "genesis-info.json";
const NODE_POLL_INTERVAL_SECONDS: u64 = 1;
pub const MAX_BLOCK_TEMPLATE_TRANSACTIONS: usize = 10_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisConfig {
    pub network: Network,
    pub network_id: String,
    pub human_name: String,
    pub status_label: String,
    pub address_prefix: String,
    pub timestamp: u64,
    pub difficulty_leading_zero_bits: u8,
    pub recipient_strategy: String,
    #[serde(default)]
    pub recipient_address: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisMarker {
    pub network_id: String,
    pub genesis_hash: String,
    pub genesis_height: u64,
    pub status_label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisReviewManifest {
    pub review_standard_id: String,
    pub network_id: String,
    pub human_name: String,
    pub status_label: String,
    pub address_prefix: String,
    pub block_time_seconds: u64,
    pub difficulty_leading_zero_bits: u8,
    pub chain_magic_hex: String,
    pub genesis_timestamp: u64,
    pub recipient_strategy: String,
    pub recipient_address: Option<String>,
    pub resolved_recipient_address: String,
    pub deterministic_genesis_hash: String,
    pub review_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisApprovalRecord {
    pub approval_standard_id: String,
    pub review_standard_id: String,
    pub network_id: String,
    pub human_name: String,
    pub status_label: String,
    pub deterministic_genesis_hash: String,
    pub approved_review_hash: String,
    pub approved_by: String,
    #[serde(default)]
    pub approval_notes: Option<String>,
    pub approved_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct GenesisApprovalStatus {
    pub network_id: String,
    pub human_name: String,
    pub status_label: String,
    pub approval_required: bool,
    pub approval_path: Option<String>,
    pub approved: bool,
    pub deterministic_genesis_hash: String,
    pub approved_genesis_hash: Option<String>,
    pub review_hash: String,
    pub approved_review_hash: Option<String>,
    pub approved_by: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ChainSummary {
    pub network_id: String,
    pub network_name: String,
    pub status: String,
    pub block_count: usize,
    pub height: u64,
    pub tip_hash: String,
    pub emitted_supply_atomic: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ChainReorgSummary {
    pub common_ancestor_height: u64,
    pub detached_blocks: usize,
    pub attached_blocks: usize,
    pub previous_tip_hash: String,
    pub new_tip_hash: String,
    pub previous_chain_work: u128,
    pub new_chain_work: u128,
    pub dropped_mempool_transactions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BalanceSummary {
    pub address: String,
    pub balance_atomic: u64,
    pub exists: bool,
    /// Next sequential spend nonce for this account (ledger-backed).
    #[serde(default)]
    pub next_nonce: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StateBalanceEntry {
    pub address: String,
    pub balance_atomic: u64,
    #[serde(default)]
    pub next_nonce: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StateSummary {
    pub status: String,
    pub network_name: String,
    pub chain_status: String,
    pub height: u64,
    pub tip_hash: String,
    pub emitted_supply_atomic: u64,
    pub tracked_addresses: usize,
    pub latest_block_base_fee_atomic: u64,
    pub latest_block_fees_atomic: u64,
    pub latest_block_burned_fees_atomic: u64,
    pub latest_block_priority_fees_atomic: u64,
    pub latest_coinbase_reward_atomic: u64,
    pub balances: Vec<StateBalanceEntry>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SendTransactionSummary {
    pub status: String,
    pub tx_hash: String,
    pub block_height: u64,
    pub block_hash: String,
    pub from: String,
    pub to: String,
    pub amount_atomic: u64,
    pub max_fee_atomic: u64,
    pub priority_fee_atomic: u64,
    pub effective_fee_atomic: u64,
    pub fee_atomic: u64,
    pub sender_balance_atomic: u64,
    pub recipient_balance_atomic: u64,
    pub miner_address: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SubmitTransactionSummary {
    pub status: String,
    pub tx_hash: String,
    pub lifecycle_status: String,
    pub mempool_size: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct MempoolStatusSummary {
    pub status: String,
    pub mempool_dir: String,
    pub pending_count: usize,
    pub anticipated_base_fee_atomic: u64,
    pub total_fees_atomic: u64,
    pub total_burned_fees_atomic: u64,
    pub total_priority_fees_atomic: u64,
    /// Best effective priority tip among pending txs (template fee market signal).
    #[serde(default)]
    pub highest_priority_fee_atomic: u64,
    /// Highest max_fee among pending txs (payer ceiling signal).
    #[serde(default)]
    pub highest_max_fee_atomic: u64,
    pub pending_hashes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct MinePendingBlockSummary {
    pub status: String,
    pub block_height: u64,
    pub block_hash: String,
    pub included_tx_hashes: Vec<String>,
    pub skipped_tx_hashes: Vec<String>,
    pub pending_remaining: usize,
    pub miner_address: String,
    pub miner_balance_atomic: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BlockTemplate {
    pub network_id: String,
    pub tip_hash: String,
    pub created_at_unix_seconds: u64,
    pub block: Block,
    pub included_tx_hashes: Vec<String>,
    pub skipped_tx_hashes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SubmittedMinedBlock {
    pub status: String,
    pub block_height: u64,
    pub block_hash: String,
    pub accepted_tx_hashes: Vec<String>,
    pub pending_remaining: Option<usize>,
    pub mempool_cleanup_complete: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeRuntimeStatus {
    pub mode: String,
    pub running: bool,
    pub pid: Option<u32>,
    pub network_id: String,
    pub network_name: String,
    pub status_label: String,
    pub data_dir: String,
    pub mempool_dir: String,
    pub runtime_dir: String,
    pub chain_initialized: bool,
    pub height: Option<u64>,
    pub block_count: usize,
    pub tip_hash: Option<String>,
    pub emitted_supply_atomic: Option<u64>,
    pub pending_count: usize,
    pub genesis_hash: Option<String>,
    pub genesis_approval_required: bool,
    pub genesis_approval_path: Option<String>,
    pub genesis_approved: bool,
    pub genesis_review_hash: Option<String>,
    pub genesis_approved_by: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PersistedRuntimeProcessState {
    #[serde(default)]
    running: bool,
    #[serde(default)]
    pid: Option<u32>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ResetSummary {
    pub status: String,
    pub network_id: String,
    pub data_dir: String,
    pub mempool_dir: String,
    pub backup_dir: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PeersSummary {
    pub mode: String,
    pub network_id: String,
    pub local_p2p_port: u16,
    pub chain_magic_hex: String,
    pub connected_peers: Vec<String>,
    pub local_peer_id: String,
    pub listen_addresses: Vec<String>,
    pub connected_peer_count: usize,
    pub validated_peer_count: usize,
    pub mining_peer_count: usize,
    pub observed_network_hashrate_hs: u64,
    pub miners: Vec<crate::p2p::NetworkMinerPresence>,
    pub validating_peer_count: usize,
    pub syncing: bool,
    pub peers: Vec<crate::p2p::ConnectedPeer>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug)]
pub enum StatusReport {
    Uninitialized {
        network_id: String,
        network_name: String,
        status: String,
        data_dir: String,
    },
    Ready(ChainSummary),
}

#[derive(Clone, Debug, Serialize)]
struct GenesisReviewPayload {
    review_standard_id: String,
    network_id: String,
    human_name: String,
    status_label: String,
    address_prefix: String,
    block_time_seconds: u64,
    difficulty_leading_zero_bits: u8,
    chain_magic_hex: String,
    genesis_timestamp: u64,
    recipient_strategy: String,
    recipient_address: Option<String>,
    resolved_recipient_address: String,
    deterministic_genesis_hash: String,
}

pub fn default_config_path(network: Network) -> PathBuf {
    match network {
        Network::Devnet => PathBuf::from("alvenqis-devnet/config/devnet.toml"),
        Network::Testnet => PathBuf::from("alvenqis-devnet/config/testnet.toml"),
        Network::MainnetCandidate => PathBuf::from(DEFAULT_MAINNET_CANDIDATE_CONFIG_PATH),
    }
}

pub fn default_data_dir(network: Network) -> PathBuf {
    default_network_root(network).join("chain")
}

pub fn default_runtime_dir(network: Network) -> PathBuf {
    default_network_root(network).join(NODE_RUNTIME_DIR_NAME)
}

pub fn runtime_dir_for_data_dir(data_dir: &Path) -> PathBuf {
    data_dir
        .parent()
        .unwrap_or(data_dir)
        .join(NODE_RUNTIME_DIR_NAME)
}

pub fn init_devnet(
    config_path: &Path,
    data_dir: &Path,
    miner_address: &str,
) -> NodeResult<ChainSummary> {
    let config = NetworkConfig::load_from_path(config_path)?;
    ensure_network_storage_path(config.network, data_dir)?;
    storage::ensure_data_dir(data_dir)?;

    match storage::load_blocks(data_dir) {
        Ok(existing_blocks) => {
            summarize_validated_blocks(config_path, &config, data_dir, &existing_blocks)
        }
        Err(NodeError::ChainNotInitialized(_)) => {
            let genesis = genesis_with_difficulty_for_network(
                config.network,
                miner_address,
                config.difficulty_leading_zero_bits,
            )?;
            storage::append_block(data_dir, &genesis)?;
            summarize_validated_blocks(config_path, &config, data_dir, &[genesis])
        }
        Err(error) => Err(error),
    }
}

pub fn status(config_path: &Path, data_dir: &Path) -> NodeResult<StatusReport> {
    let config = NetworkConfig::load_from_path(config_path)?;
    ensure_network_storage_path(config.network, data_dir)?;
    match storage::load_blocks(data_dir) {
        Ok(blocks) => Ok(StatusReport::Ready(summarize_validated_blocks(
            config_path,
            &config,
            data_dir,
            &blocks,
        )?)),
        Err(NodeError::ChainNotInitialized(_)) => Ok(StatusReport::Uninitialized {
            network_id: config.network.network_id().to_owned(),
            network_name: config.human_name,
            status: config.status_label,
            data_dir: data_dir.display().to_string(),
        }),
        Err(error) => Err(error),
    }
}

pub fn mine_dev_block(
    config_path: &Path,
    data_dir: &Path,
    miner_address: &str,
) -> NodeResult<ChainSummary> {
    mine_dev_blocks(config_path, data_dir, miner_address, 1)
}

pub fn mine_dev_blocks(
    config_path: &Path,
    data_dir: &Path,
    miner_address: &str,
    count: u64,
) -> NodeResult<ChainSummary> {
    let (config, mut blocks, _) = load_validated_chain(config_path, data_dir)?;
    let mut last_block = blocks
        .last()
        .cloned()
        .ok_or_else(|| NodeError::ChainNotInitialized(storage::chain_file_path(data_dir)))?;

    for _ in 0..count {
        let next_block = child_block_with_consensus_difficulty(
            &blocks,
            miner_address,
            last_block.header.timestamp + config.block_time_seconds,
            vec![],
            config.difficulty_leading_zero_bits,
        )?;
        storage::append_block(data_dir, &next_block)?;
        last_block = next_block.clone();
        blocks.push(next_block);
    }

    summarize_validated_blocks(config_path, &config, data_dir, &blocks)
}

pub fn mine_pending_block(
    config_path: &Path,
    data_dir: &Path,
    mempool_dir: &Path,
    miner_address: &str,
) -> NodeResult<MinePendingBlockSummary> {
    let config = NetworkConfig::load_from_path(config_path)?;
    let mut template = create_block_template(
        config_path,
        data_dir,
        mempool_dir,
        miner_address,
        config.max_mempool_transactions,
    )?;
    if template.included_tx_hashes.is_empty() {
        return Err(NodeError::Input(
            "no valid pending transactions are available to mine".to_owned(),
        ));
    }

    mine_core_block(&mut template.block);
    let submitted = submit_mined_block(config_path, data_dir, mempool_dir, &template.block)?;
    let (_, _, chain) = load_validated_chain(config_path, data_dir)?;

    Ok(MinePendingBlockSummary {
        status: prototype_mode(config.network),
        block_height: submitted.block_height,
        block_hash: submitted.block_hash,
        included_tx_hashes: submitted.accepted_tx_hashes,
        skipped_tx_hashes: template.skipped_tx_hashes,
        pending_remaining: submitted.pending_remaining.unwrap_or_default(),
        miner_address: miner_address.to_owned(),
        miner_balance_atomic: chain.state().balance_of(miner_address).as_atomic(),
    })
}

pub fn create_block_template(
    config_path: &Path,
    data_dir: &Path,
    mempool_dir: &Path,
    miner_address: &str,
    max_transactions: usize,
) -> NodeResult<BlockTemplate> {
    if max_transactions == 0 || max_transactions > MAX_BLOCK_TEMPLATE_TRANSACTIONS {
        return Err(NodeError::Input(format!(
            "max_transactions must be between 1 and {MAX_BLOCK_TEMPLATE_TRANSACTIONS}"
        )));
    }
    Address::parse(miner_address).map_err(|error| NodeError::Input(error.to_string()))?;

    // First FiroPoW light-context touch builds the epoch cache (can take tens of seconds on CPU).
    // Prewarm before load/hash so /mining/template does not appear hung forever.
    let _ = alvenqis_core::firopow::firopow_prewarm(0);

    let (config, blocks, chain) = load_validated_chain(config_path, data_dir)?;
    let pending_records = load_pending_transactions(mempool_dir)?;
    // Leave room for the coinbase under the consensus hard cap.
    let consensus_user_tx_cap = MAX_TRANSACTIONS_PER_BLOCK.saturating_sub(1);
    let limit = max_transactions
        .min(config.max_mempool_transactions)
        .min(consensus_user_tx_cap);
    // Prefer higher effective priority fees (multi-pass, nonce-safe).
    let (selected, skipped_tx_hashes) =
        crate::mempool::select_pending_for_template(&chain, pending_records, limit)?;
    let included_tx_hashes = selected
        .iter()
        .map(|record| record.tx_hash.clone())
        .collect();
    let transactions: Vec<Transaction> = selected
        .into_iter()
        .map(|record| record.transaction)
        .collect();
    let previous = blocks
        .last()
        .ok_or_else(|| NodeError::ChainNotInitialized(storage::chain_file_path(data_dir)))?;
    let base_fee = next_base_fee(Some(previous));
    let priority_fees = transactions
        .iter()
        .try_fold(Amount::ZERO, |total, transaction| {
            transaction.validate_fee_against_base_fee(base_fee)?;
            total.checked_add(transaction.effective_priority_fee(base_fee)?)
        })?;
    let mut block_transactions = Vec::with_capacity(transactions.len() + 1);
    block_transactions.push(Transaction::coinbase(
        previous.header.height + 1,
        miner_address.to_owned(),
        block_reward(previous.header.height + 1).checked_add(priority_fees)?,
    )?);
    block_transactions.extend(transactions);
    let created_at_unix_seconds = current_unix_seconds();
    // Timestamp floor: max(now, previous+1, MTP+1) so templates always meet consensus MTP.
    let mtp_floor = median_time_past(&blocks)
        .map(|m| m.saturating_add(1))
        .unwrap_or(0);
    let min_timestamp = previous.header.timestamp.saturating_add(1).max(mtp_floor);
    let block_timestamp = created_at_unix_seconds.max(min_timestamp);
    let previous_hash = previous.hash()?;
    let block = Block::new(
        config.network,
        previous.header.height + 1,
        previous_hash,
        base_fee.as_atomic(),
        block_timestamp,
        next_difficulty_for_network(config.network, &blocks, config.difficulty_leading_zero_bits),
        block_transactions,
    )?;

    Ok(BlockTemplate {
        network_id: config.network_id,
        tip_hash: hash_to_hex(&previous_hash),
        created_at_unix_seconds,
        block,
        included_tx_hashes,
        skipped_tx_hashes,
    })
}

pub fn submit_mined_block(
    config_path: &Path,
    data_dir: &Path,
    mempool_dir: &Path,
    candidate: &Block,
) -> NodeResult<SubmittedMinedBlock> {
    let config = NetworkConfig::load_from_path(config_path)?;
    ensure_network_storage_path(config.network, data_dir)?;
    let store = SqliteBlockStore::new(data_dir);
    store.append_validated(candidate, |blocks, candidate| {
        let mut chain = build_validated_chain(config_path, &config, blocks)?;
        chain.append_block(candidate.clone())?;
        Ok(())
    })?;

    let accepted_tx_hashes: Vec<String> = candidate
        .transactions
        .iter()
        .skip(1)
        .map(tx_hash_string)
        .collect();
    let accepted: std::collections::BTreeSet<&str> =
        accepted_tx_hashes.iter().map(String::as_str).collect();
    let cleanup = crate::mempool::with_mempool_lock(mempool_dir, || {
        let records = load_pending_transactions(mempool_dir)?;
        let remaining: Vec<_> = records
            .into_iter()
            .filter(|record| !accepted.contains(record.tx_hash.as_str()))
            .collect();
        crate::mempool::write_pending_transactions_in_lock(mempool_dir, &remaining)?;
        Ok(remaining.len())
    });

    Ok(SubmittedMinedBlock {
        status: prototype_mode(config.network),
        block_height: candidate.header.height,
        block_hash: hash_to_hex(&candidate.hash()?),
        accepted_tx_hashes,
        pending_remaining: cleanup.as_ref().ok().copied(),
        mempool_cleanup_complete: cleanup.is_ok(),
    })
}

pub fn validate_chain(config_path: &Path, data_dir: &Path) -> NodeResult<ChainSummary> {
    let (config, blocks, _) = load_validated_chain(config_path, data_dir)?;
    summarize_validated_blocks(config_path, &config, data_dir, &blocks)
}

pub fn adopt_candidate_chain(
    config_path: &Path,
    data_dir: &Path,
    mempool_dir: &Path,
    candidate_blocks: &[Block],
) -> NodeResult<ChainReorgSummary> {
    let config = NetworkConfig::load_from_path(config_path)?;
    ensure_network_storage_path(config.network, data_dir)?;
    let store = SqliteBlockStore::new(data_dir);
    let observed = store.load_blocks()?;
    let expected_tip = match observed.last() {
        Some(block) => hash_to_hex(&block.hash()?),
        None => {
            return Err(NodeError::ChainNotInitialized(storage::chain_file_path(
                data_dir,
            )))
        }
    };

    let (mut summary, detached, validated_candidate) = store.replace_validated(
        &expected_tip,
        candidate_blocks,
        |current_blocks, replacement| {
            let current_chain = build_validated_chain(config_path, &config, current_blocks)?;
            let candidate_chain = build_validated_chain(config_path, &config, replacement)?;
            if select_fork(current_blocks, replacement)? != ForkChoice::AdoptCandidate {
                return Err(NodeError::Input(
                    "candidate chain does not have strictly greater cumulative proof of work"
                        .to_owned(),
                ));
            }
            let ancestor = match common_ancestor_height(current_blocks, replacement)? {
                Some(height) => height,
                None => {
                    return Err(NodeError::GenesisMismatch {
                        expected: hash_to_hex(&current_blocks[0].hash()?),
                        actual: hash_to_hex(&replacement[0].hash()?),
                    })
                }
            };
            let detached: Vec<Block> = current_blocks
                .iter()
                .filter(|block| block.header.height > ancestor)
                .cloned()
                .collect();
            let attached_blocks = replacement
                .iter()
                .filter(|block| block.header.height > ancestor)
                .count();
            let new_tip_hash = match replacement.last() {
                Some(block) => hash_to_hex(&block.hash()?),
                None => String::new(),
            };
            let summary = ChainReorgSummary {
                common_ancestor_height: ancestor,
                detached_blocks: detached.len(),
                attached_blocks,
                previous_tip_hash: expected_tip.clone(),
                new_tip_hash,
                previous_chain_work: current_chain.cumulative_work()?,
                new_chain_work: candidate_chain.cumulative_work()?,
                dropped_mempool_transactions: Vec::new(),
            };
            Ok((summary, detached, candidate_chain))
        },
    )?;

    summary.dropped_mempool_transactions = reconcile_after_reorg(
        mempool_dir,
        &validated_candidate,
        &detached,
        config.max_mempool_transactions,
    )?;
    Ok(summary)
}

pub fn print_chain(config_path: &Path, data_dir: &Path) -> NodeResult<String> {
    let (config, blocks, chain) = load_validated_chain(config_path, data_dir)?;
    let summary = summarize_chain(&config, &blocks, &chain)?;

    let mut output = String::new();
    // fmt::Write to String is infallible in practice; still avoid expect/panic in production.
    let _ = writeln!(
        &mut output,
        "{} [{}] ({}) height={} blocks={} tip={}",
        summary.network_name,
        summary.network_id,
        summary.status,
        summary.height,
        summary.block_count,
        summary.tip_hash
    );

    for block in blocks {
        let _ = writeln!(
            &mut output,
            "network_id={} height={} timestamp={} nonce={} difficulty={} txs={} hash={} prev={}",
            block.header.network_id,
            block.header.height,
            block.header.timestamp,
            block.header.nonce,
            block.header.difficulty_leading_zero_bits,
            block.transactions.len(),
            hash_to_hex(&block.hash()?),
            hash_to_hex(&block.header.previous_hash),
        );

        if let Some(fees) = chain.state().block_fees().get(&block.header.height) {
            let reward = chain
                .state()
                .coinbase_rewards()
                .get(&block.header.height)
                .copied()
                .unwrap_or_default();
            let _ = writeln!(
                &mut output,
                "  reward_atomic={} fees_atomic={}",
                reward.as_atomic(),
                fees.as_atomic()
            );
        }
    }

    Ok(output)
}

pub fn balance(config_path: &Path, data_dir: &Path, address: &str) -> NodeResult<BalanceSummary> {
    let (_config, _blocks, chain) = load_validated_chain(config_path, data_dir)?;
    let balance = chain.state().balance_of(address);
    let next_nonce = chain.state().next_nonce_of(address);
    Ok(BalanceSummary {
        address: address.to_owned(),
        balance_atomic: balance.as_atomic(),
        exists: chain.state().balances().contains_key(address)
            || next_nonce > alvenqis_core::FIRST_ACCOUNT_NONCE,
        next_nonce,
    })
}

pub fn state(config_path: &Path, data_dir: &Path) -> NodeResult<StateSummary> {
    let (config, blocks, chain) = load_validated_chain(config_path, data_dir)?;
    state_summary(&config, &blocks, &chain)
}

pub fn send_dev_tx(
    config_path: &Path,
    data_dir: &Path,
    from_private_key_hex: &str,
    to: &str,
    amount_atomic: u64,
    fee_atomic: u64,
    miner_address: &str,
) -> NodeResult<SendTransactionSummary> {
    let private_key = PrivateKey::from_hex(from_private_key_hex)
        .map_err(|error| NodeError::Input(error.to_string()))?;
    Address::parse(to).map_err(|error| NodeError::Input(error.to_string()))?;
    let (config, blocks, mut chain) = load_validated_chain(config_path, data_dir)?;
    let last_block = blocks
        .last()
        .cloned()
        .ok_or_else(|| NodeError::ChainNotInitialized(storage::chain_file_path(data_dir)))?;

    let from =
        Address::from_public_key_for_network(&private_key.public_key(), config.network).to_string();
    let nonce = next_account_nonce(&blocks, &from);
    let anticipated_base_fee = next_base_fee(Some(&last_block));
    let transaction = Transaction::new_signed(
        1,
        nonce,
        config.network,
        &private_key,
        to.to_owned(),
        Amount::from_atomic(amount_atomic),
        Amount::from_atomic(anticipated_base_fee.as_atomic().saturating_add(fee_atomic)),
        Amount::from_atomic(fee_atomic),
        None,
    )?;
    let next_block = child_block_with_consensus_difficulty(
        &blocks,
        miner_address,
        last_block.header.timestamp + config.block_time_seconds,
        vec![transaction.clone()],
        config.difficulty_leading_zero_bits,
    )?;

    chain.append_block(next_block.clone())?;
    storage::append_block(data_dir, &next_block)?;
    let effective_fee_atomic = transaction.effective_fee(anticipated_base_fee)?.as_atomic();

    Ok(SendTransactionSummary {
        status: prototype_mode(config.network),
        tx_hash: hash_to_hex(&transaction.tx_hash()),
        block_height: next_block.header.height,
        block_hash: hash_to_hex(&next_block.hash()?),
        from: from.clone(),
        to: to.to_owned(),
        amount_atomic,
        max_fee_atomic: transaction.max_fee.as_atomic(),
        priority_fee_atomic: transaction.priority_fee.as_atomic(),
        effective_fee_atomic,
        fee_atomic,
        sender_balance_atomic: chain.state().balance_of(&from).as_atomic(),
        recipient_balance_atomic: chain.state().balance_of(to).as_atomic(),
        miner_address: miner_address.to_owned(),
    })
}

pub fn submit_transaction(
    data_dir: &Path,
    mempool_dir: &Path,
    max_mempool_transactions: usize,
    transaction: &Transaction,
) -> NodeResult<SubmitTransactionSummary> {
    let chain = load_chain_only(data_dir)?;
    let tx_hash = tx_hash_string(transaction);
    if chain
        .blocks()
        .iter()
        .flat_map(|block| block.transactions.iter())
        .any(|existing| tx_hash_string(existing) == tx_hash)
    {
        return Err(NodeError::Input(format!(
            "transaction {tx_hash} already exists in the local chain"
        )));
    }

    let transaction_network = transaction.network()?;
    if transaction_network != chain.network() {
        return Err(NodeError::NetworkMismatch {
            expected: chain.network().network_id().to_owned(),
            actual: transaction_network.network_id().to_owned(),
        });
    }

    crate::mempool::with_mempool_lock(mempool_dir, || {
        let existing_records = load_pending_transactions(mempool_dir)?;
        let (mut valid_records, _invalid_hashes, mut pending_state) =
            sanitize_pending_transactions(&chain, existing_records)?;
        if valid_records.iter().any(|record| record.tx_hash == tx_hash) {
            return Err(NodeError::Input(format!(
                "transaction {tx_hash} already exists in the local mempool"
            )));
        }

        // Per-sender pending cap: drop lowest-tip txs from the same sender first.
        if let Some(sender) = transaction.from.as_deref() {
            let mut same_sender: Vec<usize> = valid_records
                .iter()
                .enumerate()
                .filter(|(_, r)| r.transaction.from.as_deref() == Some(sender))
                .map(|(i, _)| i)
                .collect();
            if same_sender.len() >= MAX_PENDING_TXS_PER_SENDER {
                let anticipated = next_base_fee(chain.blocks().last());
                same_sender.sort_by(|&a, &b| {
                    let ta = valid_records[a]
                        .transaction
                        .effective_priority_fee(anticipated)
                        .map(|x| x.as_atomic())
                        .unwrap_or(0);
                    let tb = valid_records[b]
                        .transaction
                        .effective_priority_fee(anticipated)
                        .map(|x| x.as_atomic())
                        .unwrap_or(0);
                    ta.cmp(&tb).then_with(|| {
                        valid_records[a]
                            .received_at_unix_seconds
                            .cmp(&valid_records[b].received_at_unix_seconds)
                    })
                });
                // Keep room for the new tx.
                let drop_count = same_sender
                    .len()
                    .saturating_add(1)
                    .saturating_sub(MAX_PENDING_TXS_PER_SENDER);
                let mut drop_idxs: Vec<usize> = same_sender.into_iter().take(drop_count).collect();
                drop_idxs.sort_unstable_by(|a, b| b.cmp(a));
                for idx in drop_idxs {
                    valid_records.remove(idx);
                }
                let (rebuilt, _, new_state) = sanitize_pending_transactions(&chain, valid_records)?;
                valid_records = rebuilt;
                pending_state = new_state;
            }
        }

        let anticipated_base_fee = next_base_fee(chain.blocks().last());
        // When full, evict the lowest-fee *sender package* so higher tips can enter (TM-501).
        if valid_records.len() >= max_mempool_transactions {
            let incoming_tip = transaction
                .effective_priority_fee(anticipated_base_fee)
                .map(|a| a.as_atomic())
                .unwrap_or(0);
            let Some(victim_sender) =
                lowest_fee_sender_package(&valid_records, anticipated_base_fee, incoming_tip)
            else {
                return Err(NodeError::MempoolFull {
                    limit: max_mempool_transactions,
                });
            };
            valid_records.retain(|record| {
                record.transaction.from.as_deref() != Some(victim_sender.as_str())
            });
            // Re-sanitize after bulk package eviction.
            let (rebuilt, _, new_state) = sanitize_pending_transactions(&chain, valid_records)?;
            valid_records = rebuilt;
            pending_state = new_state;
            if valid_records.len() >= max_mempool_transactions {
                return Err(NodeError::MempoolFull {
                    limit: max_mempool_transactions,
                });
            }
        }

        validate_pending_transaction(&pending_state, transaction, anticipated_base_fee)?;
        apply_transaction(&mut pending_state, transaction, anticipated_base_fee)?;
        valid_records.push(PendingTransactionRecord {
            tx_hash: tx_hash.clone(),
            received_at_unix_seconds: current_unix_seconds(),
            transaction: transaction.clone(),
        });
        // Already holding the exclusive mempool lock; write without re-locking.
        crate::mempool::write_pending_transactions_in_lock(mempool_dir, &valid_records)?;

        Ok(SubmitTransactionSummary {
            status: prototype_mode(chain.network()),
            tx_hash: tx_hash.clone(),
            lifecycle_status: "pending".to_owned(),
            mempool_size: valid_records.len(),
        })
    })
}

pub fn mempool_status(data_dir: &Path, mempool_dir: &Path) -> NodeResult<MempoolStatusSummary> {
    let chain = load_chain_only(data_dir)?;
    let (valid_records, anticipated_base_fee) =
        crate::mempool::with_mempool_lock(mempool_dir, || {
            let pending_records = load_pending_transactions(mempool_dir)?;
            let (valid_records, _invalid_hashes, _state) =
                sanitize_pending_transactions(&chain, pending_records)?;
            crate::mempool::write_pending_transactions_in_lock(mempool_dir, &valid_records)?;
            let anticipated_base_fee = next_base_fee(chain.blocks().last());
            Ok((valid_records, anticipated_base_fee))
        })?;

    let highest_priority_fee_atomic = valid_records
        .iter()
        .filter_map(|record| {
            record
                .transaction
                .effective_priority_fee(anticipated_base_fee)
                .ok()
                .map(|a| a.as_atomic())
        })
        .max()
        .unwrap_or(0);
    let highest_max_fee_atomic = valid_records
        .iter()
        .map(|record| record.transaction.max_fee.as_atomic())
        .max()
        .unwrap_or(0);

    Ok(MempoolStatusSummary {
        status: prototype_mode(chain.network()),
        mempool_dir: mempool_dir.display().to_string(),
        pending_count: valid_records.len(),
        anticipated_base_fee_atomic: anticipated_base_fee.as_atomic(),
        total_fees_atomic: valid_records
            .iter()
            .map(|record| {
                record
                    .transaction
                    .effective_fee(anticipated_base_fee)
                    .map(|amount| amount.as_atomic())
                    .unwrap_or(0)
            })
            .sum(),
        total_burned_fees_atomic: valid_records
            .iter()
            .map(|record| {
                if record.transaction.is_coinbase() {
                    0
                } else {
                    anticipated_base_fee.as_atomic()
                }
            })
            .sum(),
        total_priority_fees_atomic: valid_records
            .iter()
            .map(|record| {
                record
                    .transaction
                    .effective_priority_fee(anticipated_base_fee)
                    .map(|amount| amount.as_atomic())
                    .unwrap_or(0)
            })
            .sum(),
        highest_priority_fee_atomic,
        highest_max_fee_atomic,
        pending_hashes: valid_records
            .into_iter()
            .map(|record| record.tx_hash)
            .collect(),
    })
}

pub fn reset_devnet(
    config_path: &Path,
    data_dir: &Path,
    mempool_dir: &Path,
    confirm: bool,
) -> NodeResult<ResetSummary> {
    let config = NetworkConfig::load_from_path(config_path)?;
    if !config.network.is_resettable() {
        return Err(NodeError::ResetNotAllowed(
            config.network.network_id().to_owned(),
        ));
    }
    ensure_network_storage_path(config.network, data_dir)?;
    ensure_network_storage_path(config.network, mempool_dir)?;
    if !confirm {
        return Err(NodeError::ResetConfirmationRequired(
            config.network.network_id().to_owned(),
        ));
    }
    if node_runtime_is_running(config.network, data_dir)? {
        return Err(NodeError::ResetWhileNodeRunning(
            config.network.network_id().to_owned(),
        ));
    }

    let backup_dir = backup_resettable_paths(data_dir, mempool_dir)?;
    storage::reset_data_dir(data_dir)?;
    clear_mempool(mempool_dir)?;
    Ok(ResetSummary {
        status: prototype_mode(config.network),
        network_id: config.network.network_id().to_owned(),
        data_dir: data_dir.display().to_string(),
        mempool_dir: mempool_dir.display().to_string(),
        backup_dir: backup_dir.map(|path| path.display().to_string()),
    })
}

pub fn start_node(
    config_path: &Path,
    data_dir: &Path,
    mempool_dir: &Path,
    force_genesis: bool,
) -> NodeResult<()> {
    NetworkConfig::load_from_path(config_path)?;
    let runtime_dir = runtime_dir_for_data_dir(data_dir);
    fs::create_dir_all(&runtime_dir)?;
    let shutdown_path = runtime_dir.join(NODE_SHUTDOWN_FILE_NAME);
    if shutdown_path.exists() {
        fs::remove_file(&shutdown_path)?;
    }

    let mut runtime = build_runtime_status(
        config_path,
        data_dir,
        mempool_dir,
        true,
        true,
        force_genesis,
    )?;
    write_runtime_status_file(&runtime_dir, &runtime)?;
    let (mut chain_fingerprint, mut mempool_fingerprint) =
        runtime_data_fingerprint(data_dir, mempool_dir);

    let stop_p2p = Arc::new(AtomicBool::new(false));
    let p2p_handle = {
        let stop = Arc::clone(&stop_p2p);
        let config_path = config_path.to_path_buf();
        let data_dir = data_dir.to_path_buf();
        let mempool_dir = mempool_dir.to_path_buf();
        let runtime_dir = runtime_dir.clone();
        thread::Builder::new()
            .name("alvenqis-p2p".to_owned())
            .spawn(move || run_p2p_service(config_path, data_dir, mempool_dir, runtime_dir, stop))?
    };

    loop {
        if shutdown_path.exists() {
            break;
        }

        if p2p_handle.is_finished() {
            return p2p_handle
                .join()
                .map_err(|_| NodeError::P2p("P2P worker panicked".to_owned()))?;
        }

        thread::sleep(Duration::from_secs(NODE_POLL_INTERVAL_SECONDS));
        let (next_chain_fingerprint, next_mempool_fingerprint) =
            runtime_data_fingerprint(data_dir, mempool_dir);
        if next_chain_fingerprint != chain_fingerprint {
            runtime = build_runtime_status(config_path, data_dir, mempool_dir, true, false, false)?;
            write_runtime_status_file(&runtime_dir, &runtime)?;
            chain_fingerprint = next_chain_fingerprint;
            mempool_fingerprint = next_mempool_fingerprint;
        } else if next_mempool_fingerprint != mempool_fingerprint {
            runtime.pending_count = load_pending_transactions(mempool_dir)?.len();
            write_runtime_status_file(&runtime_dir, &runtime)?;
            mempool_fingerprint = next_mempool_fingerprint;
        }
    }

    stop_p2p.store(true, Ordering::Relaxed);
    p2p_handle
        .join()
        .map_err(|_| NodeError::P2p("P2P worker panicked".to_owned()))??;
    if shutdown_path.exists() {
        fs::remove_file(&shutdown_path)?;
    }

    let stopped_runtime = NodeRuntimeStatus {
        running: false,
        pid: None,
        ..runtime
    };
    write_runtime_status_file(&runtime_dir, &stopped_runtime)?;
    Ok(())
}

pub fn node_status(
    config_path: &Path,
    data_dir: &Path,
    mempool_dir: &Path,
) -> NodeResult<NodeRuntimeStatus> {
    let mut status = build_runtime_status(config_path, data_dir, mempool_dir, false, false, false)?;
    let runtime_path = runtime_status_file_path(Path::new(&status.runtime_dir));
    if runtime_path.exists() {
        let persisted: PersistedRuntimeProcessState =
            serde_json::from_str(&fs::read_to_string(runtime_path)?)?;
        status.running = persisted
            .pid
            .is_some_and(|pid| persisted.running && process_is_running(pid));
        status.pid = status.running.then_some(persisted.pid).flatten();
    }
    Ok(status)
}

pub fn mine_block(
    config_path: &Path,
    data_dir: &Path,
    mempool_dir: &Path,
    miner_address: &str,
) -> NodeResult<String> {
    let pending = mempool_status(data_dir, mempool_dir)?;
    if pending.pending_count > 0 {
        return mine_pending_block(config_path, data_dir, mempool_dir, miner_address)
            .and_then(|summary| serde_json::to_string_pretty(&summary).map_err(NodeError::from));
    }

    mine_dev_block(config_path, data_dir, miner_address).map(|summary| {
        format!(
            "mined network_id={} height={} blocks={} tip_hash={}",
            summary.network_id, summary.height, summary.block_count, summary.tip_hash
        )
    })
}

pub fn peers(config_path: &Path, data_dir: &Path) -> NodeResult<PeersSummary> {
    let config = NetworkConfig::load_from_path(config_path)?;
    ensure_network_storage_path(config.network, data_dir)?;
    let status = load_p2p_status(&runtime_dir_for_data_dir(data_dir), &config)?;
    let local_p2p_port = config.p2p_listen_port();
    Ok(PeersSummary {
        mode: status.mode,
        network_id: config.network_id,
        local_p2p_port,
        chain_magic_hex: config.chain_magic_hex,
        connected_peers: status
            .peers
            .iter()
            .filter(|peer| peer.handshake_validated)
            .map(|peer| peer.peer_id.clone())
            .collect(),
        local_peer_id: status.local_peer_id,
        listen_addresses: status.listen_addresses,
        connected_peer_count: status.connected_peer_count,
        validated_peer_count: status.validated_peer_count,
        mining_peer_count: status.mining_peer_count,
        observed_network_hashrate_hs: status.observed_network_hashrate_hs,
        miners: status.miners,
        validating_peer_count: status.validating_peer_count,
        syncing: status.syncing,
        peers: status.peers,
        last_error: status.last_error,
    })
}

pub fn shutdown(network: Network, data_dir: &Path) -> NodeResult<String> {
    ensure_network_storage_path(network, data_dir)?;
    let runtime_dir = runtime_dir_for_data_dir(data_dir);
    let runtime_path = runtime_dir.join(NODE_RUNTIME_FILE_NAME);
    if !runtime_path.exists() {
        return Err(NodeError::ShutdownNotRunning(
            network.network_id().to_owned(),
        ));
    }

    fs::create_dir_all(&runtime_dir)?;
    fs::write(runtime_dir.join(NODE_SHUTDOWN_FILE_NAME), "shutdown\n")?;
    Ok(format!(
        "shutdown requested for network_id={} runtime_dir={}",
        network.network_id(),
        runtime_dir.display()
    ))
}

pub fn load_genesis_config(path: &Path) -> NodeResult<GenesisConfig> {
    let content = fs::read_to_string(path)?;
    let config: GenesisConfig = toml::from_str(&content)?;
    config.validate()?;
    Ok(config)
}

pub fn genesis_hash_hex_from_config(config_path: &Path) -> NodeResult<String> {
    Ok(hash_to_hex(
        &deterministic_genesis_from_config(config_path)?.hash()?,
    ))
}

pub fn genesis_review_manifest(config_path: &Path) -> NodeResult<GenesisReviewManifest> {
    let (network_config, genesis_config) = load_matching_genesis_inputs(config_path)?;
    let recipient = resolve_genesis_recipient(&network_config, &genesis_config)?;
    let genesis = genesis_with_timestamp_for_network(
        network_config.network,
        &recipient,
        genesis_config.timestamp,
        genesis_config.difficulty_leading_zero_bits,
    )?;
    let deterministic_genesis_hash = hash_to_hex(&genesis.hash()?);
    let payload = GenesisReviewPayload {
        review_standard_id: GENESIS_REVIEW_STANDARD_ID.to_owned(),
        network_id: network_config.network_id.clone(),
        human_name: network_config.genesis_review_human_name().to_owned(),
        status_label: network_config.status_label.clone(),
        address_prefix: network_config.address_prefix.clone(),
        block_time_seconds: network_config.block_time_seconds,
        difficulty_leading_zero_bits: genesis_config.difficulty_leading_zero_bits,
        chain_magic_hex: network_config.chain_magic_hex.clone(),
        genesis_timestamp: genesis_config.timestamp,
        recipient_strategy: genesis_config.recipient_strategy.clone(),
        recipient_address: genesis_config.recipient_address.clone(),
        resolved_recipient_address: recipient,
        deterministic_genesis_hash,
    };
    let review_hash = hash_to_hex(&blake3_hash(
        serde_json::to_string(&payload)
            .map_err(NodeError::from)?
            .as_bytes(),
    ));

    Ok(GenesisReviewManifest {
        review_standard_id: payload.review_standard_id,
        network_id: payload.network_id,
        human_name: payload.human_name,
        status_label: payload.status_label,
        address_prefix: payload.address_prefix,
        block_time_seconds: payload.block_time_seconds,
        difficulty_leading_zero_bits: payload.difficulty_leading_zero_bits,
        chain_magic_hex: payload.chain_magic_hex,
        genesis_timestamp: payload.genesis_timestamp,
        recipient_strategy: payload.recipient_strategy,
        recipient_address: payload.recipient_address,
        resolved_recipient_address: payload.resolved_recipient_address,
        deterministic_genesis_hash: payload.deterministic_genesis_hash,
        review_hash,
    })
}

pub fn write_genesis_review_manifest(
    config_path: &Path,
    output_path: &Path,
) -> NodeResult<GenesisReviewManifest> {
    let manifest = genesis_review_manifest(config_path)?;
    write_json_file(output_path, &manifest)?;
    Ok(manifest)
}

pub fn approve_genesis(
    config_path: &Path,
    review_path: &Path,
    approved_by: &str,
    approval_notes: Option<&str>,
    output_path: Option<&Path>,
) -> NodeResult<GenesisApprovalStatus> {
    let approved_by = approved_by.trim();
    if approved_by.is_empty() {
        return Err(NodeError::Input("approved_by cannot be empty".to_owned()));
    }

    let manifest = genesis_review_manifest(config_path)?;
    let review_content = fs::read_to_string(review_path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            NodeError::Input(format!(
                "genesis review file is missing at {}",
                review_path.display()
            ))
        } else {
            NodeError::Io(error)
        }
    })?;
    let review_manifest: GenesisReviewManifest = serde_json::from_str(&review_content)?;
    if review_manifest != manifest {
        return Err(NodeError::ConfigMismatch(
            "genesis review file does not match the active deterministic genesis inputs".to_owned(),
        ));
    }

    let network_config = NetworkConfig::load_from_path(config_path)?;
    let approval_path = output_path
        .map(PathBuf::from)
        .unwrap_or(genesis_approval_output_path(config_path, &network_config)?);
    let record = GenesisApprovalRecord {
        approval_standard_id: GENESIS_APPROVAL_STANDARD_ID.to_owned(),
        review_standard_id: manifest.review_standard_id.clone(),
        network_id: manifest.network_id.clone(),
        human_name: manifest.human_name.clone(),
        status_label: manifest.status_label.clone(),
        deterministic_genesis_hash: manifest.deterministic_genesis_hash.clone(),
        approved_review_hash: manifest.review_hash.clone(),
        approved_by: approved_by.to_owned(),
        approval_notes: approval_notes.map(str::to_owned),
        approved_at_unix_seconds: current_unix_seconds(),
    };
    write_json_file(&approval_path, &record)?;
    genesis_approval_status(config_path)
}

pub fn genesis_approval_status(config_path: &Path) -> NodeResult<GenesisApprovalStatus> {
    let config = NetworkConfig::load_from_path(config_path)?;
    let manifest = genesis_review_manifest(config_path)?;
    let approval_required = config.network.requires_explicit_allow();
    let approval_path = config
        .genesis_approval_path
        .as_deref()
        .map(|path| resolve_config_path(config_path, path));

    if !approval_required {
        return Ok(GenesisApprovalStatus {
            network_id: manifest.network_id,
            human_name: manifest.human_name,
            status_label: manifest.status_label,
            approval_required,
            approval_path: approval_path.map(|path| path.display().to_string()),
            approved: false,
            deterministic_genesis_hash: manifest.deterministic_genesis_hash,
            approved_genesis_hash: None,
            review_hash: manifest.review_hash,
            approved_review_hash: None,
            approved_by: None,
        });
    }

    let approval_path = genesis_approval_output_path(config_path, &config)?;
    let approval = load_genesis_approval_record(&approval_path)?;
    validate_genesis_approval_record(&manifest, &approval)?;
    Ok(GenesisApprovalStatus {
        network_id: manifest.network_id,
        human_name: manifest.human_name,
        status_label: manifest.status_label,
        approval_required,
        approval_path: Some(approval_path.display().to_string()),
        approved: true,
        deterministic_genesis_hash: manifest.deterministic_genesis_hash.clone(),
        approved_genesis_hash: Some(approval.deterministic_genesis_hash),
        review_hash: manifest.review_hash.clone(),
        approved_review_hash: Some(approval.approved_review_hash),
        approved_by: Some(approval.approved_by),
    })
}

fn build_validated_chain(
    config_path: &Path,
    config: &NetworkConfig,
    blocks: &[Block],
) -> NodeResult<Chain> {
    if config.network.requires_explicit_allow() {
        // Approval file check only — never re-mine genesis on the hot path
        // (create_block_template / RPC template would hang at difficulty 16).
        verify_existing_genesis(config_path, blocks)?;
    }
    Chain::from_blocks(config.network, blocks.iter().cloned()).map_err(NodeError::from)
}

fn summarize_validated_blocks(
    config_path: &Path,
    config: &NetworkConfig,
    data_dir: &Path,
    blocks: &[Block],
) -> NodeResult<ChainSummary> {
    if blocks.is_empty() {
        return Err(NodeError::ChainNotInitialized(storage::chain_file_path(
            data_dir,
        )));
    }

    let chain = build_validated_chain(config_path, config, blocks)?;

    summarize_chain(config, blocks, &chain)
}

fn summarize_chain(
    config: &NetworkConfig,
    blocks: &[Block],
    chain: &Chain,
) -> NodeResult<ChainSummary> {
    let height = chain.height().unwrap_or(0);
    let tip_hash = chain
        .tip_hash()?
        .map(|hash| hash_to_hex(&hash))
        .unwrap_or_default();
    let emitted_supply_atomic = chain.emitted_supply().as_atomic();

    Ok(ChainSummary {
        network_id: config.network.network_id().to_owned(),
        network_name: config.human_name.clone(),
        status: config.status_label.clone(),
        block_count: blocks.len(),
        height,
        tip_hash,
        emitted_supply_atomic,
    })
}

pub fn format_status(report: &StatusReport) -> String {
    match report {
        StatusReport::Uninitialized {
            network_id,
            network_name,
            status,
            data_dir,
        } => format!(
            "network_id={} network={} status={} initialized=false data_dir={}",
            network_id, network_name, status, data_dir
        ),
        StatusReport::Ready(summary) => format!(
            "network_id={} network={} status={} initialized=true height={} blocks={} tip_hash={} emitted_supply_atomic={}",
            summary.network_id,
            summary.network_name,
            summary.status,
            summary.height,
            summary.block_count,
            summary.tip_hash,
            summary.emitted_supply_atomic
        ),
    }
}

fn load_validated_chain(
    config_path: &Path,
    data_dir: &Path,
) -> NodeResult<(NetworkConfig, Vec<Block>, Chain)> {
    let config = NetworkConfig::load_from_path(config_path)?;
    ensure_network_storage_path(config.network, data_dir)?;
    // Do NOT call verified_mainnet_genesis_manifest / genesis_review_manifest here.
    // Those re-mine genesis at candidate difficulty (~16) on every template load and
    // freeze /mining/template for minutes on CPU-only validators.
    // Approval is enforced via verify_existing_genesis inside build_validated_chain.
    let blocks = storage::load_blocks(data_dir)?;
    let chain = build_validated_chain(config_path, &config, &blocks)?;
    Ok((config, blocks, chain))
}

fn load_chain_only(data_dir: &Path) -> NodeResult<Chain> {
    let blocks = storage::load_blocks(data_dir)?;
    let first_block = blocks
        .first()
        .ok_or_else(|| NodeError::ChainNotInitialized(storage::chain_file_path(data_dir)))?;
    Chain::from_blocks(first_block.network()?, blocks).map_err(NodeError::from)
}

fn state_summary(
    config: &NetworkConfig,
    _blocks: &[Block],
    chain: &Chain,
) -> NodeResult<StateSummary> {
    let latest_height = chain.height().unwrap_or(0);
    let latest_block_base_fee_atomic = chain
        .blocks()
        .last()
        .map(|block| block.header.base_fee_atomic)
        .unwrap_or_default();
    let latest_block_fees_atomic = chain
        .state()
        .block_fees()
        .get(&latest_height)
        .copied()
        .unwrap_or_default()
        .as_atomic();
    let latest_block_burned_fees_atomic = chain
        .state()
        .block_burned_fees()
        .get(&latest_height)
        .copied()
        .unwrap_or_default()
        .as_atomic();
    let latest_block_priority_fees_atomic = chain
        .state()
        .block_priority_fees()
        .get(&latest_height)
        .copied()
        .unwrap_or_default()
        .as_atomic();
    let latest_coinbase_reward_atomic = chain
        .state()
        .coinbase_rewards()
        .get(&latest_height)
        .copied()
        .unwrap_or_default()
        .as_atomic();

    Ok(StateSummary {
        status: prototype_mode(config.network),
        network_name: config.human_name.clone(),
        chain_status: config.status_label.clone(),
        height: latest_height,
        tip_hash: chain
            .tip_hash()?
            .map(|hash| hash_to_hex(&hash))
            .unwrap_or_default(),
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
            .map(|(address, balance)| StateBalanceEntry {
                address: address.clone(),
                balance_atomic: balance.as_atomic(),
                next_nonce: chain.state().next_nonce_of(address),
            })
            .collect(),
    })
}

fn prototype_mode(network: Network) -> String {
    format!("{} / Prototype", network.status_label())
}

fn next_account_nonce(blocks: &[Block], address: &str) -> u64 {
    blocks
        .iter()
        .flat_map(|block| block.transactions.iter())
        .filter(|transaction| transaction.from.as_deref() == Some(address))
        .map(|transaction| transaction.nonce)
        .max()
        .map_or(1, |nonce| nonce + 1)
}

impl GenesisConfig {
    fn validate(&self) -> NodeResult<()> {
        if self.network_id != self.network.network_id() {
            return Err(NodeError::ConfigMismatch(format!(
                "genesis network_id must be {}",
                self.network.network_id()
            )));
        }
        if self.human_name != self.network.human_name() {
            return Err(NodeError::ConfigMismatch(format!(
                "genesis human_name must be {}",
                self.network.human_name()
            )));
        }
        if self.status_label != self.network.status_label() {
            return Err(NodeError::ConfigMismatch(format!(
                "genesis status_label must be {}",
                self.network.status_label()
            )));
        }
        if self.address_prefix != self.network.address_prefix() {
            return Err(NodeError::ConfigMismatch(format!(
                "genesis address_prefix must be {}",
                self.network.address_prefix()
            )));
        }
        if self.recipient_strategy.trim().is_empty() {
            return Err(NodeError::ConfigMismatch(
                "genesis recipient_strategy cannot be empty".to_owned(),
            ));
        }
        Ok(())
    }
}

fn load_matching_genesis_inputs(config_path: &Path) -> NodeResult<(NetworkConfig, GenesisConfig)> {
    let network_config = NetworkConfig::load_from_path(config_path)?;
    let genesis_path = resolve_config_path(config_path, &network_config.genesis_config_path);
    let genesis_config = load_genesis_config(&genesis_path)?;
    if genesis_config.network != network_config.network {
        return Err(NodeError::ConfigMismatch(format!(
            "genesis network {} does not match node network {}",
            genesis_config.network.network_id(),
            network_config.network.network_id()
        )));
    }
    if genesis_config.difficulty_leading_zero_bits != network_config.difficulty_leading_zero_bits {
        return Err(NodeError::ConfigMismatch(
            "genesis difficulty must match node difficulty".to_owned(),
        ));
    }
    Ok((network_config, genesis_config))
}

fn resolve_genesis_recipient(
    network_config: &NetworkConfig,
    genesis_config: &GenesisConfig,
) -> NodeResult<String> {
    match genesis_config.recipient_strategy.as_str() {
        "default_miner_address" => Ok(default_miner_address(network_config.network)),
        "fixed_address" => {
            let address = genesis_config.recipient_address.clone().ok_or_else(|| {
                NodeError::ConfigMismatch("recipient_address is required".to_owned())
            })?;
            let parsed =
                Address::parse(&address).map_err(|error| NodeError::Input(error.to_string()))?;
            if parsed.network() != network_config.network {
                return Err(NodeError::NetworkMismatch {
                    expected: network_config.network.network_id().to_owned(),
                    actual: parsed.network().network_id().to_owned(),
                });
            }
            Ok(address)
        }
        other => Err(NodeError::ConfigMismatch(format!(
            "unsupported genesis recipient_strategy {other}"
        ))),
    }
}

fn deterministic_genesis_from_config(config_path: &Path) -> NodeResult<Block> {
    let (network_config, genesis_config) = load_matching_genesis_inputs(config_path)?;
    let recipient = resolve_genesis_recipient(&network_config, &genesis_config)?;

    genesis_with_timestamp_for_network(
        network_config.network,
        &recipient,
        genesis_config.timestamp,
        genesis_config.difficulty_leading_zero_bits,
    )
    .map_err(NodeError::from)
}

/// Export the deterministic genesis block JSON (mines once locally). Use for VPS import without re-mine.
pub fn export_genesis_block(config_path: &Path, output: &Path) -> NodeResult<Block> {
    let genesis = deterministic_genesis_from_config(config_path)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, serde_json::to_string_pretty(&genesis)?)?;
    Ok(genesis)
}

/// Import a pre-mined genesis block into an empty data_dir (no mining on server).
pub fn import_genesis_block(
    config_path: &Path,
    data_dir: &Path,
    genesis_file: &Path,
    force: bool,
) -> NodeResult<String> {
    let config = NetworkConfig::load_from_path(config_path)?;
    // Audit CR-H07: refuse destructive force wipe outside allowed network storage roots.
    ensure_network_storage_path(config.network, data_dir)?;
    let marker_path = genesis_marker_path(data_dir);
    if (marker_path.exists() || storage::chain_storage_exists(data_dir)) && !force {
        return Err(NodeError::Input(
            "genesis marker or chain database already exists; pass --force to replace chain root"
                .to_owned(),
        ));
    }
    let content = fs::read_to_string(genesis_file)?;
    let genesis: Block = serde_json::from_str(&content)?;
    if config.network.requires_explicit_allow() {
        // Candidate imports must stay fast on non-mining VPS hosts. The
        // published approval hash is authoritative, so validating the supplied
        // block against it is both stricter and cheaper than re-mining genesis.
        verify_existing_genesis(config_path, std::slice::from_ref(&genesis))?;
    } else {
        let expected = deterministic_genesis_from_config(config_path)?;
        let genesis_hash = genesis.hash()?;
        let expected_hash = expected.hash()?;
        if genesis_hash != expected_hash {
            return Err(NodeError::Input(format!(
                "imported genesis hash {} does not match config-deterministic hash {}",
                hash_to_hex(&genesis_hash),
                hash_to_hex(&expected_hash)
            )));
        }
    }
    // Wipe existing chain root when forcing — only after path allowlist check above.
    if force {
        // Prefer removing known chain artifacts; fall back to directory wipe only
        // for the validated network storage path.
        let _ = fs::remove_dir_all(data_dir);
    }
    storage::ensure_data_dir(data_dir)?;
    // Any pre-existing database or legacy JSONL requires --force above. This
    // prevents an import from silently replacing or auto-migrating chain data.
    let tip_path = data_dir.join("chain-tip.json");
    if tip_path.exists() {
        fs::remove_file(&tip_path)?;
    }
    storage::append_block(data_dir, &genesis)?;
    let marker = GenesisMarker {
        network_id: config.network_id,
        genesis_hash: hash_to_hex(&genesis.hash()?),
        genesis_height: genesis.header.height,
        status_label: config.status_label,
    };
    fs::write(marker_path, serde_json::to_string_pretty(&marker)?)?;
    Ok(marker.genesis_hash)
}

fn genesis_approval_output_path(config_path: &Path, config: &NetworkConfig) -> NodeResult<PathBuf> {
    let configured_path = config.genesis_approval_path.as_deref().ok_or_else(|| {
        NodeError::ConfigMismatch("mainnet candidate requires genesis_approval_path".to_owned())
    })?;
    Ok(resolve_config_path(config_path, configured_path))
}

fn load_genesis_approval_record(path: &Path) -> NodeResult<GenesisApprovalRecord> {
    let content = fs::read_to_string(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            NodeError::Input(format!(
                "genesis approval file is missing at {}",
                path.display()
            ))
        } else {
            NodeError::Io(error)
        }
    })?;
    serde_json::from_str(&content).map_err(NodeError::from)
}

fn validate_pinned_genesis_approval(
    config: &NetworkConfig,
    approval: &GenesisApprovalRecord,
) -> NodeResult<()> {
    if approval.approval_standard_id != GENESIS_APPROVAL_STANDARD_ID {
        return Err(NodeError::ConfigMismatch(format!(
            "genesis approval standard must be {}",
            GENESIS_APPROVAL_STANDARD_ID
        )));
    }
    if approval.review_standard_id != GENESIS_REVIEW_STANDARD_ID {
        return Err(NodeError::ConfigMismatch(format!(
            "genesis approval review standard must be {}",
            GENESIS_REVIEW_STANDARD_ID
        )));
    }
    if approval.network_id != config.network_id {
        return Err(NodeError::NetworkMismatch {
            expected: config.network_id.clone(),
            actual: approval.network_id.clone(),
        });
    }
    if approval.human_name != config.genesis_review_human_name()
        || approval.status_label != config.status_label
    {
        return Err(NodeError::ConfigMismatch(
            "genesis approval metadata does not match the active network config".to_owned(),
        ));
    }
    let valid_hash = |value: &str| {
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    };
    if !valid_hash(&approval.deterministic_genesis_hash)
        || !valid_hash(&approval.approved_review_hash)
    {
        return Err(NodeError::ConfigMismatch(
            "genesis approval contains an invalid canonical hash".to_owned(),
        ));
    }
    if approval.approved_by.trim().is_empty() {
        return Err(NodeError::ConfigMismatch(
            "genesis approval approved_by cannot be empty".to_owned(),
        ));
    }
    Ok(())
}

/// Reads the already reviewed approval record without re-mining genesis.
/// Runtime telemetry calls this on every chain/mempool change; the explicit
/// governance command still recomputes the deterministic review manifest.
fn pinned_genesis_approval_status(config_path: &Path) -> NodeResult<GenesisApprovalStatus> {
    let (config, genesis_config) = load_matching_genesis_inputs(config_path)?;
    if !config.network.requires_explicit_allow() {
        return genesis_approval_status(config_path);
    }
    resolve_genesis_recipient(&config, &genesis_config)?;
    let approval_path = genesis_approval_output_path(config_path, &config)?;
    let approval = load_genesis_approval_record(&approval_path)?;
    validate_pinned_genesis_approval(&config, &approval)?;
    Ok(GenesisApprovalStatus {
        network_id: config.network_id,
        human_name: config.human_name,
        status_label: config.status_label,
        approval_required: true,
        approval_path: Some(approval_path.display().to_string()),
        approved: true,
        deterministic_genesis_hash: approval.deterministic_genesis_hash.clone(),
        approved_genesis_hash: Some(approval.deterministic_genesis_hash),
        review_hash: approval.approved_review_hash.clone(),
        approved_review_hash: Some(approval.approved_review_hash),
        approved_by: Some(approval.approved_by),
    })
}

fn validate_genesis_approval_record(
    manifest: &GenesisReviewManifest,
    approval: &GenesisApprovalRecord,
) -> NodeResult<()> {
    if approval.approval_standard_id != GENESIS_APPROVAL_STANDARD_ID {
        return Err(NodeError::ConfigMismatch(format!(
            "genesis approval standard must be {}",
            GENESIS_APPROVAL_STANDARD_ID
        )));
    }
    if approval.review_standard_id != manifest.review_standard_id {
        return Err(NodeError::ConfigMismatch(format!(
            "genesis approval review standard must be {}",
            manifest.review_standard_id
        )));
    }
    if approval.network_id != manifest.network_id {
        return Err(NodeError::NetworkMismatch {
            expected: manifest.network_id.clone(),
            actual: approval.network_id.clone(),
        });
    }
    if approval.deterministic_genesis_hash != manifest.deterministic_genesis_hash {
        return Err(NodeError::ConfigMismatch(format!(
            "approved genesis hash mismatch: expected {}, got {}",
            manifest.deterministic_genesis_hash, approval.deterministic_genesis_hash
        )));
    }
    if approval.approved_review_hash != manifest.review_hash {
        return Err(NodeError::ConfigMismatch(format!(
            "approved genesis review hash mismatch: expected {}, got {}",
            manifest.review_hash, approval.approved_review_hash
        )));
    }
    if approval.approved_by.trim().is_empty() {
        return Err(NodeError::ConfigMismatch(
            "genesis approval approved_by cannot be empty".to_owned(),
        ));
    }
    Ok(())
}

fn verified_mainnet_genesis_manifest(
    config_path: &Path,
) -> NodeResult<Option<(GenesisReviewManifest, GenesisApprovalRecord, PathBuf)>> {
    let config = NetworkConfig::load_from_path(config_path)?;
    if !config.network.requires_explicit_allow() {
        return Ok(None);
    }

    let manifest = genesis_review_manifest(config_path)?;
    let approval_path = genesis_approval_output_path(config_path, &config)?;
    let approval = load_genesis_approval_record(&approval_path)?;
    validate_genesis_approval_record(&manifest, &approval)?;
    Ok(Some((manifest, approval, approval_path)))
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> NodeResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

fn build_runtime_status(
    config_path: &Path,
    data_dir: &Path,
    mempool_dir: &Path,
    running: bool,
    initialize_if_missing: bool,
    force_genesis: bool,
) -> NodeResult<NodeRuntimeStatus> {
    let config = NetworkConfig::load_from_path(config_path)?;
    ensure_network_storage_path(config.network, data_dir)?;
    ensure_network_storage_path(config.network, mempool_dir)?;
    let runtime_dir = runtime_dir_for_data_dir(data_dir);
    fs::create_dir_all(&runtime_dir)?;
    let genesis_approval = pinned_genesis_approval_status(config_path).ok();

    let summary = match storage::load_blocks(data_dir) {
        Ok(existing_blocks) => Some(summarize_validated_blocks(
            config_path,
            &config,
            data_dir,
            &existing_blocks,
        )?),
        Err(NodeError::ChainNotInitialized(_)) => {
            if initialize_if_missing {
                if config.network.requires_explicit_allow() {
                    initialize_deterministic_genesis(config_path, data_dir, force_genesis)?;
                    let blocks = storage::load_blocks(data_dir)?;
                    Some(summarize_validated_blocks(
                        config_path,
                        &config,
                        data_dir,
                        &blocks,
                    )?)
                } else {
                    None
                }
            } else {
                None
            }
        }
        Err(error) => return Err(error),
    };
    let pending = load_pending_transactions(mempool_dir)?;
    let genesis_hash = load_genesis_marker(data_dir)
        .ok()
        .map(|marker| marker.genesis_hash);

    Ok(NodeRuntimeStatus {
        mode: format!("{} / Prototype", config.status_label),
        running,
        pid: running.then(std::process::id),
        network_id: config.network_id,
        network_name: config.human_name,
        status_label: config.status_label,
        data_dir: data_dir.display().to_string(),
        mempool_dir: mempool_dir.display().to_string(),
        runtime_dir: runtime_dir.display().to_string(),
        chain_initialized: summary.is_some(),
        height: summary.as_ref().map(|inner| inner.height),
        block_count: summary.as_ref().map_or(0, |inner| inner.block_count),
        tip_hash: summary.as_ref().map(|inner| inner.tip_hash.clone()),
        emitted_supply_atomic: summary.as_ref().map(|inner| inner.emitted_supply_atomic),
        pending_count: pending.len(),
        genesis_hash,
        genesis_approval_required: config.network.requires_explicit_allow(),
        genesis_approval_path: genesis_approval
            .as_ref()
            .and_then(|status| status.approval_path.clone()),
        genesis_approved: genesis_approval
            .as_ref()
            .is_some_and(|status| status.approved),
        genesis_review_hash: genesis_approval
            .as_ref()
            .map(|status| status.review_hash.clone()),
        genesis_approved_by: genesis_approval
            .as_ref()
            .and_then(|status| status.approved_by.clone()),
    })
}

fn verify_existing_genesis(config_path: &Path, blocks: &[Block]) -> NodeResult<()> {
    let genesis = blocks
        .first()
        .ok_or_else(|| NodeError::Input("expected genesis block to exist".to_owned()))?;
    let actual_hash = genesis.hash()?;
    let actual_hex = hash_to_hex(&actual_hash);
    let (config, genesis_config) = load_matching_genesis_inputs(config_path)?;

    // Hot path: compare against GENESIS_APPROVAL only. Never call
    // genesis_review_manifest / deterministic_genesis_from_config here — those re-mine
    // at difficulty 16 and freeze public RPC (/mining/template → 504).
    if config.network.requires_explicit_allow() {
        let approval_path = genesis_approval_output_path(config_path, &config)?;
        let approval = load_genesis_approval_record(&approval_path)?;
        validate_pinned_genesis_approval(&config, &approval)?;
        let recipient = resolve_genesis_recipient(&config, &genesis_config)?;
        let coinbase_matches = genesis
            .transactions
            .first()
            .is_some_and(|transaction| transaction.is_coinbase() && transaction.to == recipient);
        if genesis.header.height != 0
            || genesis.header.network_id != config.network_id
            || genesis.header.timestamp != genesis_config.timestamp
            || genesis.header.difficulty_leading_zero_bits
                != genesis_config.difficulty_leading_zero_bits
            || !coinbase_matches
        {
            return Err(NodeError::ConfigMismatch(
                "stored genesis fields do not match the active pinned inputs".to_owned(),
            ));
        }
        let expected_hex = approval
            .deterministic_genesis_hash
            .trim()
            .to_ascii_lowercase();
        if actual_hex != expected_hex {
            return Err(NodeError::ConfigMismatch(format!(
                "genesis hash mismatch vs approval: expected {expected_hex}, got {actual_hex}"
            )));
        }
        return Ok(());
    }

    // Dev/test networks only: rebuild is cheap at low difficulty.
    let expected_hash = deterministic_genesis_from_config(config_path)?.hash()?;
    if actual_hash != expected_hash {
        return Err(NodeError::ConfigMismatch(format!(
            "genesis hash mismatch: expected {}, got {actual_hex}",
            hash_to_hex(&expected_hash),
        )));
    }
    Ok(())
}

fn initialize_deterministic_genesis(
    config_path: &Path,
    data_dir: &Path,
    force_genesis: bool,
) -> NodeResult<()> {
    let config = NetworkConfig::load_from_path(config_path)?;
    let marker_path = genesis_marker_path(data_dir);
    if !config.network.is_resettable() && marker_path.exists() && !force_genesis {
        return Err(NodeError::Input(
            "genesis marker already exists; pass --force-genesis to recreate the chain root"
                .to_owned(),
        ));
    }

    if config.network.requires_explicit_allow() {
        verified_mainnet_genesis_manifest(config_path)?;
    }
    storage::ensure_data_dir(data_dir)?;
    let genesis = deterministic_genesis_from_config(config_path)?;
    let marker = GenesisMarker {
        network_id: config.network_id,
        genesis_hash: hash_to_hex(&genesis.hash()?),
        genesis_height: genesis.header.height,
        status_label: config.status_label,
    };
    storage::append_block(data_dir, &genesis)?;
    fs::write(marker_path, serde_json::to_string_pretty(&marker)?)?;
    Ok(())
}

fn load_genesis_marker(data_dir: &Path) -> NodeResult<GenesisMarker> {
    let content = fs::read_to_string(genesis_marker_path(data_dir))?;
    serde_json::from_str(&content).map_err(NodeError::from)
}

fn genesis_marker_path(data_dir: &Path) -> PathBuf {
    data_dir
        .parent()
        .unwrap_or(data_dir)
        .join(GENESIS_MARKER_FILE_NAME)
}

fn runtime_status_file_path(runtime_dir: &Path) -> PathBuf {
    runtime_dir.join(NODE_RUNTIME_FILE_NAME)
}

fn write_runtime_status_file(runtime_dir: &Path, status: &NodeRuntimeStatus) -> NodeResult<()> {
    fs::create_dir_all(runtime_dir)?;
    fs::write(
        runtime_status_file_path(runtime_dir),
        serde_json::to_string_pretty(status)?,
    )?;
    Ok(())
}

fn file_runtime_fingerprint(path: &Path) -> (u64, Option<SystemTime>) {
    fs::metadata(path)
        .map(|metadata| (metadata.len(), metadata.modified().ok()))
        .unwrap_or((0, None))
}

fn runtime_data_fingerprint(
    data_dir: &Path,
    mempool_dir: &Path,
) -> (storage::ChainStorageFingerprint, (u64, Option<SystemTime>)) {
    (
        storage::chain_storage_fingerprint(data_dir),
        file_runtime_fingerprint(&mempool_file_path(mempool_dir)),
    )
}

fn node_runtime_is_running(network: Network, data_dir: &Path) -> NodeResult<bool> {
    ensure_network_storage_path(network, data_dir)?;
    let runtime_path = runtime_status_file_path(&runtime_dir_for_data_dir(data_dir));
    if !runtime_path.exists() {
        return Ok(false);
    }

    let status: PersistedRuntimeProcessState =
        serde_json::from_str(&fs::read_to_string(runtime_path)?)?;
    Ok(status
        .pid
        .is_some_and(|pid| status.running && process_is_running(pid)))
}

#[cfg(windows)]
fn process_is_running(pid: u32) -> bool {
    std::process::Command::new("tasklist.exe")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .ok()
        .is_some_and(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
}

#[cfg(not(windows))]
fn process_is_running(pid: u32) -> bool {
    Path::new("/proc").join(pid.to_string()).exists()
}

fn backup_resettable_paths(data_dir: &Path, mempool_dir: &Path) -> NodeResult<Option<PathBuf>> {
    let data_exists = data_dir.exists();
    let mempool_exists = mempool_dir.exists();
    if !data_exists && !mempool_exists {
        return Ok(None);
    }

    let network_root = data_dir.parent().ok_or_else(|| {
        NodeError::Input(format!(
            "cannot determine network root from data dir {}",
            data_dir.display()
        ))
    })?;
    let backup_root = network_root.join("backups").join(format!(
        "reset-{}-{}",
        current_unix_seconds(),
        std::process::id()
    ));
    fs::create_dir_all(&backup_root)?;

    move_dir_if_exists(data_dir, &backup_root.join("chain"))?;
    move_dir_if_exists(mempool_dir, &backup_root.join("mempool"))?;
    Ok(Some(backup_root))
}

fn move_dir_if_exists(source: &Path, destination: &Path) -> NodeResult<()> {
    if !source.exists() {
        return Ok(());
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    if destination.exists() {
        fs::remove_dir_all(destination)?;
    }
    fs::rename(source, destination)?;
    Ok(())
}

fn ensure_network_storage_path(network: Network, path: &Path) -> NodeResult<()> {
    let allowed_roots = [network.default_data_root(), LOCAL_OPERATOR_ROOT];
    let matches_allowed_root = path.components().any(|component| {
        allowed_roots
            .iter()
            .any(|root| component.as_os_str() == OsStr::new(root))
    });
    if matches_allowed_root {
        return Ok(());
    }

    Err(NodeError::InvalidDataPath {
        network: network.network_id().to_owned(),
        expected_root: format!("{} or {}", network.default_data_root(), LOCAL_OPERATOR_ROOT),
        actual_path: path.display().to_string(),
    })
}

fn resolve_config_path(config_path: &Path, configured_path: &str) -> PathBuf {
    let candidate = PathBuf::from(configured_path);
    if candidate.is_absolute() || candidate.exists() {
        return candidate;
    }

    if let Some(parent) = config_path.parent() {
        let joined = parent.join(&candidate);
        if joined.exists() {
            return joined;
        }

        if let Some(grandparent) = parent.parent() {
            let grandparent_joined = grandparent.join(&candidate);
            if grandparent_joined.exists() {
                return grandparent_joined;
            }
            return grandparent_joined;
        }

        return joined;
    }

    candidate
}

#[cfg(test)]
mod runtime_status_tests {
    use super::PersistedRuntimeProcessState;

    #[test]
    fn legacy_runtime_status_only_requires_process_fields() {
        let status: PersistedRuntimeProcessState = serde_json::from_str(
            r#"{"mode":"legacy","running":true,"pid":42,"network_id":"legacy"}"#,
        )
        .expect("legacy runtime status should remain readable");

        assert!(status.running);
        assert_eq!(status.pid, Some(42));
    }
}

#[cfg(test)]
mod import_genesis_path_tests {
    use super::ensure_network_storage_path;
    use alvenqis_core::Network;
    use std::path::Path;

    /// Regression CR-H07: import/force wipe only under allowlisted roots.
    #[test]
    fn network_storage_allowlist_rejects_arbitrary_paths() {
        let err = ensure_network_storage_path(
            Network::Devnet,
            Path::new("C:\\Windows\\Temp\\not-alvenqis-chain"),
        )
        .expect_err("must reject non-allowlisted path");
        let message = err.to_string().to_lowercase();
        assert!(
            message.contains("path") || message.contains("root") || message.contains("invalid"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn network_storage_allowlist_accepts_default_root_component() {
        let path = Path::new("/home/user/.alvenqis-dev/chain");
        // Devnet default root is typically `.alvenqis-dev` (component match).
        // If this environment uses a different root string, still accept local operator root.
        let result = ensure_network_storage_path(Network::Devnet, path);
        if result.is_err() {
            let local = Path::new("/tmp/.alvenqis-local/chain");
            ensure_network_storage_path(Network::Devnet, local)
                .or_else(|_| {
                    ensure_network_storage_path(
                        Network::MainnetCandidate,
                        Path::new("/data/.alvenqis-mainnet/chain"),
                    )
                })
                .expect("at least one allowlisted root pattern must pass");
        }
    }
}
