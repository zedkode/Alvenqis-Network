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
| ALV-RPC-001 | High | Accepted public-mining policy and the current gateway/RPC/smoke configuration disagree. | `Blockchain-prototype/alvenqis-release/vps-control-plane/docker/gateway/nginx.conf.template`; `compose/rpc.yaml`; `scripts/smoke-public-candidate.sh`; private decision register. | One reviewed public/private capability contract, matching configuration, negative route tests, and deployed probes. |
| ALV-RPC-002 | High | No independently operated RPC availability or project-endpoint outage proof exists. | Independent `rpc` role exists in `compose/roles.json`, but no clean-host or outage evidence is recorded. | Clean-host independent RPC deployment, client proof without project fallback, and project-endpoint outage rehearsal. |
| ALV-OPS-001 | High | The fleet admin surface has no viewer/operator authorization split and no agent-controller mTLS. | `Blockchain-prototype/alvenqis-release/vps-control-plane/admin-server/src/app.rs`. | Route-level RBAC tests, separate credentials, client-certificate validation, revocation, and negative authorization tests. |
| ALV-OPS-002 | High | The public project stack still has single-host, single-domain, and single-edge dependencies. | Public deployment records and project-operated control-plane profile. | Multi-host topology with independently controlled failure domains and no mandatory project service for participation. |
| ALV-OPS-003 | High | No clean-host independent node installation report exists. | Role overlays and static validation exist; `INDEPENDENT_NODE_OPERATOR_GUIDE.md` remains unverified. | Fresh-host transcript, pinned artifacts, node health, independent peer connectivity, backup, restore, and uninstall evidence. |
| ALV-MIN-001 | Medium | The reference pool is project-operated, single-instance, and prototype-only. | `Blockchain-prototype/alvenqis-mining-pool/`; `compose/pool.yaml`. | Independently operated pool evidence, self-hosted solo-mining proof, abuse tests, and production payout controls. |
| ALV-MIN-002 | Medium | Pool TLS certificate automation is Cloudflare DNS-01 specific. | `Blockchain-prototype/alvenqis-release/vps-control-plane/compose/pool.yaml`. | Operator-supplied certificate support or a provider-neutral ACME path, with renewal and failure tests. |
| ALV-DATA-001 | High | Indexer and pool persistence are not final production transactional storage designs. | `Blockchain-prototype/alvenqis-indexer/`; `Blockchain-prototype/alvenqis-mining-pool/`. | Approved storage decisions, migrations, corruption recovery, backup/restore, and reorg evidence. |
| ALV-REL-001 | High | No immutable external-audit commit, external review result, or signed native artifact set exists. | `EXTERNAL_SECURITY_REVIEW_SCOPE.md`; `../release/NETWORK_MATURITY.md`. | Pinned audit commit, completed independent review, tracked remediation, signed artifacts, and explicit gate approval. |

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
