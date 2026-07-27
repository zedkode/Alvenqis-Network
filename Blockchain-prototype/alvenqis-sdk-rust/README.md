# alvenqis-sdk-rust

Status: Prototype / Mainnet Candidate / not public Mainnet

The native Rust SDK provides standalone Alvenqis account/signing logic plus async
and optional blocking RPC/pool clients. It is distinct from the read-oriented
TypeScript package in `alvenqis-sdk/` (`@alvenqis/sdk`).

## Included

- network configuration and honest status labels;
- address, amount, transaction, ed25519 key, mnemonic, and derivation types;
- in-memory wallet accounts and signed transfer builder;
- async Rustls RPC client (`native` default feature);
- optional blocking RPC helpers (`blocking` feature);
- transaction submission and lifecycle polling;
- public pool reads and block-maturity helper;
- pure logic WASM feature without filesystem keystore or network client.

## Excluded

- smart contracts/VM, Passport, marketplace, staking, and pool admin signing;
- browser page-held private keys;
- plaintext keystore as a recommended storage path;
- crates.io or production-Mainnet readiness claims.

## Defaults

| Field | Value |
|---|---|
| Network | `alvenqis-mainnet-candidate` |
| Address prefix | `ALVE` |
| Public RPC | `https://rpcnode.dohotstudio.com` |
| Public pool | `http://rpcnode.dohotstudio.com/pool` |
| Local RPC | `http://127.0.0.1:10787` |

## Example

```rust
use alvenqis_sdk_rust::{Amount, NetworkConfig, TransferBuilder, WalletAccount};

let config = NetworkConfig::mainnet_candidate();
let account = WalletAccount::generate(config.network, Default::default())?;
let tx = TransferBuilder::new(config.network)
    .to(recipient)?
    .amount(Amount::from_atomic(1_000_000))?
    .nonce(1)
    .max_fee(Amount::from_atomic(2))?
    .priority_fee(Amount::from_atomic(1))?
    .sign(&account)?;
```

See `docs/API.md` for the method/type map and `docs/JS_PARITY.md` for parity
with `@alvenqis/sdk`.

## Validation

```powershell
cargo test -p alvenqis-sdk-rust --all-features
```

Optional live candidate smoke tests remain ignored by default.
