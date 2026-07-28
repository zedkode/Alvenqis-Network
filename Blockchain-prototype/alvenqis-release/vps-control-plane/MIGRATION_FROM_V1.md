# Migration from the previous Docker stack

This overlay must be copied over the full Alvenqis repository. It intentionally omits `.env`, runtime state and secret values.

```bash
cd /home/apps/alvenqis-network
cp -a alvenqis-release/vps-control-plane/.env /root/alvenqis-env-before-v2 2>/dev/null || true
cp -a alvenqis-release/vps-control-plane/state /root/alvenqis-state-before-v2

unzip /path/to/alvenqis-docker-control-plane-2.1.0-no-autoupdate.zip -d /tmp/alvenqis-v2
cp -a /tmp/alvenqis-v2/Alvenqis-Network/. /home/apps/alvenqis-network/

cd /home/apps/alvenqis-network/alvenqis-release/vps-control-plane
chmod +x scripts/*.sh docker/*.sh docker/backup-scheduler/*.sh
./scripts/repair-existing-installation.sh
```

The repair script removes the old updater script and updater container, removes old PostgreSQL/cAdvisor containers without deleting Alvenqis state, and moves any former `state/rollback` directory into `state/legacy-disabled/`.

For a legacy SQLite-only chain, repair creates the storage key once and the
node rebuilds RocksDB through a full canonical SQLite replay before startup.
If `state/data/chain/state.rocksdb/CURRENT` already exists, the matching
`state/secrets/alvenqis_storage_key` must also exist; repair deliberately
refuses to invent a new key. After migration, run:

```bash
./scripts/health-check-docker.sh
./scripts/backup-now.sh
```

The health gate compares the encrypted RocksDB state with the live SQLite
network, block count, height and tip hash.

Never use `docker compose down -v`.
