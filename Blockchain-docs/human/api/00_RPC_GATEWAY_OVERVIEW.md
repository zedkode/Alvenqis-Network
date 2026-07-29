# RPC Gateway Overview

Status: Implemented / Mainnet Candidate / public prototype exposure

`alvenqis-rpc-gateway` serves canonical node, chain, account, mempool, indexer,
P2P-summary, transaction-submission, and mining-template APIs. It reads and
validates through current Alvenqis libraries; it does not define consensus.

## Exposure profiles

- `local`: read, submission, detailed P2P, and mining routes;
- `public-read`: public read routes only;
- `public-submit`: public reads plus signed transaction submission; mining is
  registered only when `expose_mining_endpoints = true`.

The accepted deployment policy retires public HTTP mining and keeps solo mining
on loopback. The current VPS gateway, RPC profile, and public smoke script do
not yet implement one consistent policy. `/p2p/status` is also registered on
the shared router and currently exposes detailed peer telemetry. Both are open
security findings, not supported public-boundary claims.

## Safety boundaries

- the container RPC binds on a private Docker network and has no host-published
  raw port; the gateway owns public ingress;
- application access mode removes disallowed routes, not only UI links;
- request bodies, templates, transactions, CORS origins, and mempool size are
  bounded;
- mining templates are random-ID, short-lived, immutable candidates;
- node/core recompute PoW and fully validate every submitted block;
- no RPC route receives or stores wallet private keys;
- observed peer/miner totals are the local node view, never a global census.
- write/mining authentication is fail-open when no API token is configured;
  production profiles must make the credential requirement explicit.

Production readiness still requires authenticated/abuse-tested public policy,
multi-host soak, storage review, monitoring, and external security review.
