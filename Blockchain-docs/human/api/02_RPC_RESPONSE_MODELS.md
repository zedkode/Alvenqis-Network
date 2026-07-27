# RPC Response Models

Status: Implemented / Mainnet Candidate

The Rust structs in `alvenqis-rpc-gateway/src/models.rs` are the field-level source
of truth. This document defines model groups and invariants without copying an
exhaustive field list that can drift.

| Group | Current models | Important invariants |
|---|---|---|
| Identity | `HealthResponse`, `NetworkResponse` | explicit network/status; protocol, address, signature, PoW, DAA, fee, size, and timing metadata |
| Chain | `StatusResponse`, `SyncStatusResponse`, `ChainTipResponse`, `ChainHeightResponse` | optional tip before initialization; cumulative work is a decimal string; sync target may be unknown |
| Accounts | `AddressResponse`, `AddressBalanceResponse`, `AddressAccountResponse`, `StateResponse` | atomic values are integers; next nonce is ledger plus pending-mempool aware on `/account` |
| Supply | `SupplyResponse` | emitted subsidy, cap, and remaining cap; fees are not issuance |
| Transactions | `TransactionResponse`, `SubmitTransactionResponse` | pending/mined lifecycle, max/effective/burned/priority fee separation, signature metadata |
| Mempool | `MempoolResponse`, `MempoolStatusResponse` | anticipated base fee and bounded aggregate fee/priority information |
| Blocks | `BlockResponse` | canonical FiroPoW final hash, network, base fee, nonce, difficulty, and expanded transactions |
| Mining | `MiningTemplateResponse`, `MiningSubmitRequest`, `MiningSubmitResponse` | protocol ID, expiring template ID, immutable header fields, nonce, final hash, and mix hash |
| P2P | node P2P status types | local observation only; never a globally authoritative census |

Exact ALVE atomic values cross JavaScript/client boundaries as decimal integer
strings or safe bounded integers according to the consuming schema; clients must
not convert supply/balance arithmetic through floating point.

When a Rust model changes, update SDK types, desktop shared types, explorer/API
consumers, OpenAPI output, tests, and this invariant map in the same change.
