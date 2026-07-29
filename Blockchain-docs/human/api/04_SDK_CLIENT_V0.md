# Alvenqis SDK Client (v0)

Status: Prototype / Mainnet Candidate

Crate: `alvenqis-sdk-rust`
Consumers: Rust apps, `alvenqis-wallet` (blocking RPC), `alvenqis-browser-host`, desktop Tauri + keystore-helper.

This is **not** Mainnet Live. Rust crate path: `alvenqis-sdk-rust/` (import `alvenqis_sdk_rust`).
Public TypeScript read client lives separately at `alvenqis-sdk/` (`@alvenqis/sdk`).

## Features

| Cargo feature | Default | Purpose |
|---|---|---|
| `native` | yes | Async HTTP RPC (`reqwest` + rustls) |
| `blocking` | no | Blocking RPC for CLIs / native messaging host |
| `wasm` | no | Pure logic path (`wasm_logic`); no FS keystore, no RPC in v0 |

## Network defaults

| Constant / helper | Value |
|---|---|
| Network id | `alvenqis-mainnet-candidate` |
| Address prefix | `alve` |
| Public RPC | `https://rpcnode.dohotstudio.com` |
| Public pool | `https://pool.dohotstudio.com` |
| Local RPC | `http://127.0.0.1:10787` |
| Status label | from `Network::MainnetCandidate` (Planned / Mainnet Candidate) |

```rust
use alvenqis_sdk_rust::NetworkConfig;

let remote = NetworkConfig::mainnet_candidate();
let local = NetworkConfig::mainnet_candidate_local();
let custom = NetworkConfig::with_rpc(alvenqis_sdk_rust::Network::MainnetCandidate, "https://example");
```

The project-operated defaults are convenience values, not decentralized
availability. Production clients still need operator-configurable endpoint
rotation, health-based failover, and trust policy.

## Essential types (re-exported from `alvenqis-core`)

- `Network`, `Address`, `Amount`, `Transaction`
- `PrivateKey`, `PublicKey`, `MnemonicWordCount`, `WalletDerivationPath`
- Unit constants: `ATOMIC_UNITS_PER_ALVE`, `DECIMALS`, `TICKER`, and related protocol constants

## Wallet (in-memory)

```rust
use alvenqis_sdk_rust::{WalletAccount, MnemonicWordCount, Network};

let (account, mnemonic) =
    WalletAccount::generate(Network::MainnetCandidate, MnemonicWordCount::Twelve)?;
// mnemonic.phrase is shown once to the caller — SDK does not write it to disk.
let address = account.address_string();
```

## Transfer builder

```rust
use alvenqis_sdk_rust::{TransferBuilder, Amount};

let signed = TransferBuilder::new(network)
    .to(recipient)?
    .amount_alve("0.01")?
    .nonce(next_nonce)
    .fees(base_fee, priority_fee)?
    .sign(&account)?;
```

## RPC client (`native` / `blocking`)

| Method | HTTP |
|---|---|
| `health` | `GET /health` |
| `network` | `GET /network` |
| `status` | `GET /status` |
| `sync_status` | `GET /sync/status` |
| `supply` | `GET /supply` |
| `mempool_status` | `GET /mempool/status` (summary only) |
| `p2p_status` | `GET /p2p/status` |
| `tip` / `height` | `GET /chain/tip`, `/chain/height` |
| `account` | `GET /addresses/{addr}/account` |
| `balance` | `GET /addresses/{addr}/balance` |
| `transaction` | `GET /transactions/{hash}` |
| `block_latest` | `GET /blocks/latest` |
| `block_by_height` | `GET /blocks/{height}` |
| `block_by_hash` | `GET /blocks/hash/{hash}` |
| `recent_blocks(n)` | tip + N height fetches (newest first, max 32) |
| `indexer_status` | `GET /indexer/status` (optional lag fields) |
| `indexer_summary` | `GET /indexer/summary` (compatibility call; client ignores large maps; do not poll) |
| `indexer_block_latest` / `indexer_block_by_height` | `GET /indexer/blocks/*` |
| `indexer_transaction` | `GET /indexer/tx/{hash}` |
| `indexer_address` | `GET /indexer/address/{addr}` |
| `submit` | `POST /transactions` |
| `await_status` | poll transaction lifecycle |
| `pool_status` | `GET {pool}/api/v1/pool/status` |
| `pool_history` | `GET {pool}/api/v1/pool/history` |
| `pool_miner` | `GET {pool}/api/v1/miners/{addr}` |
| `pool_payouts` | `GET {pool}/api/v1/payouts` |
| `pool_blocks_with_maturity` | pool + tip → maturity list |
| `pool_block_maturity` (pure) | same rule as TS / mining-pool |

```rust
use alvenqis_sdk_rust::{NetworkConfig, RpcClient};

let client = RpcClient::new(NetworkConfig::mainnet_candidate())?;
let tip = client.tip().await?;
let recent = client.recent_blocks(5).await?;
```

Blocking (feature `blocking`):

```rust
use alvenqis_sdk_rust::BlockingRpcClient;
```

## WASM (`wasm` feature)

`alvenqis_sdk_rust::wasm_logic`: parse address, format/parse ALVE, verify transaction.
No RPC, no disk keystore. Browser signing uses **native messaging host**.

## Non-goals (v0)

- Mining template/submit API (stratum worker protocol)
- Pool **admin** / private payout mutation APIs
- Full mempool transaction list API (use gateway `/mempool` ad-hoc if needed)
- Smart contracts / NFT / Passport
- Encrypted keystore inside the SDK crate (host/wallet own that)
- crates.io “production mainnet” claims

## Consumers (current monorepo)

| Consumer | How it uses the SDK |
|---|---|
| `alvenqis-wallet` | `BlockingRpcClient` for balance + submit |
| `alvenqis-browser-host` | blocking RPC + wallet + keystore + explore methods |
| `alvenqis-desktop-v2` | async RPC + defaults; snapshot may still mix ad-hoc GETs |
| Tauri `keystore-helper` | `BlockingRpcClient::account` for prepare/sign previews |
| `alvenqis-examples` (JS) | uses TypeScript `@alvenqis/sdk` under `alvenqis-sdk/`, not this crate |

## Related

- Implementation: `alvenqis-sdk-rust/` (`README.md`, `docs/API.md`, `docs/JS_PARITY.md`)
- TypeScript client: `alvenqis-sdk/` (`@alvenqis/sdk`)
- Browser host: `alvenqis-browser/`
- RPC gateway endpoints: `01_RPC_ENDPOINTS_DRAFT.md`
