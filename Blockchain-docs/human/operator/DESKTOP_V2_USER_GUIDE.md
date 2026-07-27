# Alvenqis Control Center V2 — User Guide

Status: **Mainnet Candidate / Prototype** — not public Mainnet Live
Package: `Blockchain-prototype/alvenqis-desktop-v2`
Version: `2.0.0-candidate`

This guide covers install, daily operation, mining modes (local solo / verified Stratum TLS),
terminals, troubleshooting, and operator safety.

---

## 1. What V2 is

| Layer | Tech (current stack) |
|---|---|
| Desktop shell | **Tauri 2** + Rust |
| UI | **React 19** + **Vite 7** + TypeScript 5.8 |
| Icons | **lucide-react** |
| Local mining | `alvenqis-miner` (CUDA FiroPoW 0.9.4 only) |
| Work sources | Local solo RPC · **Stratum v1 over verified TLS** |

V2 reuses the same consensus and sidecars as V1; it adds a premium UI shell, Analytics,
Messages, faster refresh defaults, modern consoles, custom miner CLI commands, and Stratum.

---

## 2. Install & run

### Prerequisites

- Windows 11 (or Linux for AppImage/deb builds)
- **Node.js 20+**
- **Rust** stable (for Tauri / native rebuilds)
- **NVIDIA GPU + driver** for mining (CUDA-only product miner)
- Optional: CUDA toolkit when rebuilding the miner sidecar

### First run

```powershell
cd Blockchain-prototype\alvenqis-desktop-v2
npm install
npm run prepare:native
# optional full sidecars:
# npm run prepare:native:sidecars
npm run tauri:dev
```

Typecheck:

```powershell
npm run lint
```

Production build (Windows):

```powershell
npm run tauri:build:windows
```

---

## 3. First-time wallet & startup

1. Startup gate: create or import a wallet (**alve1…** Mainnet Candidate).
2. Keys stay on this device (OS keystore / vault) — never paste seeds into chat or tickets.
3. Continue to the shell when the public (or local) RPC is reachable.
4. Default gateway: `https://rpcnode.dohotstudio.com` (submit/read; mining disabled publicly).

Honesty: **Mainnet Candidate ≠ live Mainnet.**

---

## 4. Navigation (V2)

| Group | Pages |
|---|---|
| Command | Overview, **Analytics**, **Messages** |
| Portfolio | Wallet, Send & Receive, Rewards, Assets |
| Network | Miner, Pool, Explorer, Blocks, Transactions, Mempool, Network |
| System | Activity, Settings |

- **Ctrl+K** — command palette
- **Ctrl+R** — refresh telemetry
- **Ctrl+1…9** — jump to primary pages

---

## 5. Refresh cadence (faster in V2)

| Setting | Default V2 | Notes |
|---|---|---|
| Snapshot poll | **6s** | Remote floor **5s**; local floor **1.5s** |
| Live logs (idle) | **2s** | Disk tail only |
| Miner console (active) | **~0.8s** | While miner process is running |

Tune under **Settings → General**. If you hit rate limits, the app backs off automatically.

---

## 6. Mining modes

### 6.1 Local solo RPC

- Work: `{mining_rpc}/mining/template`
- Needs a local stack (`expose_mining` / `access_mode=local`) on loopback
- Public `rpcnode` keeps mining **disabled**
- The desktop rejects remote solo mining RPC URLs

### 6.2 Stratum TLS (`alvenqis-stratum-v1`)

New in V2 miner + desktop:

```toml
[source]
kind = "stratum"
host = "stratum.dohotstudio.com"
port = 3333
use_tls = true
skip_tls_verify = false
worker_name = "desktop-01"
password = ""
timeout_seconds = 20
```

**Wire protocol (line-delimited JSON-RPC):**

| Method | Direction | Purpose |
|---|---|---|
| `mining.subscribe` | client → server | Agent + protocol id |
| `mining.authorize` | client → server | `alve1….worker` + password |
| `mining.get_work` | client → server | Returns full `alvenqis-mining-v1` template |
| `mining.notify` | server → client | Push template updates |
| `mining.submit` | client → server | Share / block submission |

This is **not** Bitcoin Stratum. Templates remain FiroPoW / Alvenqis mining-v1 objects.

**Status:** client and pool server are implemented. The official endpoint requires TLS,
uses a publicly valid certificate, and rejects disabled certificate verification.

Desktop: **Miner → Work source → Stratum TLS**.

The HTTPS pool endpoint remains available for read-only statistics. Work and share
submission routes (`/api/v1/work`, `/api/v1/shares`) are retired.

---

## 7. Miner console (modern)

The Miner page embeds a V2 console with:

- Severity coloring (cmd / status / ok / warn / error)
- Text filter + severity filter
- Auto-scroll, copy, export, clear
- **Interactive command line** (allowlisted)

### Allowlisted CLI verbs

| Command | Meaning |
|---|---|
| `status` | Fetch/validate one work template |
| `devices` | List CUDA devices |
| `config` / `config validate` | Show / validate miner config |
| `benchmark --seconds N` | Short CUDA benchmark |
| `help` | Help text from miner binary |

Shell metacharacters (`; | & $ > <` …) are **refused**.

Presets and history are stored in settings (`miner_custom_commands`).

---

## 8. Predictive errors

Failed miner / gateway actions append a `[predictive]` block with:

- Machine code (`MINING_PATH_DISABLED`, `CUDA_DEVICE`, `STRATUM_CONNECT`, …)
- Plain-language summary
- Ordered recovery actions

Also surfaced via toast / Messages inbox for high severity.

---

## 9. Messages & notifications

- **Messages** page: inbox with filters (unread / mining / system / error / security)
- Toast stack for short-lived feedback
- Optional OS notifications for mined blocks / updates (Settings → Notifications)

---

## 10. Analytics

Live multi-series charts (height, hashrate, mempool, peers, rewards, gauges).
Data is **only** from the real snapshot — never invented.

---

## 11. Settings map

| Section | Contents |
|---|---|
| General | Language, refresh cadence, startup |
| Appearance | Theme, density, accent, reduce motion |
| Network | RPC URL, mining RPC |
| Mining defaults | Mode, intensity, pool list, worker, Stratum fields |
| Wallet & security | Identity, recovery flows |
| Notifications | Blocks, sound, updates |
| Data & paths | Workspace roots |
| Privacy | Hide balances, mask addresses |
| Advanced / Danger | Operator confirms, disconnect |

---

## 12. Troubleshooting

| Symptom | Check |
|---|---|
| Login / wallet fails | Create alve1 wallet; no foreign HRPs |
| Solo template 4xx | Public mining is disabled — use local loopback solo or Stratum TLS |
| 0 H/s after start | DAG warm-up 15–20s on GPU; check console for `building_dag` |
| No CUDA devices | NVIDIA driver + CUDA sidecar (`devices` command) |
| Stratum connect fails | Verify `stratum.dohotstudio.com:3333`, DNS, firewall and certificate |
| Rate limited | Raise refresh interval; use local RPC for dev |
| Grafana / VPS admin | Credentials only on VPS `/home/credentials.md` (not in Git) |

Related ops docs:

- `VPS_REHEARSAL_OPS.md`
- `PRIVATE_MINING_OPS.md`
- `CHAIN_MATURITY_OPS.md`
- `NETWORK_MATURITY.md` (G0–G4 honesty)

---

## 13. Security rules

1. Never commit `.env`, seeds, keystores, or `/home/credentials.md`.
2. No free-form shell from the miner console — allowlist only.
3. Product miner is **CUDA-only** (no CPU/OpenCL claims).
4. Do not enable public mining on the public-submit RPC profile.
5. Remote Stratum always verifies TLS; bypass is allowed only for loopback development.

---

## 14. Developer notes

| Path | Role |
|---|---|
| `src/styles/v2-shell.css` | Premium shell |
| `src/styles/modern-console.css` | Console chrome |
| `src/components/console/ModernConsole.tsx` | Terminal UX |
| `src/shared/errorPrediction.ts` | Predictive failures |
| `src-tauri/src/process.rs` | Miner start + allowlisted CLI |
| `alvenqis-miner/src/stratum.rs` | Stratum TCP/TLS client |

Regenerate icons: `npm run icons`
Tests: `npm test` · `npm run lint`

---

## 15. Roadmap honesty

| Feature | V2 status |
|---|---|
| Premium UI shell | Shipped |
| Analytics / Messages | Shipped |
| Faster refresh | Shipped |
| Modern miner console + custom commands | Shipped |
| Stratum client (TCP/TLS) | Shipped in miner |
| Stratum **server** on pool | Shipped; enabled by the VPS `pool` profile |
| External security audit / G4 | Open |

Until G4 evidence and sign-off, keep all product labels on **Mainnet Candidate / Prototype**.
