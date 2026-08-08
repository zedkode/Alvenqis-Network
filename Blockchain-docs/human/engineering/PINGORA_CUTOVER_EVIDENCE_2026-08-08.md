# Pingora controlled-VPS cutover evidence — 2026-08-08

Status: G1 controlled deployment evidence; post-cutover soak remains open

Scope: the project-operated edge and current `alvenqis-setup-external` stack.
This was a G1 deployment verification, not an independent-operator or network
decentralization proof. It did not modify consensus code, chain data, wallets,
1Panel, Vaultwarden, or the frozen legacy `alvenqis-release/vps/` package.

## Pre-cutover baseline

The public RPC reported height 0, index lag 0, and tip
`0000c29213014578ac41a748c2be3489859f1e0b1f3555bd89b7e5301632a4c5`.
Vaultwarden was healthy. The existing Nginx gateway image digest was retained
under `alvenqis-gateway:rollback-nginx-20260808` before replacement.

## Build and configuration evidence

```text
bash scripts/compose.sh build gateway
Result: alvenqis-gateway:2.1.0-local built from the pinned Pingora source.

bash scripts/compose.sh run --rm --no-deps gateway --check-config
Result: gateway configuration valid: HTTP 0.0.0.0:8080,
mTLS 0.0.0.0:10443, metrics 0.0.0.0:9091.

bash scripts/compose.sh build alvenqis-node alvenqis-ops alvenqis-explorer \
  alvenqis-website alvenqis-metrics-exporter backup-scheduler
Result: all selected images built; Rust runtime release build completed.
```

The VPS runtime build emitted one expected warning that `nvcc` is unavailable.
The server runtime does not include or require the CUDA miner. The warning did
not fail the build.

## Cutover and route evidence

Only `gateway` and `cloudflared` were recreated for the initial edge cutover.
After the gateway canary passed, the remaining Alvenqis application services
were reconciled against the prebuilt images.

```text
GET  https://rpcnode.dohotstudio.com/health             -> 200
GET  https://rpcnode.dohotstudio.com/status             -> 200
POST https://rpcnode.dohotstudio.com/transactions {}    -> 422
GET  https://rpcnode.dohotstudio.com/mining/template    -> 410
POST https://rpcnode.dohotstudio.com/mining/submit {}   -> 410
POST https://fleet.dohotstudio.com/fleet/report {}      -> 426
```

The 410 response body is the Pingora JSON contract:
`{"error":"public mining routes are unavailable"}`. Gateway metrics recorded
the RPC, health, and denied-mining route counters. Gateway logs were structured
JSON with bounded route, method, request ID, duration, and status fields.

## Authentication and transport evidence

```text
Control request without Basic credentials              -> 401
Authenticated viewer GET /api/session                  -> 200, role=viewer
Authenticated operator GET /api/session                -> 200, role=operator
Authenticated viewer POST /api/invitations {}          -> 403

Fleet mTLS without a client certificate                -> TLS rejected
Fleet mTLS with an untrusted self-signed certificate   -> TLS rejected
Fleet mTLS with an ephemeral fleet-CA certificate      -> TLS accepted;
                                                         incomplete body 422
```

The ephemeral client key, CSR, and certificate were generated below `/tmp` and
removed by the command trap. The fleet CA private key was never mounted in the
gateway. The live gateway contract was:

```text
user=10001:10001
read-only root filesystem=true
privileged=false
capabilities dropped=ALL
security option=no-new-privileges:true
```

## Full-stack and Cloudflare evidence

`health-check-docker.sh` passed after deployment. It confirmed the node, RPC,
indexer, controller, pool, monitoring dependencies, private mining template,
Stratum TLS, encrypted RocksDB readiness, and selected container memory budget.
`smoke-public-candidate.sh` and `smoke-private-mining.sh` both passed.

`cloudflare-bootstrap.sh --activate` reused the existing tunnel and applied:

| Host group | Record/route |
|---|---|
| `node`, `fleet-mtls`, `stratum` | Direct `A` records to the VPS, DNS-only |
| `control`, `rpcnode`, `fleet`, `grafana`, `prometheus`, apex, `www`, `explorer`, `pool` | Proxied CNAME records to the Cloudflare Tunnel; origin `http://gateway:8080` |

The public health verifier then passed RPC, fleet, RPC pool, and pool endpoints.
No manual Cloudflare dashboard change remained necessary.

## State preservation and deployment incident

Post-deployment status remained height 0, index lag 0, and the exact same tip
hash as the pre-cutover baseline. Vaultwarden remained `running healthy`.

During the general application reconciliation, Docker Compose returned success
after recreating application containers but left them in `Created` rather than
starting them. This caused a temporary application availability interruption.
The condition was detected immediately with `docker ps -a`; the affected
services were explicitly started, every required container reached `healthy`,
and the complete health/public/private checks above were rerun successfully.

## Remaining evidence

- bounded post-cutover latency/resource soak and intentional upstream-outage
  isolation remain to be recorded;
- WebSocket compatibility and live state-changing fleet rotation/revocation
  rehearsals were not exercised during this deployment;
- independent-host installation, independent seeds, and project-outage proof
  remain later-gate work and are not implied by this cutover.
