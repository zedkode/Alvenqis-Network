# Consensus Serialization and Test Vectors

Status: Draft audit index / final serialization freeze incomplete

Consensus review must pin exact bytes, hashes, and rejection behavior rather
than infer them from JSON or UI models.

## In-scope domains

- block header and block serialization;
- transaction signing payload, signature, and txid;
- address payload and Bech32m canonical form;
- amount and fee arithmetic;
- coinbase and supply calculation;
- FiroPoW header input, mix hash, final hash, target comparison, and epoch;
- network ID, chain magic, genesis, checkpoint, and upgrade activation fields;
- P2P handshake and message version compatibility.

## Required vector set

| Vector class | Positive cases | Negative cases |
|---|---|---|
| Addresses | each network HRP, boundary payloads | mixed case, wrong checksum, wrong network/version |
| Transactions | canonical signed transfer, fee boundaries | mutation after signing, replay domain, overflow, nonce/fee errors |
| Blocks | genesis, coinbase, normal block, halving boundary | wrong parent/network/checkpoint, invalid merkle/state/PoW |
| FiroPoW | host/core/CUDA parity across epochs | altered nonce, mix hash, header bytes, target |
| Emission | genesis and every reward transition | terminal zero reward, cap overflow |
| P2P | current handshake/messages | old version, wrong chain magic/genesis, oversized payload |

## Audit rule

Every vector must record source commit, input bytes, expected output, and the
independent implementation or method used for cross-checking. Final vectors
must be shared across core, node, miner, SDK, and client tests.

Current shared vectors remain incomplete; this document does not declare the
serialization freeze complete.
