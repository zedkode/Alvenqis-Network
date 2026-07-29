# alvenqis-rpc-gateway

Status: Mainnet Candidate / public prototype / not public Mainnet

The gateway exposes canonical chain, account, transaction, mempool, indexer,
sync, and optional mining APIs. It does not store wallet secrets or define
consensus.

## Exposure profiles

- `local`: public reads, signed submission, detailed P2P status, and mining;
- `public-read`: read routes only;
- `public-submit`: reads plus `POST /transactions`; mining returns HTTP 410 and
  cannot be enabled by configuration;
- `private-mining`: mining routes for an unpublished container network used by
  the optional pool role.

The reference VPS binds the Rust service only inside Docker. Its normal RPC
roles use `public-submit` with mining disabled. Pool roles render
`private-mining`; the public gateway returns HTTP 410 for every `/mining/*`
request. Solo mining uses local loopback RPC, while remote miners use verified
Stratum TLS. Detailed `/p2p/status` exposure remains a separate open finding.

Mining templates use `alvenqis-mining-v1`, unpredictable in-memory IDs, immutable
candidate fields, and 90-second expiry. Submissions carry nonce, final hash, and
FiroPoW mix hash; node/core recompute and fully validate before persistence.

See `../../Blockchain-docs/human/api/00_RPC_GATEWAY_OVERVIEW.md` and
`../../Blockchain-docs/human/api/01_RPC_ENDPOINTS_DRAFT.md` for profiles, the
current route list, and safety boundaries.

Run locally:

```powershell
cargo run -p alvenqis-rpc-gateway -- --config configs/rpc.mainnet-candidate.toml
```
