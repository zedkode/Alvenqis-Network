# Operator Commands

Status: Draft / Mainnet Candidate / Prototype

This document is the detailed operator command reference for local Alvenqis work.

Scope:
- local node;
- local RPC;
- local index refresh;
- local wallet CLI;
- local explorer build and dev server;
- local backup, reset and smoke test flows.

Non-goals:
- no VPS deployment commands;
- no live public mainnet claims;
- no live public testnet claims.

## Windows Toolchain Note

If this workstation resolves `cargo` to a `gnullvm` host build from a separate LLVM install, prefer the rustup-managed MSVC cargo shim:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
$env:RUSTC = "$env:USERPROFILE\.cargo\bin\rustc.exe"
& "$env:USERPROFILE\.cargo\bin\cargo.exe" -Vv
```

Expected host:
- `x86_64-pc-windows-msvc`

## Local Paths

The local operator flow uses:
- chain data: `.alvenqis-local/chain/`
- mempool data: `.alvenqis-local/mempool/`
- index data: `.alvenqis-local/indexer/`
- wallet data: `.alvenqis-local/wallets/`
- logs: `.alvenqis-local/logs/`
- backups: `.alvenqis-local/backups/`

Primary local configs:
- `configs/local.toml`
- `configs/rpc.local.toml`
- `configs/explorer.local.example.env`

## Wrapper Scripts

### Start Everything

PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\local\start-all.ps1
```

Shell:

```bash
bash scripts/local/start-all.sh
```

What it does:
- starts the local node;
- starts the local RPC gateway;
- refreshes the local index snapshot;
- starts the explorer dev server if available.

### Show Status

PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\local\status-all.ps1
```

Shell:

```bash
bash scripts/local/status-all.sh
```

What it does:
- shows managed process status;
- checks chain validation;
- checks RPC health;
- checks index status;
- shows the latest block if available.

### Mine One Block

PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\local\mine-local-block.ps1
```

Shell:

```bash
bash scripts/local/mine-local-block.sh
```

What it does:
- mines one local block;
- validates the chain afterward;
- refreshes the local index snapshot;
- prints the latest block when possible.

### Back Up Local Data

PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\local\backup-local-chain.ps1
```

Shell:

```bash
bash scripts/local/backup-local-chain.sh
```

What it does:
- creates a timestamped backup under `.alvenqis-local/backups/`;
- includes chain, mempool, indexer and logs by default;
- excludes wallet private keys by default.

### Reset Local Data Safely

PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\local\reset-local-chain.ps1
```

Shell:

```bash
bash scripts/local/reset-local-chain.sh
```

What it does:
- stops managed local processes;
- creates a backup unless `--no-backup` is explicitly passed;
- clears local chain, mempool and index data;
- keeps wallet files unless they are explicitly handled elsewhere.

### Stop Everything

PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\local\stop-all.ps1
```

Shell:

```bash
bash scripts/local/stop-all.sh
```

What it does:
- stops managed PowerShell or shell wrapper processes;
- shuts down or kills orphaned local `alvenqis-node` and `alvenqis-rpc-gateway` binaries under `.alvenqis-local/build/` if needed.

### Run Full Smoke Test

PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\local\run-local-smoke-test.ps1
```

Shell:

```bash
bash scripts/local/run-local-smoke-test.sh
```

What it verifies:
- release gate passes or basic checks pass;
- node starts or initializes;
- chain validates;
- one block can be mined;
- RPC `/health` works;
- RPC `/network` works;
- wallet can create a local wallet;
- wallet can show an address;
- indexer can refresh and report current chain data;
- explorer build works if explorer exists.

## Direct Node Commands

Start node:

```powershell
cargo run -p alvenqis-node -- --config configs/local.toml --data-dir .alvenqis-local/chain --mempool-dir .alvenqis-local/mempool start-node
```

Node status:

```powershell
cargo run -p alvenqis-node -- --config configs/local.toml --data-dir .alvenqis-local/chain --mempool-dir .alvenqis-local/mempool node-status
```

Validate chain:

```powershell
cargo run -p alvenqis-node -- --config configs/local.toml --data-dir .alvenqis-local/chain --mempool-dir .alvenqis-local/mempool validate-chain
```

Mine one block:

```powershell
cargo run -p alvenqis-node -- --config configs/local.toml --data-dir .alvenqis-local/chain --mempool-dir .alvenqis-local/mempool mine-block
```

Mine pending block:

```powershell
cargo run -p alvenqis-node -- --config configs/local.toml --data-dir .alvenqis-local/chain --mempool-dir .alvenqis-local/mempool mine-pending-block
```

Show mempool status:

```powershell
cargo run -p alvenqis-node -- --config configs/local.toml --data-dir .alvenqis-local/chain --mempool-dir .alvenqis-local/mempool mempool-status
```

Shutdown local node:

```powershell
cargo run -p alvenqis-node -- --config configs/local.toml --data-dir .alvenqis-local/chain --mempool-dir .alvenqis-local/mempool shutdown
```

## Direct RPC Commands

Start RPC:

```powershell
cargo run -p alvenqis-rpc-gateway -- --config configs/rpc.local.toml
```

Useful local reads:
- `GET http://127.0.0.1:10787/health`
- `GET http://127.0.0.1:10787/network`
- `GET http://127.0.0.1:10787/status`
- `GET http://127.0.0.1:10787/chain/tip`
- `GET http://127.0.0.1:10787/blocks/latest`
- `GET http://127.0.0.1:10787/mempool`
- `GET http://127.0.0.1:10787/mempool/status`
- `GET http://127.0.0.1:10787/indexer/status`
- `GET http://127.0.0.1:10787/indexer/overview?blocks=12&transactions=20`
- `GET http://127.0.0.1:10787/indexer/blocks?offset=0&limit=20`

`/indexer/summary` returns the complete compatibility snapshot and grows with
chain history. Do not use it for recurring monitoring or UI refreshes.

## Direct Wallet Commands

Create wallet:

```powershell
cargo run -p alvenqis-wallet -- --network mainnet-candidate --wallet-dir .alvenqis-local/wallets --signed-tx-dir .alvenqis-local/wallets/signed-txs create-wallet
```

Show address:

```powershell
cargo run -p alvenqis-wallet -- --network mainnet-candidate --wallet-dir .alvenqis-local/wallets address
```

Wallet status:

```powershell
cargo run -p alvenqis-wallet -- --network mainnet-candidate --wallet-dir .alvenqis-local/wallets wallet-status
```

Check balance:

```powershell
cargo run -p alvenqis-wallet -- --network mainnet-candidate --wallet-dir .alvenqis-local/wallets --rpc-base-url http://127.0.0.1:10787 balance <address>
```

Submit signed transaction:

```powershell
cargo run -p alvenqis-wallet -- --network mainnet-candidate --wallet-dir .alvenqis-local/wallets --signed-tx-dir .alvenqis-local/wallets/signed-txs --rpc-base-url http://127.0.0.1:10787 submit-tx --tx-file <path>
```

## Direct Indexer Commands

Refresh index snapshot:

```powershell
cargo run -p alvenqis-indexer -- --network mainnet-candidate --chain-data-dir .alvenqis-local/chain --index-dir .alvenqis-local/indexer index-chain
```

Show index status:

```powershell
cargo run -p alvenqis-indexer -- --network mainnet-candidate --index-dir .alvenqis-local/indexer status
```

Print index summary:

```powershell
cargo run -p alvenqis-indexer -- --network mainnet-candidate --index-dir .alvenqis-local/indexer print-index-summary
```

## Explorer

Build explorer:

```powershell
cd alvenqis-explorer
npm install
npm run build
```

Run explorer locally:

```powershell
cd alvenqis-explorer
$env:VITE_ALVENQIS_RPC_URL = "http://127.0.0.1:10787"
npm run dev -- --host 127.0.0.1 --port 4173
```

Expected local URL:
- `http://127.0.0.1:4173`

## Logs

Local wrapper logs live under:
- `.alvenqis-local/logs/node.log`
- `.alvenqis-local/logs/node.err.log`
- `.alvenqis-local/logs/rpc.log`
- `.alvenqis-local/logs/rpc.err.log`
- `.alvenqis-local/logs/explorer.log`
- `.alvenqis-local/logs/explorer.err.log`
- `.alvenqis-local/logs/indexer-refresh.log`
- `.alvenqis-local/logs/indexer-refresh.err.log`

## Related Documents

- `docs/operator/LOCAL_RUNBOOK.md`
- `docs/operator/TROUBLESHOOTING.md`
- `README.md`
