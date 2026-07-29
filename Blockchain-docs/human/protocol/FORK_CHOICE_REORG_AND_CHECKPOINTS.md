# Fork Choice, Reorganization, and Checkpoints

Status: Implemented candidate baseline / adversarial evidence incomplete

## Canonical selection

Alvenqis selects the fully validated compatible chain with strictly greater
cumulative proof of work. Height alone is not fork-choice authority.

The current P2P path:

1. exchanges tips and exponential block locators;
2. finds a common ancestor;
3. stages and validates headers before branch blocks;
4. bounds staged branches;
5. validates network identity, genesis, headers, transactions, state, and PoW;
6. adopts only a strictly higher-work candidate;
7. archives detached blocks transactionally;
8. reconciles still-valid detached transactions into the mempool.

## Checkpoint interaction

Release-pinned checkpoints are explicit early-network safety inputs. A
candidate branch that conflicts with an enforced checkpoint is invalid even if
it reports greater work.

Checkpoint relaxation is not part of ordinary sync or reorg logic. It may occur
only through `13_CHECKPOINT_POLICY.md` and a separately approved release.

## Open evidence

- multi-host adversarial fork competition;
- interruption and restart at every header/block stage;
- durable deep-reorg recovery;
- malformed cumulative-work and locator inputs;
- eclipse/partition recovery after independent seed failover;
- bounded memory, disk, and concurrency under hostile peers;
- deterministic replay after crash and restore.

The unresolved 2,500 ALVE validator-threshold proposal is not part of fork
choice or block validity.
