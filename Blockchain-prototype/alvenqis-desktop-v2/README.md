# Alvenqis Control Center V2

**Status:** Mainnet Candidate / Prototype — UI/UX rebuild branch product
**Package:** `alvenqis-desktop-v2` (`2.0.0-candidate`)
**Base:** evolved from the original Control Center implementation (logic, Rust bridge, mining, wallet)

V2 is a **presentation-layer evolution**: denser shell, glass panels, motion, Analytics
suite, Messages inbox, richer navigation and settings surface — while reusing the same
Tauri commands, sidecars, and RPC snapshot model.

## What is new in V2

| Area | Change |
|---|---|
| Shell | Premium sidebar, top bar, footer, mesh background (`styles/v2-shell.css`) |
| Navigation | Groups: Command / Portfolio / Network / System |
| Analytics page | Multi-chart live suite (height, hashrate, mempool, peers, rewards) |
| Messages page | Inbox filters, unread badges, system/mining/security stream |
| Overview | V2 hero CTAs (Analytics, Messages), candidate honesty copy |
| Branding | Product name **Control Center V2**, identifier `control-center-v2` |

This directory is the canonical Control Center implementation in the public repository.

## Develop

```powershell
cd Blockchain-prototype\alvenqis-desktop-v2
npm install
npm run prepare:native
npm run tauri:dev
```

Typecheck:

```powershell
npm run lint
```

Validation without producing installers:

```powershell
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/Cargo.toml --all-targets
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
```

The public web Explorer is configured independently from the JSON RPC gateway under
**Settings -> Network -> Explorer**. The default is `https://dohotstudio.com/explorer`;
custom remote Explorer endpoints must use HTTPS.

## Full user guide

See **[DESKTOP_V2_USER_GUIDE.md](../../Blockchain-docs/human/operator/DESKTOP_V2_USER_GUIDE.md)** for:

- install, mining modes (solo RPC / verified Stratum TLS)
- modern console + allowlisted custom miner commands
- predictive errors, refresh cadence, security rules

## Notes

- Stack: **Tauri 2 + React 19 + Vite 7 + Rust** sidecars (current supported line).
- Native keystore / sidecars still prepared via `scripts/prepare-native.*`.
- Root `logo.png` is the canonical brand source used by the UI and icon generation.
- Windows builds use NSIS; Linux builds support deb, AppImage and rpm through Tauri.
- Do not claim Mainnet Live; labels stay **Mainnet Candidate**.
- Heavy `node_modules` / `src-tauri/target` are not committed — install and prepare locally.
- Stratum **client** is in `alvenqis-miner`; pool must expose `alvenqis-stratum-v1` for live mining over Stratum.
