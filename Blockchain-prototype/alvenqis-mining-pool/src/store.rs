use crate::models::{
    PayoutBatch, PayoutItem, PayoutStatus, PoolBlock, PoolBlockStatus, PoolData, ShareRecord,
};
use crate::{PoolError, Result};
use atomic_write_file::AtomicWriteFile;
use fs2::FileExt;
use rand::{rngs::OsRng, RngCore};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Held for the lifetime of [`PoolStore`] so a second process cannot open the same data_dir.
struct ExclusiveDataDirLock {
    _file: File,
}

#[derive(Clone)]
pub struct PoolStore {
    path: PathBuf,
    inner: Arc<Mutex<PoolData>>,
    max_shares: usize,
    /// Process-wide exclusive lock on `data_dir/pool.lock` (single-instance guard).
    _data_dir_lock: Arc<ExclusiveDataDirLock>,
}

impl PoolStore {
    pub fn load(data_dir: PathBuf, max_shares: usize) -> Result<Self> {
        fs::create_dir_all(&data_dir).map_err(|error| PoolError::Storage(error.to_string()))?;
        let lock = acquire_exclusive_data_dir_lock(&data_dir)?;
        let path = data_dir.join("pool-state.json");
        let data = if path.exists() {
            serde_json::from_str(&fs::read_to_string(&path).map_err(storage)?).map_err(|error| {
                PoolError::Storage(format!("invalid {}: {error}", path.display()))
            })?
        } else {
            PoolData::default()
        };
        Ok(Self {
            path,
            inner: Arc::new(Mutex::new(data)),
            max_shares,
            _data_dir_lock: Arc::new(lock),
        })
    }

    /// Exclusive lock so two pool processes cannot race the same JSON snapshot.
    pub fn data_dir_lock_path(data_dir: &std::path::Path) -> PathBuf {
        data_dir.join("pool.lock")
    }

    pub fn snapshot(&self) -> Result<PoolData> {
        self.inner
            .lock()
            .map(|data| data.clone())
            .map_err(|_| PoolError::Storage("pool state lock poisoned".to_owned()))
    }

    /// Insert share. Identical `hash` is treated as idempotent success (same work/nonce
    /// resubmit after miner restart) — not a hard error.
    pub fn record_share(&self, mut share: ShareRecord) -> Result<(ShareRecord, bool)> {
        let mut data = self.lock()?;
        if let Some(existing) = data.shares.iter().find(|item| item.hash == share.hash) {
            return Ok((existing.clone(), true));
        }
        data.next_share_id = data.next_share_id.saturating_add(1);
        share.share_id = data.next_share_id;
        data.shares.push(share.clone());
        if data.shares.len() > self.max_shares {
            let remove = data.shares.len() - self.max_shares;
            data.shares.drain(0..remove);
        }
        self.save(&data)?;
        Ok((share, false))
    }

    pub fn record_block(
        &self,
        height: u64,
        hash: String,
        reward_atomic: u64,
        fee_basis_points: u16,
        pplns_window: usize,
        now: u64,
    ) -> Result<()> {
        let mut data = self.lock()?;
        if data.blocks.iter().any(|block| block.hash == hash) {
            return Ok(());
        }
        let pool_fee_atomic = ((reward_atomic as u128 * fee_basis_points as u128) / 10_000)
            .try_into()
            .map_err(|_| PoolError::Storage("pool fee overflow".to_owned()))?;
        let distributable_atomic = reward_atomic.saturating_sub(pool_fee_atomic);

        // Mining round = shares after the previous pool block (fair multi-miner split).
        // Fallback: last pplns_window shares if the round is empty.
        let previous_block_at = data
            .blocks
            .iter()
            .map(|block| block.found_at_unix_seconds)
            .max()
            .unwrap_or(0);
        let mut round_shares: Vec<&ShareRecord> = data
            .shares
            .iter()
            .filter(|share| {
                share.accepted_at_unix_seconds > previous_block_at
                    && share.accepted_at_unix_seconds <= now
            })
            .collect();
        if round_shares.is_empty() {
            round_shares = data
                .shares
                .iter()
                .rev()
                .filter(|share| share.accepted_at_unix_seconds <= now)
                .take(pplns_window.max(1))
                .collect();
        } else if round_shares.len() > pplns_window && pplns_window > 0 {
            // Cap very long rounds to the most recent pplns_window shares in-round.
            let skip = round_shares.len() - pplns_window;
            round_shares = round_shares.into_iter().skip(skip).collect();
        }
        if round_shares.is_empty() {
            return Err(PoolError::Storage(
                "cannot allocate a block without accepted shares".to_owned(),
            ));
        }

        let allocations = allocate_by_share_work(&round_shares, distributable_atomic)
            .map_err(PoolError::Storage)?;

        for (address, amount) in &allocations {
            let balance = data.accounts.entry(address.clone()).or_default();
            balance.immature_atomic = balance.immature_atomic.saturating_add(*amount);
        }
        data.blocks.push(PoolBlock {
            height,
            hash,
            reward_atomic,
            distributable_atomic,
            pool_fee_atomic,
            found_at_unix_seconds: now,
            status: PoolBlockStatus::Immature,
            allocations,
        });
        self.save(&data)
    }

    pub fn mature_blocks(
        &self,
        canonical_hashes: &BTreeMap<u64, String>,
        tip: u64,
        confirmations: u64,
    ) -> Result<()> {
        let mut data = self.lock()?;
        let mut changed = false;
        // (next_status, allocations, previous_status)
        let mut transitions = Vec::new();
        for block in &mut data.blocks {
            match block.status {
                PoolBlockStatus::Immature => {
                    // Early orphan if canonical hash at this height already diverged (reorg).
                    if let Some(canonical) = canonical_hashes.get(&block.height) {
                        if canonical != &block.hash {
                            transitions.push((
                                PoolBlockStatus::Orphaned,
                                block.allocations.clone(),
                                PoolBlockStatus::Immature,
                            ));
                            block.status = PoolBlockStatus::Orphaned;
                            changed = true;
                            continue;
                        }
                    }
                    if tip < block.height.saturating_add(confirmations) {
                        continue;
                    }
                    let Some(canonical) = canonical_hashes.get(&block.height) else {
                        continue;
                    };
                    let next = if canonical == &block.hash {
                        PoolBlockStatus::Mature
                    } else {
                        PoolBlockStatus::Orphaned
                    };
                    transitions.push((
                        next.clone(),
                        block.allocations.clone(),
                        PoolBlockStatus::Immature,
                    ));
                    block.status = next;
                    changed = true;
                }
                PoolBlockStatus::Mature => {
                    // Post-maturity reorg protection: if the canonical hash diverged, or the
                    // height is no longer on the tip, claw back unpaid mature balances.
                    let diverged = match canonical_hashes.get(&block.height) {
                        Some(canonical) => canonical != &block.hash,
                        None => block.height > tip,
                    };
                    if !diverged {
                        continue;
                    }
                    transitions.push((
                        PoolBlockStatus::Orphaned,
                        block.allocations.clone(),
                        PoolBlockStatus::Mature,
                    ));
                    block.status = PoolBlockStatus::Orphaned;
                    changed = true;
                }
                PoolBlockStatus::Orphaned => {}
            }
        }
        for (next_status, allocations, previous_status) in transitions {
            for (address, amount) in allocations {
                let balance = data.accounts.entry(address).or_default();
                match (previous_status.clone(), next_status.clone()) {
                    (PoolBlockStatus::Immature, PoolBlockStatus::Mature) => {
                        balance.immature_atomic = balance.immature_atomic.saturating_sub(amount);
                        balance.mature_atomic = balance.mature_atomic.saturating_add(amount);
                    }
                    (PoolBlockStatus::Immature, PoolBlockStatus::Orphaned) => {
                        balance.immature_atomic = balance.immature_atomic.saturating_sub(amount);
                    }
                    (PoolBlockStatus::Mature, PoolBlockStatus::Orphaned) => {
                        // Prefer clawing mature; never invent balance. If funds already moved to
                        // pending_payout / paid, only subtract what remains mature (honest loss
                        // is operator reconciliation — never silent double-spend invent).
                        let claw = amount.min(balance.mature_atomic);
                        balance.mature_atomic = balance.mature_atomic.saturating_sub(claw);
                    }
                    _ => {}
                }
            }
        }
        if changed {
            self.save(&data)?;
        }
        Ok(())
    }

    /// Void immature pool blocks whose height is above the new tip after a reorg rewind.
    /// Allocations return to zero immature (never matured) without inventing mature balances.
    pub fn void_immature_above_tip(&self, tip: u64) -> Result<usize> {
        let mut data = self.lock()?;
        let mut voided = 0_usize;
        let mut transitions = Vec::new();
        for block in &mut data.blocks {
            if block.status == PoolBlockStatus::Immature && block.height > tip {
                transitions.push(block.allocations.clone());
                block.status = PoolBlockStatus::Orphaned;
                voided += 1;
            }
        }
        for allocations in transitions {
            for (address, amount) in allocations {
                let balance = data.accounts.entry(address).or_default();
                balance.immature_atomic = balance.immature_atomic.saturating_sub(amount);
            }
        }
        if voided > 0 {
            self.save(&data)?;
        }
        Ok(voided)
    }

    pub fn prepare_payout(&self, minimum: u64, now: u64) -> Result<PayoutBatch> {
        let mut data = self.lock()?;
        let items = data
            .accounts
            .iter()
            .filter(|(_, balance)| balance.mature_atomic >= minimum)
            .map(|(address, balance)| PayoutItem {
                address: address.clone(),
                amount_atomic: balance.mature_atomic,
            })
            .collect::<Vec<_>>();
        if items.is_empty() {
            return Err(PoolError::Config(
                "no balances meet the payout threshold".to_owned(),
            ));
        }
        for item in &items {
            let balance = data.accounts.get_mut(&item.address).ok_or_else(|| {
                PoolError::Storage(format!(
                    "account {} disappeared while preparing payout",
                    item.address
                ))
            })?;
            balance.mature_atomic = balance.mature_atomic.saturating_sub(item.amount_atomic);
            balance.pending_payout_atomic = balance
                .pending_payout_atomic
                .saturating_add(item.amount_atomic);
        }
        let batch = PayoutBatch {
            payout_id: random_id(),
            created_at_unix_seconds: now,
            status: PayoutStatus::Prepared,
            items,
            transaction_hashes: Vec::new(),
        };
        data.payouts.push(batch.clone());
        self.save(&data)?;
        Ok(batch)
    }

    pub fn confirm_payout(&self, payout_id: &str, hashes: Vec<String>) -> Result<PayoutBatch> {
        if hashes.is_empty()
            || hashes
                .iter()
                .any(|hash| hex::decode(hash).map_or(true, |bytes| bytes.len() != 32))
        {
            return Err(PoolError::Config(
                "transaction hashes must be 32-byte hex".to_owned(),
            ));
        }
        let mut data = self.lock()?;
        if data.payouts.iter().any(|payout| {
            payout
                .transaction_hashes
                .iter()
                .any(|existing| hashes.iter().any(|hash| hash == existing))
        }) {
            return Err(PoolError::Config(
                "a payout transaction hash is already recorded".to_owned(),
            ));
        }
        let index = data
            .payouts
            .iter()
            .position(|payout| {
                payout.payout_id == payout_id && payout.status == PayoutStatus::Prepared
            })
            .ok_or_else(|| PoolError::Config("prepared payout not found".to_owned()))?;
        let items = data.payouts[index].items.clone();
        for item in &items {
            let balance = data.accounts.entry(item.address.clone()).or_default();
            balance.pending_payout_atomic = balance
                .pending_payout_atomic
                .saturating_sub(item.amount_atomic);
            balance.paid_atomic = balance.paid_atomic.saturating_add(item.amount_atomic);
        }
        data.payouts[index].status = PayoutStatus::Submitted;
        data.payouts[index].transaction_hashes = hashes;
        let result = data.payouts[index].clone();
        self.save(&data)?;
        Ok(result)
    }

    pub fn cancel_payout(&self, payout_id: &str) -> Result<PayoutBatch> {
        let mut data = self.lock()?;
        let index = data
            .payouts
            .iter()
            .position(|payout| {
                payout.payout_id == payout_id && payout.status == PayoutStatus::Prepared
            })
            .ok_or_else(|| PoolError::Config("prepared payout not found".to_owned()))?;
        let items = data.payouts[index].items.clone();
        for item in &items {
            let balance = data.accounts.entry(item.address.clone()).or_default();
            if balance.pending_payout_atomic < item.amount_atomic {
                return Err(PoolError::Storage(format!(
                    "pending payout balance is inconsistent for {}",
                    item.address
                )));
            }
            balance.pending_payout_atomic -= item.amount_atomic;
            balance.mature_atomic = balance.mature_atomic.saturating_add(item.amount_atomic);
        }
        data.payouts[index].status = PayoutStatus::Cancelled;
        let result = data.payouts[index].clone();
        self.save(&data)?;
        Ok(result)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, PoolData>> {
        self.inner
            .lock()
            .map_err(|_| PoolError::Storage("pool state lock poisoned".to_owned()))
    }

    fn save(&self, data: &PoolData) -> Result<()> {
        let mut file = AtomicWriteFile::options()
            .open(&self.path)
            .map_err(storage)?;
        file.write_all(
            &serde_json::to_vec_pretty(data)
                .map_err(|error| PoolError::Storage(error.to_string()))?,
        )
        .map_err(storage)?;
        file.sync_all().map_err(storage)?;
        file.commit().map_err(storage)
    }
}

fn acquire_exclusive_data_dir_lock(data_dir: &std::path::Path) -> Result<ExclusiveDataDirLock> {
    let lock_path = PoolStore::data_dir_lock_path(data_dir);
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|error| {
            PoolError::Storage(format!(
                "cannot open exclusive lock {}: {error}",
                lock_path.display()
            ))
        })?;
    file.try_lock_exclusive().map_err(|error| {
        PoolError::Storage(format!(
            "another alvenqis-mining-pool instance already holds an exclusive lock on {} ({error}). \
Only one process may use a given data_dir.",
            data_dir.display()
        ))
    })?;
    let _ = writeln!(
        file,
        "pid={} started_unix={}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    );
    Ok(ExclusiveDataDirLock { _file: file })
}

#[cfg(test)]
mod exclusive_lock_tests {
    use super::*;

    #[test]
    fn second_instance_fails_fast_on_same_data_dir() {
        let dir = tempfile::tempdir().expect("temp");
        let first = PoolStore::load(dir.path().to_path_buf(), 10).expect("first instance");
        let second = PoolStore::load(dir.path().to_path_buf(), 10);
        assert!(
            second.is_err(),
            "second open must fail while first holds lock"
        );
        let message = second.err().unwrap().to_string().to_lowercase();
        assert!(
            message.contains("lock") || message.contains("another"),
            "error should mention exclusive lock: {message}"
        );
        drop(first);
    }
}

fn share_work(bits: u8) -> u128 {
    1_u128.checked_shl(bits as u32).unwrap_or(u128::MAX)
}

/// Proportional reward split by proven share work (2^share_bits).
/// Dust is distributed with the largest-remainder method (not all to one miner).
fn allocate_by_share_work(
    shares: &[&ShareRecord],
    distributable_atomic: u64,
) -> std::result::Result<BTreeMap<String, u64>, String> {
    let mut work_by_address = BTreeMap::<String, u128>::new();
    for share in shares {
        let work = share_work(share.share_difficulty_leading_zero_bits);
        let entry = work_by_address
            .entry(share.miner_address.clone())
            .or_default();
        *entry = entry.saturating_add(work);
    }
    let total = work_by_address
        .values()
        .fold(0_u128, |sum, work| sum.saturating_add(*work));
    if total == 0 {
        return Err("PPLNS work total cannot be zero".to_owned());
    }
    if work_by_address.is_empty() {
        return Err("cannot allocate a block without accepted shares".to_owned());
    }

    // Floor amounts + fractional remainders for fair dust split.
    let mut floor_amounts = BTreeMap::<String, u64>::new();
    let mut remainders: Vec<(String, u128)> = Vec::new();
    let mut allocated = 0_u64;
    for (address, work) in &work_by_address {
        let product = distributable_atomic as u128 * *work;
        let amount = (product / total) as u64;
        let rem = product % total;
        floor_amounts.insert(address.clone(), amount);
        remainders.push((address.clone(), rem));
        allocated = allocated.saturating_add(amount);
    }
    let mut dust = distributable_atomic.saturating_sub(allocated);
    // Largest remainder first; tie-break by address for determinism.
    remainders.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (address, _) in remainders {
        if dust == 0 {
            break;
        }
        if let Some(amount) = floor_amounts.get_mut(&address) {
            *amount = amount.saturating_add(1);
            dust = dust.saturating_sub(1);
        }
    }
    // Any leftover (should be 0) goes to highest-work miner.
    if dust > 0 {
        if let Some((address, _)) = work_by_address
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then_with(|| a.0.cmp(b.0)))
        {
            if let Some(amount) = floor_amounts.get_mut(address) {
                *amount = amount.saturating_add(dust);
            }
        }
    }
    Ok(floor_amounts)
}

fn random_id() -> String {
    let mut bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn storage(error: std::io::Error) -> PoolError {
    PoolError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn share(address: &str, hash: &str, timestamp: u64) -> ShareRecord {
        ShareRecord {
            share_id: 0,
            job_id: "job-1".to_owned(),
            miner_address: address.to_owned(),
            worker_name: "worker".to_owned(),
            nonce: timestamp,
            hash: hash.to_owned(),
            share_difficulty_leading_zero_bits: 8,
            network_difficulty_leading_zero_bits: 16,
            accepted_at_unix_seconds: timestamp,
            block_candidate: false,
        }
    }

    #[test]
    fn duplicate_share_is_idempotent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = PoolStore::load(dir.path().to_path_buf(), 100).expect("store");
        let (first, dup1) = store
            .record_share(share("miner-a", "aa", 1))
            .expect("share");
        assert!(!dup1);
        let (second, dup2) = store
            .record_share(share("miner-a", "aa", 2))
            .expect("duplicate ok");
        assert!(dup2);
        assert_eq!(first.share_id, second.share_id);
        assert_eq!(store.snapshot().expect("snap").shares.len(), 1);
    }

    #[test]
    fn pplns_allocates_exact_reward_and_matures_canonical_block() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = PoolStore::load(dir.path().to_path_buf(), 100).expect("store");
        store
            .record_share(share("miner-a", "01", 1))
            .expect("share");
        let _ = store
            .record_share(share("miner-a", "02", 2))
            .expect("share");
        let _ = store
            .record_share(share("miner-b", "03", 3))
            .expect("share");
        store
            .record_block(10, "block".to_owned(), 1_000, 100, 3, 4)
            .expect("block");
        let before = store.snapshot().expect("snapshot");
        assert_eq!(before.accounts["miner-a"].immature_atomic, 660);
        assert_eq!(before.accounts["miner-b"].immature_atomic, 330);
        assert_eq!(before.blocks[0].pool_fee_atomic, 10);
        store
            .mature_blocks(&BTreeMap::from([(10, "block".to_owned())]), 22, 12)
            .expect("mature");
        let after = store.snapshot().expect("snapshot");
        assert_eq!(
            after
                .accounts
                .values()
                .map(|item| item.mature_atomic)
                .sum::<u64>(),
            990
        );
    }

    #[test]
    fn payout_moves_through_prepared_and_submitted_states() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = PoolStore::load(dir.path().to_path_buf(), 100).expect("store");
        store
            .record_share(share("miner-a", "01", 1))
            .expect("share");
        store
            .record_block(1, "block".to_owned(), 1_000, 0, 1, 2)
            .expect("block");
        store
            .mature_blocks(&BTreeMap::from([(1, "block".to_owned())]), 2, 1)
            .expect("mature");
        let payout = store.prepare_payout(500, 3).expect("prepare");
        assert_eq!(
            store.snapshot().expect("snapshot").accounts["miner-a"].pending_payout_atomic,
            1_000
        );
        store
            .confirm_payout(&payout.payout_id, vec!["00".repeat(32)])
            .expect("confirm");
        let submitted = store.snapshot().expect("snapshot");
        assert_eq!(submitted.accounts["miner-a"].pending_payout_atomic, 0);
        assert_eq!(submitted.accounts["miner-a"].paid_atomic, 1_000);
    }

    #[test]
    fn pplns_weights_shares_by_proven_difficulty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = PoolStore::load(dir.path().to_path_buf(), 100).expect("store");
        let mut low = share("miner-a", "01", 1);
        low.share_difficulty_leading_zero_bits = 8;
        let mut high = share("miner-b", "02", 2);
        high.share_difficulty_leading_zero_bits = 9;
        store.record_share(low).expect("low share");
        store.record_share(high).expect("high share");
        store
            .record_block(1, "block".to_owned(), 3_000, 0, 2, 3)
            .expect("block");
        let data = store.snapshot().expect("snapshot");
        assert_eq!(data.blocks[0].allocations["miner-a"], 1_000);
        assert_eq!(data.blocks[0].allocations["miner-b"], 2_000);
    }

    #[test]
    fn round_pplns_splits_equal_work_between_two_miners() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = PoolStore::load(dir.path().to_path_buf(), 100).expect("store");
        // Equal proven work (same share difficulty, same share count).
        store
            .record_share(share("miner-a", "a1", 10))
            .expect("share");
        store
            .record_share(share("miner-b", "b1", 11))
            .expect("share");
        store
            .record_share(share("miner-a", "a2", 12))
            .expect("share");
        store
            .record_share(share("miner-b", "b2", 13))
            .expect("share");
        store
            .record_block(5, "block-eq".to_owned(), 1_000, 0, 100, 14)
            .expect("block");
        let data = store.snapshot().expect("snap");
        let a = data.blocks[0].allocations["miner-a"];
        let b = data.blocks[0].allocations["miner-b"];
        assert_eq!(a + b, 1_000);
        assert!((a as i64 - b as i64).abs() <= 1, "a={a} b={b}");
    }

    #[test]
    fn next_block_only_uses_shares_after_previous_block() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = PoolStore::load(dir.path().to_path_buf(), 100).expect("store");
        // Round 1: only miner-a
        store
            .record_share(share("miner-a", "r1a", 1))
            .expect("share");
        store
            .record_block(1, "b1".to_owned(), 1_000, 0, 100, 2)
            .expect("block1");
        // Round 2: only miner-b (miner-a history must not steal this block)
        store
            .record_share(share("miner-b", "r2b", 3))
            .expect("share");
        store
            .record_block(2, "b2".to_owned(), 1_000, 0, 100, 4)
            .expect("block2");
        let data = store.snapshot().expect("snap");
        assert_eq!(data.blocks[0].allocations.len(), 1);
        assert_eq!(data.blocks[0].allocations["miner-a"], 1_000);
        assert_eq!(data.blocks[1].allocations.len(), 1);
        assert_eq!(data.blocks[1].allocations["miner-b"], 1_000);
        assert_eq!(data.accounts["miner-a"].immature_atomic, 1_000);
        assert_eq!(data.accounts["miner-b"].immature_atomic, 1_000);
    }

    #[test]
    fn dust_remainder_is_not_all_given_to_lexicographically_first_miner() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = PoolStore::load(dir.path().to_path_buf(), 100).expect("store");
        // 3 equal-work miners, reward 100 → floor 33 each, dust 1 via largest remainder
        store
            .record_share(share("miner-c", "c1", 1))
            .expect("share");
        store
            .record_share(share("miner-a", "a1", 2))
            .expect("share");
        store
            .record_share(share("miner-b", "b1", 3))
            .expect("share");
        store
            .record_block(1, "dust".to_owned(), 100, 0, 10, 4)
            .expect("block");
        let data = store.snapshot().expect("snap");
        let alloc = &data.blocks[0].allocations;
        assert_eq!(alloc.values().sum::<u64>(), 100);
        // Each gets 33 or 34 — never 34+34+32 style dump onto one key only from floor+all-dust-to-first
        for amount in alloc.values() {
            assert!(*amount == 33 || *amount == 34, "amount={amount}");
        }
    }

    #[test]
    fn reorg_orphans_immature_with_wrong_canonical_hash() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = PoolStore::load(dir.path().to_path_buf(), 100).expect("store");
        store
            .record_share(share("miner-a", "01", 1))
            .expect("share");
        store
            .record_block(5, "pool-block".to_owned(), 1_000, 0, 1, 2)
            .expect("block");
        // Canonical chain reorged: different hash at height 5 before maturity.
        store
            .mature_blocks(&BTreeMap::from([(5, "other-hash".to_owned())]), 5, 12)
            .expect("orphan");
        let data = store.snapshot().expect("snapshot");
        assert_eq!(data.blocks[0].status, PoolBlockStatus::Orphaned);
        assert_eq!(data.accounts["miner-a"].immature_atomic, 0);
    }

    #[test]
    fn tip_rewind_voids_immature_above_new_tip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = PoolStore::load(dir.path().to_path_buf(), 100).expect("store");
        store
            .record_share(share("miner-a", "01", 1))
            .expect("share");
        store
            .record_block(10, "block".to_owned(), 1_000, 0, 1, 2)
            .expect("block");
        let voided = store.void_immature_above_tip(5).expect("void");
        assert_eq!(voided, 1);
        let data = store.snapshot().expect("snapshot");
        assert_eq!(data.blocks[0].status, PoolBlockStatus::Orphaned);
        assert_eq!(data.accounts["miner-a"].immature_atomic, 0);
    }

    #[test]
    fn mature_block_orphaned_on_post_maturity_reorg_claws_unpaid_mature() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = PoolStore::load(dir.path().to_path_buf(), 100).expect("store");
        store
            .record_share(share("miner-a", "01", 1))
            .expect("share");
        store
            .record_block(1, "block".to_owned(), 1_000, 0, 1, 2)
            .expect("block");
        store
            .mature_blocks(&BTreeMap::from([(1, "block".to_owned())]), 2, 1)
            .expect("mature");
        let data = store.snapshot().expect("snap");
        assert_eq!(data.blocks[0].status, PoolBlockStatus::Mature);
        assert_eq!(data.accounts["miner-a"].mature_atomic, 1_000);
        // Canonical hash flips after maturity (deep reorg / wrong tip history).
        store
            .mature_blocks(&BTreeMap::from([(1, "other".to_owned())]), 20, 1)
            .expect("re-check");
        let after = store.snapshot().expect("after");
        assert_eq!(after.blocks[0].status, PoolBlockStatus::Orphaned);
        assert_eq!(after.accounts["miner-a"].mature_atomic, 0);
    }

    #[test]
    fn prepared_payout_can_be_cancelled_without_losing_balance() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = PoolStore::load(dir.path().to_path_buf(), 100).expect("store");
        store
            .record_share(share("miner-a", "01", 1))
            .expect("share");
        store
            .record_block(1, "block".to_owned(), 1_000, 0, 1, 2)
            .expect("block");
        store
            .mature_blocks(&BTreeMap::from([(1, "block".to_owned())]), 2, 1)
            .expect("mature");
        let payout = store.prepare_payout(500, 3).expect("prepare");
        store.cancel_payout(&payout.payout_id).expect("cancel");
        let data = store.snapshot().expect("snapshot");
        assert_eq!(data.accounts["miner-a"].mature_atomic, 1_000);
        assert_eq!(data.accounts["miner-a"].pending_payout_atomic, 0);
        assert_eq!(data.payouts[0].status, PayoutStatus::Cancelled);
    }
}
