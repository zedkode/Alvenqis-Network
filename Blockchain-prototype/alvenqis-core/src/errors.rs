use crate::crypto::Hash;
use std::error::Error;
use std::fmt;

pub type Result<T> = std::result::Result<T, AlvenqisError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlvenqisError {
    AmountOverflow,
    AmountParse(String),
    TooManyDecimals {
        value: String,
        max: u32,
    },
    InvalidAddress(String),
    InvalidKey(String),
    InvalidMnemonic(String),
    InvalidDerivationPath(String),
    InvalidSignature(String),
    InvalidHex(String),
    InvalidTransaction(String),
    InvalidFee(String),
    ZeroAmountTransaction,
    DuplicateTransactionHash(String),
    InsufficientBalance {
        address: String,
        available: u64,
        required: u64,
    },
    /// Account-model sequential nonce mismatch (first spend uses nonce 1).
    InvalidNonce {
        address: String,
        expected: u64,
        actual: u64,
    },
    EmptyTransactions,
    /// Block exceeds the hard transaction-count cap (including coinbase).
    TooManyTransactions {
        max: usize,
        actual: usize,
    },
    /// Transaction wire encoding exceeds the hard size cap.
    TransactionTooLarge {
        max: usize,
        actual: usize,
    },
    MissingCoinbase,
    DuplicateCoinbase,
    CoinbaseNotFirst,
    InvalidCoinbaseFee,
    InvalidCoinbaseAmount {
        expected: u64,
        actual: u64,
    },
    InvalidBaseFee {
        expected: u64,
        actual: u64,
    },
    InvalidMerkleRoot,
    InvalidNetwork {
        expected: String,
        actual: String,
    },
    InvalidPreviousHash {
        expected: Hash,
        actual: Hash,
    },
    InvalidHeight {
        expected: u64,
        actual: u64,
    },
    InvalidBlockVersion {
        expected: u32,
        actual: u32,
        height: u64,
    },
    InvalidCheckpoint {
        height: u64,
        expected: Hash,
        actual: Hash,
    },
    InvalidCheckpointDefinition(String),
    InvalidPow {
        required: u8,
        actual: u32,
    },
    /// Native FiroPoW evaluation failed — no canonical block identity is available.
    PowEvaluationFailed(String),
    InvalidDifficultyAdjustment {
        expected: u8,
        actual: u8,
    },
    InvalidCoinbaseReward {
        allowed: u64,
        actual: u64,
    },
    /// Block timestamp must be strictly greater than the previous block timestamp.
    InvalidTimestamp {
        previous: u64,
        actual: u64,
    },
    /// Block timestamp is too far ahead of the local clock (future skew).
    InvalidFutureTimestamp {
        now: u64,
        actual: u64,
        max_drift_seconds: u64,
    },
    /// Block timestamp must be strictly greater than the median time of recent ancestors (MTP).
    InvalidMedianTimePast {
        median: u64,
        actual: u64,
    },
    SupplyOverflow,
    ChainWorkOverflow,
    InvalidGenesis(String),
}

impl fmt::Display for AlvenqisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AmountOverflow => write!(f, "amount overflow"),
            Self::AmountParse(value) => write!(f, "invalid amount: {value}"),
            Self::TooManyDecimals { value, max } => {
                write!(f, "too many decimal places in amount {value}; max is {max}")
            }
            Self::InvalidAddress(message) => write!(f, "invalid address: {message}"),
            Self::InvalidKey(message) => write!(f, "invalid key: {message}"),
            Self::InvalidMnemonic(message) => write!(f, "invalid mnemonic: {message}"),
            Self::InvalidDerivationPath(message) => {
                write!(f, "invalid derivation path: {message}")
            }
            Self::InvalidSignature(message) => write!(f, "invalid signature: {message}"),
            Self::InvalidHex(message) => write!(f, "invalid hex: {message}"),
            Self::InvalidTransaction(message) => write!(f, "invalid transaction: {message}"),
            Self::InvalidFee(message) => write!(f, "invalid fee: {message}"),
            Self::ZeroAmountTransaction => {
                write!(f, "transaction amount must be greater than zero")
            }
            Self::DuplicateTransactionHash(tx_hash) => {
                write!(f, "duplicate transaction hash: {tx_hash}")
            }
            Self::InsufficientBalance {
                address,
                available,
                required,
            } => write!(
                f,
                "insufficient balance for {address}: available {available}, required {required}"
            ),
            Self::InvalidNonce {
                address,
                expected,
                actual,
            } => write!(
                f,
                "invalid nonce for {address}: expected {expected}, got {actual}"
            ),
            Self::EmptyTransactions => write!(f, "block must contain at least one transaction"),
            Self::TooManyTransactions { max, actual } => write!(
                f,
                "block has too many transactions: {actual} exceeds max {max}"
            ),
            Self::TransactionTooLarge { max, actual } => write!(
                f,
                "transaction wire encoding is too large: {actual} bytes exceeds max {max}"
            ),
            Self::MissingCoinbase => write!(f, "block is missing the first coinbase transaction"),
            Self::DuplicateCoinbase => {
                write!(f, "block contains more than one coinbase transaction")
            }
            Self::CoinbaseNotFirst => write!(f, "coinbase transaction must be first"),
            Self::InvalidCoinbaseFee => write!(f, "coinbase transaction fee must be zero"),
            Self::InvalidCoinbaseAmount { expected, actual } => write!(
                f,
                "invalid coinbase amount: expected {expected}, got {actual}"
            ),
            Self::InvalidBaseFee { expected, actual } => {
                write!(f, "invalid base fee: expected {expected}, got {actual}")
            }
            Self::InvalidMerkleRoot => write!(f, "invalid merkle root"),
            Self::InvalidNetwork { expected, actual } => {
                write!(f, "invalid network: expected {expected}, got {actual}")
            }
            Self::InvalidPreviousHash { expected, actual } => {
                write!(
                    f,
                    "invalid previous hash: expected {expected}, got {actual}"
                )
            }
            Self::InvalidHeight { expected, actual } => {
                write!(f, "invalid height: expected {expected}, got {actual}")
            }
            Self::InvalidBlockVersion {
                expected,
                actual,
                height,
            } => write!(
                f,
                "invalid block version at height {height}: expected {expected}, got {actual}"
            ),
            Self::InvalidCheckpoint {
                height,
                expected,
                actual,
            } => write!(
                f,
                "invalid checkpoint at height {height}: expected {expected}, got {actual}"
            ),
            Self::InvalidCheckpointDefinition(message) => {
                write!(f, "invalid checkpoint definition: {message}")
            }
            Self::InvalidPow { required, actual } => {
                write!(
                    f,
                    "invalid proof of work: required {required} leading zero bits, got {actual}"
                )
            }
            Self::PowEvaluationFailed(message) => {
                write!(f, "FiroPoW evaluation failed (no block identity): {message}")
            }
            Self::InvalidDifficultyAdjustment { expected, actual } => {
                write!(
                    f,
                    "invalid difficulty adjustment: expected {expected} leading zero bits, got {actual}"
                )
            }
            Self::InvalidCoinbaseReward { allowed, actual } => {
                write!(
                    f,
                    "coinbase reward exceeds allowed reward: allowed {allowed}, got {actual}"
                )
            }
            Self::InvalidTimestamp { previous, actual } => write!(
                f,
                "invalid block timestamp: must be strictly greater than previous {previous}, got {actual}"
            ),
            Self::InvalidFutureTimestamp {
                now,
                actual,
                max_drift_seconds,
            } => write!(
                f,
                "invalid block timestamp: {actual} is more than {max_drift_seconds}s ahead of local time {now}"
            ),
            Self::InvalidMedianTimePast { median, actual } => write!(
                f,
                "invalid block timestamp: must be strictly greater than median-time-past {median}, got {actual}"
            ),
            Self::SupplyOverflow => write!(f, "emitted supply would exceed max supply"),
            Self::ChainWorkOverflow => write!(f, "cumulative chain work overflow"),
            Self::InvalidGenesis(message) => write!(f, "invalid genesis configuration: {message}"),
        }
    }
}

impl Error for AlvenqisError {}
