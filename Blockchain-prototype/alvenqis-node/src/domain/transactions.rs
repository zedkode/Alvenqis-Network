use crate::domain::chain::{load_validated_chain, prototype_mode};
use crate::error::{NodeError, NodeResult};
use crate::mempool::{
    current_unix_seconds, load_pending_transactions, lowest_fee_sender_package,
    sanitize_pending_transactions, tx_hash_string, validate_pending_transaction,
    PendingTransactionRecord, MAX_PENDING_TXS_PER_SENDER,
};
use crate::storage;
use alvenqis_core::{
    apply_transaction, child_block_with_consensus_difficulty, hash_to_hex, next_base_fee, Address,
    Amount, Block, Chain, PrivateKey, Transaction,
};
use serde::Serialize;
use std::path::Path;

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
                .filter(|(_, record)| record.transaction.from.as_deref() == Some(sender))
                .map(|(index, _)| index)
                .collect();
            if same_sender.len() >= MAX_PENDING_TXS_PER_SENDER {
                let anticipated = next_base_fee(chain.blocks().last());
                same_sender.sort_by(|&first_index, &second_index| {
                    let first_tip = valid_records[first_index]
                        .transaction
                        .effective_priority_fee(anticipated)
                        .map(|amount| amount.as_atomic())
                        .unwrap_or(0);
                    let second_tip = valid_records[second_index]
                        .transaction
                        .effective_priority_fee(anticipated)
                        .map(|amount| amount.as_atomic())
                        .unwrap_or(0);
                    first_tip.cmp(&second_tip).then_with(|| {
                        valid_records[first_index]
                            .received_at_unix_seconds
                            .cmp(&valid_records[second_index].received_at_unix_seconds)
                    })
                });
                // Keep room for the new tx.
                let drop_count = same_sender
                    .len()
                    .saturating_add(1)
                    .saturating_sub(MAX_PENDING_TXS_PER_SENDER);
                let mut drop_indices: Vec<usize> =
                    same_sender.into_iter().take(drop_count).collect();
                drop_indices
                    .sort_unstable_by(|first_index, second_index| second_index.cmp(first_index));
                for index in drop_indices {
                    valid_records.remove(index);
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
                .map(|amount| amount.as_atomic())
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
                .map(|amount| amount.as_atomic())
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

fn load_chain_only(data_dir: &Path) -> NodeResult<Chain> {
    let blocks = storage::load_blocks(data_dir)?;
    let first_block = blocks
        .first()
        .ok_or_else(|| NodeError::ChainNotInitialized(storage::chain_file_path(data_dir)))?;
    Chain::from_blocks(first_block.network()?, blocks).map_err(NodeError::from)
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
