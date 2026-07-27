use crate::error::{IndexerError, IndexerResult};
use crate::storage;
use alvenqis_core::{hash_to_hex, Amount, Chain, Network, Transaction, MAX_SUPPLY_ATOMIC};
use alvenqis_node::storage as node_storage;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const DEFAULT_MAINNET_DATA_DIR: &str = ".alvenqis-mainnet/chain";
pub const DEFAULT_DEVNET_DATA_DIR: &str = DEFAULT_MAINNET_DATA_DIR;
pub const INDEXER_MODE: &str = "Draft / Local Indexer / Prototype";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupplySummary {
    pub emitted_supply_atomic: u64,
    pub max_supply_atomic: u64,
    pub remaining_supply_atomic: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexedTransaction {
    pub lifecycle_status: String,
    pub hash: String,
    pub block_height: u64,
    pub block_hash: String,
    pub block_transaction_count: usize,
    pub transaction_index: usize,
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
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexedBlock {
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
    pub transaction_hashes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AddressActivity {
    pub address: String,
    pub exists_in_ledger: bool,
    pub balance_atomic: u64,
    pub total_received_atomic: u64,
    pub total_sent_atomic: u64,
    pub mined_reward_atomic: u64,
    pub transaction_hashes: Vec<String>,
    pub sent_tx_hashes: Vec<String>,
    pub received_tx_hashes: Vec<String>,
    pub mined_block_heights: Vec<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexData {
    pub summary: IndexSummary,
    pub blocks_by_height: BTreeMap<u64, IndexedBlock>,
    pub blocks_by_hash: BTreeMap<String, IndexedBlock>,
    pub transactions_by_hash: BTreeMap<String, IndexedTransaction>,
    pub addresses: BTreeMap<String, AddressActivity>,
    pub miner_rewards_by_block: BTreeMap<u64, u64>,
    pub fees_by_block: BTreeMap<u64, u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexerStatus {
    pub mode: String,
    pub network_id: Option<String>,
    pub status_label: Option<String>,
    pub initialized: bool,
    pub index_dir: String,
    pub indexed_height: Option<u64>,
    pub indexed_block_count: usize,
    pub transaction_count: usize,
    pub address_count: usize,
    pub tip_hash: Option<String>,
    /// Chain tip observed at status time (may differ until sync/watch runs).
    #[serde(default)]
    pub chain_height: Option<u64>,
    #[serde(default)]
    pub chain_tip_hash: Option<String>,
    /// True when index tip_hash/height matches the live chain tip.
    #[serde(default)]
    pub in_sync: bool,
    /// How many blocks the index is behind the chain (0 when in sync or unknown).
    #[serde(default)]
    pub lag_blocks: u64,
}

pub fn default_network_root(network: Network) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join(network.default_data_root())
}

pub fn default_index_dir_for_network(network: Network) -> PathBuf {
    default_network_root(network).join("indexer")
}

pub fn default_index_dir() -> PathBuf {
    default_index_dir_for_network(Network::MainnetCandidate)
}

/// Observe current chain tip without building a full index.
pub fn observe_chain_tip(chain_data_dir: &Path) -> IndexerResult<(Option<u64>, Option<String>)> {
    let blocks = node_storage::load_blocks(chain_data_dir)?;
    let tip = blocks.last();
    let tip_hash = match tip {
        Some(block) => Some(hash_to_hex(&block.hash()?)),
        None => None,
    };
    Ok((tip.map(|block| block.header.height), tip_hash))
}

/// Rebuild index from chain. Always full rebuild (safe after reorg).
pub fn index_chain(chain_data_dir: &Path, index_dir: &Path) -> IndexerResult<IndexData> {
    let blocks = node_storage::load_blocks(chain_data_dir)?;
    let first_block = blocks.first().ok_or_else(|| {
        alvenqis_node::NodeError::ChainNotInitialized(node_storage::chain_file_path(chain_data_dir))
    })?;
    let network = first_block.network()?;
    let chain = Chain::from_blocks(network, blocks.iter().cloned())?;

    let mut blocks_by_height = BTreeMap::new();
    let mut blocks_by_hash = BTreeMap::new();
    let mut transactions_by_hash = BTreeMap::new();
    let mut addresses: BTreeMap<String, AddressActivity> = BTreeMap::new();
    let mut miner_rewards_by_block = BTreeMap::new();
    let mut fees_by_block = BTreeMap::new();

    for block in &blocks {
        let height = block.header.height;
        let block_hash = hash_to_hex(&block.hash()?);
        let fees_atomic = chain
            .state()
            .block_fees()
            .get(&height)
            .copied()
            .unwrap_or(Amount::ZERO)
            .as_atomic();
        let burned_fees_atomic = chain
            .state()
            .block_burned_fees()
            .get(&height)
            .copied()
            .unwrap_or(Amount::ZERO)
            .as_atomic();
        let priority_fees_atomic = chain
            .state()
            .block_priority_fees()
            .get(&height)
            .copied()
            .unwrap_or(Amount::ZERO)
            .as_atomic();
        let miner_reward_atomic = chain
            .state()
            .coinbase_rewards()
            .get(&height)
            .copied()
            .unwrap_or(Amount::ZERO)
            .as_atomic();
        let coinbase = block.transactions.first().ok_or_else(|| {
            IndexerError::NotFound(format!("missing coinbase for block {height}"))
        })?;
        let miner_address = coinbase.to.clone();
        let coinbase_payout_atomic = coinbase.amount.as_atomic();
        let mut transaction_hashes = Vec::with_capacity(block.transactions.len());

        for (transaction_index, transaction) in block.transactions.iter().enumerate() {
            let indexed_tx = indexed_transaction(
                transaction,
                height,
                &block_hash,
                block.transactions.len(),
                transaction_index,
                Amount::from_atomic(block.header.base_fee_atomic),
            );
            let tx_hash = indexed_tx.hash.clone();
            transaction_hashes.push(tx_hash.clone());

            if let Some(sender) = &indexed_tx.from {
                let sender_entry = addresses
                    .entry(sender.clone())
                    .or_insert_with(|| empty_address_activity(sender));
                sender_entry.sent_tx_hashes.push(tx_hash.clone());
                sender_entry.transaction_hashes.push(tx_hash.clone());
                sender_entry.total_sent_atomic = sender_entry.total_sent_atomic.saturating_add(
                    transaction.amount.as_atomic() + indexed_tx.effective_fee_atomic,
                );
            }

            let recipient_entry = addresses
                .entry(indexed_tx.to.clone())
                .or_insert_with(|| empty_address_activity(&indexed_tx.to));
            recipient_entry.received_tx_hashes.push(tx_hash.clone());
            recipient_entry.transaction_hashes.push(tx_hash.clone());
            recipient_entry.total_received_atomic = recipient_entry
                .total_received_atomic
                .saturating_add(transaction.amount.as_atomic());

            if transaction.is_coinbase() {
                recipient_entry.mined_block_heights.push(height);
                recipient_entry.mined_reward_atomic = recipient_entry
                    .mined_reward_atomic
                    .saturating_add(miner_reward_atomic);
            }

            transactions_by_hash.insert(tx_hash, indexed_tx);
        }

        let indexed_block = IndexedBlock {
            height,
            hash: block_hash.clone(),
            previous_hash: hash_to_hex(&block.header.previous_hash),
            merkle_root: hash_to_hex(&block.header.merkle_root),
            timestamp: block.header.timestamp,
            nonce: block.header.nonce,
            difficulty_leading_zero_bits: block.header.difficulty_leading_zero_bits,
            transaction_count: block.transactions.len(),
            miner_address,
            coinbase_payout_atomic,
            miner_reward_atomic,
            fees_atomic,
            burned_fees_atomic,
            priority_fees_atomic,
            base_fee_atomic: block.header.base_fee_atomic,
            transaction_hashes,
        };

        miner_rewards_by_block.insert(height, miner_reward_atomic);
        fees_by_block.insert(height, fees_atomic);
        blocks_by_hash.insert(block_hash, indexed_block.clone());
        blocks_by_height.insert(height, indexed_block);
    }

    for (address, balance) in chain.state().balances() {
        let entry = addresses
            .entry(address.clone())
            .or_insert_with(|| empty_address_activity(address));
        entry.exists_in_ledger = true;
        entry.balance_atomic = balance.as_atomic();
    }

    let indexed_height = chain.height();
    let tip_hash = chain.tip_hash()?.map(|hash| hash_to_hex(&hash));
    let latest_block_timestamp = indexed_height
        .and_then(|height| blocks_by_height.get(&height))
        .map(|block| block.timestamp);

    let summary = IndexSummary {
        mode: INDEXER_MODE.to_owned(),
        network: network.network_id().to_owned(),
        status: network.status_label().to_owned(),
        indexed_height,
        indexed_block_count: blocks.len(),
        transaction_count: transactions_by_hash.len(),
        address_count: addresses.len(),
        tip_hash: tip_hash.clone(),
        latest_block_hash: tip_hash,
        latest_block_timestamp,
        supply: SupplySummary {
            emitted_supply_atomic: chain.emitted_supply().as_atomic(),
            max_supply_atomic: MAX_SUPPLY_ATOMIC,
            remaining_supply_atomic: MAX_SUPPLY_ATOMIC - chain.emitted_supply().as_atomic(),
        },
    };

    let index = IndexData {
        summary,
        blocks_by_height,
        blocks_by_hash,
        transactions_by_hash,
        addresses,
        miner_rewards_by_block,
        fees_by_block,
    };
    storage::write_index(index_dir, &index)?;
    Ok(index)
}

pub fn index_devnet(devnet_data_dir: &Path, index_dir: &Path) -> IndexerResult<IndexData> {
    index_chain(devnet_data_dir, index_dir)
}

/// Return existing index only if tip and parent-link samples still match the chain;
/// when the tip only advances with continuous parents, append incrementally;
/// otherwise full reindex (safe after reorg / tip rewrite).
/// Call this after node reorgs or any tip change (audit A-H03 / A-H06).
pub fn ensure_index_matches_chain(
    chain_data_dir: &Path,
    index_dir: &Path,
) -> IndexerResult<IndexData> {
    let (chain_height, chain_tip) = observe_chain_tip(chain_data_dir)?;
    match storage::load_index(index_dir) {
        Ok(existing)
            if existing.summary.indexed_height == chain_height
                && existing.summary.tip_hash == chain_tip
                && index_parent_links_match_chain(&existing, chain_data_dir)? =>
        {
            Ok(existing)
        }
        Ok(existing) => {
            // Tip advanced on the same fork → append-only path; else full rebuild.
            if let Some(appended) = try_append_index(&existing, chain_data_dir)? {
                storage::write_index(index_dir, &appended)?;
                return Ok(appended);
            }
            index_chain(chain_data_dir, index_dir)
        }
        Err(IndexerError::IndexNotInitialized(_)) => index_chain(chain_data_dir, index_dir),
        Err(error) => Err(error),
    }
}

/// When the live chain is a pure extension of the indexed tip, index only the new blocks
/// and refresh ledger balances from a full chain state pass.
fn try_append_index(
    existing: &IndexData,
    chain_data_dir: &Path,
) -> IndexerResult<Option<IndexData>> {
    let Some(indexed_height) = existing.summary.indexed_height else {
        return Ok(None);
    };
    let Some(indexed_tip) = existing.summary.tip_hash.as_ref() else {
        return Ok(None);
    };
    let chain_blocks = node_storage::load_blocks(chain_data_dir)?;
    if chain_blocks.is_empty() {
        return Ok(None);
    }
    let Some(on_disk_at_index) = chain_blocks.get(indexed_height as usize) else {
        return Ok(None);
    };
    if on_disk_at_index.header.height != indexed_height {
        return Ok(None);
    }
    if &hash_to_hex(&on_disk_at_index.hash()?) != indexed_tip {
        // Reorg at or below indexed tip — caller must full rebuild.
        return Ok(None);
    }
    if chain_blocks.len() as u64 == indexed_height.saturating_add(1) {
        // Already at tip (len = height+1 for 0-based heights).
        return Ok(Some(existing.clone()));
    }
    if (chain_blocks.len() as u64) <= indexed_height.saturating_add(1) {
        // Chain shorter than index → reorg rewind.
        return Ok(None);
    }
    let new_slice = &chain_blocks[(indexed_height as usize + 1)..];
    if new_slice.is_empty() {
        return Ok(Some(existing.clone()));
    }
    // First new block must extend the indexed tip.
    if hash_to_hex(&new_slice[0].header.previous_hash) != *indexed_tip {
        return Ok(None);
    }
    for window in new_slice.windows(2) {
        if hash_to_hex(&window[1].header.previous_hash) != hash_to_hex(&window[0].hash()?) {
            return Ok(None);
        }
    }

    let first_block = chain_blocks.first().ok_or_else(|| {
        alvenqis_node::NodeError::ChainNotInitialized(node_storage::chain_file_path(chain_data_dir))
    })?;
    let network = first_block.network()?;
    let chain = Chain::from_blocks(network, chain_blocks.iter().cloned())?;

    let mut index = existing.clone();
    for block in new_slice {
        append_block_to_index_maps(&mut index, block, &chain)?;
    }

    // Refresh balances / supply from the full validated chain state.
    for activity in index.addresses.values_mut() {
        activity.exists_in_ledger = false;
        activity.balance_atomic = 0;
    }
    for (address, balance) in chain.state().balances() {
        let entry = index
            .addresses
            .entry(address.clone())
            .or_insert_with(|| empty_address_activity(address));
        entry.exists_in_ledger = true;
        entry.balance_atomic = balance.as_atomic();
    }

    let tip_hash = chain.tip_hash()?.map(|hash| hash_to_hex(&hash));
    let indexed_height = chain.height();
    let latest_block_timestamp = indexed_height
        .and_then(|height| index.blocks_by_height.get(&height))
        .map(|block| block.timestamp);
    index.summary = IndexSummary {
        mode: INDEXER_MODE.to_owned(),
        network: network.network_id().to_owned(),
        status: network.status_label().to_owned(),
        indexed_height,
        indexed_block_count: chain_blocks.len(),
        transaction_count: index.transactions_by_hash.len(),
        address_count: index.addresses.len(),
        tip_hash: tip_hash.clone(),
        latest_block_hash: tip_hash,
        latest_block_timestamp,
        supply: SupplySummary {
            emitted_supply_atomic: chain.emitted_supply().as_atomic(),
            max_supply_atomic: MAX_SUPPLY_ATOMIC,
            remaining_supply_atomic: MAX_SUPPLY_ATOMIC - chain.emitted_supply().as_atomic(),
        },
    };
    Ok(Some(index))
}

fn append_block_to_index_maps(
    index: &mut IndexData,
    block: &alvenqis_core::Block,
    chain: &Chain,
) -> IndexerResult<()> {
    let height = block.header.height;
    let block_hash = hash_to_hex(&block.hash()?);
    let fees_atomic = chain
        .state()
        .block_fees()
        .get(&height)
        .copied()
        .unwrap_or(Amount::ZERO)
        .as_atomic();
    let burned_fees_atomic = chain
        .state()
        .block_burned_fees()
        .get(&height)
        .copied()
        .unwrap_or(Amount::ZERO)
        .as_atomic();
    let priority_fees_atomic = chain
        .state()
        .block_priority_fees()
        .get(&height)
        .copied()
        .unwrap_or(Amount::ZERO)
        .as_atomic();
    let miner_reward_atomic = chain
        .state()
        .coinbase_rewards()
        .get(&height)
        .copied()
        .unwrap_or(Amount::ZERO)
        .as_atomic();
    let coinbase = block
        .transactions
        .first()
        .ok_or_else(|| IndexerError::NotFound(format!("missing coinbase for block {height}")))?;
    let miner_address = coinbase.to.clone();
    let coinbase_payout_atomic = coinbase.amount.as_atomic();
    let mut transaction_hashes = Vec::with_capacity(block.transactions.len());

    for (transaction_index, transaction) in block.transactions.iter().enumerate() {
        let indexed_tx = indexed_transaction(
            transaction,
            height,
            &block_hash,
            block.transactions.len(),
            transaction_index,
            Amount::from_atomic(block.header.base_fee_atomic),
        );
        let tx_hash = indexed_tx.hash.clone();
        transaction_hashes.push(tx_hash.clone());

        if let Some(sender) = &indexed_tx.from {
            let sender_entry = index
                .addresses
                .entry(sender.clone())
                .or_insert_with(|| empty_address_activity(sender));
            sender_entry.sent_tx_hashes.push(tx_hash.clone());
            sender_entry.transaction_hashes.push(tx_hash.clone());
            sender_entry.total_sent_atomic = sender_entry
                .total_sent_atomic
                .saturating_add(transaction.amount.as_atomic() + indexed_tx.effective_fee_atomic);
        }

        let recipient_entry = index
            .addresses
            .entry(indexed_tx.to.clone())
            .or_insert_with(|| empty_address_activity(&indexed_tx.to));
        recipient_entry.received_tx_hashes.push(tx_hash.clone());
        recipient_entry.transaction_hashes.push(tx_hash.clone());
        recipient_entry.total_received_atomic = recipient_entry
            .total_received_atomic
            .saturating_add(transaction.amount.as_atomic());

        if transaction.is_coinbase() {
            recipient_entry.mined_block_heights.push(height);
            recipient_entry.mined_reward_atomic = recipient_entry
                .mined_reward_atomic
                .saturating_add(miner_reward_atomic);
        }

        index.transactions_by_hash.insert(tx_hash, indexed_tx);
    }

    let indexed_block = IndexedBlock {
        height,
        hash: block_hash.clone(),
        previous_hash: hash_to_hex(&block.header.previous_hash),
        merkle_root: hash_to_hex(&block.header.merkle_root),
        timestamp: block.header.timestamp,
        nonce: block.header.nonce,
        difficulty_leading_zero_bits: block.header.difficulty_leading_zero_bits,
        transaction_count: block.transactions.len(),
        miner_address,
        coinbase_payout_atomic,
        miner_reward_atomic,
        fees_atomic,
        burned_fees_atomic,
        priority_fees_atomic,
        base_fee_atomic: block.header.base_fee_atomic,
        transaction_hashes,
    };

    index
        .miner_rewards_by_block
        .insert(height, miner_reward_atomic);
    index.fees_by_block.insert(height, fees_atomic);
    index
        .blocks_by_hash
        .insert(block_hash, indexed_block.clone());
    index.blocks_by_height.insert(height, indexed_block);
    Ok(())
}

/// Verify tip height/hash and a trailing parent-link sample against the live chain.
fn index_parent_links_match_chain(index: &IndexData, chain_data_dir: &Path) -> IndexerResult<bool> {
    let Some(tip_height) = index.summary.indexed_height else {
        return Ok(index.summary.tip_hash.is_none());
    };
    let chain_blocks = node_storage::load_blocks(chain_data_dir)?;
    if chain_blocks.is_empty() {
        return Ok(false);
    }
    let Some(chain_tip) = chain_blocks.last() else {
        return Ok(false);
    };
    if chain_tip.header.height != tip_height {
        return Ok(false);
    }
    let chain_tip_hash = hash_to_hex(&chain_tip.hash()?);
    if index.summary.tip_hash.as_deref() != Some(chain_tip_hash.as_str()) {
        return Ok(false);
    }
    // Walk back up to 16 parents (or whole chain if shorter) for linkage integrity.
    let sample = tip_height.min(16);
    for offset in 0..=sample {
        let height = tip_height.saturating_sub(offset);
        let Some(indexed) = index.blocks_by_height.get(&height) else {
            return Ok(false);
        };
        let Some(on_chain) = chain_blocks.get(height as usize) else {
            return Ok(false);
        };
        if on_chain.header.height != height {
            return Ok(false);
        }
        if indexed.hash != hash_to_hex(&on_chain.hash()?) {
            return Ok(false);
        }
        if height > 0 {
            let parent = &chain_blocks[(height - 1) as usize];
            if indexed.previous_hash != hash_to_hex(&parent.hash()?) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Alias used by continuous watch / operator tooling.
pub fn sync_index(chain_data_dir: &Path, index_dir: &Path) -> IndexerResult<IndexData> {
    ensure_index_matches_chain(chain_data_dir, index_dir)
}

/// Poll chain tip and rebuild index when it diverges. Returns after `max_iterations` if set.
pub fn watch_index(
    chain_data_dir: &Path,
    index_dir: &Path,
    interval_seconds: u64,
    max_iterations: Option<u64>,
) -> IndexerResult<()> {
    let interval = std::time::Duration::from_secs(interval_seconds.max(1));
    let mut i = 0_u64;
    loop {
        let before = storage::load_index(index_dir).ok();
        let index = sync_index(chain_data_dir, index_dir)?;
        let rebuilt = before
            .as_ref()
            .map(|b| b.summary.tip_hash != index.summary.tip_hash)
            .unwrap_or(true);
        if rebuilt {
            eprintln!(
                "alvenqis-indexer: synced height={:?} tip={}",
                index.summary.indexed_height,
                index.summary.tip_hash.as_deref().unwrap_or("-")
            );
        }
        i = i.saturating_add(1);
        if max_iterations.is_some_and(|max| i >= max) {
            return Ok(());
        }
        std::thread::sleep(interval);
    }
}

pub fn load_index(index_dir: &Path) -> IndexerResult<IndexData> {
    storage::load_index(index_dir)
}

pub fn reset_index(index_dir: &Path) -> IndexerResult<()> {
    storage::reset_index_dir(index_dir)
}

pub fn indexer_status(index_dir: &Path) -> IndexerResult<IndexerStatus> {
    indexer_status_with_chain(index_dir, None)
}

/// Status with optional live chain tip comparison (maturity: detect lag / post-reorg staleness).
pub fn indexer_status_with_chain(
    index_dir: &Path,
    chain_data_dir: Option<&Path>,
) -> IndexerResult<IndexerStatus> {
    let (chain_height, chain_tip_hash) = match chain_data_dir {
        Some(dir) => observe_chain_tip(dir).unwrap_or((None, None)),
        None => (None, None),
    };
    match storage::load_index(index_dir) {
        Ok(index) => {
            let in_sync = chain_tip_hash.is_some()
                && index.summary.tip_hash == chain_tip_hash
                && index.summary.indexed_height == chain_height;
            let lag_blocks = match (chain_height, index.summary.indexed_height) {
                (Some(chain), Some(idx)) if chain > idx => chain - idx,
                (Some(chain), None) => chain.saturating_add(1),
                _ => 0,
            };
            Ok(IndexerStatus {
                mode: INDEXER_MODE.to_owned(),
                network_id: Some(index.summary.network.clone()),
                status_label: Some(index.summary.status.clone()),
                initialized: true,
                index_dir: index_dir.display().to_string(),
                indexed_height: index.summary.indexed_height,
                indexed_block_count: index.summary.indexed_block_count,
                transaction_count: index.summary.transaction_count,
                address_count: index.summary.address_count,
                tip_hash: index.summary.tip_hash,
                chain_height,
                chain_tip_hash,
                in_sync: chain_data_dir.is_none() || in_sync,
                lag_blocks,
            })
        }
        Err(IndexerError::IndexNotInitialized(_)) => Ok(IndexerStatus {
            mode: INDEXER_MODE.to_owned(),
            network_id: None,
            status_label: None,
            initialized: false,
            index_dir: index_dir.display().to_string(),
            indexed_height: None,
            indexed_block_count: 0,
            transaction_count: 0,
            address_count: 0,
            tip_hash: None,
            chain_height,
            chain_tip_hash: chain_tip_hash.clone(),
            in_sync: false,
            lag_blocks: chain_height.map(|h| h.saturating_add(1)).unwrap_or(0),
        }),
        Err(error) => Err(error),
    }
}

pub fn find_block(index_dir: &Path, height: u64) -> IndexerResult<IndexedBlock> {
    // Prefer indexed SQLite lookup (PRIMARY KEY on height).
    if let Some(block) = storage::find_block_db(index_dir, &height.to_string())? {
        return Ok(block);
    }
    Err(IndexerError::NotFound(format!(
        "block at height {height} not found"
    )))
}

pub fn latest_block(index_dir: &Path) -> IndexerResult<IndexedBlock> {
    let index = storage::load_index(index_dir)?;
    let height = index
        .summary
        .indexed_height
        .ok_or_else(|| IndexerError::NotFound("no indexed block available".to_owned()))?;
    find_block(index_dir, height)
}

pub fn find_transaction(index_dir: &Path, tx_hash: &str) -> IndexerResult<IndexedTransaction> {
    if let Some(tx) = storage::find_transaction_db(index_dir, tx_hash)? {
        return Ok(tx);
    }
    Err(IndexerError::NotFound(format!(
        "transaction with hash {tx_hash} not found"
    )))
}

pub fn find_address(index_dir: &Path, address: &str) -> IndexerResult<AddressActivity> {
    if let Some(activity) = storage::find_address_db(index_dir, address)? {
        return Ok(activity);
    }
    Err(IndexerError::NotFound(format!(
        "address {address} not found in index"
    )))
}

fn indexed_transaction(
    transaction: &Transaction,
    block_height: u64,
    block_hash: &str,
    block_transaction_count: usize,
    transaction_index: usize,
    base_fee: Amount,
) -> IndexedTransaction {
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
    IndexedTransaction {
        lifecycle_status: "mined".to_owned(),
        hash: hash_to_hex(&transaction.tx_hash()),
        block_height,
        block_hash: block_hash.to_owned(),
        block_transaction_count,
        transaction_index,
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
    }
}

fn empty_address_activity(address: &str) -> AddressActivity {
    AddressActivity {
        address: address.to_owned(),
        exists_in_ledger: false,
        balance_atomic: 0,
        total_received_atomic: 0,
        total_sent_atomic: 0,
        mined_reward_atomic: 0,
        transaction_hashes: Vec::new(),
        sent_tx_hashes: Vec::new(),
        received_tx_hashes: Vec::new(),
        mined_block_heights: Vec::new(),
    }
}
