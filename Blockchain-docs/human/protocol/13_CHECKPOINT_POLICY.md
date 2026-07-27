# Checkpoint Policy

Status: Implemented / Mainnet Candidate

## Accepted Launch Direction

`TM-110` freezes Alvenqis's early-network checkpoint direction as:
- social/hardcoded checkpoints in early environments;
- checkpoints carried by node releases;
- progressive relaxation only through an explicit later decision;
- current canonical Mainnet Candidate checkpoint at height `0`.

## Rule

Checkpoint policy ID:
- `alvenqis-hardcoded-checkpoints-v1`

Checkpoint mode:
- `social-hardcoded-early-network`

Current active canonical checkpoint:
- network: `alvenqis-mainnet-candidate`
- height: `0`
- hash: `0000c29213014578ac41a748c2be3489859f1e0b1f3555bd89b7e5301632a4c5`

## Current Implementation Note

- `alvenqis-core` now exposes checkpoint schedules per network;
- `alvenqis-core` now validates checkpointed heights during chain acceptance and full chain rebuild;
- the current repository pins the deterministic Mainnet Candidate genesis hash as the first canonical checkpoint;
- Devnet and Testnet do not yet pin additional checkpoint heights in the current repository state.

## Relaxation Path

This launch policy assumes:
- new PoW networks are vulnerable during early hashrate growth;
- checkpointing is an early-network safety measure, not a permanent decentralization target.

Any checkpoint removal or relaxation later must:
- be explicit;
- be documented;
- not silently happen through implementation drift.

## Impact Notes

- Core: chain validation must reject blocks at checkpoint heights when the hash mismatches.
- Node: startup and validation built on top of `Chain::from_blocks(...)` inherit checkpoint enforcement automatically.
- Wallet, Explorer and RPC: no user-facing live claim should imply checkpoint independence until policy is explicitly relaxed.
- Docs: public communication should describe checkpointing honestly as an early-network safety rule, not as a forever rule.
