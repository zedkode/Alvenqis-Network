/**
 * @alvenqis/sdk — public read client for Alvenqis Mainnet Candidate.
 *
 * Scope: RPC gateway + mining pool public HTTP APIs.
 * Non-goals: smart contracts, key custody, private admin pool endpoints.
 */

export {
  AlvenqisClient,
  AlvenqisError,
  createAlvenqisClient,
  ALVENQIS_FIRST_ACCOUNT_NONCE
} from "./client.js";
export { poolBlockMaturity, type MaturityProgress } from "./maturity.js";
export type {
  AddressAccount,
  AddressBalance,
  Atomic,
  ChainStatus,
  HealthResponse,
  NetworkLimits,
  PoolBlock,
  PoolHistory,
  PoolStatus,
  PoolWorker,
  SignedTransactionBody,
  SubmitTransactionResponse,
  AlvenqisClientOptions
} from "./types.js";

export const ALVENQIS_DEFAULT_RPC_URL = "https://rpcnode.dohotstudio.com";
export const ALVENQIS_DEFAULT_MINING_RPC_URL = "http://127.0.0.1:10787";
export const ALVENQIS_DEFAULT_POOL_URL = "https://pool.dohotstudio.com";
export const ALVENQIS_DEFAULT_STRATUM_HOST = "stratum.dohotstudio.com";
export const ALVENQIS_DEFAULT_STRATUM_PORT = 3333;
export const ALVENQIS_NETWORK_ID = "alvenqis-mainnet-candidate";
