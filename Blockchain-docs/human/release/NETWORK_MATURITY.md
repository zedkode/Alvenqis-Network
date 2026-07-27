# Network Maturity and Release Readiness

Status: **Draft / Mainnet Candidate / Prototype — not a public live Mainnet**

This document is the single source of truth for **what the repository is allowed to claim** and **what each gate authorizes**.  
It exists so operators, auditors and agents do not treat a green CI or release-gate run as a production launch.

---

## 1. Labels (use exactly)

| Label | Meaning | Public launch? |
|---|---|---|
| **Draft** | Specs or code in progress; behavior may change | No |
| **Prototype** | Runnable for local / controlled rehearsal | No |
| **Mainnet Candidate** | Configured like a future mainnet (`ALVE`, ports, genesis pin) for **operator rehearsal only** | **No** |
| **Public Testnet** | Independent public test network (not currently operated as product default) | N/A today |
| **Mainnet (live)** | Public production network with independent review and multi-host soak | Only after §4 gates |

**Forbidden shorthand:** calling the current chain “mainnet”, “production mainnet”, or “live network” without the §4 public-launch gates.

**Allowed shorthand:** “Mainnet Candidate”, “candidate chain”, “rehearsal VPS”, “local operator stack”.

---

## 2. What exists today (honest)

Runnable under Mainnet Candidate / Prototype:

- `alvenqis-core` consensus and FiroPoW 0.9.4 validation
- `alvenqis-node` + transactional SQLite storage + P2P (libp2p) prototype
- `alvenqis-rpc-gateway` (loopback / reverse-proxy TLS patterns)
- `alvenqis-wallet` CLI, Control Center (**Tauri** is the product target)
- `alvenqis-miner` (NVIDIA CUDA-only FiroPoW; no CPU/OpenCL fallback) against RPC or pool
- `alvenqis-mining-pool` **prototype** (not a public production pool)
- `alvenqis-indexer` / explorer / website (candidate-scoped)
- Optional VPS control-plane packaging under `alvenqis-release/vps-control-plane/`

Still **not** production-complete (non-exhaustive):

- Independent multi-host soak + public seed topology
- Independent node SQLite backup/restore, disk-failure, and multi-host durability review
- Full peer scoring / ban policy operational maturity
- Production mining-pool (HSM payout, multi-instance admission)
- External independent security review of the live deploy path
- Cryptographically signed desktop packages and signed update metadata (the app now requires approval and verifies release `SHA256SUMS`, but that file is not yet independently signed)
- Smart contracts, staking, DAO, marketplace, Passport, NFTs (explicit non-goals)

### Maturity progress (engineering — 2026-07-17+)

| Area | Candidate-class now | Still open for G4 |
|---|---|---|
| Node storage | SQLite strict schema; ACID tip/reorg transactions; WAL + `synchronous=FULL`; hash/link checks; JSONL migration; online backup + integrity check | Independent restore/disk-failure drills and multi-host soak |
| Node reorg | `adopt_candidate_chain` + mempool reconcile + P2P staged fork + **header-first**; detached blocks archived transactionally | Durable resume of pre-adoption branches and deep-reorg recovery |
| Pool payouts | Confirm requires **on-chain tx lookup** covering each miner amount | Offline/HSM signer, multi-coordinator |
| Peer scoring | Score + temporary bans (persist `peer-reputation.json`), refuse banned peers | Distributed ban lists, IP-level DDoS edge |
| Indexer | Tip-hash rebuild, atomic `index.json`, dedicated timer writer, read-only RPC cache, bounded overview/pagination | Incremental detach, continuous daemon as default product |
| RPC multi-client | Serialized chain-cache refresh, cached tip/work metadata, bounded index routes, `/status` lag fields | In-process rate limits, durable cache, production load test |
| Pool | Immature→Mature after N confs; **Mature re-check on reorg** claws unpaid mature | Shared admission, HSM signer, production storage |
| Developer surface | `@alvenqis/sdk` read client + examples | Signed-tx helpers without key custody |

---

## 3. Gate ladder (what each check allows)

| Gate | Command / doc | Authorizes | Does **not** authorize |
|---|---|---|---|
| **G0 — Hygiene** | secret / hygiene / config scanners | Continue development | Deploy, launch |
| **G1 — Local release gate** | `scripts/release/release-gate.ps1` / `.sh` · `docs/release/RELEASE_GATE.md` | Ship **artifacts** and **docs consistency** for Mainnet Candidate **rehearsal** | Public mainnet; “production ready” claims |
| **G2 — Mainnet Candidate checklist** | `docs/release/MAINNET_CANDIDATE_CHECKLIST.md` | Operator rehearsal (local / controlled VPS) with candidate configs | Public mainnet |
| **G3 — Security gate** | `docs/security/SECURITY_GATE.md` | Security baseline for candidate | External audit sign-off |
| **G4 — Public launch** | §4 below | First public mainnet claim | — |

### G1 output language

Scripts must print that G1 is a **candidate rehearsal gate**, not a launch approval.  
See `RELEASE_GATE.md` and the release-gate scripts.

---

## 4. Public Mainnet launch requirements (G4)

All of the following must be true and documented **outside** this draft approval alone:

1. **Independent genesis verification** (not only in-repo `GENESIS_APPROVAL.*.json`).
2. **Multi-host soak** of encrypted P2P with public/bootstrap seeds and measured reorg behavior.
3. **Production storage and ops** review (backup, restore, disk failure, monitoring, incident runbook).
4. **RPC / mining abuse** testing (rate limits, auth boundaries, public surface minimalism).
5. **External security review** of node, RPC, wallet keystore and deploy packaging.
6. **Explicit go-live decision** recorded in `memory/DECISIONS.md` and public docs with date and signatories.
7. Network label in configs/UI switches from **Mainnet Candidate** to **Mainnet** only after (1)–(6).

Until then:

- Product UIs and RPC `status_label` remain **Mainnet Candidate** / Prototype.
- Pool remains **prototype** even if reachable on a rehearsal host.
- VPS endpoints used for development are **rehearsal**, not “live mainnet”.

---

## 5. Documentation map

| Document | Role |
|---|---|
| **This file** | Maturity labels + gate ladder (source of truth) |
| `RELEASE_GATE.md` | How to run G1; what pass means |
| `MAINNET_CANDIDATE_CHECKLIST.md` | G2 operator/config checklist |
| `GENESIS.md` + review/approval JSON | Genesis pin for candidate only |
| `docs/security/*` | Security gates and secret policy |
| Root `README.md` | Human-facing status banner linking here |
| `AGENTS.md` | Allowed public claim statuses for agents |
| `Blockchain-docs/human/operator/CHAIN_MATURITY_OPS.md` | Task 3 operator drills (SQLite restore, reorg, soak checklist) — **not** G4 approval |

---

## 6. Change control

When readiness improves:

1. Update this file first (labels and remaining blockers).
2. Update checklist items that became true.
3. Update root `README.md` status line only if it still matches this file.
4. Never remove the “not public mainnet” language until G4 is complete.

Last reviewed: 2026-07-19 (FiroPoW CUDA-only miner parity, bounded RPC/index polling, approved SHA-256-verified updater, 1.0.0 candidate preparation; G4 remains incomplete).
