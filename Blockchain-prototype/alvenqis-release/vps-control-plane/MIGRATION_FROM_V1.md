# Migration from the previous Docker stack

This overlay must be copied over the full Alvenqis repository. It intentionally omits `.env`, runtime state and secret values.

```bash
cd /home/apps/alvenqis-network
cp -a alvenqis-release/vps-control-plane/.env /root/alvenqis-env-before-v2 2>/dev/null || true
cp -a alvenqis-release/vps-control-plane/state /root/alvenqis-state-before-v2

unzip /path/to/alvenqis-docker-control-plane-2.1.0-no-autoupdate.zip -d /tmp/alvenqis-v2
cp -a /tmp/alvenqis-v2/Alvenqis-Network/. /home/apps/alvenqis-network/

cd /home/apps/alvenqis-network/alvenqis-release/vps-control-plane
chmod +x scripts/*.sh docker/*.sh docker/caddy/*.sh docker/backup-scheduler/*.sh
./scripts/repair-existing-installation.sh
```

The repair script removes the old updater script and updater container, removes old PostgreSQL/cAdvisor containers without deleting Alvenqis state, and moves any former `state/rollback` directory into `state/legacy-disabled/`.

Never use `docker compose down -v`.
