# RPC Gateway Overview

Status: Implemented / Mainnet Candidate / public prototype exposure

`alvenqis-rpc-gateway` serves canonical node, chain, account, mempool, indexer,
P2P-summary, transaction-submission, and mining-template APIs. It reads and
validates through current Alvenqis libraries; it does not define consensus.

## Exposure profiles

- `local`: read, submission, detailed P2P, and mining routes;
- `public-read`: public read routes only;
- `public-submit`: public reads plus signed transaction submission; mining
  routes return HTTP 410 and cannot be enabled by configuration;
- `private-mining`: mining routes for an unpublished container-network
  listener used by the optional pool role.

The accepted deployment policy retires public HTTP mining and keeps solo mining
on loopback. The application router, VPS gateway, role renderer, public smoke,
private pool smoke, desktop defaults, and operator documentation now implement
that contract. Deployment evidence is still required before assuming a running
rehearsal host has the reviewed revision. `/p2p/status` remains registered on
the shared router and exposes detailed peer telemetry; that separate security
finding remains open.

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
- transaction-submission authentication remains a separate production policy
  item; public mining is unavailable regardless of token configuration.

Production readiness still requires authenticated/abuse-tested public policy,
multi-host soak, storage review, monitoring, and external security review.
