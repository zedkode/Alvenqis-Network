/**
 * Print public pool coordinator status.
 * Usage: node scripts/pool-status.mjs [poolUrl]
 */
import { createAlvenqisClient } from "../../alvenqis-sdk/dist/index.js";

const poolUrl =
  process.argv[2] ?? process.env.ALVENQIS_POOL_URL ?? "http://rpcnode.dohotstudio.com/pool";
const client = createAlvenqisClient({ poolUrl });

const pool = await client.poolStatus();

console.log("POOL", poolUrl);
console.log("name", pool.pool_name, "·", pool.status_label);
console.log("upstream", pool.upstream_status, pool.upstream_error ?? "");
console.log("workers online", pool.connected_workers);
console.log("hashrate_hs", pool.estimated_hashrate_hs);
console.log("shares", pool.accepted_shares, "blocks", pool.blocks_found, "matured", pool.matured_blocks);
console.log("maturity_confirmations", pool.block_maturity_confirmations);
console.log("pool_address", pool.pool_address);
console.log("fee_bp", pool.pool_fee_basis_points, "scheme", pool.payout_scheme);

const workers = pool.workers ?? [];
console.log("--- workers ---");
for (const w of workers.slice(0, 20)) {
  console.log(
    `${w.online ? "ON " : "off"} ${w.worker_name} ${w.miner_address.slice(0, 12)}… hs=${w.estimated_hashrate_hs} shares=${w.accepted_shares}`
  );
}
