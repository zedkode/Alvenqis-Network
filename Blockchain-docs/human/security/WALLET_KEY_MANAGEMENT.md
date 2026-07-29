# Wallet Key Management and Recovery

Status: Implemented candidate storage / recovery and external review incomplete

## Current boundaries

- ed25519 signing seeds derive addresses through the frozen Bech32m scheme;
- BIP39 English mnemonics and hardened SLIP-0010 derivation are supported;
- Mainnet Candidate wallet storage requires encrypted keystore handling;
- current encrypted storage uses Argon2id and AES-256-GCM;
- desktop native helpers use the operating-system credential store;
- renderer, website, explorer, RPC, pool, and VPS services must never receive
  mnemonic or private-key material;
- signing intent must show network, recipient, amount, and fees before approval.

## Open audit scenarios

- weak, empty, malformed, and Unicode passphrases;
- nonce/IV uniqueness and authenticated-metadata coverage;
- interrupted create/import/rotate/restore operations;
- corrupted, truncated, downgraded, or swapped keystore files;
- OS credential-store denial, lock, migration, and account change;
- backup/restore on a second clean device;
- clipboard, logs, crash dumps, argv, stdout, and renderer-message leakage;
- wrong-network and stale-transaction signing;
- dependency compromise and malicious update;
- hardware-wallet and offline-signing boundaries.

## Recovery rule

A recovery test is complete only when the restored wallet derives the expected
address, can prepare and sign a reviewed candidate transaction, and exposes no
secret through logs or public APIs. Test balances are not real funds and do not
authorize public Mainnet claims.
