# Ledger state and transaction-hash retention

Status: Mainnet Candidate / Prototype

## Primary replay protection

Alvenqis uses an **account model** with sequential per-address nonces
(`FIRST_ACCOUNT_NONCE` = 1). After a spend from address `A` with nonce `n` is
applied, the ledger requires nonce `n+1` for the next spend from `A`. That alone
prevents replaying the same signed body once it has been confirmed.

## Defense-in-depth hash set

`LedgerState` also tracks applied transaction hashes
(`applied_transaction_hashes`) so that:

- coinbase bodies remain unique within the retention window;
- identical payloads cannot be double-applied if an intermediate path skips the
  nonce check;
- operators get a clear `DuplicateTransactionHash` error in the recent window.

## Bound: `TX_HASH_RETENTION_BLOCKS` (1024)

The hash set is **not** unbounded. Hashes are stored per height and pruned so
only the most recent **1024** confirmed blocks contribute to the in-memory set
(constant `TX_HASH_RETENTION_BLOCKS` in `alvenqis-core/src/state.rs`).

Older hashes may leave the set; spend replay remains blocked by nonces.

## Why not keep every hash forever?

Long-running nodes would grow RAM without bound. With sequential nonces, full
history is unnecessary for correctness of spend replay.
