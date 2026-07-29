# Local Runbook

Status: Draft / Mainnet Candidate / Prototype

This runbook is for local operator use only. It is not a VPS deployment guide and it does not imply any live public network status.

## Local Layout

The local operator workflow uses one safe workspace-local root:

```text
.alvenqis-local/
  chain/
  mempool/
  indexer/
  wallets/
    signed-txs/
  logs/
  backups/
  build/
```

Notes:
- wallet private keys stay under `.alvenqis-local/wallets/`;
- signed transaction files stay under `.alvenqis-local/wallets/signed-txs/`;
- logs stay under `.alvenqis-local/logs/`;
- local Cargo build artifacts are redirected into `.alvenqis-local/build/` so normal repo-hygiene gates are not polluted.

## Prerequisites

- Rust with `cargo`, `rustfmt` and `clippy`
- Node.js with npm
- run commands from the repository root

Windows note:
- the local scripts prefer the rustup-managed cargo shim if it exists at `%USERPROFILE%\.cargo\bin\cargo.exe`.

## Quick Start

PowerShell:

```powershell
.\Blockchain-scripts\local\start-all.ps1
```

Shell:

```bash
bash Blockchain-scripts/local/start-all.sh
```

What starts:
- `alvenqis-node` in local mainnet-candidate mode
- `alvenqis-rpc-gateway` bound to `127.0.0.1:10787`
- a one-shot `alvenqis-indexer` refresh
- `alvenqis-explorer` dev server if the app exists

## Health Checks

Show local status:

```powershell
.\Blockchain-scripts\local\status-all.ps1
```

Key checks:
- node runtime state
- chain validation
- mempool summary
- RPC `/health`
- RPC `/network`
- latest block view
- SQLite index status
- managed log paths

## Mining One Local Block

PowerShell:

```powershell
.\Blockchain-scripts\local\mine-local-block.ps1
```

Shell:

```bash
bash Blockchain-scripts/local/mine-local-block.sh
```

This mines one block using the local operator chain and refreshes the SQLite
index after the block is written.

## Wallet Flow

Create a local wallet:

```powershell
cargo --manifest-path Blockchain-prototype/Cargo.toml run -p alvenqis-wallet -- --network mainnet-candidate --wallet-dir .alvenqis-local/wallets --signed-tx-dir .alvenqis-local/wallets/signed-txs create-wallet
```

Show the local address:

```powershell
cargo --manifest-path Blockchain-prototype/Cargo.toml run -p alvenqis-wallet -- --network mainnet-candidate --wallet-dir .alvenqis-local/wallets address
```

Check a balance through local RPC:

```powershell
cargo --manifest-path Blockchain-prototype/Cargo.toml run -p alvenqis-wallet -- --network mainnet-candidate --wallet-dir .alvenqis-local/wallets --rpc-base-url http://127.0.0.1:10787 balance <address>
```

Submit a signed transaction if supported:

```powershell
cargo --manifest-path Blockchain-prototype/Cargo.toml run -p alvenqis-wallet -- --network mainnet-candidate --wallet-dir .alvenqis-local/wallets --signed-tx-dir .alvenqis-local/wallets/signed-txs --rpc-base-url http://127.0.0.1:10787 submit-tx --tx-file .alvenqis-local/wallets/signed-txs/<tx-hash>.json
```

## Explorer

The explorer reads only the local RPC gateway.

Start through the local wrapper:

```powershell
.\Blockchain-scripts\local\start-all.ps1
```

Or manually:

```powershell
cd Blockchain-prototype/alvenqis-explorer
npm install
$env:VITE_ALVENQIS_RPC_URL = "http://127.0.0.1:10787"
npm run dev -- --host 127.0.0.1 --port 4173
```

## Logs

Local log files are written to:

```text
.alvenqis-local/logs/
```

Expected files:
- `node.log` / `node.err.log`
- `rpc.log` / `rpc.err.log`
- `explorer.log` / `explorer.err.log`
- `indexer-refresh.log` / `indexer-refresh.err.log`

## Backup

Create a backup:

```powershell
.\Blockchain-scripts\local\backup-local-chain.ps1
```

For Task 3 **SQLite online backup → isolated restore** evidence (integrity-checked,
writes `maturity-evidence/`), use:

```powershell
powershell -File Blockchain-scripts\operator\sqlite-restore-drill.ps1
```

See [CHAIN_MATURITY_OPS.md](CHAIN_MATURITY_OPS.md).

By default the backup includes:
- chain data
- mempool data
- SQLite index
- local logs
- local genesis marker if present

By default the backup does not include:
- wallet private keys
- wallet JSON files

## Safe Reset

PowerShell:

```powershell
.\Blockchain-scripts\local\reset-local-chain.ps1
```

Shell:

```bash
bash Blockchain-scripts/local/reset-local-chain.sh
```

Behavior:
- stops managed local processes first
- creates a backup automatically unless `--no-backup` is explicitly passed
- clears local chain, mempool and SQLite index
- keeps wallet material in place unless you remove it manually

## Local Smoke Test

PowerShell:

```powershell
.\Blockchain-scripts\local\run-local-smoke-test.ps1
```

Shell:

```bash
bash Blockchain-scripts/local/run-local-smoke-test.sh
```

The smoke test covers:
- release gate or basic validation
- node startup
- chain validation
- one local block mined
- RPC `/health`
- RPC `/network`
- wallet create and address display
- SQLite index refresh
- explorer build if the app exists
