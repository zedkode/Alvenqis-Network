# Alvenqis 02 — Architecture and Product Layers

Status: Canonical architecture summary

## Layer model

### Base layer — implemented candidate

- `alvenqis-core`: consensus and protocol source of truth;
- `alvenqis-node`: chain persistence, mempool, P2P, fork choice, templates;
- `alvenqis-rpc-gateway`: profiled HTTP read/submission/mining boundaries;
- `alvenqis-indexer`: transactional SQLite indexing and query data;
- `alvenqis-miner`: NVIDIA CUDA-only FiroPoW search;
- `alvenqis-mining-pool`: off-chain pool accounting prototype.

### Execution layer — planned

Smart-contract execution, deterministic gas metering, contract lifecycle,
events, native application assets, and VRC standards are planned or research.
Reserved folders and website concepts do not make these implemented features.

### Product layer — mixed maturity

- Tauri Control Center: Windows/Linux candidate product path;
- wallet CLI and platform keystore flows: candidate/prototype;
- explorer and website: implemented UIs with candidate data boundaries;
- TypeScript/Rust SDKs and examples: read-oriented prototypes;
- browser extension: prototype with strict capability limits;
- Passport, marketplace, governance UI, storage products, and encrypted
  communication products: planned/research.

## Trust boundaries

- Core decides validity.
- Node decides template construction, transaction selection, and persistence.
- Miners may search only the nonce space of immutable work.
- RPC never owns wallet secrets or weakens node validation.
- Pool shares are off-chain accounting; only network-target blocks enter core.
- Website and desktop render observed data and must not invent global totals.
- VPS node roles perform full validation and no nonce search. An explicitly
  enabled pool role coordinates Stratum work but still contains no miner binary
  or wallet key.

## Deployment boundaries

- Mainnet Candidate is the product/operator profile.
- In the active container profile, RPC binds on the private Docker network and
  the gateway owns public ingress. The raw RPC port is not host-published.
- `/p2p/status` is currently part of the shared router, and the accepted public
  mining policy is inconsistent with the active gateway/RPC/smoke configuration.
  Both remain documented security blockers.
- `alvenqis-release/vps-control-plane/` is active; `alvenqis-release/vps/` is frozen.
- Tauri is the only Control Center product; Electron must not be reintroduced.

## Storage boundary

The node uses SQLite as the canonical block oracle and RocksDB for state/mempool
in the VPS profile. SQLite provides a strict versioned schema, WAL plus full
synchronous durability, transactional canonical updates, detached-block
archival, integrity checks, online backup, and one-way legacy import. The
indexer now uses transactional SQLite; pool persistence still requires a
production redesign. Cross-engine consistency, restore drills, disk-failure
exercises, and multi-host soak remain G4 blockers.
