# Product Layer

Status: Mainnet Candidate / mixed implementation maturity

## Current product map

| Surface | Status |
|---|---|
| Windows/Linux Control Center | Tauri 1.0.0 Mainnet Candidate; unsigned candidate packages |
| CUDA miner | Implemented NVIDIA CUDA-only FiroPoW miner |
| Wallet CLI / platform keystore | Prototype / candidate; production recovery and signing review incomplete |
| Explorer / Indexer / RPC | Implemented candidate services reading the node SQLite store; index snapshots and abuse hardening remain incomplete |
| Website/Admin | Implemented website and CMS prototype with honest candidate data boundaries |
| SDKs and examples | Prototype developer clients |
| Android | Wallet/monitor prototype; no local mining and no unauthenticated remote control |
| Browser extension | Native-host prototype; not store-ready |
| Mining pool | Off-chain prototype; no production payout signer/storage |
| Passport, marketplace, contracts, staking, DAO | Planned or research; not live |

## Product rules

- Tauri is the only desktop Control Center release path.
- Windows and Linux use the same product architecture; platform-specific
  keystore and packaging boundaries remain native.
- Android is not a local node/miner product.
- User-facing values must come from real RPC/indexer/miner/pool data or show an
  explicit unavailable/planned state.
- A product page or reserved folder does not prove a feature exists.
- Remote node/miner control requires a separately approved authenticated,
  permissioned, auditable API.
