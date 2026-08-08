# Fleet Audit-Log Hash-Chain Status — 2026-08-08

Status: Deployed defensive implementation / immutable CI and external anchoring pending

## Scope checked

This review covers controlled fleet mutations persisted by
`alvenqis-setup-external/admin-server`. It does not cover consensus, public P2P
participation, wallet activity, or third-party systems.

## Current safeguard

- Every new controlled mutation receives a monotonic sequence number, the
  previous entry's SHA-256 digest, and its own SHA-256 digest.
- The versioned domain `alvenqis-admin-audit-v1` and length-prefixed fields make
  the digest input deterministic without depending on JSON field order.
- The first record links to an all-zero genesis digest. Later records link to
  the exact preceding digest.
- Admin-server startup validates sequence, previous-digest linkage, and the
  recomputed digest before accepting an existing chained store.
- A store containing only legacy unchained records is migrated once and written
  through the existing atomic temporary-file rename path. A mixed legacy and
  chained store is rejected instead of being silently repaired.
- Routine retention no longer removes old audit entries. Observation and
  idempotency-record bounds are unchanged.

## Local verification

From `Blockchain-prototype` on 2026-08-08:

```text
cargo test -p alvenqis-vps-admin
33 passed; 0 failed

cargo clippy -p alvenqis-vps-admin --all-targets -- -D warnings
exit 0
```

The focused tests prove that chained entries survive reload; modified,
reordered, and mixed legacy/chained records cause load rejection; and a fully
legacy record set is migrated into a valid chain.

## Controlled deployment evidence

The current source was rebuilt without Docker layer cache and deployed to the
project-operated VPS on 2026-08-08. The integrated stack health check passed,
all 22 Alvenqis containers reported healthy, and the public RPC role contract
returned `410 application/json` for both `/mining/template` and
`/mining/submit`.

The live control state contained no `fleet.json` and therefore no persisted
audit entries before or after restart. Startup of the deployed admin service
proves the empty-store path, but it does not constitute live legacy-migration or
non-empty-chain evidence. Those cases remain covered only by the focused Rust
tests above until a controlled project operation creates an audit record.

## Planned improvement

- Run the same checks on an immutable GitHub revision.
- Capture non-empty deployed-chain evidence after an authorized controlled
  fleet operation. If a legacy store is encountered later, verify its migration
  and perform a bounded tamper drill only on a disposable copy.
- Sign or independently retain periodic chain-head checkpoints. Without that
  anchor, a privileged writer able to replace the store can recompute the full
  chain, and tail truncation cannot be proven from the local file alone.
- Define monitoring and archival policy because audit history now grows rather
  than being trimmed at 20,000 records.

## Evidence boundary

The hash chain is deployed but currently has no live entries. It is a
tamper-evident local data structure, not an immutable external ledger or an
external review result. `TM-1216` therefore remains In Progress.
