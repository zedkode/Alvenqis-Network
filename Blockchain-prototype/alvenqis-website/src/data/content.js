export const navItems = [
  { label: 'Core', href: '/core' },
  { label: 'Mining', href: '/mining' },
  { label: 'Wallet', href: '/wallet' },
  { label: 'Explorer', href: '/explorer' },
  { label: 'Downloads', href: '/downloads' },
  { label: 'Passport', href: '/passport' },
  { label: 'Developers', href: '/developers' },
  { label: 'Roadmap', href: '/roadmap' },
  { label: 'Status', href: '/status' },
]

export const networkStats = [
  { label: 'Max supply', value: '60M ALVE' },
  { label: 'Block time', value: '60 sec' },
  { label: 'Halving', value: '~3 years' },
  { label: 'Initial reward', value: '19.02587519 ALVE' },
]

export const productLayers = [
  {
    title: 'Base Layer',
    eyebrow: 'PoW first',
    text: 'Blocks, transactions, native ALVE, mining, fees, state, mempool, P2P sync, reorg handling and final settlement.',
  },
  {
    title: 'Execution Layer',
    eyebrow: 'Rust/WASM direction',
    text: 'Low-fee smart contracts, dApps, NFT standards, app and game runtime, predictable fees and indexable events.',
  },
  {
    title: 'Product Layer',
    eyebrow: 'Ownership UX',
    text: 'Wallet, explorer, indexer, RPC gateway, Passport proofs, marketplace, SDK, website, admin and status surfaces.',
  },
]

export const confirmationModel = [
  ['Pending UX', '1-3 sec', 'Wallet can show a pending state after mempool propagation.'],
  ['1 confirmation', '~60 sec', 'One mined block under the current block-time target.'],
  ['App/game safe', '3-6 blocks', 'Suggested range for regular app, game and marketplace interactions.'],
  ['High value', '12+ blocks', 'Suggested deeper confirmation range for higher-value movement.'],
]

export const chainFacts = [
  ['Consensus base', 'PoW first', 'PoLW remains research / upgrade path, not a first-launch claim.'],
  ['Core language', 'Rust', 'Chosen for performance, memory safety and distributed-system tooling.'],
  ['Native asset', 'ALVE', 'Used for rewards, transfers, fees and future product-layer settlement.'],
  ['Data model', 'Open', 'UTXO/account, DAA, PoW hash and VM choices are still protocol decisions.'],
]

export const onChainItems = [
  'ALVE transfers',
  'fees',
  'smart contract state',
  'native assets',
  'NFT ownership',
  'software license proofs',
  'Passport commitments',
  'identity public keys',
  'access rights',
  'file hashes',
  'storage proofs',
  'marketplace settlement',
]

export const offChainItems = [
  'large files',
  'NFT images',
  'game assets',
  'encrypted messages',
  'storage blobs',
  'private profile data',
  'chat history',
  'large metadata',
  'media files',
  'file replicas',
  'communication payloads',
  'digital delivery payloads',
]

export const standards = [
  ['VRC-20', 'Fungible tokens'],
  ['VRC-721', 'Unique NFTs'],
  ['VRC-1155', 'Multi-assets and game items'],
  ['VRC-LICENSE', 'Software license proofs'],
  ['VRC-PASS', 'Passport identity proofs'],
  ['VRC-GAME', 'Game inventory and achievements'],
  ['VRC-COMM', 'Encrypted channel permissions'],
  ['VRC-FILE', 'File and storage proofs'],
  ['VRC-MARKET', 'Marketplace settlement'],
]

export const ecosystemProducts = [
  ['Wallet', 'Create/import wallets, send and receive ALVE, connect to dApps and display ownership proofs.'],
  ['Explorer', 'Search blocks, transactions, addresses, assets, contract events and network health.'],
  ['Indexer', 'Transform chain events into queryable data for explorer, apps, dashboards and marketplace UX.'],
  ['RPC Gateway', 'Public/private API layer for wallets, apps, explorer and admin tools.'],
  ['Passport', 'Human-friendly proof layer for licenses, access, reputation, achievements and authenticity.'],
  ['SDK', 'TypeScript and Rust tooling for developers building apps, games and digital products on Alvenqis.'],
]

export const coreModules = [
  ['Blocks', 'Block header, body, parent hash, timestamp, difficulty, nonce, reward and validation context.'],
  ['Transactions', 'Account-based signed transfers with sequential nonces, fee caps and priority tips.'],
  ['Mempool', 'Bounded pending transactions, validation, propagation and reorg reconciliation.'],
  ['State', 'ALVE balances, emitted supply, burned fees and account nonces; assets/contracts remain planned.'],
  ['P2P sync', 'Noise/Yamux peers, header-first branch verification, cumulative-work selection and bounded reorgs.'],
  ['Storage', 'Auditable JSONL chain plus atomic index snapshots; production database work remains open.'],
  ['CLI', 'Node control, mining commands, status checks and developer workflows.'],
  ['Validation', 'Consensus rules, difficulty, fees, signatures and deterministic verification.'],
]

export const miningModules = [
  ['Solo miner', 'NVIDIA CUDA-only FiroPoW miner using versioned RPC templates and submissions.'],
  ['Pool server', 'Reachable Mainnet Candidate prototype with VarDiff shares, PPLNS accounting and maturity tracking.'],
  ['Reward schedule', 'Initial reward 19.02587519 ALVE per block, halving around every 1,576,800 blocks.'],
  ['Difficulty', 'Implemented LWMA-style adjustment over a 60-block window for a 60-second target.'],
  ['Hardware direction', 'FiroPoW 0.9.4 product search is NVIDIA CUDA-only; CPU/OpenCL mining is removed.'],
  ['Transparency', 'Pool and hashrate values are observed prototype data, not guaranteed global totals or production readiness.'],
]

export const walletFeatures = [
  ['Create/import', 'Implemented BIP39/SLIP-0010 ed25519 wallet flows with platform keystore boundaries.'],
  ['Send/receive', 'Candidate ALVE transfer UX with account nonce, fee and lifecycle context.'],
  ['Assets', 'VRC assets, NFTs, licenses and Passport records remain planned and must show unavailable states.'],
  ['Connect apps', 'App and game connection UX after RPC, SDK and contract standards mature.'],
  ['Security', 'Clear warnings, backup flows, local signing and no fake production readiness.'],
  ['Status-aware UX', 'Wallet actions remain gated until the Mainnet Candidate RPC and node are healthy.'],
]

export const explorerFeatures = [
  ['Latest blocks', 'Height, hash, time, reward, difficulty and miner metadata where available.'],
  ['Transactions', 'Status, fees, confirmations, sender/receiver or state transitions.'],
  ['Addresses', 'ALVE balance, assets, NFTs, license proofs and public Passport commitments.'],
  ['Assets', 'VRC-20, VRC-721, VRC-1155 and marketplace settlement surfaces.'],
  ['Contracts', 'Events, calls, state changes and execution metadata after VM direction is ready.'],
  ['Network health', 'RPC status, node sync, Mainnet Candidate health and known limitations.'],
]

export const developerStack = [
  ['Rust core', 'Implemented core, node, CUDA miner, RPC gateway and indexer candidate stack.'],
  ['TypeScript SDK', 'Implemented read-oriented RPC/pool client with examples; broader app tooling remains planned.'],
  ['WASM contracts', 'Rust/WASM contract direction with deterministic execution and gas metering.'],
  ['Indexer storage', 'Atomic snapshot index today; production query database remains an explicit decision.'],
  ['Node storage', 'Append/fsync JSONL today; production segmented/database storage remains an explicit decision.'],
  ['React product UI', 'Website, wallet, explorer, admin and status dashboards.'],
]

export const roadmap = [
  ['G0', 'Specification clarity', 'Maintained', 'Canonical decisions, open questions, documentation inventory and honest public boundaries.'],
  ['G1', 'Candidate hygiene', 'Active', 'Tests, lint, scans, documentation audit and reproducible Windows/Linux/VPS artifacts.'],
  ['G2', 'Controlled rehearsal', 'Active', 'Candidate configs, health checks, backup, rollback and updater failure paths.'],
  ['G3', 'Multi-host evidence', 'In progress', 'Production storage review, P2P/pool soak, RPC abuse tests and platform QA.'],
  ['G4', 'Public Mainnet approval', 'Blocked', 'Independent review, signed artifacts, production operations and named go-live signatories.'],
  ['Post-G4', 'Execution products', 'Research', 'Contracts, VRC assets, Passport and marketplace only after base-layer maturity.'],
]

export const openDecisions = [
  ['Scaling', 'Future scaling and whether sharding is ever needed remain research.'],
  ['Storage', 'Production node, indexer and pool storage plus migrations remain open.'],
  ['Serialization', 'Stable versioned block and transaction wire formats still require final vectors.'],
  ['Contract gas', 'Transfer fees are implemented; deterministic contract execution metering is not.'],
  ['VM', 'wasmtime, wasmer or custom VM remains a research decision.'],
  ['Tokenomics', 'Final genesis allocation, treasury, founder and vesting policy must be explicit.'],
  ['Governance', 'Long-term upgrade/community governance is unresolved; no DAO exists.'],
  ['Launch', 'Independent review, signing and G4 go-live approval remain incomplete.'],
]

export const docsCards = [
  ['Architecture', 'Base, execution and product layers.'],
  ['Protocol', 'Blocks, transactions, fees, mining and validation.'],
  ['Tokenomics', 'Supply, rewards, halving and pending allocation decisions.'],
  ['Standards', 'VRC token, NFT, license, Passport, game, comms, file and market standards.'],
  ['Security', 'Threat model, release gates, audits and responsible disclosure.'],
  ['Operations', 'Mainnet Candidate monitoring, releases, backups and incident response.'],
]

export const passportUseCases = [
  ['Game achievements', 'A player can prove rare achievements, inventory ownership or seasonal participation without exposing private profile data.'],
  ['Software licenses', 'A buyer can hold a license proof for apps, plugins, digital tools or downloadable creator products.'],
  ['Creator drops', 'Creators can attach authenticity, access and delivery proofs to digital goods and limited releases.'],
  ['Reputation', 'Apps can read public proof records while keeping sensitive identity fields hidden.'],
  ['Marketplace access', 'Sellers and buyers can use proof-based permissions for gated files, support channels and ownership transfer.'],
  ['Miner/operator badge', 'A future operator proof can represent support, participation or infrastructure contribution.'],
]

export const whitepaperSections = [
  ['Identity', 'Alvenqis Network / ALVE, Rust-based, mineable Layer 1 for ownership and low-fee applications.'],
  ['Economics', '60M max supply, 60 second blocks, halving around three years and defined initial reward.'],
  ['Architecture', 'Base, execution and product layers with large payloads off-chain and settlement/proofs on-chain.'],
  ['Standards', 'VRC-20, VRC-721, VRC-1155, VRC-LICENSE, VRC-PASS, VRC-GAME, VRC-COMM, VRC-FILE, VRC-MARKET.'],
  ['Roadmap', 'Specs, core chain, Mainnet Candidate, RPC/wallet/explorer and security gates.'],
  ['Risk', 'Production storage, serialization, multi-host evidence, signing and independent review remain before G4.'],
]

export const tokenomicsRows = [
  ['Native asset', 'ALVE', 'Used for rewards, transfers, fees and future settlement surfaces.'],
  ['Max supply', '60,000,000 ALVE', 'Capped supply model from current source info.'],
  ['Block time', '60 seconds', 'Designed for better app/game UX than 10-minute blocks.'],
  ['Initial reward', '19.02587519 ALVE', 'Derived from 60M max supply and 1,576,800-block halving cycle.'],
  ['Halving interval', '1,576,800 blocks', 'Around three years at 60-second block target.'],
  ['Treasury/premine', 'Open decision', 'Genesis allocation, treasury and vesting must be explicit before public token claims.'],
]

export const faqItems = [
  ['Is Alvenqis mainnet available?', 'No public production Mainnet is claimed. The current chain and packages are Mainnet Candidate prototypes pending G4.'],
  ['Is the wallet available?', 'Candidate wallet tooling and the Tauri Control Center exist, but packages and recovery flows are not yet production-approved or signed.'],
  ['Is there a mining pool?', 'A reachable Mainnet Candidate pool prototype exists. It is not a production pool and still lacks production storage and an offline/HSM payout signer.'],
  ['Why not store everything on-chain?', 'Large files, messages, media and private data stay off-chain. The chain anchors settlement, ownership, permissions, commitments and hashes.'],
  ['What is Alvenqis Passport?', 'A planned proof layer for licenses, app access, ownership, reputation, achievements and authenticity without mandatory public KYC.'],
  ['What blocks public launch?', 'Independent genesis/security review, multi-host soak, production storage/operations, RPC abuse testing, signed artifacts and explicit G4 approval.'],
]
