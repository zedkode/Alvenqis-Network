# Security Docs

Threat models, review notes, audit preparation and secure development guidance.

Status: Current Mainnet Candidate security index

All security work is limited by
[Defensive Security Engineering Scope](DEFENSIVE_SECURITY_SCOPE.md): project-owned
code and explicitly authorized environments, prevention and remediation goals,
bounded testing, and no testing of unrelated systems.

Start with [Verification and Audit Status](AUDIT_STATUS.md) for the current
evidence boundary and the explicit statement that no external audit is claimed.

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
- P2P transport, persistent peer reputation, Noise/Yamux, and genesis/network
  validation exist, but discovery diversity, source-network admission,
  multi-host soak, and abuse evidence are incomplete;
- public RPC and local/private mining capabilities are separated in the current
  code; deployed endpoint parity and independent availability evidence remain
  required.

Audit and risk entry points:

- `DEFENSIVE_SECURITY_SCOPE.md` — authorization boundary and defensive testing rules;
- `AUDIT_STATUS.md` - current verification evidence and claim boundary;

- `KNOWN_LIMITATIONS.md` — open findings and required closure evidence;
- `AUDIT_LOG_HASH_CHAIN_STATUS_2026-08-08.md` — local fleet audit-chain
  implementation, verification, and remaining anchoring boundary;
- `THREAT_MODEL.md` — assets, trust boundaries, actors, and priority scenarios;
- `EXTERNAL_SECURITY_REVIEW_SCOPE.md` — immutable external-review package;
- `WALLET_KEY_MANAGEMENT.md` — custody, keystore, and recovery boundary;
- `RPC_STRATUM_POOL_ABUSE_MODEL.md` — public API and mining abuse model;
- `SUPPLY_CHAIN_AND_ARTIFACTS.md` — SBOM, signing, and provenance requirements;
- `DEPENDENCY_AUDIT_2026-07-30.md` — dated Cargo audit, deny-policy results,
  critical dependency review, remediation, and retained findings;
- `UNSAFE_CODE_INVENTORY_2026-07-30.md` — dated inventory of Rust unsafe
  boundaries and missing or partial safety justifications;
- `RESPONSIBLE_DISCLOSURE.md` — private reporting expectations;
- `../release/DECENTRALIZATION_READINESS.md` — row-by-row centralization register.
