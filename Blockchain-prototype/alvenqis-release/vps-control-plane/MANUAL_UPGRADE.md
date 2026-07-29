# Manual upgrades only

This package contains no automatic updater. There is no Watchtower, updater service, scheduled pull, mutable `latest` default, update endpoint, rollback endpoint, or update button. Cloudflared is started with `--no-autoupdate`.

Full rehearsal ops (inventory, backup, restore drill, public smoke): see the monorepo operator runbook
[`Blockchain-docs/human/operator/VPS_REHEARSAL_OPS.md`](../../../Blockchain-docs/human/operator/VPS_REHEARSAL_OPS.md)
(or the same path from the monorepo root).

An upgrade is an explicit operator procedure:

```bash
cd /home/apps/alvenqis-network/alvenqis-release/vps-control-plane
# or: .../Blockchain-prototype/alvenqis-release/vps-control-plane
./scripts/backup-now.sh
git status
git fetch --all --tags
# Review and explicitly check out the chosen commit or tag.
./scripts/validate-stack.sh --require-docker
./scripts/compose.sh config --quiet
COMPOSE_PARALLEL_LIMIT=1 ./scripts/compose.sh up -d --build
# Overlay selection comes only from ALVENQIS_OPERATOR_ROLE and the explicit
# ENABLE_POOL/CLOUDFLARE_MODE values in .env.
./scripts/health-check-docker.sh
./scripts/verify-public-health.sh
# from any laptop with network:
# ./scripts/smoke-public-candidate.sh
# or: powershell -File scripts/smoke-public-candidate.ps1
```

Do not use `docker compose down -v`. A source rollback is also manual: restore the reviewed Git commit and the matching configuration backup, rebuild, then run health checks.

Before replacing a runtime image, confirm that `.env` still contains
`ALVENQIS_REQUIRE_STORAGE_ENCRYPTION=true` and that
`state/secrets/alvenqis_storage_key` is the original 32-byte hexadecimal key.
Do not regenerate this key over an existing `state.rocksdb`.

## Restore drill (matching backup)

```bash
# path is the stamp directory created by backup-now.sh
RESTORE_CONFIRM=yes ./scripts/restore-from-backup.sh state/backups/<UTC-stamp>
./scripts/health-check-docker.sh
./scripts/smoke-public-candidate.sh
```

`restore-from-backup.sh` refuses to run without `RESTORE_CONFIRM=yes`, never runs
`docker compose down -v`, and writes a pre-restore live snapshot under
`state/backups/pre-restore-<stamp>/` when live paths exist. A valid backup
contains exactly these checksummed files:

- `BACKUP_COMPLETE`;
- `alvenqis-state.tar.gz`;
- `alvenqis-rocksdb-backup.tar.gz`;
- `alvenqis-secrets.tar.gz.enc`.

The restore is rejected unless the RocksDB key ID, network, block count, height
and tip hash match the marker and the staged SQLite replay. Use
`RESTORE_SECRETS=false` only when the current host already has the matching
storage key; a different key is rejected before the live project is stopped.
