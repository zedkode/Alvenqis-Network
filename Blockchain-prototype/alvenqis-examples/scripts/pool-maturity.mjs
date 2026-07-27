/**
 * Explain pool block maturity vs chain tip (why blocks stay "immature").
 * Usage: node scripts/pool-maturity.mjs
 */
import { createAlvenqisClient } from "../../alvenqis-sdk/dist/index.js";

const client = createAlvenqisClient({
  rpcUrl: process.env.ALVENQIS_RPC_URL ?? "https://rpcnode.dohotstudio.com",
  poolUrl: process.env.ALVENQIS_POOL_URL ?? "http://rpcnode.dohotstudio.com/pool"
});

const chain = await client.status();
const rows = await client.poolBlocksWithMaturity();

console.log("chain tip", chain.height);
console.log("pool blocks", rows.length);
console.log("");
console.log("height  status              conf   need_tip  remaining");
for (const row of rows) {
  const m = row.maturity;
  console.log(
    String(row.height).padStart(6),
    m.label.padEnd(20),
    `${m.confirmations}/${m.required}`.padEnd(6),
    String(m.matureAtTip).padStart(8),
    String(m.remaining).padStart(9)
  );
}
console.log("");
console.log("Rule: mature when tip >= block_height + block_maturity_confirmations (usually 12).");
