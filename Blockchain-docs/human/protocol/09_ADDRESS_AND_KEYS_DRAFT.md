# Address And Keys Draft

Status: Draft

Implementation gate:
- the launch-scope address and signing rule is now frozen through `TM-107`;
- `TM-111` now freezes the launch-scope wallet seed and derivation rule.

## Fixed Direction

The address prefix direction is:
- `ALVE`

## Accepted Launch Standard

`TM-107` now fixes the launch-scope standard as:
- signature scheme: ed25519;
- address encoding: Bech32m;
- canonical case: lowercase only;
- address payload layout: `version-byte || 32-byte ed25519 public key`;
- payload version: `0`;
- visible forms: `dalve1...`, `talve1...`, `alve1...` by network;
- transaction signing domain: `alvenqis-tx-ed25519-v1`;
- direct key rule: raw 32-byte ed25519 signing seed -> public key -> address.

These decisions freeze the launch-scope address and key direction without claiming that wallet packaging or public-network rollout is live.

## Current Launch-Scope Behavior

The current implementation now enforces:
- addresses are encoded as Bech32m with network-separated prefixes;
- the visible forms are `dalve1...` for Devnet, `talve1...` for Testnet and `alve1...` for Mainnet Candidate;
- the address payload stores a version byte plus the 32-byte ed25519 public key;
- Bech32m checksum verification is mandatory;
- only lowercase canonical strings are accepted;
- transaction signing and verification use ed25519 keypairs and the fixed signing domain above.

`TM-111` now adds the frozen wallet seed rule:
- mnemonic standard: BIP39 English;
- accepted mnemonic sizes: 12 or 24 words;
- derivation scheme: BIP39 seed -> SLIP-0010 ed25519;
- path template: `m/44'/7330'/account'/change'/address_index'`;
- path policy: all segments are hardened for ed25519 compatibility;
- current `coin_type`: `7330` as the repository's provisional launch constant until a final SLIP-44 assignment exists.

## Network Separation Rule

The current draft implementation separates environments at the address layer:
- Devnet addresses use `dalve`;
- Testnet addresses use `talve`;
- Mainnet Candidate addresses use `alve`;
- sender and recipient addresses inside the same transaction must belong to the same network;
- wallet, node and RPC flows must reject cross-network address use.

## Open Decisions

- support for multisig or alternative key types;
- replay and transaction-version policy beyond the frozen signing domain.

## Current Non-Goals

- no production key storage design yet;
- no encrypted keystore flow yet;
- no hardware-wallet integration yet.

## Phase 4C Draft Wallet CLI Note

The current local prototype now includes a Rust wallet CLI for private-devnet use only:
- wallet files are stored under the user home `.alvenqis-dev/wallets/` by default;
- signed transactions are stored under the user home `.alvenqis-dev/signed-txs/` by default;
- mnemonic-derived wallets now use the frozen BIP39 plus SLIP-0010 rule above;
- future testnet and mainnet-candidate wallet material must stay under `.alvenqis-testnet/` and `.alvenqis-mainnet/` respectively;
- no encrypted keystore or browser-extension flow exists;
- private key storage remains Draft / Devnet-only / Prototype and must not be treated as production-safe.

## Documentation Rule

Until broader launch packaging work is complete:
- refer to addresses as Alvenqis addresses using the frozen Bech32m plus ed25519 launch standard;
- keep readiness language honest and avoid calling the wallet or network live;
- treat the mnemonic and derivation rule as frozen, but keep wallet packaging and secret-storage UX as separate follow-up work.

## Impact Notes

- Core: signature verification and transaction hashing depend on these choices.
- Wallet: account creation can now follow the frozen mnemonic rule or raw private-key import, while keystore UX remains separate work.
- Explorer and Indexer: address indexing and display formats can now treat the visible address form as frozen for the launch-scope prototype.
- RPC: transaction and network responses can now expose the frozen address and signing standard identifiers explicitly.
