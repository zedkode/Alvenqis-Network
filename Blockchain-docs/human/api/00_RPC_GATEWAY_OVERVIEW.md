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

The reference VPS deliberately enables public solo-mining template/submission
behind HTTPS and stricter reverse-proxy rate limits. This is a Mainnet Candidate
prototype exposure, not a claim that unauthenticated mining RPC has passed
production abuse testing. Detailed operator/P2P routes remain local.

## Safety boundaries

- non-local RPC binds to loopback behind the TLS reverse proxy;
- application access mode removes disallowed routes, not only UI links;
- request bodies, templates, transactions, CORS origins, and mempool size are
  bounded;
- mining templates are random-ID, short-lived, immutable candidates;
- node/core recompute PoW and fully validate every submitted block;
- no RPC route receives or stores wallet private keys;
- observed peer/miner totals are the local node view, never a global census.

Production readiness still requires authenticated/abuse-tested public policy,
multi-host soak, storage review, monitoring, and external security review.
