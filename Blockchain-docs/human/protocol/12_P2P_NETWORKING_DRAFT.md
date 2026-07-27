# Alvenqis P2P Networking

Status: Draft / Mainnet Candidate / Prototype

Alvenqis nodes use `rust-libp2p` with encrypted Noise sessions over TCP and Yamux stream multiplexing. Protocol paths include the active `network_id` and protocol version. A peer handshake additionally binds the chain magic and actual local genesis hash, preventing synchronization across incompatible networks.

The current implementation provides:

- persistent ed25519 node identity stored beside chain runtime data;
- explicit seed multiaddresses or `host:port` entries;
- bounded request/response block batches;
- periodic tip exchange and direct-extension block synchronization;
- signed pending-transaction propagation through gossipsub;
- complete node validation before received data is persisted;
- local peer telemetry for connected, handshake-validated, mining and validating peers;
- signed miner-presence propagation through gossipsub, deduplicated by the originating Peer ID;
- observed mining telemetry with a 30-second freshness window and summed hashrate for miners visible through the P2P mesh;
- a two-node test covering transaction propagation, mining telemetry aggregation, block sync and mempool cleanup.

Peer and miner totals are the local node's current P2P observation, not a globally authoritative network census or a consensus input. A node publishes mining presence only when a recent local miner heartbeat exists. Stale announcements expire after 30 seconds, and signed gossipsub origin metadata must match the announced Peer ID. `validating = true` means a full PoW node independently verifies chain data; it does not represent staking rights.

Independent solo miners do not combine nonce ranges, shares or rewards. They compete for the same canonical block independently. A mining pool would coordinate work and payouts, but it is not required for multiple solo miners to secure the same network or for nodes to display their observed aggregate hashrate.

P2P v3 uses exponential block locators to find a common ancestor, incrementally
accepts direct extensions, stages divergent branches up to 2,048 blocks, and
adopts only a fully validated branch with strictly greater cumulative work.
Detached valid transactions are reconciled back into the mempool; the SQLite
canonical-chain change and detached-block archival share one ACID transaction.
Header-first synchronization, disk-backed pre-adoption branch resume,
deep-reorg recovery, production peer scoring/bans,
broader discovery, NAT traversal, and multi-host soak remain required before G4.
