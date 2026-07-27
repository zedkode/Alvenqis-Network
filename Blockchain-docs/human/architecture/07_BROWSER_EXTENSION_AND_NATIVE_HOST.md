# Browser Extension and Native Host

Status: Prototype / Mainnet Candidate / not store-ready

## Architecture

```text
alvenqis-browser/
├── extension/  Manifest V3 JavaScript UI and native-message bridge
└── host/       Rust `alvenqis-browser-host` using `alvenqis-sdk-rust`
```

| Surface | Holds keys? | Responsibility |
|---|---:|---|
| Popup/service worker | No | display, request confirmation, native-message transport |
| Content scripts | No | no active dApp-connect protocol |
| Rust native host | Yes, only while unlocked | encrypted keystore, sign, submit, RPC/index reads |
| Rust SDK/core | Library boundary | addresses, amounts, transactions, signing, clients |

## Native messaging protocol

Browser mode uses little-endian `u32` message length followed by UTF-8 JSON.
Development mode uses JSON Lines. Requests and responses carry a caller ID,
method, success flag, result, or error.

Current method groups:

- health/network: ping, network info, RPC/sync/mempool/indexer status, supply;
- chain reads: tip, recent/latest/height/hash blocks, transaction and address;
- keystore: status, create, unlock, lock, export public, change passphrase,
  delete;
- wallet: session status, balance/account, prepare-and-sign, submit, send.

The Rust protocol enum and handler in `alvenqis-browser/host/src/protocol.rs` are
the method-level source of truth.

## Recovery and security

- Host CLI `--init-wallet` may show a new mnemonic once to the operator terminal.
- Host CLI `--import-mnemonic` accepts recovery words without involving JS.
- Extension `create_wallet` never receives the mnemonic.
- Extension sessions are not a backup strategy.
- Secrets do not enter RPC, website, logs, or repository files.
- Optional OS confirmation is an extra prompt, not a replacement for keystore
  encryption or transaction validation.

## Non-goals

- browser-store distribution;
- local mining or pool work;
- WASM-held production keys;
- unrestricted website/dApp signing;
- remote operator control.

Registration and CLI commands are maintained in `../../alvenqis-browser/README.md`.
