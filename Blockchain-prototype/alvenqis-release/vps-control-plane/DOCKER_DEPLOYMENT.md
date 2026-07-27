# Alvenqis Docker Control Plane 2.1.0-no-autoupdate

This is the corrected Docker-first overlay for Alvenqis Mainnet Candidate / Prototype.

Implemented corrections:

- runtime storage is mounted at `/data/.alvenqis-mainnet`, so node paths are `/data/.alvenqis-mainnet/chain`, `/mempool`, `/indexer` and `/node`;
- an empty chain imports the pinned pre-mined candidate genesis bundled at
  `docs/release/genesis.mainnet-candidate.block.json`; VPS startup never mines
  or regenerates candidate genesis;
- PostgreSQL and postgres-exporter are absent until a real indexer database adapter exists;
- the web UI, backup scheduler and log collector do not mount Docker socket;
- one non-public, token-authenticated broker owns the single Docker socket mount and exposes an action allow-list only;
- cAdvisor is removed; host metrics use node-exporter;
- `alvenqis-metrics-exporter` scrapes live RPC/pool JSON status into Prometheus (chain height, indexer lag, peers, pool workers/hashrate);
- Grafana dashboards under `monitoring/grafana/dashboards/` use real PromQL against Prometheus (`uid=alvenqis-prometheus`);
- Tini exists once, in the runtime image;
- fleet enrollment generates a Docker-native installation command;
- backups archive real filesystem state and encrypt the secrets archive;
- automatic updates are removed completely.

The package has no Watchtower, updater container, update script, update/rollback API, scheduled image pull, mutable `latest` default, or update buttons. The included GitHub workflow is manual-dispatch only and requires an explicit tag. Cloudflared runs with `--no-autoupdate`. See `MANUAL_UPGRADE.md` for the only supported upgrade flow.

## Fresh installation

Apply this overlay over the full repository, then:

```bash
cd alvenqis-release/vps-control-plane
chmod +x scripts/*.sh docker/*.sh docker/caddy/*.sh docker/backup-scheduler/*.sh
./scripts/install-docker-stack.sh
```

Use the SSH tunnel printed by the installer. The installer builds from the checked-out repository; it does not pull a newer Alvenqis runtime.

## Repair an earlier installation

Preserve `.env` and `state/`, apply the overlay, then:

```bash
./scripts/repair-existing-installation.sh
```

Never run `docker compose down -v` unless permanent destruction of all Alvenqis state is intentional.
