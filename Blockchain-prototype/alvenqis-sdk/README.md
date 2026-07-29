# @alvenqis/sdk

Public **TypeScript** client for Alvenqis **Mainnet Candidate / Prototype**.

## Scope

| Included | Not included |
|----------|----------------|
| RPC health, chain status, tip, blocks | Wallet key generation / signing |
| Address balance & account (next nonce) | Smart contracts / VM |
| Pool status, history, miner view, payouts list | Pool admin payout APIs |
| Block maturity helper (`height + N confs`) | Marketplace, Passport, staking |

Network label: **not public live Mainnet**. See
`../../Blockchain-docs/human/release/NETWORK_MATURITY.md`.

## Install (workspace)

```bash
cd alvenqis-sdk
npm install
npm run build
```

## Quick start

```ts
import { createAlvenqisClient, poolBlockMaturity } from "@alvenqis/sdk";

const alvenqis = createAlvenqisClient({
  rpcUrl: "https://rpcnode.dohotstudio.com",
  poolUrl: "http://rpcnode.dohotstudio.com/pool",
});

const health = await alvenqis.health();
const chain = await alvenqis.status();
const pool = await alvenqis.poolStatus();

console.log(health.network_id, chain.height, pool.connected_workers);

// Maturity: immature until tip >= blockHeight + block_maturity_confirmations (default 12)
const blocks = await alvenqis.poolBlocksWithMaturity();
for (const b of blocks) {
  console.log(b.height, b.maturity.label, b.maturity.remaining);
}
```

## API surface (v0.1)

### RPC

- `health()`
- `status()`
- `chainTip()`
- `blockByHeight(height)`
- `transaction(hash)`
- `addressBalance(alve1…)`
- `addressAccount(alve1…)`
- `indexerSummary()`
- `p2pStatus()`

### Pool (public)

- `poolStatus()`
- `poolHistory()`
- `poolMiner(address)`
- `poolPayouts()`
- `poolBlocksWithMaturity()`

### Helpers

- `poolBlockMaturity(height, tip, required, statusField?)`

## Examples

See `../alvenqis-examples/`.

## License

Apache-2.0 (aligns with protocol / tooling direction in `docs/legal/LICENSING_POLICY.md`).
