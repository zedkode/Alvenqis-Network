# Alvenqis control-plane monitoring

Real Prometheus scrape targets, exporters and Grafana dashboards for the Alvenqis Docker control plane (`compose` project name: `alvenqis-control-plane`).

Brand: **Alvenqis / ALVE** only. VPS roles: validator / full node / storage / optional pool — **not** a CUDA miner host.

## Architecture

| Component | Role |
| --- | --- |
| `prometheus` | Scrapes jobs below; evaluates `alerts.yml` |
| `alvenqis-metrics-exporter` | Polls and caches live RPC/pool JSON APIs → Prometheus text on `:9101/metrics` |
| `blackbox-exporter` | Lightweight liveness probes for RPC, control, ops, Grafana, Loki, pool |
| `node-exporter` | Host CPU/RAM/disk/network + textfile dir `state/metrics` (backups) |
| `grafana` | Provisioned datasources + file dashboards |
| `loki` + `alloy` | Container logs |

Chain height, indexer lag, peer counts, mempool and pool hashrate/shares/blocks are **not** invented in Grafana — they are scraped from:

- `http://alvenqis-rpc:10787/status`
- `http://alvenqis-rpc:10787/indexer/status`
- `http://alvenqis-rpc:10787/sync/status`
- `http://alvenqis-rpc:10787/p2p/status`
- `http://alvenqis-rpc:10787/mempool/status`
- `http://alvenqis-pool:30787/api/v1/pool/status` (when `ENABLE_POOL=true`)

## Prometheus jobs

| Job name | Target(s) | What it collects |
| --- | --- | --- |
| `prometheus` | `prometheus:9090` | Prometheus self metrics |
| `node-exporter` | `node-exporter:9100` | Host + textfile (`alvenqis_backup_*`) |
| `alvenqis-metrics` | `alvenqis-metrics-exporter:9101` | Chain, indexer, P2P, mempool, pool gauges |
| `alvenqis-http` | blackbox → RPC/control/ops/metrics/Grafana/Loki | `probe_success`, `probe_duration_seconds` |
| `alvenqis-pool-http` | blackbox → pool health | Pool liveness probe (down when pool profile is off) |
| `blackbox-exporter` | `blackbox-exporter:9115` | Exporter self metrics |

Config file: `monitoring/prometheus/prometheus.yml`
Alerts: `monitoring/prometheus/alerts.yml`

### Key metric names (`job=alvenqis-metrics`)

| Metric | Meaning |
| --- | --- |
| `alvenqis_chain_height` | Canonical tip height |
| `alvenqis_chain_ready` | Chain initialized |
| `alvenqis_rpc_up` | RPC `/status` reachable |
| `alvenqis_indexer_height` | Indexed tip height |
| `alvenqis_indexer_lag_blocks_effective` | Blocks behind tip |
| `alvenqis_indexer_in_sync_effective` | 1 when tip matches |
| `alvenqis_p2p_connected_peers` | Connected peers |
| `alvenqis_p2p_validated_peers` | Validated peers |
| `alvenqis_mempool_pending_count` | Mempool size |
| `alvenqis_pool_enabled` | `ENABLE_POOL` flag |
| `alvenqis_pool_up` | Pool status reachable |
| `alvenqis_pool_connected_workers` | Workers |
| `alvenqis_pool_estimated_hashrate_hs` | Pool H/s |
| `alvenqis_pool_accepted_shares` | Shares |
| `alvenqis_pool_blocks_found` | Blocks found |
| `alvenqis_backup_last_success_unixtime` | Via node-exporter textfile |

## Grafana datasource UIDs

| UID | Type | URL |
| --- | --- | --- |
| `alvenqis-prometheus` | Prometheus | `http://prometheus:9090` |
| `alvenqis-loki` | Loki | `http://loki:3100` |

Provisioned from `monitoring/grafana/provisioning/datasources/datasources.yml`.

## Dashboard UIDs

| UID | Title | File |
| --- | --- | --- |
| `alvenqis-docker-overview` | Alvenqis Network Overview | `grafana/dashboards/alvenqis-overview.json` |
| `alvenqis-chain` | Alvenqis Chain & Indexer | `grafana/dashboards/alvenqis-chain.json` |
| `alvenqis-network` | Alvenqis P2P & Sync | `grafana/dashboards/alvenqis-network.json` |
| `alvenqis-pool` | Alvenqis Mining Pool | `grafana/dashboards/alvenqis-pool.json` |
| `alvenqis-host` | Alvenqis Host Metrics | `grafana/dashboards/alvenqis-host.json` |
| `alvenqis-ops` | Alvenqis Ops & Logs | `grafana/dashboards/alvenqis-ops.json` |

Folder in Grafana UI: **Alvenqis Network** (file provider, path `/var/lib/grafana/dashboards`).

Regenerate after metric/export changes:

```bash
python3 monitoring/grafana/scripts/generate-dashboards.py
```

## Operator: import / view dashboards

Dashboards are **auto-provisioned** on stack deploy — no manual import is required when using the control-plane compose stack.

1. Open Grafana (`https://$GRAFANA_HOST` or tunnel).
2. Log in with `GRAFANA_ADMIN_USER` / `state/secrets/grafana_password`.
3. Go to **Dashboards → Alvenqis Network**.
4. Open **Alvenqis Network Overview** (`uid=alvenqis-docker-overview`).

### Manual re-import (if Grafana state was wiped)

1. Ensure volumes still mount:
   - `./monitoring/grafana/provisioning` → `/etc/grafana/provisioning`
   - `./monitoring/grafana/dashboards` → `/var/lib/grafana/dashboards`
2. Restart Grafana:
   `docker compose --env-file .env up -d grafana`
3. Or **Dashboards → New → Import → Upload JSON** and select
   `monitoring/grafana/dashboards/alvenqis-overview.json`,
   choosing datasource **Prometheus** (`alvenqis-prometheus`).

### Verify scrapes are live

```bash
# Metrics exporter health + sample series
docker compose exec -T alvenqis-metrics-exporter curl -fsS http://127.0.0.1:9101/health
docker compose exec -T alvenqis-metrics-exporter curl -fsS http://127.0.0.1:9101/metrics | head

# Prometheus targets
docker compose exec -T prometheus wget -qO- http://127.0.0.1:9090/api/v1/targets
```

Confirm jobs `alvenqis-metrics`, `node-exporter`, `alvenqis-http` show `health: up`.

## Pool profile notes

- Set `ENABLE_POOL=true` in `.env`; the allowlisted role resolver adds
  `compose/pool.yaml` for `pool`, `stratum`, or an enabled `full-stack`.
- Metrics exporter reads the same flag and only expects pool status when enabled.
- Alert `AlvenqisPoolDown` fires only when `alvenqis_pool_enabled == 1`.
- Blackbox job `alvenqis-pool-http` may show down when the pool profile is off; it is informational.
- Heavy status/indexer/P2P reads are not duplicated by blackbox; one exporter
  sample is cached for `METRICS_CACHE_TTL_SECONDS` (default ten seconds).

## Backup textfile metric

Successful `scripts/backup-now.sh` writes:

`state/metrics/alvenqis_backup.prom` → scraped by node-exporter textfile collector.

## Validation

```bash
cd alvenqis-release/vps-control-plane
./scripts/validate-stack.sh
```
