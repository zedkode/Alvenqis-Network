/**
 * Keep these aligned with `alvenqis-sdk-rust` / `@alvenqis/sdk` (TS in alvenqis-sdk/) / `alvenqis-core` Mainnet Candidate defaults.
 * Rust desktop shell reads the same values from `alvenqis_sdk_rust::DEFAULT_*` constants.
 */
export const RPC_URL = "https://rpcnode.dohotstudio.com";
export const MINING_RPC_URL = "http://rpcnode.dohotstudio.com";
export const LOCAL_RPC_URL = "http://127.0.0.1:10787";
export const POOL_URL = `${MINING_RPC_URL}/pool`;
export const NETWORK_ID = "alvenqis-mainnet-candidate";
export const STATUS_LABEL = "Planned / Mainnet Candidate";
export const ADDRESS_PREFIX = "alve";
export const TICKER = "ALVE";
/** Default network snapshot poll (V2 snappier; remote floor still applies). */
export const REFRESH_INTERVAL_MS = 6_000;
/** Floor when the configured RPC is remote (not loopback). */
export const REMOTE_REFRESH_MIN_MS = 5_000;
/** Floor for local loopback RPC. */
export const LOCAL_REFRESH_MIN_MS = 1_500;
/** Miner / node log tail cadence while idle. */
export const LIVE_LOG_INTERVAL_MS = 2_000;
/** Miner console while actively mining. */
export const MINER_CONSOLE_ACTIVE_MS = 800;
export const APP_VERSION = "2.0.0-candidate";
export const APP_NAME = "Alvenqis Control Center V2";
/** Allowlisted miner CLI verbs for the interactive console (no free-form shell). */
export const MINER_SAFE_COMMANDS = [
  "status",
  "devices",
  "config",
  "config validate",
  "benchmark",
  "help"
] as const;
