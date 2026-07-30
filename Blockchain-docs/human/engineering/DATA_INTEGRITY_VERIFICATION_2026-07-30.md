# Canonical SQLite Data-Integrity Verification — 2026-07-30

Status: Local G1 engineering evidence / `TM-307` remains In Progress

## Scope

Owned target: `alvenqis-node` canonical SQLite storage and local node runtime.

Defensive objective: detect semantic corruption that SQLite page checks alone
cannot detect, including a serialized block body that no longer matches its
stored identity or transaction Merkle root.

Environment: local Linux Cargo workspace. No public endpoint, SSH connection,
deployment, live database, disk-failure simulation, or consensus source file
was used.

## Reproduced gaps

Two regression tests were written before the fix:

| Test | Pre-fix result | What the failure demonstrated |
|---|---|---|
| `cached_load_rejects_tampered_block_body` | Failed as expected, exit 101 | A warmed validation cache could accept changed `block_json` when the stored hash column was unchanged. |
| `integrity_check_rejects_merkle_root_mismatch_when_stored_hash_matches` | Failed as expected, exit 101 | SQLite `integrity_check` plus transaction-index checking did not recompute the transaction Merkle root. |

The tests mutate only disposable temporary SQLite fixtures.

## Implemented safeguards

- Validation-cache tokens now bind both the stored block hash and a Blake3 hash
  of the serialized block body.
- Every canonical block load recomputes and compares the transaction Merkle
  root, including cache hits.
- The deep integrity command checks SQLite pages and foreign keys, bypasses the
  expensive-hash cache, recomputes canonical block hashes and transaction
  Merkle roots, validates linkage and transaction-index entries, and reports
  counts plus a domain-separated Merkle commitment over canonical block hashes.
- The diagnostic commitment is operational evidence only and does not affect
  consensus or block validity.
- `start-node` performs the deep check before starting P2P and every 21,600
  seconds by default. It records `storage-integrity.json` in the runtime
  directory and stops the P2P worker if a periodic check fails.
- Intervals below 60 seconds are rejected to avoid accidental continuous
  full-chain verification.

## Verification

Focused post-fix checks:

```text
cargo test -p alvenqis-node cached_load_rejects_tampered_block_body --lib -- --nocapture
result: 1 passed; 0 failed; exit 0

cargo test -p alvenqis-node integrity_check_rejects_merkle_root_mismatch_when_stored_hash_matches --lib -- --nocapture
result: 1 passed; 0 failed; exit 0

cargo test -p alvenqis-node storage_integrity_interval_rejects_excessive_frequency --lib -- --nocapture
result: 1 passed; 0 failed; exit 0

cargo test -p alvenqis-node storage_integrity_status_records_diagnostic_commitment --lib -- --nocapture
result: 1 passed; 0 failed; exit 0

cargo test -p alvenqis-node integrity_report_commits_canonical_block_order --lib -- --nocapture
result: 1 passed; 0 failed; exit 0
```

Broader local verification:

```text
cargo test -p alvenqis-node
result: library 55 passed; integration 40 passed; doc tests passed; exit 0

cargo test --workspace
result: exit 0; one public-network smoke remained explicitly ignored
environment note: `nvcc` was unavailable, so CUDA device execution was not exercised

cargo clippy -p alvenqis-node --all-targets -- -D warnings
result: exit 0

cargo fmt --all --check
result: exit 0

node Blockchain-scripts/docs/audit-docs.mjs
result: 146 documents audited; exit 0

node Blockchain-scripts/docs/check-english-content.mjs
result: 843 source text files checked; exit 0

bash Blockchain-scripts/security/check-repo-hygiene.sh
result: exit 0

bash Blockchain-scripts/git/check-forbidden-files.sh
result: secret and forbidden-file checks passed; exit 0
```

GitHub CI and immutable-commit evidence were not produced in this pass.

## Remaining work

This evidence does not complete `TM-307` or `TM-1004`. Remaining work includes:

- quarantine and operator-guided repair after detection;
- corruption/recovery drills for RocksDB state, mempool, indexer, pool, and
  orphan archive semantics;
- crash-boundary and interrupted-repair tests;
- measured deep-check cost on a larger transaction-bearing chain;
- independent-host recovery evidence;
- immutable commit and corresponding CI evidence.
