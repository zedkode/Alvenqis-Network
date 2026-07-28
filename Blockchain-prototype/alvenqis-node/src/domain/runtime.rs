use crate::config::NetworkConfig;
use crate::domain::chain::{
    ensure_network_storage_path, prototype_mode, summarize_validated_blocks,
};
use crate::domain::genesis::{
    initialize_deterministic_genesis, load_genesis_marker, pinned_genesis_approval_status,
};
use crate::error::{NodeError, NodeResult};
use crate::mempool::{
    clear_mempool, current_unix_seconds, default_network_root,
    load_pending_transactions_for_chain, mempool_runtime_fingerprint,
};
use crate::p2p::{load_p2p_status, run_p2p_service};
use crate::storage;
use alvenqis_core::Network;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

pub const DEFAULT_DATA_DIR: &str = ".alvenqis-mainnet/chain";
const NODE_RUNTIME_DIR_NAME: &str = "node";
const NODE_RUNTIME_FILE_NAME: &str = "runtime.json";
const NODE_SHUTDOWN_FILE_NAME: &str = "shutdown.signal";
const NODE_POLL_INTERVAL_SECONDS: u64 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeRuntimeStatus {
    pub mode: String,
    pub running: bool,
    pub pid: Option<u32>,
    pub network_id: String,
    pub network_name: String,
    pub status_label: String,
    pub data_dir: String,
    pub mempool_dir: String,
    pub runtime_dir: String,
    pub chain_initialized: bool,
    pub height: Option<u64>,
    pub block_count: usize,
    pub tip_hash: Option<String>,
    pub emitted_supply_atomic: Option<u64>,
    pub pending_count: usize,
    pub genesis_hash: Option<String>,
    pub genesis_approval_required: bool,
    pub genesis_approval_path: Option<String>,
    pub genesis_approved: bool,
    pub genesis_review_hash: Option<String>,
    pub genesis_approved_by: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PersistedRuntimeProcessState {
    #[serde(default)]
    running: bool,
    #[serde(default)]
    pid: Option<u32>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ResetSummary {
    pub status: String,
    pub network_id: String,
    pub data_dir: String,
    pub mempool_dir: String,
    pub backup_dir: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PeersSummary {
    pub mode: String,
    pub network_id: String,
    pub local_p2p_port: u16,
    pub chain_magic_hex: String,
    pub connected_peers: Vec<String>,
    pub local_peer_id: String,
    pub listen_addresses: Vec<String>,
    pub configured_seed_count: usize,
    pub discovered_peer_count: usize,
    pub connected_peer_count: usize,
    pub validated_peer_count: usize,
    pub mining_peer_count: usize,
    pub observed_network_hashrate_hs: u64,
    pub miners: Vec<crate::p2p::NetworkMinerPresence>,
    pub validating_peer_count: usize,
    pub syncing: bool,
    pub banned_peer_count: usize,
    pub peers: Vec<crate::p2p::ConnectedPeer>,
    pub last_error: Option<String>,
}

pub fn default_data_dir(network: Network) -> PathBuf {
    default_network_root(network).join("chain")
}

pub fn default_runtime_dir(network: Network) -> PathBuf {
    default_network_root(network).join(NODE_RUNTIME_DIR_NAME)
}

pub fn runtime_dir_for_data_dir(data_dir: &Path) -> PathBuf {
    data_dir
        .parent()
        .unwrap_or(data_dir)
        .join(NODE_RUNTIME_DIR_NAME)
}

pub fn reset_devnet(
    config_path: &Path,
    data_dir: &Path,
    mempool_dir: &Path,
    confirm: bool,
) -> NodeResult<ResetSummary> {
    let config = NetworkConfig::load_from_path(config_path)?;
    if !config.network.is_resettable() {
        return Err(NodeError::ResetNotAllowed(
            config.network.network_id().to_owned(),
        ));
    }
    ensure_network_storage_path(config.network, data_dir)?;
    ensure_network_storage_path(config.network, mempool_dir)?;
    if !confirm {
        return Err(NodeError::ResetConfirmationRequired(
            config.network.network_id().to_owned(),
        ));
    }
    if node_runtime_is_running(config.network, data_dir)? {
        return Err(NodeError::ResetWhileNodeRunning(
            config.network.network_id().to_owned(),
        ));
    }

    let backup_dir = backup_resettable_paths(data_dir, mempool_dir)?;
    storage::reset_data_dir(data_dir)?;
    clear_mempool(mempool_dir)?;
    Ok(ResetSummary {
        status: prototype_mode(config.network),
        network_id: config.network.network_id().to_owned(),
        data_dir: data_dir.display().to_string(),
        mempool_dir: mempool_dir.display().to_string(),
        backup_dir: backup_dir.map(|path| path.display().to_string()),
    })
}

pub fn start_node(
    config_path: &Path,
    data_dir: &Path,
    mempool_dir: &Path,
    force_genesis: bool,
) -> NodeResult<()> {
    NetworkConfig::load_from_path(config_path)?;
    let runtime_dir = runtime_dir_for_data_dir(data_dir);
    fs::create_dir_all(&runtime_dir)?;
    let shutdown_path = runtime_dir.join(NODE_SHUTDOWN_FILE_NAME);
    if shutdown_path.exists() {
        fs::remove_file(&shutdown_path)?;
    }

    let mut runtime = build_runtime_status(
        config_path,
        data_dir,
        mempool_dir,
        true,
        true,
        force_genesis,
    )?;
    write_runtime_status_file(&runtime_dir, &runtime)?;
    let (mut chain_fingerprint, mut mempool_fingerprint) =
        runtime_data_fingerprint(data_dir, mempool_dir);

    let stop_p2p = Arc::new(AtomicBool::new(false));
    let p2p_handle = {
        let stop = Arc::clone(&stop_p2p);
        let config_path = config_path.to_path_buf();
        let data_dir = data_dir.to_path_buf();
        let mempool_dir = mempool_dir.to_path_buf();
        let runtime_dir = runtime_dir.clone();
        thread::Builder::new()
            .name("alvenqis-p2p".to_owned())
            .spawn(move || run_p2p_service(config_path, data_dir, mempool_dir, runtime_dir, stop))?
    };

    loop {
        if shutdown_path.exists() {
            break;
        }

        if p2p_handle.is_finished() {
            return p2p_handle
                .join()
                .map_err(|_| NodeError::P2p("P2P worker panicked".to_owned()))?;
        }

        thread::sleep(Duration::from_secs(NODE_POLL_INTERVAL_SECONDS));
        let (next_chain_fingerprint, next_mempool_fingerprint) =
            runtime_data_fingerprint(data_dir, mempool_dir);
        if next_chain_fingerprint != chain_fingerprint {
            runtime = build_runtime_status(config_path, data_dir, mempool_dir, true, false, false)?;
            write_runtime_status_file(&runtime_dir, &runtime)?;
            chain_fingerprint = next_chain_fingerprint;
            mempool_fingerprint = next_mempool_fingerprint;
        } else if next_mempool_fingerprint != mempool_fingerprint {
            runtime.pending_count =
                load_pending_transactions_for_chain(data_dir, mempool_dir)?.len();
            write_runtime_status_file(&runtime_dir, &runtime)?;
            mempool_fingerprint = next_mempool_fingerprint;
        }
    }

    stop_p2p.store(true, Ordering::Relaxed);
    p2p_handle
        .join()
        .map_err(|_| NodeError::P2p("P2P worker panicked".to_owned()))??;
    if shutdown_path.exists() {
        fs::remove_file(&shutdown_path)?;
    }

    let stopped_runtime = NodeRuntimeStatus {
        running: false,
        pid: None,
        ..runtime
    };
    write_runtime_status_file(&runtime_dir, &stopped_runtime)?;
    Ok(())
}

pub fn node_status(
    config_path: &Path,
    data_dir: &Path,
    mempool_dir: &Path,
) -> NodeResult<NodeRuntimeStatus> {
    let mut status = build_runtime_status(config_path, data_dir, mempool_dir, false, false, false)?;
    let runtime_path = runtime_status_file_path(Path::new(&status.runtime_dir));
    if runtime_path.exists() {
        let persisted: PersistedRuntimeProcessState =
            serde_json::from_str(&fs::read_to_string(runtime_path)?)?;
        status.running = persisted
            .pid
            .is_some_and(|pid| persisted.running && process_is_running(pid));
        status.pid = status.running.then_some(persisted.pid).flatten();
    }
    Ok(status)
}

pub fn peers(config_path: &Path, data_dir: &Path) -> NodeResult<PeersSummary> {
    let config = NetworkConfig::load_from_path(config_path)?;
    ensure_network_storage_path(config.network, data_dir)?;
    let status = load_p2p_status(&runtime_dir_for_data_dir(data_dir), &config)?;
    let local_p2p_port = config.p2p_listen_port();
    Ok(PeersSummary {
        mode: status.mode,
        network_id: config.network_id,
        local_p2p_port,
        chain_magic_hex: config.chain_magic_hex,
        connected_peers: status
            .peers
            .iter()
            .filter(|peer| peer.handshake_validated)
            .map(|peer| peer.peer_id.clone())
            .collect(),
        local_peer_id: status.local_peer_id,
        listen_addresses: status.listen_addresses,
        configured_seed_count: status.configured_seed_count,
        discovered_peer_count: status.discovered_peer_count,
        connected_peer_count: status.connected_peer_count,
        validated_peer_count: status.validated_peer_count,
        mining_peer_count: status.mining_peer_count,
        observed_network_hashrate_hs: status.observed_network_hashrate_hs,
        miners: status.miners,
        validating_peer_count: status.validating_peer_count,
        syncing: status.syncing,
        banned_peer_count: status.banned_peer_count,
        peers: status.peers,
        last_error: status.last_error,
    })
}

pub fn shutdown(network: Network, data_dir: &Path) -> NodeResult<String> {
    ensure_network_storage_path(network, data_dir)?;
    let runtime_dir = runtime_dir_for_data_dir(data_dir);
    let runtime_path = runtime_dir.join(NODE_RUNTIME_FILE_NAME);
    if !runtime_path.exists() {
        return Err(NodeError::ShutdownNotRunning(
            network.network_id().to_owned(),
        ));
    }

    fs::create_dir_all(&runtime_dir)?;
    fs::write(runtime_dir.join(NODE_SHUTDOWN_FILE_NAME), "shutdown\n")?;
    Ok(format!(
        "shutdown requested for network_id={} runtime_dir={}",
        network.network_id(),
        runtime_dir.display()
    ))
}

fn build_runtime_status(
    config_path: &Path,
    data_dir: &Path,
    mempool_dir: &Path,
    running: bool,
    initialize_if_missing: bool,
    force_genesis: bool,
) -> NodeResult<NodeRuntimeStatus> {
    let config = NetworkConfig::load_from_path(config_path)?;
    ensure_network_storage_path(config.network, data_dir)?;
    ensure_network_storage_path(config.network, mempool_dir)?;
    let runtime_dir = runtime_dir_for_data_dir(data_dir);
    fs::create_dir_all(&runtime_dir)?;
    let genesis_approval = pinned_genesis_approval_status(config_path).ok();

    let summary = match storage::load_blocks(data_dir) {
        Ok(existing_blocks) => Some(summarize_validated_blocks(
            config_path,
            &config,
            data_dir,
            &existing_blocks,
        )?),
        Err(NodeError::ChainNotInitialized(_)) => {
            if initialize_if_missing {
                if config.network.requires_explicit_allow() {
                    initialize_deterministic_genesis(config_path, data_dir, force_genesis)?;
                    let blocks = storage::load_blocks(data_dir)?;
                    Some(summarize_validated_blocks(
                        config_path,
                        &config,
                        data_dir,
                        &blocks,
                    )?)
                } else {
                    None
                }
            } else {
                None
            }
        }
        Err(error) => return Err(error),
    };
    let pending = load_pending_transactions_for_chain(data_dir, mempool_dir)?;
    let genesis_hash = load_genesis_marker(data_dir)
        .ok()
        .map(|marker| marker.genesis_hash);

    Ok(NodeRuntimeStatus {
        mode: format!("{} / Prototype", config.status_label),
        running,
        pid: running.then(std::process::id),
        network_id: config.network_id,
        network_name: config.human_name,
        status_label: config.status_label,
        data_dir: data_dir.display().to_string(),
        mempool_dir: mempool_dir.display().to_string(),
        runtime_dir: runtime_dir.display().to_string(),
        chain_initialized: summary.is_some(),
        height: summary.as_ref().map(|inner| inner.height),
        block_count: summary.as_ref().map_or(0, |inner| inner.block_count),
        tip_hash: summary.as_ref().map(|inner| inner.tip_hash.clone()),
        emitted_supply_atomic: summary.as_ref().map(|inner| inner.emitted_supply_atomic),
        pending_count: pending.len(),
        genesis_hash,
        genesis_approval_required: config.network.requires_explicit_allow(),
        genesis_approval_path: genesis_approval
            .as_ref()
            .and_then(|status| status.approval_path.clone()),
        genesis_approved: genesis_approval
            .as_ref()
            .is_some_and(|status| status.approved),
        genesis_review_hash: genesis_approval
            .as_ref()
            .map(|status| status.review_hash.clone()),
        genesis_approved_by: genesis_approval
            .as_ref()
            .and_then(|status| status.approved_by.clone()),
    })
}

fn runtime_status_file_path(runtime_dir: &Path) -> PathBuf {
    runtime_dir.join(NODE_RUNTIME_FILE_NAME)
}

fn write_runtime_status_file(runtime_dir: &Path, status: &NodeRuntimeStatus) -> NodeResult<()> {
    fs::create_dir_all(runtime_dir)?;
    fs::write(
        runtime_status_file_path(runtime_dir),
        serde_json::to_string_pretty(status)?,
    )?;
    Ok(())
}

fn runtime_data_fingerprint(
    data_dir: &Path,
    mempool_dir: &Path,
) -> (storage::ChainStorageFingerprint, (u64, Option<SystemTime>)) {
    (
        storage::chain_storage_fingerprint(data_dir),
        mempool_runtime_fingerprint(data_dir, mempool_dir),
    )
}

fn node_runtime_is_running(network: Network, data_dir: &Path) -> NodeResult<bool> {
    ensure_network_storage_path(network, data_dir)?;
    let runtime_path = runtime_status_file_path(&runtime_dir_for_data_dir(data_dir));
    if !runtime_path.exists() {
        return Ok(false);
    }

    let status: PersistedRuntimeProcessState =
        serde_json::from_str(&fs::read_to_string(runtime_path)?)?;
    Ok(status
        .pid
        .is_some_and(|pid| status.running && process_is_running(pid)))
}

#[cfg(windows)]
fn process_is_running(pid: u32) -> bool {
    std::process::Command::new("tasklist.exe")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .ok()
        .is_some_and(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
}

#[cfg(not(windows))]
fn process_is_running(pid: u32) -> bool {
    Path::new("/proc").join(pid.to_string()).exists()
}

fn backup_resettable_paths(data_dir: &Path, mempool_dir: &Path) -> NodeResult<Option<PathBuf>> {
    let data_exists = data_dir.exists();
    let mempool_exists = mempool_dir.exists();
    if !data_exists && !mempool_exists {
        return Ok(None);
    }

    let network_root = data_dir.parent().ok_or_else(|| {
        NodeError::Input(format!(
            "cannot determine network root from data dir {}",
            data_dir.display()
        ))
    })?;
    let backup_root = network_root.join("backups").join(format!(
        "reset-{}-{}",
        current_unix_seconds(),
        std::process::id()
    ));
    fs::create_dir_all(&backup_root)?;

    move_dir_if_exists(data_dir, &backup_root.join("chain"))?;
    move_dir_if_exists(mempool_dir, &backup_root.join("mempool"))?;
    Ok(Some(backup_root))
}

fn move_dir_if_exists(source: &Path, destination: &Path) -> NodeResult<()> {
    if !source.exists() {
        return Ok(());
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    if destination.exists() {
        fs::remove_dir_all(destination)?;
    }
    fs::rename(source, destination)?;
    Ok(())
}

#[cfg(test)]
mod runtime_status_tests {
    use super::PersistedRuntimeProcessState;

    #[test]
    fn legacy_runtime_status_only_requires_process_fields() {
        let status: PersistedRuntimeProcessState = serde_json::from_str(
            r#"{"mode":"legacy","running":true,"pid":42,"network_id":"legacy"}"#,
        )
        .expect("legacy runtime status should remain readable");

        assert!(status.running);
        assert_eq!(status.pid, Some(42));
    }
}
