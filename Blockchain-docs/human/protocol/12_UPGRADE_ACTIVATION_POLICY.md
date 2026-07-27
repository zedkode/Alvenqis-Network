# Upgrade Activation Policy

Status: Implemented / Mainnet Candidate

## Accepted Launch Direction

`TM-109` freezes Alvenqis's launch upgrade activation policy as:
- fixed-height flag-day activation only;
- launch protocol version `1`;
- launch block version `1`;
- no scheduled protocol upgrades are active yet in the current repository state.

## Rule

For launch-scope Alvenqis validation:
- each block height maps to exactly one expected block version;
- block versions do not negotiate dynamically;
- a node must reject a block whose version does not match the expected version for that height.

Current implementation note:
- `alvenqis-core` now computes the expected block version from the active network plus block height;
- the current policy returns version `1` at genesis and beyond because no later upgrade heights are pinned yet;
- the rule is already wired into chain validation, so wrong-version blocks are rejected.

## Launch Policy Shape

Policy ID:
- `alvenqis-flag-day-upgrade-v1`

Activation mode:
- `fixed-height-flag-day`

Launch baseline:
- protocol version `1`
- block version `1`

Current scheduled upgrades:
- none

## Later Migration Path

The accepted launch rule does not assume:
- miner voting;
- rolling-version negotiation;
- soft-fork style capability signaling;
- DAO-based activation;
- runtime feature negotiation.

Those remain later follow-up work.

If a later upgrade path is introduced, it must be explicitly frozen in a later task and must not silently replace the launch rule.

## Impact Notes

- Core: block construction and validation must agree on the expected version for each height.
- Node: local or candidate nodes should never emit a block version that consensus would reject at the same height.
- Wallet and RPC: downstream tooling may still treat transaction-version policy as Draft until a separate freeze closes that scope.
- Explorer and Indexer: block pages may surface version fields, but should not imply any active post-launch upgrade schedule yet.
