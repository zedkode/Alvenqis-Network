# alvenqis-indexer

Status: Draft / Mainnet Candidate / Prototype

Primary candidate commands:
- `index-chain` — full rebuild from chain JSONL
- `sync` — rebuild **only when** chain tip ≠ index tip (reorg-safe)
- `watch --interval-seconds 5` — continuous poll + sync (operator / VPS helper)
- `status` — includes `chain_height`, `in_sync`, `lag_blocks` when chain path is set
- `find-block` / `find-tx` / `find-address` / `latest-block`

Current storage:
- candidate default index path: user home `.alvenqis-mainnet/indexer/`
- local snapshot: `index.json` written **atomically** (crash-safe)

Reorg / tip change:
- `ensure_index_matches_chain` / `sync` compare tip hash + height and full-rebuild when they diverge
- RPC gateway calls this before serving `/indexer/*` routes
- not yet incremental (detach ancestor only) — full rebuild is correct, O(n)

Current limitations:
- no public production deployment claim;
- full rebuild (not incremental segment attach);
- no production database;
- no live public-network claim.
