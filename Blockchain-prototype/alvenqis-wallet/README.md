# alvenqis-wallet

Status: Draft / Mainnet Candidate / Prototype

Primary candidate commands:
- `create-wallet --network mainnet-candidate --word-count 24`
- `import-mnemonic --network mainnet-candidate --phrase "<words>"`
- `import-private-key --network mainnet-candidate --private-key-hex <hex>`
- `network --network mainnet-candidate`
- `address --network mainnet-candidate`
- `balance --network mainnet-candidate <address>`
- `sign-tx --network mainnet-candidate --to <address> --amount <amount> --fee <fee>`
- `submit-tx --network mainnet-candidate --tx-file <file>`
- `wallet-status --network mainnet-candidate`

Current scope:
- local wallet files under the matching network root;
- local signed transaction files under the matching network root;
- deterministic address and signing primitives from `alvenqis-core`;
- BIP39 English mnemonic creation and SLIP-0010 ed25519 derivation on `m/44'/7330'/account'/change'/address_index'`;
- local RPC balance reads and local RPC transaction submission.

Safety rules:
- wallet files must never be committed;
- mnemonic phrases are shown at creation time and must be backed up immediately because they are not reprinted later by the CLI;
- signing fails when address network and wallet network do not match;
- Mainnet Candidate still means local Prototype use, not a live public wallet launch;
- back up private key material securely if a wallet is created or imported.
