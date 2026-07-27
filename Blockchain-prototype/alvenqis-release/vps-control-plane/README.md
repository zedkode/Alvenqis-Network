# Alvenqis Docker VPS Control Plane

Status: Mainnet Candidate / Prototype. This is not a Mainnet Live declaration.

This directory is the only active Alvenqis VPS deployment. It is Docker-only.
The old systemd, host Nginx and automatic-update installers are not part of the
active package. Migration scripts may stop and disable their services, but do
not delete legacy units, containers or data.

## Services

- non-mining full validation and P2P node;
- non-published RPC gateway with public `/mining/*` retired at Caddy;
- TLS-only Stratum pool (`ENABLE_POOL`) using that gateway only over the private Docker network;
- read-only indexer loop;
- authenticated controller or fleet agent;
- Caddy with optional Cloudflare Tunnel or direct DNS;
- Prometheus, Alertmanager, Grafana, Loki, Alloy, node exporter and alvenqis-metrics-exporter (chain/RPC/indexer/pool gauges);
- encrypted local and optional R2/S3 backups;
- one token-authenticated Docker broker with the only Docker socket mount.

The VPS image does not contain the CUDA miner or wallet keys. Pool payouts
remain a separate offline or HSM-backed operator responsibility.

## Validate

```bash
cd alvenqis-release/vps-control-plane
./scripts/validate-stack.sh --require-docker
```

## Install

```bash
./scripts/install-docker-stack.sh
```

The bootstrap UI binds to loopback. Use the SSH tunnel printed by the script,
complete the form, then trigger deploy. The installer builds from the checked
out immutable release bundle and never pulls a newer Alvenqis source tree.

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

## Secured pool mining ops

Remote mining uses the direct DNS-only Stratum TLS endpoint. Public HTTP mining
stays retired, while the HTTPS pool endpoint remains read-only for statistics:
[`Blockchain-docs/human/operator/PRIVATE_MINING_OPS.md`](../../../Blockchain-docs/human/operator/PRIVATE_MINING_OPS.md).

```bash
# require HTTP 410 for /mining/* and verify the Stratum TLS certificate
./scripts/smoke-private-mining.sh
```

See [INSTALL_AND_UNINSTALL.md](INSTALL_AND_UNINSTALL.md) for the complete
operator flow and [DOCKER_DEPLOYMENT.md](DOCKER_DEPLOYMENT.md) for architecture
and security boundaries.
