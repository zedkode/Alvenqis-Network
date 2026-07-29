# Alvenqis 02 — Architecture and Product Layers

Status: Canonical architecture summary

## Layer model

### Base layer — implemented candidate

- `alvenqis-core`: consensus and protocol source of truth;
- `alvenqis-node`: chain persistence, mempool, P2P, fork choice, templates;
- `alvenqis-rpc-gateway`: profiled HTTP read/submission/mining boundaries;
- `alvenqis-indexer`: reorg-correct snapshot indexing and query data;
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
- Android and browser extension: prototypes with strict capability limits;
- Passport, marketplace, governance UI, storage products, and encrypted
  communication products: planned/research.

## Trust boundaries

- Core decides validity.
- Node decides template construction, transaction selection, and persistence.
- Miners may search only the nonce space of immutable work.
- RPC never owns wallet secrets or weakens node validation.
- Pool shares are off-chain accounting; only network-target blocks enter core.
- Website and desktop render observed data and must not invent global totals.
- VPS nodes are non-mining validators and hold no wallet material.

## Deployment boundaries

- Mainnet Candidate is the product/operator profile.
- The public RPC process binds to loopback behind a TLS reverse proxy.
- Mining and detailed operator routes remain local unless a separately reviewed
  authenticated design exists.
- `alvenqis-release/vps-control-plane/` is active; `alvenqis-release/vps/` is frozen.
- Tauri is the only Control Center product; Electron must not be reintroduced.

## Storage boundary

The node uses the accepted embedded SQLite backend with a strict versioned
schema, WAL plus full synchronous durability, transactional canonical updates,
detached-block archival, integrity checks, online backup, and automatic
one-way import from legacy JSONL while preserving the source file. Indexer and
pool snapshots still require separate transactional storage decisions. Node
restore drills, durable pre-adoption branch resume, disk-failure exercises, and
multi-host soak remain G4 blockers.
