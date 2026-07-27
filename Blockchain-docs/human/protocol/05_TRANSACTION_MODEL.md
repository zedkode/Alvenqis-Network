# Transaction Model

Status: Implemented candidate model / stable serialization freeze pending

## Current account-based transaction

- version and account nonce;
- optional sender (`None` only for coinbase);
- recipient;
- amount;
- maximum fee and priority fee;
- optional memo hash;
- optional ed25519 public key and signature.

Signing commits to domain `alvenqis-tx-ed25519-v1`, network ID, and every unsigned
field. The sender address is derived from the public key and must match the
declared network. Transaction IDs deterministically hash the signed form.

## Current lifecycle

Wallet composition/signing → RPC submission → mempool validation → P2P gossip →
template inclusion → block validation/persistence → indexer/explorer query.
Detached valid transactions may return to the mempool during a reorganization.

## Current fee and state rules

- zero transfers, invalid addresses/signatures/nonces, duplicate hashes,
  insufficient balances, and invalid fee bounds are rejected;
- base fee is burned and priority fee is paid to the miner;
- coinbase is unsigned and must exactly match allowed subsidy plus tips;
- exact atomic amounts use integers.

## Still open

TM-203 must freeze stable versioned transaction serialization and txid vectors.
Contract calls, native-asset operations, multisig, hardware wallets, replacement
policy, and broader replay/version negotiation require separate decisions.
