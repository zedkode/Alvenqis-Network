# Docker Installation and Retention

Status: Mainnet Candidate / Prototype

## Requirements

- Ubuntu 24.04 or another supported Docker Engine host;
- Docker Engine with Compose v2;
- `bash`, `curl`, `openssl`, `python3`, `jq`, `rsync`, `flock` and `tar`;
- inbound TCP 20787 for P2P;
- inbound TCP 3333 when the Stratum pool is enabled;
- inbound 80/443 only for direct-DNS mode;
- 6 vCPU, 12 GB RAM and 100 GB NVMe for the complete stack;
- an immutable Alvenqis Docker control-plane archive and SHA-256 file.

## Verify and extract a release

```bash
curl -fsSLO https://example.invalid/alvenqis-docker-control-plane.tar.gz
curl -fsSLO https://example.invalid/alvenqis-docker-control-plane.tar.gz.sha256
sha256sum --check alvenqis-docker-control-plane.tar.gz.sha256
sudo install -d -m 0755 /opt/alvenqis
sudo tar -xzf alvenqis-docker-control-plane.tar.gz -C /opt/alvenqis
cd /opt/alvenqis/alvenqis-release/vps-control-plane
```

The archive contains the reviewed Rust source required for deterministic local
image builds. It contains no `.env`, secret, runtime state or pre-existing
wallet data.

## Fresh controller

```bash
sudo ./scripts/bootstrap-host.sh
```

Create the printed loopback SSH tunnel, open the setup page and provide the
controller, DNS, monitoring and optional pool values. Do not enable the pool
without an approved reward address and offline signing process.

The bootstrap installs all required APT packages and Docker Compose, enables
Docker at boot, starts the installer and writes `/root/alvenqis-login.txt` with
mode `0600`. After deployment, the final panel and Grafana credentials are
written to `state/control/LOGIN.txt`, also with mode `0600`.

## Existing systemd or earlier Docker installation

```bash
sudo ./scripts/repair-existing-installation.sh
```

The repair path is intentionally non-destructive:

- legacy systemd units are stopped and disabled, not removed;
- legacy containers are stopped and retained or renamed;
- legacy chain, control and pool data is copied into `state/`;
- the old source remains available for rollback;
- no command runs `docker compose down -v`.

## Health

```bash
sudo ./scripts/runtime-preflight.sh
sudo ./scripts/health-check-docker.sh
./scripts/compose.sh ps
```

The health script requires every enabled container to be healthy and bounded,
then checks the real chain tip, index lag, configured/validated P2P peers,
authenticated RocksDB parity, private mining template, pool upstream and
Stratum certificate. Set
`P2P_MIN_VALIDATED_PEERS=1` on non-bootstrap nodes. A controller bootstrap may
use `0` while waiting for its first inbound peer; monitoring still alerts on
zero validated peers.

When Cloudflare is enabled, also verify every configured public hostname after
the tunnel or DNS activation completes. P2P and Stratum records must remain
DNS-only because Cloudflare's HTTP proxy is not transparent blockchain TCP.

## Uninstall

```bash
sudo ./scripts/uninstall-docker-stack.sh
```

The default uninstall stops the Alvenqis Docker stack and preserves `state/`,
`.env`, secrets and legacy data. Data destruction is not part of the normal
uninstall or repair flow. Archive and remove retained data only through a
separate, explicit operator-approved procedure.

The retained `state/secrets/alvenqis_storage_key` must stay with
`state/data/chain/state.rocksdb` and every incremental RocksDB backup. Removing
only the key is irreversible data loss, not an uninstall.
