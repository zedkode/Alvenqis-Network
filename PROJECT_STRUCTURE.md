# Alvenqis Network — project structure

Public repository layout for humans and GitHub.
For a short orientation, see [`init.md`](./init.md).

```text
Alvenqis_Network/                          # git root
├── README.md                              # product overview + experimental notices
├── init.md                                # where to find everything
├── PROJECT_STRUCTURE.md                   # this file
├── .gitignore
├── .github/                               # CI workflows
│
├── Blockchain-prototype/                  # IMPLEMENTED code (Cargo workspace root)
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── clippy.toml
│   ├── VERSION
│   ├── configs/                           # node + RPC TOML configs
│   ├── shared/                            # brand, constants, schemas
│   ├── alvenqis-core/                     # consensus, PoW, crypto, types
│   ├── alvenqis-node/                     # full node
│   ├── alvenqis-rpc-gateway/              # HTTP RPC
│   ├── alvenqis-indexer/
│   ├── alvenqis-wallet/                   # CLI wallet
│   ├── alvenqis-sdk-rust/
│   ├── alvenqis-sdk/                      # TypeScript SDK
│   ├── alvenqis-miner/                    # CUDA FiroPoW miner
│   ├── alvenqis-mining-pool/
│   ├── alvenqis-browser/                  # extension + native host
│   ├── alvenqis-desktop-v2/               # Control Center (own Tauri workspace)
│   ├── alvenqis-explorer/
│   ├── alvenqis-website/
│   ├── alvenqis-examples/
│   ├── alvenqis-devnet/
│   ├── alvenqis-docker/
│   └── alvenqis-release/                  # packaging + Alvenqis Setup External
│
├── Blockchain-docs/
│   ├── human/                             # public human documentation site sources
│   │   ├── protocol/
│   │   ├── architecture/
│   │   ├── api/
│   │   ├── mining/
│   │   ├── operator/
│   │   ├── security/
│   │   ├── release/
│   │   ├── tokenomics/
│   │   └── ...
│
└── Blockchain-scripts/
    ├── lib/                               # shared path helpers
    ├── local/                             # start/stop local stack
    ├── dev/                               # devnet helpers
    ├── browser/                           # native host registration
    ├── release/                           # installers + release gate
    ├── security/                          # secret/hygiene checks
    ├── git/
    ├── github/
    ├── operator/
    └── pipeline/                          # optional multi-agent PR helpers
```

## Cargo workspace

- **Root:** `Blockchain-prototype/Cargo.toml`
- **Members:** core, node, rpc-gateway, wallet, sdk-rust, browser/host, indexer, miner, mining-pool, vps-admin
- **Not workspace members (on purpose):** desktop-v2 (own Tauri workspace), explorer/website (npm)

```bash
cd Blockchain-prototype
cargo test --workspace
cargo build -p alvenqis-rpc-gateway --release
```

## Frontends

| App | Path | Stack |
|-----|------|--------|
| Control Center | `Blockchain-prototype/alvenqis-desktop-v2` | Tauri 2 + React/Vite |
| Explorer | `Blockchain-prototype/alvenqis-explorer` | Vite + React |
| Website | `Blockchain-prototype/alvenqis-website` | Vite + React |
| Browser extension | `Blockchain-prototype/alvenqis-browser/extension` | MV3 |

## Scripts entrypoints

| Task | Script |
|------|--------|
| Local all-in-one | `Blockchain-scripts/local/start-all.ps1` |
| Release gate G1 | `Blockchain-scripts/release/release-gate.ps1` / `.sh` |
| Register browser host | `Blockchain-scripts/browser/register-native-host.ps1` |
| Windows installer | `Blockchain-scripts/release/build-windows-installer.ps1` |

## What is not in git (by design)

See `.gitignore`. Highlights:

- `target/`, `node_modules/`, `dist/`, `release-artifacts/`
- secrets / keystores / `.env`
- local development-tool state and private review material
- private task/decision material and agent-only instructions

## Status labels

| Area | Status |
|------|--------|
| L1 core + node + RPC + miner + pool + wallet + SDKs | Implemented (Mainnet Candidate) |
| Desktop Control Center | Implemented (prototype UX) |
| Explorer / website | Implemented (prototype) |
| Browser host | Implemented (security-hardened) |
| Future product tracks | Planned/deferred in roadmap and the private task tracker; no placeholder package is treated as implementation |

## Naming

- Project: **Alvenqis Network**
- Unit: **ALVE**
- Address prefix: **alve**
- Network id: **alvenqis-mainnet-candidate**
- Do **not** use Veiron / Vireon branding in new code or docs.
