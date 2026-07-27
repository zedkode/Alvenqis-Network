# Transfer Fees and Contract Gas

Status: Transfer fee model implemented / contract gas unresolved

## Implemented transfer fee model

- each block carries a base fee;
- signed transfers carry maximum-fee and priority-fee values;
- the base fee is burned;
- the miner receives the priority tip;
- wallet/node/mempool validation enforces fee bounds;
- ledger state tracks burned fees separately from emitted subsidy;
- base-fee adjustment is deterministic from prior block utilization.

This is the accepted TM-106 implementation direction. Wire serialization and
long-term economic review remain candidate gates, but documentation must not
describe the transfer fee model as nonexistent or miner-only.

## Unresolved contract execution model

- VM/runtime choice;
- deterministic gas units and metering;
- relationship between transfer base fees and execution resource prices;
- receipts, refunds, failure semantics, and state access pricing.

No contract gas behavior may be inferred from the implemented transfer fee
model until those items are explicitly accepted.
