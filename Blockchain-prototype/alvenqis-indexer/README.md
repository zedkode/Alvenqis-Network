# alvenqis-indexer

Status: Draft / Mainnet Candidate / Prototype

Primary candidate commands:
- `index-chain` — full rebuild from canonical chain storage
- `sync` — append when the indexed tip is the canonical parent; full rebuild
  after a reorg or incompatible tip
- `watch --interval-seconds 5` — continuous poll + sync (operator / VPS helper)
- `status` — includes `chain_height`, `in_sync`, `lag_blocks` when chain path is set
- `find-block` / `find-tx` / `find-address` / `latest-block`

Current storage:
- candidate default index path: user home `.alvenqis-mainnet/indexer/`
- primary index: `index.sqlite3` with WAL and transactional full replacements
- legacy `index.json`: imported once when present and retained as a rollback
  artifact; it is not the active store

Reorg / tip change:
- `ensure_index_matches_chain` / `sync` compare tip hash, height, and parent links
- canonical append-only advancement is incremental
- reorg or parent mismatch triggers a correct O(n) full rebuild
- RPC gateway calls this before serving `/indexer/*` routes

Current limitations:
- no public production deployment claim;
- reorg recovery uses an O(n) rebuild rather than incremental detach/attach;
- RPC index-cache invalidation still fingerprints the legacy JSON path;
- no multi-process soak or independent corruption/restore evidence;
- no live public-network claim.
