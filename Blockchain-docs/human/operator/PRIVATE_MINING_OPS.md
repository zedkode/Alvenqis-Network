# Secured Mining Operations

Status: **Mainnet Candidate / Prototype — not Mainnet Live**

Remote pool mining uses `alvenqis-stratum-v1` over verified TLS. Public HTTP
template/share endpoints are retired. Local solo mining remains available only
through a loopback RPC.

## Security boundaries

| Surface | Rule |
|---|---|
| Public edge | Caddy returns HTTP 410 for every `/mining/*` request |
| Pool work/share | `stratum+tls://stratum.dohotstudio.com:3333` |
| Pool HTTPS | Read-only status, history, miner and payout views |
| Solo mining | Loopback RPC only, normally `http://127.0.0.1:10787` |
| VPS compute | No `alvenqis-miner` binary; no CPU/GPU hashing process |
| Pool upstream | The single RPC gateway over Docker-only networking, no host port |
| Product compute | NVIDIA CUDA only; no CPU/OpenCL fallback |

## VPS pool profile

```bash
# .env
ENABLE_POOL=true
POOL_ADDRESS=alve1...
STRATUM_HOST=stratum.dohotstudio.com
STRATUM_PORT=3333

./scripts/backup-now.sh
./scripts/install-docker-stack.sh
./scripts/health-check-docker.sh
./scripts/smoke-private-mining.sh
```

The `pool` profile starts:

- `alvenqis-pool`, exposing TCP 3333 with native TLS;
- `stratum-certbot`, issuing/renewing the certificate with Cloudflare DNS-01.

The pool obtains templates from the existing `alvenqis-rpc` service over the
private Docker network. There is no separate `alvenqis-mining-rpc` service.
The gateway is not a miner, and the VPS runtime Dockerfile intentionally does
not build or copy `alvenqis-miner`.

## Cloudflare routing

- `stratum.dohotstudio.com` is an unproxied DNS-only A record to the VPS.
- Standard Cloudflare Tunnel remains for HTTP services and the website.
- `dohotstudio.com` and `www.dohotstudio.com` are tunnel hostnames routed to
  the configured website runtime (`WEBSITE_ORIGIN`).
- Do not place arbitrary Stratum TCP behind a standard HTTP Tunnel route.

## Desktop configuration

Both Control Center applications default to:

```toml
[source]
kind = "stratum"
host = "stratum.dohotstudio.com"
port = 3333
use_tls = true
skip_tls_verify = false
worker_name = "desktop-01"
password = ""
timeout_seconds = 20
```

Saved legacy HTTP-pool sessions migrate to Stratum TLS. Remote plaintext
Stratum and disabled certificate verification are rejected.

## Local solo

Use only with a local candidate node:

```powershell
curl.exe -sS http://127.0.0.1:10787/health
curl.exe -sS "http://127.0.0.1:10787/mining/template?miner_address=YOUR_ALVE_ADDRESS"
```

The desktop backend rejects a non-loopback solo mining RPC.

## Verification

```bash
./scripts/smoke-public-candidate.sh
./scripts/smoke-private-mining.sh
```

Required evidence:

- public `/mining/template` returns 410;
- Stratum certificate validates for the configured hostname;
- pool `/health` reports `"stratum_tls": true` and
  `"http_mining_api": false`;
- pool statistics remain reachable through HTTPS;
- the public edge returns HTTP 410 for `/mining/*`;
- Docker image inspection shows no `alvenqis-miner` binary.

## Forbidden

- publishing the private RPC container or bypassing the Caddy route boundary;
- remote `stratum+tcp` without TLS;
- disabling TLS verification on a remote endpoint;
- CPU/OpenCL fallback or VPS hashing;
- claiming Mainnet Live before the release gates are complete.
