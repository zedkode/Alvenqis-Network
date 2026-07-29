# Supply And Reward Math

Status: Implemented candidate arithmetic / allocation policy incomplete

## Fixed Values

- max supply: `60,000,000 ALVE`
- halving interval: `1,576,800 blocks`
- initial reward: `19.02587519 ALVE`
- blocks per year: `525,600`

## Derivation

```text
initial_reward = max_supply / (2 * halving_interval)
initial_reward = 60,000,000 / (2 * 1,576,800)
ideal_initial_reward = 19.025875190258...
implemented_initial_reward = 19.02587519
```

## Time Interpretation

```text
1 block every 60 seconds
1,440 blocks per day
525,600 blocks per year
1,576,800 blocks per 3-year halving interval
```

## Precision Note

The canonical amount model uses:
- 8 decimals;
- 100,000,000 atomic units per ALVE.

This fixes amount precision while stable wire serialization and final allocation
policy remain separate gates.

Because rewards use integer atomic units and integer halving, the implemented
schedule issues `5,999,999,968,382,400` atomic units
(`59,999,999.68382400 ALVE`). It remains `31,617,600` atomic units
(`0.31617600 ALVE`) below the supply cap.

## Impact Notes

- Core: reward calculation and supply accounting depend on amount precision.
- Wallet and Explorer: display logic depends on the final unit model.
- Indexer and RPC: total supply and reward endpoints depend on agreed atomic rules.
