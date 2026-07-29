# Alvenqis 03 — Roadmap and Repository Structure

Status: Canonical maturity-gated roadmap summary

The private detailed execution tracker is `../../internal/TASK_MASTER.md`. This document defines
the order and truth boundaries, not a second independent backlog.

## Repository structure

| Area | Canonical paths |
|---|---|
| Protocol | `Blockchain-prototype/alvenqis-core/`, `Blockchain-docs/human/protocol/`, `Blockchain-prototype/shared/` |
| Node/network | `Blockchain-prototype/alvenqis-node/`, `Blockchain-prototype/configs/` |
| APIs/data | `Blockchain-prototype/alvenqis-rpc-gateway/`, `Blockchain-prototype/alvenqis-indexer/`, `Blockchain-prototype/alvenqis-explorer/` |
| Mining | `Blockchain-prototype/alvenqis-miner/`, `Blockchain-prototype/alvenqis-mining-pool/`, `Blockchain-docs/human/mining/` |
| Clients | `Blockchain-prototype/alvenqis-desktop-v2/`, `Blockchain-prototype/alvenqis-wallet/`, `Blockchain-prototype/alvenqis-android/`, `Blockchain-prototype/alvenqis-browser/` |
| Website | `Blockchain-prototype/alvenqis-website/` |
| Operations | `Blockchain-scripts/`, `Blockchain-prototype/alvenqis-release/vps-control-plane/`, `Blockchain-docs/human/operator/` |
| Maturity | `Blockchain-docs/human/release/`, `Blockchain-docs/internal/memory/` |

All active component names use `alvenqis-*`. References to the retired Veiron or
Vireon branding are historical errors and must not guide new paths or package names.

## Maturity order

1. G0: specification and open-decision clarity.
2. G1: formatting, tests, lint, security scans, reproducible candidate builds.
3. G2: controlled operator rehearsal with correct configs and rollback.
4. G3: multi-host candidate evidence, abuse tests, recovery, and platform QA.
5. G4: independent review, signed artifacts, production operations approval,
   and explicit public go-live decision.

Passing a lower gate never implies a higher gate.

## Current priorities

1. keep CUDA/core FiroPoW parity and package validation reproducible;
2. validate SQLite recovery and finalize fork-aware synchronization plus
   transactional indexer/pool storage;
3. complete multi-host node and pool soak evidence;
4. harden public RPC authentication/rate/abuse controls;
5. complete external security and genesis review;
6. sign native packages and prove upgrade/data-retention behavior;
7. make an explicit G4 decision before changing public labels.

Smart contracts, Passport, marketplace, DAO, and staking do not move ahead of
base-layer production maturity merely because their folders or website pages
exist.
