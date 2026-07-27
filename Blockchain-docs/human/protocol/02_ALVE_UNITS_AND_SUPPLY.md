# ALVE Units And Supply

Status: Implemented / Mainnet Candidate

## Supply Facts

Fixed supply facts:
- max supply target: 60,000,000 ALVE;
- emission model: halving every 1,576,800 blocks;
- initial block reward: 19.02587519 ALVE.

## Fixed Unit Model For The Current Core Prototype

For the canonical Alvenqis amount model:
- decimals: 8;
- atomic units per ALVE: 100,000,000.

Reason:
- the initial reward uses 8 decimal places;
- 8 decimals preserve exact representation for the documented reward schedule;
- the Phase 1 local core request explicitly fixes these values.

## Human And Atomic Representation

Examples:
- `1 ALVE` = `100,000,000` atomic units;
- `19.02587519 ALVE` = `1,902,587,519` atomic units;
- `60,000,000 ALVE` = `6,000,000,000,000,000` atomic units.

## Documentation Rule

The unit precision is fixed for the current protocol.

Even so:
- stable production serialization and database schemas still have separate maturity gates;
- unit precision alone does not settle fee policy, address format or final transaction encoding.

## Impact Notes

- Core: amount types and overflow handling depend on the final atomic precision.
- Wallet: send forms, display formatting and fee display depend on unit confirmation.
- Explorer and Indexer: balance rendering and supply charts depend on atomic precision.
- RPC: numeric encoding rules still depend on the rest of the protocol format decisions.
- Docs and Website/Admin: treat decimals and atomic units as fixed candidate protocol facts.
