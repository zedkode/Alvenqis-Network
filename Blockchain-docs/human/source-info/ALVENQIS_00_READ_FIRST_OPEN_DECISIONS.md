# Alvenqis 00 — Read First: Decisions and Critical Gaps

Status: Canonical read-first register

This file separates accepted launch facts from unresolved design work. An agent
or contributor must not turn a recommendation into protocol behavior without an
explicit accepted decision.

## Resolved launch facts

- State model: account based.
- Consensus PoW: FiroPoW 0.9.4, AlvenqisPoW v1, period length 1.
- Product mining: NVIDIA CUDA-only; core host validation is not CPU mining.
- Difficulty adjustment: LWMA-style retargeting over the current difficulty
  representation.
- Signatures and addresses: ed25519, canonical lowercase Bech32m, network-
  separated HRPs (`dalve`, `talve`, `alve`).
- Transfer fee implementation: base fee burned plus miner priority tip.
- Upgrade activation: fixed-height flag day for launch scope.
- Early checkpoints: release-pinned checkpoints, currently including genesis.
- Desktop product: Tauri Control Center only.
- Active VPS package: `alvenqis-release/vps-control-plane/` only.

## Unresolved protocol and policy decisions

- scaling and whether any future sharding model is needed;
- smart-contract VM/runtime and deterministic contract gas metering;
- final genesis allocation, premine, treasury, founder, and vesting policy;
- governance and approval process before any community governance exists;
- post-genesis checkpoint schedule and later relaxation criteria;
- production hardening, migration, and recovery policy for indexer SQLite and
  the pool's current persistence;
- public RPC authentication and abuse-control design;
- mobile remote-control authentication, authorization, and audit model.

The complete private active list is `../../internal/memory/OPEN_QUESTIONS.md`. Do not silently
close any of those items in code, documentation, website copy, or release notes.

## Launch blockers

The repository is Mainnet Candidate, not public Mainnet. G4 remains blocked by
independent genesis verification, multi-host soak evidence, node SQLite
backup/restore and operations review, RPC abuse testing, external security review, signed
native artifacts, and an explicit go-live decision with named signatories.

See `../release/NETWORK_MATURITY.md` for the authoritative maturity ladder.
