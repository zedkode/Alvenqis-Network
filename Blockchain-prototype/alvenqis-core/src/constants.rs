pub const PROJECT_NAME: &str = "Alvenqis Network";
pub const TICKER: &str = "ALVE";
pub const ADDRESS_PREFIX: &str = "alve";
pub const DEVNET_ADDRESS_PREFIX: &str = "dalve";
pub const TESTNET_ADDRESS_PREFIX: &str = "talve";
pub const MAINNET_CANDIDATE_ADDRESS_PREFIX: &str = "alve";
pub const DECIMALS: u32 = 8;
pub const ATOMIC_UNITS_PER_ALVE: u64 = 100_000_000;
pub const MAX_SUPPLY_ALVE: u64 = 60_000_000;
pub const MAX_SUPPLY_ATOMIC: u64 = 6_000_000_000_000_000;
pub const BLOCK_TIME_SECONDS: u64 = 60;
/// Reject headers whose timestamp is more than this many seconds ahead of local time.
/// Matches common PoW practice (clock skew allowance without allowing unbounded future headers).
pub const MAX_FUTURE_BLOCK_DRIFT_SECONDS: u64 = 7_200;
/// Window size for Median-Time-Past (MTP) timestamp floor (Bitcoin-style, odd for clean median).
pub const MEDIAN_TIME_PAST_WINDOW: usize = 11;
pub const HALVING_INTERVAL_BLOCKS: u64 = 1_576_800;
pub const INITIAL_BLOCK_REWARD_ALVE: &str = "19.02587519";
pub const INITIAL_BLOCK_REWARD_ATOMIC: u64 = 1_902_587_519;
pub const CONSENSUS_STATUS: &str = "PoW first";
pub const POW_HASH_ALGORITHM: &str = "FiroPoW-0.9.4";
pub const DAA_ALGORITHM: &str = "LWMA";
pub const DAA_WINDOW_BLOCKS: usize = 60;
// Allow DAA to observe up to 12× target (12 min) of slow blocks so difficulty
// recovers faster after hashrate drops (still targets BLOCK_TIME_SECONDS=60).
pub const DAA_SOLVETIME_CLAMP_MULTIPLIER: u64 = 12;
pub const FEE_POLICY: &str = "EIP-1559-like base fee burn plus priority tip";
pub const INITIAL_BASE_FEE_ATOMIC: u64 = 1;
pub const MIN_BASE_FEE_ATOMIC: u64 = 1;
pub const BASE_FEE_MAX_CHANGE_DENOMINATOR: u64 = 8;
pub const TARGET_TRANSACTIONS_PER_BLOCK: u64 = 1;
/// Hard cap on transactions per block (including coinbase). DoS / bandwidth bound.
pub const MAX_TRANSACTIONS_PER_BLOCK: usize = 1_024;
/// Hard cap on a single transaction wire encoding size (bytes).
pub const MAX_TRANSACTION_WIRE_BYTES: usize = 16_384;
pub const FUTURE_RESEARCH_STATUS: &str = "PoLW / energy-aware mining";
pub const CURRENT_STATUS: &str = "Draft / Mainnet Candidate / Prototype";
pub const ADDRESS_FORMAT_STATUS: &str = "Prototype implementation on frozen launch standard";
pub const SIGNATURES_STATUS: &str = "Prototype implementation on frozen launch standard";
pub const TX_SIGNING_DOMAIN_PREFIX: &[u8] = b"alvenqis-tx-ed25519-v1";
