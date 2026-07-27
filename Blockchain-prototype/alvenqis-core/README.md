# alvenqis-core

Status: Implemented / Mainnet Candidate / not public Mainnet

`alvenqis-core` is the canonical source for Alvenqis protocol behavior. Product and
service layers may optimize execution, but they must not weaken or replace core
validation.

## Current scope

- exact ALVE atomic-unit and emission arithmetic;
- account-based signed transactions using ed25519 and network-separated
  lowercase Bech32m addresses;
- deterministic transaction, merkle, block-header, and payload hashing;
- FiroPoW 0.9.4 (`AlvenqisPoW v1`, period length 1) consensus hashing through the
  vendored native reference implementation;
- LWMA-style next-difficulty validation;
- base-fee burn and miner-priority-tip accounting;
- block, transaction, checkpoint, upgrade-version, and state validation;
- cumulative-work fork choice primitives and full-chain state rebuild;
- BIP39 plus hardened SLIP-0010 ed25519 wallet derivation helpers;
- deterministic network and genesis parameters;
- Rust tests and cross-backend PoW parity fixtures.

The CUDA miner searches nonces on NVIDIA GPUs. Core still recomputes and
validates submitted FiroPoW work on the host; that is consensus validation, not
CPU mining.

## Boundaries

- Mainnet Candidate is not a public Mainnet launch.
- JSONL chain persistence, P2P orchestration, RPC, wallet storage, indexing,
  pool accounting, and desktop UX belong to neighboring components.
- Smart-contract execution, native application assets, Passport, marketplace,
  staking, and DAO behavior are not implemented protocol features.
- Transaction serialization and production storage still require their
  dedicated maturity gates before G4.

See `../docs/protocol/`, `../docs/release/NETWORK_MATURITY.md`, and
`../memory/DECISIONS.md` for the current specification, maturity boundary, and
accepted-decision register.
