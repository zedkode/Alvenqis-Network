use super::models::{
    AddressAccountResponse, AddressBalanceResponse, BlockResponse, ChainHeightResponse,
    ChainTipResponse, HealthResponse, IndexedAddressResponse, IndexedBlockResponse,
    IndexedTransactionResponse, IndexerStatusResponse, IndexerSummaryResponse,
    MempoolStatusResponse, NetworkResponse, P2pStatusResponse, PoolBlockWithMaturity,
    PoolHistoryResponse, PoolStatusResponse, StatusResponse, SubmitTransactionResponse,
    SupplyResponse, SyncStatusResponse, TransactionResponse,
};
use crate::config::NetworkConfig;
use crate::error::{Result, SdkError};
use crate::maturity::{pool_block_maturity, DEFAULT_BLOCK_MATURITY_CONFIRMATIONS};
use std::time::{Duration, Instant};

const RPC_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const RPC_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const RPC_GET_ATTEMPTS: usize = 3;

fn retry_delay(attempt: usize) -> Duration {
    Duration::from_millis(250 * (attempt as u64 + 1))
}

fn retryable_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

/// Async JSON HTTP client for the Alvenqis RPC gateway + public pool APIs.
#[derive(Clone, Debug)]
pub struct RpcClient {
    config: NetworkConfig,
    http: reqwest::Client,
}

impl RpcClient {
    pub fn new(config: NetworkConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("alvenqis-sdk-rust/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(RPC_CONNECT_TIMEOUT)
            .timeout(RPC_REQUEST_TIMEOUT)
            .build()
            .map_err(|error| SdkError::rpc(error.to_string()))?;
        Ok(Self { config, http })
    }

    pub fn config(&self) -> &NetworkConfig {
        &self.config
    }

    pub async fn health(&self) -> Result<HealthResponse> {
        self.get_json("/health").await
    }

    pub async fn network(&self) -> Result<NetworkResponse> {
        self.get_json("/network").await
    }

    pub async fn status(&self) -> Result<StatusResponse> {
        self.get_json("/status").await
    }

    pub async fn tip(&self) -> Result<ChainTipResponse> {
        self.get_json("/chain/tip").await
    }

    pub async fn height(&self) -> Result<ChainHeightResponse> {
        self.get_json("/chain/height").await
    }

    pub async fn sync_status(&self) -> Result<SyncStatusResponse> {
        self.get_json("/sync/status").await
    }

    pub async fn supply(&self) -> Result<SupplyResponse> {
        self.get_json("/supply").await
    }

    pub async fn mempool_status(&self) -> Result<MempoolStatusResponse> {
        self.get_json("/mempool/status").await
    }

    /// Gateway P2P summary (`GET /p2p/status`).
    pub async fn p2p_status(&self) -> Result<P2pStatusResponse> {
        self.get_json("/p2p/status").await
    }

    pub async fn balance(&self, address: &str) -> Result<AddressBalanceResponse> {
        self.get_json(&format!("/addresses/{address}/balance"))
            .await
    }

    pub async fn account(&self, address: &str) -> Result<AddressAccountResponse> {
        self.get_json(&format!("/addresses/{address}/account"))
            .await
    }

    /// Ledger-backed next sequential spend nonce for `address`.
    pub async fn next_nonce(&self, address: &str) -> Result<u64> {
        Ok(self.account(address).await?.next_nonce)
    }

    pub async fn transaction(&self, tx_hash: &str) -> Result<TransactionResponse> {
        self.get_json(&format!("/transactions/{tx_hash}")).await
    }

    pub async fn block_latest(&self) -> Result<BlockResponse> {
        self.get_json("/blocks/latest").await
    }

    pub async fn block_by_height(&self, height: u64) -> Result<BlockResponse> {
        self.get_json(&format!("/blocks/{height}")).await
    }

    pub async fn block_by_hash(&self, hash: &str) -> Result<BlockResponse> {
        self.get_json(&format!("/blocks/hash/{hash}")).await
    }

    /// Fetch up to `count` recent blocks ending at the current tip (newest first).
    pub async fn recent_blocks(&self, count: usize) -> Result<Vec<BlockResponse>> {
        let count = count.clamp(1, 32);
        let tip = self.tip().await?;
        let mut blocks = Vec::with_capacity(count);
        let start = tip.height.saturating_sub((count as u64).saturating_sub(1));
        for height in (start..=tip.height).rev() {
            match self.block_by_height(height).await {
                Ok(block) => blocks.push(block),
                Err(SdkError::RpcHttp { status: 404, .. }) => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(blocks)
    }

    pub async fn indexer_status(&self) -> Result<IndexerStatusResponse> {
        self.get_json("/indexer/status").await
    }

    /// Parses only the `summary` object from `/indexer/summary` (ignores large maps).
    pub async fn indexer_summary(&self) -> Result<IndexerSummaryResponse> {
        self.get_json("/indexer/summary").await
    }

    pub async fn indexer_block_latest(&self) -> Result<IndexedBlockResponse> {
        self.get_json("/indexer/blocks/latest").await
    }

    pub async fn indexer_block_by_height(&self, height: u64) -> Result<IndexedBlockResponse> {
        self.get_json(&format!("/indexer/blocks/{height}")).await
    }

    pub async fn indexer_transaction(&self, hash: &str) -> Result<IndexedTransactionResponse> {
        self.get_json(&format!("/indexer/tx/{hash}")).await
    }

    pub async fn indexer_address(&self, address: &str) -> Result<IndexedAddressResponse> {
        self.get_json(&format!("/indexer/address/{address}")).await
    }

    /// Submit a signed transaction (any JSON-serializable body matching the gateway schema).
    pub async fn submit<T: serde::Serialize>(
        &self,
        transaction: &T,
    ) -> Result<SubmitTransactionResponse> {
        let url = self.config.rpc_url("/transactions");
        let response = self
            .http
            .post(&url)
            .json(transaction)
            .send()
            .await
            .map_err(|error| SdkError::rpc(format!("POST {url}: {error}")))?;
        Self::decode_response(response, &url).await
    }

    // —— Public pool (read-only) ——

    pub async fn pool_status(&self) -> Result<PoolStatusResponse> {
        self.get_pool_json("/api/v1/pool/status").await
    }

    pub async fn pool_history(&self) -> Result<PoolHistoryResponse> {
        self.get_pool_json("/api/v1/pool/history").await
    }

    pub async fn pool_miner(&self, address: &str) -> Result<serde_json::Value> {
        let a = address.trim();
        if !a.starts_with("alve1") {
            return Err(SdkError::input(
                "miner address must be a alve1… Mainnet Candidate address",
            ));
        }
        self.get_pool_json(&format!("/api/v1/miners/{a}")).await
    }

    pub async fn pool_payouts(&self) -> Result<serde_json::Value> {
        self.get_pool_json("/api/v1/payouts").await
    }

    /// Pool blocks (history preferred, else recent) with maturity progress vs chain tip.
    pub async fn pool_blocks_with_maturity(&self) -> Result<Vec<PoolBlockWithMaturity>> {
        let chain = self.status().await?;
        let pool = self.pool_status().await?;
        let tip = chain.height;
        let required = pool
            .block_maturity_confirmations
            .unwrap_or(DEFAULT_BLOCK_MATURITY_CONFIRMATIONS);

        let mut blocks = pool.recent_blocks;
        if let Ok(history) = self.pool_history().await {
            if !history.blocks.is_empty() {
                blocks = history.blocks;
            }
        }

        Ok(blocks
            .into_iter()
            .map(|b| {
                let status = b.status.clone().unwrap_or_else(|| "unknown".to_owned());
                let maturity = pool_block_maturity(b.height, tip, required, Some(status.as_str()));
                PoolBlockWithMaturity {
                    height: b.height,
                    hash: b.hash,
                    status,
                    reward_atomic: b.reward_atomic,
                    maturity,
                }
            })
            .collect())
    }

    /// Poll transaction lifecycle until `mined` or timeout.
    pub async fn await_status(
        &self,
        tx_hash: &str,
        timeout: Duration,
        poll_interval: Duration,
    ) -> Result<TransactionResponse> {
        let started = Instant::now();
        let mut last_status = String::from("unknown");
        loop {
            if started.elapsed() >= timeout {
                return Err(SdkError::TxTimeout {
                    elapsed_ms: started.elapsed().as_millis() as u64,
                    last_status,
                });
            }

            match self.transaction(tx_hash).await {
                Ok(tx) => {
                    last_status = tx.lifecycle_status.clone();
                    if tx.lifecycle_status.eq_ignore_ascii_case("mined") {
                        return Ok(tx);
                    }
                }
                Err(SdkError::RpcHttp { status: 404, .. }) => {
                    last_status = String::from("not_found");
                }
                Err(error) => return Err(error),
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.config.rpc_url(path);
        self.get_url_json(&url).await
    }

    async fn get_pool_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.config.pool_url(path)?;
        self.get_url_json(&url).await
    }

    async fn get_url_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        for attempt in 0..RPC_GET_ATTEMPTS {
            let response = match self.http.get(url).send().await {
                Ok(response) => response,
                Err(error)
                    if attempt + 1 < RPC_GET_ATTEMPTS
                        && (error.is_connect() || error.is_timeout()) =>
                {
                    tokio::time::sleep(retry_delay(attempt)).await;
                    continue;
                }
                Err(error) => return Err(SdkError::rpc(format!("GET {url}: {error}"))),
            };
            if attempt + 1 < RPC_GET_ATTEMPTS && retryable_status(response.status()) {
                tokio::time::sleep(retry_delay(attempt)).await;
                continue;
            }
            return Self::decode_response(response, url).await;
        }
        unreachable!("RPC GET retry loop always returns on its final attempt")
    }

    async fn decode_response<T: serde::de::DeserializeOwned>(
        response: reqwest::Response,
        url: &str,
    ) -> Result<T> {
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| SdkError::rpc(format!("read body {url}: {error}")))?;
        if !status.is_success() {
            return Err(SdkError::RpcHttp {
                status: status.as_u16(),
                url: url.to_owned(),
                body,
            });
        }
        serde_json::from_str(&body).map_err(|error| SdkError::RpcDecode(error.to_string()))
    }
}

#[cfg(feature = "blocking")]
pub mod blocking {
    use super::*;

    /// Blocking RPC client for simple CLI tooling.
    #[derive(Clone, Debug)]
    pub struct BlockingRpcClient {
        config: NetworkConfig,
        http: reqwest::blocking::Client,
    }

    impl BlockingRpcClient {
        pub fn new(config: NetworkConfig) -> Result<Self> {
            let http = reqwest::blocking::Client::builder()
                .user_agent(concat!("alvenqis-sdk-rust/", env!("CARGO_PKG_VERSION")))
                .connect_timeout(RPC_CONNECT_TIMEOUT)
                .timeout(RPC_REQUEST_TIMEOUT)
                .build()
                .map_err(|error| SdkError::rpc(error.to_string()))?;
            Ok(Self { config, http })
        }

        pub fn config(&self) -> &NetworkConfig {
            &self.config
        }

        pub fn health(&self) -> Result<HealthResponse> {
            self.get_json("/health")
        }

        pub fn network(&self) -> Result<NetworkResponse> {
            self.get_json("/network")
        }

        pub fn status(&self) -> Result<StatusResponse> {
            self.get_json("/status")
        }

        pub fn tip(&self) -> Result<ChainTipResponse> {
            self.get_json("/chain/tip")
        }

        pub fn height(&self) -> Result<ChainHeightResponse> {
            self.get_json("/chain/height")
        }

        pub fn sync_status(&self) -> Result<SyncStatusResponse> {
            self.get_json("/sync/status")
        }

        pub fn supply(&self) -> Result<SupplyResponse> {
            self.get_json("/supply")
        }

        pub fn mempool_status(&self) -> Result<MempoolStatusResponse> {
            self.get_json("/mempool/status")
        }

        pub fn p2p_status(&self) -> Result<P2pStatusResponse> {
            self.get_json("/p2p/status")
        }

        pub fn balance(&self, address: &str) -> Result<AddressBalanceResponse> {
            self.get_json(&format!("/addresses/{address}/balance"))
        }

        pub fn account(&self, address: &str) -> Result<AddressAccountResponse> {
            self.get_json(&format!("/addresses/{address}/account"))
        }

        /// Ledger-backed next sequential spend nonce for `address`.
        pub fn next_nonce(&self, address: &str) -> Result<u64> {
            Ok(self.account(address)?.next_nonce)
        }

        pub fn transaction(&self, tx_hash: &str) -> Result<TransactionResponse> {
            self.get_json(&format!("/transactions/{tx_hash}"))
        }

        pub fn block_latest(&self) -> Result<BlockResponse> {
            self.get_json("/blocks/latest")
        }

        pub fn block_by_height(&self, height: u64) -> Result<BlockResponse> {
            self.get_json(&format!("/blocks/{height}"))
        }

        pub fn block_by_hash(&self, hash: &str) -> Result<BlockResponse> {
            self.get_json(&format!("/blocks/hash/{hash}"))
        }

        /// Fetch up to `count` recent blocks ending at the current tip (newest first).
        pub fn recent_blocks(&self, count: usize) -> Result<Vec<BlockResponse>> {
            let count = count.clamp(1, 32);
            let tip = self.tip()?;
            let mut blocks = Vec::with_capacity(count);
            let start = tip.height.saturating_sub((count as u64).saturating_sub(1));
            for height in (start..=tip.height).rev() {
                match self.block_by_height(height) {
                    Ok(block) => blocks.push(block),
                    Err(SdkError::RpcHttp { status: 404, .. }) => continue,
                    Err(error) => return Err(error),
                }
            }
            Ok(blocks)
        }

        pub fn indexer_status(&self) -> Result<IndexerStatusResponse> {
            self.get_json("/indexer/status")
        }

        pub fn indexer_summary(&self) -> Result<IndexerSummaryResponse> {
            self.get_json("/indexer/summary")
        }

        pub fn indexer_block_latest(&self) -> Result<IndexedBlockResponse> {
            self.get_json("/indexer/blocks/latest")
        }

        pub fn indexer_block_by_height(&self, height: u64) -> Result<IndexedBlockResponse> {
            self.get_json(&format!("/indexer/blocks/{height}"))
        }

        pub fn indexer_transaction(&self, hash: &str) -> Result<IndexedTransactionResponse> {
            self.get_json(&format!("/indexer/tx/{hash}"))
        }

        pub fn indexer_address(&self, address: &str) -> Result<IndexedAddressResponse> {
            self.get_json(&format!("/indexer/address/{address}"))
        }

        pub fn submit<T: serde::Serialize>(
            &self,
            transaction: &T,
        ) -> Result<SubmitTransactionResponse> {
            let url = self.config.rpc_url("/transactions");
            let response = self
                .http
                .post(&url)
                .json(transaction)
                .send()
                .map_err(|error| SdkError::rpc(format!("POST {url}: {error}")))?;
            Self::decode_response(response, &url)
        }

        pub fn pool_status(&self) -> Result<PoolStatusResponse> {
            self.get_pool_json("/api/v1/pool/status")
        }

        pub fn pool_history(&self) -> Result<PoolHistoryResponse> {
            self.get_pool_json("/api/v1/pool/history")
        }

        pub fn pool_miner(&self, address: &str) -> Result<serde_json::Value> {
            let a = address.trim();
            if !a.starts_with("alve1") {
                return Err(SdkError::input(
                    "miner address must be a alve1… Mainnet Candidate address",
                ));
            }
            self.get_pool_json(&format!("/api/v1/miners/{a}"))
        }

        pub fn pool_payouts(&self) -> Result<serde_json::Value> {
            self.get_pool_json("/api/v1/payouts")
        }

        pub fn pool_blocks_with_maturity(&self) -> Result<Vec<PoolBlockWithMaturity>> {
            let chain = self.status()?;
            let pool = self.pool_status()?;
            let tip = chain.height;
            let required = pool
                .block_maturity_confirmations
                .unwrap_or(DEFAULT_BLOCK_MATURITY_CONFIRMATIONS);

            let mut blocks = pool.recent_blocks;
            if let Ok(history) = self.pool_history() {
                if !history.blocks.is_empty() {
                    blocks = history.blocks;
                }
            }

            Ok(blocks
                .into_iter()
                .map(|b| {
                    let status = b.status.clone().unwrap_or_else(|| "unknown".to_owned());
                    let maturity =
                        pool_block_maturity(b.height, tip, required, Some(status.as_str()));
                    PoolBlockWithMaturity {
                        height: b.height,
                        hash: b.hash,
                        status,
                        reward_atomic: b.reward_atomic,
                        maturity,
                    }
                })
                .collect())
        }

        fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
            let url = self.config.rpc_url(path);
            self.get_url_json(&url)
        }

        fn get_pool_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
            let url = self.config.pool_url(path)?;
            self.get_url_json(&url)
        }

        fn get_url_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
            for attempt in 0..RPC_GET_ATTEMPTS {
                let response = match self.http.get(url).send() {
                    Ok(response) => response,
                    Err(error)
                        if attempt + 1 < RPC_GET_ATTEMPTS
                            && (error.is_connect() || error.is_timeout()) =>
                    {
                        std::thread::sleep(retry_delay(attempt));
                        continue;
                    }
                    Err(error) => return Err(SdkError::rpc(format!("GET {url}: {error}"))),
                };
                if attempt + 1 < RPC_GET_ATTEMPTS && retryable_status(response.status()) {
                    std::thread::sleep(retry_delay(attempt));
                    continue;
                }
                return Self::decode_response(response, url);
            }
            unreachable!("RPC GET retry loop always returns on its final attempt")
        }

        fn decode_response<T: serde::de::DeserializeOwned>(
            response: reqwest::blocking::Response,
            url: &str,
        ) -> Result<T> {
            let status = response.status();
            let body = response
                .text()
                .map_err(|error| SdkError::rpc(format!("read body {url}: {error}")))?;
            if !status.is_success() {
                return Err(SdkError::RpcHttp {
                    status: status.as_u16(),
                    url: url.to_owned(),
                    body,
                });
            }
            serde_json::from_str(&body).map_err(|error| SdkError::RpcDecode(error.to_string()))
        }
    }
}

#[cfg(test)]
mod retry_tests {
    use super::*;

    #[test]
    fn retries_only_transient_http_statuses() {
        assert!(retryable_status(reqwest::StatusCode::TOO_MANY_REQUESTS));
        assert!(retryable_status(reqwest::StatusCode::BAD_GATEWAY));
        assert!(retryable_status(reqwest::StatusCode::SERVICE_UNAVAILABLE));
        assert!(!retryable_status(reqwest::StatusCode::BAD_REQUEST));
        assert!(!retryable_status(reqwest::StatusCode::GONE));
    }
}
