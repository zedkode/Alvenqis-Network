# SQLite Backup and Restore Drill — 2026-07-30

Status: Point-in-time local recovery evidence / `TM-308` remains In Progress

## Scope

This report records a Linux Drill A run of the node SQLite online-backup and
isolated-restore workflow. The run used the pinned Mainnet Candidate genesis in
a temporary `.alvenqis-local` root. It did not access a VPS, replace a live
database, simulate disk failure, or use an independent second host.

Repository base commit: `f31b900de715a90358d1a6869f5a22f22ab09040`

The working tree contained uncommitted G1 work, including the drill changes
described here. This evidence therefore applies to that working-tree state, not
to an immutable release commit.

## What the drill checked

The Bash drill:

1. ran `verify-chain-database` against the source;
2. ran `validate-chain` and captured source network ID, height, block count,
   and tip hash;
3. created an online backup through SQLite's backup API;
4. copied that backup into a fresh isolated data directory;
5. ran `verify-chain-database` against the restore;
6. compared the backup and restored-file SHA-256 values;
7. ran `validate-chain` against the restore and required its identity to match
   the source exactly;
8. wrote `evidence.json` and a complete `drill.log` transcript.

The destructive `DISK_FAILURE_SIM` path remained disabled.

## Result

| Check | Result | Evidence |
|---|---|---|
| Source SQLite integrity | Confirmed | `verify-chain-database` exited 0 |
| Online backup creation | Confirmed | backup file created and verified by the node |
| Isolated restore integrity | Confirmed | restored `verify-chain-database` exited 0 |
| Backup/copy byte identity | Confirmed | SHA-256 values matched |
| Network identity | Confirmed | `alvenqis-mainnet-candidate` on source and restore |
| Height and block count | Confirmed | height `0`, blocks `1` on source and restore |
| Tip identity | Confirmed | source and restore tip hashes matched |
| Disk-failure simulation | Not run | explicitly disabled |
| Independent-host restore | Not run | SSH/deployment hold remained active |
| Windows PowerShell drill | Not run | PowerShell was unavailable in the Linux environment |

Backup and restored-file SHA-256:

```text
24258cc1e213f85687d079a6b949cdc0c644198f3c741d78a253d0bf847a7e14
```

Source and restored tip:

```text
0000c29213014578ac41a748c2be3489859f1e0b1f3555bd89b7e5301632a4c5
```

## Attached terminal evidence

The successful drill started at `20260730T143233Z`. Its generated evidence was
validated by parsing `evidence.json` and asserting `pass=true`,
`identity_verified=true`, matching SHA-256 values, and matching tip hashes.

```text
==> Preflight verify-chain-database
valid SQLite chain database data_dir=<temporary-root>/.alvenqis-local/chain
  OK  preflight_integrity

==> Capture source chain identity
valid network_id=alvenqis-mainnet-candidate network=Alvenqis Mainnet Candidate height=0 blocks=1 tip_hash=0000c29213014578ac41a748c2be3489859f1e0b1f3555bd89b7e5301632a4c5
  OK  source_identity

==> Online backup-chain-database
  OK  online_backup — sha256=24258cc1e213f85687d079a6b949cdc0c644198f3c741d78a253d0bf847a7e14

==> Isolated restore + integrity
valid SQLite chain database data_dir=<temporary-root>/.alvenqis-local/maturity-evidence/.../isolated-restore
  OK  isolated_integrity
  OK  restored_file_hash — sha256=24258cc1e213f85687d079a6b949cdc0c644198f3c741d78a253d0bf847a7e14

==> Isolated validate-chain + identity comparison
valid network_id=alvenqis-mainnet-candidate network=Alvenqis Mainnet Candidate height=0 blocks=1 tip_hash=0000c29213014578ac41a748c2be3489859f1e0b1f3555bd89b7e5301632a4c5
  OK  restored_identity_match

PASS: Drill A SQLite backup/restore evidence recorded (not G4).
```

An earlier fixture attempt was rejected before backup because its data path did
not contain the required `.alvenqis-local` component. The successful fixture
used the storage allowlist-compliant path.

## Remaining work

This local result does not complete `TM-308`, `TM-1206`, or `TM-1207`.
Outstanding evidence includes:

- chain plus persisted state and indexer recovery as one consistent set;
- a scheduled retention policy and automated pruning proof;
- a restore on a fresh independent host;
- a larger transaction-bearing chain fixture;
- Windows execution of the PowerShell drill;
- explicitly authorized disk-failure recovery rehearsal;
- an immutable commit with corresponding CI evidence.

