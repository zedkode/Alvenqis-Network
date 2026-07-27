export const RPC_BASE_URL = (
  import.meta.env.VITE_ALVENQIS_RPC_URL ?? "https://rpcnode.dohotstudio.com"
).replace(/\/+$/, "");

export interface HealthResponse {
  ok: boolean;
  service: string;
  mode: string;
  network_id: string;
  network_name: string;
  status_label: string;
}

export interface NetworkResponse {
  protocol_parameters_id: string;
  protocol_version: number;
  block_version: number;
  network_id: string;
  network_name: string;
  status_label: string;
  ticker: string;
  address_prefix: string;
  address_standard_id: string;
  address_encoding: string;
  address_checksum_rule: string;
  address_payload_version: number;
  public_key_scheme: string;
  signature_standard_id: string;
  signature_scheme: string;
  tx_signing_domain: string;
  key_derivation_policy_id: string;
  block_time_seconds: number;
  decimals: number;
  atomic_units_per_alve: number;
  max_supply_atomic: number;
  halving_interval_blocks: number;
  initial_block_reward_atomic: number;
  pow_hash_algorithm: string;
  difficulty_adjustment_algorithm: string;
  fee_policy: string;
  default_rpc_port: number;
  default_p2p_port: number;
}

export interface StatusResponse {
  network_id: string;
  network_name: string;
  status_label: string;
  initialized: boolean;
  block_count: number;
  height: number | null;
  tip_hash: string | null;
  emitted_supply_atomic: number | null;
}

export interface IndexerStatusResponse {
  mode: string;
  initialized: boolean;
  index_dir: string;
  indexed_height: number | null;
  indexed_block_count: number;
  transaction_count: number;
  address_count: number;
  tip_hash: string | null;
}

export interface SupplySummary {
  emitted_supply_atomic: number;
  max_supply_atomic: number;
  remaining_supply_atomic: number;
}

export interface IndexedTransaction {
  lifecycle_status: string;
  hash: string;
  block_height: number;
  block_hash: string;
  block_transaction_count: number;
  transaction_index: number;
  version: number;
  nonce: number;
  from: string | null;
  to: string;
  amount_atomic: number;
  fee_atomic: number;
  max_fee_atomic: number;
  priority_fee_atomic: number;
  effective_fee_atomic: number;
  burned_fee_atomic: number;
  effective_priority_fee_atomic: number;
  base_fee_atomic: number;
  memo_hash: string | null;
  sender_public_key_hex: string | null;
  signature_hex: string | null;
  authorization_state: string;
}

export interface RpcTransactionResponse {
  lifecycle_status: string;
  hash: string;
  block_height: number | null;
  block_hash: string | null;
  version: number;
  nonce: number;
  from: string | null;
  to: string;
  amount_atomic: number;
  fee_atomic: number;
  max_fee_atomic: number;
  priority_fee_atomic: number;
  effective_fee_atomic: number;
  burned_fee_atomic: number;
  effective_priority_fee_atomic: number;
  base_fee_atomic: number;
  memo_hash: string | null;
  sender_public_key_hex: string | null;
  signature_hex: string | null;
  authorization_state: string;
  signature_standard_id: string;
  signatures_status: string;
}

export interface IndexedBlock {
  network_id: string;
  height: number;
  hash: string;
  previous_hash: string;
  merkle_root: string;
  base_fee_atomic: number;
  timestamp: number;
  nonce: number;
  difficulty_leading_zero_bits: number;
  transaction_count: number;
  miner_address: string;
  coinbase_payout_atomic: number;
  miner_reward_atomic: number;
  fees_atomic: number;
  burned_fees_atomic: number;
  priority_fees_atomic: number;
  transaction_hashes: string[];
}

export interface AddressActivity {
  address: string;
  exists_in_ledger: boolean;
  balance_atomic: number;
  total_received_atomic: number;
  total_sent_atomic: number;
  mined_reward_atomic: number;
  transaction_hashes: string[];
  sent_tx_hashes: string[];
  received_tx_hashes: string[];
  mined_block_heights: number[];
}

export interface IndexSummary {
  mode: string;
  network: string;
  status: string;
  indexed_height: number | null;
  indexed_block_count: number;
  transaction_count: number;
  address_count: number;
  tip_hash: string | null;
  latest_block_hash: string | null;
  latest_block_timestamp: number | null;
  supply: SupplySummary;
}

export interface IndexData {
  summary: IndexSummary;
  blocks_by_height: Record<string, IndexedBlock>;
  blocks_by_hash: Record<string, IndexedBlock>;
  transactions_by_hash: Record<string, IndexedTransaction>;
  addresses: Record<string, AddressActivity>;
  miner_rewards_by_block: Record<string, number>;
  fees_by_block: Record<string, number>;
}

export interface IndexOverviewResponse {
  summary: IndexSummary;
  recent_blocks: IndexedBlock[];
  recent_transactions: IndexedTransaction[];
}

export interface PaginatedResponse<T> {
  total: number;
  offset: number;
  limit: number;
  items: T[];
}

export interface MempoolStatusResponse {
  status: string;
  pending_count: number;
  anticipated_base_fee_atomic: number;
  total_fees_atomic: number;
  total_burned_fees_atomic: number;
  total_priority_fees_atomic: number;
  highest_priority_fee_atomic?: number;
  highest_max_fee_atomic?: number;
}

export interface MempoolResponse extends MempoolStatusResponse {
  transactions: RpcTransactionResponse[];
}

export interface ConnectedPeer {
  peer_id: string;
  address: string | null;
  handshake_validated: boolean;
  best_height: number | null;
  best_hash: string | null;
  validating: boolean;
  mining: boolean;
  hashrate_hs: number;
  connected_at_unix_seconds: number;
  last_error: string | null;
}

export interface NetworkMinerPresence {
  peer_id: string;
  hashrate_hs: number;
  template_height: number;
  updated_at_unix_seconds: number;
  local: boolean;
}

export interface P2pStatusResponse {
  mode: string;
  protocol_version: number;
  network_id: string;
  chain_magic_hex: string;
  local_peer_id: string;
  listen_addresses: string[];
  configured_seed_count: number;
  connected_peer_count: number;
  validated_peer_count: number;
  mining_peer_count: number;
  observed_network_hashrate_hs: number;
  miners: NetworkMinerPresence[];
  validating_peer_count: number;
  syncing: boolean;
  peers: ConnectedPeer[];
  last_error: string | null;
  updated_at_unix_seconds: number;
}

export interface FleetNodeResponse {
  node_id: string;
  node_name: string;
  advertise_host: string;
  p2p_multiaddr: string;
  online: boolean;
  height: number | null;
  connected_peers: number;
  mining_peers: number;
  observed_hashrate_hs: number;
  last_seen_unix_seconds: number;
}

export interface FleetTopologyResponse {
  mode: string;
  network_id: string;
  registered_node_count: number;
  online_node_count: number;
  direct_validated_connections: number;
  observed_miner_count: number;
  observed_hashrate_hs: number;
  nodes: FleetNodeResponse[];
}

export interface PoolStatusResponse {
  protocol: string;
  mode: string;
  status_label: string;
  pool_name: string;
  network_id: string;
  pool_address: string;
  upstream_status: string;
  upstream_error: string | null;
  pool_fee_basis_points: number;
  payout_scheme: string;
  connected_workers: number;
  estimated_hashrate_hs: number;
  blocks_found: number;
  matured_blocks: number;
  vardiff_enabled: boolean;
  target_share_seconds: number;
  rejected_requests: number;
  rate_limited_requests: number;
  active_bans: number;
}

export async function fetchJson<T>(path: string): Promise<T> {
  const response = await fetch(`${RPC_BASE_URL}${path}`);
  if (!response.ok) {
    let detail = response.statusText;
    try {
      const json = (await response.json()) as { error?: string };
      detail = json.error ?? detail;
    } catch {
      // keep status text when JSON parsing fails
    }
    throw new Error(`${response.status} ${detail}`);
  }
  return (await response.json()) as T;
}

export function indexerLag(
  chainHeight: number | null,
  indexedHeight: number | null
): number | null {
  if (chainHeight === null || indexedHeight === null) {
    return null;
  }
  return Math.max(0, chainHeight - indexedHeight);
}

export function indexedBlocks(index: IndexData): IndexedBlock[] {
  return Object.values(index.blocks_by_height).sort((a, b) => b.height - a.height);
}

export function indexedTransactions(index: IndexData): IndexedTransaction[] {
  return Object.values(index.transactions_by_hash).sort(
    (a, b) => b.block_height - a.block_height || b.transaction_index - a.transaction_index
  );
}

export function indexedAddresses(index: IndexData): AddressActivity[] {
  return Object.values(index.addresses).sort(
    (a, b) => b.balance_atomic - a.balance_atomic || a.address.localeCompare(b.address)
  );
}

export async function fetchLatestIndexedBlocks(count: number): Promise<IndexedBlock[]> {
  const status = await fetchJson<IndexerStatusResponse>("/indexer/status");
  if (!status.initialized || status.indexed_height === null) {
    return [];
  }

  const heights: number[] = [];
  for (
    let height = status.indexed_height;
    height >= 0 && heights.length < count;
    height -= 1
  ) {
    heights.push(height);
    if (height === 0) {
      break;
    }
  }

  return Promise.all(
    heights.map((height) => fetchJson<IndexedBlock>(`/indexer/blocks/${height}`))
  );
}

export async function fetchLatestMinedTransactions(
  count: number
): Promise<IndexedTransaction[]> {
  const blocks = await fetchLatestIndexedBlocks(5);
  const hashes = blocks.flatMap((block) => block.transaction_hashes).slice(0, count);
  return Promise.all(
    hashes.map((hash) => fetchJson<IndexedTransaction>(`/indexer/tx/${hash}`))
  );
}
