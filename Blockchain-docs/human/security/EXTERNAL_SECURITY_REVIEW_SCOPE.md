# External Security Review Scope

Status: Draft audit package / not yet ready for external sign-off

This file prepares an immutable review scope for a Rust Layer 1 security
assessment or bug-bounty/audit-competition intake. It does not state that an
audit has started or passed.

## Snapshot requirement

Before an engagement starts, the audit coordinator must record:

| Field | Required value |
|---|---|
| Git repository | `https://github.com/zedkode/Alvenqis-Network` |
| Audit branch or tag | Immutable reviewed ref |
| Audit commit SHA | Full 40-character commit |
| Cargo lock hash | SHA-256 of `Blockchain-prototype/Cargo.lock` |
| Documentation inventory hash | SHA-256 of `Blockchain-docs/human/DOCUMENTATION_INVENTORY.md` |
| Build environment | Rust, Node, CUDA, OS, and Docker versions |
| In-scope deployment profile | Exact role overlays and environment template |

No audit commit is designated by this document. Working-tree or moving-branch
reviews are not accepted as release evidence.

## Primary assets in scope

| Priority | Asset | Paths | Review focus |
|---|---|---|---|
| P0 | Consensus, state, serialization, economics | `Blockchain-prototype/alvenqis-core/` | Determinism, issuance, fees, signatures, replay, checkpoints, overflow, fork invariants |
| P0 | Node, storage, mempool, P2P, sync | `Blockchain-prototype/alvenqis-node/` | Corruption, migration, reorg, eclipse, Sybil, bounded resources, network identity |
| P0 | Wallet and desktop signing boundary | `Blockchain-prototype/alvenqis-wallet/`, `Blockchain-prototype/alvenqis-desktop-v2/src-tauri/` | Key custody, recovery, confirmation, command exposure, update integrity |
| P1 | RPC gateway | `Blockchain-prototype/alvenqis-rpc-gateway/` | Capability profiles, parsing, quotas, CORS, upstream trust, mining boundaries |
| P1 | Miner and pool | `Blockchain-prototype/alvenqis-miner/`, `Blockchain-prototype/alvenqis-mining-pool/` | Work integrity, CUDA/core parity, Stratum TLS, share accounting, payout safety |
| P1 | Release and control plane | `Blockchain-scripts/`, `Blockchain-prototype/alvenqis-release/vps-control-plane/` | Installer, secrets, RBAC, mTLS, container isolation, backups, upgrades, supply chain |
| P2 | Indexer, explorer, website, SDKs | `Blockchain-prototype/alvenqis-indexer/`, `Blockchain-prototype/alvenqis-explorer/`, `Blockchain-prototype/alvenqis-website/`, `Blockchain-prototype/alvenqis-sdk*/` | Reorg correctness, untrusted rendering, API parity, false-data boundaries |

## Explicitly out of scope

- `Blockchain-prototype/alvenqis-release/vps/` because it is frozen legacy code;
- future product tracks documented as Planned or Deferred because they are not
  active implementations;
- generated artifacts, local runtime data, caches, staging, editor files, and
  private workspace instructions;
- the unresolved 2,500 ALVE validator-threshold proposal;
- any checkpoint relaxation not already approved through the checkpoint policy;
- smart contracts, staking, DAO, marketplace, Passport, and NFT behavior,
  because those products are not implemented.

Out-of-scope code must not be deployed as if it were covered by the review.

## Known issues supplied to reviewers

Reviewers must receive `KNOWN_LIMITATIONS.md`,
`../release/DECENTRALIZATION_READINESS.md`, `THREAT_MODEL.md`, and all prior
audit reports selected by the coordinator. Known findings remain in scope for
impact analysis unless the engagement contract explicitly excludes them.

## Reproduction commands

```bash
node Blockchain-scripts/docs/audit-docs.mjs
bash Blockchain-scripts/security/check-secrets.sh
bash Blockchain-scripts/security/check-repo-hygiene.sh
bash Blockchain-scripts/security/check-config-safety.sh
bash Blockchain-prototype/alvenqis-release/vps-control-plane/scripts/validate-stack.sh

cd Blockchain-prototype
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --release --locked
```

CUDA review additionally requires the supported NVIDIA toolchain and physical
GPU execution. A host-only test is not CUDA mining evidence.

## Reviewer deliverables

- severity-ranked findings with exact commit and paths;
- runnable proof of concept or deterministic reproduction where safe;
- affected invariants and realistic impact;
- remediation recommendation without silently changing consensus;
- retest result on a separate remediation commit;
- list of unreviewed paths, unreachable code, environment blockers, and
  assumptions.

## Project response contract

Every accepted finding receives an owner, status, remediation commit, test,
retest evidence, and disclosure decision. A clean report means only that no
additional finding was reported within the agreed scope and methods; it never
means zero vulnerabilities.
