use crate::constants::{
    ATOMIC_UNITS_PER_ALVE, BLOCK_TIME_SECONDS, CONSENSUS_STATUS, DAA_ALGORITHM, DECIMALS,
    FEE_POLICY, HALVING_INTERVAL_BLOCKS, INITIAL_BLOCK_REWARD_ALVE, INITIAL_BLOCK_REWARD_ATOMIC,
    MAX_FUTURE_BLOCK_DRIFT_SECONDS, MAX_SUPPLY_ALVE, MAX_SUPPLY_ATOMIC, MAX_TRANSACTIONS_PER_BLOCK,
    MAX_TRANSACTION_WIRE_BYTES, MEDIAN_TIME_PAST_WINDOW, POW_HASH_ALGORITHM, PROJECT_NAME, TICKER,
};
use crate::network::Network;
use crate::upgrade::{LAUNCH_BLOCK_VERSION, LAUNCH_PROTOCOL_VERSION};

pub const PROTOCOL_PARAMETERS_ID: &str = "alvenqis-launch-parameters-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolParameters {
    pub parameters_id: &'static str,
    pub protocol_version: u32,
    pub block_version: u32,
    pub network: Network,
    pub network_id: &'static str,
    pub network_name: &'static str,
    pub status_label: &'static str,
    pub project_name: &'static str,
    pub ticker: &'static str,
    pub address_prefix: &'static str,
    pub decimals: u32,
    pub atomic_units_per_alve: u64,
    pub max_supply_alve: u64,
    pub max_supply_atomic: u64,
    pub block_time_seconds: u64,
    pub halving_interval_blocks: u64,
    pub initial_block_reward_alve: &'static str,
    pub initial_block_reward_atomic: u64,
    pub consensus: &'static str,
    pub pow_hash_algorithm: &'static str,
    pub difficulty_adjustment_algorithm: &'static str,
    pub fee_policy: &'static str,
    pub default_rpc_port: u16,
    pub default_p2p_port: u16,
    pub chain_magic_bytes: [u8; 4],
    /// Consensus hard caps (DoS bounds) published for clients/miners.
    pub max_transactions_per_block: usize,
    pub max_transaction_wire_bytes: usize,
    pub median_time_past_window: usize,
    pub max_future_block_drift_seconds: u64,
}

pub const fn launch_protocol_parameters(network: Network) -> ProtocolParameters {
    ProtocolParameters {
        parameters_id: PROTOCOL_PARAMETERS_ID,
        protocol_version: LAUNCH_PROTOCOL_VERSION,
        block_version: LAUNCH_BLOCK_VERSION,
        network,
        network_id: network.network_id(),
        network_name: network.human_name(),
        status_label: network.status_label(),
        project_name: PROJECT_NAME,
        ticker: TICKER,
        address_prefix: network.address_prefix(),
        decimals: DECIMALS,
        atomic_units_per_alve: ATOMIC_UNITS_PER_ALVE,
        max_supply_alve: MAX_SUPPLY_ALVE,
        max_supply_atomic: MAX_SUPPLY_ATOMIC,
        block_time_seconds: BLOCK_TIME_SECONDS,
        halving_interval_blocks: HALVING_INTERVAL_BLOCKS,
        initial_block_reward_alve: INITIAL_BLOCK_REWARD_ALVE,
        initial_block_reward_atomic: INITIAL_BLOCK_REWARD_ATOMIC,
        consensus: CONSENSUS_STATUS,
        pow_hash_algorithm: POW_HASH_ALGORITHM,
        difficulty_adjustment_algorithm: DAA_ALGORITHM,
        fee_policy: FEE_POLICY,
        default_rpc_port: network.default_rpc_port(),
        default_p2p_port: network.default_p2p_port(),
        chain_magic_bytes: network.chain_magic_bytes(),
        max_transactions_per_block: MAX_TRANSACTIONS_PER_BLOCK,
        max_transaction_wire_bytes: MAX_TRANSACTION_WIRE_BYTES,
        median_time_past_window: MEDIAN_TIME_PAST_WINDOW,
        max_future_block_drift_seconds: MAX_FUTURE_BLOCK_DRIFT_SECONDS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_parameters_are_network_specific_without_changing_economics() {
        let devnet = launch_protocol_parameters(Network::Devnet);
        let candidate = launch_protocol_parameters(Network::MainnetCandidate);

        assert_eq!(candidate.parameters_id, PROTOCOL_PARAMETERS_ID);
        assert_eq!(candidate.network_id, "alvenqis-mainnet-candidate");
        assert_eq!(candidate.address_prefix, "alve");
        assert_eq!(candidate.default_rpc_port, 10_787);
        assert_eq!(candidate.default_p2p_port, 20_787);
        assert_ne!(devnet.network_id, candidate.network_id);
        assert_ne!(devnet.address_prefix, candidate.address_prefix);
        assert_eq!(devnet.max_supply_atomic, candidate.max_supply_atomic);
        assert_eq!(devnet.block_time_seconds, candidate.block_time_seconds);
        assert_eq!(devnet.initial_block_reward_atomic, 1_902_587_519);
        assert_eq!(
            candidate.max_transactions_per_block,
            MAX_TRANSACTIONS_PER_BLOCK
        );
        assert_eq!(
            candidate.max_transaction_wire_bytes,
            MAX_TRANSACTION_WIRE_BYTES
        );
        assert_eq!(candidate.median_time_past_window, MEDIAN_TIME_PAST_WINDOW);
        assert_eq!(
            candidate.max_future_block_drift_seconds,
            MAX_FUTURE_BLOCK_DRIFT_SECONDS
        );
    }
}
