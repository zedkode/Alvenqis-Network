# alvenqis-node

Status: Mainnet Candidate / Prototype / not public Mainnet

`alvenqis-node` owns canonical chain persistence, mempool admission, mining
templates, submitted-block validation, and libp2p synchronization around
`alvenqis-core` consensus.

## P2P v3 and fork choice

- TCP transport with Noise encryption and Yamux multiplexing;
- persistent node identity and handshake binding to protocol version, network
  ID, chain magic, and actual genesis hash;
- signed transaction and miner-presence gossip;
- independent seed dialing with exponential backoff and deterministic per-node
  jitter, plus discovery accounting learned only from identified Alvenqis
  connections;
- exponential block locators and header-first verification before bodies;
- bounded direct-extension and divergent-branch synchronization;
- adoption only after full validation and strictly greater cumulative work;
- deterministic equal-work retention;
- transactional SQLite canonical-chain replacement, detached-block archival,
  and detached-transaction mempool recovery;
- atomic, fail-closed peer reputation persistence, observed connected uptime,
  temporary or permanent operator bans, explicit unban, and refusal of banned
  peers before application admission;
- libp2p connection limits before handshake: at most 128 peers, eight outbound
  connections, 32 pending inbound connections, eight pending outbound
  connections, two links per peer, 32 Yamux streams per link, and four
  concurrent staged branch synchronizations.

All connected nodes must use P2P protocol v3. Staged reorganization is bounded
to 2,048 blocks. Deep-reorg recovery and multi-host soak remain production
gates.

The limits above are the supported ceiling for a combined 6 vCPU / 12 GB node
host. Lower `max_peers` for hosts that also run indexing or explorer workloads.
Values above 128 and more than 32 configured seeds are rejected at startup.

## Peer administration

Peer IDs are authenticated libp2p identities. Operator actions are written to
an atomic, locked queue and are applied by the P2P service; malformed
reputation or administration files stop startup instead of silently discarding
ban state.

```text
alvenqis-node --config configs/mainnet-candidate.toml --data-dir /var/lib/alvenqis/.alvenqis-mainnet/chain ban-peer 12D3KooW... --reason "protocol abuse" --duration-seconds 3600
alvenqis-node --config configs/mainnet-candidate.toml --data-dir /var/lib/alvenqis/.alvenqis-mainnet/chain unban-peer 12D3KooW...
```

`--duration-seconds 0` creates a permanent operator ban. Non-zero durations are
capped at 30 days. Operator actions do not fabricate positive or negative
rating events. `peers` reports observed uptime seconds, successful/failed
connections, validated handshakes, useful events, protocol faults, discovery
count, and active bans.

## Persistence and safety

- tip growth and validated reorganization use SQLite ACID transactions with
  WAL, `synchronous=FULL`, a versioned strict schema, and a 30-second busy timeout;
- reorganization archives detached blocks in `orphaned_blocks` before changing
  the canonical chain in the same transaction;
- legacy `chain.jsonl` data is structurally validated and migrated atomically to
  `chain.sqlite3`; the original JSONL remains untouched as rollback evidence;
- online backups use SQLite's backup API and are integrity-checked before success;
- the database must live on a local filesystem with correct locking and sync
  semantics; NFS/network-share placement is unsupported;
- candidate genesis review/approval and height-zero checkpoint are mandatory;
- wrong network/genesis, broken linkage, invalid PoW/difficulty/version/time,
  duplicate transactions, invalid state, and coinbase overpayment are rejected;
- Mainnet Candidate reset is unavailable and regeneration requires explicit
  `--force-genesis` review flow;
- `SqliteBlockStore` is the accepted cross-platform node backend. Independent
  backup/restore, disk-failure, and multi-host soak evidence remains required
  before G4.

## Mining integration

`create_block_template` creates a bounded immutable nonce-zero candidate.
`submit_mined_block` rechecks current tip, consensus work, state, and mempool
effects under lock before atomic persistence. Miners cannot select reward, fees,
timestamp, difficulty, transactions, or network identity.

## Primary commands

- `start-node`, `node-status`, `validate-chain`, `peers`, `shutdown`;
- `backup-chain-database`, `verify-chain-database`;
- `mempool-status`, `balance`, `state`, `submit-tx`;
- `print-genesis-hash`, `export-genesis-review`, `approve-genesis`,
  `genesis-approval-status`;
- operator/test block helpers documented by the CLI.

`configs/mainnet-candidate.toml` is the default product/operator configuration.
Devnet/Testnet configurations are internal test profiles.
