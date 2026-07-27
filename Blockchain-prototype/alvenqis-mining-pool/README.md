# Alvenqis Mining Pool

**Status: Mainnet Candidate / Prototype. Not a live public mining pool.**

`alvenqis-mining-pool` coordinates miners without changing Alvenqis consensus. It obtains canonical block templates from RPC, lowers only the off-chain share target, validates every submitted hash, and forwards a candidate only when it also satisfies the real network target.

Implemented:

- HTTP work and share protocol `alvenqis-pool-v1`;
- address and worker-name validation;
- duplicate, stale, malformed and low-difficulty share rejection;
- per-worker variable difficulty with bounded targets and in-flight assignment safety;
- per-worker and trusted-proxy client rate limits, worker caps and temporary abuse bans;
- network-target candidate forwarding;
- persistent atomic JSON accounting;
- work-weighted PPLNS allocation in integer atomic units;
- configurable fee, maturity and minimum payout;
- immature, mature, pending and paid balances;
- authenticated payout batch preparation and confirmation;
- safe cancellation of an unsigned prepared payout batch;
- worker, estimated hashrate, block and payout dashboard.

Payout batches are accounting instructions. A separate offline/operator wallet must sign and submit payout transactions. The public process does not load a private key or recovery phrase.

## Run Locally

```powershell
Copy-Item alvenqis-mining-pool/config.example.toml alvenqis-mining-pool/config.toml
New-Item -ItemType Directory -Force .alvenqis-local/mining-pool
$token = -join ((1..64) | ForEach-Object { '{0:x}' -f (Get-Random -Maximum 16) })
Set-Content -NoNewline .alvenqis-local/mining-pool/admin.token $token
cargo run -p alvenqis-mining-pool -- --config alvenqis-mining-pool/config.toml
```

Configure `alvenqis-miner` with a pool source:

```toml
miner_address = "alve1..."
schema_version = 4
backend_mode = "cuda"
nonce_batch_size = 131072
gpu_intensity = 90
gpu_batch_size = 131072
gpu_devices = []
kernel_validation = true
template_refresh_seconds = 5
status_interval_seconds = 3

[source]
kind = "pool"
url = "http://127.0.0.1:30787"
worker_name = "desktop-01"
timeout_seconds = 10
```

Run `cargo run -p alvenqis-miner -- --config PATH mine`.

## API

- `GET /health`
- `GET /api/v1/pool/status`
- `GET /api/v1/work?miner_address=...&worker_name=...`
- `POST /api/v1/shares`
- `GET /api/v1/miners/{address}`
- `GET /api/v1/payouts`

Admin payout endpoints require `Authorization: Bearer TOKEN`.

- `POST /admin/v1/payouts/prepare`
- `POST /admin/v1/payouts/{payout_id}/confirm`
- `POST /admin/v1/payouts/{payout_id}/cancel`

VarDiff targets one accepted share per configured interval. The coordinator retains all difficulties issued to a worker for the lifetime of a job, accepts valid in-flight work and credits the highest target actually proven by the submitted hash.

### Multi-miner rewards (PPLNS)

- **Share target is always easier than network** (`share_network_gap_bits`, default 4). If share difficulty equals network difficulty, the fastest GPU finds every full block alone and other miners get almost nothing.
- Each found block is split **only among shares accepted since the previous pool block** (mining round), weighted by proven work `2^share_bits` (proxy for hashrate contribution).
- Rounding dust uses the **largest-remainder** method so leftover atomics are not dumped onto a single address.
- On-chain coinbase still pays the **pool address**; individual balances appear under pool accounts (`immature` → `mature` after confirmations) and require operator payout batches.

## Remaining Production Gates

- reorg-safe accounting tied to the final chain reorg model;
- production database and multi-coordinator leader election;
- distributed rate limiting and coordinated bans across multiple pool frontends;
- TLS deployment, DDoS protection and penetration testing;
- payout signer or HSM integration and legal/accounting review;
- multi-host soak tests and independent audit.
