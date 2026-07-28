use crate::amount::Amount;
use crate::block::Block;
use crate::consensus::validate_next_block;
use crate::crypto::Hash;
use crate::errors::{AlvenqisError, Result};
use crate::network::Network;
use crate::state::{apply_block, LedgerState};

pub type ChainWork = u128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForkChoice {
    KeepCurrent,
    AdoptCandidate,
}

pub fn block_work(block: &Block) -> Result<ChainWork> {
    1_u128
        .checked_shl(u32::from(block.header.difficulty_leading_zero_bits))
        .ok_or(crate::errors::AlvenqisError::ChainWorkOverflow)
}

pub fn cumulative_work(blocks: &[Block]) -> Result<ChainWork> {
    blocks.iter().try_fold(0_u128, |total, block| {
        total
            .checked_add(block_work(block)?)
            .ok_or(crate::errors::AlvenqisError::ChainWorkOverflow)
    })
}

pub fn common_ancestor_height(current: &[Block], candidate: &[Block]) -> Result<Option<u64>> {
    let mut last_match = None;
    for (left, right) in current.iter().zip(candidate) {
        if left.hash()? != right.hash()? {
            break;
        }
        last_match = Some(left.header.height);
    }
    Ok(last_match)
}

pub fn select_fork(current: &[Block], candidate: &[Block]) -> Result<ForkChoice> {
    let current_genesis = current.first().ok_or_else(|| {
        crate::errors::AlvenqisError::InvalidGenesis("current chain is empty".to_owned())
    })?;
    let candidate_genesis = candidate.first().ok_or_else(|| {
        crate::errors::AlvenqisError::InvalidGenesis("candidate chain is empty".to_owned())
    })?;
    if current_genesis.hash()? != candidate_genesis.hash()? {
        return Err(crate::errors::AlvenqisError::InvalidGenesis(
            "candidate chain has a different genesis".to_owned(),
        ));
    }

    if cumulative_work(candidate)? > cumulative_work(current)? {
        Ok(ForkChoice::AdoptCandidate)
    } else {
        Ok(ForkChoice::KeepCurrent)
    }
}

#[derive(Clone, Debug)]
pub struct Chain {
    network: Network,
    blocks: Vec<Block>,
    state: LedgerState,
}

impl Chain {
    pub fn new(network: Network) -> Self {
        Self {
            network,
            blocks: Vec::new(),
            state: LedgerState::new(),
        }
    }

    pub fn from_blocks<I>(network: Network, blocks: I) -> Result<Self>
    where
        I: IntoIterator<Item = Block>,
    {
        let mut chain = Self::new(network);
        for block in blocks {
            chain.append_block(block)?;
        }
        Ok(chain)
    }

    pub fn from_persisted_state<I>(network: Network, blocks: I, state: LedgerState) -> Result<Self>
    where
        I: IntoIterator<Item = Block>,
    {
        let blocks: Vec<Block> = blocks.into_iter().collect();
        for block in &blocks {
            let block_network = block.network()?;
            if block_network != network {
                return Err(AlvenqisError::InvalidNetwork {
                    expected: network.network_id().to_owned(),
                    actual: block.header.network_id.clone(),
                });
            }
        }

        let Some(tip) = blocks.last() else {
            if state != LedgerState::new() {
                return Err(invalid_persisted_state(
                    "non-empty ledger state cannot restore an empty chain",
                ));
            }
            return Ok(Self {
                network,
                blocks,
                state,
            });
        };

        let applied_height = state
            .applied_block_height()
            .ok_or_else(|| invalid_persisted_state("applied block height is missing"))?;
        if applied_height != tip.header.height {
            return Err(invalid_persisted_state(format!(
                "applied height {applied_height} does not match tip height {}",
                tip.header.height
            )));
        }

        let persisted_tip_hash = state
            .tip_hash()
            .ok_or_else(|| invalid_persisted_state("tip hash is missing"))?;
        let actual_tip_hash = tip.hash()?;
        if persisted_tip_hash != actual_tip_hash {
            return Err(invalid_persisted_state(
                "persisted tip hash does not match the block tip",
            ));
        }

        let persisted_tip_timestamp = state
            .tip_timestamp()
            .ok_or_else(|| invalid_persisted_state("tip timestamp is missing"))?;
        if persisted_tip_timestamp != tip.header.timestamp {
            return Err(invalid_persisted_state(format!(
                "tip timestamp {persisted_tip_timestamp} does not match block timestamp {}",
                tip.header.timestamp
            )));
        }

        Ok(Self {
            network,
            blocks,
            state,
        })
    }

    pub fn append_block(&mut self, block: Block) -> Result<()> {
        validate_next_block(
            self.network,
            &self.blocks,
            self.blocks.last(),
            &block,
            self.state.emitted_supply(),
        )?;
        apply_block(&mut self.state, &block)?;
        self.blocks.push(block);
        Ok(())
    }

    pub fn height(&self) -> Option<u64> {
        self.blocks.last().map(|block| block.header.height)
    }

    pub fn tip_hash(&self) -> Result<Option<Hash>> {
        self.blocks.last().map(|block| block.hash()).transpose()
    }

    pub fn emitted_supply(&self) -> Amount {
        self.state.emitted_supply()
    }

    pub fn cumulative_work(&self) -> Result<ChainWork> {
        cumulative_work(&self.blocks)
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    pub fn state(&self) -> &LedgerState {
        &self.state
    }

    pub const fn network(&self) -> Network {
        self.network
    }
}

fn invalid_persisted_state(message: impl Into<String>) -> AlvenqisError {
    AlvenqisError::InvalidGenesis(format!(
        "persisted ledger state mismatch: {}",
        message.into()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{devnet_genesis_with_difficulty, Address, Network, PrivateKey};

    fn address(seed: u8) -> String {
        Address::from_public_key_for_network(
            &PrivateKey::from_bytes([seed; 32]).public_key(),
            Network::Devnet,
        )
        .to_string()
    }

    #[test]
    fn work_is_two_to_the_leading_zero_bits() {
        let block = devnet_genesis_with_difficulty(&address(1), 12).expect("genesis");
        assert_eq!(block_work(&block).expect("work"), 1_u128 << 12);
    }

    #[test]
    fn equal_work_keeps_the_current_chain() {
        let genesis = devnet_genesis_with_difficulty(&address(2), 4).expect("genesis");
        assert_eq!(
            select_fork(
                std::slice::from_ref(&genesis),
                std::slice::from_ref(&genesis)
            )
            .expect("fork choice"),
            ForkChoice::KeepCurrent
        );
    }

    #[test]
    fn common_ancestor_requires_matching_block_hashes() {
        let genesis = devnet_genesis_with_difficulty(&address(3), 4).expect("genesis");
        let mut changed = genesis.clone();
        changed.header.nonce = changed.header.nonce.saturating_add(1);
        assert_eq!(
            common_ancestor_height(
                std::slice::from_ref(&genesis),
                std::slice::from_ref(&genesis)
            )
            .expect("ancestor"),
            Some(0)
        );
        assert_eq!(
            common_ancestor_height(&[changed], &[]).expect("ancestor empty"),
            None
        );
    }

    #[test]
    fn different_genesis_is_rejected_before_work_comparison() {
        let left = devnet_genesis_with_difficulty(&address(4), 4).expect("left");
        let right = devnet_genesis_with_difficulty(&address(5), 4).expect("right");
        let error = select_fork(&[left], &[right]).expect_err("different genesis");
        assert!(error.to_string().contains("different genesis"));
    }
}
