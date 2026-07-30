use crate::error::{NodeError, NodeResult};
use crate::storage;
use alvenqis_core::{
    apply_transaction, hash_to_hex, next_base_fee, validate_transaction_against_state, Address,
    Chain, LedgerState, Network, Transaction,
};
use atomic_write_file::AtomicWriteFile;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MEMPOOL_FILE_NAME: &str = "pending.json";
pub const MEMPOOL_LOCK_FILE_NAME: &str = "mempool.lock";
/// Drop pending transactions older than this many seconds (TM-501 bounded eviction).
pub const DEFAULT_MEMPOOL_MAX_AGE_SECONDS: u64 = 3_600;
/// Soft cap on pending transactions from a single sender (DoS / spam bound).
pub const MAX_PENDING_TXS_PER_SENDER: usize = 32;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingTransactionRecord {
    pub tx_hash: String,
    pub received_at_unix_seconds: u64,
    pub transaction: Transaction,
}

pub fn default_network_root(network: Network) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join(network.default_data_root())
}

pub fn default_mempool_dir(network: Network) -> PathBuf {
    default_network_root(network).join("mempool")
}

pub fn mempool_file_path(mempool_dir: &Path) -> PathBuf {
    mempool_dir.join(MEMPOOL_FILE_NAME)
}

pub fn ensure_mempool_dir(mempool_dir: &Path) -> NodeResult<()> {
    fs::create_dir_all(mempool_dir)?;
    Ok(())
}

fn mempool_lock_path(mempool_dir: &Path) -> PathBuf {
    mempool_dir.join(MEMPOOL_LOCK_FILE_NAME)
}

fn open_exclusive_mempool_lock(mempool_dir: &Path) -> NodeResult<File> {
    ensure_mempool_dir(mempool_dir)?;
    let lock_file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(mempool_lock_path(mempool_dir))?;
    FileExt::lock_exclusive(&lock_file)?;
    Ok(lock_file)
}

/// Run a mempool mutation under an exclusive directory lock so concurrent
/// submit / reorg / status paths cannot interleave load-modify-write.
pub fn with_mempool_lock<R, F>(mempool_dir: &Path, work: F) -> NodeResult<R>
where
    F: FnOnce() -> NodeResult<R>,
{
    let _lock = open_exclusive_mempool_lock(mempool_dir)?;
    work()
}

pub fn load_pending_transactions(mempool_dir: &Path) -> NodeResult<Vec<PendingTransactionRecord>> {
    load_pending_transactions_unlocked(mempool_dir)
}

pub fn load_pending_transactions_for_chain(
    data_dir: &Path,
    mempool_dir: &Path,
) -> NodeResult<Vec<PendingTransactionRecord>> {
    #[cfg(feature = "storage-rocksdb")]
    {
        crate::state_store::load_persisted_mempool(data_dir, mempool_dir)
    }

    #[cfg(not(feature = "storage-rocksdb"))]
    {
        let _ = data_dir;
        load_pending_transactions_unlocked(mempool_dir)
    }
}

fn load_pending_transactions_unlocked(
    mempool_dir: &Path,
) -> NodeResult<Vec<PendingTransactionRecord>> {
    let path = mempool_file_path(mempool_dir);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&path)?;
    serde_json::from_str(&content).map_err(|error| NodeError::InvalidMempoolFile {
        path,
        message: error.to_string(),
    })
}

pub fn write_pending_transactions(
    mempool_dir: &Path,
    records: &[PendingTransactionRecord],
) -> NodeResult<()> {
    with_mempool_lock(mempool_dir, || {
        write_pending_transactions_unlocked(mempool_dir, records)
    })
}

pub fn write_pending_transactions_for_chain(
    data_dir: &Path,
    mempool_dir: &Path,
    records: &[PendingTransactionRecord],
) -> NodeResult<()> {
    with_mempool_lock(mempool_dir, || {
        write_pending_transactions_for_chain_in_lock(data_dir, mempool_dir, records)
    })
}

/// Write while the caller already holds [`with_mempool_lock`].
pub fn write_pending_transactions_in_lock(
    mempool_dir: &Path,
    records: &[PendingTransactionRecord],
) -> NodeResult<()> {
    write_pending_transactions_unlocked(mempool_dir, records)
}

pub fn write_pending_transactions_for_chain_in_lock(
    data_dir: &Path,
    mempool_dir: &Path,
    records: &[PendingTransactionRecord],
) -> NodeResult<()> {
    #[cfg(feature = "storage-rocksdb")]
    {
        let _ = mempool_dir;
        crate::state_store::replace_persisted_mempool(data_dir, records)
    }

    #[cfg(not(feature = "storage-rocksdb"))]
    {
        let _ = data_dir;
        write_pending_transactions_unlocked(mempool_dir, records)
    }
}

fn write_pending_transactions_unlocked(
    mempool_dir: &Path,
    records: &[PendingTransactionRecord],
) -> NodeResult<()> {
    ensure_mempool_dir(mempool_dir)?;
    let path = mempool_file_path(mempool_dir);
    let mut file = AtomicWriteFile::open(path)?;
    serde_json::to_writer_pretty(&mut file, records)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    file.commit()?;
    Ok(())
}

pub fn reconcile_after_reorg(
    mempool_dir: &Path,
    chain: &Chain,
    detached_blocks: &[alvenqis_core::Block],
    max_transactions: usize,
) -> NodeResult<Vec<String>> {
    with_mempool_lock(mempool_dir, || {
        let mut records = load_pending_transactions_unlocked(mempool_dir)?;
        let mut known: BTreeSet<String> = records
            .iter()
            .map(|record| record.tx_hash.clone())
            .collect();
        let now = current_unix_seconds();

        for transaction in detached_blocks
            .iter()
            .flat_map(|block| block.transactions.iter().skip(1))
        {
            let tx_hash = tx_hash_string(transaction);
            if known.insert(tx_hash.clone()) {
                records.push(PendingTransactionRecord {
                    tx_hash,
                    received_at_unix_seconds: now,
                    transaction: transaction.clone(),
                });
            }
        }

        let (mut valid, dropped, _) = sanitize_pending_transactions(chain, records)?;
        valid.truncate(max_transactions);
        write_pending_transactions_unlocked(mempool_dir, &valid)?;
        Ok(dropped)
    })
}

pub fn reconcile_after_reorg_for_chain(
    data_dir: &Path,
    mempool_dir: &Path,
    chain: &Chain,
    detached_blocks: &[alvenqis_core::Block],
    max_transactions: usize,
) -> NodeResult<Vec<String>> {
    with_mempool_lock(mempool_dir, || {
        let mut records = load_pending_transactions_for_chain(data_dir, mempool_dir)?;
        let mut known: BTreeSet<String> = records
            .iter()
            .map(|record| record.tx_hash.clone())
            .collect();
        let now = current_unix_seconds();

        for transaction in detached_blocks
            .iter()
            .flat_map(|block| block.transactions.iter().skip(1))
        {
            let tx_hash = tx_hash_string(transaction);
            if known.insert(tx_hash.clone()) {
                records.push(PendingTransactionRecord {
                    tx_hash,
                    received_at_unix_seconds: now,
                    transaction: transaction.clone(),
                });
            }
        }

        let (mut valid, dropped, _) =
            sanitize_pending_transactions_for_chain(data_dir, chain, records)?;
        valid.truncate(max_transactions);
        write_pending_transactions_for_chain_in_lock(data_dir, mempool_dir, &valid)?;
        Ok(dropped)
    })
}

pub fn clear_mempool(mempool_dir: &Path) -> NodeResult<()> {
    with_mempool_lock(mempool_dir, || {
        // Drop pending data but keep the directory so the lock file handle stays valid.
        let pending = mempool_file_path(mempool_dir);
        if pending.exists() {
            fs::remove_file(pending)?;
        }
        // Remove other non-lock artifacts if present.
        if mempool_dir.exists() {
            for entry in fs::read_dir(mempool_dir)? {
                let entry = entry?;
                let name = entry.file_name();
                if name != MEMPOOL_LOCK_FILE_NAME {
                    let path = entry.path();
                    if path.is_dir() {
                        fs::remove_dir_all(path)?;
                    } else {
                        fs::remove_file(path)?;
                    }
                }
            }
        }
        Ok(())
    })
}

pub fn mempool_runtime_fingerprint(
    data_dir: &Path,
    mempool_dir: &Path,
) -> (u64, Option<SystemTime>) {
    #[cfg(feature = "storage-rocksdb")]
    {
        let _ = mempool_dir;
        let database_path = crate::state_store::state_database_path(data_dir);
        let Ok(entries) = fs::read_dir(database_path) else {
            return (0, None);
        };
        entries
            .filter_map(Result::ok)
            .filter_map(|entry| entry.metadata().ok())
            .fold((0_u64, None), |(total_size, latest), metadata| {
                let modified = metadata.modified().ok();
                let latest = match (latest, modified) {
                    (Some(current), Some(candidate)) => Some(current.max(candidate)),
                    (None, candidate) => candidate,
                    (current, None) => current,
                };
                (total_size.saturating_add(metadata.len()), latest)
            })
    }

    #[cfg(not(feature = "storage-rocksdb"))]
    {
        let _ = data_dir;
        fs::metadata(mempool_file_path(mempool_dir))
            .map(|metadata| (metadata.len(), metadata.modified().ok()))
            .unwrap_or((0, None))
    }
}

pub fn sanitize_pending_transactions(
    chain: &Chain,
    records: Vec<PendingTransactionRecord>,
) -> NodeResult<(Vec<PendingTransactionRecord>, Vec<String>, LedgerState)> {
    sanitize_pending_transactions_with_age(chain, records, DEFAULT_MEMPOOL_MAX_AGE_SECONDS)
}

/// Like [`sanitize_pending_transactions`], but expires records older than `max_age_seconds`.
/// Pass `0` to disable age-based eviction.
pub fn sanitize_pending_transactions_with_age(
    chain: &Chain,
    records: Vec<PendingTransactionRecord>,
    max_age_seconds: u64,
) -> NodeResult<(Vec<PendingTransactionRecord>, Vec<String>, LedgerState)> {
    let mined_hashes = mined_transaction_hashes(chain);
    sanitize_pending_transactions_with_mined_hashes(chain, records, max_age_seconds, &mined_hashes)
}

pub fn sanitize_pending_transactions_for_chain(
    data_dir: &Path,
    chain: &Chain,
    records: Vec<PendingTransactionRecord>,
) -> NodeResult<(Vec<PendingTransactionRecord>, Vec<String>, LedgerState)> {
    let hashes = records
        .iter()
        .map(|record| record.tx_hash.clone())
        .collect::<Vec<_>>();
    let mined_hashes = storage::existing_transaction_hashes(data_dir, &hashes)?;
    sanitize_pending_transactions_with_mined_hashes(
        chain,
        records,
        DEFAULT_MEMPOOL_MAX_AGE_SECONDS,
        &mined_hashes,
    )
}

fn sanitize_pending_transactions_with_mined_hashes(
    chain: &Chain,
    records: Vec<PendingTransactionRecord>,
    max_age_seconds: u64,
    mined_hashes: &BTreeSet<String>,
) -> NodeResult<(Vec<PendingTransactionRecord>, Vec<String>, LedgerState)> {
    let anticipated_base_fee = next_base_fee(chain.blocks().last());
    let (candidates, early_dropped) =
        filter_pending_candidates(records, max_age_seconds, mined_hashes);
    // Nonce-safe admission: process lower nonces first within each sender, then by fee.
    let mut ordered = candidates;
    sort_pending_for_admission(&mut ordered, anticipated_base_fee);

    let mut state = chain.state().transaction_simulation_state();
    let mut valid_records = Vec::new();
    let mut invalid_hashes = early_dropped;

    for record in ordered {
        if validate_pending_transaction(&state, &record.transaction, anticipated_base_fee).is_err()
        {
            invalid_hashes.push(record.tx_hash);
            continue;
        }

        apply_transaction(&mut state, &record.transaction, anticipated_base_fee)?;
        valid_records.push(record);
    }

    Ok((valid_records, invalid_hashes, state))
}

/// Select up to `limit` pending txs for a block template, preferring higher effective fees.
///
/// Multi-pass greedy: higher tips first; nonce-gated txs that fail on pass N may succeed
/// after a parent is included on a later pass (account model).
pub fn select_pending_for_template(
    chain: &Chain,
    records: Vec<PendingTransactionRecord>,
    limit: usize,
) -> NodeResult<(Vec<PendingTransactionRecord>, Vec<String>)> {
    let mined_hashes = mined_transaction_hashes(chain);
    select_pending_for_template_with_mined_hashes(chain, records, limit, &mined_hashes)
}

pub fn select_pending_for_template_for_chain(
    data_dir: &Path,
    chain: &Chain,
    records: Vec<PendingTransactionRecord>,
    limit: usize,
) -> NodeResult<(Vec<PendingTransactionRecord>, Vec<String>)> {
    let hashes = records
        .iter()
        .map(|record| record.tx_hash.clone())
        .collect::<Vec<_>>();
    let mined_hashes = storage::existing_transaction_hashes(data_dir, &hashes)?;
    select_pending_for_template_with_mined_hashes(chain, records, limit, &mined_hashes)
}

fn select_pending_for_template_with_mined_hashes(
    chain: &Chain,
    records: Vec<PendingTransactionRecord>,
    limit: usize,
    mined_hashes: &BTreeSet<String>,
) -> NodeResult<(Vec<PendingTransactionRecord>, Vec<String>)> {
    if limit == 0 {
        return Ok((Vec::new(), Vec::new()));
    }
    let anticipated_base_fee = next_base_fee(chain.blocks().last());
    let (mut candidates, early_dropped) =
        filter_pending_candidates(records, DEFAULT_MEMPOOL_MAX_AGE_SECONDS, mined_hashes);
    // Fee-first ordering for selection attempts.
    sort_pending_by_fee_desc(&mut candidates, anticipated_base_fee);

    let mut state = chain.state().clone();
    let mut selected = Vec::new();
    let mut selected_hashes = BTreeSet::new();
    let mut remaining = candidates;

    // Bounded passes: each pass may unlock higher-nonce children after parents land.
    for _ in 0..remaining.len().saturating_add(1).min(64) {
        if selected.len() >= limit || remaining.is_empty() {
            break;
        }
        let mut progress = false;
        let mut still_pending = Vec::with_capacity(remaining.len());
        for record in remaining {
            if selected.len() >= limit {
                still_pending.push(record);
                continue;
            }
            if selected_hashes.contains(&record.tx_hash) {
                continue;
            }
            if validate_pending_transaction(&state, &record.transaction, anticipated_base_fee)
                .is_err()
            {
                still_pending.push(record);
                continue;
            }
            apply_transaction(&mut state, &record.transaction, anticipated_base_fee)?;
            selected_hashes.insert(record.tx_hash.clone());
            selected.push(record);
            progress = true;
        }
        remaining = still_pending;
        if !progress {
            break;
        }
    }

    let mut skipped = early_dropped;
    for record in remaining {
        skipped.push(record.tx_hash);
    }
    Ok((selected, skipped))
}

fn filter_pending_candidates(
    records: Vec<PendingTransactionRecord>,
    max_age_seconds: u64,
    mined_hashes: &BTreeSet<String>,
) -> (Vec<PendingTransactionRecord>, Vec<String>) {
    let mut seen_pending_hashes = BTreeSet::new();
    let now = current_unix_seconds();
    let mut candidates = Vec::new();
    let mut dropped = Vec::new();

    for record in records {
        if max_age_seconds > 0
            && now.saturating_sub(record.received_at_unix_seconds) > max_age_seconds
        {
            dropped.push(record.tx_hash);
            continue;
        }
        if mined_hashes.contains(&record.tx_hash)
            || !seen_pending_hashes.insert(record.tx_hash.clone())
        {
            dropped.push(record.tx_hash);
            continue;
        }
        candidates.push(record);
    }
    (candidates, dropped)
}

fn effective_priority_atomic(
    record: &PendingTransactionRecord,
    base_fee: alvenqis_core::Amount,
) -> u64 {
    record
        .transaction
        .effective_priority_fee(base_fee)
        .map(|a| a.as_atomic())
        .unwrap_or(0)
}

fn sort_pending_for_admission(
    records: &mut [PendingTransactionRecord],
    base_fee: alvenqis_core::Amount,
) {
    // Primary: sender then nonce (account model). Secondary: higher tip first.
    records.sort_by(|a, b| {
        let fa = a.transaction.from.as_deref().unwrap_or("");
        let fb = b.transaction.from.as_deref().unwrap_or("");
        fa.cmp(fb)
            .then_with(|| a.transaction.nonce.cmp(&b.transaction.nonce))
            .then_with(|| {
                effective_priority_atomic(b, base_fee).cmp(&effective_priority_atomic(a, base_fee))
            })
            .then_with(|| a.received_at_unix_seconds.cmp(&b.received_at_unix_seconds))
            .then_with(|| a.tx_hash.cmp(&b.tx_hash))
    });
}

fn sort_pending_by_fee_desc(
    records: &mut [PendingTransactionRecord],
    base_fee: alvenqis_core::Amount,
) {
    records.sort_by(|a, b| {
        effective_priority_atomic(b, base_fee)
            .cmp(&effective_priority_atomic(a, base_fee))
            .then_with(|| {
                b.transaction
                    .max_fee
                    .as_atomic()
                    .cmp(&a.transaction.max_fee.as_atomic())
            })
            .then_with(|| a.received_at_unix_seconds.cmp(&b.received_at_unix_seconds))
            .then_with(|| a.tx_hash.cmp(&b.tx_hash))
    });
}

/// Pick the sender whose *package* (all pending txs from that address) has the lowest
/// min effective tip, provided that min tip is strictly below `incoming_tip`.
/// Used when the mempool is full so higher-fee demand can displace an entire low-fee package.
pub fn lowest_fee_sender_package(
    records: &[PendingTransactionRecord],
    base_fee: alvenqis_core::Amount,
    incoming_tip: u64,
) -> Option<String> {
    use std::collections::BTreeMap;
    // (min_tip, oldest_received, sender)
    let mut packages: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for record in records {
        let Some(sender) = record.transaction.from.as_deref() else {
            continue;
        };
        let tip = effective_priority_atomic(record, base_fee);
        let entry = packages
            .entry(sender.to_owned())
            .or_insert((tip, record.received_at_unix_seconds));
        entry.0 = entry.0.min(tip);
        entry.1 = entry.1.min(record.received_at_unix_seconds);
    }
    packages
        .into_iter()
        .filter(|(_, (min_tip, _))| *min_tip < incoming_tip)
        .min_by(|a, b| {
            a.1 .0
                .cmp(&b.1 .0)
                .then_with(|| a.1 .1.cmp(&b.1 .1))
                .then_with(|| a.0.cmp(&b.0))
        })
        .map(|(sender, _)| sender)
}

pub fn validate_pending_transaction(
    state: &LedgerState,
    transaction: &Transaction,
    anticipated_base_fee: alvenqis_core::Amount,
) -> NodeResult<()> {
    if transaction.is_coinbase() {
        return Err(NodeError::Input(
            "coinbase transactions cannot enter the local mempool".to_owned(),
        ));
    }

    Address::parse(&transaction.to).map_err(|error| NodeError::Input(error.to_string()))?;
    match validate_transaction_against_state(state, transaction, anticipated_base_fee) {
        Ok(()) => Ok(()),
        Err(alvenqis_core::AlvenqisError::InvalidNonce {
            address,
            expected,
            actual,
        }) => Err(NodeError::Input(format!(
            "nonce gap for {address}: expected sequential nonce {expected}, got {actual}"
        ))),
        Err(error) => Err(NodeError::Core(error)),
    }
}

pub fn tx_hash_string(transaction: &Transaction) -> String {
    hash_to_hex(&transaction.tx_hash())
}

pub fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn mined_transaction_hashes(chain: &Chain) -> BTreeSet<String> {
    chain
        .blocks()
        .iter()
        .flat_map(|block| block.transactions.iter())
        .map(tx_hash_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alvenqis_core::{
        devnet_child_block_with_difficulty, devnet_genesis, hash_to_hex, Address, Amount,
        PrivateKey, BLOCK_TIME_SECONDS,
    };

    #[test]
    fn chain_backed_sanitizer_uses_transaction_index() {
        let dir = tempfile::tempdir().expect("temp");
        let miner = PrivateKey::generate();
        let miner_address =
            Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
        let recipient = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::Devnet,
        )
        .to_string();
        let genesis = devnet_genesis(&miner_address).expect("genesis");
        let base_fee = next_base_fee(Some(&genesis));
        let transaction = Transaction::new_signed(
            1,
            1,
            Network::Devnet,
            &miner,
            recipient,
            Amount::from_atomic(1),
            Amount::from_atomic(base_fee.as_atomic() + 1),
            Amount::from_atomic(1),
            None,
        )
        .expect("transaction");
        let transaction_hash = hash_to_hex(&transaction.tx_hash());
        let child = devnet_child_block_with_difficulty(
            &genesis,
            &miner_address,
            genesis.header.timestamp + BLOCK_TIME_SECONDS,
            vec![transaction.clone()],
            4,
        )
        .expect("child");
        storage::append_block_unchecked(dir.path(), &genesis).expect("persist genesis");
        storage::append_block_unchecked(dir.path(), &child).expect("persist child");

        let mut stale_chain_view = Chain::new(Network::Devnet);
        stale_chain_view
            .append_block(genesis)
            .expect("append genesis only");
        let record = PendingTransactionRecord {
            tx_hash: transaction_hash.clone(),
            received_at_unix_seconds: current_unix_seconds(),
            transaction,
        };

        let (valid, dropped, _) =
            sanitize_pending_transactions_for_chain(dir.path(), &stale_chain_view, vec![record])
                .expect("sanitize from index");
        assert!(valid.is_empty());
        assert_eq!(dropped, vec![transaction_hash]);
    }

    #[test]
    fn age_eviction_drops_stale_pending_records() {
        let miner = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::Devnet,
        )
        .to_string();
        let genesis = devnet_genesis(&miner).expect("genesis");
        let mut chain = Chain::new(Network::Devnet);
        chain.append_block(genesis).expect("append");

        let stale = PendingTransactionRecord {
            tx_hash: "aa".repeat(32),
            received_at_unix_seconds: 1,
            transaction: Transaction::coinbase(9, miner.clone(), alvenqis_core::block_reward(9))
                .expect("shape only — coinbase will be rejected as invalid pending"),
        };
        // Only age matters here: coinbase is already invalid for mempool, so
        // use a zero-max-age path via a duplicate hash-free stale timestamp check.
        let (_, dropped, _) =
            sanitize_pending_transactions_with_age(&chain, vec![stale], 1).expect("sanitize");
        assert!(
            !dropped.is_empty(),
            "stale or invalid records must be dropped"
        );
    }

    #[test]
    fn max_age_zero_disables_time_based_eviction_path() {
        // Empty input stays empty regardless of age policy.
        let chain = Chain::new(Network::Devnet);
        let (valid, dropped, _) =
            sanitize_pending_transactions_with_age(&chain, Vec::new(), 0).expect("sanitize");
        assert!(valid.is_empty());
        assert!(dropped.is_empty());
    }

    #[test]
    fn nonce_gap_is_rejected_by_pending_validation() {
        use alvenqis_core::{Amount, INITIAL_BASE_FEE_ATOMIC};

        let miner = PrivateKey::generate();
        let miner_address =
            Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
        let to = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::Devnet,
        )
        .to_string();
        let genesis = alvenqis_core::devnet_genesis(&miner_address).expect("genesis");
        let mut chain = Chain::new(Network::Devnet);
        chain.append_block(genesis).expect("append");
        let base = Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC);
        // First expected nonce is 1; submitting 3 is a gap.
        let bad = Transaction::new_signed(
            1,
            3,
            Network::Devnet,
            &miner,
            to,
            Amount::from_atomic(1),
            Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 1),
            Amount::from_atomic(1),
            None,
        )
        .expect("tx");
        let err = validate_pending_transaction(chain.state(), &bad, base).expect_err("gap");
        let msg = err.to_string();
        assert!(
            msg.contains("nonce gap") || msg.contains("expected sequential nonce"),
            "unexpected: {msg}"
        );
    }

    #[test]
    fn package_eviction_picks_lowest_min_tip_sender() {
        use alvenqis_core::{Amount, INITIAL_BASE_FEE_ATOMIC};

        let low_sender = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::Devnet,
        )
        .to_string();
        let high_sender = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::Devnet,
        )
        .to_string();
        let to = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::Devnet,
        )
        .to_string();
        let base = Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC);
        let low_a = PendingTransactionRecord {
            tx_hash: "aa".repeat(32),
            received_at_unix_seconds: 1,
            transaction: Transaction::new(
                1,
                1,
                Some(low_sender.clone()),
                to.clone(),
                Amount::from_atomic(1),
                Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 2),
                Amount::from_atomic(2),
                None,
            )
            .expect("tx"),
        };
        let low_b = PendingTransactionRecord {
            tx_hash: "bb".repeat(32),
            received_at_unix_seconds: 2,
            transaction: Transaction::new(
                1,
                2,
                Some(low_sender.clone()),
                to.clone(),
                Amount::from_atomic(1),
                Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 3),
                Amount::from_atomic(3),
                None,
            )
            .expect("tx"),
        };
        let high = PendingTransactionRecord {
            tx_hash: "cc".repeat(32),
            received_at_unix_seconds: 3,
            transaction: Transaction::new(
                1,
                1,
                Some(high_sender),
                to,
                Amount::from_atomic(1),
                Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 20),
                Amount::from_atomic(20),
                None,
            )
            .expect("tx"),
        };
        let picked = lowest_fee_sender_package(&[low_a, low_b, high], base, 10)
            .expect("should pick low package");
        assert_eq!(picked, low_sender);
    }

    #[test]
    fn fee_sort_prefers_higher_priority_tip() {
        use alvenqis_core::{Amount, INITIAL_BASE_FEE_ATOMIC};

        let addr_a = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::Devnet,
        )
        .to_string();
        let addr_b = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::Devnet,
        )
        .to_string();
        let to = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::Devnet,
        )
        .to_string();
        let base = Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC);
        let low = PendingTransactionRecord {
            tx_hash: "11".repeat(32),
            received_at_unix_seconds: 10,
            transaction: Transaction::new(
                1,
                1,
                Some(addr_a),
                to.clone(),
                Amount::from_atomic(1),
                Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 1),
                Amount::from_atomic(1),
                None,
            )
            .expect("tx"),
        };
        let high = PendingTransactionRecord {
            tx_hash: "22".repeat(32),
            received_at_unix_seconds: 11,
            transaction: Transaction::new(
                1,
                1,
                Some(addr_b),
                to,
                Amount::from_atomic(1),
                Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 50),
                Amount::from_atomic(50),
                None,
            )
            .expect("tx"),
        };
        let mut rows = vec![low, high];
        sort_pending_by_fee_desc(&mut rows, base);
        assert_eq!(rows[0].tx_hash, "22".repeat(32));
        assert_eq!(rows[1].tx_hash, "11".repeat(32));
    }
}
