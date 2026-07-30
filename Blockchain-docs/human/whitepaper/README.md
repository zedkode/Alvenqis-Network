# Alvenqis Network Whitepaper

Status: **Draft / Mainnet Candidate / Experimental Prototype**

Version: 0.1-draft  
Last reviewed: 2026-07-29

> This document describes the current candidate design and implemented
> boundaries. It is not an external audit, investment document, public Mainnet
> announcement, or guarantee of future functionality. ALVE on the candidate
> network is experimental and is not authorized for real-funds use.

## 1. Executive summary

Alvenqis Network is an experimental Rust-based Layer 1 blockchain focused on:

- independently verifiable state transitions;
- deterministic protocol serialization;
- FiroPoW 0.9.4 proof of work;
- GPU mining through the NVIDIA CUDA product path;
- local-first wallet and node operation;
- explicit separation between public RPC reads/submission and private mining
  capabilities;
- reproducible candidate release gates;
- incremental removal of project-operated infrastructure dependencies.

The current network is a **Mainnet Candidate**, not a public production
Mainnet. The candidate implementation includes a node, wallet, RPC gateway,
indexer, explorer, mining pool, miner, desktop control center, browser native
host, Rust SDK and role-based Alvenqis Setup External.

The repository does not currently claim:

- public Mainnet readiness;
- completed decentralization;
- an independent external security audit;
- formal verification;
- production financial safety;
- a production staking, smart-contract or on-chain governance system.

## 2. Design principles

### 2.1 Verifiability before convenience

Consensus behavior is implemented in the Rust core and exercised by deterministic
test vectors and release-gate tests. Public documentation links to implementation
and verification commands rather than replacing them with marketing claims.

### 2.2 Local operation remains possible

The target architecture keeps validation, wallet operation and solo mining
possible without requiring a project-hosted public RPC. Solo mining uses a local
loopback RPC connected to local sidecars. Pool miners use authenticated Stratum
TLS.

### 2.3 Public and privileged capabilities are separate

Public RPC profiles expose read APIs and, where explicitly configured,
transaction submission. Public `/mining/*` routes are unavailable. Mining
templates and block submission are limited to local or unpublished
container-network RPC profiles.

### 2.4 Maturity is evidence-based

The G0-G4 ladder separates repository hygiene, local candidate release checks,
operator rehearsal, security readiness and public launch approval. Passing G1
does not authorize a public Mainnet claim.

## 3. System architecture

The implementation is divided into four practical layers:

1. **Consensus and ledger layer**  
   Blocks, transactions, proof-of-work validation, difficulty, supply,
   checkpoints, fork choice and state transitions.
2. **Network and service layer**  
   P2P synchronization, RPC, indexer, explorer APIs, mining pool and Stratum.
3. **Operator layer**  
   Role-based Compose profiles, local scripts, health checks, monitoring and
   controlled release tooling.
4. **Client layer**  
   Desktop control center, browser native host, explorer, website and SDKs.

Current implementation maps are maintained in:

- [System Overview](../architecture/00_SYSTEM_OVERVIEW.md)
- [Base Layer](../architecture/01_BASE_LAYER.md)
- [Execution Layer](../architecture/02_EXECUTION_LAYER.md)
- [Product Layer](../architecture/03_PRODUCT_LAYER.md)
- [Dependency Impact Map](../architecture/04_CORE_WALLET_RPC_INDEXER_EXPLORER_IMPACT.md)

There is no active mobile client or mobile release artifact. Any future mobile
implementation requires a new design and security review.

## 4. Consensus and chain identity

### 4.1 Proof of work

Alvenqis validates blocks with FiroPoW 0.9.4. The consensus core evaluates and
verifies proof-of-work headers. Product mining is an NVIDIA CUDA path; CPU,
OpenCL, hybrid and host-emulated product mining are not supported.

The canonical proof-of-work description is:

- [Proof of Work](../protocol/06_CONSENSUS_POW.md)
- [CUDA and ASIC Resistance](../mining/CUDA_AND_ASIC_RESISTANCE.md)
- [Consensus Serialization and Test Vectors](../protocol/CONSENSUS_SERIALIZATION_AND_TEST_VECTORS.md)

### 4.2 Difficulty and timing

Candidate parameters, target block interval and difficulty behavior are defined
in the protocol documents and validated implementation. Draft behavior is not a
license to change consensus values without the protected review path.

- [Chain Parameters](../protocol/01_CHAIN_PARAMETERS.md)
- [Difficulty Adjustment](../protocol/07_DIFFICULTY_ADJUSTMENT_DRAFT.md)

### 4.3 Genesis and checkpoints

The Mainnet Candidate genesis inputs and frozen output are documented and
reviewed as candidate artifacts. The current checkpoint policy pins the
candidate genesis at height zero. Checkpoint enforcement is not removed by the
decentralization program; relaxation requires the documented policy path.

- [Genesis](../release/GENESIS.md)
- [Genesis Ceremony and Allocation](../release/GENESIS_CEREMONY_AND_ALLOCATION.md)
- [Checkpoint Policy](../protocol/13_CHECKPOINT_POLICY.md)
- [Fork Choice, Reorg and Checkpoints](../protocol/FORK_CHOICE_REORG_AND_CHECKPOINTS.md)

## 5. Transactions, state and supply

Transactions use explicit network identity, nonce, recipient, amount, fee and
signature fields. State transition validation rejects invalid signatures,
cross-network recipients, nonce violations, insufficient balances and supply
overflow.

The current candidate model includes:

- deterministic transaction and block encoding;
- signed account transfers;
- coinbase rewards;
- bounded transaction-hash retention;
- persisted ledger state with recovery checks;
- network-specific address prefixes.

Canonical references:

- [Block Structure](../protocol/04_BLOCK_STRUCTURE.md)
- [Transaction Model](../protocol/05_TRANSACTION_MODEL.md)
- [Addresses and Keys](../protocol/09_ADDRESS_AND_KEYS_DRAFT.md)
- [Storage Consistency and Recovery](../architecture/STORAGE_CONSISTENCY_AND_RECOVERY.md)
- [Ledger State and Transaction Retention](../architecture/LEDGER_STATE_AND_TX_HASH_RETENTION.md)

Supply and emission are described in:

- [ALVE Units and Supply](../protocol/02_ALVE_UNITS_AND_SUPPLY.md)
- [Emission and Halving](../protocol/03_EMISSION_AND_HALVING.md)
- [Supply and Reward Math](../tokenomics/01_SUPPLY_AND_REWARD_MATH.md)

Candidate balances may be reset and have no official exchange or redemption
value.

## 6. Peer-to-peer network

The node uses libp2p transports and identifies peers by cryptographic PeerId.
The current candidate supports bounded connections, peer reputation, persistent
bans and seed retry behavior.

Important open decentralization work remains:

- bootstrap diversity is not yet independently proven;
- project-operated seed dependency is not yet closed;
- active discovery and failover evidence are incomplete;
- IP, subnet and ASN-aware admission controls remain planned;
- permissionless multi-operator soak evidence remains outstanding.

These limitations are tracked openly in:

- [P2P Networking](../protocol/12_P2P_NETWORKING_DRAFT.md)
- [Seed Diversity and Discovery](../operator/SEED_DIVERSITY_AND_DISCOVERY.md)
- [Decentralization Readiness](../release/DECENTRALIZATION_READINESS.md)
- [Known Limitations](../security/KNOWN_LIMITATIONS.md)

## 7. RPC, indexer and explorer

The RPC gateway has explicit access profiles:

- `local` for loopback operation;
- `public-read` for public read-only service;
- `public-submit` for public reads and transaction submission;
- `private-mining` for unpublished local or container-network mining service.

Public mining endpoints are unavailable by policy. The indexer derives query
state from the node and the explorer consumes public APIs. Public service
availability does not replace independent node operation.

References:

- [RPC Gateway](../api/00_RPC_GATEWAY_OVERVIEW.md)
- [RPC Endpoints](../api/01_RPC_ENDPOINTS_DRAFT.md)
- [Response Models](../api/02_RPC_RESPONSE_MODELS.md)
- [Chain Health](../operator/CHAIN_HEALTH.md)

## 8. Mining and pool operation

### 8.1 Solo mining

Solo mining is local-first:

1. run a local node, wallet, RPC and indexer sidecar set;
2. connect the miner to loopback RPC;
3. request work locally;
4. submit candidate blocks locally;
5. let the node validate and propagate accepted blocks.

The desktop does not silently fall back to a project public RPC for solo mining.

### 8.2 Pool mining

Pool miners use Stratum TLS. Plaintext sent to the TLS listener is rejected.
The pool obtains templates from Docker-private RPC and does not require public
mining routes.

Independent pool operation, signer isolation and multi-operator evidence remain
required before production claims.

References:

- [GPU Mining](../mining/GPU_MINING.md)
- [Mining Pool Protocol](../protocol/13_MINING_POOL_PROTOCOL_DRAFT.md)
- [Private Mining Operations](../operator/PRIVATE_MINING_OPS.md)
- [Mining Pool Risks](../security/MINING_POOL_RISKS.md)

## 9. Wallet and clients

The local wallet uses deterministic key derivation, encrypted keystore material
and explicit network separation. The desktop control center presents real
service states and must not simulate connectivity or synchronization.

The browser integration uses a native host boundary instead of exposing wallet
secrets directly to arbitrary web content.

References:

- [Wallet Key Management](../security/WALLET_KEY_MANAGEMENT.md)
- [Client Platform Direction](../architecture/06_CLIENT_PLATFORM_DIRECTION.md)
- [Browser Extension and Native Host](../architecture/07_BROWSER_EXTENSION_AND_NATIVE_HOST.md)
- [Desktop User Guide](../operator/DESKTOP_V2_USER_GUIDE.md)

## 10. Security model

Security is treated as a set of bounded controls, not a certification claim.
Current controls include repository hygiene checks, secret scanning, workflow
pinning, strict Rust linting, deterministic tests, RPC profile validation,
encrypted transport for Stratum and explicit release maturity wording.

Open risks include incomplete external review, candidate infrastructure
availability, P2P admission hardening, independent operator evidence, update
signing readiness and production incident rehearsal.

Canonical security documents:

- [Threat Model](../security/THREAT_MODEL.md)
- [Production Risks](../security/PRODUCTION_RISKS.md)
- [Known Limitations](../security/KNOWN_LIMITATIONS.md)
- [Supply Chain and Artifacts](../security/SUPPLY_CHAIN_AND_ARTIFACTS.md)
- [External Security Review Scope](../security/EXTERNAL_SECURITY_REVIEW_SCOPE.md)
- [Verification and Audit Status](../security/AUDIT_STATUS.md)

## 11. Decentralization program

Decentralization is measured by whether an independent operator can validate,
synchronize, read, submit, recover and mine without mandatory project-operated
services.

The candidate has not yet met that complete standard. The tracked program
includes:

- diverse PeerId-pinned bootstrap sources;
- active peer discovery and bounded failover;
- independent clean-host node installation;
- independent RPC, explorer and solo-mining paths;
- independent pool operation;
- project-infrastructure outage rehearsal;
- explicit disclosure of checkpoint and maintainer-governance boundaries.

No document may claim complete decentralization until the required gates and
independent evidence exist.

## 12. Governance and upgrades

Current protocol changes are maintainer-reviewed repository changes. There is no
active DAO or on-chain parameter voting system. Upgrade activation follows the
documented candidate policy.

The unresolved validator threshold question remains outside this whitepaper and
is not silently resolved here.

References:

- [Upgrade Activation](../protocol/12_UPGRADE_ACTIVATION_POLICY.md)
- [Agents and Governance](../source-info/ALVENQIS_04_AGENTS_AND_GOVERNANCE.md)

## 13. Operations and release maturity

Candidate software is validated through:

```bash
bash Blockchain-scripts/release/release-gate.sh
```

On Windows:

```powershell
.\Blockchain-scripts\release\release-gate.ps1
```

These commands validate repository hygiene, documentation, Rust code, web
applications and control-plane configuration. A passing G1 result authorizes
candidate artifacts and controlled rehearsal only.

- [Network Maturity](../release/NETWORK_MATURITY.md)
- [Release Gate](../release/RELEASE_GATE.md)
- [Candidate Checklist](../release/MAINNET_CANDIDATE_CHECKLIST.md)
- [Independent Node Operator Guide](../operator/INDEPENDENT_NODE_OPERATOR_GUIDE.md)

## 14. Roadmap boundaries

Near-term work prioritizes:

1. deterministic green G1 evidence on one immutable commit;
2. G2 P2P and independent operator rehearsal;
3. G3 security controls and project-outage proof;
4. external review preparation;
5. G4 human go/no-go review only after all required evidence exists.

Smart contracts, staking, DAO governance, mobile clients and broader product
layers are future work unless promoted through accepted design and task
tracking.

## 15. Verification and source map

The whitepaper is an explanatory index. Normative behavior remains in validated
code, protocol documents and accepted decisions.

Run the documentation checks with:

```bash
node Blockchain-scripts/docs/check-english-content.mjs
node Blockchain-scripts/docs/audit-docs.mjs
```

Start review with:

- [Documentation Policy](../DOCUMENTATION_POLICY.md)
- [Protocol Index](../protocol/README.md)
- [Security Index](../security/README.md)
- [Network Maturity](../release/NETWORK_MATURITY.md)
- [Documentation Inventory](../DOCUMENTATION_INVENTORY.md)

## 16. Disclaimer

This is experimental software. No statement in this draft is financial, legal
or investment advice. No availability, security, mining revenue, exchange value
or future feature is guaranteed.
