# Block Structure

Status: Implemented candidate structure / stable serialization freeze pending

## Current block header

- `version: u32`;
- `network_id: String`;
- `height: u64`;
- `previous_hash: 32 bytes`;
- `merkle_root: 32 bytes`;
- `base_fee_atomic: u64`;
- `timestamp: u64`;
- `nonce: u64`;
- `mix_hash: 32 bytes`;
- `difficulty_leading_zero_bits: u8`.

A block contains the header and an ordered transaction list with coinbase first.
Canonical chain identity is the recomputed FiroPoW final hash. The mix hash is
part of the submitted/stored solution and is verified by core.

## Current validation

- network, height, previous hash, version, timestamp, MTP/future drift;
- merkle root, transaction count/size, duplicate hashes, and state transitions;
- exact next difficulty and FiroPoW target;
- exact coinbase subsidy plus priority tips;
- base-fee and supply accounting;
- checkpoints and cumulative-work fork choice.

## Still open

TM-202 must freeze and publish stable versioned header serialization and test
vectors. Future state/receipt commitments for a contract layer are not part of
the current block header and must not be documented as implemented.
