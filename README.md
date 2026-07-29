# Alvenqis Network

> **Repo map (2026 layout):** [`init.md`](./init.md) · [`PROJECT_STRUCTURE.md`](./PROJECT_STRUCTURE.md)

> **Code:** [`Blockchain-prototype/`](./Blockchain-prototype/) ·

> **Docs:** [`Blockchain-docs/`](./Blockchain-docs/) ·

> **Scripts:** [`Blockchain-scripts/`](./Blockchain-scripts/)

Alvenqis Network is an independent, Rust-based Proof-of-Work Layer 1 blockchain currently under active development.

The project is being built as a complete blockchain ecosystem rather than as a token deployed on an existing network. Its architecture includes a native blockchain core, consensus rules, node software, peer-to-peer networking, wallet infrastructure, mining software, an RPC gateway, an indexer, a block explorer, developer tooling and desktop control applications.

Alvenqis is currently classified as a:

> **Mainnet Candidate / Experimental Prototype**

This classification is important.

Alvenqis Network is not currently a public production blockchain, and ALVE generated or transferred inside the current network must not be treated as real money, legal tender, an investment product or a production cryptocurrency.

---

## Important experimental-network notice

The current Alvenqis network exists for:

* blockchain development;
* protocol testing;
* local-network operation;
* controlled multi-node rehearsals;
* wallet testing;
* mining testing;
* RPC and explorer development;
* infrastructure validation;
* security research;
* release engineering;
* preparation for a possible future public network.

The current network is not intended for real financial activity.

### ALVE does not represent real funds at this stage

ALVE balances created on the current Mainnet Candidate, local network, development network or any other experimental Alvenqis environment:

* do not represent legal tender;
* do not represent fiat currency;
* do not represent bank deposits;
* do not represent electronic money;
* do not represent stablecoins;
* do not represent shares or ownership in an organization;
* do not represent a promise of future profit;
* do not represent a guaranteed future cryptocurrency;
* do not have an official exchange value;
* are not redeemable for euros, dollars or other currencies;
* are not guaranteed to migrate to a future production network;
* may be deleted if the experimental blockchain is reset;
* may become invalid after protocol changes;
* may be lost because of software defects;
* must not be sold as production assets;
* must not be accepted as payment for real goods or services;
* must not be advertised as an investment opportunity.

Any ALVE balance shown by the current wallet, explorer, miner, indexer or node is an experimental protocol balance used to test blockchain behavior.

The presence of a wallet balance does not prove monetary value.

The presence of mining rewards does not prove profitability.

The presence of a maximum supply does not create market value.

The existence of a working blockchain does not mean that its native unit is ready for public financial use.

No official token sale, exchange listing, public investment round, guaranteed liquidity program or redemption mechanism is currently provided by this repository.

---

## What exactly is Alvenqis Network?

Alvenqis Network is a complete Layer 1 blockchain project being developed from its own protocol implementation.

A Layer 1 blockchain is the base network that directly defines:

* how transactions are represented;
* how balances are calculated;
* how blocks are produced;
* how blocks are validated;
* how network consensus is reached;
* how miners participate;
* how transaction fees are processed;
* how blockchain history is selected;
* how competing chain branches are resolved;
* how nodes communicate;
* how the native asset is issued;
* how protocol upgrades are introduced.

Alvenqis is not simply:

* a website;
* a wallet interface;
* an ERC-20 token;
* a token running on Ethereum;
* a Solana token;
* a database with blockchain branding;
* a simulated explorer;
* a centralized payment balance;
* a reskinned cryptocurrency client.

The long-term objective is to operate Alvenqis as an independent distributed network in which multiple nodes can validate the same protocol rules without relying on one central database.

The current implementation already includes important parts of that foundation, but the network has not yet completed the testing, security and operational requirements necessary for public production use.

---

## Why Alvenqis is being built

Alvenqis is being developed as a general-purpose blockchain foundation for digital ownership and application infrastructure.

The project is intended to explore how one network could eventually support:

* native value transfers;
* digital ownership records;
* applications and games;
* software licenses;
* native assets;
* NFTs and game items;
* identity and reputation proofs;
* authenticity proofs;
* access permissions;
* file ownership commitments;
* storage proofs;
* encrypted communication permissions;
* creator products;
* marketplace settlement;
* developer integrations.

The aim is not to place every file, image, message or application record directly on the blockchain.

Alvenqis follows an on-chain and off-chain separation model.

### Information intended for on-chain storage

The blockchain may eventually store:

* ALVE transfers;
* transaction fees;
* balances;
* account nonces;
* ownership records;
* asset identifiers;
* smart-contract state;
* metadata hashes;
* software-license proofs;
* identity public keys;
* Passport commitments;
* access permissions;
* authenticity proofs;
* file hashes;
* storage commitments;
* marketplace settlement records;
* encrypted-channel permissions;
* message receipt hashes.

### Information intended for off-chain storage

Large or private information should remain outside the blockchain:

* images;
* videos;
* application files;
* game assets;
* encrypted message contents;
* private profile information;
* media archives;
* software packages;
* large NFT metadata;
* storage blobs;
* communication history;
* document contents;
* file replicas.

The blockchain should preserve the verifiable proof, ownership relationship or permission record without permanently copying all associated data into every node.

---

## Current project status

| Component               | Current status          | Intended use                            |
| ----------------------- | ----------------------- | --------------------------------------- |
| Blockchain core         | Implemented prototype   | Consensus and protocol development      |
| Account ledger          | Implemented             | Experimental balances and nonces        |
| Blocks                  | Implemented             | Chain construction and validation       |
| Transactions            | Implemented             | Experimental ALVE transfers             |
| Digital signatures      | Implemented             | Transaction authorization               |
| Proof of Work           | Implemented             | Candidate block production              |
| FiroPoW validation      | Implemented             | GPU-oriented mining                     |
| Difficulty logic        | Implemented prototype   | Mining-target adjustment                |
| Coinbase rewards        | Implemented             | Experimental block rewards              |
| Fee handling            | Implemented prototype   | Transaction-priority testing            |
| Mempool                 | Implemented prototype   | Pending transaction handling            |
| Fork choice             | Implemented prototype   | Competing-chain selection               |
| Reorganization handling | Implemented prototype   | Chain-branch replacement                |
| Node runtime            | Runnable                | Local and controlled operation          |
| P2P networking          | Prototype               | Seed-based synchronization; diverse active discovery remains open |
| RPC gateway             | Runnable                | Wallet, miner and explorer access       |
| Wallet CLI              | Runnable                | Experimental wallet operations          |
| Desktop Control Center  | Mainnet Candidate       | Local node, wallet and miner management |
| Indexer                 | Runnable prototype      | Searchable blockchain records           |
| Explorer                | Runnable                | Candidate-chain inspection              |
| NVIDIA GPU miner        | Implemented             | Experimental FiroPoW mining             |
| CPU miner               | Not supported           | Removed from official direction         |
| OpenCL miner            | Not supported           | No official fallback                    |
| Mining pool             | Experimental            | Controlled pool testing                 |
| Public seed network     | Not production-ready    | Future infrastructure                   |
| Public testnet          | Not officially operated | Future milestone                        |
| Public mainnet          | Not launched            | Requires launch approval                |
| Smart contracts         | Planned                 | Future execution layer                  |
| Native custom assets    | Planned                 | Future protocol capability              |
| NFTs                    | Planned                 | Future ownership standards              |
| Passport identity layer | Planned                 | Future proof and identity layer         |
| Storage proofs          | Planned                 | Future storage infrastructure           |
| Encrypted communication | Planned                 | Future off-chain communication layer    |
| Marketplace             | Planned                 | Future settlement and product layer     |
| Staking                 | Not implemented         | Not part of current consensus           |
| DAO governance          | Not implemented         | Future research only                    |

A component being present in the repository does not automatically mean it is suitable for production deployment.

---

## Network parameters

The following parameters describe the current Alvenqis protocol direction and Mainnet Candidate configuration.

| Parameter                      | Current value              |
| ------------------------------ | -------------------------- |
| Project name                   | Alvenqis Network             |
| Native asset                   | ALVE                       |
| Address prefix                 | `alve`                     |
| Blockchain type                | Layer 1                    |
| Core implementation            | Rust                       |
| Ledger model                   | Account-based              |
| Consensus model                | Proof of Work              |
| Mining algorithm               | FiroPoW 0.9.4              |
| Official mining direction      | NVIDIA GPU / CUDA          |
| CPU mining                     | Not supported              |
| OpenCL mining                  | Not supported              |
| Target block time              | 60 seconds                 |
| Target blocks per minute       | 1                          |
| Target blocks per hour         | 60                         |
| Target blocks per day          | 1,440                      |
| Target blocks per year         | 525,600                    |
| Maximum theoretical supply     | 60,000,000 ALVE            |
| Initial block reward           | 19.02587519 ALVE           |
| Halving interval               | 1,576,800 blocks           |
| Approximate halving period     | 3 years                    |
| Atomic precision               | 8 decimal places           |
| Smallest intended unit         | 0.00000001 ALVE            |
| Future consensus research      | PoLW / energy-aware mining |
| Current smart-contract support | Not implemented            |
| Current staking support        | Not implemented            |

### Block time

Alvenqis targets one block every 60 seconds.

This means the protocol aims for approximately:

```text
1 block per minute
60 blocks per hour
1,440 blocks per day
525,600 blocks per 365-day year
```

The 60-second value is a target, not a guarantee.

Actual block production may be faster or slower depending on:

* total network hashrate;
* mining difficulty;
* difficulty-adjustment behavior;
* miner participation;
* node connectivity;
* block propagation;
* software performance;
* temporary network instability.

### Confirmation model

The expected user experience is:

```text
Pending transaction:
Usually visible shortly after mempool propagation.

1 confirmation:
Approximately 60 seconds under target conditions.

Application or game settlement:
Approximately 3–6 blocks, depending on risk.

Higher-risk settlement:
12 or more blocks may be appropriate.
```

These are design guidelines, not guarantees of finality.

Proof-of-Work blockchains may experience chain reorganizations. Applications must choose confirmation requirements according to the value and risk of the action being performed.

### Maximum supply

The planned maximum theoretical supply is:

```text
60,000,000 ALVE
```

The target is produced through a decreasing block-reward schedule.

The maximum-supply figure is a protocol rule, not a financial valuation.

A limited supply does not guarantee:

* scarcity-driven demand;
* market liquidity;
* exchange support;
* price appreciation;
* mining profitability;
* adoption;
* legal classification;
* future production launch.

### Initial block reward

The initial planned block reward is:

```text
19.02587519 ALVE per block
```

At the target block rate, the approximate initial issuance would be:

```text
19.02587519 × 1,440
≈ 27,397.26027360 ALVE per day

19.02587519 × 525,600
≈ 10,000,000 ALVE per year
```

These values describe protocol issuance under ideal target timing.

They do not represent real financial income.

Mining results may differ because of:

* variable block intervals;
* difficulty changes;
* rejected blocks;
* stale blocks;
* pool fees;
* software downtime;
* network resets;
* consensus updates;
* reward-rule changes before public launch.

### Halving schedule

The planned reward-halving interval is:

```text
1,576,800 blocks
```

At exactly one block per minute, this corresponds to approximately:

```text
1,576,800 ÷ 525,600
= 3 years
```

The calendar date of a halving depends on actual block production. It is controlled by block height, not by a fixed calendar date.

The expected schedule begins approximately as follows:

| Reward era | Approximate block range |     Block reward |
| ---------- | ----------------------: | ---------------: |
| Era 1      |             0–1,576,799 | 19.02587519 ALVE |
| Era 2      |     1,576,800–3,153,599 |  9.51293759 ALVE |
| Era 3      |     3,153,600–4,730,399 |  4.75646879 ALVE |
| Era 4      |     4,730,400–6,307,199 |  2.37823439 ALVE |
| Era 5      |     6,307,200–7,883,999 |  1.18911719 ALVE |

Exact atomic-unit rounding must follow consensus code.

Documentation must never override the amount calculated by the implemented protocol.

### Transaction precision

ALVE is designed with eight decimal places.

```text
1 ALVE = 100,000,000 atomic units
1 atomic unit = 0.00000001 ALVE
```

All consensus calculations should use integers expressed in atomic units.

Floating-point arithmetic must not be used for consensus-critical balance, fee, reward or supply calculations.

### Consensus

The current consensus mechanism is Proof of Work.

Miners:

1. obtain a candidate block template;
2. process the block header through the mining algorithm;
3. search for a valid nonce;
4. submit a candidate block;
5. receive an experimental protocol reward if the block is accepted.

Full nodes independently verify the submitted block.

A miner cannot define valid transactions, create unlimited balances or bypass consensus rules merely by producing a hash. Nodes must reject blocks that violate protocol requirements.

### Mining algorithm

The current mining direction is:

```text
FiroPoW 0.9.4
NVIDIA GPU
CUDA
```

The official Alvenqis miner does not currently provide:

* CPU mining;
* OpenCL mining;
* browser mining;
* mobile mining;
* hidden background mining.

Future algorithm changes must be handled through an explicit protocol-upgrade process.

Changing the Proof-of-Work algorithm after a public launch would be a consensus-breaking change and must not happen silently.

### Future PoLW research

Alvenqis may research Proof of Low Work or energy-aware mining models in the future.

This does not mean PoLW is currently active.

The current priorities remain:

* stable Proof-of-Work consensus;
* deterministic validation;
* reliable node operation;
* functional synchronization;
* safe mining;
* predictable difficulty;
* protocol testing;
* security review.

No PoLW mechanism should be advertised as implemented until it exists in consensus code and has been independently evaluated.

---

## Architecture

Alvenqis is organized into three major conceptual layers.

## 1. Base layer

The base layer is the blockchain itself.

It is responsible for:

* block structure;
* transaction structure;
* cryptographic signatures;
* address validation;
* balances;
* account nonces;
* block rewards;
* transaction fees;
* mempool rules;
* Proof-of-Work validation;
* difficulty;
* chain selection;
* reorganizations;
* checkpoints;
* network identifiers;
* genesis configuration;
* protocol versions;
* peer synchronization;
* final settlement.

This is the most mature part of the current implementation.

## 2. Execution layer

The future execution layer is intended to support application logic.

Possible capabilities include:

* deterministic smart contracts;
* Rust-oriented development;
* WASM execution;
* gas or bounded resource usage;
* contract storage;
* contract events;
* fungible assets;
* NFTs;
* game items;
* software licenses;
* application permissions;
* marketplace escrow;
* identity proofs.

The execution layer is currently planned and must not be described as operational.

## 3. Product layer

The product layer contains the tools and services people interact with.

It includes or may eventually include:

* wallet;
* desktop Control Center;
* node-management tools;
* miner;
* mining pool;
* explorer;
* indexer;
* RPC gateway;
* developer SDK;
* browser integrations;
* Passport;
* marketplace;
* storage services;
* encrypted communication;
* application and game integrations.

Some product-layer components are runnable prototypes. Others remain planned.

---

## Repository structure

```text
Alvenqis-Network/
├── Blockchain-prototype/
│   ├── configs/
│   ├── shared/
│   ├── alvenqis-core/
│   ├── alvenqis-node/
│   ├── alvenqis-rpc-gateway/
│   ├── alvenqis-wallet/
│   ├── alvenqis-sdk/
│   ├── alvenqis-sdk-rust/
│   ├── alvenqis-indexer/
│   ├── alvenqis-explorer/
│   ├── alvenqis-miner/
│   ├── alvenqis-mining-pool/
│   ├── alvenqis-desktop-v2/
│   ├── alvenqis-mobile-core/
│   ├── alvenqis-browser/
│   ├── alvenqis-release/
│   ├── alvenqis-website/
│   └── Cargo.toml
├── Blockchain-docs/
└── Blockchain-scripts/
```

### `alvenqis-core`

The consensus-critical Rust library.

It contains or is expected to contain:

* amounts and atomic units;
* addresses;
* cryptographic keys;
* signatures;
* transactions;
* blocks;
* Merkle roots;
* genesis rules;
* chain state;
* reward calculations;
* supply calculations;
* fees;
* Proof of Work;
* FiroPoW;
* difficulty;
* chain selection;
* reorganization logic;
* protocol upgrades;
* wire serialization;
* validation rules.

`alvenqis-core` is the primary source of truth for blockchain behavior.

### `alvenqis-node`

The blockchain node runtime.

Responsibilities include:

* loading blockchain data;
* accepting blocks;
* validating transactions;
* maintaining the mempool;
* building mining templates;
* communicating with peers;
* synchronizing chain history;
* exposing node status;
* storing candidate-network data;
* reporting health and diagnostics.

### `alvenqis-rpc-gateway`

The controlled API layer between node infrastructure and clients.

It may expose:

* network status;
* chain height;
* block queries;
* transaction queries;
* address queries;
* transaction submission;
* mining templates;
* candidate-block submission;
* indexer data;
* operator endpoints;
* health endpoints.

Sensitive endpoints must fail closed behind explicit authentication and network
protection. The current RPC token check permits write/mining requests when no
token is configured; this is an open finding in
`Blockchain-docs/human/security/KNOWN_LIMITATIONS.md`.

### `alvenqis-wallet`

Wallet-related command-line and library functionality.

Current or intended responsibilities:

* wallet creation;
* mnemonic import;
* key derivation;
* address generation;
* balance queries;
* transaction construction;
* local signing;
* transaction broadcasting;
* backup warnings;
* network selection.

Wallet software must never transmit private keys or mnemonic phrases to the public RPC gateway.

### `alvenqis-miner`

Standalone mining software for Alvenqis.

Current direction:

* FiroPoW;
* NVIDIA CUDA;
* solo mining;
* RPC mining;
* experimental pool connectivity;
* worker statistics;
* block-candidate submission.

Mining during the current stage generates experimental ALVE only.

### `alvenqis-mining-pool`

Experimental pool infrastructure.

Potential responsibilities:

* worker sessions;
* mining jobs;
* share validation;
* difficulty assignment;
* block-candidate tracking;
* immature rewards;
* matured rewards;
* payout accounting;
* miner statistics;
* pool monitoring.

The pool is not currently a production financial service.

### `alvenqis-indexer`

Converts blockchain history into searchable records for:

* blocks;
* transactions;
* addresses;
* balances;
* miner rewards;
* network statistics;
* explorer pages;
* wallet history;
* future assets and contracts.

### `alvenqis-explorer`

A web interface for examining the experimental blockchain.

The explorer may display:

* latest blocks;
* block hashes;
* block height;
* transactions;
* addresses;
* mining rewards;
* candidate-network status;
* supply statistics;
* node-derived information.

Explorer information is derived from experimental network data and must not be interpreted as proof of real-world monetary value.

### Alvenqis Control Center

The desktop Control Center is intended to combine:

* node management;
* wallet operations;
* mining controls;
* explorer access;
* network statistics;
* local diagnostics;
* logs;
* release status;
* operator tooling.

It is a Mainnet Candidate application, not a certified production financial wallet.

---

## Local development and testing

### Rust checks

```powershell
cargo fmt --manifest-path Blockchain-prototype\Cargo.toml --all --check
cargo test --manifest-path Blockchain-prototype\Cargo.toml --workspace --tests
cargo clippy --manifest-path Blockchain-prototype\Cargo.toml --workspace --all-targets -- -D warnings
```

Passing these commands means that the tested code passed the repository's automated checks.

It does not mean:

* the project has passed an external audit;
* the node is secure against all attacks;
* the wallet is safe for real funds;
* the network is ready for public launch;
* all protocol behavior has been formally verified.

### Local stack

```powershell
.\Blockchain-scripts\local\start-all.ps1
```

Check status:

```powershell
.\Blockchain-scripts\local\status-all.ps1
```

Run smoke tests:

```powershell
.\Blockchain-scripts\local\run-local-smoke-test.ps1
```

Mine a local block:

```powershell
.\Blockchain-scripts\local\mine-local-block.ps1
```

Stop managed processes:

```powershell
.\Blockchain-scripts\local\stop-all.ps1
```

Reset the local chain:

```powershell
.\Blockchain-scripts\local\reset-local-chain.ps1
```

A local reset may permanently remove all experimental chain history and balances in that environment.

---

## Mainnet Candidate definition

The term **Mainnet Candidate** means that the software and configuration are being shaped toward a possible future mainnet architecture.

It does not mean:

* public mainnet;
* production blockchain;
* approved financial network;
* audited wallet;
* audited consensus;
* public token launch;
* exchange-ready asset;
* guaranteed future launch;
* immutable production ledger;
* real-funds support.

The Mainnet Candidate environment allows the project to test production-shaped behavior without falsely claiming that production has begun.

---

## Requirements before any public mainnet

Before Alvenqis can be described as a public mainnet, the project should complete at least:

* independent genesis verification;
* reproducible release builds;
* signed release artifacts;
* checksum publication;
* external consensus review;
* external wallet and keystore review;
* RPC security review;
* miner and pool review;
* long-running multi-host tests;
* adversarial synchronization tests;
* reorganization tests;
* corruption recovery tests;
* backup and restoration tests;
* disk-failure simulations;
* network partition tests;
* peer-abuse testing;
* RPC load testing;
* denial-of-service testing;
* mining-endpoint abuse testing;
* node monitoring;
* incident-response procedures;
* seed-node redundancy;
* operator runbooks;
* legal review for public financial use;
* explicit launch approval.

Until those gates are completed, all network activity must remain experimental.

---

## Maintainer

Alvenqis Network was initiated and is currently maintained by:

```text
Project founder: z3dC0d3
GitHub repository: zedkode/Alvenqis-Network
```

The maintainer currently acts as:

* project founder;
* lead protocol designer;
* lead repository maintainer;
* technical decision owner;
* release coordinator;
* architecture coordinator;
* infrastructure coordinator;
* documentation owner;
* product-direction owner;
* final reviewer for consensus-sensitive changes.

The maintainer is responsible for coordinating decisions across:

* `alvenqis-core`;
* node behavior;
* network parameters;
* mining;
* wallet behavior;
* RPC interfaces;
* indexer compatibility;
* explorer compatibility;
* desktop products;
* infrastructure;
* release engineering;
* documentation;
* public project status.

### Maintainer authority during development

During the current development stage, project governance is maintainer-led.

This means protocol decisions, repository structure, release status and public-readiness declarations require approval from the primary maintainer.

This centralized development model is a project-management decision and must not be confused with the intended decentralization of a future blockchain network.

A blockchain can have distributed consensus while its source-code project remains maintainer-governed.

### Consensus-sensitive changes

The following changes require explicit maintainer review:

* block format;
* transaction format;
* transaction signing;
* address format;
* genesis configuration;
* chain identifiers;
* maximum supply;
* block rewards;
* halving rules;
* atomic precision;
* transaction fees;
* difficulty adjustment;
* Proof-of-Work rules;
* mining algorithm;
* fork choice;
* reorganization behavior;
* protocol upgrades;
* checkpoint rules;
* wallet derivation;
* peer compatibility;
* serialization.

These changes may invalidate wallets, blocks or entire network histories. They must not be merged as routine user-interface updates.

### Maintainer security policy

The maintainer will never require contributors or users to publish:

* private keys;
* mnemonic phrases;
* wallet passwords;
* API tokens;
* database passwords;
* signing keys;
* `.env` files;
* Cloudflare credentials;
* server credentials.

Anyone claiming that such information is required for support should be treated as untrusted.

### Future contributors

Alvenqis may accept contributions in areas such as:

* Rust development;
* blockchain testing;
* networking;
* cryptographic review;
* wallet security;
* mining optimization;
* RPC development;
* indexer development;
* explorer development;
* infrastructure;
* technical documentation;
* reproducible builds;
* security research.

Contribution does not automatically grant authority over consensus or release status.

---

## Security

Never commit:

```text
.env
private keys
mnemonic phrases
wallet files
API tokens
database credentials
RPC credentials
release signing keys
Cloudflare credentials
production configuration secrets
```

Security reports should be handled through responsible disclosure rather than immediately publishing exploitable details.

Before public production use, the following require independent review:

* consensus;
* cryptography integration;
* transaction validation;
* address handling;
* wallet derivation;
* keystore encryption;
* RPC authorization;
* peer networking;
* miner communication;
* pool accounting;
* release distribution;
* update mechanisms.

---

## Current limitations

Alvenqis currently has several important limitations:

* there is no public production mainnet;
* ALVE is not real money;
* current ALVE balances may be reset;
* there is no official exchange value;
* there is no official redemption system;
* there is no independent external audit;
* there is no completed production security review;
* there is no formally verified consensus implementation;
* there is no long-running permissionless public network;
* there is no production mining pool;
* there is no production payout signer;
* there is no live smart-contract runtime;
* there is no staking system;
* there is no DAO;
* there is no active Passport protocol;
* there is no active NFT protocol;
* there is no active marketplace;
* there is no production storage-proof network;
* there is no production encrypted-messaging network;
* infrastructure and storage remain candidate-grade;
* protocol rules may still change;
* candidate chains may be reset;
* releases may contain defects;
* APIs may change without backward compatibility.

These limitations are published deliberately so that experimental software is not mistaken for a finished financial product.

---

## Planned development direction

### Base-network hardening

* production-grade chain storage;
* stronger database durability;
* PeerId-pinned independent bootstrap and active discovery;
* per-IP/subnet/ASN admission controls;
* adversarial synchronization and deep-resume validation;
* stronger reorganization tests;
* long-running soak tests;
* multi-host deployment;
* redundant seed infrastructure;
* monitoring and alerting;
* backup and recovery;
* incident-response procedures.

### Wallet maturity

* improved keystore protection;
* isolated signing;
* hardware-backed signing research;
* transaction previews;
* stronger user warnings;
* secure backup flows;
* signed desktop releases;
* signed update metadata;
* reproducible builds.

### Developer platform

* stable RPC versions;
* TypeScript SDK;
* Rust SDK;
* code examples;
* local application sandbox;
* contract tooling;
* test utilities;
* event indexing;
* application authentication.

### Execution-layer research

* deterministic WASM execution;
* gas metering;
* contract storage;
* contract events;
* resource limits;
* contract standards;
* asset standards;
* safe protocol upgrades.

### Product research

* VRC-20 fungible assets;
* VRC-721 unique assets;
* VRC-1155 multi-assets and game items;
* VRC-LICENSE software-license proofs;
* VRC-PASS Passport proofs;
* VRC-GAME game ownership and achievements;
* VRC-COMM encrypted communication permissions;
* VRC-FILE storage and file proofs;
* VRC-MARKET marketplace settlement.

These names represent proposed standards, not currently live protocols.

---

## Project summary

Alvenqis Network is an attempt to build a complete independent Layer 1 blockchain ecosystem from its own Rust-based protocol foundation.

The project is not limited to creating a native coin. Its broader objective is to establish the technical base required for a network that can eventually support digital ownership, software licensing, applications, games, identity proofs, native assets, NFTs, storage commitments and encrypted digital services.

The current repository already goes beyond a concept document.

It contains working or partially working implementations for:

* blockchain consensus rules;
* blocks;
* transactions;
* cryptographic signing;
* account balances;
* account nonces;
* mining rewards;
* maximum-supply calculations;
* halving behavior;
* transaction fees;
* mempool processing;
* Proof-of-Work validation;
* FiroPoW mining;
* chain selection;
* blockchain reorganizations;
* node operation;
* RPC access;
* wallet operations;
* indexing;
* block exploration;
* NVIDIA GPU mining;
* experimental pool mining;
* desktop network management;
* local and VPS-oriented deployment tools.

However, the existence of these components does not make Alvenqis a finished public blockchain.

The project remains in the difficult stage between a functional prototype and production infrastructure.

The most important work ahead is not simply adding more visible features. It is proving that the existing protocol and infrastructure can operate:

* consistently;
* securely;
* reproducibly;
* across independent hosts;
* under network failures;
* under hostile input;
* during chain reorganizations;
* during storage failures;
* without centralized data corruption;
* without exposing wallet secrets;
* without relying on undocumented behavior.

Alvenqis must demonstrate that different nodes running the same protocol reach the same result.

It must demonstrate that invalid blocks and transactions are rejected consistently.

It must demonstrate that wallets sign exactly what users intend.

It must demonstrate that mining cannot bypass issuance rules.

It must demonstrate that supply calculations remain correct across reward eras.

It must demonstrate that operators can back up, restore and recover their infrastructure.

It must demonstrate that public releases are authentic and reproducible.

Until these conditions are met, Alvenqis should be described honestly as a Mainnet Candidate and experimental blockchain.

The current ALVE asset exists only as a protocol unit inside experimental environments. It is not intended for real funds, investment, commercial payment or financial settlement.

A possible future production ALVE network may only be considered after technical gates, security reviews, legal review and an explicit launch decision.

No future launch, exchange listing, price, liquidity or conversion of experimental balances is guaranteed.

The long-term success of Alvenqis depends on engineering quality, transparent status reporting, independent review, reliable infrastructure and real decentralized participation—not on promotional claims.

---

## Honest public wording

Appropriate descriptions:

```text
Rust-based Layer 1 under development
Mainnet Candidate
Experimental blockchain
Protocol prototype
Local operator network
Controlled multi-node rehearsal
Experimental GPU mining
Experimental ALVE balances
Planned smart-contract platform
```

Descriptions that must not be used at this stage:

```text
Mainnet live
Production-ready blockchain
Audited cryptocurrency
Real-funds wallet
Guaranteed investment
Guaranteed returns
Guaranteed mining income
Exchange-ready asset
Live staking rewards
Live DAO
Live NFT ecosystem
Live smart-contract platform
Production mining pool
```

---

## Financial and legal disclaimer

This repository provides experimental software.

It does not provide:

* financial advice;
* investment advice;
* legal advice;
* banking services;
* custody services;
* exchange services;
* guaranteed returns;
* guaranteed mining income;
* guaranteed token value;
* guaranteed liquidity;
* guaranteed future launch;
* guaranteed conversion of experimental balances.

Do not use the current Alvenqis software to store, transfer or represent funds that you cannot afford to lose.

Do not purchase experimental ALVE from anyone claiming that it has an official monetary value.

Do not send money to anyone promising guaranteed ALVE allocations, guaranteed exchange listings or guaranteed mining returns.

Any future public sale, exchange integration, payment system, marketplace, treasury, staking mechanism or token distribution requires separate technical and legal review.

---

## License

Licensing may differ by component.

Protocol and chain-critical components may use an open-source license, while administrative, operational or commercial product layers may use different terms.

Every distributable component must include a clear license before public release.

The presence of visible source code does not automatically grant unrestricted permission to copy, relicense, resell or represent the project.

---

## Disclaimer

Alvenqis Network is under active development.

The software may contain:

* defects;
* vulnerabilities;
* incomplete features;
* breaking changes;
* data-loss risks;
* compatibility problems;
* incorrect user-interface information;
* unstable APIs;
* unreviewed cryptographic integration.

Use it only for development, research and controlled experimental operation.

Do not use the current network for real funds.
