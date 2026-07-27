/** Atomic ALVE amounts as decimal strings when possible; gateways may also return numbers. */
export type Atomic = string | number;

export interface HealthResponse {
  ok: boolean;
  service?: string;
  mode?: string;
  network_id?: string;
  network_name?: string;
  status_label?: string;
}

export interface ChainStatus {
  network_id?: string;
  network_name?: string;
  status_label?: string;
  initialized?: boolean;
  block_count?: number;
  height?: number | null;
  tip_hash?: string | null;
  emitted_supply_atomic?: Atomic;
  index_in_sync?: boolean;
  index_lag_blocks?: number;
  cumulative_work?: string | null;
}

export interface AddressBalance {
  address: string;
  balance_atomic: Atomic;
  exists?: boolean;
  /** Ledger-backed next sequential spend nonce when gateway provides it. */
  next_nonce?: number;
}

/** Prefer this when you only need balance + nonce (lighter than full account). */
export type AddressBalanceWithNonce = AddressBalance & { next_nonce: number };

export interface SubmitTransactionResponse {
  status: string;
  tx_hash: string;
  lifecycle_status: string;
  mempool_size?: number;
}

/**
 * Signed transfer body accepted by `POST /transactions`.
 * Build offline (wallet/SDK-rust); this client only relays JSON.
 */
export interface SignedTransactionBody {
  version: number;
  nonce: number;
  from: string;
  to: string;
  amount: { atomic?: number } | number | string;
  max_fee?: { atomic?: number } | number | string;
  fee?: { atomic?: number } | number | string;
  priority_fee?: { atomic?: number } | number | string;
  memo_hash?: string | null;
  sender_public_key?: string;
  signature?: string;
  [key: string]: unknown;
}

export interface AddressAccount {
  address: string;
  exists?: boolean;
  balance_atomic: Atomic;
  /** Next sequential spend nonce (ledger-backed; first spend is 1). */
  next_nonce: number;
  tip_hash?: string | null;
  tip_height?: number | null;
  anticipated_base_fee_atomic?: Atomic;
}

/** Body / timing consensus limits from `GET /network`. */
export interface NetworkLimits {
  protocol_parameters_id?: string;
  protocol_version?: number;
  network_id?: string;
  max_transactions_per_block?: number;
  max_transaction_wire_bytes?: number;
  median_time_past_window?: number;
  max_future_block_drift_seconds?: number;
  first_account_nonce?: number;
  block_time_seconds?: number;
  pow_hash_algorithm?: string;
  fee_policy?: string;
}

export interface PoolWorker {
  miner_address: string;
  worker_name: string;
  accepted_shares: number;
  blocks_found: number;
  estimated_hashrate_hs: number;
  assigned_difficulty_leading_zero_bits: number;
  last_share_unix_seconds: number;
  online: boolean;
}

export interface PoolStatus {
  protocol?: string;
  mode?: string;
  status_label?: string;
  pool_name?: string;
  network_id?: string;
  pool_address?: string;
  upstream_status?: string;
  upstream_error?: string | null;
  pool_fee_basis_points?: number;
  payout_scheme?: string;
  minimum_payout_atomic?: Atomic;
  block_maturity_confirmations?: number;
  vardiff_enabled?: boolean;
  target_share_seconds?: number;
  accepted_shares?: number;
  connected_workers?: number;
  estimated_hashrate_hs?: number;
  blocks_found?: number;
  matured_blocks?: number;
  rejected_requests?: number;
  rate_limited_requests?: number;
  active_bans?: number;
  workers?: PoolWorker[];
  recent_blocks?: PoolBlock[];
}

export interface PoolBlock {
  height: number;
  hash: string;
  reward_atomic?: Atomic;
  distributable_atomic?: Atomic;
  pool_fee_atomic?: Atomic;
  found_at_unix_seconds?: number;
  status?: string;
  allocations?: Record<string, Atomic>;
}

export interface PoolHistory {
  protocol?: string;
  pool_name?: string;
  network_id?: string;
  pool_address?: string;
  status_label?: string;
  accepted_shares_counter?: number;
  connected_workers?: number;
  estimated_hashrate_hs?: number;
  blocks_found?: number;
  matured_blocks?: number;
  workers?: PoolWorker[];
  blocks?: PoolBlock[];
  shares?: unknown[];
  payouts?: unknown[];
  accounts?: unknown[];
}

export interface AlvenqisClientOptions {
  /** RPC / gateway base, e.g. https://rpcnode.dohotstudio.com */
  rpcUrl?: string;
  /** Pool coordinator base, e.g. http://rpcnode.dohotstudio.com/pool */
  poolUrl?: string;
  /** Optional fetch implementation (defaults to global fetch). */
  fetch?: typeof fetch;
  /** Request timeout in ms (best-effort via AbortSignal). */
  timeoutMs?: number;
}
