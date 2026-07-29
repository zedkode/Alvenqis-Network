# Alvenqis 05 — Accepted Decisions and Recommendations

Status: Canonical decision summary

`../../memory/DECISIONS.md` is the full accepted-decision register. This file is
a readable summary and deliberately separates accepted decisions from proposals.

## Accepted

- account-based launch ledger;
- FiroPoW 0.9.4, AlvenqisPoW v1, period length 1;
- NVIDIA CUDA-only product mining with canonical host validation;
- LWMA-style difficulty adjustment;
- ed25519, Bech32m, and network-separated address HRPs;
- BIP39 English mnemonics and hardened SLIP-0010 derivation policy;
- base-fee burn plus miner priority tip for the current transfer model;
- fixed-height launch upgrade activation;
- release-pinned early checkpoints;
- Rust core and service implementation;
- Tauri-only Windows/Linux Control Center product path;
- user-approved, checksum-verified desktop updates;
- non-mining VPS validators and the control-plane-only packaging path;
- embedded SQLite for canonical node storage across Windows, Linux, and Docker,
  using a bundled post-WAL-reset-fix release, WAL, `synchronous=FULL`, strict
  schema versioning, integrity checks, and preserved JSONL migration input;
- off-chain PPLNS pool accounting that never replaces network validation;
- Apache-2.0 direction for protocol/chain-critical components and proprietary
  business/admin/operations boundaries unless explicitly relicensed.

## Recommended but unresolved

- single-chain scaling before any sharding work;
- a specific WASM runtime and deterministic contract gas model;
- final premine, treasury, founder, and vesting policy;
- long-term governance and community upgrade approval;
- production indexer and mining-pool database/storage technologies;
- authenticated remote control and public RPC security architecture;
- post-genesis checkpoint schedule and relaxation.

These recommendations must remain Draft or Research until explicitly accepted
and moved into `memory/DECISIONS.md`.

## Superseded approaches

- Blake3 leading-zero PoW is historical and not active consensus.
- CPU, OpenCL, hybrid, and host-emulated product mining are removed.
- Electron and egui are not Control Center release paths.
- `alvenqis-release/vps/` is legacy/frozen and receives no product/security work.
- Legacy `alvenqis-*` names must not be introduced into new packages or docs.
