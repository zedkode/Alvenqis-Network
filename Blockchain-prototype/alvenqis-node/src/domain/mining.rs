use crate::config::NetworkConfig;
use crate::domain::chain::{
    build_validated_chain, ensure_network_storage_path, load_validated_chain, prototype_mode,
    summarize_validated_blocks, ChainSummary,
};
use crate::domain::transactions::mempool_status;
use crate::error::{NodeError, NodeResult};
use crate::mempool::{current_unix_seconds, load_pending_transactions, tx_hash_string};
use crate::storage::{self, BlockStore, SqliteBlockStore};
use alvenqis_core::{
    block_reward, child_block_with_consensus_difficulty, hash_to_hex, median_time_past,
    mine_block as mine_core_block, next_base_fee, next_difficulty_for_network, Address, Amount,
    Block, Transaction, MAX_TRANSACTIONS_PER_BLOCK,
};
use serde::Serialize;
use std::path::Path;

pub const MAX_BLOCK_TEMPLATE_TRANSACTIONS: usize = 10_000;

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
        .map(|median| median.saturating_add(1))
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
