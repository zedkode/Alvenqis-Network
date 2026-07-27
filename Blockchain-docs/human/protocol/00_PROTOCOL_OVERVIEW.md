# Alvenqis Protocol Overview

Status: Implemented / Mainnet Candidate / not public Mainnet

## Current launch protocol

- proprietary Rust-based Layer 1 with ALVE as the native asset;
- account-based balances and nonce-ordered signed transfers;
- ed25519 signatures and network-separated Bech32m addresses;
- FiroPoW 0.9.4 (`AlvenqisPoW v1`, period length 1);
- NVIDIA CUDA-only product mining with canonical core validation;
- LWMA-style difficulty retargeting and cumulative-work fork choice;
- 60-second block target, 60,000,000 ALVE supply cap, 1,576,800-block
  halvings, and 19.02587519 ALVE initial reward;
- base-fee burn plus miner priority tip;
- fixed-height protocol upgrades and release-pinned early checkpoints;
- libp2p network identity bound to protocol version, network, chain magic, and
  genesis hash.

## Open protocol work

- stable final block and transaction serialization/test-vector freezes;
- SQLite restore/disk-failure evidence, durable pre-adoption branch resume, and
  header-first synchronization;
- smart-contract VM and deterministic gas model;
- final genesis allocation/treasury policy;
- longer-term scaling, governance, and checkpoint-relaxation policy.

## Data boundary

The chain stores settlement, balances, state transitions, hashes, proofs,
permissions, and explicitly implemented protocol state. Large files, media,
private profiles, and encrypted message payloads remain off-chain.

See `../release/NETWORK_MATURITY.md`: implemented candidate protocol behavior is
not equivalent to a G4 public Mainnet launch.
