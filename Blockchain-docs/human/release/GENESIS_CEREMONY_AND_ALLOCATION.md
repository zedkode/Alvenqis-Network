# Genesis Ceremony and Allocation

Status: Draft / independent ceremony and final allocation unresolved

The current deterministic Mainnet Candidate genesis is rehearsal material. It
is pinned by repository review/approval artifacts and the height-zero
checkpoint, but it is not independently verified production genesis.

## Current facts

- deterministic network/config inputs are documented in `GENESIS.md`;
- the current recipient derives from the public development seed `[7; 32]`;
- the approved candidate hash is enforced at startup and checkpoint validation;
- final premine, treasury, founder, vesting, and custody policy is unresolved.

## Production ceremony requirements

1. freeze all consensus/config inputs on an immutable commit;
2. publish the allocation policy and machine-readable allocation;
3. remove development-key custody from any production allocation;
4. generate and verify genesis independently on multiple clean systems;
5. compare serialized block bytes, review hash, genesis hash, and checkpoint;
6. obtain named reviewer attestations and signatures;
7. publish artifact hashes and reproducible commands;
8. test startup, mismatch rejection, backup, restore, and fresh-node sync;
9. record an explicit go/no-go decision without changing the network label
   before all G4 requirements pass.

This document does not resolve the allocation policy and does not authorize
regenerating or replacing the current checkpoint.
