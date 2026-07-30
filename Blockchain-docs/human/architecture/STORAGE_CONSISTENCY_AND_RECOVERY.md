# Storage Consistency and Recovery

Status: Draft audit contract / Mainnet Candidate

## Current storage roles

| Store | Current role | Authority |
|---|---|---|
| Node SQLite | Canonical blocks, canonical transaction-location index, tip/reorg transaction boundary, detached block archive | Canonical block oracle |
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

- versioned SQLite schema, page-integrity checks, and foreign-key checks;
- WAL and full synchronous durability for canonical node SQLite;
- transactional canonical tip/reorg changes and detached-block archival;
- validation-cache entries bind the stored hash and serialized block body, so a
  body change cannot reuse an earlier expensive PoW-hash validation;
- every canonical load recomputes the transaction Merkle root; the deep
  integrity path additionally recomputes every stored block hash and emits a
  domain-separated diagnostic Merkle commitment over canonical block hashes;
- node startup runs the deep integrity path before starting P2P, repeats it
  every six hours by default, records `storage-integrity.json` in the runtime
  directory, and stops the P2P worker when a periodic check fails;
- schema-v2 transaction locations keyed by transaction hash, with height and
  position validated against canonical block contents;
- atomic schema-v1-to-v2 migration that backfills transaction locations from
  the already stored canonical block bodies;
- RocksDB presence/integrity checks and encrypted storage key;
- bounded, encrypted backup tooling in the control plane;
- indexer SQLite with legacy JSON import;
- startup refusal when encrypted state exists without its matching key.
- local SQLite Drill A records a full transcript, verifies the backup and
  restored-file SHA-256 values, and compares source/restored chain identity.

The transaction-location hash index is not a uniqueness constraint. Until the
canonical duplicate-transaction rules tracked by `TM-203` are complete, lookup
retains the earlier behavior of returning the first canonical occurrence by
height and position. Append and reorg maintain the index in the canonical
SQLite transaction; deleting a canonical block removes its indexed locations
through the declared foreign key. Duplicate admission, mined-transaction RPC
lookup, and data-directory-backed mempool sanitation/template/reorg paths query
the bounded candidate hash set through this index rather than constructing a
set from every canonical block.

## Open audit work

- define and test the exact SQLite-to-RocksDB commit/recovery protocol;
- crash at every write boundary and prove deterministic startup recovery;
- add quarantine and operator-guided repair for detected canonical SQLite
  corruption; current behavior detects and stops but does not mutate or repair;
- corrupt RocksDB state, mempool, indexer, pool, and archive rows independently
  and record detection/repair behavior;
- extend the local SQLite evidence in
  `../engineering/BACKUP_RESTORE_DRILL_2026-07-30.md` to a fresh independent
  host and compare genesis, height, tip, state root, and sampled balances;
- fix RPC index-cache invalidation for SQLite updates;
- prove deep reorg and interrupted pre-adoption branch recovery;
- replace or harden pool persistence for multi-instance production use.

## Evidence commands

```bash
cd Blockchain-prototype
cargo test -p alvenqis-node --locked
cargo test -p alvenqis-node cached_load_rejects_tampered_block_body --lib
cargo test -p alvenqis-node integrity_check_rejects_merkle_root_mismatch_when_stored_hash_matches --lib
cargo test -p alvenqis-indexer --locked
cargo test -p alvenqis-mining-pool --locked
```

Operator evidence uses `operator/CHAIN_MATURITY_OPS.md` and the active
control-plane backup/restore scripts.
