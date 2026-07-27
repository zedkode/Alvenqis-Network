use crate::amount::Amount;
use crate::block::Block;
use crate::consensus::{block_reward, expected_coinbase_amount, validate_coinbase_structure};
use crate::constants::MAX_SUPPLY_ATOMIC;
use crate::crypto::Hash;
use crate::errors::{AlvenqisError, Result};
use crate::hash_to_hex;
use crate::transaction::Transaction;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlockLedgerSummary {
    pub total_fees: Amount,
    pub burned_fees: Amount,
    pub priority_fees: Amount,
    pub coinbase_reward: Amount,
    pub base_fee: Amount,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TransactionFeeSummary {
    pub total_fee: Amount,
    pub burned_fee: Amount,
    pub priority_fee: Amount,
}

/// First non-coinbase spend from an account must use this nonce (matches wallet/RPC).
pub const FIRST_ACCOUNT_NONCE: u64 = 1;

/// How many recent confirmed blocks keep their tx hashes in the in-memory set.
///
/// Per-account sequential nonces ([`FIRST_ACCOUNT_NONCE`] / `nonces`) are the
/// primary spend-replay defense: once a nonce is consumed, the same signed
/// spend cannot apply again even if its hash is pruned.
///
/// This set remains as **defense-in-depth** for same-hash re-injection (coinbase
/// uniqueness, accidental double-apply of identical bodies) within a bounded
/// window so long-running nodes do not retain every historical hash forever.
/// Documented also in `Blockchain-docs/human/architecture/` (ledger state notes).
pub const TX_HASH_RETENTION_BLOCKS: u64 = 1024;
pub const BLOCK_METRICS_RETENTION_BLOCKS: u64 = 2048;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LedgerState {
    balances: BTreeMap<String, Amount>,
    /// Next expected sequential nonce per address (absent ⇒ [`FIRST_ACCOUNT_NONCE`]).
    nonces: BTreeMap<String, u64>,
    emitted_supply: Amount,
    block_fees: BTreeMap<u64, Amount>,
    block_burned_fees: BTreeMap<u64, Amount>,
    block_priority_fees: BTreeMap<u64, Amount>,
    coinbase_rewards: BTreeMap<u64, Amount>,
    total_burned_fees: Amount,
    /// Union of retained per-height tx hashes (see [`TX_HASH_RETENTION_BLOCKS`]).
    applied_transaction_hashes: BTreeSet<String>,
    /// Per-height hash lists so retention pruning can drop old entries exactly.
    tx_hashes_by_height: BTreeMap<u64, Vec<String>>,
    applied_block_height: Option<u64>,
    tip_hash: Option<Hash>,
    /// Timestamp of the tip block (for state-layer monotonicity checks).
    tip_timestamp: Option<u64>,
}

impl LedgerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn balances(&self) -> &BTreeMap<String, Amount> {
        &self.balances
    }

    pub fn balance_of(&self, address: &str) -> Amount {
        self.balances.get(address).copied().unwrap_or(Amount::ZERO)
    }

    /// Next sequential nonce the account must use for a non-coinbase spend.
    pub fn next_nonce_of(&self, address: &str) -> u64 {
        self.nonces
            .get(address)
            .copied()
            .unwrap_or(FIRST_ACCOUNT_NONCE)
    }

    pub fn nonces(&self) -> &BTreeMap<String, u64> {
        &self.nonces
    }

    pub fn emitted_supply(&self) -> Amount {
        self.emitted_supply
    }

    pub fn block_fees(&self) -> &BTreeMap<u64, Amount> {
        &self.block_fees
    }

    pub fn coinbase_rewards(&self) -> &BTreeMap<u64, Amount> {
        &self.coinbase_rewards
    }

    pub fn block_burned_fees(&self) -> &BTreeMap<u64, Amount> {
        &self.block_burned_fees
    }

    pub fn block_priority_fees(&self) -> &BTreeMap<u64, Amount> {
        &self.block_priority_fees
    }

    pub fn total_burned_fees(&self) -> Amount {
        self.total_burned_fees
    }

    pub fn applied_block_height(&self) -> Option<u64> {
        self.applied_block_height
    }

    pub fn applied_transaction_hashes(&self) -> &BTreeSet<String> {
        &self.applied_transaction_hashes
    }

    pub fn tip_hash(&self) -> Option<Hash> {
        self.tip_hash
    }

    pub fn tip_timestamp(&self) -> Option<u64> {
        self.tip_timestamp
    }

    pub fn transaction_simulation_state(&self) -> Self {
        Self {
            block_fees: BTreeMap::new(),
            block_burned_fees: BTreeMap::new(),
            block_priority_fees: BTreeMap::new(),
            coinbase_rewards: BTreeMap::new(),
            ..self.clone()
        }
    }

    fn ensure_transaction_hash_is_new(&self, transaction: &Transaction) -> Result<()> {
        let tx_hash = hash_to_hex(&transaction.tx_hash());
        if self.applied_transaction_hashes.contains(&tx_hash) {
            return Err(AlvenqisError::DuplicateTransactionHash(tx_hash));
        }
        Ok(())
    }

    fn record_applied_tx_hash(&mut self, height: u64, tx_hash: String) {
        self.applied_transaction_hashes.insert(tx_hash.clone());
        self.tx_hashes_by_height
            .entry(height)
            .or_default()
            .push(tx_hash);
    }

    /// Drop hashes older than the retention window (keeps the most recent
    /// [`TX_HASH_RETENTION_BLOCKS`] heights inclusive of tip).
    fn prune_tx_hash_retention(&mut self) {
        let Some(tip) = self.applied_block_height else {
            return;
        };
        // Keep tip and the previous (N-1) heights → N blocks total.
        let min_keep = tip.saturating_sub(TX_HASH_RETENTION_BLOCKS.saturating_sub(1));
        let stale_heights: Vec<u64> = self
            .tx_hashes_by_height
            .range(..min_keep)
            .map(|(height, _)| *height)
            .collect();
        for height in stale_heights {
            if let Some(hashes) = self.tx_hashes_by_height.remove(&height) {
                for hash in hashes {
                    self.applied_transaction_hashes.remove(&hash);
                }
            }
        }
    }

    fn prune_block_metrics_retention(&mut self) {
        let Some(tip) = self.applied_block_height else {
            return;
        };
        let min_keep = tip.saturating_sub(BLOCK_METRICS_RETENTION_BLOCKS.saturating_sub(1));
        self.block_fees.retain(|height, _| *height >= min_keep);
        self.block_burned_fees
            .retain(|height, _| *height >= min_keep);
        self.block_priority_fees
            .retain(|height, _| *height >= min_keep);
        self.coinbase_rewards
            .retain(|height, _| *height >= min_keep);
    }

    /// Approximate number of retained heights (for tests / diagnostics).
    pub fn retained_tx_hash_heights(&self) -> usize {
        self.tx_hashes_by_height.len()
    }
}

pub fn validate_transaction_against_state(
    state: &LedgerState,
    transaction: &Transaction,
    base_fee: Amount,
) -> Result<()> {
    if transaction.is_coinbase() {
        return Ok(());
    }

    state.ensure_transaction_hash_is_new(transaction)?;
    transaction.verify()?;
    transaction.validate_fee_against_base_fee(base_fee)?;
    if transaction.amount == Amount::ZERO {
        return Err(AlvenqisError::ZeroAmountTransaction);
    }

    let sender = transaction.from.as_deref().ok_or_else(|| {
        AlvenqisError::InvalidTransaction("non-coinbase transaction requires sender".to_owned())
    })?;
    let expected_nonce = state.next_nonce_of(sender);
    if transaction.nonce != expected_nonce {
        return Err(AlvenqisError::InvalidNonce {
            address: sender.to_owned(),
            expected: expected_nonce,
            actual: transaction.nonce,
        });
    }
    let required = transaction.total_debit(base_fee)?;
    let available = state.balance_of(sender);
    if available < required {
        return Err(AlvenqisError::InsufficientBalance {
            address: sender.to_owned(),
            available: available.as_atomic(),
            required: required.as_atomic(),
        });
    }

    Ok(())
}

pub fn block_fee_summary(block: &Block) -> Result<BlockLedgerSummary> {
    validate_coinbase_structure(&block.transactions)?;
    let base_fee = Amount::from_atomic(block.header.base_fee_atomic);
    let mut total_fees = Amount::ZERO;
    let mut burned_fees = Amount::ZERO;
    let mut priority_fees = Amount::ZERO;
    for transaction in block.transactions.iter().skip(1) {
        total_fees = total_fees.checked_add(transaction.effective_fee(base_fee)?)?;
        burned_fees = burned_fees.checked_add(base_fee)?;
        priority_fees = priority_fees.checked_add(transaction.effective_priority_fee(base_fee)?)?;
    }
    Ok(BlockLedgerSummary {
        total_fees,
        burned_fees,
        priority_fees,
        coinbase_reward: block_reward(block.header.height),
        base_fee,
    })
}

pub fn apply_transaction(
    state: &mut LedgerState,
    transaction: &Transaction,
    base_fee: Amount,
) -> Result<TransactionFeeSummary> {
    // Height for hash-retention bookkeeping: tip+1 while applying a candidate block,
    // or 0 when applying genesis / standalone coinbase before height is committed.
    let record_height = state
        .applied_block_height
        .map_or(0, |h| h.saturating_add(1));
    apply_transaction_at_height(state, transaction, base_fee, record_height)
}

fn apply_transaction_at_height(
    state: &mut LedgerState,
    transaction: &Transaction,
    base_fee: Amount,
    height: u64,
) -> Result<TransactionFeeSummary> {
    if transaction.is_coinbase() {
        state.ensure_transaction_hash_is_new(transaction)?;
        let tx_hash = hash_to_hex(&transaction.tx_hash());
        // Zero coinbase is valid post-subsidy when consensus expected amount is 0 (CR-H09).
        if transaction.amount != Amount::ZERO {
            state.credit_balance(&transaction.to, transaction.amount)?;
        }
        state.record_applied_tx_hash(height, tx_hash);
        return Ok(TransactionFeeSummary::default());
    }

    validate_transaction_against_state(state, transaction, base_fee)?;
    let tx_hash = hash_to_hex(&transaction.tx_hash());
    let sender = transaction.from.as_deref().ok_or_else(|| {
        AlvenqisError::InvalidTransaction(
            "non-coinbase transaction is missing sender after validation".to_owned(),
        )
    })?;
    let total_fee = transaction.effective_fee(base_fee)?;
    let burned_fee = base_fee;
    let priority_fee = transaction.effective_priority_fee(base_fee)?;
    let total_debit = transaction.total_debit(base_fee)?;
    state.debit_balance(sender, total_debit)?;
    state.credit_balance(&transaction.to, transaction.amount)?;
    state.total_burned_fees = state.total_burned_fees.checked_add(burned_fee)?;
    state
        .nonces
        .insert(sender.to_owned(), transaction.nonce.saturating_add(1));
    state.record_applied_tx_hash(height, tx_hash);
    Ok(TransactionFeeSummary {
        total_fee,
        burned_fee,
        priority_fee,
    })
}

pub fn validate_block_against_state(
    state: &LedgerState,
    block: &Block,
) -> Result<BlockLedgerSummary> {
    let expected_height = state.applied_block_height.map_or(0, |height| height + 1);
    if block.header.height != expected_height {
        return Err(AlvenqisError::InvalidHeight {
            expected: expected_height,
            actual: block.header.height,
        });
    }

    let expected_previous_hash = state.tip_hash.unwrap_or_else(Hash::zero);
    if block.header.previous_hash != expected_previous_hash {
        return Err(AlvenqisError::InvalidPreviousHash {
            expected: expected_previous_hash,
            actual: block.header.previous_hash,
        });
    }

    // Same timestamp rule as structural consensus (defense in depth for state-only apply).
    if let Some(previous_ts) = state.tip_timestamp {
        if block.header.timestamp <= previous_ts {
            return Err(AlvenqisError::InvalidTimestamp {
                previous: previous_ts,
                actual: block.header.timestamp,
            });
        }
    }

    // Structural coinbase + exact amount via the shared consensus helper.
    validate_coinbase_structure(&block.transactions)?;
    let base_fee = Amount::from_atomic(block.header.base_fee_atomic);
    let expected_amount =
        expected_coinbase_amount(block.header.height, base_fee, &block.transactions)?;
    let coinbase = block
        .transactions
        .first()
        .ok_or(AlvenqisError::MissingCoinbase)?;
    if coinbase.amount != expected_amount {
        return Err(AlvenqisError::InvalidCoinbaseAmount {
            expected: expected_amount.as_atomic(),
            actual: coinbase.amount.as_atomic(),
        });
    }
    // Allow exact zero coinbase when expected is zero (post-subsidy liveness, CR-H09).
    if coinbase.amount == Amount::ZERO && expected_amount != Amount::ZERO {
        return Err(AlvenqisError::ZeroAmountTransaction);
    }

    // Reject duplicate nonces from the same sender inside one block before apply
    // (clearer than failing mid-apply on the second spend).
    {
        let mut seen_sender_nonce = BTreeSet::new();
        for transaction in block.transactions.iter().skip(1) {
            if let Some(sender) = transaction.from.as_deref() {
                if !seen_sender_nonce.insert((sender.to_owned(), transaction.nonce)) {
                    return Err(AlvenqisError::InvalidNonce {
                        address: sender.to_owned(),
                        expected: state.next_nonce_of(sender),
                        actual: transaction.nonce,
                    });
                }
            }
        }
    }

    let mut simulated = state.transaction_simulation_state();
    let mut total_fees = Amount::ZERO;
    let mut burned_fees = Amount::ZERO;
    let mut priority_fees = Amount::ZERO;
    for transaction in block.transactions.iter().skip(1) {
        let fee_summary = apply_transaction(&mut simulated, transaction, base_fee)?;
        total_fees = total_fees.checked_add(fee_summary.total_fee)?;
        burned_fees = burned_fees.checked_add(fee_summary.burned_fee)?;
        priority_fees = priority_fees.checked_add(fee_summary.priority_fee)?;
    }

    // Recompute expected with simulated priority fees — must match helper (exact).
    let coinbase_reward = block_reward(block.header.height);
    let expected_from_sim = coinbase_reward.checked_add(priority_fees)?;
    if coinbase.amount != expected_from_sim {
        return Err(AlvenqisError::InvalidCoinbaseAmount {
            expected: expected_from_sim.as_atomic(),
            actual: coinbase.amount.as_atomic(),
        });
    }

    let updated_supply = state.emitted_supply.checked_add(coinbase_reward)?;
    if updated_supply.as_atomic() > MAX_SUPPLY_ATOMIC {
        return Err(AlvenqisError::SupplyOverflow);
    }

    Ok(BlockLedgerSummary {
        total_fees,
        burned_fees,
        priority_fees,
        coinbase_reward,
        base_fee,
    })
}

pub fn apply_block(state: &mut LedgerState, block: &Block) -> Result<BlockLedgerSummary> {
    let summary = validate_block_against_state(state, block)?;

    let base_fee = summary.base_fee;
    let height = block.header.height;
    for transaction in block.transactions.iter().skip(1) {
        apply_transaction_at_height(state, transaction, base_fee, height)?;
    }

    let coinbase = block
        .transactions
        .first()
        .ok_or(AlvenqisError::MissingCoinbase)?;
    apply_transaction_at_height(state, coinbase, base_fee, height)?;
    state.emitted_supply = state.emitted_supply.checked_add(summary.coinbase_reward)?;
    state
        .block_fees
        .insert(block.header.height, summary.total_fees);
    state
        .block_burned_fees
        .insert(block.header.height, summary.burned_fees);
    state
        .block_priority_fees
        .insert(block.header.height, summary.priority_fees);
    state
        .coinbase_rewards
        .insert(block.header.height, summary.coinbase_reward);
    state.applied_block_height = Some(block.header.height);
    state.tip_hash = Some(block.hash()?);
    state.tip_timestamp = Some(block.header.timestamp);
    state.prune_tx_hash_retention();
    state.prune_block_metrics_retention();
    Ok(summary)
}

impl LedgerState {
    fn credit_balance(&mut self, address: &str, amount: Amount) -> Result<()> {
        let next = self.balance_of(address).checked_add(amount)?;
        self.balances.insert(address.to_owned(), next);
        Ok(())
    }

    fn debit_balance(&mut self, address: &str, amount: Amount) -> Result<()> {
        let next = self.balance_of(address).checked_sub(amount)?;
        self.balances.insert(address.to_owned(), next);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::Address;
    use crate::chain::Chain;
    use crate::constants::{
        BLOCK_TIME_SECONDS, INITIAL_BASE_FEE_ATOMIC, INITIAL_BLOCK_REWARD_ATOMIC,
    };
    use crate::genesis::{
        devnet_child_block_with_difficulty, devnet_genesis_with_difficulty,
        DEVNET_GENESIS_TIMESTAMP,
    };
    use crate::network::Network;
    use crate::signing::PrivateKey;
    use crate::{AlvenqisError, Amount, Transaction};

    fn make_signed_transfer(
        sender: &PrivateKey,
        recipient: &str,
        amount: u64,
        fee: u64,
        nonce: u64,
    ) -> Transaction {
        Transaction::new_signed(
            1,
            nonce,
            Network::Devnet,
            sender,
            recipient.to_owned(),
            Amount::from_atomic(amount),
            Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + fee),
            Amount::from_atomic(fee),
            None,
        )
        .expect("signed transaction")
    }

    #[test]
    fn coinbase_increases_miner_balance() {
        let miner = PrivateKey::generate();
        let miner_address =
            Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
        let genesis = devnet_genesis_with_difficulty(&miner_address, 4).expect("genesis");

        let mut state = LedgerState::new();
        apply_block(&mut state, &genesis).expect("apply genesis");

        assert_eq!(
            state.balance_of(&miner_address).as_atomic(),
            INITIAL_BLOCK_REWARD_ATOMIC
        );
    }

    #[test]
    fn signed_transfer_moves_balance() {
        let miner = PrivateKey::generate();
        let recipient = PrivateKey::generate();
        let miner_address =
            Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
        let recipient_address =
            Address::from_public_key_for_network(&recipient.public_key(), Network::Devnet)
                .to_string();

        let genesis = devnet_genesis_with_difficulty(&miner_address, 4).expect("genesis");
        let transfer = make_signed_transfer(&miner, &recipient_address, 100, 1, 1);
        let child = devnet_child_block_with_difficulty(
            &genesis,
            &miner_address,
            DEVNET_GENESIS_TIMESTAMP + BLOCK_TIME_SECONDS,
            vec![transfer],
            4,
        )
        .expect("child");

        let mut chain = Chain::new(Network::Devnet);
        chain.append_block(genesis).expect("append genesis");
        chain.append_block(child).expect("append child");

        assert_eq!(chain.state().next_nonce_of(&miner_address), 2);
        assert_eq!(
            chain.state().balance_of(&recipient_address).as_atomic(),
            100
        );
        assert_eq!(
            chain.state().balance_of(&miner_address).as_atomic(),
            INITIAL_BLOCK_REWARD_ATOMIC * 2 - 100 - INITIAL_BASE_FEE_ATOMIC
        );
    }

    #[test]
    fn fee_is_collected_by_miner() {
        let miner = PrivateKey::generate();
        let recipient = PrivateKey::generate();
        let miner_address =
            Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
        let recipient_address =
            Address::from_public_key_for_network(&recipient.public_key(), Network::Devnet)
                .to_string();

        let genesis = devnet_genesis_with_difficulty(&miner_address, 4).expect("genesis");
        let transfer = make_signed_transfer(&miner, &recipient_address, 100, 7, 1);
        let child = devnet_child_block_with_difficulty(
            &genesis,
            &miner_address,
            DEVNET_GENESIS_TIMESTAMP + BLOCK_TIME_SECONDS,
            vec![transfer],
            4,
        )
        .expect("child");

        let mut chain = Chain::new(Network::Devnet);
        chain.append_block(genesis).expect("append genesis");
        chain.append_block(child).expect("append child");

        assert_eq!(
            chain.state().balance_of(&miner_address).as_atomic(),
            INITIAL_BLOCK_REWARD_ATOMIC * 2 - 100 - INITIAL_BASE_FEE_ATOMIC
        );
        assert_eq!(
            chain
                .state()
                .block_fees()
                .get(&1)
                .copied()
                .unwrap_or(Amount::ZERO)
                .as_atomic(),
            8
        );
        assert_eq!(
            chain
                .state()
                .block_priority_fees()
                .get(&1)
                .copied()
                .unwrap_or(Amount::ZERO)
                .as_atomic(),
            7
        );
        assert_eq!(
            chain
                .state()
                .block_burned_fees()
                .get(&1)
                .copied()
                .unwrap_or(Amount::ZERO)
                .as_atomic(),
            INITIAL_BASE_FEE_ATOMIC
        );
    }

    #[test]
    fn insufficient_balance_is_rejected() {
        let sender = PrivateKey::generate();
        let recipient = PrivateKey::generate();
        let recipient_address =
            Address::from_public_key_for_network(&recipient.public_key(), Network::Devnet)
                .to_string();
        let transaction = make_signed_transfer(&sender, &recipient_address, 100, 1, 1);

        let state = LedgerState::new();
        let error = validate_transaction_against_state(
            &state,
            &transaction,
            Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC),
        )
        .expect_err("insufficient balance must fail");
        assert!(matches!(error, AlvenqisError::InsufficientBalance { .. }));
    }

    #[test]
    fn wrong_account_nonce_is_rejected() {
        let miner = PrivateKey::generate();
        let recipient = PrivateKey::generate();
        let miner_address =
            Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
        let recipient_address =
            Address::from_public_key_for_network(&recipient.public_key(), Network::Devnet)
                .to_string();
        let genesis = devnet_genesis_with_difficulty(&miner_address, 4).expect("genesis");
        let mut state = LedgerState::new();
        apply_block(&mut state, &genesis).expect("genesis");
        // First spend must be nonce 1; nonce 2 is invalid.
        let bad = make_signed_transfer(&miner, &recipient_address, 10, 1, 2);
        let error = validate_transaction_against_state(
            &state,
            &bad,
            Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC),
        )
        .expect_err("nonce gap must fail");
        assert!(matches!(
            error,
            AlvenqisError::InvalidNonce {
                expected: 1,
                actual: 2,
                ..
            }
        ));
    }

    #[test]
    fn total_supply_never_exceeds_max_supply() {
        let miner = PrivateKey::generate();
        let miner_address =
            Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
        let genesis = devnet_genesis_with_difficulty(&miner_address, 4).expect("genesis");

        let mut state = LedgerState::new();
        apply_block(&mut state, &genesis).expect("apply genesis");
        assert!(state.emitted_supply().as_atomic() <= MAX_SUPPLY_ATOMIC);
    }

    #[test]
    fn applying_same_block_twice_is_rejected() {
        let miner = PrivateKey::generate();
        let miner_address =
            Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
        let genesis = devnet_genesis_with_difficulty(&miner_address, 4).expect("genesis");

        let mut state = LedgerState::new();
        apply_block(&mut state, &genesis).expect("first apply");
        let error = apply_block(&mut state, &genesis).expect_err("duplicate block must fail");
        assert!(matches!(error, AlvenqisError::InvalidHeight { .. }));
    }

    #[test]
    fn supply_overflow_is_rejected_when_rebuilding_state() {
        let miner = PrivateKey::generate();
        let miner_address =
            Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
        let genesis = devnet_genesis_with_difficulty(&miner_address, 4).expect("genesis");

        let mut state = LedgerState::new();
        state.emitted_supply = Amount::from_atomic(MAX_SUPPLY_ATOMIC);
        let error =
            validate_block_against_state(&state, &genesis).expect_err("supply overflow must fail");
        assert!(matches!(error, AlvenqisError::SupplyOverflow));
    }

    /// Applied tx-hash set is bounded by [`TX_HASH_RETENTION_BLOCKS`] (defense-in-depth
    /// window; nonces remain primary replay protection).
    #[test]
    fn applied_tx_hash_set_does_not_grow_past_retention_window() {
        let miner = PrivateKey::generate();
        let miner_address =
            Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
        // Difficulty 0 keeps this test fast (no real mining work).
        let mut parent = devnet_genesis_with_difficulty(&miner_address, 0).expect("genesis");
        let mut state = LedgerState::new();
        apply_block(&mut state, &parent).expect("apply genesis");

        let extra = 32_u64;
        let total_blocks = BLOCK_METRICS_RETENTION_BLOCKS.max(TX_HASH_RETENTION_BLOCKS) + extra;
        for height in 1..=total_blocks {
            let child = devnet_child_block_with_difficulty(
                &parent,
                &miner_address,
                DEVNET_GENESIS_TIMESTAMP + BLOCK_TIME_SECONDS * height,
                vec![],
                0,
            )
            .expect("child");
            apply_block(&mut state, &child).expect("apply child");
            parent = child;
            assert!(
                state.retained_tx_hash_heights() as u64 <= TX_HASH_RETENTION_BLOCKS,
                "retained heights {} exceeded window {} at height {}",
                state.retained_tx_hash_heights(),
                TX_HASH_RETENTION_BLOCKS,
                height
            );
            // One coinbase hash per retained height in this pure-coinbase chain.
            assert!(
                state.applied_transaction_hashes().len() as u64 <= TX_HASH_RETENTION_BLOCKS,
                "hash set grew past retention bound"
            );
            assert!(state.block_fees().len() as u64 <= BLOCK_METRICS_RETENTION_BLOCKS);
            assert!(state.block_burned_fees().len() as u64 <= BLOCK_METRICS_RETENTION_BLOCKS);
            assert!(state.block_priority_fees().len() as u64 <= BLOCK_METRICS_RETENTION_BLOCKS);
            assert!(state.coinbase_rewards().len() as u64 <= BLOCK_METRICS_RETENTION_BLOCKS);
        }
        assert_eq!(
            state.retained_tx_hash_heights() as u64,
            TX_HASH_RETENTION_BLOCKS
        );
        assert_eq!(
            state.block_fees().len() as u64,
            BLOCK_METRICS_RETENTION_BLOCKS
        );
    }
}
