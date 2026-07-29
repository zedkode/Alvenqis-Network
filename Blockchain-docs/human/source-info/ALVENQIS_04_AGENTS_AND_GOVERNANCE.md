# Alvenqis 04 — Change and Documentation Governance

Status: Accepted workspace governance

## Change order

1. Read the source-info set and memory decision registers.
2. Inspect implementation and neighboring packages.
3. Identify validation commands and downstream impact.
4. Change the source-of-truth layer first.
5. Update dependent schemas, clients, operations, and documentation.
6. Validate in proportion to risk and preserve evidence.
7. Update `memory/` after meaningful work.

Consensus and chain-parameter changes require an explicit reviewed decision.
Documentation cleanup may align text with existing verified behavior, but it
must not invent a new consensus rule.

## Public-claim governance

Allowed maturity labels are Draft, Planned, Research, Private Devnet, Public
Testnet, Mainnet Candidate, Coming Soon, Prototype, and Experimental. Public
Mainnet is allowed only after G4.

The existence of a folder, UI card, API sketch, VPS process, mined block, or
green CI run is not proof that a product is production-ready or publicly live.

## Protocol governance status

No DAO or on-chain governance system exists. The long-term community governance
model remains unresolved. Until it is approved, repository changes follow
maintainer review, explicit decision records, consensus tests, release gates,
and conservative public claims. This operational rule is not a claim of a
permanent centralized governance protocol.

## Documentation governance

`../DOCUMENTATION_POLICY.md` and `../documentation-manifest.json` define document
precedence, publication, historical retention, and the Markdown web reader.
Run `node Blockchain-scripts/docs/audit-docs.mjs --write` from the repository
root whenever behavior or status changes materially.

## Security and secrets

Never commit seeds, private keys, passwords, API tokens, wallet files, private
user data, or live credentials. Stop publication when checks fail; do not bypass
security, consensus, or release gates to obtain a green status.
