# Chain Maturity Ops (Task 3 — prototype hardening)

Status: **Mainnet Candidate / Prototype — not public Mainnet**

Task 3 collects **operator evidence** for G4-adjacent storage and multi-host
maturity items called out in
[`NETWORK_MATURITY.md`](../release/NETWORK_MATURITY.md):

| G4-adjacent item | Task 3 drill | Waives G4? |
|---|---|---|
| Node SQLite online backup + restore + integrity | Drill A (local script) | **No** — still needs independent review |
| Disk-failure recovery | Drill B (optional live sim) | **No** |
| Reorg / higher-work adoption | Drill C (tests + optional two-node) | **No** |
| Multi-host soak + seeds | Drill D (checklist only) | **No** |
| VPS package backup/restore | Task 1 + Drill E | **No** |

Honesty rules:

- A green local drill is **rehearsal evidence**, not go-live approval.
- Do **not** copy live `chain.sqlite3` between hosts as a “sync” shortcut.
- Do **not** claim Mainnet Live after these drills.

Related:

- [VPS_REHEARSAL_OPS.md](VPS_REHEARSAL_OPS.md) (Task 1)
- [PRIVATE_MINING_OPS.md](PRIVATE_MINING_OPS.md) (Task 2)
- [CHAIN_HEALTH.md](CHAIN_HEALTH.md) (continuous health probes)
- [LOCAL_RUNBOOK.md](LOCAL_RUNBOOK.md)
- [BACKUP_RESTORE_DRILL_2026-07-30.md](../engineering/BACKUP_RESTORE_DRILL_2026-07-30.md)

---

## Drill A — SQLite online backup → isolated restore (required)

Works on a local candidate chain under `.alvenqis-local/` (or `ALVENQIS_LOCAL_ROOT`).

```powershell
# from monorepo root
powershell -File Blockchain-scripts\operator\sqlite-restore-drill.ps1
```

```bash
bash Blockchain-scripts/operator/sqlite-restore-drill.sh
```

What it does:

1. `verify-chain-database` on live local data dir
2. `backup-chain-database` into
   `.alvenqis-local/maturity-evidence/sqlite-restore-<UTC>/online-backup/chain.sqlite3`
3. Copies backup into a **fresh isolated** data dir
4. `verify-chain-database` on the restore
5. compares backup and restored-file SHA-256 values
6. runs `validate-chain` on the restore and requires network ID, height, block
   count, and tip hash to match the source
7. writes `evidence.json` plus a complete `drill.log` transcript

For an isolated fixture or non-default build cache, set
`ALVENQIS_LOCAL_ROOT`, `ALVENQIS_LOCAL_NODE_CONFIG`, and
`ALVENQIS_BUILD_DIR`. These overrides do not relax the node's storage-path
allowlist.

Success criteria:

- Online backup file exists and integrity_check = ok
- Restored DB integrity_check = ok
- Backup and restored-file SHA-256 values match
- Source and restored network ID, height, block count, and tip hash match
- Restored tip hash matches pre-backup tip when chain was non-empty

If there is no local chain yet, initialize via [LOCAL_RUNBOOK.md](LOCAL_RUNBOOK.md)
(`start-all` / genesis import) first — the drill refuses an empty missing DB.

---

## Drill B — Disk-failure simulation (optional, live-touching)

Only after Drill A passes. **Stops** the managed local node if running and
replaces the live SQLite file from the backup.

```powershell
powershell -File Blockchain-scripts\operator\sqlite-restore-drill.ps1 `
  -SimulateDiskFailure -ConfirmLiveRestore
```

```bash
DISK_FAILURE_SIM=yes RESTORE_CONFIRM=yes \
  bash Blockchain-scripts/operator/sqlite-restore-drill.sh
```

Steps (scripted):

1. Fresh online backup
2. Move `chain.sqlite3` → `chain.sqlite3.failed-<stamp>`
3. Install backup as new `chain.sqlite3`
4. Integrity verify
5. Operator restarts local stack and confirms `/status` tip

Never run Drill B against a VPS live path without Task 1 package backup first.

---

## Drill C — Reorg maturity evidence

### C1 — Automated (always available in CI/dev)

From `Blockchain-prototype/`:

```powershell
cargo test -p alvenqis-node --test devnet invalid_reorg_candidate_never_changes_canonical_storage -- --nocapture
cargo test -p alvenqis-node --test devnet detached_valid_transaction_returns_to_mempool_after_reorg -- --nocapture
cargo test -p alvenqis-node restore_from_online_backup_matches_tip_and_validates -- --nocapture
```

Record: command, commit SHA, PASS/FAIL in SESSION_LOG.

### C2 — Two-node competing branch (operator multi-host)

Requires **two** controlled hosts or two local data roots with P2P:

1. Same genesis + network ID
2. Divergent valid branches after a common height
3. Connect peers; higher **cumulative work** must win
4. Loser mempool reconcile; no manual DB copy

Evidence fields:

- host A/B heights before and after
- tip hashes
- cumulative_work values from `/status`
- whether adoption required staged fork validation

Do **not** force either chain by file copy.

---

## Drill D — Multi-host soak checklist (operator)

Minimum soak for Task 3 **evidence** (still not G4):

| Check | How |
|---|---|
| ≥2 nodes same genesis | `/status` tip_hash at height 0 or shared ancestor |
| P2P peers > 0 both ways | `/p2p/status` or Control Center Network |
| Tip growth or stable tip for ≥24h | chain health workflow / probe |
| Indexer lag bounded | `index_in_sync` / `index_lag_blocks` |
| No unauthorized public mining | Task 1/2 smokes |
| Backup taken once per host | Task 1 / Drill A |

Public continuous probe (no SSH):

```powershell
node Blockchain-scripts\operator\maturity-health.mjs
# pool is optional unless ALVENQIS_REQUIRE_POOL=1
```

```powershell
powershell -File Blockchain-scripts\operator\chain-maturity-snapshot.ps1
```

---

## Drill E — Alvenqis Setup External backup/restore (package level)

Already defined in Task 1:

```bash
./scripts/backup-now.sh
RESTORE_CONFIRM=yes ./scripts/restore-from-backup.sh state/backups/<stamp>
./scripts/health-check-docker.sh
./scripts/smoke-public-candidate.sh
```

This protects compose state + chain volume. Pair with Drill A for **node-level**
SQLite API backup semantics on a local or extracted data dir.

---

## Evidence log template (paste into SESSION_LOG)

```text
## YYYY-MM-DD - Task 3 chain maturity evidence
- git: <short sha>
- Drill A sqlite-restore: PASS/FAIL path=.alvenqis-local/maturity-evidence/...
- Drill B disk-failure: skipped | PASS/FAIL
- Drill C reorg tests: PASS/FAIL
- Drill C two-node: skipped | heights A/B before/after tips ...
- Drill D soak: hosts=... duration=... peers=... final height=...
- Drill E VPS package restore: skipped | PASS/FAIL
- Label remains Mainnet Candidate (not Mainnet Live)
```

---

## What “done” means for Task 3 (monorepo + first evidence)

| Checkpoint | Owner |
|---|---|
| Runbook + scripts in monorepo | engineering |
| Drill A PASS once on operator machine | operator |
| Drill C1 reorg/backup unit tests PASS | engineering/CI |
| Drill D checklist filled for ≥1 multi-host attempt or explicit “single-host only” note | operator |
| Memory updated | engineering |
| G4 still open | everyone |

Task 3 **complete tooling** ≠ G4 launch.
