//! RPC + public pool client surface for Alvenqis Mainnet Candidate.
//!
//! Includes L1 gateway reads/writes and **public** mining-pool read APIs.
//! Does not include pool admin / private payout mutation endpoints.

mod client;
mod models;

#[cfg(feature = "blocking")]
pub use client::blocking::BlockingRpcClient;
pub use client::RpcClient;
pub use models::{
    AddressAccountResponse, AddressBalanceResponse, AtomicValue, BlockResponse,
    ChainHeightResponse, ChainTipResponse, HealthResponse, IndexSummary, IndexedAddressResponse,
    IndexedBlockResponse, IndexedTransactionResponse, IndexerStatusResponse,
    IndexerSummaryResponse, MempoolStatusResponse, NetworkResponse, P2pStatusResponse, PoolBlock,
    PoolBlockWithMaturity, PoolHistoryResponse, PoolStatusResponse, PoolWorker, StatusResponse,
    SubmitTransactionResponse, SupplyResponse, SupplySummary, SyncStatusResponse,
    TransactionResponse,
};
