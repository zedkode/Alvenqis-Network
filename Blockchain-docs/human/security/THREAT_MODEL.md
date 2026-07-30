# Alvenqis Network Threat Model

Status: Draft / pre-audit / Mainnet Candidate

This document defines the current review model. It is not an external audit and
does not claim complete threat coverage.

## Security objectives

1. Invalid blocks and transactions never become canonical.
2. Network identity, genesis, checkpoints, and consensus rules cannot drift
   silently between nodes.
3. A peer, RPC client, miner, pool worker, or fleet operator cannot bypass core
   validation.
4. Wallet keys and seeds never cross into RPC, website, explorer, pool, or VPS
   control-plane trust domains.
5. A project-operated service outage does not become a mandatory network outage.
6. Recovery preserves canonical state integrity and does not accept an
   unverified copied database as consensus evidence.
7. Public interfaces fail closed and expose only the documented capability
   profile.

## Assets

| Asset | Source of truth | Primary failure impact |
|---|---|---|
| Consensus rules and serialization | `Blockchain-prototype/alvenqis-core/` | Chain split, invalid issuance, replay, or state divergence |
| Canonical chain and state | `Blockchain-prototype/alvenqis-node/` | Corruption, rollback, unavailable validation, or incorrect fork choice |
| P2P identity and topology | `Blockchain-prototype/alvenqis-node/src/p2p.rs` | Eclipse, Sybil saturation, partition, or metadata spoofing |
| RPC capability boundary | `Blockchain-prototype/alvenqis-rpc-gateway/` and control-plane gateway config | Unauthorized mutation/mining access, denial of service, or dishonest health |
| Wallet keys and signing intent | `Blockchain-prototype/alvenqis-wallet/` and `alvenqis-desktop-v2/` | Key theft, unauthorized signing, or wrong-network transfer |
| Mining work and shares | `alvenqis-miner/` and `alvenqis-mining-pool/` | Invalid work, share theft, payout fraud, or mining centralization |
| Release and operator authority | `Blockchain-scripts/` and `alvenqis-release/alvenqis-setup-external/` | Supply-chain compromise, fleet takeover, destructive upgrade, or data loss |

## Trust boundaries

- Core validation is authoritative; network, RPC, pool, desktop, and website
  inputs are untrusted.
- Node storage is local operator state. Another host's database is not a trusted
  synchronization mechanism.
- RPC is a capability gateway, not a source of consensus authority.
- Pool shares are off-chain accounting and do not change network validity.
- Fleet administration governs project-operated infrastructure only; it does
  not grant P2P admission or validator privilege.
- Public telemetry is a local observation, not proof of a global network total.
- Checkpoints are explicit early-network safety inputs governed by the
  checkpoint policy, not an undeclared update channel.

## Threat actors

- unauthenticated internet client;
- malicious or compromised P2P peer;
- Sybil operator controlling many PeerIds or source addresses;
- malicious miner or pool worker;
- compromised pool, RPC, seed, DNS, edge, or update infrastructure;
- compromised fleet credential or operator workstation;
- dependency or release supply-chain attacker;
- local malware with user-level or administrator-level access;
- accidental operator error during backup, restore, migration, or upgrade;
- maintainer error introducing consensus or documentation drift.

## Priority threat scenarios

| ID | Scenario | Existing controls | Open work |
|---|---|---|---|
| TM-P2P-01 | Seed compromise eclipses a new node. | Network/genesis handshake, bounded peer count, seed redial backoff. | PeerId pinning, independent seeds, active discovery, and topology tests. |
| TM-P2P-02 | Many PeerIds from one network source consume inbound capacity. | Per-PeerId connection limits and persisted reputation. | IP/subnet/ASN admission and abuse tests. |
| TM-RPC-01 | Public edge exposes a capability intended to remain private. | Application and gateway return HTTP 410 for public mining; pool mining RPC is Docker-private; route and static tests cover the boundary. | Prove the deployed revision, complete transaction-submit authentication policy, and run abuse tests. |
| TM-OPS-01 | One credential gains read and mutation access across the fleet. | Authentication boundary and action allowlist. | Viewer/operator RBAC, mTLS, revocation, and external audit anchoring. |
| TM-STO-01 | Corruption or unsafe restore changes canonical state. | SQLite integrity checks, transactional writes, RocksDB state checks, encrypted backup material. | Independent restore, disk-failure, migration, and long-duration tests. |
| TM-WAL-01 | Renderer or remote API obtains key custody or signs without clear intent. | Tauri command boundary and local key-handling design. | Recovery, signing-failure, permission, and external-review evidence. |
| TM-MIN-01 | Pool or RPC becomes the only practical mining path. | Local loopback solo design and optional pool role. | Self-hosted solo proof, independent pool proof, and project-outage rehearsal. |
| TM-REL-01 | A malicious artifact or mutable image is distributed. | Lockfiles, checksums, workflow pinning, explicit versions. | Reproducible builds, publisher signatures, signed update metadata, and independent verification. |

## Validation baseline

```powershell
Blockchain-scripts\security\check-secrets.ps1
Blockchain-scripts\security\check-repo-hygiene.ps1
Blockchain-scripts\security\check-config-safety.ps1
node Blockchain-scripts\docs\audit-docs.mjs
```

```bash
cd Blockchain-prototype
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The open findings and required evidence are tracked in
`KNOWN_LIMITATIONS.md`.
