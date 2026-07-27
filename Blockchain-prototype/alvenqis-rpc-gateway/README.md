# alvenqis-rpc-gateway

Status: Mainnet Candidate / public prototype / not public Mainnet

The gateway exposes canonical chain, account, transaction, mempool, indexer,
sync, and optional mining APIs. It does not store wallet secrets or define
consensus.

## Exposure profiles

- `local`: public reads, signed submission, detailed P2P status, and mining;
- `public-read`: read routes only;
- `public-submit`: reads plus `POST /transactions`; mining requires explicit
  `expose_mining_endpoints = true`.

The reference VPS binds the Rust service to `127.0.0.1`, enables public-submit
plus solo mining, and exposes it through HTTPS with request/body/rate limits.
This deliberate public mining surface is candidate/prototype behavior and still
requires abuse testing before G4. Detailed `/p2p/status` remains local.

Mining templates use `alvenqis-mining-v1`, unpredictable in-memory IDs, immutable
candidate fields, and 90-second expiry. Submissions carry nonce, final hash, and
FiroPoW mix hash; node/core recompute and fully validate before persistence.

See `../docs/api/00_RPC_GATEWAY_OVERVIEW.md` and
`../docs/api/01_RPC_ENDPOINTS_DRAFT.md` for profiles, the current route list,
and safety boundaries.

Run locally:

```powershell
cargo run -p alvenqis-rpc-gateway -- --config configs/rpc.mainnet-candidate.toml
```
