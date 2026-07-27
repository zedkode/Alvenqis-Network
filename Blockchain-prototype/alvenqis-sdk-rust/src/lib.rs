//! # alvenqis-sdk-rust
//!
//! **Standalone** Rust client SDK for **Alvenqis Network Mainnet Candidate**
//! (prototype — not Mainnet Live).
//!
//! Protocol primitives (address, amount, keys, transactions) are **vendored**
//! inside this crate — there is **no** dependency on `alvenqis-core`.
//!
//! Coexists with the public TypeScript package `@alvenqis/sdk` at monorepo path `alvenqis-sdk/`.
//!
//! ## Essential surface
//! - [`NetworkConfig`] — network + RPC (+ public pool) base URLs
//! - [`WalletAccount`] — in-memory mnemonic / key account
//! - [`TransferBuilder`] — build and sign transfers
//! - [`RpcClient`] — async HTTP RPC + public pool (feature `native`, default)
//! - [`pool_block_maturity`] — pure maturity helper
//!
//! ## Non-goals
//! pool admin mutations, contracts, browser store extension.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod config;
pub mod error;
pub mod maturity;
pub mod protocol;
pub mod tx;
pub mod wallet;

#[cfg(feature = "native")]
pub mod rpc;

pub use config::{
    NetworkConfig, DEFAULT_LOCAL_MAINNET_CANDIDATE_RPC, DEFAULT_MAINNET_CANDIDATE_MINING_RPC,
    DEFAULT_MAINNET_CANDIDATE_POOL, DEFAULT_MAINNET_CANDIDATE_RPC,
    DEFAULT_MAINNET_CANDIDATE_STRATUM_HOST, DEFAULT_MAINNET_CANDIDATE_STRATUM_PORT,
};
pub use error::{Result, SdkError};
pub use maturity::{
    pool_block_maturity, MaturityProgress, MaturityStatus, DEFAULT_BLOCK_MATURITY_CONFIRMATIONS,
};
pub use tx::{SignedTransfer, TransferBuilder};
pub use wallet::{GeneratedMnemonic, WalletAccount};

#[cfg(feature = "native")]
pub use rpc::{
    AddressAccountResponse, AddressBalanceResponse, AtomicValue, BlockResponse,
    ChainHeightResponse, ChainTipResponse, HealthResponse, IndexSummary, IndexedAddressResponse,
    IndexedBlockResponse, IndexedTransactionResponse, IndexerStatusResponse,
    IndexerSummaryResponse, MempoolStatusResponse, NetworkResponse, P2pStatusResponse, PoolBlock,
    PoolBlockWithMaturity, PoolHistoryResponse, PoolStatusResponse, PoolWorker, RpcClient,
    StatusResponse, SubmitTransactionResponse, SupplyResponse, SupplySummary, SyncStatusResponse,
    TransactionResponse,
};

#[cfg(all(feature = "native", feature = "blocking"))]
pub use rpc::BlockingRpcClient;

// Protocol surface (standalone — not re-exported from alvenqis-core).
pub use protocol::{
    generate_mnemonic, hash_to_hex, Address, AlvenqisError, Amount, MnemonicWordCount, Network,
    PrivateKey, PublicKey, Signature, Transaction, UnsignedTransaction, WalletDerivationPath,
    ATOMIC_UNITS_PER_ALVE, BLOCK_TIME_SECONDS, CURRENT_STATUS, DECIMALS, INITIAL_BASE_FEE_ATOMIC,
    INITIAL_BLOCK_REWARD_ATOMIC, MAX_SUPPLY_ATOMIC, TICKER,
};

/// Compile-time marker used by wasm feature checks and docs.
#[cfg(feature = "wasm")]
pub const WASM_LOGIC_FEATURE: bool = true;

#[cfg(feature = "wasm")]
/// Pure helpers safe to call from wasm hosts (no filesystem, no native TLS RPC in v0).
pub mod wasm_logic {
    use crate::error::Result;
    use crate::protocol::{Address, Amount, Transaction};

    pub fn parse_address(value: &str) -> Result<Address> {
        Ok(Address::parse(value)?)
    }

    pub fn format_alve(amount: Amount) -> String {
        amount.format_alve()
    }

    pub fn parse_alve(value: &str) -> Result<Amount> {
        Ok(Amount::parse_alve(value)?)
    }

    pub fn verify_transaction(tx: &Transaction) -> Result<()> {
        Ok(tx.verify()?)
    }
}
