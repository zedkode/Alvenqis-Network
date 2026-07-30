# Mainnet Candidate Checklist (G2)

Status: Draft / Mainnet Candidate / Prototype — not public Mainnet

Completing this checklist authorizes controlled operator rehearsal only. Public
Mainnet still requires every G4 criterion in `NETWORK_MATURITY.md` and an
explicit owner decision.

## G1 prerequisite

- [ ] `node Blockchain-scripts/docs/audit-docs.mjs` exits 0.
- [ ] The Windows or Bash G1 release gate exits 0 on the same commit.
- [ ] `alvenqis-setup-external/scripts/validate-stack.sh --require-docker` exits 0.
- [ ] The public source-information set is version-controlled and all local
      documentation links resolve.
- [ ] `KNOWN_LIMITATIONS.md` and `DECENTRALIZATION_READINESS.md` match the
      reviewed code/config snapshot.

## Immutable rehearsal snapshot

- [ ] Record the full Git commit and hashes of `Cargo.lock`, candidate configs,
      genesis approval, genesis block, and documentation inventory.
- [ ] Verify the deterministic genesis hash independently.
- [ ] Confirm the active config, review, approval, block artifact, and checkpoint
      all name the same candidate genesis.
- [ ] Resolve and test the legacy `docs/release/...` approval path without
      changing the approved genesis hash.
- [ ] Confirm `allow_mainnet_candidate = true` is required and reset remains
      refused for Mainnet Candidate data.

## Independent node rehearsal

- [ ] Start from a clean supported host before Docker installation.
- [ ] Verify pinned artifacts before installation.
- [ ] Install the explicit `node` role; do not rely on `full-stack`.
- [ ] Confirm the rendered role excludes project edge, website, explorer,
      monitoring, control panel, pool, wallet, and miner.
- [ ] Confirm local storage ownership, encryption key retention, resource limits,
      log rotation, and restart policy.
- [ ] Record node PeerId, candidate network identity, genesis, height, tip, and
      connected peer evidence.
- [ ] Complete backup and restore onto a second fresh host.
- [ ] Complete uninstall with an explicit retained-data decision.

## P2P decentralization add-on

- [ ] Configure three to five independently operated seeds.
- [ ] Pin every seed to its expected PeerId.
- [ ] Demonstrate bounded active discovery and seed failover.
- [ ] Demonstrate reserved outbound capacity.
- [ ] Enforce and test admission by IP/subnet/ASN in addition to PeerId.
- [ ] Capture multi-host sync, transaction propagation, reconnect, resume, and
      reorg evidence across independent failure domains.
- [ ] Demonstrate that project DNS/seed failure does not prevent an already
      configured independent node from operating.

## RPC, indexer, and explorer rehearsal

- [ ] Reconcile public submit/read, local solo-mining, private pool-mining, and
      detailed P2P capability profiles.
- [ ] Require write/mining authentication to fail closed when credentials are
      absent.
- [ ] Classify `/p2p/status` data and prove detailed peer telemetry is not
      exposed unintentionally.
- [ ] Fix and test SQLite-aware index cache freshness.
- [ ] Start an independently operated RPC/indexer/explorer profile with no
      hidden project endpoint fallback.
- [ ] Demonstrate project RPC outage while independent clients remain usable.

## Mining rehearsal

- [ ] Prove local loopback solo mining with a physical supported NVIDIA GPU.
- [ ] Prove remote Stratum TLS with certificate verification enabled.
- [ ] Keep public HTTP `/mining/*` unavailable under the accepted policy.
- [ ] Prove the pool role is optional and no VPS image contains a miner binary
      or wallet key.
- [ ] Document provider-neutral certificate input before claiming the pool role
      is infrastructure-independent.

## Operations and security

- [ ] Complete viewer/operator RBAC and negative authorization tests.
- [ ] Complete agent-controller mTLS, rotation, and revocation tests.
- [ ] Validate backup, restore, disk-pressure, process failure, bad upgrade,
      rollback, and incident procedures.
- [ ] Run RPC/P2P/Stratum abuse and concurrency tests.
- [ ] Record monitoring and alert evidence without treating telemetry as global
      network truth.

## G2 stop condition

Publish the evidence paths and stop. G2 does not authorize a public Mainnet,
external-audit sign-off, decentralization status, or zero-vulnerability claim.
