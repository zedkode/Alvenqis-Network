# Alvenqis Network — init

> **Mainnet Candidate / Experimental Prototype**
> ALVE on this network is **not** real money. Do not treat balances as funds.

This file is the map of the repository after the 2026 layout restructure.
Start here, then open `PROJECT_STRUCTURE.md` for the complete public layout.
See also `llms.txt` for the short, spec-format pointer version used by external agent tooling.

## Where is the monorepo?

Git root:

```text
Alvenqis_Network/          ← this repository
```

## Top-level layout (read this first)

| Path | Purpose |
|------|---------|
| **`Blockchain-prototype/`** | Everything that is **implemented**: Rust crates, desktop, explorer, website, android, configs, shared assets. **Cargo workspace root.** |
| **`planned/`** | Product areas that exist only as placeholders / README stubs (not production code yet). |
| **`Blockchain-docs/`** | Public operator, protocol, architecture, API, mining, security, and release documentation. |
| **`Blockchain-scripts/`** | Operator / CI / local / release / security scripts. |
| **`InternalAI/`** | Agent-facing decentralization program (directive, audit, task-master addendum). Internal use — not public copy. See `InternalAI/00_DECENTRALIZATION_READ_FIRST.md`. |
| **`reform.md`** | Phase-by-phase technical scope/implementation detail for the decentralization + hardening work. Context document only — the real tracker is `Blockchain-docs/internal/TASK_MASTER.md`. |
| **`init.md`** | This file — orientation. |
| **`PROJECT_STRUCTURE.md`** | Public structure doc (safe for GitHub). |
| **`llms.md`** | Full LLM/agent briefing. |
| **`llms.txt`** | Short, spec-format pointer file for external agent tooling. |
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

## What is planned only

Under `planned/` — README stubs, **no** full implementation:

- benchmarks, contracts, faucet, governance, infra, marketplace
- monitoring, ops, passport, research, security suite, tests harness

See each folder's README for intent.

## Decentralization program (`InternalAI/`)

An agent-facing program correcting evidenced centralization gaps (single VPS, single unpinned P2P
seed, admin panel without role separation). Read order:

1. `InternalAI/00_DECENTRALIZATION_READ_FIRST.md`
2. `Blockchain-docs/internal/TASK_MASTER.md` (the real tracker — the addendum below adds to it, does
   not replace it)
3. `InternalAI/01_DECENTRALIZATION_DIRECTIVE.md`
4. `InternalAI/02_DECENTRALIZATION_AUDIT.md`
5. `InternalAI/04_TASK_MASTER_DECENTRALIZATION_ADDENDUM.md`
6. `reform.md` (repo root) — scope/implementation detail per phase; status columns there are
   illustrative, real status lives in `TASK_MASTER.md`

## Documentation map

| Need | Open |
|------|------|
| Setup / protocol / API / mining / security (public) | `Blockchain-docs/human/` |
| Release / maturity gates | `Blockchain-docs/human/release/` |
| Official task tracker (internal) | `Blockchain-docs/internal/TASK_MASTER.md` |
| Decentralization program (internal) | `InternalAI/` |
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
5. Never self-declare "decentralized", "audited", or "Mainnet" labels — these are gated by
   `NETWORK_MATURITY.md` §4 and an explicit owner decision only.

## Next reading

1. `README.md` — product + legal notice
2. `PROJECT_STRUCTURE.md` — full tree
3. `Blockchain-docs/human/SETUP.md` — operator setup
4. `InternalAI/00_DECENTRALIZATION_READ_FIRST.md` — if working on decentralization/hardening tasks
