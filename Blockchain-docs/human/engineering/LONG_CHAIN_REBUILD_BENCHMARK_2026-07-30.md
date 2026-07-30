# Long-Chain Rebuild Benchmark — 2026-07-30

Status: Point-in-time engineering measurement / `TM-205` remains In Progress

## Scope

This report measures cold validation and state replay from the canonical SQLite
block store. It is a local, synthetic Devnet benchmark for the storage work
mapped from `reform.md` 2.1 to `TM-205`; it does not change serialization,
consensus, genesis, checkpoint, or production configuration.

The benchmark:

1. creates an isolated temporary Devnet;
2. mines a configurable number of empty child blocks using the normal node path;
3. starts a fresh child process so in-process validation caches are empty;
4. runs `validate_chain`;
5. verifies the rebuilt block count and height;
6. records elapsed time, SQLite bytes, and Linux peak RSS when `/proc` is available.

Source:
`Blockchain-prototype/alvenqis-node/examples/long_chain_rebuild.rs`.

## Command

```bash
cd Blockchain-prototype
cargo run -p alvenqis-node --example long_chain_rebuild --release -- --blocks 1000
```

The default is 1,000 child blocks. Larger local measurements can use
`--blocks COUNT`; no live endpoint or external host is involved.

## Environment

- repository baseline: `f31b900de715a90358d1a6869f5a22f22ab09040`
  plus the benchmark working-tree diff;
- operating system: Linux 7.1.3-2-cachyos, x86_64;
- processor: Intel Core i9-14900HX, 32 logical CPUs;
- memory: 32,548,640 KiB reported by `/proc/meminfo`;
- Rust: `rustc 1.97.0`;
- Cargo: `cargo 1.97.0`;
- profile: `release`.

## Results

| Child blocks | Resulting blocks | Height | Fixture build | Cold rebuild | Peak RSS | SQLite bytes |
|---:|---:|---:|---:|---:|---:|---:|
| 100 | 101 | 100 | 4,694 ms | 1,460 ms | 22,628 KiB | 172,032 |
| 1,000 | 1,001 | 1,000 | 47,895 ms | 12,969 ms | 27,072 KiB | 1,138,688 |

Both cold child processes exited successfully and returned the expected block
count and height.

## Interpretation

- Cold rebuild time increased by approximately 8.9 times when the child-block
  count increased by 10 times in this two-point sample.
- The measurement confirms that full replay remains material at only 1,000
  blocks. It supports, rather than closes, the planned header/index residency
  and on-demand body architecture.
- Fixture-build time includes Devnet proof-of-work and one durable SQLite append
  per block. It is not a node-startup measurement.
- Empty-block Devnet data does not represent transaction-heavy, reorg-heavy, or
  production-scale history.
- Peak RSS is available only on Linux. Timing is machine-specific and is not an
  accepted cross-platform performance threshold.

## Remaining `TM-205` evidence

`TM-205` must remain In Progress until:

- `TM-203` provides its required serialization prerequisite;
- explicit cross-platform rebuild-time and memory acceptance targets are approved;
- larger and transaction-bearing fixtures are measured;
- the planned headers/in-memory-index/on-disk-body architecture is implemented
  without an unapproved consensus-sensitive change;
- the completed design is covered by focused regression tests and broader node
  validation.

