# Protocol Implementation and Maturity Phases

Status: Current phase map / historical phase details live in project memory

The numbered development phases that created the prototype are complete history,
not a description of current missing features. Current work is controlled by
maturity gates and `../../TASK_MASTER.md`.

## Implemented candidate baseline

- account ledger, addresses, signing, wallet tooling, mempool, fees, emission;
- FiroPoW, LWMA, templates, CUDA miner, pool prototype;
- transactional SQLite chain storage, P2P v3, cumulative-work fork choice,
  bounded reorganization, and detached-block archival;
- RPC, index synchronization, explorer, website, SDKs;
- Tauri Windows/Linux Control Center and platform packaging;
- VPS node/RPC/indexer/admin/pool control plane with non-mining nodes;
- versioned genesis review/approval, checkpoints, and release gates.

## Current maturity work

### G1 — candidate hygiene

Formatting, tests, lint, dependency/security scans, documentation audit, and
reproducible Windows/Linux/VPS candidate packaging.

### G2 — controlled rehearsal

Config validation, operator runbooks, backups, rollback, updater failure paths,
and candidate endpoint health.

### G3 — multi-host evidence

Header-first/fork-aware synchronization, peer reputation, node SQLite
restore/disk-failure review, indexer/pool storage, multi-host soak,
pool maturity/reorg/payout exercises, RPC abuse tests,
and platform upgrade/data-retention QA.

### G4 — public Mainnet approval

Independent genesis and security review, signed native artifacts, production
operations ownership, external evidence, and explicit named go-live approval.

## Deferred product phases

Smart contracts, VRC standards, Passport, marketplace, staking, DAO, and broader
off-chain products remain planned/research. They do not become active merely
because a folder, specification, or website page exists.
