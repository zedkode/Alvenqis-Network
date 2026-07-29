# Mining Pool API

Status: Draft / Mainnet Candidate / Prototype

Protocol identifier: `alvenqis-pool-v1`.

## Public Endpoints

- `GET /health`
- `GET /api/v1/pool/status`
- `GET /api/v1/pool/history`
- `GET /api/v1/miners/{address}`
- `GET /api/v1/payouts`

Mining work and shares use `alvenqis-stratum-v1` over TLS, not public HTTP
`/work` or `/shares` routes.

The status response includes VarDiff policy, each worker's latest assigned difficulty, rejected requests, rate-limited requests and active process-local bans. It never exposes client IP addresses.

## Admin Endpoints

- `POST /admin/v1/payouts/prepare`
- `POST /admin/v1/payouts/{payout_id}/confirm`
- `POST /admin/v1/payouts/{payout_id}/cancel`

Admin endpoints require a bearer token loaded from a file outside the repository. Preparing a batch does not sign or broadcast transactions. Cancelling is allowed only while a batch is still prepared and restores its amounts to mature balances.

The project edge exposes read-only pool data. The service applies process-local
request and worker limits; public deployment additionally requires
reverse-proxy limits, distributed controls and upstream DDoS protection.
