# Known Limitations and Open Security Findings

Status: Open risk register / Mainnet Candidate / not an audit result

This register lists verified gaps that must be visible to maintainers,
operators, and external reviewers. An item remains open until a technical
change, a failing-then-passing test, an exit-code-zero verification command, and
deployment evidence where applicable are linked from this file.

## Release-blocking findings

| ID | Severity | Open finding | Current evidence | Closure evidence required |
|---|---|---|---|---|
| ALV-NET-001 | High | Candidate bootstrap defaults to one unpinned project DNS seed and has no active peer discovery. | `Blockchain-prototype/configs/mainnet-candidate.toml`; `Blockchain-prototype/alvenqis-node/src/p2p.rs` uses unknown-PeerId dialing. | Three to five independently operated PeerId-pinned seeds, bounded active discovery, failover tests, and live reachability evidence. |
| ALV-NET-002 | High | Inbound admission is bounded per PeerId, not per IP, subnet, or ASN. | `Blockchain-prototype/alvenqis-node/src/p2p.rs` connection-limit configuration. | Per-source admission controls, Sybil/load tests, and preserved outbound capacity. |
| ALV-NET-003 | High | The affected `yamux 0.12.1` path is locally remediated at `80dc72b5e54a`: a reviewed compatibility adapter retains the 32-stream cap while resolving only fixed `yamux 0.13.10`. Immutable GitHub and Dependabot verification are pending. | Both Cargo graphs exclude `yamux 0.12.1`; the malformed-frame regression, multi-node P2P tests, workspace tests, strict Clippy, and fresh dependency scans pass; see `YAMUX_BACKPORT_EVIDENCE_2026-07-30.md`. | Green same-commit GitHub checks, successful Dependabot re-check, and eventual replacement of the temporary backport by a reviewed published rust-libp2p upgrade. |
| ALV-RPC-001 | High | Repository capability policy is reconciled, but no probe proves the running rehearsal gateway uses that revision. | Application public profiles and the gateway return HTTP 410 for `/mining/*`; pool roles render Docker-private mining RPC; desktop solo defaults to loopback; route/static tests cover the contract. | Immutable commit with green G1 plus a deployed public HTTP 410 probe and Docker-private pool-template probe. |
| ALV-RPC-002 | High | No independently operated RPC availability or project-endpoint outage proof exists. | Independent `rpc` role exists in `compose/roles.json`, but no clean-host or outage evidence is recorded. | Clean-host independent RPC deployment, client proof without project fallback, and project-endpoint outage rehearsal. |
| ALV-RPC-003 | High | `/p2p/status` is registered on the shared router and exposes detailed peer addresses, scores, and uptime. | `Blockchain-prototype/alvenqis-rpc-gateway/src/routes/mod.rs`; node P2P status models. | Public/private telemetry schema, aggregate-only public route, authorization tests, and deployed negative probes. |
| ALV-RPC-004 | High | Write/mining authentication permits requests when no API token is configured. | `Blockchain-prototype/alvenqis-rpc-gateway/src/middleware/auth.rs`. | Fail-closed production configuration and negative tests for missing/malformed credentials. |
| ALV-OPS-001 | High | The fleet admin surface has no viewer/operator authorization split and no agent-controller mTLS. | `Blockchain-prototype/alvenqis-release/alvenqis-setup-external/admin-server/src/app.rs`. | Route-level RBAC tests, separate credentials, client-certificate validation, revocation, and negative authorization tests. |
| ALV-OPS-002 | High | The public project stack still has single-host, single-domain, and single-edge dependencies. | Public deployment records and project-operated control-plane profile. | Multi-host topology with independently controlled failure domains and no mandatory project service for participation. |
| ALV-OPS-003 | High | No clean-host independent node installation report exists. | Role overlays and static validation exist; `INDEPENDENT_NODE_OPERATOR_GUIDE.md` remains unverified. | Fresh-host transcript, pinned artifacts, node health, independent peer connectivity, backup, restore, and uninstall evidence. |
| ALV-MIN-001 | Medium | The reference pool is project-operated, single-instance, and prototype-only. | `Blockchain-prototype/alvenqis-mining-pool/`; `compose/pool.yaml`. | Independently operated pool evidence, self-hosted solo-mining proof, abuse tests, and production payout controls. |
| ALV-MIN-002 | Medium | Pool TLS certificate automation is Cloudflare DNS-01 specific. | `Blockchain-prototype/alvenqis-release/alvenqis-setup-external/compose/pool.yaml`. | Operator-supplied certificate support or a provider-neutral ACME path, with renewal and failure tests. |
| ALV-DATA-001 | High | Indexer and pool persistence are not final production transactional storage designs. | `Blockchain-prototype/alvenqis-indexer/`; `Blockchain-prototype/alvenqis-mining-pool/`. | Approved storage decisions, migrations, corruption recovery, backup/restore, and reorg evidence. |
| ALV-DATA-002 | High | RPC index-cache invalidation fingerprints the legacy index path while the indexer writes SQLite. | `alvenqis-rpc-gateway/src/cache/index_cache.rs`; `alvenqis-indexer/src/storage.rs`. | SQLite-aware invalidation or query boundary plus regression tests proving fresh reads after index updates. |
| ALV-DATA-003 | Medium | Canonical SQLite corruption is now detected at startup and periodically, but automatic quarantine/repair and equivalent state/indexer/pool coverage are not implemented. | Deep node integrity report, runtime `storage-integrity.json`, and failing-then-passing cached-body/Merkle corruption tests. | Operator-guided quarantine and rebuild workflow, independent corruption drills for each store, bounded large-chain cost evidence, and immutable CI results. |
| ALV-CLI-001 | High | Explorer, desktop, and SDK defaults depend on one project-operated endpoint set and have no quorum/failover policy. | Explorer API defaults, desktop settings/constants, and SDK network config. | Operator endpoint profiles, health-based rotation, TLS policy, no hidden project fallback, and project-outage tests. |
| ALV-CNS-001 | High | Candidate-genesis mining uses a first-thread-wins multi-threaded nonce search, so the generated block hash can differ by platform or scheduler and intermittently violate the pinned height-zero checkpoint. | `Blockchain-prototype/alvenqis-core/src/genesis.rs`; `Blockchain-prototype/alvenqis-core/src/firopow.rs`; `Blockchain-prototype/alvenqis-core/native/crypto/progpow/alvenqis_firopow_ffi.cpp`; failed Linux Rust CI checkpoint tests. | Separately approved consensus diff, deterministic nonce-selection proof across thread counts and platforms, unchanged or explicitly re-approved candidate genesis, and green checkpoint tests on Windows and Linux. |
| ALV-DESKTOP-001 | Medium | Tauri 2.11.5 on Linux transitively requires the unmaintained GTK3 stack and `glib 0.18.5`, which has RUSTSEC-2024-0429 in `VariantStrIter`. The application and resolved Tauri/GTK sources do not call the affected API; `cargo deny` carries one scoped, documented exception. | `Blockchain-prototype/alvenqis-desktop-v2/src-tauri/Cargo.lock`; `Blockchain-prototype/deny.toml`; dated dependency-audit report. | Upstream-compatible migration away from GTK3/glib 0.18 or a reviewed compatible backport, removal of the deny exception, Linux desktop build/test evidence, and fresh `cargo audit`/`cargo deny` results. |
| ALV-REL-001 | High | No immutable external-audit commit, external review result, or signed native artifact set exists. | `EXTERNAL_SECURITY_REVIEW_SCOPE.md`; `../release/NETWORK_MATURITY.md`. | Pinned audit commit, completed independent review, tracked remediation, signed artifacts, and explicit gate approval. |
| ALV-REL-002 | High | Candidate config retains a legacy genesis-approval path that does not match the repository layout. | `Blockchain-prototype/configs/mainnet-candidate.toml`; `../release/GENESIS.md`. | One canonical resolver path, startup tests from repository and packaged layouts, and unchanged approved genesis hash. |

## Deliberate or unresolved governance constraints

| ID | Status | Constraint |
|---|---|---|
| ALV-GOV-001 | Retained by policy | Early release-pinned checkpoints remain enforced. Relaxation may occur only through the documented checkpoint policy. |
| ALV-GOV-002 | Maintainer-led | Repository and protocol-change review is maintainer-led; no DAO or on-chain governance exists. |
| ALV-GOV-003 | Explicitly blocked | No 2,500 ALVE validator threshold is active. The question remains unresolved and must not be implemented as a side effect of other work. |

## Reporting rule

Do not convert this register into a statement that the network is
decentralized, audited, production-ready, or free of vulnerabilities. Update it
when evidence changes, and retain closed entries with their closure commit and
commands.
