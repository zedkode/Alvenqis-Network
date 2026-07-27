# Core, Wallet, RPC, Indexer, and Explorer Impact Map

Status: Current dependency reference

| Source change | Required downstream review |
|---|---|
| Amounts, emission, fees | wallet formatting/composition, RPC models, index totals, explorer displays, website tokenomics |
| Address/signature/derivation | wallet/keystore, transaction submission, RPC validation, SDKs, browser/mobile clients |
| Block/transaction serialization | node storage/P2P, hashes, RPC/index schemas, explorer, test vectors |
| PoW/DAA/templates | node validation, miner parity, pool shares/jobs, RPC mining API, genesis evidence |
| Fork choice/reorg | storage, mempool reconciliation, indexer, pool maturity, explorer lifecycle |
| Network identity/genesis | every service config, clients, checkpoints, release and VPS artifacts |
| Exposure/auth policy | RPC router, reverse proxy, desktop/mobile clients, operator docs, threat model |

Core changes land first. Dependent layers may add stricter operational controls,
but they may not reinterpret consensus. Exact atomic values cross client bridges
as decimal strings, never floating-point values.
