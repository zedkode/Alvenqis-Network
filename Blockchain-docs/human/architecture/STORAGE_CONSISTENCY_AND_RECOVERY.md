# Storage Consistency and Recovery

Status: Draft audit contract / Mainnet Candidate

## Current storage roles

| Store | Current role | Authority |
|---|---|---|
| Node SQLite | Canonical blocks, tip/reorg transaction boundary, detached block archive | Canonical block oracle |
| Node RocksDB | State and mempool in the VPS profile | Derived operational state that must agree with the canonical chain |
| Indexer SQLite | Address, transaction, and block query index | Rebuildable derived data |
| Pool persistence | Off-chain shares, allocations, maturity, and payout workflow | Never consensus authority |

## Required invariants

1. A reported ready tip has matching canonical block and derived state.
2. A reorg does not expose a half-applied canonical chain/state/index view.
3. Recovery never trusts another host's copied live database as consensus.
4. Every backup includes the matching storage-encryption identity.
5. Indexer and pool data can be rebuilt or reconciled against canonical node
   data.
6. Legacy migration is one-way, validated, and preserves its source input.

## Current controls

- versioned SQLite schema and integrity checks;
- WAL and full synchronous durability for canonical node SQLite;
- transactional canonical tip/reorg changes and detached-block archival;
- RocksDB presence/integrity checks and encrypted storage key;
- bounded, encrypted backup tooling in the control plane;
- indexer SQLite with legacy JSON import;
- startup refusal when encrypted state exists without its matching key.

## Open audit work

- define and test the exact SQLite-to-RocksDB commit/recovery protocol;
- crash at every write boundary and prove deterministic startup recovery;
- corrupt each store independently and record detection/repair behavior;
- restore onto a fresh host and compare genesis, height, tip, state root, and
  sampled balances;
- fix RPC index-cache invalidation for SQLite updates;
- prove deep reorg and interrupted pre-adoption branch recovery;
- replace or harden pool persistence for multi-instance production use.

## Evidence commands

```bash
cd Blockchain-prototype
cargo test -p alvenqis-node --locked
cargo test -p alvenqis-indexer --locked
cargo test -p alvenqis-mining-pool --locked
```

Operator evidence uses `operator/CHAIN_MATURITY_OPS.md` and the active
control-plane backup/restore scripts.
