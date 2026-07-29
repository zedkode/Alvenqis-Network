# Troubleshooting

Status: Draft / Mainnet Candidate / Prototype

## `alvenqis-node` rejects `.alvenqis-local` paths

Expected fix in this workspace:
- local operator scripts pass `.alvenqis-local/...` explicitly
- node path validation now allows `.alvenqis-local` as a safe local operator root

If commands still fail:
- run them from the repository root
- verify the command includes `--data-dir .alvenqis-local/chain --mempool-dir .alvenqis-local/mempool`

## RPC starts but reads the wrong chain path

Symptoms:
- `/status` shows uninitialized while the node already wrote blocks

Checks:
- use `configs/rpc.local.toml`
- run from the repository root so relative `.alvenqis-local/...` paths resolve correctly
- verify `.alvenqis-local/chain/chain.sqlite3` exists; a legacy `chain.jsonl`
  should be imported automatically on the next node/storage read
- run `alvenqis-node --data-dir .alvenqis-local/chain verify-chain-database`
  before attempting recovery from a backup

## Explorer cannot build because `tsc` is missing

Cause:
- `alvenqis-explorer/node_modules` does not exist yet

Fix:

```powershell
cd alvenqis-explorer
npm install
npm run build
```

## Release gate fails after local explorer work

Cause:
- local operator use may install `alvenqis-explorer/node_modules`

Fix:
- run the release gate before local smoke work when possible
- if needed, remove `alvenqis-explorer/node_modules` and rerun the gate

## `cargo` uses the wrong Windows toolchain

Symptoms:
- mismatched host toolchain
- unexpected build failures on Windows

Fix:
- prefer `%USERPROFILE%\.cargo\bin\cargo.exe`
- or run:

```powershell
cargo +stable-x86_64-pc-windows-msvc test --workspace
```

## Ports are already in use

Local defaults:
- RPC: `10787`
- Explorer dev server: `4173`

Fix:
- run `.\Blockchain-scripts\local\stop-all.ps1`
- close stale terminals or lingering processes
- rerun `.\Blockchain-scripts\local\start-all.ps1`

## Reset refuses to proceed

Expected behavior:
- reset creates a backup first unless `--no-backup` is explicitly passed

Examples:

```powershell
.\Blockchain-scripts\local\reset-local-chain.ps1
.\Blockchain-scripts\local\reset-local-chain.ps1 -NoBackup
```

## Wallet balance reads fail

Checks:
- ensure the RPC gateway is running on `http://127.0.0.1:10787`
- verify the wallet command points to that base URL
- verify the address belongs to the active network prefix `ALVE`

## Indexer looks stale after mining

The current indexer is a one-shot snapshot, not a daemon.

Refresh it manually:

```powershell
cargo run -p alvenqis-indexer -- --network mainnet-candidate --chain-data-dir .alvenqis-local/chain --index-dir .alvenqis-local/indexer index-chain
```

Or use:

```powershell
.\Blockchain-scripts\local\mine-local-block.ps1
```

That wrapper refreshes the local index after mining.
