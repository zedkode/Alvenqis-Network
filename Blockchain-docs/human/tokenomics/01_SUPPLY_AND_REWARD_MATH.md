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
initial_reward = 19.02587519
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

## Impact Notes

- Core: reward calculation and supply accounting depend on amount precision.
- Wallet and Explorer: display logic depends on the final unit model.
- Indexer and RPC: total supply and reward endpoints depend on agreed atomic rules.
