use crate::error::RpcError;
use crate::models::{HealthResponse, NetworkResponse};
use crate::state::RpcState;
use alvenqis_node::{load_p2p_status, runtime_dir_for_data_dir, NetworkConfig, P2pStatus};
use axum::extract::State;
use axum::Json;
use std::path::Path;

pub(crate) async fn p2p_status(State(state): State<RpcState>) -> Result<Json<P2pStatus>, RpcError> {
    let node_config = NetworkConfig::load_from_path(&state.node_config_path)?;
    let status = load_p2p_status(
        &runtime_dir_for_data_dir(Path::new(&state.config.chain_data_path)),
        &node_config,
    )?;
    Ok(Json(status))
}

pub(crate) async fn health(State(state): State<RpcState>) -> Json<HealthResponse> {
    let exposure = match state.config.access_mode {
        crate::config::RpcAccessMode::Local => "Local only",
        crate::config::RpcAccessMode::PublicRead => "Public read",
        crate::config::RpcAccessMode::PublicSubmit => {
            if state.config.mining_endpoints_enabled() {
                "Public submit + mining (loopback/proxy-deny required)"
            } else {
                "Public submit (mining disabled)"
            }
        }
        crate::config::RpcAccessMode::PrivateMining => {
            "Private container-network mining (no published host port)"
        }
    };
    Json(HealthResponse {
        ok: true,
        service: "alvenqis-rpc-gateway",
        mode: format!("{} / {exposure}", state.config.status_label),
        network_id: state.config.network_id.clone(),
        network_name: state.config.human_name.clone(),
        status_label: state.config.status_label.clone(),
    })
}

pub(crate) async fn network(State(state): State<RpcState>) -> Json<NetworkResponse> {
    let protocol = alvenqis_core::launch_protocol_parameters(state.config.network);
    Json(NetworkResponse {
        protocol_parameters_id: protocol.parameters_id,
        protocol_version: protocol.protocol_version,
        block_version: protocol.block_version,
        network_id: state.config.network_id.clone(),
        network_name: state.config.human_name.clone(),
        status_label: state.config.status_label.clone(),
        ticker: protocol.ticker,
        address_prefix: protocol.address_prefix.to_owned(),
        address_standard_id: alvenqis_core::launch_address_standard(state.config.network)
            .standard_id,
        address_encoding: alvenqis_core::launch_address_standard(state.config.network).encoding,
        address_checksum_rule: alvenqis_core::launch_address_standard(state.config.network)
            .checksum_rule,
        address_payload_version: alvenqis_core::launch_address_standard(state.config.network)
            .payload_version,
        public_key_scheme: alvenqis_core::launch_signing_standard().public_key_scheme,
        signature_standard_id: alvenqis_core::launch_signing_standard().standard_id,
        signature_scheme: alvenqis_core::launch_signing_standard().signature_scheme,
        tx_signing_domain: alvenqis_core::launch_signing_standard().tx_signing_domain,
        key_derivation_policy_id: alvenqis_core::launch_key_derivation_policy().policy_id,
        block_time_seconds: protocol.block_time_seconds,
        decimals: protocol.decimals,
        atomic_units_per_alve: protocol.atomic_units_per_alve,
        max_supply_atomic: protocol.max_supply_atomic,
        halving_interval_blocks: protocol.halving_interval_blocks,
        initial_block_reward_atomic: protocol.initial_block_reward_atomic,
        pow_hash_algorithm: protocol.pow_hash_algorithm,
        difficulty_adjustment_algorithm: protocol.difficulty_adjustment_algorithm,
        fee_policy: protocol.fee_policy,
        default_rpc_port: protocol.default_rpc_port,
        default_p2p_port: protocol.default_p2p_port,
        max_transactions_per_block: protocol.max_transactions_per_block,
        max_transaction_wire_bytes: protocol.max_transaction_wire_bytes,
        median_time_past_window: protocol.median_time_past_window,
        max_future_block_drift_seconds: protocol.max_future_block_drift_seconds,
        first_account_nonce: alvenqis_core::FIRST_ACCOUNT_NONCE,
    })
}
