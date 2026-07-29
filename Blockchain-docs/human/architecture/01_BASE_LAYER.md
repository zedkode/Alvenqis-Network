# Base Layer

Status: Implemented / Mainnet Candidate / production hardening incomplete

## Implemented scope

- account-based blocks, transactions, balances, nonces, and supply accounting;
- FiroPoW 0.9.4 and LWMA-style difficulty validation;
- base-fee burn and miner priority-tip accounting;
- mempool admission, templates, submitted-block validation, and persistence;
- libp2p transport, handshake validation, gossip, bounded sync, cumulative-work
  fork choice, and reorganization primitives;
- RPC, a transactional SQLite explorer index, CUDA-only mining, and off-chain
  pool accounting.

## Still open or incomplete

- stable final block and transaction serialization freezes;
- production proof for the SQLite/RocksDB consistency, migration, corruption
  recovery, and branch-storage contract;
- bounded verification under adversarial load, diverse discovery,
  IP/subnet/ASN admission, and deep-resume/reorg recovery;
- final genesis allocation/treasury policy and independent genesis review;
- multi-host soak, RPC abuse testing, and signed release artifacts;
- contract execution and gas metering.

## Dependency boundary

Wallets and clients depend on canonical address, transaction, fee, and network
rules. RPC/indexer/explorer depend on core data and must label local observations
as local. Miners and pools cannot bypass node/core validation.
