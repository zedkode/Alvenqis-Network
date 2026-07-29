# Alvenqis 05 — Accepted Decisions and Recommendations

Status: Canonical decision summary

`../../internal/memory/DECISIONS.md` is the full private accepted-decision register. This file is
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
- non-mining VPS node roles plus optional pool coordination, with no VPS miner
  binary or wallet custody;
- embedded SQLite for the canonical block oracle and RocksDB state/mempool in
  the VPS profile, with versioning, integrity checks, and preserved migration
  input;
- transactional SQLite for the current indexer implementation;
- off-chain PPLNS pool accounting that never replaces network validation;
- Apache-2.0 direction for protocol/chain-critical components and proprietary
  business/admin/operations boundaries unless explicitly relicensed.

## Recommended but unresolved

- single-chain scaling before any sharding work;
- a specific WASM runtime and deterministic contract gas model;
- final premine, treasury, founder, and vesting policy;
- long-term governance and community upgrade approval;
- production hardening/migration for indexer SQLite and production pool storage;
- authenticated remote control and public RPC security architecture;
- post-genesis checkpoint schedule and relaxation.

These recommendations must remain Draft or Research until explicitly accepted
and moved into `../../internal/memory/DECISIONS.md`.

## Superseded approaches

- Blake3 leading-zero PoW is historical and not active consensus.
- CPU, OpenCL, hybrid, and host-emulated product mining are removed.
- Electron and egui are not Control Center release paths.
- `alvenqis-release/vps/` is legacy/frozen and receives no product/security work.
- Retired Veiron or Vireon names must not be introduced into new packages or docs.
