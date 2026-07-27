# Consensus proof of work

Status: **Implemented / Mainnet Candidate**

## Fixed direction

- Alvenqis starts with Proof of Work.
- AlvenqisPoW version 1 is FiroPoW 0.9.4, based on ProgPoW revision 0.9.4.
- The period length is one block.
- The canonical block identifier is the validated FiroPoW final hash.
- Product nonce search is NVIDIA CUDA-only.
- Host FiroPoW remains mandatory for independent consensus validation.
- PoLW and energy-aware mining remain future research.
- Target block time is 60 seconds.

Blake3 leading-zero work was an earlier candidate prototype and is not the
active consensus PoW. Blake3 may still be used by non-PoW structures where
their specifications require it.

## Block acceptance

A node accepts a mined block only when all of the following hold:

- the canonical FiroPoW header preimage is well-formed;
- nonce, `mix_hash`, and `final_hash` recompute exactly;
- the final hash satisfies the required leading-zero-bit target;
- the declared difficulty equals the expected DAA output;
- version activation and checkpoint rules pass;
- previous-hash linkage, transactions, rewards, fees, and ledger transitions
  pass full validation.

The miner is not trusted. CUDA searches candidates, while `alvenqis-core`
independently validates every solution. CPU validation is not CPU mining: it
does not scan nonce ranges.

## Cumulative-work fork choice

- A block with `d` required leading-zero bits contributes `2^d` work units.
- Chain work is the checked `u128` sum of every validated block, including genesis.
- Competing branches must share the deterministic genesis and pass complete validation.
- A candidate is adopted only when cumulative work is strictly greater.
- Equal-work candidates retain the current chain.
- Canonical replacement is atomic and detached non-coinbase transactions are
  revalidated before returning to the mempool.

## Change control

Any change to the FiroPoW revision, period length, epoch/DAG rules, preimage,
target interpretation, or PoW version is consensus-critical. It requires a
documented activation or pre-launch reset, deterministic vectors, coordinated
node/miner/pool rollout, and new genesis approval evidence.

This implementation remains subject to the public launch gates in
`docs/release/NETWORK_MATURITY.md`; a working miner does not itself authorize a
Mainnet Live claim.
