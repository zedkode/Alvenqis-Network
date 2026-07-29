# Alvenqis 01 — Source Information Master

Status: Canonical identity and launch-facts summary

## Identity

| Field | Value |
|---|---|
| Project | Alvenqis Network |
| Native asset | ALVE |
| Public address HRP | `alve` |
| Project type | Proprietary Rust-based Layer 1 |
| Current maturity | Mainnet Candidate / Prototype |
| Public Mainnet | No |

Alvenqis is built as its own system. It may learn from other networks, but product
copy and technical documentation must not claim unimplemented compatibility or
describe Alvenqis as a rebrand of another chain.

## Fixed launch economics

| Parameter | Value |
|---|---:|
| Maximum supply | 60,000,000 ALVE |
| Atomic units per ALVE | 100,000,000 |
| Target block time | 60 seconds |
| Halving interval | 1,576,800 blocks |
| Initial block reward | 19.02587519 ALVE |
| Initial block reward, atomic | 1,902,587,519 |

The implemented transfer-fee direction burns the base fee and pays the priority
tip to the miner. Contract gas metering and broader allocation policy remain
separate decisions.

## Fixed launch protocol direction

- account-based ledger;
- FiroPoW 0.9.4 (`AlvenqisPoW v1`, period length 1);
- LWMA-style difficulty retargeting;
- ed25519 signatures and Bech32m addresses;
- fixed-height launch upgrade activation;
- release-pinned early checkpoints;
- PoW first, with energy-aware mining only as future research.

## On-chain and off-chain boundary

On-chain data is limited to settlement, ownership, balances, state transitions,
hashes, proofs, permissions, public keys, and future explicitly implemented
protocol records. Large files, media, encrypted messages, private profiles, and
application payloads remain off-chain.

## Product truth boundary

Implemented candidate components include core, node, wallet tooling, RPC,
indexer, explorer, CUDA miner, pool prototype, Tauri Control Center, SDKs, and
the VPS control plane. Smart contracts, staking, DAO, marketplace, Passport,
native NFT/product standards, and public production Mainnet are not live.

Exact maturity and limitations belong in `../release/NETWORK_MATURITY.md` rather
than duplicated marketing claims.
