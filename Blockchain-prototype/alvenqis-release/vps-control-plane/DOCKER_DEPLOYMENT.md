# Alvenqis Docker Control Plane 2.1.0-no-autoupdate

This is the corrected Docker-first overlay for Alvenqis Mainnet Candidate / Prototype.

Implemented corrections:

- runtime definitions are split under `compose/`, and `compose/roles.json`
  selects an allowlisted operator role without implicitly loading project edge,
  Cloudflare, pool or observability services;
- runtime storage is mounted at `/data/.alvenqis-mainnet`, so node paths are `/data/.alvenqis-mainnet/chain`, `/mempool`, `/indexer` and `/node`;
- an empty chain imports the pinned pre-mined candidate genesis bundled at
  `docs/release/genesis.mainnet-candidate.block.json`; VPS startup never mines
  or regenerates candidate genesis;
- PostgreSQL and postgres-exporter are absent until a real indexer database adapter exists;
- the web UI, backup scheduler and log collector do not mount Docker socket;
- one non-public, token-authenticated broker owns the single Docker socket mount and exposes an action allow-list only;
- cAdvisor is retained only in the explicit project-observability overlay;
  host metrics use node-exporter and container metrics use bounded cAdvisor
  collection;
- `alvenqis-metrics-exporter` scrapes live RPC/pool JSON status into Prometheus (chain height, indexer lag, peers, pool workers/hashrate);
- one bounded RPC gateway serves the public read/submit API and the pool's
  Docker-internal mining API, avoiding a second chain-loading gateway;
- the hardened Nginx gateway isolates RPC availability from Grafana, Loki and
  ops readiness;
- SQLite remains the canonical block oracle while RocksDB stores authenticated
  canonical state, block metadata and the persistent mempool in dedicated
  LZ4-compressed column families;
- every service has memory, CPU, PID, restart, health and bounded log settings;
- P2P and Stratum remain direct TCP listeners; Cloudflare records for both are
  DNS-only and never use the HTTP proxy;
- the pool supervisor reloads renewed Stratum TLS material by restarting only
  the pool listener, not the node or RPC;
- the indexer supervisor prevents overlapping syncs and uses bounded
  exponential retry while exporting real lag;
- Grafana dashboards under `monitoring/grafana/dashboards/` use real PromQL against Prometheus (`uid=alvenqis-prometheus`);
- Tini exists once, in the runtime image;
- fleet enrollment generates a Docker-native installation command;
- backups use RocksDB BackupEngine incrementally, take online SQLite snapshots,
  require matching tips, encrypt the secrets archive and checksum the exact
  restore set;
- automatic updates are removed completely.

The package has no Watchtower, updater container, update script, update/rollback API, scheduled image pull, mutable `latest` default, or update buttons. The included GitHub workflow is manual-dispatch only and requires an explicit tag. Cloudflared runs with `--no-autoupdate`. See `MANUAL_UPGRADE.md` for the only supported upgrade flow.

## Fresh installation

Apply this overlay over the full repository, then:

```bash
cd alvenqis-release/vps-control-plane
chmod +x scripts/*.sh docker/*.sh docker/backup-scheduler/*.sh
./scripts/install-docker-stack.sh
```

Use the SSH tunnel printed by the installer. The installer builds from the checked-out repository; it does not pull a newer Alvenqis runtime.
Before compose starts, `scripts/runtime-preflight.sh` verifies actual Docker
host CPU/RAM/free disk, TCP port ranges, seed multiaddresses and pool TLS
requirements. The full stack defaults require 6 CPUs, at least 11 GiB visible
RAM and 32 GiB free disk.

`state/secrets/alvenqis_storage_key` is generated only for a fresh state root
and mounted read-only into runtime containers. Existing RocksDB data without
that exact key is a fatal preflight error; the installer never silently creates
a replacement key.

## Repair an earlier installation

Preserve `.env` and `state/`, apply the overlay, then:

```bash
./scripts/repair-existing-installation.sh
```

Never run `docker compose down -v` unless permanent destruction of all Alvenqis state is intentional.
