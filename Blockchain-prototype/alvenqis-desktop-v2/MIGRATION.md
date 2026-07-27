# Desktop Control Center — Tauri (product path)

## Decision

`alvenqis-desktop-tauri` is the **only** desktop Control Center. The former Electron tree (`alvenqis-desktop-electron`) has been **removed** from the repository so it cannot be confused with Tauri packaging or release.

## Completed

- [x] Scaffold Tauri 2 + React + Vite
- [x] Port Control Center pages + design system
- [x] `window.alvenqis` → Tauri `invoke` bridge
- [x] Rust: network snapshot, seeds, wallet, txs, operator, logs, paths
- [x] Full Settings (12 sections) + `settings.json` persistence
- [x] Theme / density / accent / reduce-motion
- [x] Keystore helper under `native/keystore-helper`
- [x] Stage helper into `src-tauri/binaries` via `prepare-native`
- [x] Mined-block notifications + sound (settings-gated)
- [x] Mining defaults + live log cadence from Settings
- [x] Operator confirmations for destructive commands
- [x] Packaging: externalBin + resources + NSIS/MSI (Windows) + deb/AppImage/rpm (Linux)
- [x] Update service skeleton with honest unavailable/disabled phases
- [x] Remove Electron desktop tree and keystore fallback paths

## Next

1. [ ] End-to-end QA: wallet create/import, funded transfer, miner start/stop, multi-peer
2. [ ] Visual QA at 100% / 125% / 150% scaling
3. [ ] Signed Tauri updater feed + `createUpdaterArtifacts`
4. [ ] CI job for `prepare-native` + `tauri build` (smoke)
5. [ ] Linux packaging smoke on Ubuntu/Debian/Arch hosts

## Run

```powershell
cd alvenqis-desktop-tauri
npm install
npm run tauri:dev
```

## Build (release)

| Platform | Entry |
|---|---|
| Windows | `scripts/release/build-windows-installer.ps1` |
| Linux | `scripts/release/build-linux-desktop.sh` · `docs/operator/LINUX_DESKTOP.md` |
