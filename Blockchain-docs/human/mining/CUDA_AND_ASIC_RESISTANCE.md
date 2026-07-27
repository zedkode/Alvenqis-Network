# CUDA and ASIC-resistance model

Status: **FiroPoW 0.9.4 implemented; independent security review still required**

Alvenqis PoW version 1 is FiroPoW 0.9.4, a ProgPoW-family memory-hard algorithm.
The implementation is GPU-oriented and uses a large epoch DAG intended to make
memory bandwidth a material part of mining cost. This is a stronger
commodity-GPU orientation than the previous Blake3 leading-zero prototype, but
it is not a guarantee that specialized hardware can never be built.

## Trust boundaries

- `alvenqis-core` is the consensus source of truth.
- CUDA may search nonces but cannot define acceptance rules.
- Every CUDA solution is revalidated by canonical host code before submission.
- Every node independently validates the block's `mix_hash`, `final_hash`,
  target, difficulty, transactions, and state transition.
- CUDA DAG samples are checked against the canonical core during initialization.
- CPU mining and OpenCL mining are absent from the product miner.

## Compatibility rule

Changing the PoW algorithm, FiroPoW revision, period length, epoch rules, header
preimage, or target interpretation is a consensus change. It requires explicit
activation or a pre-launch chain reset, new deterministic vectors, coordinated
node/miner/pool deployment, and updated genesis approval evidence.

Blake3 remains available for non-PoW structures where the protocol specifies it;
it is not the block mining algorithm.

## Required release evidence

- forced CUDA parity on a real NVIDIA device;
- deterministic core vectors;
- DAG item parity at multiple indices;
- solo and pool end-to-end submissions;
- release builds that fail without real CUDA linkage;
- independent review before a public Mainnet claim.
