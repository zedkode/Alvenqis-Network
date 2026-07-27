# Docker Installation and Retention

Status: Mainnet Candidate / Prototype

## Requirements

- Ubuntu 24.04 or another supported Docker Engine host;
- Docker Engine with Compose v2;
- `bash`, `curl`, `openssl`, `python3`, `jq` and `tar`;
- inbound TCP 20787 for P2P;
- inbound 80/443 only for direct-DNS mode;
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
sudo ./scripts/install-docker-stack.sh
```

Create the printed loopback SSH tunnel, open the setup page and provide the
controller, DNS, monitoring and optional pool values. Do not enable the pool
without an approved reward address and offline signing process.

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
sudo ./scripts/health-check-docker.sh
docker compose --env-file .env -f compose.yaml ps
```

When Cloudflare is enabled, also verify every configured public hostname after
the tunnel or DNS activation completes.

## Uninstall

```bash
sudo ./scripts/uninstall-docker-stack.sh
```

The default uninstall stops the Alvenqis Docker stack and preserves `state/`,
`.env`, secrets and legacy data. Data destruction is not part of the normal
uninstall or repair flow. Archive and remove retained data only through a
separate, explicit operator-approved procedure.
