# Mining Pool API

Status: Draft / Mainnet Candidate / Prototype

Protocol identifier: `alvenqis-pool-v1`.

## Public Endpoints

- `GET /health`
- `GET /api/v1/pool/status`
- `GET /api/v1/work?miner_address=ADDRESS&worker_name=NAME`
- `POST /api/v1/shares`
- `GET /api/v1/miners/{address}`
- `GET /api/v1/payouts`

A normal accepted share returns `pending_local`; a network-valid block accepted by the upstream node returns `accepted` with a block height.

The status response includes VarDiff policy, each worker's latest assigned difficulty, rejected requests, rate-limited requests and active process-local bans. It never exposes client IP addresses.

## Admin Endpoints

- `POST /admin/v1/payouts/prepare`
- `POST /admin/v1/payouts/{payout_id}/confirm`
- `POST /admin/v1/payouts/{payout_id}/cancel`

Admin endpoints require a bearer token loaded from a file outside the repository. Preparing a batch does not sign or broadcast transactions. Cancelling is allowed only while a batch is still prepared and restores its amounts to mature balances.

The VPS proxy exposes the pool under `/pool/`. The service applies process-local request and worker limits; public deployment additionally requires reverse-proxy limits, distributed controls and upstream DDoS protection.
