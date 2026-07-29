# Decentralization Readiness Register

Status: Draft readiness register / all closure claims evidence-gated

The project goal is to remove every mandatory project-controlled dependency
from validating, synchronizing, reading, submitting, mining, recovering, and
independently operating the network. This document reports progress; it does
not label the current network decentralized.

## Exit definition

The target is reached only when an independent operator can:

1. build or verify pinned artifacts without private project access;
2. start a validating node without a project-owned seed, RPC, domain, tunnel,
   pool, controller, or credential;
3. discover and maintain diverse peers across independent failure domains;
4. expose an operator-owned RPC and explorer without project infrastructure;
5. solo mine through a local node and optionally choose among independently
   operated pools;
6. back up, restore, upgrade, and recover without project intervention;
7. participate under the same consensus validation rules as project nodes;
8. verify protocol changes through a documented process that cannot silently
   alter consensus or checkpoint policy.

## Current centralization inventory

| Row | Point | Current state | G1 change | Closure gate |
|---:|---|---|---|---|
| 1 | Single project public host/domain/edge | **Open** | Role-based Compose overlays and project-neutral defaults reduce packaging coupling. The project deployment remains one failure domain and no independent clean-host proof exists. | G2–G4 |
| 2 | Single unpinned seed and no active discovery | **Open** | No closure. Candidate config still names one project DNS seed. | G2 |
| 3 | Peer admission not bounded by IP/subnet/ASN | **Open** | Outbound reservation and per-PeerId controls exist; source-network admission does not. | G2–G3 |
| 4 | Single/degraded or dishonest public RPC dependency | **Open** | Independent RPC role packaging exists. Public mining capability policy is internally inconsistent and no independent RPC/outage proof exists. | G3 |
| 5 | Fleet administration lacks role separation | **Open** | Deployment roles are service roles, not human authorization roles. Viewer/operator RBAC and agent-controller mTLS remain absent. | G3 |
| 6 | Project-operated single pool | **Open** | Pool role is optional and independently configurable, but no independent pool or solo-path evidence exists. | G3 |
| 7 | Release-pinned checkpoints | **Open by deliberate safety policy** | No relaxation was attempted. | Documented policy path only |
| 8 | Maintainer-led repository governance | **Disclosed** | This is project governance, not P2P admission. No community governance process is implemented. | G4 disclosure/decision |
| 9 | Proposed 2,500 ALVE validator threshold | **Blocked / not implemented** | No change is authorized. | Separate explicit decision |
| 10 | No DAO or on-chain parameter governance | **Open and disclosed** | Maintainer review remains the current protocol-change path. | G4 disclosure/decision |

## Evidence required to close a row

- immutable commit or release reference;
- exact configuration and deployment role;
- automated negative and positive tests;
- exit code 0 from relevant validation commands;
- multi-host or clean-host transcript where infrastructure is involved;
- independent-operator evidence where independence is claimed;
- documentation and task-tracker update;
- owner gate approval.

No row is closed by writing this document. Current technical findings remain in
`../security/KNOWN_LIMITATIONS.md`.
