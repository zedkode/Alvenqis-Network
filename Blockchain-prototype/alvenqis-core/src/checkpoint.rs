use crate::crypto::Hash;
use crate::errors::{AlvenqisError, Result};
use crate::network::Network;
use serde::Serialize;

pub const CHECKPOINT_POLICY_ID: &str = "alvenqis-hardcoded-checkpoints-v1";
pub const CHECKPOINT_POLICY_MODE: &str = "social-hardcoded-early-network";
pub const CHECKPOINT_POLICY_RELAXATION: &str =
    "Checkpoints remain active for early environments and can be reduced later only through an explicit follow-up protocol freeze.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ChainCheckpoint {
    pub height: u64,
    pub hash_hex: &'static str,
    pub status_label: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CheckpointPolicy {
    pub policy_id: &'static str,
    pub mode: &'static str,
    pub network_id: &'static str,
    pub checkpoints: Vec<ChainCheckpoint>,
    pub relaxation_path: &'static str,
}

const DEVNET_CHECKPOINTS: [ChainCheckpoint; 0] = [];
const TESTNET_CHECKPOINTS: [ChainCheckpoint; 0] = [];
const MAINNET_CANDIDATE_CHECKPOINTS: [ChainCheckpoint; 1] = [ChainCheckpoint {
    height: 0,
    // Alvenqis rebrand genesis (network_id alvenqis-mainnet-candidate, FiroPoW 0.9.4).
    // Prior pre-Alvenqis candidate hashes are retired and must not be reused.
    hash_hex: "0000c29213014578ac41a748c2be3489859f1e0b1f3555bd89b7e5301632a4c5",
    status_label: "Planned / Mainnet Candidate",
}];

pub fn scheduled_checkpoints(network: Network) -> &'static [ChainCheckpoint] {
    match network {
        Network::Devnet => &DEVNET_CHECKPOINTS,
        Network::Testnet => &TESTNET_CHECKPOINTS,
        Network::MainnetCandidate => &MAINNET_CANDIDATE_CHECKPOINTS,
    }
}

pub fn checkpoint_at_height(network: Network, height: u64) -> Option<ChainCheckpoint> {
    scheduled_checkpoints(network)
        .iter()
        .copied()
        .find(|checkpoint| checkpoint.height == height)
}

pub fn validate_checkpoint(network: Network, height: u64, actual_hash: Hash) -> Result<()> {
    let Some(checkpoint) = checkpoint_at_height(network, height) else {
        return Ok(());
    };
    let expected_hash =
        Hash::from_hex(checkpoint.hash_hex).map_err(AlvenqisError::InvalidCheckpointDefinition)?;
    if actual_hash != expected_hash {
        return Err(AlvenqisError::InvalidCheckpoint {
            height,
            expected: expected_hash,
            actual: actual_hash,
        });
    }
    Ok(())
}

pub fn launch_checkpoint_policy(network: Network) -> CheckpointPolicy {
    CheckpointPolicy {
        policy_id: CHECKPOINT_POLICY_ID,
        mode: CHECKPOINT_POLICY_MODE,
        network_id: network.network_id(),
        checkpoints: scheduled_checkpoints(network).to_vec(),
        relaxation_path: CHECKPOINT_POLICY_RELAXATION,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{block_reward, genesis_with_timestamp_for_network};

    #[test]
    fn mainnet_candidate_launch_policy_has_genesis_checkpoint() {
        let policy = launch_checkpoint_policy(Network::MainnetCandidate);

        assert_eq!(policy.policy_id, CHECKPOINT_POLICY_ID);
        assert_eq!(policy.mode, CHECKPOINT_POLICY_MODE);
        assert_eq!(policy.checkpoints.len(), 1);
        assert_eq!(policy.checkpoints[0].height, 0);
    }

    #[test]
    fn candidate_genesis_matches_checkpoint_hash() {
        // Must match default_miner_address / genesis recipient strategy.
        let recipient = "alve1qr4y5mrru2w9yz4774g8kyewchue23mk46ltu7ujgg0w56g5gmfzcnfqv0q";
        let genesis = genesis_with_timestamp_for_network(
            Network::MainnetCandidate,
            recipient,
            1_720_000_000,
            16,
        )
        .expect("genesis");

        validate_checkpoint(
            Network::MainnetCandidate,
            0,
            genesis.hash().expect("genesis hash"),
        )
        .expect("genesis checkpoint should match");
        assert_eq!(genesis.transactions[0].amount, block_reward(0));
    }

    #[test]
    fn wrong_checkpoint_hash_is_rejected() {
        let wrong_hash =
            Hash::from_hex("1111a26d0a9da9577f94350eaed9568f04e7e823f9e2ee5d0df0df52597779c2")
                .expect("hash");

        let error = validate_checkpoint(Network::MainnetCandidate, 0, wrong_hash)
            .expect_err("wrong checkpoint hash must fail");
        assert!(matches!(error, AlvenqisError::InvalidCheckpoint { .. }));
    }
}
