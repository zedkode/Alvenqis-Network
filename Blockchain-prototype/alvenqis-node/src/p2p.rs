use crate::config::NetworkConfig;
#[cfg(test)]
use crate::devnet::genesis_hash_hex_from_config;
use crate::devnet::{adopt_candidate_chain, submit_mined_block, submit_transaction};
use crate::error::{NodeError, NodeResult};
use crate::peer_reputation::{
    acknowledge_admin_requests, enqueue_admin_request, load_admin_requests, PeerAdminAction,
    ReputationStore, DEFAULT_BAN_SECONDS,
};
use crate::storage;
use alvenqis_core::{cumulative_work, hash_to_hex, Block, Chain, Transaction};
use atomic_write_file::AtomicWriteFile;
use futures::StreamExt;
use libp2p::connection_limits;
use libp2p::gossipsub;
use libp2p::identify;
use libp2p::identity::Keypair;
use libp2p::ping;
use libp2p::request_response::{self, ProtocolSupport};
use libp2p::swarm::dial_opts::DialOpts;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{noise, yamux, Multiaddr, PeerId, StreamProtocol, Swarm, SwarmBuilder};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::net::IpAddr;
use std::num::NonZeroU8;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Protocol v3: peer scoring/bans + header-first branch verification (A-H05).
pub const P2P_PROTOCOL_VERSION: u32 = 3;
pub const P2P_STATUS_FILE_NAME: &str = "p2p-status.json";
const P2P_IDENTITY_FILE_NAME: &str = "p2p-identity.key";
const MAX_SYNC_BLOCKS: usize = 32;
const MAX_SYNC_HEADERS: usize = 512;
const MAX_STAGED_REORG_BLOCKS: usize = 2_048;
/// Hard cap on staged headers per peer branch (audit CR-H06 memory DoS).
/// Bodies remain capped by MAX_STAGED_REORG_BLOCKS; headers were previously unbounded.
const MAX_STAGED_HEADERS: usize = 2_048;
/// Refuse advertised height deltas larger than this without intermediate checkpoints.
const MAX_HEADER_HEIGHT_DELTA: u64 = 2_048;
const MINER_PRESENCE_TTL_SECONDS: u64 = 30;
const MAX_CONCURRENT_SYNC_BRANCHES: usize = 4;
const MAX_OUTBOUND_PEERS: usize = 8;
const MAX_PENDING_INBOUND_CONNECTIONS: u32 = 32;
const MAX_PENDING_OUTBOUND_CONNECTIONS: u32 = 8;
const MAX_CONNECTIONS_PER_PEER: u32 = 2;
const MAX_NEGOTIATING_INBOUND_STREAMS: usize = 16;
const MAX_STREAMS_PER_CONNECTION: usize = 32;
const SEED_RECONNECT_BASE_SECONDS: u64 = 1;
const SEED_RECONNECT_MAX_SECONDS: u64 = 300;
const SEED_RECONNECT_JITTER_PERCENT: u64 = 25;
const MAX_OPERATOR_BAN_SECONDS: u64 = 30 * 24 * 60 * 60;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct P2pHandshake {
    pub network_id: String,
    pub chain_magic_hex: String,
    pub p2p_port: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerHello {
    pub protocol_version: u32,
    pub network_id: String,
    pub chain_magic_hex: String,
    pub genesis_hash: String,
    pub best_height: u64,
    pub best_hash: String,
    pub cumulative_work: String,
    pub validating: bool,
    pub mining: bool,
    #[serde(default)]
    pub hashrate_hs: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BlockLocatorEntry {
    height: u64,
    hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct HeaderSummary {
    height: u64,
    hash: String,
    previous_hash: String,
    difficulty_leading_zero_bits: u8,
}

#[derive(Clone, Debug)]
struct PendingBranch {
    remote: PeerHello,
    ancestor_height: u64,
    direct_extension: bool,
    next_height: u64,
    /// Header-first verification buffer (A-H05) before full block download.
    headers_verified: bool,
    headers: Vec<HeaderSummary>,
    blocks: Vec<Block>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectedPeer {
    pub peer_id: String,
    pub address: Option<String>,
    pub handshake_validated: bool,
    pub best_height: Option<u64>,
    pub best_hash: Option<String>,
    #[serde(default)]
    pub cumulative_work: Option<String>,
    pub validating: bool,
    pub mining: bool,
    #[serde(default)]
    pub hashrate_hs: u64,
    pub connected_at_unix_seconds: u64,
    #[serde(default)]
    pub outbound: bool,
    pub last_error: Option<String>,
    #[serde(default)]
    pub reputation_score: i32,
    #[serde(default)]
    pub banned: bool,
    #[serde(default)]
    pub observed_uptime_seconds: u64,
    #[serde(default)]
    pub successful_connections: u64,
    #[serde(default)]
    pub failed_connections: u64,
    #[serde(default)]
    pub validated_handshakes: u64,
    #[serde(default)]
    pub good_events: u64,
    #[serde(default)]
    pub bad_events: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkMinerPresence {
    pub peer_id: String,
    pub hashrate_hs: u64,
    pub template_height: u64,
    pub updated_at_unix_seconds: u64,
    pub local: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct P2pStatus {
    pub mode: String,
    pub protocol_version: u32,
    pub network_id: String,
    pub chain_magic_hex: String,
    #[serde(default)]
    pub local_cumulative_work: String,
    pub local_peer_id: String,
    pub listen_addresses: Vec<String>,
    pub configured_seed_count: usize,
    pub connected_peer_count: usize,
    pub validated_peer_count: usize,
    pub mining_peer_count: usize,
    #[serde(default)]
    pub observed_network_hashrate_hs: u64,
    #[serde(default)]
    pub miners: Vec<NetworkMinerPresence>,
    pub validating_peer_count: usize,
    pub syncing: bool,
    /// Best remote tip height we are (or were) catching up to (durable sync resume).
    #[serde(default)]
    pub sync_target_height: Option<u64>,
    #[serde(default)]
    pub sync_target_hash: Option<String>,
    #[serde(default)]
    pub sync_target_work: Option<String>,
    /// Next height to request when resuming an incomplete sync toward `sync_target_*`.
    #[serde(default)]
    pub sync_resume_height: Option<u64>,
    pub peers: Vec<ConnectedPeer>,
    pub last_error: Option<String>,
    pub updated_at_unix_seconds: u64,
    #[serde(default)]
    pub banned_peer_count: usize,
    #[serde(default)]
    pub reputation_enabled: bool,
    #[serde(default)]
    pub discovered_peer_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum SyncRequest {
    Hello(PeerHello),
    FindAncestor {
        hello: PeerHello,
        locator: Vec<BlockLocatorEntry>,
    },
    /// Header-first: lightweight chain proof before block bodies (v3).
    Headers {
        hello: PeerHello,
        start_height: u64,
        max_headers: usize,
    },
    Blocks {
        hello: PeerHello,
        start_height: u64,
        max_blocks: usize,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum SyncResponse {
    HelloAccepted(PeerHello),
    CommonAncestor {
        hello: PeerHello,
        height: u64,
    },
    Headers {
        hello: PeerHello,
        headers: Vec<HeaderSummary>,
    },
    Blocks {
        hello: PeerHello,
        blocks: Vec<Block>,
    },
    Rejected {
        reason: String,
    },
}

#[derive(NetworkBehaviour)]
#[behaviour(prelude = "libp2p::swarm::derive_prelude")]
struct AlvenqisBehaviour {
    sync: request_response::json::Behaviour<SyncRequest, SyncResponse>,
    gossipsub: gossipsub::Behaviour,
    identify: identify::Behaviour,
    ping: ping::Behaviour,
    limits: connection_limits::Behaviour,
}

#[derive(Clone, Debug)]
struct SeedDialState {
    configured: String,
    address: Multiaddr,
    attempts: u32,
    next_attempt: Instant,
}

#[derive(Deserialize)]
struct LocalMinerPresence {
    status: String,
    network_id: String,
    #[serde(default)]
    height: u64,
    #[serde(default)]
    hashrate_hs: f64,
    updated_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MiningPresenceAnnouncement {
    protocol_version: u32,
    network_id: String,
    peer_id: String,
    hashrate_hs: u64,
    template_height: u64,
    updated_at_unix_seconds: u64,
}

pub fn local_p2p_handshake(config: &NetworkConfig) -> P2pHandshake {
    P2pHandshake {
        network_id: config.network_id.clone(),
        chain_magic_hex: config.chain_magic_hex.clone(),
        p2p_port: config.p2p_listen_port(),
    }
}

pub fn validate_p2p_handshake(config: &NetworkConfig, remote: &P2pHandshake) -> NodeResult<()> {
    if remote.network_id != config.network_id {
        return Err(NodeError::NetworkMismatch {
            expected: config.network_id.clone(),
            actual: remote.network_id.clone(),
        });
    }
    if remote.chain_magic_hex != config.chain_magic_hex {
        return Err(NodeError::ChainMagicMismatch {
            expected: config.chain_magic_hex.clone(),
            actual: remote.chain_magic_hex.clone(),
        });
    }
    Ok(())
}

pub fn load_p2p_status(runtime_dir: &Path, config: &NetworkConfig) -> NodeResult<P2pStatus> {
    let path = runtime_dir.join(P2P_STATUS_FILE_NAME);
    if !path.exists() {
        return Ok(P2pStatus {
            mode: format!("{} / P2P offline", config.status_label),
            protocol_version: P2P_PROTOCOL_VERSION,
            network_id: config.network_id.clone(),
            chain_magic_hex: config.chain_magic_hex.clone(),
            local_cumulative_work: String::new(),
            local_peer_id: String::new(),
            listen_addresses: Vec::new(),
            configured_seed_count: config.seed_nodes.len(),
            connected_peer_count: 0,
            validated_peer_count: 0,
            mining_peer_count: 0,
            observed_network_hashrate_hs: 0,
            miners: Vec::new(),
            validating_peer_count: 0,
            syncing: false,
            sync_target_height: None,
            sync_target_hash: None,
            sync_target_work: None,
            sync_resume_height: None,
            peers: Vec::new(),
            last_error: None,
            updated_at_unix_seconds: unix_seconds(),
            banned_peer_count: 0,
            reputation_enabled: true,
            discovered_peer_count: 0,
        });
    }
    let status: P2pStatus = serde_json::from_str(&fs::read_to_string(path)?)?;
    if status.network_id != config.network_id || status.chain_magic_hex != config.chain_magic_hex {
        return Err(NodeError::ConfigMismatch(
            "persisted P2P status belongs to another network".to_owned(),
        ));
    }
    Ok(status)
}

pub fn run_p2p_service(
    config_path: PathBuf,
    data_dir: PathBuf,
    mempool_dir: PathBuf,
    runtime_dir: PathBuf,
    stop: Arc<AtomicBool>,
) -> NodeResult<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .max_blocking_threads(8)
        .enable_all()
        .build()
        .map_err(|error| NodeError::P2p(error.to_string()))?;
    runtime.block_on(run_p2p_service_async(
        config_path,
        data_dir,
        mempool_dir,
        runtime_dir,
        stop,
    ))
}

async fn run_p2p_service_async(
    config_path: PathBuf,
    data_dir: PathBuf,
    mempool_dir: PathBuf,
    runtime_dir: PathBuf,
    stop: Arc<AtomicBool>,
) -> NodeResult<()> {
    let config = NetworkConfig::load_from_path(&config_path)?;
    fs::create_dir_all(&runtime_dir)?;
    let identity = load_or_create_identity(&runtime_dir)?;
    let local_peer_id = identity.public().to_peer_id();
    let sync_protocol = StreamProtocol::try_from_owned(format!(
        "/alvenqis/{}/sync/{}",
        config.network_id, P2P_PROTOCOL_VERSION
    ))
    .map_err(|error| NodeError::P2p(error.to_string()))?;
    let transaction_topic = gossipsub::IdentTopic::new(format!(
        "alvenqis/{}/transactions/{}",
        config.network_id, P2P_PROTOCOL_VERSION
    ));
    let mining_topic = gossipsub::IdentTopic::new(format!(
        "alvenqis/{}/mining-presence/{}",
        config.network_id, P2P_PROTOCOL_VERSION
    ));
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(1))
        .max_transmit_size(512 * 1024)
        .validation_mode(gossipsub::ValidationMode::Strict)
        .build()
        .map_err(|error| NodeError::P2p(error.to_string()))?;
    let mut gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(identity.clone()),
        gossipsub_config,
    )
    .map_err(|error| NodeError::P2p(error.to_string()))?;
    gossipsub
        .subscribe(&transaction_topic)
        .map_err(|error| NodeError::P2p(error.to_string()))?;
    gossipsub
        .subscribe(&mining_topic)
        .map_err(|error| NodeError::P2p(error.to_string()))?;
    let max_peers = u32::try_from(config.max_peers)
        .map_err(|_| NodeError::P2p("max_peers exceeds libp2p limits".to_owned()))?;
    let max_inbound = if max_peers > 1 {
        max_peers.saturating_sub(1)
    } else {
        1
    };
    let max_outbound = max_peers.min(MAX_OUTBOUND_PEERS as u32);
    let connection_limits = connection_limits::ConnectionLimits::default()
        .with_max_pending_incoming(Some(MAX_PENDING_INBOUND_CONNECTIONS.min(max_peers)))
        .with_max_pending_outgoing(Some(MAX_PENDING_OUTBOUND_CONNECTIONS.min(max_peers)))
        .with_max_established_incoming(Some(max_inbound))
        .with_max_established_outgoing(Some(max_outbound))
        .with_max_established_per_peer(Some(MAX_CONNECTIONS_PER_PEER))
        .with_max_established(Some(max_peers));
    let mut yamux_config = yamux::Config::default();
    yamux_config.set_max_num_streams(MAX_STREAMS_PER_CONNECTION);
    let mut swarm = SwarmBuilder::with_existing_identity(identity)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default().nodelay(true),
            noise::Config::new,
            move || yamux_config.clone(),
        )
        .map_err(|error| NodeError::P2p(error.to_string()))?
        .with_dns()
        .map_err(|error| NodeError::P2p(error.to_string()))?
        .with_behaviour(move |key| {
            let codec =
                request_response::json::codec::Codec::<SyncRequest, SyncResponse>::default()
                    .set_request_size_maximum(256 * 1024)
                    .set_response_size_maximum(16 * 1024 * 1024);
            AlvenqisBehaviour {
                sync: request_response::Behaviour::with_codec(
                    codec,
                    [(sync_protocol, ProtocolSupport::Full)],
                    request_response::Config::default()
                        .with_request_timeout(Duration::from_secs(45)),
                ),
                gossipsub,
                identify: identify::Behaviour::new(identify::Config::new(
                    format!("alvenqis/{P2P_PROTOCOL_VERSION}"),
                    key.public(),
                )),
                ping: ping::Behaviour::new(ping::Config::new()),
                limits: connection_limits::Behaviour::new(connection_limits),
            }
        })
        .map_err(|error| NodeError::P2p(error.to_string()))?
        .with_swarm_config(|swarm_config| {
            swarm_config
                .with_idle_connection_timeout(Duration::from_secs(90))
                .with_max_negotiating_inbound_streams(MAX_NEGOTIATING_INBOUND_STREAMS)
                .with_dial_concurrency_factor(
                    NonZeroU8::new(2).expect("dial concurrency factor is non-zero"),
                )
        })
        .build();

    let listen_address = listen_multiaddr(&config)?;
    swarm
        .listen_on(listen_address)
        .map_err(|error| NodeError::P2p(error.to_string()))?;

    let mut status = P2pStatus {
        mode: format!("{} / Encrypted P2P Prototype", config.status_label),
        protocol_version: P2P_PROTOCOL_VERSION,
        network_id: config.network_id.clone(),
        chain_magic_hex: config.chain_magic_hex.clone(),
        local_cumulative_work: String::new(),
        local_peer_id: local_peer_id.to_string(),
        listen_addresses: Vec::new(),
        configured_seed_count: config.seed_nodes.len(),
        connected_peer_count: 0,
        validated_peer_count: 0,
        mining_peer_count: 0,
        observed_network_hashrate_hs: 0,
        miners: Vec::new(),
        validating_peer_count: 0,
        syncing: false,
        sync_target_height: None,
        sync_target_hash: None,
        sync_target_work: None,
        sync_resume_height: None,
        peers: Vec::new(),
        last_error: None,
        updated_at_unix_seconds: unix_seconds(),
        banned_peer_count: 0,
        reputation_enabled: true,
        discovered_peer_count: 0,
    };
    // Restore durable sync target from disk so restarts can continue toward the same tip.
    if let Ok(previous) = load_p2p_status(&runtime_dir, &config) {
        status.sync_target_height = previous.sync_target_height;
        status.sync_target_hash = previous.sync_target_hash;
        status.sync_target_work = previous.sync_target_work;
        status.sync_resume_height = previous.sync_resume_height;
        if status.sync_target_hash.is_some() {
            status.syncing = true;
        }
    }
    let mut peers = BTreeMap::<PeerId, ConnectedPeer>::new();
    let mut reputation = ReputationStore::load(&runtime_dir)?;
    reputation.prune_expired_bans();
    process_admin_requests(&runtime_dir, &mut reputation, None)?;
    let mut pending_branches = BTreeMap::<PeerId, PendingBranch>::new();
    let mut network_miners = BTreeMap::<PeerId, NetworkMinerPresence>::new();
    let mut discovered_peers = BTreeSet::<PeerId>::new();
    let mut published_transactions = BTreeSet::<String>::new();
    let now = Instant::now();
    let mut seed_states = config
        .seed_nodes
        .iter()
        .map(|seed| {
            Ok(SeedDialState {
                configured: seed.clone(),
                address: seed_multiaddr(seed)?,
                attempts: 0,
                next_attempt: now,
            })
        })
        .collect::<NodeResult<Vec<_>>>()?;
    dial_due_seeds(
        &mut swarm,
        &config,
        &local_peer_id,
        &mut seed_states,
        &mut status,
    );
    persist_status(
        &runtime_dir,
        &config,
        &data_dir,
        &local_peer_id,
        &mut status,
        &peers,
        &mut network_miners,
        &reputation,
        discovered_peers.len(),
    )?;

    let mut tick = tokio::time::interval(Duration::from_secs(1));
    let mut tick_count = 0_u64;
    loop {
        tokio::select! {
            _ = tick.tick() => {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                tick_count = tick_count.saturating_add(1);
                process_admin_requests(&runtime_dir, &mut reputation, Some(&mut swarm))?;
                let newly_banned = peers
                    .keys()
                    .filter(|peer_id| reputation.is_banned(&peer_id.to_string()))
                    .copied()
                    .collect::<Vec<_>>();
                for peer_id in newly_banned {
                    reputation.observe_disconnect(&peer_id.to_string());
                    peers.remove(&peer_id);
                    pending_branches.remove(&peer_id);
                }
                dial_due_seeds(
                    &mut swarm,
                    &config,
                    &local_peer_id,
                    &mut seed_states,
                    &mut status,
                );
                if tick_count.is_multiple_of(5) {
                    match local_hello(&config, &data_dir) {
                        Ok(hello) => {
                            // Retry Hello for every idle live connection. A peer with an
                            // in-flight branch must finish that request chain first;
                            // overlapping Hello responses can otherwise replace its staged
                            // branch while header/body responses are still arriving.
                            for peer_id in peers
                                .keys()
                                .filter(|peer_id| {
                                    !pending_branches.contains_key(peer_id)
                                        && !peer_is_active_sync_target(&status, &peers, peer_id)
                                })
                                .copied()
                                .collect::<Vec<_>>()
                            {
                                swarm
                                    .behaviour_mut()
                                    .sync
                                    .send_request(&peer_id, SyncRequest::Hello(hello.clone()));
                            }
                        }
                        Err(error) => status.last_error = Some(error.to_string()),
                    }
                    if !peers.is_empty() {
                        if let Some(local) =
                            local_miner_presence(&config, &data_dir, &local_peer_id)
                        {
                            let announcement = MiningPresenceAnnouncement {
                                protocol_version: P2P_PROTOCOL_VERSION,
                                network_id: config.network_id.clone(),
                                peer_id: local.peer_id,
                                hashrate_hs: local.hashrate_hs,
                                template_height: local.template_height,
                                updated_at_unix_seconds: local.updated_at_unix_seconds,
                            };
                            match serde_json::to_vec(&announcement) {
                                Ok(payload) => {
                                    if let Err(error) = swarm
                                        .behaviour_mut()
                                        .gossipsub
                                        .publish(mining_topic.clone(), payload)
                                    {
                                        status.last_error = Some(format!(
                                            "mining presence gossip failed: {error}"
                                        ));
                                    }
                                }
                                Err(error) => status.last_error = Some(error.to_string()),
                            }
                        }
                    }
                }
                // Redial seeds when we have zero validated peers. A single
                // non-Alvenqis connection must not block bootstrap forever.
                // Empty seed list is a valid solo-bootstrap configuration — do not surface noise.
                let has_validated_peer = peers.values().any(|peer| peer.handshake_validated);
                if config.seed_nodes.is_empty()
                    && !has_validated_peer
                    && tick_count.is_multiple_of(60)
                {
                    // Clear stale protocol errors from random dialers when operating seedless.
                    if status
                        .last_error
                        .as_deref()
                        .is_some_and(|e| e.contains("supports none of the requested protocols"))
                    {
                        status.last_error = Some(
                            "solo mode: no seed_nodes configured; waiting for inbound peers".into(),
                        );
                    }
                }
                // Drop peers that never complete Alvenqis handshake (wrong protocol / stale).
                if tick_count.is_multiple_of(10) {
                    let now = unix_seconds();
                    let stale: Vec<_> = peers
                        .iter()
                        .filter(|(_, peer)| {
                            !peer.handshake_validated
                                && now.saturating_sub(peer.connected_at_unix_seconds) >= 30
                        })
                        .map(|(peer_id, _)| *peer_id)
                        .collect();
                    for peer_id in stale {
                        reputation.penalize(
                            &peer_id.to_string(),
                            10,
                            "handshake timeout",
                            DEFAULT_BAN_SECONDS,
                        );
                        status.last_error = Some(format!(
                            "disconnecting unvalidated peer {peer_id} after handshake timeout (score {})",
                            reputation.score_of(&peer_id.to_string())
                        ));
                        let _ = swarm.disconnect_peer_id(peer_id);
                        reputation.observe_disconnect(&peer_id.to_string());
                        peers.remove(&peer_id);
                        pending_branches.remove(&peer_id);
                    }
                }
                // Persist reputation periodically so bans survive process restarts.
                if tick_count.is_multiple_of(10) {
                    reputation.persist(&runtime_dir)?;
                }
                if let Ok(records) = crate::mempool::load_pending_transactions(&mempool_dir) {
                    for record in records {
                        if published_transactions.insert(record.tx_hash.clone()) {
                            match serde_json::to_vec(&record.transaction) {
                                Ok(payload) => {
                                    if let Err(error) = swarm
                                        .behaviour_mut()
                                        .gossipsub
                                        .publish(transaction_topic.clone(), payload)
                                    {
                                        status.last_error = Some(format!(
                                            "transaction gossip failed: {error}"
                                        ));
                                    }
                                }
                                Err(error) => status.last_error = Some(error.to_string()),
                            }
                        }
                    }
                }
                reputation.prune_expired_bans();
                persist_status(
                    &runtime_dir,
                    &config,
                    &data_dir,
                    &local_peer_id,
                    &mut status,
                    &peers,
                    &mut network_miners,
                    &reputation,
                    discovered_peers.len(),
                )?;
            }
            event = swarm.select_next_some() => {
                handle_swarm_event(
                    event,
                    &mut swarm,
                    &config_path,
                    &config,
                    &data_dir,
                    &mempool_dir,
                    &mut status,
                    &mut peers,
                    &mut pending_branches,
                    &transaction_topic.hash(),
                    &mining_topic.hash(),
                    &mut network_miners,
                    &mut reputation,
                    &mut discovered_peers,
                    &mut seed_states,
                )?;
            }
        }
    }
    status.mode = format!("{} / P2P stopped", config.status_label);
    status.syncing = false;
    for peer_id in peers.keys() {
        reputation.observe_disconnect(&peer_id.to_string());
    }
    peers.clear();
    pending_branches.clear();
    network_miners.clear();
    reputation.persist(&runtime_dir)?;
    persist_status(
        &runtime_dir,
        &config,
        &data_dir,
        &local_peer_id,
        &mut status,
        &peers,
        &mut network_miners,
        &reputation,
        discovered_peers.len(),
    )
}

// Swarm handler needs the full runtime graph; a Context struct is backlog (see WORKSPACE_AND_LINTS.md).
#[allow(clippy::too_many_arguments)]
fn handle_swarm_event(
    event: SwarmEvent<AlvenqisBehaviourEvent>,
    swarm: &mut Swarm<AlvenqisBehaviour>,
    config_path: &Path,
    config: &NetworkConfig,
    data_dir: &Path,
    mempool_dir: &Path,
    status: &mut P2pStatus,
    peers: &mut BTreeMap<PeerId, ConnectedPeer>,
    pending_branches: &mut BTreeMap<PeerId, PendingBranch>,
    transaction_topic: &gossipsub::TopicHash,
    mining_topic: &gossipsub::TopicHash,
    network_miners: &mut BTreeMap<PeerId, NetworkMinerPresence>,
    reputation: &mut ReputationStore,
    discovered_peers: &mut BTreeSet<PeerId>,
    seed_states: &mut [SeedDialState],
) -> NodeResult<()> {
    match event {
        SwarmEvent::NewListenAddr { address, .. } => {
            let address = address.to_string();
            if !status.listen_addresses.contains(&address) {
                status.listen_addresses.push(address);
            }
        }
        SwarmEvent::ConnectionEstablished {
            peer_id,
            endpoint,
            num_established,
            ..
        } => {
            if peers.len() >= config.max_peers && !peers.contains_key(&peer_id) {
                let _ = swarm.disconnect_peer_id(peer_id);
                status.last_error = Some(format!(
                    "peer limit {} reached; refused additional peer",
                    config.max_peers
                ));
                return Ok(());
            }
            // Refuse peers currently on the ban list (A-H05).
            if reputation.is_banned(&peer_id.to_string()) {
                status.last_error = Some(format!(
                    "refused banned peer {peer_id} (score {})",
                    reputation.score_of(&peer_id.to_string())
                ));
                let _ = swarm.disconnect_peer_id(peer_id);
                return Ok(());
            }
            let outbound = endpoint.is_dialer();
            let is_new_peer = !peers.contains_key(&peer_id);
            peers.entry(peer_id).or_insert_with(|| ConnectedPeer {
                peer_id: peer_id.to_string(),
                address: Some(endpoint.get_remote_address().to_string()),
                handshake_validated: false,
                best_height: None,
                best_hash: None,
                cumulative_work: None,
                validating: false,
                mining: false,
                hashrate_hs: 0,
                connected_at_unix_seconds: unix_seconds(),
                outbound,
                last_error: None,
                reputation_score: reputation.score_of(&peer_id.to_string()),
                banned: false,
                observed_uptime_seconds: 0,
                successful_connections: 0,
                failed_connections: 0,
                validated_handshakes: 0,
                good_events: 0,
                bad_events: 0,
            });
            if is_new_peer {
                reputation.observe_connection(&peer_id.to_string());
            }
            // Handshake every newly established connection with this peer.
            // Multi-miner / multi-node sync requires Hello on each first link,
            // not only when the swarm-wide peer count happens to be one.
            if num_established.get() == 1 {
                match local_hello(config, data_dir) {
                    Ok(hello) => {
                        swarm
                            .behaviour_mut()
                            .sync
                            .send_request(&peer_id, SyncRequest::Hello(hello));
                    }
                    Err(error) => {
                        status.last_error =
                            Some(format!("could not build Hello for peer {peer_id}: {error}"));
                    }
                }
            }
        }
        SwarmEvent::ConnectionClosed {
            peer_id,
            num_established,
            ..
        } => {
            // libp2p can temporarily maintain multiple links to one PeerId (simultaneous dial or
            // seed redial). Closing one link must not erase the still-live peer or its sync state.
            if num_established == 0 {
                let closed_sync_target = peer_is_active_sync_target(status, peers, &peer_id);
                if let Some(peer) = peers.remove(&peer_id) {
                    reputation.observe_disconnect(&peer_id.to_string());
                    if peer.outbound {
                        reset_seed_backoff(seed_states);
                    }
                }
                pending_branches.remove(&peer_id);
                if closed_sync_target {
                    clear_sync_target(status);
                }
            }
        }
        SwarmEvent::Behaviour(AlvenqisBehaviourEvent::Sync(event)) => match event {
            request_response::Event::Message { peer, message, .. } => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    let was_validated = peers
                        .get(&peer)
                        .is_some_and(|remote| remote.handshake_validated);
                    let response = handle_sync_request(config, data_dir, &peer, request, peers);
                    if !was_validated
                        && peers
                            .get(&peer)
                            .is_some_and(|remote| remote.handshake_validated)
                    {
                        reputation.observe_validation(&peer.to_string());
                        reputation.reward(&peer.to_string(), 2, "handshake validated");
                        status.last_error = None;
                        reset_seed_backoff(seed_states);
                    }
                    if swarm
                        .behaviour_mut()
                        .sync
                        .send_response(channel, response)
                        .is_err()
                    {
                        status.last_error = Some(format!("could not send response to {peer}"));
                    }
                }
                request_response::Message::Response { response, .. } => {
                    let was_validated = peers
                        .get(&peer)
                        .is_some_and(|remote| remote.handshake_validated);
                    if let Err(error) = handle_sync_response(
                        config_path,
                        config,
                        data_dir,
                        mempool_dir,
                        &peer,
                        response,
                        swarm,
                        peers,
                        pending_branches,
                        status,
                        reputation,
                    ) {
                        let message = error.to_string();
                        // Severe faults (forged PoW / header mismatch) ban immediately.
                        reputation.apply_protocol_fault(&peer.to_string(), &message);
                        if let Some(remote) = peers.get_mut(&peer) {
                            remote.last_error = Some(message.clone());
                            remote.reputation_score = reputation.score_of(&peer.to_string());
                            remote.banned = reputation.is_banned(&peer.to_string());
                        }
                        status.last_error =
                            Some(format!("rejected data from peer {peer}: {message}"));
                        let _ = swarm.disconnect_peer_id(peer);
                        if reputation.is_banned(&peer.to_string()) {
                            pending_branches.remove(&peer);
                            reputation.observe_disconnect(&peer.to_string());
                            peers.remove(&peer);
                        }
                    } else if !was_validated
                        && peers
                            .get(&peer)
                            .is_some_and(|remote| remote.handshake_validated)
                    {
                        reputation.observe_validation(&peer.to_string());
                        reputation.reward(&peer.to_string(), 2, "handshake validated");
                        status.last_error = None;
                        reset_seed_backoff(seed_states);
                    }
                }
            },
            request_response::Event::OutboundFailure { peer, error, .. } => {
                let message = error.to_string();
                let failed_sync_target = peer_is_active_sync_target(status, peers, &peer);
                // Release any staged request chain so the periodic Hello retry can
                // restart ancestor discovery after a timeout or connection reset.
                pending_branches.remove(&peer);
                if failed_sync_target {
                    clear_sync_target(status);
                }
                if let Some(remote) = peers.get_mut(&peer) {
                    remote.last_error = Some(message.clone());
                }
                status.last_error = Some(message.clone());
                // Foreign libp2p nodes without /alvenqis/.../sync never validate.
                // Drop them so seeds can redial and real miners can connect.
                if message.contains("supports none of the requested protocols")
                    || message.contains("UnsupportedProtocols")
                    || message.contains("Unsupported protocol")
                {
                    // Repeated non-Alvenqis dialers get soft penalties; after threshold they are banned.
                    reputation.penalize(
                        &peer.to_string(),
                        15,
                        "unsupported sync protocol",
                        DEFAULT_BAN_SECONDS / 2,
                    );
                    let _ = swarm.disconnect_peer_id(peer);
                    reputation.observe_disconnect(&peer.to_string());
                    peers.remove(&peer);
                    pending_branches.remove(&peer);
                }
            }
            request_response::Event::InboundFailure { peer, error, .. } => {
                let message = error.to_string();
                if let Some(remote) = peers.get_mut(&peer) {
                    remote.last_error = Some(message.clone());
                }
                status.last_error = Some(message);
            }
            request_response::Event::ResponseSent { .. } => {}
        },
        SwarmEvent::Behaviour(AlvenqisBehaviourEvent::Gossipsub(gossipsub::Event::Message {
            propagation_source,
            message,
            ..
        })) => {
            if &message.topic == transaction_topic {
                match serde_json::from_slice::<Transaction>(&message.data) {
                    Ok(transaction) => {
                        if let Err(error) = submit_transaction(
                            data_dir,
                            mempool_dir,
                            config.max_mempool_transactions,
                            &transaction,
                        ) {
                            status.last_error = Some(format!(
                                "rejected transaction from peer {propagation_source}: {error}"
                            ));
                        }
                    }
                    Err(error) => {
                        status.last_error = Some(format!(
                            "malformed transaction from peer {propagation_source}: {error}"
                        ));
                    }
                }
            } else if &message.topic == mining_topic {
                match serde_json::from_slice::<MiningPresenceAnnouncement>(&message.data) {
                    Ok(announcement) => {
                        if let Err(error) = accept_mining_presence(
                            config,
                            message.source,
                            announcement,
                            network_miners,
                        ) {
                            status.last_error = Some(format!(
                                "rejected mining presence from {propagation_source}: {error}"
                            ));
                        }
                    }
                    Err(error) => {
                        status.last_error = Some(format!(
                            "malformed mining presence from {propagation_source}: {error}"
                        ));
                    }
                }
            }
        }
        SwarmEvent::Behaviour(AlvenqisBehaviourEvent::Identify(identify::Event::Received {
            peer_id,
            info,
            ..
        })) => {
            if info.protocol_version == format!("alvenqis/{P2P_PROTOCOL_VERSION}")
                && !reputation.is_banned(&peer_id.to_string())
            {
                discovered_peers.insert(peer_id);
            }
        }
        SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
            if let Some(peer_id) = peer_id {
                reputation.observe_failed_connection(&peer_id.to_string());
            }
            status.last_error = Some(error.to_string());
        }
        SwarmEvent::IncomingConnectionError { error, .. } => {
            let message = error.to_string();
            // Internet scanners and plain TCP health probes cannot negotiate Noise/Yamux. They
            // must not overwrite the node's operational status with a false P2P outage.
            if !is_expected_inbound_transport_noise(&message) {
                status.last_error = Some(message);
            }
        }
        _ => {}
    }
    Ok(())
}

fn is_expected_inbound_transport_noise(message: &str) -> bool {
    message
        .to_ascii_lowercase()
        .contains("failed to negotiate transport protocol")
}

fn handle_sync_request(
    config: &NetworkConfig,
    data_dir: &Path,
    peer_id: &PeerId,
    request: SyncRequest,
    peers: &mut BTreeMap<PeerId, ConnectedPeer>,
) -> SyncResponse {
    let remote_hello = match &request {
        SyncRequest::Hello(hello) => hello,
        SyncRequest::FindAncestor { hello, .. } => hello,
        SyncRequest::Headers { hello, .. } => hello,
        SyncRequest::Blocks { hello, .. } => hello,
    };
    let expected_genesis = match local_genesis_hash(data_dir) {
        Ok(hash) => hash,
        Err(error) => {
            return SyncResponse::Rejected {
                reason: error.to_string(),
            }
        }
    };
    if let Err(error) = validate_peer_hello(config, &expected_genesis, remote_hello) {
        return SyncResponse::Rejected {
            reason: error.to_string(),
        };
    }
    update_validated_peer(peers, peer_id, remote_hello);
    let local = match local_hello(config, data_dir) {
        Ok(hello) => hello,
        Err(error) => {
            return SyncResponse::Rejected {
                reason: error.to_string(),
            }
        }
    };
    match request {
        SyncRequest::Hello(_) => SyncResponse::HelloAccepted(local),
        SyncRequest::FindAncestor { locator, .. } => {
            let local_blocks = match storage::load_blocks(data_dir) {
                Ok(blocks) => blocks,
                Err(error) => {
                    return SyncResponse::Rejected {
                        reason: error.to_string(),
                    }
                }
            };
            let ancestor = locator.into_iter().find(|entry| {
                local_blocks
                    .get(entry.height as usize)
                    .and_then(|block| block.hash().ok())
                    .is_some_and(|hash| hash_to_hex(&hash) == entry.hash)
            });
            match ancestor {
                Some(entry) => SyncResponse::CommonAncestor {
                    hello: local,
                    height: entry.height,
                },
                None => SyncResponse::Rejected {
                    reason: "no common ancestor exists for the advertised genesis".to_owned(),
                },
            }
        }
        SyncRequest::Headers {
            start_height,
            max_headers,
            ..
        } => {
            let headers = storage::load_blocks(data_dir)
                .ok()
                .map(|blocks| {
                    blocks
                        .into_iter()
                        .filter(|block| block.header.height >= start_height)
                        .take(max_headers.min(MAX_SYNC_HEADERS))
                        .filter_map(|block| {
                            let hash = block.hash().ok()?;
                            Some(HeaderSummary {
                                height: block.header.height,
                                hash: hash_to_hex(&hash),
                                previous_hash: hash_to_hex(&block.header.previous_hash),
                                difficulty_leading_zero_bits: block
                                    .header
                                    .difficulty_leading_zero_bits,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            SyncResponse::Headers {
                hello: local,
                headers,
            }
        }
        SyncRequest::Blocks {
            start_height,
            max_blocks,
            ..
        } => {
            let blocks = storage::load_blocks(data_dir)
                .map(|blocks| {
                    blocks
                        .into_iter()
                        .filter(|block| block.header.height >= start_height)
                        .take(max_blocks.min(MAX_SYNC_BLOCKS))
                        .collect()
                })
                .unwrap_or_default();
            SyncResponse::Blocks {
                hello: local,
                blocks,
            }
        }
    }
}

// Same as handle_swarm_event — wide runtime graph; Context struct is backlog.
#[allow(clippy::too_many_arguments)]
fn handle_sync_response(
    config_path: &Path,
    config: &NetworkConfig,
    data_dir: &Path,
    mempool_dir: &Path,
    peer_id: &PeerId,
    response: SyncResponse,
    swarm: &mut Swarm<AlvenqisBehaviour>,
    peers: &mut BTreeMap<PeerId, ConnectedPeer>,
    pending_branches: &mut BTreeMap<PeerId, PendingBranch>,
    status: &mut P2pStatus,
    reputation: &mut ReputationStore,
) -> NodeResult<()> {
    match response {
        SyncResponse::Rejected { reason } => {
            reputation.penalize(&peer_id.to_string(), 15, &reason, DEFAULT_BAN_SECONDS);
            if let Some(peer) = peers.get_mut(peer_id) {
                peer.last_error = Some(reason.clone());
                peer.reputation_score = reputation.score_of(&peer_id.to_string());
                peer.banned = reputation.is_banned(&peer_id.to_string());
            }
            status.last_error = Some(format!("peer {peer_id} rejected handshake: {reason}"));
            let _ = swarm.disconnect_peer_id(*peer_id);
        }
        SyncResponse::HelloAccepted(hello) => {
            validate_peer_hello(config, &local_genesis_hash(data_dir)?, &hello)?;
            update_validated_peer(peers, peer_id, &hello);
            let local = local_hello(config, data_dir)?;
            if remote_has_more_work(&hello, &local)? && hello.best_hash != local.best_hash {
                if !pending_branches.contains_key(peer_id)
                    && pending_branches.len() >= MAX_CONCURRENT_SYNC_BRANCHES
                {
                    status.last_error = Some(format!(
                        "sync branch limit {MAX_CONCURRENT_SYNC_BRANCHES} reached; deferred peer {peer_id}"
                    ));
                    return Ok(());
                }
                status.syncing = true;
                remember_sync_target(status, &hello);
                swarm.behaviour_mut().sync.send_request(
                    peer_id,
                    SyncRequest::FindAncestor {
                        hello: local.clone(),
                        locator: block_locator(data_dir)?,
                    },
                );
            } else if status.sync_target_hash.as_deref() == Some(local.best_hash.as_str()) {
                // Reached the durable target tip.
                clear_sync_target(status);
            }
        }
        SyncResponse::CommonAncestor { hello, height } => {
            validate_peer_hello(config, &local_genesis_hash(data_dir)?, &hello)?;
            update_validated_peer(peers, peer_id, &hello);
            let local = local_hello(config, data_dir)?;
            if !remote_has_more_work(&hello, &local)? {
                pending_branches.remove(peer_id);
                clear_sync_target(status);
                return Ok(());
            }
            remember_sync_target(status, &hello);
            let local_blocks = storage::load_blocks(data_dir)?;
            if local_blocks.get(height as usize).is_none() {
                return Err(NodeError::P2p(format!(
                    "peer selected unavailable ancestor height {height}"
                )));
            }
            let mut next_height = height.saturating_add(1);
            // Resume past already-applied heights when the durable target matches.
            if status.sync_target_hash.as_deref() == Some(hello.best_hash.as_str()) {
                if let Some(resume) = status.sync_resume_height {
                    if resume > next_height && resume <= hello.best_height {
                        next_height = resume;
                    }
                }
            }
            pending_branches.insert(
                *peer_id,
                PendingBranch {
                    remote: hello,
                    ancestor_height: height,
                    direct_extension: height == local.best_height,
                    next_height,
                    headers_verified: false,
                    headers: Vec::new(),
                    blocks: Vec::new(),
                },
            );
            // Header-first: verify lightweight chain before downloading bodies.
            request_branch_headers(config, data_dir, peer_id, swarm, pending_branches)?;
        }
        SyncResponse::Headers { hello, headers } => {
            validate_peer_hello(config, &local_genesis_hash(data_dir)?, &hello)?;
            update_validated_peer(peers, peer_id, &hello);
            let branch = pending_branches.get_mut(peer_id).ok_or_else(|| {
                NodeError::P2p("received unsolicited headers response".to_owned())
            })?;
            if hello.best_hash != branch.remote.best_hash {
                return Err(NodeError::P2p(
                    "peer changed tip during header sync".to_owned(),
                ));
            }
            validate_header_chain(data_dir, branch, &headers)?;
            let incoming = headers.len();
            if branch.headers.len().saturating_add(incoming) > MAX_STAGED_HEADERS {
                return Err(NodeError::P2p(format!(
                    "staged headers exceed limit of {MAX_STAGED_HEADERS} (DoS protection)"
                )));
            }
            let height_span = branch
                .remote
                .best_height
                .saturating_sub(branch.ancestor_height);
            if height_span > MAX_HEADER_HEIGHT_DELTA {
                return Err(NodeError::P2p(format!(
                    "header height delta {height_span} exceeds limit of {MAX_HEADER_HEIGHT_DELTA}"
                )));
            }
            branch.headers.extend(headers);
            let last_height = branch.headers.last().map(|h| h.height);
            if last_height.is_some_and(|h| h < branch.remote.best_height) {
                branch.next_height = last_height.map_or(branch.next_height, |h| h + 1);
                request_branch_headers(config, data_dir, peer_id, swarm, pending_branches)?;
                return Ok(());
            }
            if branch.headers.last().map(|h| h.hash.as_str())
                != Some(branch.remote.best_hash.as_str())
            {
                return Err(NodeError::P2p(
                    "header chain does not reach the peer committed tip".to_owned(),
                ));
            }
            branch.headers_verified = true;
            // Prefer durable resume height once headers for the target tip are complete.
            let mut next = branch.ancestor_height.saturating_add(1);
            if status.sync_target_hash.as_deref() == Some(branch.remote.best_hash.as_str()) {
                if let Some(resume) = status.sync_resume_height {
                    if resume > next && resume <= branch.remote.best_height {
                        next = resume;
                    }
                }
            }
            branch.next_height = next;
            reputation.reward(&peer_id.to_string(), 5, "headers verified");
            request_branch_chunk(config, data_dir, peer_id, swarm, pending_branches)?;
        }
        SyncResponse::Blocks { hello, blocks } => {
            validate_peer_hello(config, &local_genesis_hash(data_dir)?, &hello)?;
            update_validated_peer(peers, peer_id, &hello);
            let branch = pending_branches.get_mut(peer_id).ok_or_else(|| {
                NodeError::P2p("received an unsolicited branch block response".to_owned())
            })?;
            if !branch.headers_verified {
                return Err(NodeError::P2p(
                    "blocks received before header-first verification".to_owned(),
                ));
            }
            if hello.best_hash != branch.remote.best_hash
                || hello.cumulative_work != branch.remote.cumulative_work
            {
                return Err(NodeError::P2p(
                    "peer changed its advertised branch during synchronization".to_owned(),
                ));
            }
            if blocks.is_empty() {
                return Err(NodeError::P2p(
                    "peer returned an empty incomplete branch".to_owned(),
                ));
            }
            if blocks
                .last()
                .is_some_and(|block| block.header.height > branch.remote.best_height)
            {
                return Err(NodeError::P2p(
                    "peer sent blocks beyond its committed tip".to_owned(),
                ));
            }
            // Bodies must match previously verified headers.
            for block in &blocks {
                let hash = hash_to_hex(&block.hash()?);
                if !branch
                    .headers
                    .iter()
                    .any(|h| h.hash == hash && h.height == block.header.height)
                {
                    return Err(NodeError::P2p(
                        "block body does not match verified header set".to_owned(),
                    ));
                }
            }
            if branch.direct_extension {
                branch.next_height = blocks
                    .last()
                    .map_or(branch.next_height, |block| block.header.height + 1);
                for block in blocks {
                    submit_mined_block(config_path, data_dir, mempool_dir, &block)?;
                }
                status.sync_resume_height = Some(branch.next_height);
                remember_sync_target(status, &branch.remote);
                let local = local_hello(config, data_dir)?;
                if local.best_height < branch.remote.best_height {
                    request_branch_chunk(config, data_dir, peer_id, swarm, pending_branches)?;
                } else if local.best_hash == branch.remote.best_hash
                    && local.cumulative_work == branch.remote.cumulative_work
                {
                    reputation.reward(&peer_id.to_string(), 10, "direct extension applied");
                    pending_branches.remove(peer_id);
                    clear_sync_target(status);
                } else {
                    return Err(NodeError::P2p(
                        "direct extension does not match the peer work commitment".to_owned(),
                    ));
                }
                return Ok(());
            }
            if branch.blocks.len().saturating_add(blocks.len()) > MAX_STAGED_REORG_BLOCKS {
                return Err(NodeError::P2p(format!(
                    "fork exceeds the staged reorg limit of {MAX_STAGED_REORG_BLOCKS} blocks"
                )));
            }
            branch.next_height = blocks
                .last()
                .map_or(branch.next_height, |block| block.header.height + 1);
            branch.blocks.extend(blocks);
            let candidate = staged_candidate(data_dir, branch)?;
            Chain::from_blocks(config.network, candidate.iter().cloned())?;
            let received_tip = candidate
                .last()
                .ok_or_else(|| NodeError::P2p("validated reorg candidate is empty".to_owned()))?;
            if received_tip.header.height < branch.remote.best_height {
                request_branch_chunk(config, data_dir, peer_id, swarm, pending_branches)?;
                return Ok(());
            }
            if received_tip.header.height != branch.remote.best_height
                || hash_to_hex(&received_tip.hash()?) != branch.remote.best_hash
                || cumulative_work(&candidate)?.to_string() != branch.remote.cumulative_work
            {
                return Err(NodeError::P2p(
                    "downloaded branch does not match the peer work commitment".to_owned(),
                ));
            }
            adopt_candidate_chain(config_path, data_dir, mempool_dir, &candidate)?;
            reputation.reward(&peer_id.to_string(), 15, "reorg branch adopted");
            pending_branches.remove(peer_id);
            clear_sync_target(status);
        }
    }
    Ok(())
}

fn remember_sync_target(status: &mut P2pStatus, remote: &PeerHello) {
    status.syncing = true;
    status.sync_target_height = Some(remote.best_height);
    status.sync_target_hash = Some(remote.best_hash.clone());
    status.sync_target_work = Some(remote.cumulative_work.clone());
}

fn clear_sync_target(status: &mut P2pStatus) {
    status.syncing = false;
    status.sync_target_height = None;
    status.sync_target_hash = None;
    status.sync_target_work = None;
    status.sync_resume_height = None;
}

fn peer_is_active_sync_target(
    status: &P2pStatus,
    peers: &BTreeMap<PeerId, ConnectedPeer>,
    peer_id: &PeerId,
) -> bool {
    status.syncing
        && status.sync_target_hash.as_deref()
            == peers
                .get(peer_id)
                .and_then(|peer| peer.best_hash.as_deref())
}

fn parse_chain_work(value: &str) -> NodeResult<u128> {
    value
        .parse::<u128>()
        .map_err(|_| NodeError::P2p("peer advertised invalid cumulative chain work".to_owned()))
}

fn remote_has_more_work(remote: &PeerHello, local: &PeerHello) -> NodeResult<bool> {
    Ok(parse_chain_work(&remote.cumulative_work)? > parse_chain_work(&local.cumulative_work)?)
}

/// Build an exponential block locator (Bitcoin-style) for ancestor discovery.
/// Caps size for bandwidth and always ends at genesis when the chain is non-empty.
fn block_locator(data_dir: &Path) -> NodeResult<Vec<BlockLocatorEntry>> {
    const MAX_LOCATOR_ENTRIES: usize = 32;
    let blocks = storage::load_blocks(data_dir)?;
    if blocks.is_empty() {
        return Ok(Vec::new());
    }
    let mut locator = Vec::new();
    let mut index = blocks.len().saturating_sub(1);
    let mut step = 1_usize;
    let mut seen_heights = BTreeSet::new();
    loop {
        if locator.len() >= MAX_LOCATOR_ENTRIES.saturating_sub(1) && index != 0 {
            // Reserve the last slot for genesis.
            index = 0;
        }
        let block = &blocks[index];
        if seen_heights.insert(block.header.height) {
            locator.push(BlockLocatorEntry {
                height: block.header.height,
                hash: hash_to_hex(&block.hash()?),
            });
        }
        if index == 0 {
            break;
        }
        index = index.saturating_sub(step);
        // After the first 8 tip steps, double the stride (exponential back-off).
        if locator.len() > 8 {
            step = step.saturating_mul(2).max(1);
        }
    }
    Ok(locator)
}

fn staged_candidate(data_dir: &Path, branch: &PendingBranch) -> NodeResult<Vec<Block>> {
    let mut candidate: Vec<Block> = storage::load_blocks(data_dir)?
        .into_iter()
        .take(branch.ancestor_height as usize + 1)
        .collect();
    candidate.extend(branch.blocks.iter().cloned());
    Ok(candidate)
}

fn request_branch_headers(
    config: &NetworkConfig,
    data_dir: &Path,
    peer_id: &PeerId,
    swarm: &mut Swarm<AlvenqisBehaviour>,
    pending_branches: &BTreeMap<PeerId, PendingBranch>,
) -> NodeResult<()> {
    let branch = pending_branches
        .get(peer_id)
        .ok_or_else(|| NodeError::P2p("missing staged branch state".to_owned()))?;
    swarm.behaviour_mut().sync.send_request(
        peer_id,
        SyncRequest::Headers {
            hello: local_hello(config, data_dir)?,
            start_height: branch.next_height,
            max_headers: MAX_SYNC_HEADERS,
        },
    );
    Ok(())
}

fn request_branch_chunk(
    config: &NetworkConfig,
    data_dir: &Path,
    peer_id: &PeerId,
    swarm: &mut Swarm<AlvenqisBehaviour>,
    pending_branches: &BTreeMap<PeerId, PendingBranch>,
) -> NodeResult<()> {
    let branch = pending_branches
        .get(peer_id)
        .ok_or_else(|| NodeError::P2p("missing staged branch state".to_owned()))?;
    let start_height = branch.next_height;
    swarm.behaviour_mut().sync.send_request(
        peer_id,
        SyncRequest::Blocks {
            hello: local_hello(config, data_dir)?,
            start_height,
            max_blocks: MAX_SYNC_BLOCKS,
        },
    );
    Ok(())
}

/// Verify header linkage against local ancestor tip and prior header batch.
fn validate_header_chain(
    data_dir: &Path,
    branch: &PendingBranch,
    headers: &[HeaderSummary],
) -> NodeResult<()> {
    if headers.is_empty() {
        return Err(NodeError::P2p("peer returned empty headers".to_owned()));
    }
    let local_blocks = storage::load_blocks(data_dir)?;
    let mut prev_hash = if branch.headers.is_empty() {
        let ancestor = local_blocks
            .get(branch.ancestor_height as usize)
            .ok_or_else(|| NodeError::P2p("ancestor missing for header validation".to_owned()))?;
        hash_to_hex(&ancestor.hash()?)
    } else {
        branch
            .headers
            .last()
            .map(|h| h.hash.clone())
            .ok_or_else(|| NodeError::P2p("empty prior headers".to_owned()))?
    };
    let mut expected_height = if branch.headers.is_empty() {
        branch.ancestor_height + 1
    } else {
        branch
            .headers
            .last()
            .map(|h| h.height + 1)
            .unwrap_or(branch.ancestor_height + 1)
    };
    for header in headers {
        if header.height != expected_height {
            return Err(NodeError::P2p(format!(
                "header height gap: expected {expected_height}, got {}",
                header.height
            )));
        }
        if header.previous_hash != prev_hash {
            return Err(NodeError::P2p(format!(
                "header previous_hash mismatch at height {}",
                header.height
            )));
        }
        if header.hash.len() != 64 || !header.hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(NodeError::P2p("header hash is not 32-byte hex".to_owned()));
        }
        prev_hash = header.hash.clone();
        expected_height = expected_height.saturating_add(1);
    }
    Ok(())
}

fn local_hello(config: &NetworkConfig, data_dir: &Path) -> NodeResult<PeerHello> {
    let blocks = storage::load_blocks(data_dir)?;
    let tip = blocks
        .last()
        .ok_or_else(|| NodeError::ChainNotInitialized(storage::chain_file_path(data_dir)))?;
    let miner = read_local_miner_metrics(config, data_dir);
    Ok(PeerHello {
        protocol_version: P2P_PROTOCOL_VERSION,
        network_id: config.network_id.clone(),
        chain_magic_hex: config.chain_magic_hex.clone(),
        genesis_hash: hash_to_hex(&blocks[0].hash()?),
        best_height: tip.header.height,
        best_hash: hash_to_hex(&tip.hash()?),
        cumulative_work: cumulative_work(&blocks)?.to_string(),
        validating: true,
        mining: miner.is_some(),
        hashrate_hs: miner.map_or(0, |metrics| miner_hashrate_hs(&metrics)),
    })
}

fn validate_peer_hello(
    config: &NetworkConfig,
    expected_genesis: &str,
    hello: &PeerHello,
) -> NodeResult<()> {
    if hello.protocol_version != P2P_PROTOCOL_VERSION {
        return Err(NodeError::P2p(format!(
            "unsupported protocol version {}; expected {}",
            hello.protocol_version, P2P_PROTOCOL_VERSION
        )));
    }
    validate_p2p_handshake(
        config,
        &P2pHandshake {
            network_id: hello.network_id.clone(),
            chain_magic_hex: hello.chain_magic_hex.clone(),
            p2p_port: 0,
        },
    )?;
    if hello.genesis_hash != expected_genesis {
        return Err(NodeError::GenesisMismatch {
            expected: expected_genesis.to_owned(),
            actual: hello.genesis_hash.clone(),
        });
    }
    Ok(())
}

fn local_genesis_hash(data_dir: &Path) -> NodeResult<String> {
    let blocks = storage::load_blocks(data_dir)?;
    match blocks.first() {
        Some(block) => Ok(hash_to_hex(&block.hash()?)),
        None => Err(NodeError::ChainNotInitialized(storage::chain_file_path(
            data_dir,
        ))),
    }
}

fn local_miner_presence(
    config: &NetworkConfig,
    data_dir: &Path,
    local_peer_id: &PeerId,
) -> Option<NetworkMinerPresence> {
    read_local_miner_metrics(config, data_dir).map(|metrics| NetworkMinerPresence {
        peer_id: local_peer_id.to_string(),
        hashrate_hs: miner_hashrate_hs(&metrics),
        template_height: metrics.height,
        updated_at_unix_seconds: metrics.updated_at_unix_seconds,
        local: true,
    })
}

fn read_local_miner_metrics(config: &NetworkConfig, data_dir: &Path) -> Option<LocalMinerPresence> {
    let metrics_path = data_dir
        .parent()
        .unwrap_or(data_dir)
        .join("miner")
        .join("metrics.json");
    fs::read(metrics_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<LocalMinerPresence>(&bytes).ok())
        .filter(|metrics| {
            metrics.status == "mining"
                && metrics.network_id == config.network_id
                && metrics.hashrate_hs.is_finite()
                && metrics.hashrate_hs > 0.0
                && unix_seconds().saturating_sub(metrics.updated_at_unix_seconds)
                    <= MINER_PRESENCE_TTL_SECONDS
        })
}

fn miner_hashrate_hs(metrics: &LocalMinerPresence) -> u64 {
    metrics.hashrate_hs.round().clamp(0.0, u64::MAX as f64) as u64
}

fn accept_mining_presence(
    config: &NetworkConfig,
    message_source: Option<PeerId>,
    announcement: MiningPresenceAnnouncement,
    network_miners: &mut BTreeMap<PeerId, NetworkMinerPresence>,
) -> NodeResult<()> {
    if announcement.protocol_version != P2P_PROTOCOL_VERSION {
        return Err(NodeError::P2p(format!(
            "unsupported mining presence protocol version {}",
            announcement.protocol_version
        )));
    }
    if announcement.network_id != config.network_id {
        return Err(NodeError::NetworkMismatch {
            expected: config.network_id.clone(),
            actual: announcement.network_id,
        });
    }
    let announced_peer = announcement
        .peer_id
        .parse::<PeerId>()
        .map_err(|error| NodeError::P2p(format!("invalid mining peer ID: {error}")))?;
    if message_source != Some(announced_peer) {
        return Err(NodeError::P2p(
            "mining presence signer does not match its announced peer ID".to_owned(),
        ));
    }
    let now = unix_seconds();
    if announcement.updated_at_unix_seconds > now.saturating_add(10)
        || now.saturating_sub(announcement.updated_at_unix_seconds) > MINER_PRESENCE_TTL_SECONDS
    {
        return Err(NodeError::P2p(
            "mining presence timestamp is outside the accepted freshness window".to_owned(),
        ));
    }
    if announcement.hashrate_hs == 0 {
        return Err(NodeError::P2p(
            "mining presence hashrate must be greater than zero".to_owned(),
        ));
    }
    network_miners.insert(
        announced_peer,
        NetworkMinerPresence {
            peer_id: announcement.peer_id,
            hashrate_hs: announcement.hashrate_hs,
            template_height: announcement.template_height,
            updated_at_unix_seconds: announcement.updated_at_unix_seconds,
            local: false,
        },
    );
    Ok(())
}

fn update_validated_peer(
    peers: &mut BTreeMap<PeerId, ConnectedPeer>,
    peer_id: &PeerId,
    hello: &PeerHello,
) {
    if let Some(peer) = peers.get_mut(peer_id) {
        peer.handshake_validated = true;
        peer.best_height = Some(hello.best_height);
        peer.best_hash = Some(hello.best_hash.clone());
        peer.cumulative_work = Some(hello.cumulative_work.clone());
        peer.validating = hello.validating;
        peer.mining = hello.mining;
        peer.hashrate_hs = hello.hashrate_hs;
        peer.last_error = None;
    }
}

#[allow(clippy::too_many_arguments)]
fn persist_status(
    runtime_dir: &Path,
    config: &NetworkConfig,
    data_dir: &Path,
    local_peer_id: &PeerId,
    status: &mut P2pStatus,
    peers: &BTreeMap<PeerId, ConnectedPeer>,
    network_miners: &mut BTreeMap<PeerId, NetworkMinerPresence>,
    reputation: &ReputationStore,
    discovered_peer_count: usize,
) -> NodeResult<()> {
    status.local_cumulative_work = local_hello(config, data_dir)
        .map(|hello| hello.cumulative_work)
        .unwrap_or_default();
    let mut peer_list: Vec<ConnectedPeer> = peers.values().cloned().collect();
    for peer in &mut peer_list {
        peer.reputation_score = reputation.score_of(&peer.peer_id);
        peer.banned = reputation.is_banned(&peer.peer_id);
        peer.observed_uptime_seconds = reputation.observed_uptime_seconds(&peer.peer_id);
        if let Some(record) = reputation.record(&peer.peer_id) {
            peer.successful_connections = record.successful_connections;
            peer.failed_connections = record.failed_connections;
            peer.validated_handshakes = record.validated_handshakes;
            peer.good_events = record.good_events;
            peer.bad_events = record.bad_events;
        }
    }
    status.peers = peer_list;
    status.connected_peer_count = status.peers.len();
    status.validated_peer_count = status
        .peers
        .iter()
        .filter(|peer| peer.handshake_validated)
        .count();
    let now = unix_seconds();
    network_miners.retain(|_, miner| {
        now.saturating_sub(miner.updated_at_unix_seconds) <= MINER_PRESENCE_TTL_SECONDS
    });
    let mut miners = network_miners.values().cloned().collect::<Vec<_>>();
    if let Some(local) = local_miner_presence(config, data_dir, local_peer_id) {
        miners.push(local);
    }
    miners.sort_by(|left, right| left.peer_id.cmp(&right.peer_id));
    status.mining_peer_count = miners.len();
    status.observed_network_hashrate_hs = miners.iter().fold(0_u64, |total, miner| {
        total.saturating_add(miner.hashrate_hs)
    });
    status.miners = miners;
    status.validating_peer_count = status
        .peers
        .iter()
        .filter(|peer| peer.handshake_validated && peer.validating)
        .count();
    status.banned_peer_count = reputation.active_ban_count();
    status.reputation_enabled = true;
    status.discovered_peer_count = discovered_peer_count;
    status.updated_at_unix_seconds = unix_seconds();
    let mut file = AtomicWriteFile::open(runtime_dir.join(P2P_STATUS_FILE_NAME))?;
    serde_json::to_writer_pretty(&mut file, status)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    file.commit()?;
    Ok(())
}

fn load_or_create_identity(runtime_dir: &Path) -> NodeResult<Keypair> {
    let path = runtime_dir.join(P2P_IDENTITY_FILE_NAME);
    if path.exists() {
        return Keypair::from_protobuf_encoding(&fs::read(path)?)
            .map_err(|error| NodeError::P2p(format!("invalid persisted P2P identity: {error}")));
    }
    let identity = Keypair::generate_ed25519();
    let encoded = identity
        .to_protobuf_encoding()
        .map_err(|error| NodeError::P2p(error.to_string()))?;
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        file.write_all(&encoded)?;
    }
    #[cfg(not(unix))]
    {
        fs::write(&path, encoded)?;
    }
    Ok(identity)
}

pub fn queue_peer_ban(
    runtime_dir: &Path,
    peer_id: &str,
    reason: &str,
    duration_seconds: u64,
) -> NodeResult<String> {
    peer_id
        .parse::<PeerId>()
        .map_err(|error| NodeError::Input(format!("invalid peer ID: {error}")))?;
    let reason = reason.trim();
    if reason.is_empty() || reason.len() > 256 || reason.chars().any(char::is_control) {
        return Err(NodeError::Input(
            "ban reason must contain 1 to 256 printable characters".to_owned(),
        ));
    }
    if duration_seconds > MAX_OPERATOR_BAN_SECONDS {
        return Err(NodeError::Input(format!(
            "ban duration cannot exceed {MAX_OPERATOR_BAN_SECONDS} seconds; use 0 for permanent"
        )));
    }
    let request = enqueue_admin_request(
        runtime_dir,
        peer_id.to_owned(),
        PeerAdminAction::Ban {
            reason: reason.to_owned(),
            duration_seconds,
        },
    )?;
    Ok(request.request_id)
}

pub fn queue_peer_unban(runtime_dir: &Path, peer_id: &str) -> NodeResult<String> {
    peer_id
        .parse::<PeerId>()
        .map_err(|error| NodeError::Input(format!("invalid peer ID: {error}")))?;
    let request = enqueue_admin_request(runtime_dir, peer_id.to_owned(), PeerAdminAction::Unban)?;
    Ok(request.request_id)
}

fn process_admin_requests(
    runtime_dir: &Path,
    reputation: &mut ReputationStore,
    mut swarm: Option<&mut Swarm<AlvenqisBehaviour>>,
) -> NodeResult<()> {
    let requests = load_admin_requests(runtime_dir)?;
    if requests.is_empty() {
        return Ok(());
    }
    let mut request_ids = Vec::with_capacity(requests.len());
    for request in &requests {
        reputation.apply_admin_request(request);
        if reputation.is_banned(&request.peer_id) {
            if let (Some(swarm), Ok(peer_id)) =
                (swarm.as_deref_mut(), request.peer_id.parse::<PeerId>())
            {
                let _ = swarm.disconnect_peer_id(peer_id);
            }
        }
        request_ids.push(request.request_id.clone());
    }
    reputation.persist(runtime_dir)?;
    acknowledge_admin_requests(runtime_dir, &request_ids)
}

fn dial_due_seeds(
    swarm: &mut Swarm<AlvenqisBehaviour>,
    config: &NetworkConfig,
    local_peer_id: &PeerId,
    seed_states: &mut [SeedDialState],
    status: &mut P2pStatus,
) {
    if seed_states.is_empty() {
        return;
    }
    let network_info = swarm.network_info();
    let counters = network_info.connection_counters();
    let target = seed_states
        .len()
        .min(MAX_OUTBOUND_PEERS)
        .min(config.max_peers);
    let active =
        counters.num_established_outgoing() as usize + counters.num_pending_outgoing() as usize;
    let mut available = target.saturating_sub(active);
    if available == 0 {
        return;
    }
    let now = Instant::now();
    for seed in seed_states
        .iter_mut()
        .filter(|seed| seed.next_attempt <= now)
    {
        if available == 0 {
            break;
        }
        let attempt = seed.attempts;
        let dial_options = DialOpts::unknown_peer_id()
            .address(seed.address.clone())
            .allocate_new_port()
            .build();
        if let Err(error) = swarm.dial(dial_options) {
            status.last_error = Some(format!("seed dial failed for {}: {error}", seed.configured));
        }
        seed.attempts = seed.attempts.saturating_add(1);
        seed.next_attempt = now + seed_reconnect_delay(&seed.configured, local_peer_id, attempt);
        available = available.saturating_sub(1);
    }
}

fn reset_seed_backoff(seed_states: &mut [SeedDialState]) {
    let now = Instant::now();
    for seed in seed_states {
        seed.attempts = 0;
        seed.next_attempt = now;
    }
}

fn seed_reconnect_delay(seed: &str, local_peer_id: &PeerId, attempt: u32) -> Duration {
    let exponent = attempt.min(8);
    let base = SEED_RECONNECT_BASE_SECONDS
        .saturating_mul(1_u64 << exponent)
        .min(SEED_RECONNECT_MAX_SECONDS);
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in seed
        .bytes()
        .chain(local_peer_id.to_bytes())
        .chain(attempt.to_le_bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let spread = base.saturating_mul(SEED_RECONNECT_JITTER_PERCENT) / 100;
    let jitter = if spread == 0 {
        0
    } else {
        hash % spread.saturating_mul(2).saturating_add(1)
    };
    Duration::from_secs(
        base.saturating_sub(spread)
            .saturating_add(jitter)
            .clamp(1, SEED_RECONNECT_MAX_SECONDS),
    )
}

fn listen_multiaddr(config: &NetworkConfig) -> NodeResult<Multiaddr> {
    let ip: IpAddr = config.p2p_bind_host.parse().map_err(|_| {
        NodeError::ConfigMismatch("p2p_bind_host must be an IPv4 or IPv6 address".to_owned())
    })?;
    let value = match ip {
        IpAddr::V4(ip) => format!("/ip4/{ip}/tcp/{}", config.p2p_listen_port()),
        IpAddr::V6(ip) => format!("/ip6/{ip}/tcp/{}", config.p2p_listen_port()),
    };
    value
        .parse()
        .map_err(|error| NodeError::P2p(format!("invalid listen address: {error}")))
}

fn seed_multiaddr(seed: &str) -> NodeResult<Multiaddr> {
    let seed = seed.trim();
    if seed.starts_with('/') {
        let canonical = canonicalize_numeric_dns_multiaddr(seed);
        return canonical
            .parse()
            .map_err(|error| NodeError::P2p(format!("invalid seed multiaddr {seed}: {error}")));
    }
    let (host, port) = seed.rsplit_once(':').ok_or_else(|| {
        NodeError::ConfigMismatch(format!("seed node {seed} must be a multiaddr or host:port"))
    })?;
    let port: u16 = port
        .parse()
        .map_err(|_| NodeError::ConfigMismatch(format!("seed node {seed} has an invalid port")))?;
    if port == 0 {
        return Err(NodeError::ConfigMismatch(format!(
            "seed node {seed} has an invalid port"
        )));
    }
    let host = host.trim_matches(['[', ']']);
    let protocol = match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(_)) => "ip4",
        Ok(IpAddr::V6(_)) => "ip6",
        Err(_) => "dns4",
    };
    format!("/{protocol}/{host}/tcp/{port}")
        .parse()
        .map_err(|error| NodeError::P2p(format!("invalid seed node {seed}: {error}")))
}

fn canonicalize_numeric_dns_multiaddr(seed: &str) -> String {
    let mut parts: Vec<&str> = seed.split('/').collect();
    if parts.len() >= 5 && matches!(parts[1], "dns4" | "dns6") {
        match parts[2].parse::<IpAddr>() {
            Ok(IpAddr::V4(_)) => parts[1] = "ip4",
            Ok(IpAddr::V6(_)) => parts[1] = "ip6",
            Err(_) => {}
        }
    }
    parts.join("/")
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devnet::{init_devnet, mine_dev_blocks, mine_pending_block};
    use alvenqis_core::{Address, Amount, Network, PrivateKey, INITIAL_BASE_FEE_ATOMIC};
    use std::net::TcpListener;
    use std::sync::atomic::AtomicU16;
    use std::sync::{Mutex, MutexGuard};
    use std::thread;
    use std::time::Instant;
    use tempfile::tempdir;

    fn p2p_network_test_guard() -> MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn free_port_pair() -> (u16, u16) {
        static PORT_ATTEMPT: AtomicU16 = AtomicU16::new(0);
        let process_offset = (std::process::id() as u16).wrapping_mul(2);

        for _ in 0..1_000 {
            let attempt = PORT_ATTEMPT.fetch_add(1, Ordering::Relaxed);
            let offset = process_offset.wrapping_add(attempt.wrapping_mul(2)) % 8_000;
            let first_port = 20_000 + offset;
            let second_port = first_port + 1;
            let first = TcpListener::bind(("127.0.0.1", first_port));
            let second = TcpListener::bind(("127.0.0.1", second_port));
            if first.is_ok() && second.is_ok() {
                return (first_port, second_port);
            }
        }

        panic!("could not reserve a non-ephemeral TCP port pair for P2P tests");
    }

    fn write_devnet_config(path: &Path, port: u16, seeds: &[String]) {
        let seeds = serde_json::to_string(seeds).expect("seed JSON is TOML compatible");
        let content = format!(
            r#"network = "devnet"
network_id = "alvenqis-devnet"
human_name = "Alvenqis Devnet"
status_label = "Draft / Private Devnet"
block_time_seconds = 60
difficulty_leading_zero_bits = 4
ticker = "ALVE"
address_prefix = "dalve"
max_supply = "60000000"
halving_interval = 1576800
initial_block_reward = "19.02587519"
default_rpc_port = 8787
default_p2p_port = 18787
p2p_bind_host = "127.0.0.1"
p2p_listen_port = {port}
max_peers = 8
seed_nodes = {seeds}
max_mempool_transactions = 32
genesis_config_path = "alvenqis-devnet/config/genesis-devnet.json"
chain_magic_hex = "414c4456"
allow_mainnet_candidate = false
"#
        );
        fs::create_dir_all(path.parent().expect("config parent")).expect("config dir");
        fs::write(path, content).expect("config");
    }

    fn write_miner_metrics(data_dir: &Path, hashrate_hs: u64) {
        let miner_dir = data_dir.parent().expect("data parent").join("miner");
        fs::create_dir_all(&miner_dir).expect("miner dir");
        fs::write(
            miner_dir.join("metrics.json"),
            serde_json::json!({
                "status": "mining",
                "network_id": Network::Devnet.network_id(),
                "height": 1,
                "hashrate_hs": hashrate_hs,
                "updated_at_unix_seconds": unix_seconds()
            })
            .to_string(),
        )
        .expect("miner metrics");
    }

    fn wait_until_result(mut predicate: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if predicate() {
                return true;
            }
            if Instant::now() >= deadline {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        false
    }

    fn wait_until(predicate: impl FnMut() -> bool) {
        assert!(
            wait_until_result(predicate),
            "condition did not become true within 30 seconds"
        );
    }

    #[test]
    fn seed_addresses_use_the_correct_transport_protocol() {
        assert_eq!(
            seed_multiaddr("127.0.0.1:20787")
                .expect("IPv4 seed")
                .to_string(),
            "/ip4/127.0.0.1/tcp/20787"
        );
        assert_eq!(
            seed_multiaddr("[::1]:20787")
                .expect("IPv6 seed")
                .to_string(),
            "/ip6/::1/tcp/20787"
        );
        assert_eq!(
            seed_multiaddr("seed.alvenqis.example:20787")
                .expect("DNS seed")
                .to_string(),
            "/dns4/seed.alvenqis.example/tcp/20787"
        );
    }

    #[test]
    fn legacy_numeric_dns_seed_is_canonicalized() {
        assert_eq!(
            seed_multiaddr("/dns4/127.0.0.1/tcp/20787")
                .expect("legacy IPv4 seed")
                .to_string(),
            "/ip4/127.0.0.1/tcp/20787"
        );
    }

    #[test]
    fn candidate_bootstrap_uses_dedicated_direct_p2p_host() {
        let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("configs/mainnet-candidate.toml");
        let config = NetworkConfig::load_from_path(&config_path).expect("config");
        assert_eq!(
            config.seed_nodes,
            vec!["node.dohotstudio.com:20787".to_owned()]
        );
        assert!(config
            .seed_nodes
            .iter()
            .all(|seed| !seed.contains("rpcnode.dohotstudio.com")));
    }

    #[test]
    fn peer_limit_is_bounded_for_the_supported_vps_profile() {
        let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("configs/mainnet-candidate.toml");
        let mut config = NetworkConfig::load_from_path(&config_path).expect("config");
        config.max_peers = crate::config::MAX_P2P_PEERS;
        config.validate().expect("supported limit");
        config.max_peers = crate::config::MAX_P2P_PEERS + 1;
        assert!(matches!(
            config.validate(),
            Err(NodeError::ConfigMismatch(message)) if message.contains("max_peers")
        ));
    }

    #[test]
    fn seed_reconnect_backoff_is_bounded_and_jittered_per_node() {
        let first = Keypair::generate_ed25519().public().to_peer_id();
        let second = Keypair::generate_ed25519().public().to_peer_id();
        for attempt in 0..32 {
            let first_delay = seed_reconnect_delay("seed.example:20787", &first, attempt);
            let second_delay = seed_reconnect_delay("seed.example:20787", &second, attempt);
            assert!(first_delay >= Duration::from_secs(1));
            assert!(first_delay <= Duration::from_secs(SEED_RECONNECT_MAX_SECONDS));
            assert!(second_delay >= Duration::from_secs(1));
            assert!(second_delay <= Duration::from_secs(SEED_RECONNECT_MAX_SECONDS));
        }
        assert!((0..8).any(|attempt| {
            seed_reconnect_delay("seed.example:20787", &first, attempt)
                != seed_reconnect_delay("seed.example:20787", &second, attempt)
        }));
    }

    #[test]
    fn plain_tcp_probe_does_not_become_a_p2p_outage() {
        assert!(is_expected_inbound_transport_noise(
            "Listen error: Failed to negotiate transport protocol(s)"
        ));
        assert!(!is_expected_inbound_transport_noise(
            "seed dial failed: connection refused"
        ));
    }

    #[test]
    fn wrong_genesis_is_rejected() {
        let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("configs/mainnet-candidate.toml");
        let config = NetworkConfig::load_from_path(&config_path).expect("config");
        let hello = PeerHello {
            protocol_version: P2P_PROTOCOL_VERSION,
            network_id: config.network_id.clone(),
            chain_magic_hex: config.chain_magic_hex.clone(),
            genesis_hash: "00".repeat(32),
            best_height: 0,
            best_hash: "00".repeat(32),
            cumulative_work: "16".to_owned(),
            validating: true,
            mining: false,
            hashrate_hs: 0,
        };
        assert!(matches!(
            validate_peer_hello(
                &config,
                &genesis_hash_hex_from_config(&config_path).expect("genesis hash"),
                &hello,
            ),
            Err(NodeError::GenesisMismatch { .. })
        ));
    }

    #[test]
    fn devnet_and_candidate_handshakes_cannot_mix() {
        let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("configs/mainnet-candidate.toml");
        let config = NetworkConfig::load_from_path(&config_path).expect("config");
        let remote = P2pHandshake {
            network_id: Network::Devnet.network_id().to_owned(),
            chain_magic_hex: config.chain_magic_hex.clone(),
            p2p_port: Network::Devnet.default_p2p_port(),
        };
        assert!(matches!(
            validate_p2p_handshake(&config, &remote),
            Err(NodeError::NetworkMismatch { .. })
        ));
    }

    #[test]
    fn two_nodes_connect_and_sync_a_direct_chain_extension() {
        let _network_test_guard = p2p_network_test_guard();
        let temp = tempdir().expect("tempdir");
        let (first_port, second_port) = free_port_pair();
        let first_config = temp.path().join("first/devnet.toml");
        let second_config = temp.path().join("second/devnet.toml");
        write_devnet_config(&first_config, first_port, &[]);
        write_devnet_config(
            &second_config,
            second_port,
            &[format!("/ip4/127.0.0.1/tcp/{first_port}")],
        );
        let first_data = temp.path().join(".alvenqis-dev/first/chain");
        let first_mempool = temp.path().join(".alvenqis-dev/first/mempool");
        let first_runtime = temp.path().join(".alvenqis-dev/first/node");
        let second_data = temp.path().join(".alvenqis-dev/second/chain");
        let second_mempool = temp.path().join(".alvenqis-dev/second/mempool");
        let second_runtime = temp.path().join(".alvenqis-dev/second/node");
        let miner_key = PrivateKey::generate();
        let miner = Address::from_public_key_for_network(&miner_key.public_key(), Network::Devnet)
            .to_string();
        init_devnet(&first_config, &first_data, &miner).expect("first genesis");
        let shared_genesis = storage::load_blocks(&first_data)
            .expect("first chain")
            .into_iter()
            .next()
            .expect("first genesis");
        storage::ensure_data_dir(&second_data).expect("second chain directory");
        storage::append_block(&second_data, &shared_genesis).expect("shared second genesis");
        write_miner_metrics(&first_data, 1_000_000);
        write_miner_metrics(&second_data, 2_000_000);

        let first_stop = Arc::new(AtomicBool::new(false));
        let first_handle = {
            let stop = Arc::clone(&first_stop);
            let config = first_config.clone();
            let data = first_data.clone();
            let mempool = first_mempool.clone();
            let runtime = first_runtime.clone();
            thread::spawn(move || run_p2p_service(config, data, mempool, runtime, stop))
        };
        wait_until(|| {
            let config = NetworkConfig::load_from_path(&first_config).expect("first config");
            load_p2p_status(&first_runtime, &config)
                .is_ok_and(|status| !status.listen_addresses.is_empty())
        });

        let second_stop = Arc::new(AtomicBool::new(false));
        let second_handle = {
            let stop = Arc::clone(&second_stop);
            let config = second_config.clone();
            let data = second_data.clone();
            let mempool = second_mempool.clone();
            let runtime = second_runtime.clone();
            thread::spawn(move || run_p2p_service(config, data, mempool, runtime, stop))
        };
        wait_until(|| {
            let config = NetworkConfig::load_from_path(&second_config).expect("second config");
            load_p2p_status(&second_runtime, &config)
                .is_ok_and(|status| status.validated_peer_count == 1)
        });
        wait_until(|| {
            let config = NetworkConfig::load_from_path(&second_config).expect("second config");
            load_p2p_status(&second_runtime, &config).is_ok_and(|status| {
                status.mining_peer_count == 2
                    && status.observed_network_hashrate_hs == 3_000_000
                    && status.miners.iter().filter(|miner| miner.local).count() == 1
            })
        });

        let recipient = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::Devnet,
        );
        let transaction = Transaction::new_signed(
            1,
            1,
            Network::Devnet,
            &miner_key,
            recipient.to_string(),
            Amount::from_atomic(100),
            Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 1),
            Amount::from_atomic(1),
            None,
        )
        .expect("signed transaction");
        submit_transaction(&first_data, &first_mempool, 32, &transaction)
            .expect("submit to first mempool");
        wait_until(|| {
            crate::mempool::load_pending_transactions(&second_mempool)
                .is_ok_and(|records| records.len() == 1)
        });

        mine_pending_block(&first_config, &first_data, &first_mempool, &miner)
            .expect("mine pending transaction");
        wait_until(|| {
            storage::load_blocks(&second_data)
                .is_ok_and(|blocks| blocks.last().is_some_and(|block| block.header.height == 1))
        });
        // Mempool clear can lag block apply slightly under async P2P.
        wait_until(|| {
            crate::mempool::load_pending_transactions(&second_mempool)
                .is_ok_and(|records| records.is_empty())
        });

        first_stop.store(true, Ordering::Relaxed);
        first_handle
            .join()
            .expect("first thread")
            .expect("first P2P");
        wait_until(|| {
            let config = NetworkConfig::load_from_path(&second_config).expect("second config");
            load_p2p_status(&second_runtime, &config)
                .is_ok_and(|status| status.connected_peer_count == 0)
        });

        let restarted_stop = Arc::new(AtomicBool::new(false));
        let restarted_handle = {
            let stop = Arc::clone(&restarted_stop);
            let config = first_config.clone();
            let data = first_data.clone();
            let mempool = first_mempool.clone();
            let runtime = first_runtime.clone();
            thread::spawn(move || run_p2p_service(config, data, mempool, runtime, stop))
        };
        wait_until(|| {
            let config = NetworkConfig::load_from_path(&second_config).expect("second config");
            load_p2p_status(&second_runtime, &config)
                .is_ok_and(|status| status.validated_peer_count == 1)
        });

        restarted_stop.store(true, Ordering::Relaxed);
        second_stop.store(true, Ordering::Relaxed);
        restarted_handle
            .join()
            .expect("restarted first thread")
            .expect("restarted first P2P");
        second_handle
            .join()
            .expect("second thread")
            .expect("second P2P");
    }

    #[test]
    fn two_divergent_nodes_reorg_to_the_higher_work_branch() {
        let _network_test_guard = p2p_network_test_guard();
        let temp = tempdir().expect("tempdir");
        let (first_port, second_port) = free_port_pair();
        let first_config = temp.path().join("first/devnet.toml");
        let second_config = temp.path().join("second/devnet.toml");
        write_devnet_config(&first_config, first_port, &[]);
        write_devnet_config(
            &second_config,
            second_port,
            &[format!("/ip4/127.0.0.1/tcp/{first_port}")],
        );
        let first_data = temp.path().join(".alvenqis-dev/first/chain");
        let first_mempool = temp.path().join(".alvenqis-dev/first/mempool");
        let first_runtime = temp.path().join(".alvenqis-dev/first/node");
        let second_data = temp.path().join(".alvenqis-dev/second/chain");
        let second_mempool = temp.path().join(".alvenqis-dev/second/mempool");
        let second_runtime = temp.path().join(".alvenqis-dev/second/node");
        let genesis_miner = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::Devnet,
        )
        .to_string();
        let first_miner = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::Devnet,
        )
        .to_string();
        let second_miner = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::Devnet,
        )
        .to_string();
        init_devnet(&first_config, &first_data, &genesis_miner).expect("first genesis");
        let shared_genesis = storage::load_blocks(&first_data)
            .expect("first chain")
            .into_iter()
            .next()
            .expect("first genesis");
        storage::ensure_data_dir(&second_data).expect("second chain directory");
        storage::append_block(&second_data, &shared_genesis).expect("shared second genesis");
        mine_dev_blocks(&first_config, &first_data, &first_miner, 2)
            .expect("first two-block branch");
        mine_dev_blocks(&second_config, &second_data, &second_miner, 1)
            .expect("second divergent block");
        let expected = storage::load_blocks(&first_data).expect("expected branch");
        assert_ne!(
            expected[1].hash().expect("hash a"),
            storage::load_blocks(&second_data).expect("second branch")[1]
                .hash()
                .expect("hash b")
        );

        let first_stop = Arc::new(AtomicBool::new(false));
        let first_handle = {
            let stop = Arc::clone(&first_stop);
            let config = first_config.clone();
            let data = first_data.clone();
            let mempool = first_mempool.clone();
            let runtime = first_runtime.clone();
            thread::spawn(move || run_p2p_service(config, data, mempool, runtime, stop))
        };
        wait_until(|| {
            let config = NetworkConfig::load_from_path(&first_config).expect("first config");
            load_p2p_status(&first_runtime, &config)
                .is_ok_and(|status| !status.listen_addresses.is_empty())
        });
        let second_stop = Arc::new(AtomicBool::new(false));
        let second_handle = {
            let stop = Arc::clone(&second_stop);
            let config = second_config.clone();
            let data = second_data.clone();
            let mempool = second_mempool.clone();
            let runtime = second_runtime.clone();
            thread::spawn(move || run_p2p_service(config, data, mempool, runtime, stop))
        };

        if !wait_until_result(|| {
            storage::load_blocks(&second_data).is_ok_and(|blocks| blocks == expected)
        }) {
            first_stop.store(true, Ordering::Relaxed);
            second_stop.store(true, Ordering::Relaxed);
            let first_status = NetworkConfig::load_from_path(&first_config)
                .ok()
                .and_then(|config| load_p2p_status(&first_runtime, &config).ok());
            let second_status = NetworkConfig::load_from_path(&second_config)
                .ok()
                .and_then(|config| load_p2p_status(&second_runtime, &config).ok());
            let actual = storage::load_blocks(&second_data).ok();
            let first_result = first_handle.join();
            let second_result = second_handle.join();
            panic!(
                "divergent nodes did not converge within 30 seconds; first_status={first_status:?}; second_status={second_status:?}; actual_chain={actual:?}; first_thread={first_result:?}; second_thread={second_result:?}"
            );
        }
        first_stop.store(true, Ordering::Relaxed);
        second_stop.store(true, Ordering::Relaxed);
        first_handle
            .join()
            .expect("first thread")
            .expect("first P2P");
        second_handle
            .join()
            .expect("second thread")
            .expect("second P2P");
    }

    /// Regression: staged header/body caps must stay enforced (CR-H06).
    #[test]
    fn staged_sync_limits_remain_positive_and_bounded() {
        // Values are compile-time constants; keep runtime asserts for discoverability
        // in test output without triggering clippy::assertions_on_constants.
        let headers = MAX_STAGED_HEADERS;
        let bodies = MAX_STAGED_REORG_BLOCKS;
        assert!(headers > 0 && headers <= 10_000);
        assert!(bodies > 0 && bodies <= 10_000);
        assert_eq!(headers, bodies);
    }
}
