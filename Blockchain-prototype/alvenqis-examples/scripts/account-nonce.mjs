#!/usr/bin/env node
/**
 * Print ledger-backed next_nonce + balance for a Mainnet Candidate address.
 *
 * Usage:
 *   node alvenqis-examples/scripts/account-nonce.mjs [alve1…]
 *   ALVENQIS_RPC_URL=https://rpcnode.dohotstudio.com node …
 */
import { createAlvenqisClient, ALVENQIS_FIRST_ACCOUNT_NONCE } from "../../alvenqis-sdk/dist/index.js";

const address =
  process.argv[2]?.trim() ||
  process.env.ALVENQIS_ADDRESS ||
  "alve1qr4y5mrru2w9yz4774g8kyewchue23mk46ltu7ujgg0w56g5gmfzc8s6fh0";

const rpcUrl = process.env.ALVENQIS_RPC_URL || "https://rpcnode.dohotstudio.com";
const client = createAlvenqisClient({ rpcUrl });

const [account, network, status] = await Promise.all([
  client.addressAccount(address),
  client.network().catch(() => null),
  client.status().catch(() => null),
]);

const nextNonce = await client.nextNonce(address);

console.log(
  JSON.stringify(
    {
      address,
      balance_atomic: account.balance_atomic,
      next_nonce: nextNonce,
      first_account_nonce: network?.first_account_nonce ?? ALVENQIS_FIRST_ACCOUNT_NONCE,
      tip_height: account.tip_height ?? status?.height ?? null,
      tip_hash: account.tip_hash ?? status?.tip_hash ?? null,
      anticipated_base_fee_atomic: account.anticipated_base_fee_atomic,
      max_transactions_per_block: network?.max_transactions_per_block ?? null,
      cumulative_work: status?.cumulative_work ?? null,
    },
    null,
    2
  )
);
