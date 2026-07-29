# Alvenqis Docker VPS Control Plane

Status: Mainnet Candidate / Prototype. This is not a Mainnet Live declaration.

This directory is the only active Alvenqis VPS deployment. It is Docker-only.
The old systemd, host Nginx and automatic-update installers are not part of the
active package. Migration scripts may stop and disable their services, but do
not delete legacy units, containers or data.

## Services

- non-mining full validation and P2P node;
- one Docker-private RPC process for chain reads, transaction submission,
  Stratum work and rate-limited public solo-mining templates/submissions;
- TLS-only Stratum pool (`ENABLE_POOL`) with direct DNS-only TCP routing;
- supervised read-only indexer loop with bounded exponential retry;
- authenticated controller or fleet agent;
- hardened Nginx gateway with optional Cloudflare Tunnel or direct DNS;
- Prometheus, Alertmanager, Grafana, Loki, Alloy, node exporter and alvenqis-metrics-exporter (chain/RPC/indexer/pool gauges);
- SQLite canonical blocks plus RocksDB canonical state, block metadata and
  persistent mempool, with LZ4 column families and authenticated
  XChaCha20-Poly1305 value envelopes;
- verified incremental RocksDB backups, online SQLite snapshots, an encrypted
  secrets archive and optional R2/S3 replication;
- one token-authenticated Docker broker with the only Docker socket mount.

The VPS image does not contain the CUDA miner or wallet keys. Pool payouts
remain a separate offline or HSM-backed operator responsibility.

## Capacity and availability boundary

The default limits are designed for one 6 vCPU, 12 GB RAM, 100 GB NVMe host.
The maximum active container limits stay below 10.5 GiB even with pool,
Cloudflare and backup profiles enabled. Docker logs are rotated, Prometheus is
bounded to 8 GB/15 days and Loki defaults to seven days. Runtime preflight
retains at least 32 GiB free by default; operators may raise, but must not
silently lower, `VPS_MIN_FREE_DISK_BYTES` after capacity planning.

This single-host deployment cannot truthfully guarantee zero downtime. It uses
healthchecks, readiness gates, restart policies, bounded retries, TLS
certificate reload and dependency isolation so monitoring failures do not take
RPC offline. Real host, chain, P2P, indexer, pool-upstream and TLS checks run
before a deploy is accepted.

The current Rust indexer is correct but rebuilds O(n) after a tip divergence.
This runtime prevents overlapping rebuilds and backs off failures; incremental
index attach/detach requires a future change outside this control-plane scope.

## Validate

```bash
cd alvenqis-release/vps-control-plane
./scripts/runtime-preflight.sh
./scripts/validate-stack.sh --require-docker
```

## Install

```bash
./scripts/install-docker-stack.sh
```

The bootstrap UI binds to loopback. Use the SSH tunnel printed by the script,
complete the form, then trigger deploy. The installer builds from the checked
out immutable release bundle and never pulls a newer Alvenqis source tree.

The `validator` bootstrap role is a PoW full-validation node that verifies
blocks and transactions. Alvenqis does not expose a staking-validator role.

## Composable operator roles

`compose/roles.json` is the allowlist used by the installer, health checks,
backup tooling, broker and generated enrollment commands. Set
`ALVENQIS_OPERATOR_ROLE` explicitly:

| Role | Runtime overlays |
|---|---|
| `node` / `validator` | `base.yaml`, `node.yaml` |
| `rpc` | node overlays plus `rpc.yaml` |
| `indexer` | RPC overlays plus `indexer-explorer.yaml` without the explorer profile |
| `indexer-explorer` / `explorer` | RPC overlays plus `indexer-explorer.yaml` |
| `pool` / `stratum` | RPC overlays plus `pool.yaml`; requires `ENABLE_POOL=true` |
| `full-stack` | role overlays plus explicit project edge and observability overlays |

Independent roles do not load the project edge, Cloudflare, website or project
monitoring overlays. Existing project installations without the new variable
resolve to `full-stack` for upgrade compatibility; new `.env.example` files
default to `node`.

Use `./scripts/compose.sh config --quiet`, `./scripts/compose.sh up -d --build`
and `./scripts/compose.sh ps` instead of assembling `-f` arguments manually.

Fleet enrollment is optional. A standalone node can render and start only its
selected role:

```bash
./scripts/enroll-docker-node.sh \
  --standalone \
  --role node \
  --node-name independent-node-1 \
  --p2p-host node1.example.org \
  --seed /dns4/seed.example.org/tcp/20787
```

`MINING_RPC_BIND=docker-internal` is a fail-closed policy marker in G1; the RPC
capability reconciliation remains a later gate and this split does not publish
a new mining listener.

The installer creates `state/secrets/alvenqis_storage_key` once. That key is
part of the storage identity: deleting, regenerating or rotating it without an
explicit data migration makes the current RocksDB state and its backups
unreadable. Never copy a RocksDB backup without its matching encrypted secrets
archive.

## Repair or migrate an existing host

```bash
./scripts/repair-existing-installation.sh
```

Repair creates a rollback copy of `.env`, copies legacy data, stops and disables
conflicting host services, and stops or renames conflicting containers. It does
not delete the old units, containers or data directories.

## Manual upgrade only

Automatic updates, Watchtower and mutable `latest` tags are forbidden. Follow
[MANUAL_UPGRADE.md](MANUAL_UPGRADE.md) with an immutable version and verified
checksum.

## Rehearsal ops (Task 1)

Operator runbook (inventory, backup, restore drill, public smoke):
[`Blockchain-docs/human/operator/VPS_REHEARSAL_OPS.md`](../../../Blockchain-docs/human/operator/VPS_REHEARSAL_OPS.md)
from the monorepo root.

```bash
# public smoke from any host with network (no SSH):
./scripts/smoke-public-candidate.sh
# Windows:
# powershell -File scripts/smoke-public-candidate.ps1

# on VPS after backup-now.sh:
# RESTORE_CONFIRM=yes ./scripts/restore-from-backup.sh state/backups/<UTC-stamp>
```

The restore command verifies the exact checksum manifest, archive member types,
storage key ID, incremental RocksDB backup, and full SQLite replay in an
isolated stage before it stops the live project. After installation it repeats
the parity check; any failure after mutation restores the pre-restore snapshot
before restarting the owned Alvenqis services.

## Secured pool mining ops

Remote pool mining uses the direct DNS-only Stratum TLS endpoint. Solo mining
uses the HTTPS RPC endpoint, while the pool HTTPS endpoint remains read-only:
[`Blockchain-docs/human/operator/PRIVATE_MINING_OPS.md`](../../../Blockchain-docs/human/operator/PRIVATE_MINING_OPS.md).

```bash
# verify the public solo template and Stratum TLS certificate
ALVENQIS_SMOKE_MINER_ADDRESS=alve1... ./scripts/smoke-private-mining.sh
```

See [INSTALL_AND_UNINSTALL.md](INSTALL_AND_UNINSTALL.md) for the complete
operator flow and [DOCKER_DEPLOYMENT.md](DOCKER_DEPLOYMENT.md) for architecture
and security boundaries.
