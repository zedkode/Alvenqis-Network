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
COMPOSE_PARALLEL_LIMIT=1 docker compose --env-file .env -f compose.yaml --profile cloudflare --profile pool --profile backup up -d --build
# add --profile pool only when pool is intentionally enabled
./scripts/health-check-docker.sh
./scripts/verify-public-health.sh
# from any laptop with network:
# ./scripts/smoke-public-candidate.sh
# or: powershell -File scripts/smoke-public-candidate.ps1
```

Do not use `docker compose down -v`. A source rollback is also manual: restore the reviewed Git commit and the matching configuration backup, rebuild, then run health checks.

## Restore drill (matching backup)

```bash
# path is the stamp directory created by backup-now.sh
RESTORE_CONFIRM=yes ./scripts/restore-from-backup.sh state/backups/<UTC-stamp>
./scripts/health-check-docker.sh
./scripts/smoke-public-candidate.sh
```

`restore-from-backup.sh` refuses to run without `RESTORE_CONFIRM=yes`, never runs
`docker compose down -v`, and writes a pre-restore live snapshot under
`state/backups/pre-restore-<stamp>/` when live paths exist.
