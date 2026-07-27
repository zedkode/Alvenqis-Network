# Alvenqis Tokenomics Overview

Status: Implemented candidate economics / allocation policy incomplete

## Implemented facts

- native asset: ALVE;
- maximum-supply cap: 60,000,000 ALVE;
- decimals: 8 and 100,000,000 atomic units per ALVE;
- target block time: 60 seconds;
- halving interval: 1,576,800 blocks;
- initial block reward: 19.02587519 ALVE;
- reward uses integer atomic units and right-shift halving;
- transfer base fee is burned and the priority tip is paid to the miner;
- emitted supply counts subsidy only, never transferred fees.

## Unresolved policy

- final genesis/premine statement beyond the implemented candidate genesis;
- treasury or development allocation;
- founder/team allocation and vesting;
- contract execution gas metering;
- long-term governance over economic changes.

These unresolved items block a final production token-allocation statement, but
they do not make the implemented reward or transfer-fee code nonexistent.

## Communication rule

Describe exact implemented arithmetic as candidate protocol behavior. Describe
allocation, treasury, investment, listing, yield, staking, and future contract
economics only as unresolved or planned. Never promise returns.
