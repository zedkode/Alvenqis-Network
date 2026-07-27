/**
 * Print gateway health + chain tip for Mainnet Candidate.
 * Usage: node scripts/chain-status.mjs [rpcUrl]
 */
import { createAlvenqisClient } from "../../alvenqis-sdk/dist/index.js";

const rpcUrl = process.argv[2] ?? process.env.ALVENQIS_RPC_URL ?? "https://rpcnode.dohotstudio.com";
const client = createAlvenqisClient({ rpcUrl });

const [health, status] = await Promise.all([client.health(), client.status()]);

console.log("RPC", rpcUrl);
console.log("health.ok", health.ok, "network", health.network_id ?? status.network_id);
console.log("status_label", status.status_label ?? health.status_label);
console.log("height", status.height, "blocks", status.block_count);
console.log("tip", status.tip_hash);
console.log("emitted_supply_atomic", status.emitted_supply_atomic);
