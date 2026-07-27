export type Atomic = string;

export interface WalletMetadata {
  wallet_id: string;
  display_name: string;
  schema: string;
  network_id: string;
  address: string;
  public_key_hex: string;
  key_origin: string;
  derivation_path: string;
}

export interface DesktopBlock {
  height: number;
  hash: string;
  previous_hash: string;
  merkle_root: string;
  timestamp: number;
  nonce: number;
  difficulty_leading_zero_bits: number;
  transaction_count: number;
  miner_address: string;
  coinbase_payout_atomic: Atomic;
  miner_reward_atomic: Atomic;
  fees_atomic: Atomic;
  burned_fees_atomic: Atomic;
  priority_fees_atomic: Atomic;
  base_fee_atomic: Atomic;
  transaction_hashes: string[];
}

export interface DesktopTransaction {
  lifecycle_status: string;
  hash: string;
  block_height: number;
  transaction_index: number;
  nonce: number;
  from: string | null;
  to: string;
  amount_atomic: Atomic;
  effective_fee_atomic: Atomic;
  burned_fee_atomic: Atomic;
  effective_priority_fee_atomic: Atomic;
  authorization_state: string;
}

export interface DesktopPeer {
  peer_id: string;
  address: string | null;
  handshake_validated: boolean;
  best_height: number | null;
  validating: boolean;
  mining: boolean;
  hashrate_hs: number;
  last_error: string | null;
  /** Peer reputation score from node P2P (default ~50). */
  reputation_score?: number;
  /** True while the peer is under a temporary ban window. */
  banned?: boolean;
}

export interface DesktopNetworkMiner {
  peer_id: string;
  hashrate_hs: number;
  template_height: number;
  updated_at_unix_seconds: number;
  local: boolean;
}

export interface DesktopFleetNode {
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

export interface NetworkSnapshot {
  online: boolean;
  /**
   * Gateway is reachable but a recent poll hit timeout / 429 / 503 / 504.
   * UI should stay on last-known data and slow down polling — not treat as full outage.
   */
  degraded?: boolean;
  status_label: string;
  height: number | null;
  block_count: number;
  mempool_count: number;
  mempool_transactions: DesktopTransaction[];
  mempool_anticipated_base_fee_atomic: Atomic;
  mempool_total_fees_atomic: Atomic;
  mempool_total_burned_fees_atomic: Atomic;
  mempool_total_priority_fees_atomic: Atomic;
  balance_atomic: Atomic | null;
  emitted_supply_atomic: Atomic | null;
  max_supply_atomic: Atomic | null;
  tip_hash: string | null;
  indexed_height: number | null;
  indexed_blocks: number;
  indexed_transactions: number;
  indexed_addresses: number;
  latest_block_timestamp: number | null;
  latest_block_transactions: number;
  latest_block_reward_atomic: Atomic | null;
  latest_block_fees_atomic: Atomic | null;
  node_running: boolean;
  rpc_running: boolean;
  indexer_ready: boolean;
  miner_running: boolean;
  miner_hashrate_hs: number | null;
  miner_height: number | null;
  miner_accepted_blocks: number | null;
  /** Pool shares accepted (pending_local / share path). */
  miner_accepted_shares: number | null;
  miner_status: string | null;
  /** Warm-up / template / CUDA error text from metrics.json */
  miner_last_error?: string | null;
  miner_template_id: string | null;
  miner_difficulty_leading_zero_bits: number | null;
  miner_share_difficulty_leading_zero_bits: number | null;
  miner_eta_block_seconds: number | null;
  miner_eta_share_seconds: number | null;
  miner_hashes_attempted: string | null;
  miner_updated_at_unix_seconds: number | null;
  /** Configured product backend. Legacy values are migrated to CUDA. */
  miner_backend_mode: string | null;
  /** Active backend name reported by alvenqis-miner metrics */
  miner_active_backend: string | null;
  /** Active CUDA device count from metrics */
  miner_gpu_devices?: number | null;
  local_peer_id: string | null;
  p2p_listen_addresses: string[];
  configured_seed_count: number;
  connected_peer_count: number;
  validated_peer_count: number;
  mining_peer_count: number;
  observed_network_hashrate_hs: number;
  miners: DesktopNetworkMiner[];
  validating_peer_count: number;
  /** Active temporary bans from peer reputation (A-H05). */
  banned_peer_count?: number;
  reputation_enabled?: boolean;
  p2p_syncing: boolean;
  p2p_error: string | null;
  sync_status: "offline" | "discovering" | "syncing" | "synced";
  sync_target_height: number | null;
  sync_remaining_blocks: number | null;
  sync_progress_percent: number | null;
  sync_target_peer_count: number;
  recent_blocks: DesktopBlock[];
  recent_transactions: DesktopTransaction[];
  peers: DesktopPeer[];
  fleet_nodes?: DesktopFleetNode[];
  fleet_registered_nodes?: number;
  fleet_online_nodes?: number;
  pool_online?: boolean;
  pool_name?: string | null;
  pool_workers?: number;
  pool_hashrate_hs?: number;
  pool_blocks_found?: number;
  pool_vardiff_target_seconds?: number | null;
  pool_rejected_requests?: number;
  pool_rate_limited_requests?: number;
  pool_active_bans?: number;
  detail: string;
}

export interface PreparedTransaction {
  recipient: string;
  amount_atomic: Atomic;
  tip_atomic: Atomic;
  base_fee_atomic: Atomic;
  total_atomic: Atomic;
  available_atomic: Atomic;
  nonce: number;
  chain_tip: string;
}

export interface SubmissionResult {
  tx_hash: string;
  lifecycle_status: string;
  mempool_size: number;
}

export type UpdatePhase =
  | "idle"
  | "disabled"
  | "checking"
  | "available"
  | "unavailable"
  | "downloading"
  | "downloaded"
  | "installing"
  | "error";

export interface UpdateProgress {
  percent: number;
  transferred: number;
  total: number;
  bytes_per_second: number;
}

export interface UpdateState {
  phase: UpdatePhase;
  current_version: string;
  available_version: string | null;
  release_name: string | null;
  release_date: string | null;
  message: string;
  manual: boolean;
  progress: UpdateProgress | null;
  /** Components included in the release update. */
  components?: string[];
}

export type OperatorCommand =
  | "start" | "stop" | "restart" | "status" | "mine"
  | "validate" | "backup" | "miner-start" | "miner-stop";

export interface MinerStartOptions {
  /** Solo RPC or pool mining through Stratum TCP/TLS; nonce search is CUDA-only. */
  mode: "solo" | "stratum";
  /** Product compute backend. */
  backend?: "cuda";
  gpu_intensity?: number;
  gpu_devices?: string[];
  gpu_batch_size?: number;
  template_refresh_seconds?: number;
  status_interval_seconds?: number;
  worker_name?: string;
  /** Stratum host (when mode=stratum). */
  stratum_host?: string;
  stratum_port?: number;
  stratum_use_tls?: boolean;
  stratum_skip_tls_verify?: boolean;
  stratum_password?: string;
  stratum_timeout_seconds?: number;
}

/** Result of Control Center explorer in-app lookup (public RPC data only). */
export type ExplorerLookupKind =
  | "block"
  | "transaction"
  | "address"
  | "peer"
  | "pool_worker"
  | "pool"
  | "not_found";

export interface ExplorerLookupResult {
  kind: ExplorerLookupKind;
  query: string;
  query_kind?: string;
  data?: Record<string, unknown> | null;
  raw?: Record<string, unknown> | null;
  sources?: string[];
  notes?: string[];
  message?: string;
}

/** Device row from `alvenqis-miner devices --json`. */
export interface MiningDeviceInfo {
  id: string;
  backend: "gpu-cuda" | string;
  name: string;
  vendor: string;
  index: number;
  compute_units?: number | null;
  global_memory_bytes?: number | null;
  selected?: boolean;
}

export interface WalletCreateResult {
  metadata: WalletMetadata;
  recovery_confirmed: boolean;
  /** One-time BIP39 phrase for in-app backup UI. */
  recovery_phrase?: string;
}

export interface RpcSettings {
  rpc_url: string;
  default_rpc_url: string;
}

/** Visual shell — base dark/light + optional variants. */
export type ThemeId =
  | "dark"
  | "light"
  | "midnight"
  | "high-contrast"
  /** @deprecated prefer "dark" */
  | "alvenqis-dark"
  /** @deprecated prefer "midnight" */
  | "alvenqis-midnight";
export type DensityId = "comfortable" | "compact";
export type LanguageId = "en" | "ro";
export type AccentId = "cyan" | "gold" | "emerald";

/** Full Control Center preferences persisted by the Tauri backend. */
export interface AppSettings {
  rpc_url: string;
  mining_rpc_url: string;
  language: LanguageId;
  theme: ThemeId;
  density: DensityId;
  accent: AccentId;
  refresh_interval_ms: number;
  live_log_interval_ms: number;
  reduce_motion: boolean;
  confirm_before_operator: boolean;
  auto_start_services: boolean;
  start_minimized: boolean;
  notify_block_mined: boolean;
  notify_sound: boolean;
  notify_updates: boolean;
  /** Check GitHub Releases for application updates. */
  auto_update: boolean;
  /** Poll interval in seconds (min 60, default 900). */
  auto_update_interval_secs: number;
  hide_balances: boolean;
  mask_addresses: boolean;
  show_advanced_metrics: boolean;
  show_technical_labels: boolean;
  /** Default work source for Miner page: solo RPC or pool through Stratum TLS. */
  default_miner_mode: "solo" | "stratum";
  /** Default compute backend; only NVIDIA CUDA is supported. */
  default_miner_backend: "cuda";
  default_gpu_intensity: number;
  /** Explicit CUDA dispatch size; 0 selects intensity-based automatic sizing. */
  default_gpu_batch_size: number;
  default_template_refresh_seconds: number;
  default_status_interval_seconds: number;
  /** Selected CUDA device ids from alvenqis-miner devices (empty = all CUDA GPUs). */
  default_gpu_devices: string[];
  default_pool_url: string;
  /** Multi-pool list for Pool Control page (HTTP or HTTPS base URLs). */
  pool_urls: string[];
  default_worker_name: string;
  stratum_host: string;
  stratum_port: number;
  stratum_use_tls: boolean;
  stratum_skip_tls_verify: boolean;
  stratum_password: string;
  stratum_timeout_seconds: number;
  /** Saved custom miner console command lines (allowlisted verbs only). */
  miner_custom_commands: string[];
  default_page: string;
  open_external_explorer: boolean;
  keep_logs_days: number;
}

/** Public pool status / history payload (gateway pool HTTP API). */
export interface PoolWorkerRow {
  miner_address: string;
  worker_name: string;
  accepted_shares: number;
  blocks_found: number;
  estimated_hashrate_hs: number;
  assigned_difficulty_leading_zero_bits: number;
  last_share_unix_seconds: number;
  online: boolean;
}

export interface PoolBlockRow {
  height: number;
  hash: string;
  reward_atomic: number | string;
  distributable_atomic?: number | string;
  pool_fee_atomic?: number | string;
  found_at_unix_seconds: number;
  status: string;
  allocations?: Record<string, number | string>;
}

export interface PoolShareRow {
  share_id: number;
  job_id: string;
  miner_address: string;
  worker_name: string;
  nonce: number;
  hash: string;
  share_difficulty_leading_zero_bits: number;
  network_difficulty_leading_zero_bits: number;
  accepted_at_unix_seconds: number;
  block_candidate: boolean;
}

export interface PoolPayoutRow {
  payout_id: string;
  created_at_unix_seconds: number;
  status: string;
  items: Array<{ address: string; amount_atomic: number | string }>;
  transaction_hashes: string[];
}

export interface PoolAccountRow {
  address: string;
  immature_atomic: number | string;
  mature_atomic: number | string;
  pending_payout_atomic: number | string;
  paid_atomic: number | string;
}

export interface PoolCatalogEntry {
  pool_url: string;
  online: boolean;
  error?: string;
  pool_name?: string | null;
  status_label?: string | null;
  network_id?: string | null;
  connected_workers?: number;
  estimated_hashrate_hs?: number;
  blocks_found?: number;
  accepted_shares?: number;
  upstream_status?: string | null;
  pool_address?: string | null;
}

export interface PoolSnapshot {
  online: boolean;
  pool_url: string;
  fetched_at_unix_seconds: number;
  health?: Record<string, unknown> | null;
  status: Record<string, unknown>;
  history_available: boolean;
  workers: PoolWorkerRow[];
  blocks: PoolBlockRow[];
  shares: PoolShareRow[];
  payouts: PoolPayoutRow[];
  accounts: PoolAccountRow[];
  miner?: Record<string, unknown> | null;
  configured_pools: string[];
  default_pool_url: string;
}

export interface PoolCatalog {
  default_pool_url: string;
  pools: PoolCatalogEntry[];
  fetched_at_unix_seconds: number;
}

export interface PathInfo {
  workspace: string;
  local_root: string;
  user_data: string;
  settings_file: string;
  logs_dir: string;
  chain_data_hint: string;
  keystore_helper: string;
  platform: "windows" | "linux" | "other";
  app_version: string;
  packaged: boolean;
}

export interface DiagnosticsInfo {
  node_pid_present: boolean;
  rpc_pid_present: boolean;
  miner_pid_present: boolean;
  explorer_pid_present: boolean;
  node_log_bytes: number;
  rpc_log_bytes: number;
  miner_log_bytes: number;
  explorer_log_bytes: number;
  metrics_present: boolean;
  node_config_present: boolean;
}

/** Critical-path self-check from the Tauri backend (no UI dependency). */
export interface RuntimeHealth {
  ok: boolean;
  packaged: boolean;
  workspace: string;
  workspace_ok: boolean;
  local_root: string;
  local_root_writable: boolean;
  local_root_name_ok: boolean;
  user_data: string;
  user_data_writable: boolean;
  settings_file: string;
  settings_writable: boolean;
  keystore_helper: string;
  keystore_helper_ok: boolean;
  operator_script: string;
  operator_script_ok: boolean;
  local_operator_script_ok: boolean;
  bundled_node_ok: boolean;
  bundled_rpc_ok: boolean;
  bundled_miner_ok: boolean;
  bundled_indexer_ok: boolean;
  configs_ok: boolean;
  rpc_port_free: boolean;
  p2p_port_free: boolean;
  issues: string[];
}

export interface AlvenqisBridge {
  network: {
    snapshot(): Promise<NetworkSnapshot>;
    addSeed(seed: string): Promise<string>;
  };
  wallet: {
    metadata(): Promise<WalletMetadata | null>;
    list(): Promise<WalletMetadata[]>;
    select(walletId: string): Promise<WalletMetadata>;
    create(displayName: string): Promise<WalletCreateResult>;
    /** In-app paste import when recoveryPhrase is provided. */
    import(displayName: string, recoveryPhrase?: string): Promise<WalletMetadata>;
    remove(): Promise<void>;
  };
  transactions: {
    prepare(recipient: string, amount: string, tip: string): Promise<PreparedTransaction>;
    signAndSubmit(prepared: PreparedTransaction, confirmed: boolean): Promise<SubmissionResult>;
  };
  operator: {
    run(command: OperatorCommand, minerOptions?: MinerStartOptions): Promise<string>;
  };
  logs: {
    recent(service: string, lines?: number): Promise<string>;
    export(service: string): Promise<string | null>;
  };
  miner: {
    devices(): Promise<MiningDeviceInfo[]>;
    /**
     * Run an allowlisted alvenqis-miner CLI verb (status/devices/config/benchmark).
     * Free-form shell is refused for security.
     */
    runCommand(line: string): Promise<string>;
  };
  explorer: {
    open(path: string): Promise<void>;
    /** Safe in-app lookup against gateway/indexer/pool/P2P (public data only). */
    lookup(query: string): Promise<ExplorerLookupResult>;
  };
  pool: {
    /** Full public snapshot for one pool URL (status + history + optional miner). */
    snapshot(poolUrl?: string | null, minerAddress?: string | null): Promise<PoolSnapshot>;
    /** Probe all configured pool URLs. */
    catalog(): Promise<PoolCatalog>;
  };
  settings: {
    rpc(): Promise<RpcSettings>;
    setRpcUrl(value: string): Promise<string>;
    get(): Promise<AppSettings>;
    update(patch: Partial<AppSettings>): Promise<AppSettings>;
    reset(): Promise<AppSettings>;
    defaults(): Promise<AppSettings>;
    paths(): Promise<PathInfo>;
    diagnostics(): Promise<DiagnosticsInfo>;
    openPath(kind: "workspace" | "local_root" | "logs" | "user_data" | "settings_file"): Promise<void>;
    health(): Promise<RuntimeHealth>;
  };
  updates: {
    state(): Promise<UpdateState>;
    check(): Promise<UpdateState>;
    download(): Promise<void>;
    install(restart: boolean): Promise<void>;
    onState(listener: (state: UpdateState) => void): () => void;
  };
  app: {
    platform: "windows" | "linux" | "other";
    workspace(): Promise<string>;
    minimize(): Promise<void>;
    maximize(): Promise<void>;
    close(): Promise<void>;
    version(): Promise<string>;
  };
}
