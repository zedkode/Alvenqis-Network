# Independent Node Operator Guide

Status: Draft G2 guide / role rendering verified / clean-host proof pending

This guide uses only the operator's own host, storage, DNS choices, and
credentials. It does not require the project website, RPC, pool, control panel,
Cloudflare account, or monitoring stack. It is not yet clean-host closure
evidence because diverse PeerId-pinned bootstrap and active discovery are not
implemented.

## Current prerequisites

- a reviewed immutable repository commit or release bundle;
- Linux host with Docker Engine, Docker Compose v2, Python 3, OpenSSL, and Bash;
- TCP port `20787` reachable for inbound P2P when the operator chooses to accept
  inbound peers;
- at least one compatible seed supplied by the operator;
- local storage suitable for Docker bind mounts.

The installer currently verifies these prerequisites but does not install the
operating-system packages. A true package-manager-to-running-node one-command
installer remains G2 work.

## Configure a standalone node

```bash
cd Blockchain-prototype/alvenqis-release/vps-control-plane
cp .env.example .env
```

Set at minimum:

```dotenv
ALVENQIS_OPERATOR_ROLE=node
ALVENQIS_DEPLOYMENT_ROLE=node
COMPOSE_PROJECT_NAME=alvenqis-independent-node
ALVENQIS_STATE_ROOT=/var/lib/alvenqis/node
P2P_ADVERTISE_HOST=node.operator.example
P2P_BIND_ADDRESS=0.0.0.0
P2P_PORT=20787
MAX_P2P_PEERS=64
SEED_NODES_TOML="/dns4/seed.operator.example/tcp/20787"
CLOUDFLARE_MODE=disabled
ENABLE_POOL=false
```

Do not copy project secrets or a project `.env`. Do not copy a live node
database from another host as a synchronization shortcut.

## Validate and start

```bash
./scripts/runtime-preflight.sh
./scripts/validate-stack.sh --require-docker
./scripts/install-docker-stack.sh --role node
```

The bootstrap UI is loopback-only. Use the SSH tunnel printed by the installer,
review the generated values, and deploy the `node` role. After deployment:

```bash
./scripts/compose.sh config --quiet
./scripts/compose.sh ps
docker compose logs --tail=200 alvenqis-node
```

The rendered role must contain `alvenqis-node` and must not contain the project
edge, website, explorer, Cloudflare tunnel, Grafana, Prometheus, control panel,
pool, or wallet/miner binaries.

## Verify network identity

Verify from local node state and logs:

- network ID is `alvenqis-mainnet-candidate`;
- chain magic and genesis match the reviewed candidate artifacts;
- the node reports its own PeerId;
- every connected peer passes network/genesis handshake validation;
- synchronization occurs through P2P, not database copying.

The current seed parser accepts host/port and transport multiaddresses but does
not enforce `/p2p/<PeerId>` pinning. Therefore this guide cannot yet satisfy the
decentralization bootstrap exit criterion.

## Backup and restore

```bash
./scripts/backup-now.sh
RESTORE_CONFIRM=yes ./scripts/restore-from-backup.sh state/backups/<UTC-stamp>
./scripts/compose.sh ps
```

Keep the storage encryption key and encrypted secret archive with the backup.
Never regenerate the storage key for an existing encrypted state.

## G2 completion evidence still required

- fresh supported Linux host transcript starting before Docker installation;
- pinned release checksum/signature verification;
- at least three independently operated PeerId-pinned seeds;
- active discovery and failover;
- bidirectional P2P connectivity and synchronization;
- backup and restore onto a second fresh host;
- project website/RPC/pool/controller outage with the node still operational;
- documented uninstall and retained-data decision.
