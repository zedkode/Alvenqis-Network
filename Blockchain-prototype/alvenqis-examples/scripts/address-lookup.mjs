/**
 * Lookup a alve1 address balance/account on the public gateway.
 * Usage: node scripts/address-lookup.mjs alve1...
 */
import { createAlvenqisClient } from "../../alvenqis-sdk/dist/index.js";

const address = process.argv[2];
if (!address || !address.startsWith("alve1")) {
  console.error("Usage: node scripts/address-lookup.mjs alve1...");
  process.exit(2);
}

const client = createAlvenqisClient({
  rpcUrl: process.env.ALVENQIS_RPC_URL ?? "https://rpcnode.dohotstudio.com"
});

const [balance, account] = await Promise.all([
  client.addressBalance(address),
  client.addressAccount(address).catch(() => null)
]);

console.log("address", address);
console.log("balance_atomic", balance.balance_atomic);
if (account) {
  console.log("exists", account.exists);
  console.log("next_nonce", account.next_nonce);
  console.log("tip_height", account.tip_height);
  console.log("anticipated_base_fee_atomic", account.anticipated_base_fee_atomic);
}
