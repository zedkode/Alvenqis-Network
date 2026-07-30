# Verification and Audit Status

Status: **Current internal verification record / no external audit**

Last reviewed: 2026-07-29

## Claim boundary

Alvenqis Network has automated tests, static checks and internal review
material. It has **not** completed an independent external security audit, formal
verification or public Mainnet launch review.

This page records evidence and open boundaries. It must not be used to claim
that the project has zero vulnerabilities or is safe for real funds.

## Current verification surface

| Area | Current evidence | Current boundary |
|---|---|---|
| Documentation | English-content scanner, local-link audit and generated publication inventory | Correct structure does not prove protocol correctness |
| Repository hygiene | Secret, forbidden-file, config-safety and workflow-pinning scanners | Pattern scanners do not replace credential rotation or human review |
| Rust workspace | Formatting, workspace tests, strict Clippy, RocksDB feature checks, deterministic FiroPoW search, and frozen candidate-genesis proof validation | G1 must be green on one immutable commit |
| RPC capability model | Profile validation and route tests separate public RPC from local/private mining; point tip queries and incremental cache-extension tests cover the read path | Deployed public endpoints require matching runtime probes |
| Indexer synchronization | Append-only incremental indexing is compared with a full rebuild; reorgs retain the full-rebuild fallback | Long-running deployed catch-up and recovery still require runtime evidence |
| Web products | Explorer and website lint/build/test jobs | Build success does not prove public service availability |
| VPS control plane | Static validation, Compose role rendering and runtime image build | Independent clean-host and multi-host operation remain later-gate evidence |
| Desktop | Type checks, unit tests, web build and native Rust tests | Platform packaging and signing remain separate release evidence |
| Supply chain | GitHub Actions are pinned to full commit SHAs; the node depends directly on the required rust-libp2p sub-crates and `Cargo.lock` excludes the unused vulnerable Hickory DNS path | Maintainer keys, dependency review, and release signing require operational controls |

## G1 evidence policy

G1 is complete only when:

1. the canonical local release gate exits successfully;
2. the equivalent GitHub Release Gate and Rust CI are green on the same commit;
3. public documentation and capability claims match that commit;
4. the commit identifier and command results are recorded without changing the
   network maturity label.

The evidence record will link the immutable commit after those conditions are
met.

## Open security and decentralization work

The current open boundaries include:

- no independent external security audit;
- incomplete independent bootstrap and active discovery evidence;
- incomplete IP, subnet and ASN-aware P2P admission controls;
- incomplete independent clean-host and project-outage rehearsals;
- incomplete independent pool and payout-signer evidence;
- incomplete update-signing and production incident evidence;
- retained genesis checkpoint enforcement under the documented policy;
- maintainer-led protocol governance rather than on-chain governance.

See:

- [Known Limitations](KNOWN_LIMITATIONS.md)
- [Production Risks](PRODUCTION_RISKS.md)
- [Threat Model](THREAT_MODEL.md)
- [External Security Review Scope](EXTERNAL_SECURITY_REVIEW_SCOPE.md)
- [Decentralization Readiness](../release/DECENTRALIZATION_READINESS.md)
- [Network Maturity](../release/NETWORK_MATURITY.md)

## Reproduction commands

```bash
node Blockchain-scripts/docs/check-english-content.mjs
node Blockchain-scripts/docs/audit-docs.mjs
bash Blockchain-scripts/security/check-secrets.sh
bash Blockchain-scripts/security/check-repo-hygiene.sh
bash Blockchain-scripts/security/check-config-safety.sh
bash Blockchain-scripts/security/check-workflow-pinning.sh
bash Blockchain-scripts/release/release-gate.sh
```

Windows release-gate equivalent:

```powershell
.\Blockchain-scripts\release\release-gate.ps1
```

## Reporting

Potential vulnerabilities should follow the private process in
[Responsible Disclosure](RESPONSIBLE_DISCLOSURE.md). Do not publish secrets,
wallet material or exploit details in a public issue.
