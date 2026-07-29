# RPC Endpoints

Status: Implemented / Mainnet Candidate / route availability depends on profile

The filename retains `_DRAFT` for link stability. The route list below reflects
the current Axum router in
`Blockchain-prototype/alvenqis-rpc-gateway/src/routes/mod.rs`.

## Public read routes

- `GET /health`
- `GET /network`
- `GET /status`
- `GET /sync/status`
- `GET /chain/tip`
- `GET /chain/height`
- `GET /addresses/{address}`
- `GET /addresses/{address}/balance`
- `GET /addresses/{address}/account`
- `GET /state`
- `GET /supply`
- `GET /blocks/latest`
- `GET /blocks/{height}`
- `GET /blocks/hash/{hash}`
- `GET /transactions/{hash}`
- `GET /mempool`
- `GET /mempool/status`
- `GET /indexer/status`
- `GET /indexer/overview?blocks={1..100}&transactions={1..200}`
- `GET /indexer/blocks?offset={n}&limit={1..100}`
- `GET /indexer/blocks/latest`
- `GET /indexer/blocks/{height}`
- `GET /indexer/blocks/hash/{hash}`
- `GET /indexer/transactions?offset={n}&limit={1..100}`
- `GET /indexer/addresses?offset={n}&limit={1..100}`
- `GET /indexer/tx/{hash}`
- `GET /indexer/address/{address}`
- `GET /indexer/summary` (unbounded compatibility snapshot; do not poll)
- `GET /p2p/status`

`/status` includes chain/index tip agreement, lag, and cumulative work. Account
responses expose ledger-backed balance/nonce plus current tip/base-fee context
for remote wallet composition. The dedicated indexer service is the sole index
writer and uses SQLite. RPC cache invalidation currently fingerprints the
legacy index path rather than the SQLite database, so cache freshness is an
open correctness finding. Frequently refreshed clients must use bounded
overview or paginated routes.

## Submission route

- `POST /transactions`

Available in `local` and `public-submit`. The gateway rejects coinbase,
malformed or cross-network addresses, invalid signatures/nonces/fees,
insufficient balances, duplicates, oversized bodies, and mempool overflow.

## Mining routes

- `GET /mining/template?miner_address=<alve1...>`
- `POST /mining/submit`

Routes are available in local/private-mining modes. Accepted policy keeps them
off the public edge, but the current reference gateway and public smoke still
expect exposure. Do not treat that mismatch as a supported capability.

Templates use protocol `alvenqis-mining-v1`, contain an immutable FiroPoW 0.9.4
candidate, expire after 90 seconds, and have unpredictable in-memory IDs. A
submit carries template ID, nonce, final hash, and FiroPoW mix hash. Node/core
recompute the work and perform complete state/chain validation before atomic
persistence. Results distinguish accepted, stale, and rejected work.

`/p2p/status` currently returns detailed peer entries, including network
addresses, scores, and uptime. This route needs an explicit public/private data
classification. Clients that only need aggregate progress should use
`/sync/status`.

## Explicit non-goals

- wallet signing or secret custody;
- unauthenticated remote process control;
- admin/fleet-control APIs;
- guaranteed globally complete peer/miner statistics;
- production-readiness claims for the current public prototype exposure.
