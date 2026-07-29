# Alvenqis Network — init

> **Mainnet Candidate / Experimental Prototype**
> ALVE on this network is **not** real money. Do not treat balances as funds.

This file is the map of the repository after the 2026 layout restructure.
Start here, then open `PROJECT_STRUCTURE.md` for the complete public layout.

## Where is the monorepo?

Git root:

```text
Alvenqis_Network/          ← this repository
```

## Top-level layout (read this first)

| Path | Purpose |
|------|---------|
| **`Blockchain-prototype/`** | Everything that is **implemented**: Rust crates, desktop, explorer, website, android, configs, shared assets. **Cargo workspace root.** |
| **`Blockchain-docs/`** | Public operator, protocol, architecture, API, mining, security, and release documentation. |
| **`Blockchain-scripts/`** | Operator / CI / local / release / security scripts. |
| **`init.md`** | This file — orientation. |
| **`PROJECT_STRUCTURE.md`** | Public structure doc (safe for GitHub). |
| **`README.md`** | Project overview and legal/experimental notices. |
| **`.github/`** | CI workflows. |

## Quick commands

```powershell
# Rust workspace (from repo root)
cd Blockchain-prototype
cargo test --workspace
cargo build -p alvenqis-node --release

# Desktop Control Center
cd Blockchain-prototype\alvenqis-desktop-v2
npm install
npm run prepare:native:sidecars
npm run tauri:dev

# Local stack helpers
cd ..\..\Blockchain-scripts\local
.\start-all.ps1
.\status-all.ps1

# Browser native host
cd ..\browser
.\register-native-host.ps1 -ExtensionId <id> -Build
```

## What is already built (prototype)

Under `Blockchain-prototype/`:

| Component | Path | Notes |
|-----------|------|--------|
| Consensus / PoW / crypto | `alvenqis-core` | FiroPoW 0.9.4, account model, ALVE |
| Node | `alvenqis-node` | Chain, mempool, P2P |
| RPC | `alvenqis-rpc-gateway` | HTTP API, mining routes, profiles |
| Indexer | `alvenqis-indexer` | Address / block indexes |
| Wallet CLI | `alvenqis-wallet` | Keys, transfers |
| SDKs | `alvenqis-sdk-rust`, `alvenqis-sdk` | Rust + TypeScript clients |
| GPU miner | `alvenqis-miner` | CUDA FiroPoW |
| Pool | `alvenqis-mining-pool` | Share coordinator |
| Browser extension + host | `alvenqis-browser` | Native messaging host (no keys in extension) |
| Desktop Control Center | `alvenqis-desktop-v2` | Tauri app |
| Explorer | `alvenqis-explorer` | Vite React UI |
| Website | `alvenqis-website` | Marketing / info site |
| Android scaffold | `alvenqis-android` + `alvenqis-mobile-core` | Mobile bridge |
| VPS control plane | `alvenqis-release/vps-control-plane` | Docker stack, admin |
| Network configs | `configs/` | local + mainnet-candidate |
| Shared brand/constants | `shared/` | logos, schemas, constants |

## Planned work

Future product tracks remain planned or deferred in the public roadmap. A
folder or UI concept is not treated as implementation.

## Documentation map

| Need | Open |
|------|------|
| Setup / protocol / API / mining / security (public) | `Blockchain-docs/human/` |
| Release / maturity gates | `Blockchain-docs/human/release/` |
| Decentralization readiness | `Blockchain-docs/human/release/DECENTRALIZATION_READINESS.md` |
| Known security limitations | `Blockchain-docs/human/security/KNOWN_LIMITATIONS.md` |
| Repo map for GitHub | `PROJECT_STRUCTURE.md` |

## Scripts map

| Area | Path |
|------|------|
| Local chain smoke | `Blockchain-scripts/local/` |
| Devnet | `Blockchain-scripts/dev/` |
| Browser host install | `Blockchain-scripts/browser/` |
| Release / installers | `Blockchain-scripts/release/` |
| Security hygiene | `Blockchain-scripts/security/` |
| Git helpers | `Blockchain-scripts/git/` |
| Path helpers | `Blockchain-scripts/lib/repo-paths.ps1` / `.sh` |

## Network facts (do not invent alternatives)

- **network_id:** `alvenqis-mainnet-candidate`
- **address HRP:** `alve`
- **PoW:** FiroPoW 0.9.4 (AlvenqisPoW v1)
- **Public RPC (candidate):** `https://rpcnode.dohotstudio.com`
- **Status:** Mainnet Candidate — experimental, not production finance

## Security rules (non-negotiable)

1. Never commit secrets, mnemonics, keystores, or `.env` files.
2. Browser host: recovery phrase is **host-only**; OS confirm for sign/send is **default on**.
3. Do not reintroduce Veiron/Vireon identity strings into user-facing or wire protocol surfaces.
4. Do not mine/re-mine genesis on the RPC hot path.
5. Do not describe the network as decentralized, audited, or public Mainnet
   without the evidence and approvals in `NETWORK_MATURITY.md`.

## Next reading

1. `README.md` — product + legal notice
2. `PROJECT_STRUCTURE.md` — full tree
3. `Blockchain-docs/human/SETUP.md` — operator setup
4. `Blockchain-docs/human/release/DECENTRALIZATION_READINESS.md` — current centralization register
