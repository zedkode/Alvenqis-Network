use crate::error::{RpcError, RpcResult};
use crate::state::RpcState;
use alvenqis_core::{hash_to_hex, Block, Chain};
use alvenqis_node::{storage, SqliteBlockStore};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

#[derive(Clone, Debug)]
pub(crate) struct CachedChain {
    /// Fingerprint of the SQLite database and WAL when loaded.
    fingerprint: storage::ChainStorageFingerprint,
    blocks: Arc<Vec<Block>>,
    chain: Arc<Chain>,
    height: Option<u64>,
    tip_hash: Option<String>,
    emitted_supply_atomic: u64,
    cumulative_work: Option<String>,
}

#[derive(Debug)]
pub struct LoadedChain {
    pub blocks: Arc<Vec<Block>>,
    pub chain: Arc<Chain>,
    pub height: Option<u64>,
    pub tip_hash: Option<String>,
    pub emitted_supply_atomic: u64,
    pub cumulative_work: Option<String>,
}

pub(crate) fn file_fingerprint(path: &Path) -> (u64, u64) {
    match fs::metadata(path) {
        Ok(metadata) => {
            let length = metadata.len();
            let modified_at = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            (length, modified_at)
        }
        Err(_) => (0, 0),
    }
}

pub fn load_chain(state: &RpcState) -> RpcResult<LoadedChain> {
    let chain_path = Path::new(&state.config.chain_data_path);
    let fingerprint = storage::chain_storage_fingerprint(chain_path);

    // Hold the lock through a cache miss so concurrent dashboard requests do
    // not all replay and validate the complete chain after the same new block.
    let mut guard = state
        .chain_cache
        .lock()
        .map_err(|_| RpcError::Config("chain cache lock poisoned".to_owned()))?;
    if let Some(cached) = guard.as_ref() {
        if cached.fingerprint == fingerprint {
            return Ok(LoadedChain {
                blocks: Arc::clone(&cached.blocks),
                chain: Arc::clone(&cached.chain),
                height: cached.height,
                tip_hash: cached.tip_hash.clone(),
                emitted_supply_atomic: cached.emitted_supply_atomic,
                cumulative_work: cached.cumulative_work.clone(),
            });
        }
    }

    let previous_cache = guard.as_ref().cloned();
    let blocks = load_blocks_from_cache_or_storage(chain_path, previous_cache.as_ref())?;
    if blocks.is_empty() {
        *guard = None;
        return Err(RpcError::Node(
            alvenqis_node::NodeError::ChainNotInitialized(storage::chain_file_path(chain_path)),
        ));
    }
    let chain = if let Some(cached) = previous_cache {
        if blocks.len() >= cached.blocks.len() && blocks[..cached.blocks.len()] == cached.blocks[..]
        {
            let mut chain = (*cached.chain).clone();
            for block in &blocks[cached.blocks.len()..] {
                chain.append_block(block.clone())?;
            }
            chain
        } else {
            Chain::from_blocks(state.config.network, blocks.iter().cloned())?
        }
    } else {
        Chain::from_blocks(state.config.network, blocks.iter().cloned())?
    };
    let height = chain.height();
    let tip_hash = chain.tip_hash()?.map(|hash| hash_to_hex(&hash));
    let emitted_supply_atomic = chain.emitted_supply().as_atomic();
    let cumulative_work = chain.cumulative_work().ok().map(|work| work.to_string());
    let blocks = Arc::new(blocks);
    let chain = Arc::new(chain);
    *guard = Some(CachedChain {
        fingerprint,
        blocks: Arc::clone(&blocks),
        chain: Arc::clone(&chain),
        height,
        tip_hash: tip_hash.clone(),
        emitted_supply_atomic,
        cumulative_work: cumulative_work.clone(),
    });
    Ok(LoadedChain {
        blocks,
        chain,
        height,
        tip_hash,
        emitted_supply_atomic,
        cumulative_work,
    })
}

fn load_blocks_from_cache_or_storage(
    chain_path: &Path,
    previous_cache: Option<&CachedChain>,
) -> RpcResult<Vec<Block>> {
    let Some(cached) = previous_cache.filter(|cached| !cached.blocks.is_empty()) else {
        return storage::load_blocks(chain_path).map_err(RpcError::from);
    };
    let next_height = u64::try_from(cached.blocks.len())
        .map_err(|_| RpcError::Config("cached chain length exceeds u64".to_owned()))?;
    let suffix = SqliteBlockStore::new(chain_path)
        .load_blocks_from_height(next_height.saturating_sub(1))
        .map_err(RpcError::from)?;
    let extends_cached_tip = suffix
        .first()
        .zip(cached.blocks.last())
        .is_some_and(|(stored_parent, cached_tip)| stored_parent == cached_tip);
    if !extends_cached_tip {
        return storage::load_blocks(chain_path).map_err(RpcError::from);
    }

    let mut blocks = Vec::with_capacity(cached.blocks.len() + suffix.len().saturating_sub(1));
    blocks.extend(cached.blocks.iter().cloned());
    blocks.extend(suffix.into_iter().skip(1));
    Ok(blocks)
}

pub(crate) async fn load_chain_async(state: &RpcState) -> RpcResult<LoadedChain> {
    let state = state.clone();
    tokio::task::spawn_blocking(move || load_chain(&state))
        .await
        .map_err(|error| RpcError::Config(format!("chain read task failed: {error}")))?
}

pub(crate) async fn load_tip_block_async(state: &RpcState) -> RpcResult<Option<Block>> {
    let chain_path = PathBuf::from(&state.config.chain_data_path);
    tokio::task::spawn_blocking(move || SqliteBlockStore::new(chain_path).load_tip_block())
        .await
        .map_err(|error| RpcError::Config(format!("chain tip read task failed: {error}")))?
        .map_err(RpcError::from)
}
