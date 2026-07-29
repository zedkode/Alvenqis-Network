# alvenqis-sdk-rust — API map (v0.1)

Crate: `alvenqis-sdk-rust`
Import: `use alvenqis_sdk_rust::…`

## Config

| Item | Notes |
|---|---|
| `DEFAULT_MAINNET_CANDIDATE_RPC` | `https://rpcnode.dohotstudio.com` |
| `DEFAULT_MAINNET_CANDIDATE_MINING_RPC` | `http://127.0.0.1:10787` |
| `DEFAULT_MAINNET_CANDIDATE_POOL` | `https://pool.dohotstudio.com` |
| `DEFAULT_LOCAL_MAINNET_CANDIDATE_RPC` | `http://127.0.0.1:10787` |
| `NetworkConfig::mainnet_candidate()` | RPC + pool public defaults |
| `NetworkConfig::mainnet_candidate_local()` | local RPC, empty pool |
| `NetworkConfig::with_rpc(network, url)` | custom RPC, empty pool |
| `NetworkConfig::with_rpc_and_pool(…)` | custom both |
| `NetworkConfig::with_pool_url(self, url)` | builder-style pool set |
| `rpc_url(path)` / `pool_url(path)` | path join (pool errors if unset) |

## Wallet / tx

| Item | Notes |
|---|---|
| `WalletAccount` | generate / import mnemonic or key |
| `TransferBuilder` | build + sign transfer |
| `SignedTransfer` | signed payload helper |
| Protocol types (vendored in-crate) | `Address`, `Amount`, `Network`, `Transaction`, keys, … |

## Maturity (pure)

| Item | Notes |
|---|---|
| `DEFAULT_BLOCK_MATURITY_CONFIRMATIONS` | `12` |
| `pool_block_maturity(height, tip, required, status?)` | → `MaturityProgress` |
| `MaturityStatus` | Immature / Mature / Orphaned / Unknown |

Rule: mature when `tip >= height + required` (or pool status field says mature/orphan).

## RpcClient / BlockingRpcClient

Requires feature `native` (default). Blocking needs `blocking`.

### Chain / account

`health`, `network`, `status`, `tip`, `height`, `sync_status`, `supply`, `mempool_status`, `p2p_status`, `balance`, `account`, `transaction`, `block_latest`, `block_by_height`, `block_by_hash`, `recent_blocks`, `submit`, `await_status`

### Indexer

`indexer_status`, `indexer_summary`, `indexer_block_latest`, `indexer_block_by_height`, `indexer_transaction`, `indexer_address`

### Pool (public read)

`pool_status`, `pool_history`, `pool_miner`, `pool_payouts`, `pool_blocks_with_maturity`

## Response types (selected)

| Type | Source |
|---|---|
| `HealthResponse`, `StatusResponse`, … | gateway JSON |
| `P2pStatusResponse` | `GET /p2p/status` (optional fields) |
| `PoolStatusResponse`, `PoolHistoryResponse`, `PoolBlock`, `PoolWorker` | pool public JSON |
| `AtomicValue` | string or number atomic amounts |
| `PoolBlockWithMaturity` | block + `MaturityProgress` |
| `SdkError` | core / input / HTTP / decode / timeout |

## Features matrix

| Feature | Enables |
|---|---|
| `native` | `rpc` module, async client |
| `blocking` | `BlockingRpcClient` |
| `wasm` | `wasm_logic` pure helpers only |
