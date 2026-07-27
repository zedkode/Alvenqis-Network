# Security Docs

Threat models, review notes, audit preparation and secure development guidance.

Status: Current Mainnet Candidate security index

Current network-separation safeguards:
- Devnet, Testnet and Mainnet Candidate use separate network IDs and chain magic values.
- Devnet, Testnet and Mainnet Candidate use separate default data roots: `.alvenqis-dev/`, `.alvenqis-testnet/` and `.alvenqis-mainnet/`.
- Mainnet Candidate startup requires `allow_mainnet_candidate = true`.
- Reset commands must refuse Mainnet Candidate.
- Wallet signing must surface the active network before transaction signing.
- RPC responses must expose `network_id` and status labels so downstream tools do not infer the wrong environment.
- Addresses are network-prefixed and cross-network sender or recipient mixes are rejected.
- Block validation rejects a block whose `network_id` does not match the active chain.
- P2P handshake validation rejects mismatched `network_id` or chain magic values.

Remaining boundaries:
- no claim of live public testnet or live mainnet;
- no production key custody or HSM integration;
- P2P transport exists with Noise/Yamux and genesis/network validation, but
  production peer reputation, multi-host soak, and abuse evidence are incomplete;
- public RPC/mining exposure is a rate-limited prototype, not an authenticated
  production control plane.
