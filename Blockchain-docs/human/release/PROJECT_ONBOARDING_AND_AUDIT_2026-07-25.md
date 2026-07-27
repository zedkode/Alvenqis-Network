# Alvenqis Network — Project Onboarding & Problem Audit

| | |
|---|---|
| **Date** | 2026-07-25 |
| **Audience** | New contributors, operators, AI agents |
| **Repo root** | `Alvenqis_Network/` |
| **Status label** | **Mainnet Candidate / Prototype** — not public Mainnet |
| **Related** | `CODE_REVIEW_REPORT_2026-07-19.md`, `docs/release/NETWORK_MATURITY.md`, `AGENTS.md` |

This document answers four questions:

1. What is this project?
2. How does it work?
3. What must you know before changing anything?
4. What problems still block public launch? (audit)

---

## 1. What is this project?

**Alvenqis Network** is a full **Layer 1 blockchain** monorepo written primarily in **Rust**, with TypeScript/React clients for desktop, explorer, and website.

It is **not**:

- an ERC-20 / Solana token
- a reskinned explorer on someone else's chain
- a production public mainnet (yet)

It **is**:

- an independent **account-based** ledger
- **Proof-of-Work** consensus using **FiroPoW 0.9.4** (NVIDIA CUDA miner only)
- a full stack: core → node → RPC → wallet → indexer → explorer → desktop Control Center → mining pool prototype → VPS control plane

### Native asset (protocol parameters)

| Parameter | Value |
|---|---|
| Ticker | **ALVE** |
| Address prefix | `ALVE` (mainnet-candidate), `dalve` (devnet), `talve` (testnet) |
| Ledger model | Account-based (not UTXO) |
| Block time target | 60 seconds |
| Max supply | 60,000,000 ALVE |
| Decimals | 8 (1 ALVE = 100,000,000 atomic) |
| Initial reward | 19.02587519 ALVE / block |
| Halving | every 1,576,800 blocks (~3 years at target) |
| Signing | Ed25519 + Bech32m addresses |
| PoW | FiroPoW 0.9.4, LWMA difficulty |

**Critical honesty rule:** ALVE balances on this network are **experimental**. They are not money, not an investment, and may be wiped by resets or protocol changes.

### Maturity ladder (do not skip)

| Gate | Meaning |
|---|---|
| G0 | Hygiene / secret scanners |
| G1 | Local release-gate (software hygiene for **rehearsal**) |
| G2 | Mainnet Candidate operator checklist |
| G3 | Security gate |
| G4 | Public Mainnet (requires external review + multi-host soak + signed go-live) |

**Green G1 ≠ launch approval.** Source of truth: `docs/release/NETWORK_MATURITY.md`.

---

## 2. How it works

### Logical layers

```text
┌─────────────────────────────────────────────────────────────┐
│  PRODUCT LAYER                                              │
│  Tauri Control Center · Explorer · Website · Android · SDK  │
├─────────────────────────────────────────────────────────────┤
│  ACCESS LAYER                                               │
│  RPC gateway · Indexer · Mining pool · Browser native host  │
├─────────────────────────────────────────────────────────────┤
│  NODE LAYER                                                 │
│  alvenqis-node · SQLite chain · Mempool · P2P (libp2p)      │
├─────────────────────────────────────────────────────────────┤
│  BASE / CONSENSUS                                           │
│  alvenqis-core · FiroPoW · fees · ledger · addresses        │
└─────────────────────────────────────────────────────────────┘
```

### Data flow (simplified)

1. **Wallet** creates a signed account-model transaction (nonce, max_fee, priority_fee).
2. **RPC** accepts submit (when enabled) → node **mempool**.
3. **Miner** (CUDA) pulls a template → finds FiroPoW solution → submits block.
4. **Node** revalidates PoW + txs in **core**, persists via **SQLite**, gossips over **P2P**.
5. **Indexer** builds searchable state; **Explorer** / **Control Center** read RPC + index.

### Sources of truth

| Concern | Owner |
|---|---|
| Protocol validity | `alvenqis-core` |
| Chain acceptance & storage | `alvenqis-node` |
| Constants / schemas / IDs | `shared/` |
| Intended behavior & maturity claims | `docs/` |
| Public messaging | `alvenqis-website` (not protocol truth) |
| Project memory for agents | `memory/` |
| Canonical product rules | `docs/source-info/` + `newrename/` package |

### Cargo workspace (Rust, built together)

| Crate | Role |
|---|---|
| `alvenqis-core` | Consensus, PoW, ledger, addresses |
| `alvenqis-node` | Storage, mempool, P2P, mining helpers |
| `alvenqis-rpc-gateway` | HTTP JSON API |
| `alvenqis-wallet` | CLI wallet |
| `alvenqis-sdk-rust` | Rust SDK |
| `alvenqis-browser/host` | Browser extension native messaging host |
| `alvenqis-indexer` | Chain index (JSON prototype storage) |
| `alvenqis-miner` | NVIDIA CUDA FiroPoW miner |
| `alvenqis-mining-pool` | Pool **prototype** |
| `alvenqis-mobile-core` | Mobile FFI |
| `alvenqis-release/vps-control-plane/admin-server` | Fleet admin agent |

### Outside root workspace (intentional)

| Tree | Tooling |
|---|---|
| `alvenqis-desktop-tauri` | **Product** Control Center (npm + Tauri) |
| `alvenqis-explorer` | Vite/React explorer |
| `alvenqis-website` | Public site + admin |
| `alvenqis-android` | Kotlin + mobile-core |
| Placeholder packages (`contracts`, `faucet`, `marketplace`, `passport`, …) | README / planned only |

### Environments

| Network | Data root (typical) | Address prefix | Notes |
|---|---|---|---|
| Devnet | `.alvenqis-dev/` | `dalve` | Reset allowed |
| Testnet | `.alvenqis-testnet/` | `talve` | Internal |
| Mainnet Candidate | `.alvenqis-mainnet/` or `.alvenqis-local/` | `ALVE` | Needs `allow_mainnet_candidate`; **not** public Mainnet |

Default public RPC for operators/clients often points at the rehearsal TLS host `https://rpcnode.dohotstudio.com` (loopback ports remain local-only on VPS).

---

## 3. What you need to know

### Non-negotiable rules (`AGENTS.md`)

1. Read `docs/source-info/` + `memory/` before architecture/protocol decisions.
2. English only for code, docs, APIs, public copy.
3. Never commit secrets, seeds, private keys, tokens.
4. **Do not modify** legacy `alvenqis-release/vps/` — use `vps-control-plane/` only.
5. Only claim features that exist, run, and are documented.
6. Allowed status words: Draft, Planned, Research, Private Devnet, Public Testnet, Mainnet Candidate, Coming Soon, Prototype, Experimental.
7. After meaningful work: validate, summarize, update `memory/`.

### What is implemented vs planned

**Runnable today (candidate/prototype):** core, node (SQLite), RPC, wallet CLI, CUDA miner, pool prototype, indexer, explorer, Tauri Control Center, VPS control-plane packaging.

**Not production-complete:** multi-host public seed topology, full P2P DoS resistance, RPC auth/rate limits, signed desktop/VPS updates, production pool (HSM payouts), external security audit.

**Explicit non-goals until G-level advances:** smart contracts, staking, DAO, marketplace, Passport, NFTs, CPU/OpenCL mining.

### Open product decisions (do not silently “fix” in code)

See `memory/OPEN_QUESTIONS.md` — e.g. VM choice, genesis/premine/treasury policy, production DB choices, 2,500 ALVE validator-role conflict (off-consensus vs hybrid stake).

### Rebrand note (as of 2026-07-25)

Branch `rebrand/alvenqis-network` may contain incomplete rename residue (`veiron-*` / `vireon-*` trees, logo churn). Prefer `alvenqis-*` paths; treat leftover brand trees as migration noise until cleaned.

---

## 4. How to start

### Prerequisites

- **Rust** (stable) + Cargo
- **Node.js** + npm (explorer, website, Tauri UI)
- **Windows PowerShell** or bash (operator scripts)
- Optional: **NVIDIA CUDA** toolchain for GPU mining
- Optional: Docker only on Linux VPS (not required for local Windows node stack)

### A. Read first (30–60 min)

```text
1. README.md
2. AGENTS.md
3. docs/release/NETWORK_MATURITY.md
4. memory/PROJECT_MEMORY.md
5. memory/NEXT_STEPS.md
6. docs/architecture/00_SYSTEM_OVERVIEW.md
```

### B. Build & test Rust workspace

```powershell
cd Alvenqis_Network
cargo test --workspace
```

G1 hygiene (rehearsal gate only):

```powershell
.\scripts\release\release-gate.ps1
```

### C. Local operator stack (node + RPC + explorer)

```powershell
cd Alvenqis_Network
.\scripts\local\alvenqis-local.ps1 start
.\scripts\local\alvenqis-local.ps1 status
.\scripts\local\alvenqis-local.ps1 mine
.\scripts\local\alvenqis-local.ps1 stop
```

Root entrypoint (packaged + monorepo aware):

```powershell
.\alvenqis.ps1 help
```

Local data typically under `.alvenqis-local/` (or `%LOCALAPPDATA%\Alvenqis\ControlCenter\.alvenqis-local` when packaged).

### D. Explorer only

```powershell
cd alvenqis-explorer
npm install
# set VITE_ALVENQIS_RPC_URL to local gateway if needed
npm run dev
```

### E. Desktop Control Center (product UI)

```powershell
cd alvenqis-desktop-tauri
npm install
# follow package README for Tauri + native sidecars
npm run tauri dev
```

### F. CUDA miner (requires NVIDIA GPU + wallet address)

```powershell
.\alvenqis.ps1 miner-start -MinerAddress "alve1..."
.\alvenqis.ps1 status
.\alvenqis.ps1 miner-stop
```

### G. Where to put new work

| Change type | Primary package |
|---|---|
| Consensus / PoW / fees | `alvenqis-core` |
| Storage / P2P / reorg | `alvenqis-node` |
| HTTP API | `alvenqis-rpc-gateway` |
| Wallet CLI secrets UX | `alvenqis-wallet` |
| Desktop UX | `alvenqis-desktop-tauri` |
| Pool prototype | `alvenqis-mining-pool` |
| VPS deploy | `alvenqis-release/vps-control-plane/` only |
| Constants shared by clients | `shared/constants/` |

---

## 5. Problem audit (prioritized)

Severity: **Critical** > **High** > **Medium** > **Low**.  
IDs align with `CODE_REVIEW_REPORT_2026-07-19.md` (`CR-*`) where still open; re-verified 2026-07-25 on current sources.

### 5.1 Overall readiness

| Zone | Score (0–5) | Note |
|---|:---:|---|
| Protocol / core | 3.5 | Real FiroPoW + account model; residual hash/liveness issues |
| Node storage | 3.5 | SQLite ACID progress; restore/disk-failure evidence incomplete |
| P2P | ~2.5–3.0 | Header-first + reputation; DoS caps incomplete |
| RPC | ~2.5–3.0 | Access modes improved; no in-process auth/rate limit |
| Wallet CLI | 2.5 | Encryption on candidate path; secret I/O still dangerous |
| Desktop Tauri | 3.5 | Best product path; code-signing incomplete |
| Mining pool | 2.0 | Prototype accounting/DoS issues |
| Browser host | 1.5 | Recovery + confirm defaults still unsafe |
| Android | 2.0 | Keystore without user-auth gate |
| VPS control-plane | 2.5 | Loopback patterns good; auto-update risk high |
| Docs honesty | 4.0 | Strong maturity discipline |

**Public Mainnet readiness: not ready (~2.5 / 5).**

### 5.2 Critical (P0)

| ID | Component | Problem | Impact | Fix direction |
|---|---|---|---|---|
| **CR-C01** | `alvenqis-browser/host` | `create_wallet` binds `_mnemonic` and **never returns recovery phrase** (`protocol.rs`) | Permanent fund loss if keystore lost | Refuse create without one-time OS recovery display / CLI-only create |
| **CR-C02** | `alvenqis-browser/host` | `require_os_confirm` defaults **false** | Malicious extension can sign/send after unlock | Default `true` for all sign/send |

### 5.3 High (P1)

| ID | Component | Problem | Impact |
|---|---|---|---|
| **CR-H01** | wallet CLI | Secrets on argv (`import-mnemonic` / keys) | Shell history / process list leak |
| **CR-H02** | wallet CLI | Mnemonic / key material printed to stdout | Scrollback / CI log leak |
| **CR-H03** | RPC | No app-level auth on submit/mining | Any client that reaches port can abuse |
| **CR-H04** | RPC | No in-process rate limiting | DoS if proxy misconfigured |
| **CR-H05** | RPC mining | Template cache fillable by distinct miner addresses | Miner DoS |
| **CR-H06** | node P2P | Staged headers without PoW can grow unbounded | Memory DoS |
| **CR-H07** | node | `import_genesis_block --force` can wipe arbitrary `--data-dir` | Destructive footgun |
| **CR-H08** | core | `Block::pow_hash` → `Hash::zero()` on error (`block.rs`) | False identity / tip confusion under failure |
| **CR-H09** | core | Post-subsidy empty blocks with coinbase must be `> 0` | Long-horizon liveness gap |
| **CR-H10** | pool | Bans claimable worker identities | Competitor DoS of miners |
| **CR-H11** | pool | Payout confirm may not bind `from == pool_address` | Fake “paid” using third-party transfers |
| **CR-H12** | VPS | Unattended auto-update (~15m) checksum-only | Compromised GitHub release → host takeover |
| **CR-H13** | desktop/VPS | Updates lack independent publisher signatures / Authenticode | Supply-chain trust = GitHub account |
| **CR-H14** | Android | Keystore decrypt without user authentication | Unlocked-device malware risk |

### 5.4 Medium (sample — see full code review)

| Theme | Examples |
|---|---|
| Consensus validation cost | Sigs before cheap PoW reject; no total block wire size cap |
| State growth | Unbounded applied-tx hash sets |
| Node durability | Tip crash recovery edge cases; path allowlist edge cases |
| P2P maturity | Gossip junk penalties, durable pre-adoption branch resume |
| Indexer | JSON snapshot storage vs production DB |
| Ops | Multi-host soak, SQLite restore/disk-failure drills, second-node enrolment |

### 5.5 Product / process risks (not pure code bugs)

1. **Mainnet Candidate mis-labeling** — operators or marketing calling rehearsal “live mainnet”.
2. **Rebrand residue** — incomplete `veiron`/`vireon` trees can confuse builds and agents.
3. **Open protocol decisions** — VM, treasury/premine, validator-bond design unresolved.
4. **Placeholder packages** — README-only crates look like products; they are not.
5. **G4 blockers still open** — independent genesis verification, multi-host soak, external security review, signed go-live decision.

### 5.6 Recommended fix order (practical)

1. **Wallet safety:** browser host CR-C01/C02; wallet CLI CR-H01/H02.
2. **Network abuse:** RPC auth + rate limits; P2P header caps (CR-H03–H06).
3. **Core correctness hygiene:** fallible block hash; post-subsidy coinbase (CR-H08/H09).
4. **Pool prototype:** do not publicize until worker auth + payout binding + HSM path (CR-H10/H11).
5. **Supply chain:** disable unattended VPS apply-by-default; add signatures (CR-H12/H13).
6. **Ops evidence:** SQLite restore drills, multi-host P2P soak, then G3/G4 paperwork.

### 5.7 What is already strong (keep)

- Honest maturity docs and claim discipline
- Real FiroPoW with core revalidation of miner candidates
- Account model + fee burn/tip split direction
- SQLite ACID progress on node storage
- RPC exposure modes (local mining off by default for public binds)
- Tauri as single Windows/Linux Control Center product path (Electron removed)
- Peer reputation / temporary bans foundation

---

## 6. Quick command cheat sheet

```powershell
# Tests
cargo test --workspace

# Local stack
.\scripts\local\alvenqis-local.ps1 start|status|mine|stop|smoke|logs

# Release hygiene (G1 only)
.\scripts\release\release-gate.ps1

# Operator maturity probe (if deployed)
node scripts/operator/maturity-health.mjs
```

---

## 7. Document maintenance

- When a CR-* item is fixed: update this file **and** `CODE_REVIEW_REPORT_2026-07-19.md` status, plus `memory/NEXT_STEPS.md`.
- When readiness changes: update `docs/release/NETWORK_MATURITY.md` first.
- Do not claim G4 progress without multi-host evidence and an explicit decision in `memory/DECISIONS.md`.
