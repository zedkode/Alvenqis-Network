# alvenqis-examples

Runnable scripts for **Alvenqis Mainnet Candidate** using `@alvenqis/sdk`.

## Setup

```bash
cd ../alvenqis-sdk
npm install
npm run build

cd ../alvenqis-examples
# no install required — scripts import the built SDK from ../alvenqis-sdk/dist
```

## Scripts

| Command | What it does |
|---------|----------------|
| `npm run chain-status` | Gateway health + tip height |
| `npm run pool-status` | Public pool workers / hashrate / blocks |
| `npm run pool-maturity` | Why pool blocks are immature (conf progress) |
| `npm run address-lookup -- alve1…` | Balance + next nonce |

Optional env:

```bash
export ALVENQIS_RPC_URL=https://rpcnode.dohotstudio.com
export ALVENQIS_POOL_URL=http://rpcnode.dohotstudio.com/pool
```

## Notes

- Network is **Mainnet Candidate / Prototype**, not public live Mainnet.
- Scripts are **read-only** — no key material, no contract ABI.
