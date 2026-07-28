use crate::cache::chain_cache::file_fingerprint;
use crate::error::{RpcError, RpcResult};
use crate::state::RpcState;
use alvenqis_indexer::{
    load_index as load_index_snapshot, AddressActivity, IndexData, IndexedTransaction,
};
use std::path::Path;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(crate) struct CachedIndex {
    fingerprint: (u64, u64),
    pub(crate) data: Arc<IndexData>,
    pub(crate) transactions: Arc<Vec<IndexedTransaction>>,
    pub(crate) addresses: Arc<Vec<AddressActivity>>,
}

fn index_file_fingerprint(index_data_path: &Path) -> (u64, u64) {
    file_fingerprint(&alvenqis_indexer::storage::index_file_path(index_data_path))
}

fn load_cached_index(state: &RpcState) -> RpcResult<CachedIndex> {
    // The dedicated indexer writes atomically; request handlers only read/cache.
    let index_path = Path::new(&state.config.indexer_data_path);
    let fingerprint = index_file_fingerprint(index_path);
    let mut guard = state
        .index_cache
        .lock()
        .map_err(|_| RpcError::Config("index cache lock poisoned".to_owned()))?;
    if let Some(cached) = guard.as_ref() {
        if cached.fingerprint == fingerprint {
            return Ok(cached.clone());
        }
    }

    let data = Arc::new(load_index_snapshot(index_path)?);
    let mut transactions: Vec<IndexedTransaction> =
        data.transactions_by_hash.values().cloned().collect();
    transactions.sort_by(|left, right| {
        right
            .block_height
            .cmp(&left.block_height)
            .then_with(|| right.transaction_index.cmp(&left.transaction_index))
            .then_with(|| right.hash.cmp(&left.hash))
    });
    let mut addresses: Vec<AddressActivity> = data.addresses.values().cloned().collect();
    addresses.sort_by(|left, right| {
        right
            .balance_atomic
            .cmp(&left.balance_atomic)
            .then_with(|| left.address.cmp(&right.address))
    });
    let cached = CachedIndex {
        fingerprint,
        data: Arc::clone(&data),
        transactions: Arc::new(transactions),
        addresses: Arc::new(addresses),
    };
    *guard = Some(cached.clone());
    Ok(cached)
}

pub(crate) async fn load_cached_index_async(state: &RpcState) -> RpcResult<CachedIndex> {
    let state = state.clone();
    tokio::task::spawn_blocking(move || load_cached_index(&state))
        .await
        .map_err(|error| RpcError::Config(format!("index read task failed: {error}")))?
}

pub(crate) async fn load_index_data_async(state: &RpcState) -> RpcResult<Arc<IndexData>> {
    Ok(load_cached_index_async(state).await?.data)
}

pub fn load_index_data(state: &RpcState) -> RpcResult<Arc<IndexData>> {
    Ok(load_cached_index(state)?.data)
}
