use crate::amount::Amount;
use crate::block::Block;
use crate::checkpoint::validate_checkpoint;
use crate::constants::{
    BASE_FEE_MAX_CHANGE_DENOMINATOR, BLOCK_TIME_SECONDS, DAA_SOLVETIME_CLAMP_MULTIPLIER,
    DAA_WINDOW_BLOCKS, HALVING_INTERVAL_BLOCKS, INITIAL_BASE_FEE_ATOMIC,
    INITIAL_BLOCK_REWARD_ATOMIC, MAX_FUTURE_BLOCK_DRIFT_SECONDS, MAX_SUPPLY_ATOMIC,
    MAX_TRANSACTIONS_PER_BLOCK, MAX_TRANSACTION_WIRE_BYTES, MEDIAN_TIME_PAST_WINDOW,
    MIN_BASE_FEE_ATOMIC, TARGET_TRANSACTIONS_PER_BLOCK,
};
use crate::errors::{AlvenqisError, Result};
use crate::hash_to_hex;
use crate::network::Network;
use crate::pow::FiroPow;
use crate::transaction::Transaction;
use crate::upgrade::expected_block_version;
use std::collections::BTreeSet;

pub fn block_reward(height: u64) -> Amount {
    let halvings = height / HALVING_INTERVAL_BLOCKS;
    if halvings >= 64 {
        return Amount::ZERO;
    }

    Amount::from_atomic(INITIAL_BLOCK_REWARD_ATOMIC >> (halvings as u32))
}

pub fn total_theoretical_emission() -> Result<Amount> {
    let mut total = Amount::ZERO;
    let mut reward = INITIAL_BLOCK_REWARD_ATOMIC;

    while reward > 0 {
        let epoch_total = reward
            .checked_mul(HALVING_INTERVAL_BLOCKS)
            .ok_or(AlvenqisError::AmountOverflow)?;
        total = total.checked_add(Amount::from_atomic(epoch_total))?;
        reward /= 2;
    }

    Ok(total)
}

/// Target comparison for FiroPoW final hashes (leading-zero-bits boundary).
#[inline]
pub fn check_pow(hash: &crate::crypto::Hash, required_leading_zero_bits: u8) -> bool {
    FiroPow.meets_target(hash, required_leading_zero_bits)
}

pub fn initial_base_fee() -> Amount {
    Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC)
}

pub fn next_base_fee(previous: Option<&Block>) -> Amount {
    previous.map_or_else(initial_base_fee, next_base_fee_from_block)
}

pub fn next_base_fee_from_block(previous: &Block) -> Amount {
    let previous_base_fee = previous.header.base_fee_atomic.max(MIN_BASE_FEE_ATOMIC);
    let target = TARGET_TRANSACTIONS_PER_BLOCK.max(1);
    let tx_count = previous.transactions.len().saturating_sub(1) as u64;
    if tx_count == target {
        return Amount::from_atomic(previous_base_fee);
    }

    let delta_numerator = previous_base_fee.saturating_mul(tx_count.abs_diff(target));
    let mut delta = delta_numerator / target / BASE_FEE_MAX_CHANGE_DENOMINATOR.max(1);
    if delta == 0 {
        delta = 1;
    }

    let next = if tx_count > target {
        previous_base_fee.saturating_add(delta)
    } else {
        previous_base_fee
            .saturating_sub(delta)
            .max(MIN_BASE_FEE_ATOMIC)
    };
    Amount::from_atomic(next)
}

pub fn next_difficulty_for_network(
    network: Network,
    previous_blocks: &[Block],
    fallback_difficulty: u8,
) -> u8 {
    lwma_next_difficulty(
        previous_blocks,
        BLOCK_TIME_SECONDS,
        network
            .difficulty_adjustment_window()
            .max(DAA_WINDOW_BLOCKS),
        fallback_difficulty,
        network.minimum_difficulty_leading_zero_bits(),
        network.maximum_difficulty_leading_zero_bits(),
    )
}

pub fn lwma_next_difficulty(
    previous_blocks: &[Block],
    target_block_time_seconds: u64,
    window: usize,
    fallback_difficulty: u8,
    minimum_difficulty: u8,
    maximum_difficulty: u8,
) -> u8 {
    let fallback = fallback_difficulty.clamp(minimum_difficulty, maximum_difficulty);
    if previous_blocks.len() < 2 || window == 0 {
        return previous_blocks
            .last()
            .map(|block| {
                block
                    .header
                    .difficulty_leading_zero_bits
                    .clamp(minimum_difficulty, maximum_difficulty)
            })
            .unwrap_or(fallback);
    }

    let sample_size = previous_blocks.len().min(window + 1);
    let sample = &previous_blocks[previous_blocks.len() - sample_size..];
    let target = target_block_time_seconds.max(1);
    let mut weighted_solvetime = 0_u128;
    let mut weighted_target = 0_u128;

    for (index, pair) in sample.windows(2).enumerate() {
        let previous = &pair[0];
        let current = &pair[1];
        let raw_solvetime = current
            .header
            .timestamp
            .saturating_sub(previous.header.timestamp);
        let clamped_solvetime =
            raw_solvetime.clamp(1, target.saturating_mul(DAA_SOLVETIME_CLAMP_MULTIPLIER));
        let weight = (index + 1) as u128;
        weighted_solvetime += u128::from(clamped_solvetime) * weight;
        weighted_target += u128::from(target) * weight;
    }

    let last_difficulty = sample
        .last()
        .map(|block| block.header.difficulty_leading_zero_bits)
        .unwrap_or(fallback)
        .clamp(minimum_difficulty, maximum_difficulty);
    if weighted_solvetime == 0 || weighted_target == 0 {
        return last_difficulty;
    }

    let increase_threshold = weighted_target.saturating_mul(90) / 100;
    let decrease_threshold = weighted_target.saturating_mul(110) / 100;

    if weighted_solvetime <= increase_threshold {
        let ratio = (weighted_target / weighted_solvetime).max(1);
        let step = ratio.ilog2().clamp(1, 4) as u8;
        return last_difficulty
            .saturating_add(step)
            .clamp(minimum_difficulty, maximum_difficulty);
    }

    if weighted_solvetime >= decrease_threshold {
        let ratio = (weighted_solvetime / weighted_target).max(1);
        let step = ratio.ilog2().clamp(1, 4) as u8;
        return last_difficulty
            .saturating_sub(step)
            .clamp(minimum_difficulty, maximum_difficulty);
    }

    last_difficulty
}

/// Subsidy (halving schedule) only — fees are layered separately via
/// [`expected_coinbase_amount`].
pub fn block_subsidy(height: u64) -> Amount {
    block_reward(height)
}

/// Canonical coinbase amount: **exact** subsidy + collected priority fees.
/// Used by both structural consensus validation and ledger-state validation so
/// the two paths cannot diverge (range vs exact mismatch class).
pub fn expected_coinbase_amount(
    height: u64,
    block_base_fee: Amount,
    transactions: &[Transaction],
) -> Result<Amount> {
    let subsidy = block_reward(height);
    let collected_priority_fees =
        transactions
            .iter()
            .skip(1)
            .try_fold(Amount::ZERO, |total, transaction| {
                // Non-coinbase fee schedule must be valid against this block's base fee.
                transaction.validate_fee_against_base_fee(block_base_fee)?;
                total.checked_add(transaction.effective_priority_fee(block_base_fee)?)
            })?;
    subsidy.checked_add(collected_priority_fees)
}

/// Structural coinbase rules shared by consensus (and callable from state helpers).
pub fn validate_coinbase_structure(transactions: &[Transaction]) -> Result<&Transaction> {
    let coinbase = transactions.first().ok_or(AlvenqisError::MissingCoinbase)?;
    if !coinbase.is_coinbase() {
        return Err(AlvenqisError::CoinbaseNotFirst);
    }
    if coinbase.max_fee != Amount::ZERO || coinbase.priority_fee != Amount::ZERO {
        return Err(AlvenqisError::InvalidCoinbaseFee);
    }
    if transactions
        .iter()
        .skip(1)
        .any(|transaction| transaction.is_coinbase())
    {
        return Err(AlvenqisError::DuplicateCoinbase);
    }
    Ok(coinbase)
}

/// Exact coinbase amount check (underpayment and overpayment both fail).
pub fn validate_coinbase_amount(
    height: u64,
    block_base_fee: Amount,
    transactions: &[Transaction],
) -> Result<Amount> {
    let coinbase = validate_coinbase_structure(transactions)?;
    let expected = expected_coinbase_amount(height, block_base_fee, transactions)?;
    if coinbase.amount != expected {
        return Err(AlvenqisError::InvalidCoinbaseAmount {
            expected: expected.as_atomic(),
            actual: coinbase.amount.as_atomic(),
        });
    }
    // Audit CR-H09: post-subsidy empty blocks must be allowed when expected coinbase is 0
    // (subsidy exhausted and no priority tips). Non-coinbase zero-value transfers still fail shape checks.
    if coinbase.amount == Amount::ZERO && expected != Amount::ZERO {
        return Err(AlvenqisError::ZeroAmountTransaction);
    }
    Ok(expected)
}

/// Timestamps must be strictly increasing along the canonical chain.
/// Genesis (no previous) accepts any timestamp for monotonicity, including zero.
/// Additionally, no block may be more than [`MAX_FUTURE_BLOCK_DRIFT_SECONDS`] ahead of
/// the validating node's local clock (prevents unbounded future headers).
pub fn validate_block_timestamp(previous: Option<&Block>, candidate: &Block) -> Result<()> {
    if let Some(prev) = previous {
        if candidate.header.timestamp <= prev.header.timestamp {
            return Err(AlvenqisError::InvalidTimestamp {
                previous: prev.header.timestamp,
                actual: candidate.header.timestamp,
            });
        }
    }
    validate_block_not_too_far_in_future(candidate.header.timestamp, unix_now_seconds())
}

/// Median-Time-Past of up to the last [`MEDIAN_TIME_PAST_WINDOW`] ancestor blocks.
///
/// Returns `None` when there are no ancestors (genesis). Otherwise the median of the
/// trailing timestamps (sorted). With an odd window the median is the middle sample.
pub fn median_time_past(previous_blocks: &[Block]) -> Option<u64> {
    if previous_blocks.is_empty() {
        return None;
    }
    let start = previous_blocks
        .len()
        .saturating_sub(MEDIAN_TIME_PAST_WINDOW);
    let mut times: Vec<u64> = previous_blocks[start..]
        .iter()
        .map(|b| b.header.timestamp)
        .collect();
    times.sort_unstable();
    // For even counts take the upper middle (conservative floor).
    let mid = times.len() / 2;
    Some(times[mid])
}

/// Candidate timestamp must be strictly greater than MTP of ancestors.
pub fn validate_median_time_past(previous_blocks: &[Block], candidate: &Block) -> Result<()> {
    if let Some(median) = median_time_past(previous_blocks) {
        if candidate.header.timestamp <= median {
            return Err(AlvenqisError::InvalidMedianTimePast {
                median,
                actual: candidate.header.timestamp,
            });
        }
    }
    Ok(())
}

/// Reject timestamps more than `MAX_FUTURE_BLOCK_DRIFT_SECONDS` ahead of `now`.
pub fn validate_block_not_too_far_in_future(timestamp: u64, now: u64) -> Result<()> {
    let max_allowed = now.saturating_add(MAX_FUTURE_BLOCK_DRIFT_SECONDS);
    if timestamp > max_allowed {
        return Err(AlvenqisError::InvalidFutureTimestamp {
            now,
            actual: timestamp,
            max_drift_seconds: MAX_FUTURE_BLOCK_DRIFT_SECONDS,
        });
    }
    Ok(())
}

fn unix_now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn validate_next_block(
    expected_network: Network,
    previous_blocks: &[Block],
    previous: Option<&Block>,
    candidate: &Block,
    emitted_supply: Amount,
) -> Result<()> {
    if candidate.transactions.is_empty() {
        return Err(AlvenqisError::EmptyTransactions);
    }
    if candidate.transactions.len() > MAX_TRANSACTIONS_PER_BLOCK {
        return Err(AlvenqisError::TooManyTransactions {
            max: MAX_TRANSACTIONS_PER_BLOCK,
            actual: candidate.transactions.len(),
        });
    }
    // Cheap structural body checks first (size + duplicate hashes). Signature
    // verification is intentionally deferred until AFTER PoW so invalid-work
    // candidates cannot force per-tx crypto work (DoS resistance).
    let mut seen_transaction_hashes = BTreeSet::new();
    for transaction in &candidate.transactions {
        let wire_len = transaction.encode().len();
        if wire_len > MAX_TRANSACTION_WIRE_BYTES {
            return Err(AlvenqisError::TransactionTooLarge {
                max: MAX_TRANSACTION_WIRE_BYTES,
                actual: wire_len,
            });
        }
        let tx_hash = hash_to_hex(&transaction.tx_hash());
        if !seen_transaction_hashes.insert(tx_hash.clone()) {
            return Err(AlvenqisError::DuplicateTransactionHash(tx_hash));
        }
    }
    if candidate.header.network_id != expected_network.network_id() {
        return Err(AlvenqisError::InvalidNetwork {
            expected: expected_network.network_id().to_owned(),
            actual: candidate.header.network_id.clone(),
        });
    }

    let expected_height = previous.map_or(0, |block| block.header.height + 1);
    if candidate.header.height != expected_height {
        return Err(AlvenqisError::InvalidHeight {
            expected: expected_height,
            actual: candidate.header.height,
        });
    }

    // Consensus rule: strict timestamp monotonicity (prevents equal/backdated headers).
    validate_block_timestamp(previous, candidate)?;
    // Median-Time-Past floor (Bitcoin-style) over the recent ancestor window.
    validate_median_time_past(previous_blocks, candidate)?;

    let expected_version = expected_block_version(expected_network, expected_height);
    if candidate.header.version != expected_version {
        return Err(AlvenqisError::InvalidBlockVersion {
            expected: expected_version,
            actual: candidate.header.version,
            height: candidate.header.height,
        });
    }

    let expected_base_fee = next_base_fee(previous);
    if candidate.header.base_fee_atomic != expected_base_fee.as_atomic() {
        return Err(AlvenqisError::InvalidBaseFee {
            expected: expected_base_fee.as_atomic(),
            actual: candidate.header.base_fee_atomic,
        });
    }

    let minimum_difficulty = expected_network.minimum_difficulty_leading_zero_bits();
    let maximum_difficulty = expected_network.maximum_difficulty_leading_zero_bits();
    if candidate.header.difficulty_leading_zero_bits < minimum_difficulty
        || candidate.header.difficulty_leading_zero_bits > maximum_difficulty
    {
        return Err(AlvenqisError::InvalidDifficultyAdjustment {
            expected: next_difficulty_for_network(
                expected_network,
                previous_blocks,
                candidate.header.difficulty_leading_zero_bits,
            ),
            actual: candidate.header.difficulty_leading_zero_bits,
        });
    }

    if previous.is_some() {
        let expected_difficulty = next_difficulty_for_network(
            expected_network,
            previous_blocks,
            candidate.header.difficulty_leading_zero_bits,
        );
        if candidate.header.difficulty_leading_zero_bits != expected_difficulty {
            return Err(AlvenqisError::InvalidDifficultyAdjustment {
                expected: expected_difficulty,
                actual: candidate.header.difficulty_leading_zero_bits,
            });
        }
    }

    let expected_previous_hash = match previous {
        Some(block) => block.hash()?,
        None => crate::crypto::Hash::zero(),
    };
    if candidate.header.previous_hash != expected_previous_hash {
        return Err(AlvenqisError::InvalidPreviousHash {
            expected: expected_previous_hash,
            actual: candidate.header.previous_hash,
        });
    }

    validate_checkpoint(expected_network, candidate.header.height, candidate.hash()?)?;

    let recomputed_merkle = candidate.recompute_merkle_root()?;
    if recomputed_merkle != candidate.header.merkle_root {
        return Err(AlvenqisError::InvalidMerkleRoot);
    }

    // Canonical PoW (FiroPoW 0.9.4) BEFORE per-transaction signature verification.
    // Cheap target check rejects junk candidates without paying for ed25519 verify.
    FiroPow.ensure_valid(candidate)?;

    let block_base_fee = Amount::from_atomic(candidate.header.base_fee_atomic);
    // Exact coinbase: same helper as ledger-state validation (audit A-M01).
    validate_coinbase_amount(
        candidate.header.height,
        block_base_fee,
        &candidate.transactions,
    )?;

    // Signatures only after valid work (and structural coinbase) — every body once.
    for transaction in &candidate.transactions {
        transaction.verify()?;
        let transaction_network = transaction.network()?;
        if transaction_network != expected_network {
            return Err(AlvenqisError::InvalidNetwork {
                expected: expected_network.network_id().to_owned(),
                actual: transaction_network.network_id().to_owned(),
            });
        }
    }

    let allowed_reward = block_reward(candidate.header.height);
    let updated_supply = emitted_supply.checked_add(allowed_reward)?;
    if updated_supply.as_atomic() > MAX_SUPPLY_ATOMIC {
        return Err(AlvenqisError::SupplyOverflow);
    }

    Ok(())
}

#[cfg(test)]
mod pow_before_signature_tests {
    use super::*;
    use crate::address::Address;
    use crate::constants::{BLOCK_TIME_SECONDS, INITIAL_BASE_FEE_ATOMIC};
    use crate::crypto::Hash;
    use crate::errors::AlvenqisError;
    use crate::genesis::{
        devnet_child_block_with_difficulty, devnet_genesis_with_difficulty,
        DEVNET_GENESIS_TIMESTAMP,
    };
    use crate::network::Network;
    use crate::signing::PrivateKey;
    use crate::transaction::Transaction;
    use crate::Amount;

    /// Invalid PoW must fail before signature verification would reject an unsigned spend.
    /// Evidence: error is `InvalidPow` (or other PoW-family), not a signature/verify error.
    #[test]
    fn invalid_pow_rejects_before_signature_verification() {
        let miner = PrivateKey::generate();
        let spender = PrivateKey::generate();
        let miner_address =
            Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
        let recipient =
            Address::from_public_key_for_network(&spender.public_key(), Network::Devnet)
                .to_string();

        // Use network-minimum difficulty so structural DAA checks pass before PoW.
        let bits = Network::Devnet.minimum_difficulty_leading_zero_bits();
        let genesis = devnet_genesis_with_difficulty(&miner_address, bits).expect("genesis");
        // Unsigned spend body: verify() would fail if signature checks ran first.
        let unsigned = Transaction::new(
            1,
            1,
            Some(miner_address.clone()),
            recipient,
            Amount::from_atomic(1),
            Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 1),
            Amount::from_atomic(1),
            None,
        )
        .expect("shape");

        let mut child = devnet_child_block_with_difficulty(
            &genesis,
            &miner_address,
            DEVNET_GENESIS_TIMESTAMP + BLOCK_TIME_SECONDS,
            vec![unsigned],
            bits,
        )
        .expect("child template");
        // Corrupt PoW fields after a valid-looking template is produced.
        child.header.mix_hash = Hash::from_bytes([0xAB; 32]);
        child.header.nonce = child.header.nonce.wrapping_add(0xDEAD_BEEF);

        let error = validate_next_block(
            Network::Devnet,
            std::slice::from_ref(&genesis),
            Some(&genesis),
            &child,
            Amount::from_atomic(0),
        )
        .expect_err("invalid PoW must fail");

        let message = error.to_string().to_lowercase();
        assert!(
            matches!(error, AlvenqisError::InvalidPow { .. })
                || message.contains("pow")
                || message.contains("mix")
                || message.contains("firopow")
                || message.contains("work"),
            "expected early PoW rejection, got: {error:?} / {message}"
        );
        assert!(
            !message.contains("signature") && !message.contains("unsigned"),
            "signature verification must not be the failure mode for invalid PoW: {message}"
        );
    }
}
