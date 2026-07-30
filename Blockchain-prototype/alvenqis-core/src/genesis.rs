use crate::block::Block;
use crate::consensus::{
    block_reward, check_pow, initial_base_fee, next_base_fee, next_difficulty_for_network,
};
use crate::crypto::Hash;
use crate::errors::{AlvenqisError, Result};
use crate::firopow::FiroPow;
use crate::network::Network;
use crate::transaction::Transaction;

pub const DEVNET_STATUS_WARNING: &str =
    "Internal test profile only. The operator-facing network is Mainnet Candidate.";
pub const DEVNET_GENESIS_TIMESTAMP: u64 = 1_720_000_000;
pub const DEVNET_DIFFICULTY_LEADING_ZERO_BITS: u8 = 8;

const MAINNET_CANDIDATE_GENESIS_RECIPIENT: &str =
    "alve1qr4y5mrru2w9yz4774g8kyewchue23mk46ltu7ujgg0w56g5gmfzcnfqv0q";
const MAINNET_CANDIDATE_GENESIS_TIMESTAMP: u64 = 1_720_000_000;
const MAINNET_CANDIDATE_GENESIS_DIFFICULTY_LEADING_ZERO_BITS: u8 = 16;
const MAINNET_CANDIDATE_GENESIS_NONCE: u64 = 88_489;
const MAINNET_CANDIDATE_GENESIS_MIX_HASH: [u8; 32] = [
    130, 241, 68, 4, 33, 28, 133, 191, 38, 57, 131, 251, 77, 100, 190, 247, 41, 198, 239, 216, 205,
    43, 129, 234, 170, 246, 109, 99, 43, 39, 81, 57,
];

pub fn devnet_genesis(recipient: &str) -> Result<Block> {
    devnet_genesis_with_difficulty(recipient, DEVNET_DIFFICULTY_LEADING_ZERO_BITS)
}

pub fn devnet_genesis_with_difficulty(
    recipient: &str,
    difficulty_leading_zero_bits: u8,
) -> Result<Block> {
    genesis_with_difficulty_for_network(Network::Devnet, recipient, difficulty_leading_zero_bits)
}

pub fn genesis_with_difficulty_for_network(
    network: Network,
    recipient: &str,
    difficulty_leading_zero_bits: u8,
) -> Result<Block> {
    genesis_with_timestamp_for_network(
        network,
        recipient,
        DEVNET_GENESIS_TIMESTAMP,
        difficulty_leading_zero_bits,
    )
}

pub fn genesis_with_timestamp_for_network(
    network: Network,
    recipient: &str,
    timestamp: u64,
    difficulty_leading_zero_bits: u8,
) -> Result<Block> {
    if recipient.trim().is_empty() {
        return Err(AlvenqisError::InvalidGenesis(
            "recipient cannot be empty".to_owned(),
        ));
    }

    let coinbase = Transaction::coinbase(0, recipient.to_owned(), block_reward(0))?;
    let mut block = Block::new(
        network,
        0,
        Hash::zero(),
        initial_base_fee().as_atomic(),
        timestamp,
        difficulty_leading_zero_bits,
        vec![coinbase],
    )?;
    if network == Network::MainnetCandidate
        && recipient == MAINNET_CANDIDATE_GENESIS_RECIPIENT
        && timestamp == MAINNET_CANDIDATE_GENESIS_TIMESTAMP
        && difficulty_leading_zero_bits == MAINNET_CANDIDATE_GENESIS_DIFFICULTY_LEADING_ZERO_BITS
    {
        block.header.nonce = MAINNET_CANDIDATE_GENESIS_NONCE;
        block.header.mix_hash = Hash::from_bytes(MAINNET_CANDIDATE_GENESIS_MIX_HASH);
        FiroPow.ensure_valid(&block)?;
        return Ok(block);
    }
    mine_block(&mut block);
    Ok(block)
}

pub fn devnet_child_block(
    previous: &Block,
    recipient: &str,
    timestamp: u64,
    transactions: Vec<Transaction>,
) -> Result<Block> {
    devnet_child_block_with_difficulty(
        previous,
        recipient,
        timestamp,
        transactions,
        DEVNET_DIFFICULTY_LEADING_ZERO_BITS,
    )
}

pub fn devnet_child_block_with_difficulty(
    previous: &Block,
    recipient: &str,
    timestamp: u64,
    transactions: Vec<Transaction>,
    difficulty_leading_zero_bits: u8,
) -> Result<Block> {
    if recipient.trim().is_empty() {
        return Err(AlvenqisError::InvalidGenesis(
            "recipient cannot be empty".to_owned(),
        ));
    }

    let base_fee = next_base_fee(Some(previous));
    let total_priority_fees =
        transactions
            .iter()
            .try_fold(crate::Amount::ZERO, |total, transaction| {
                transaction.validate_fee_against_base_fee(base_fee)?;
                total.checked_add(transaction.effective_priority_fee(base_fee)?)
            })?;
    let network = previous.network()?;
    let mut block_transactions = Vec::with_capacity(transactions.len() + 1);
    block_transactions.push(Transaction::coinbase(
        previous.header.height + 1,
        recipient.to_owned(),
        block_reward(previous.header.height + 1).checked_add(total_priority_fees)?,
    )?);
    block_transactions.extend(transactions);

    let mut block = Block::new(
        network,
        previous.header.height + 1,
        previous.hash()?,
        base_fee.as_atomic(),
        timestamp.max(previous.header.timestamp.saturating_add(1)),
        difficulty_leading_zero_bits,
        block_transactions,
    )?;
    mine_block(&mut block);
    Ok(block)
}

pub fn child_block_with_consensus_difficulty(
    previous_chain: &[Block],
    recipient: &str,
    timestamp: u64,
    transactions: Vec<Transaction>,
    fallback_difficulty_leading_zero_bits: u8,
) -> Result<Block> {
    let previous = previous_chain.last().ok_or_else(|| {
        AlvenqisError::InvalidGenesis("previous chain cannot be empty".to_owned())
    })?;
    let network = previous.network()?;
    let next_difficulty = next_difficulty_for_network(
        network,
        previous_chain,
        fallback_difficulty_leading_zero_bits,
    );
    devnet_child_block_with_difficulty(
        previous,
        recipient,
        timestamp,
        transactions,
        next_difficulty,
    )
}

/// Host multi-thread light-context FiroPoW search for genesis/tests/operator one-shot.
/// **Not** product continuous mining (that is GPU-orchestrated in `alvenqis-miner`).
pub fn mine_block(block: &mut Block) {
    use crate::firopow::{mine_firopow_solution, FiroPow};
    let bits = block.header.difficulty_leading_zero_bits;
    let mut start = 0u64;
    // Larger chunks amortize the multi-threaded one-shot search overhead.
    let chunk = 250_000u64;
    loop {
        if let Ok(Some((nonce, out))) = mine_firopow_solution(block, bits, start, chunk) {
            block.header.nonce = nonce;
            block.header.mix_hash = out.mix_hash;
            debug_assert!(FiroPow.ensure_valid(block).is_ok());
            return;
        }
        start = start.wrapping_add(chunk);
        if start == 0 {
            break;
        }
    }
    loop {
        if let Ok(out) = block.pow_hash_with_nonce(block.header.nonce) {
            if check_pow(&out.final_hash, bits) {
                block.header.mix_hash = out.mix_hash;
                return;
            }
        }
        block.header.nonce = block.header.nonce.wrapping_add(1);
    }
}
