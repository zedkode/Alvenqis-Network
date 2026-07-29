# Production Risks

Status: Mainnet Candidate risk register / not public Mainnet

## Consensus and storage

- Canonical node storage uses accepted transactional SQLite with WAL,
  `synchronous=FULL`, versioned strict schema, integrity checks, online backup,
  and preserved JSONL migration input. Independent restore, disk-failure, and
  long-running multi-process soak evidence is still required.
- Node databases must remain on local filesystems; network filesystems with
  unreliable locking or sync semantics are unsupported.
- The indexer uses transactional SQLite, but reorg recovery still performs an
  O(n) rebuild and RPC cache invalidation still fingerprints the legacy JSON
  path. Pool persistence is not a final transactional production design.
- Fork choice/reorganization, detached-block archival, header-first
  synchronization, and bounded resume exist. Deep durable pre-adoption branch
  storage and multi-host adversarial soak remain open.
- Stable block/transaction serialization and independent genesis verification
  are incomplete.

## Network and public APIs

- libp2p uses Noise/Yamux and network/genesis handshakes, but production peer
  reputation, bans, discovery diversity, topology soak, and DDoS evidence remain.
- Public HTTPS RPC has application exposure profiles, CORS, request limits, and
  reverse-proxy rate limits; authenticated abuse-tested public policy is not
  complete.
- Accepted policy requires public `/mining/*` routes to return HTTP 410 and
  keeps solo mining on loopback. Repository application, gateway, role, smoke,
  desktop, and documentation surfaces now agree; the running rehearsal host
  still requires an immutable-revision probe before deployment closure.
- Peer/miner/hashrate figures are observed local telemetry, not global truth.

## Wallet and clients

- Platform keystores reduce desktop key exposure, but recovery, migration,
  funded end-to-end flows, hardware signing, and external review remain.
- The wallet CLI's storage boundary must not be presented as equivalent to an
  audited production keystore.
- Android/browser/remote control remain constrained prototypes; no secret or
  privileged operation may move into an unauthenticated renderer/API.

## Miner and pool

- CUDA/core parity is tested, but wider GPU/driver/platform diversity and
  independent native-code review remain.
- Pool admission is process-local and storage is not transactional production
  storage; multi-instance controls, DDoS testing, reorg soak, and offline/HSM
  payout signing remain mandatory.
- VPS nodes must never install miners or wallet secrets.

## Release and operations

- Candidate checksums are integrity evidence, not publisher identity. Windows,
  Linux, and updater artifacts remain unsigned until native signing is complete.
- Installer upgrade/uninstall data-retention, rollback, and interruption paths
  require hands-on platform QA.
- At least three independent hosts, documented backup/restore drills, alerting,
  incident ownership, external security review, and explicit go-live sign-off
  remain G4 blockers.

See `../release/NETWORK_MATURITY.md` for gate ownership and
`../DOCUMENTATION_POLICY.md` for truth/claim rules.
