# Blockchain-prototype

**Implemented** Alvenqis Network code. This directory is the **Cargo workspace root**.

```bash
cd Blockchain-prototype
cargo test --workspace
cargo build --workspace --release
```

| Crate / app | Role |
|-------------|------|
| `alvenqis-core` | Consensus primitives, FiroPoW, crypto |
| `alvenqis-node` | Full node |
| `alvenqis-rpc-gateway` | HTTP RPC |
| `alvenqis-indexer` | Indexer |
| `alvenqis-wallet` | Wallet CLI |
| `alvenqis-sdk-rust` / `alvenqis-sdk` | SDKs |
| `alvenqis-miner` | CUDA miner |
| `alvenqis-mining-pool` | Mining pool |
| `alvenqis-browser` | Extension + native host |
| `alvenqis-desktop-v2` | Control Center |
| `alvenqis-explorer` | Explorer UI |
| `alvenqis-website` | Website |
| `alvenqis-android` / `alvenqis-mobile-core` | Mobile |
| `alvenqis-release` | Packaging + VPS control plane |
| `configs/` | Network configs |
| `shared/` | Brand, constants, schemas |

Future product tracks remain documented as planned or deferred; no placeholder
folder is treated as implementation.
Scripts: `../Blockchain-scripts/`.
Docs: `../Blockchain-docs/`.
