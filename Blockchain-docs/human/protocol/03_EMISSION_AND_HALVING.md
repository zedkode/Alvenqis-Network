# Emission And Halving

Status: Implemented candidate emission / allocation policy incomplete

## Fixed Emission Facts

- max supply target: 60,000,000 ALVE;
- block time target: 60 seconds;
- blocks per year: 525,600;
- halving interval: 1,576,800 blocks;
- initial reward: 19.02587519 ALVE per block.

## Formula

Source-info uses a geometric emission model:

```text
initial_reward = max_supply / (2 * halving_interval)
initial_reward = 60,000,000 / (2 * 1,576,800)
initial_reward = 19.02587519 ALVE
```

## Schedule Interpretation

Implemented emission interpretation:
- reward epoch 0: `19.02587519 ALVE` per block;
- each halving epoch reduces the reward by half;
- long-run issuance converges toward the max supply target.

## Missing Policy Items

The following are still open and materially affect the final supply model:
- genesis allocation policy;
- premine or zero-premine confirmation;
- treasury allocation policy;
- founder or team vesting policy;
- contract gas economics and long-term policy after block subsidy declines.

These must be resolved before a final public allocation policy is approved. The
reward and supply-cap code already exists and is validated separately.

## Impact Notes

- Core: coinbase rules and total-supply accounting depend on these policies.
- Wallet: supply and reward displays must use draft labels until reviewed.
- Explorer and Indexer: emission charts can be documented now but remain provisional.
- RPC: supply endpoints should be designed only after amount precision and genesis policy are finalized.
