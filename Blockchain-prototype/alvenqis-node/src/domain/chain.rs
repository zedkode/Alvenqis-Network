use crate::config::NetworkConfig;
use crate::domain::genesis::verify_existing_genesis;
use crate::error::{NodeError, NodeResult};
use crate::mempool::reconcile_after_reorg;
use crate::storage::{self, BlockStore, SqliteBlockStore};
use alvenqis_core::{
    block_fee_summary, common_ancestor_height, hash_to_hex, select_fork, Block, Chain, ForkChoice,
    Network,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub const LOCAL_OPERATOR_ROOT: &str = ".alvenqis-local";

#[derive(Clone)]
struct CachedValidatedChain {
    fingerprint: storage::ChainStorageFingerprint,
    blocks: Vec<Block>,
    chain: Chain,
}

static VALIDATED_CHAIN_CACHE: OnceLock<Mutex<BTreeMap<PathBuf, CachedValidatedChain>>> =
    OnceLock::new();

#[derive(Clone, Debug)]
pub struct ChainSummary {
    pub network_id: String,
    pub network_name: String,
    pub status: String,
    pub block_count: usize,
    pub height: u64,
    pub tip_hash: String,
    pub emitted_supply_atomic: u64,
}

#[derive(Clone, Debug)]
pub enum StatusReport {
    Uninitialized {
        network_id: String,
        network_name: String,
        status: String,
        data_dir: String,
    },
    Ready(ChainSummary),
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ChainReorgSummary {
    pub common_ancestor_height: u64,
    pub detached_blocks: usize,
    pub attached_blocks: usize,
    pub previous_tip_hash: String,
    pub new_tip_hash: String,
    pub previous_chain_work: u128,
    pub new_chain_work: u128,
    pub dropped_mempool_transactions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BalanceSummary {
    pub address: String,
    pub balance_atomic: u64,
    pub exists: bool,
    /// Next sequential spend nonce for this account (ledger-backed).
    #[serde(default)]
    pub next_nonce: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StateBalanceEntry {
    pub address: String,
    pub balance_atomic: u64,
    #[serde(default)]
    pub next_nonce: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StateSummary {
    pub status: String,
    pub network_name: String,
    pub chain_status: String,
    pub height: u64,
    pub tip_hash: String,
    pub emitted_supply_atomic: u64,
    pub tracked_addresses: usize,
    pub latest_block_base_fee_atomic: u64,
    pub latest_block_fees_atomic: u64,
    pub latest_block_burned_fees_atomic: u64,
    pub latest_block_priority_fees_atomic: u64,
    pub latest_coinbase_reward_atomic: u64,
    pub balances: Vec<StateBalanceEntry>,
}

pub fn status(config_path: &Path, data_dir: &Path) -> NodeResult<StatusReport> {
    let config = NetworkConfig::load_from_path(config_path)?;
    ensure_network_storage_path(config.network, data_dir)?;
    match storage::load_blocks(data_dir) {
        Ok(blocks) => Ok(StatusReport::Ready(summarize_validated_blocks(
            config_path,
            &config,
            data_dir,
            &blocks,
        )?)),
        Err(NodeError::ChainNotInitialized(_)) => Ok(StatusReport::Uninitialized {
            network_id: config.network.network_id().to_owned(),
            network_name: config.human_name,
            status: config.status_label,
            data_dir: data_dir.display().to_string(),
        }),
        Err(error) => Err(error),
    }
}

pub fn validate_chain(config_path: &Path, data_dir: &Path) -> NodeResult<ChainSummary> {
    let (config, blocks, _) = load_validated_chain(config_path, data_dir)?;
    summarize_validated_blocks(config_path, &config, data_dir, &blocks)
}

pub fn adopt_candidate_chain(
    config_path: &Path,
    data_dir: &Path,
    mempool_dir: &Path,
    candidate_blocks: &[Block],
) -> NodeResult<ChainReorgSummary> {
    let config = NetworkConfig::load_from_path(config_path)?;
    ensure_network_storage_path(config.network, data_dir)?;
    let store = SqliteBlockStore::new(data_dir);
    let observed = store.load_blocks()?;
    let expected_tip = match observed.last() {
        Some(block) => hash_to_hex(&block.hash()?),
        None => {
            return Err(NodeError::ChainNotInitialized(storage::chain_file_path(
                data_dir,
            )))
        }
    };

    let (mut summary, detached, validated_candidate) = store.replace_validated(
        &expected_tip,
        candidate_blocks,
        |current_blocks, replacement| {
            let current_chain = build_validated_chain(config_path, &config, current_blocks)?;
            let candidate_chain = build_validated_chain(config_path, &config, replacement)?;
            if select_fork(current_blocks, replacement)? != ForkChoice::AdoptCandidate {
                return Err(NodeError::Input(
                    "candidate chain does not have strictly greater cumulative proof of work"
                        .to_owned(),
                ));
            }
            let ancestor = match common_ancestor_height(current_blocks, replacement)? {
                Some(height) => height,
                None => {
                    return Err(NodeError::GenesisMismatch {
                        expected: hash_to_hex(&current_blocks[0].hash()?),
                        actual: hash_to_hex(&replacement[0].hash()?),
                    })
                }
            };
            let detached: Vec<Block> = current_blocks
                .iter()
                .filter(|block| block.header.height > ancestor)
                .cloned()
                .collect();
            let attached_blocks = replacement
                .iter()
                .filter(|block| block.header.height > ancestor)
                .count();
            let new_tip_hash = match replacement.last() {
                Some(block) => hash_to_hex(&block.hash()?),
                None => String::new(),
            };
            let summary = ChainReorgSummary {
                common_ancestor_height: ancestor,
                detached_blocks: detached.len(),
                attached_blocks,
                previous_tip_hash: expected_tip.clone(),
                new_tip_hash,
                previous_chain_work: current_chain.cumulative_work()?,
                new_chain_work: candidate_chain.cumulative_work()?,
                dropped_mempool_transactions: Vec::new(),
            };
            Ok((summary, detached, candidate_chain))
        },
    )?;

    summary.dropped_mempool_transactions = reconcile_after_reorg(
        mempool_dir,
        &validated_candidate,
        &detached,
        config.max_mempool_transactions,
    )?;
    Ok(summary)
}

pub fn print_chain(config_path: &Path, data_dir: &Path) -> NodeResult<String> {
    let (config, blocks, chain) = load_validated_chain(config_path, data_dir)?;
    let summary = summarize_chain(&config, &blocks, &chain)?;

    let mut output = String::new();
    // fmt::Write to String is infallible in practice; still avoid expect/panic in production.
    let _ = writeln!(
        &mut output,
        "{} [{}] ({}) height={} blocks={} tip={}",
        summary.network_name,
        summary.network_id,
        summary.status,
        summary.height,
        summary.block_count,
        summary.tip_hash
    );

    for block in blocks {
        let _ = writeln!(
            &mut output,
            "network_id={} height={} timestamp={} nonce={} difficulty={} txs={} hash={} prev={}",
            block.header.network_id,
            block.header.height,
            block.header.timestamp,
            block.header.nonce,
            block.header.difficulty_leading_zero_bits,
            block.transactions.len(),
            hash_to_hex(&block.hash()?),
            hash_to_hex(&block.header.previous_hash),
        );

        let metrics = block_fee_summary(&block)?;
        let _ = writeln!(
            &mut output,
            "  reward_atomic={} fees_atomic={}",
            metrics.coinbase_reward.as_atomic(),
            metrics.total_fees.as_atomic()
        );
    }

    Ok(output)
}

pub fn balance(config_path: &Path, data_dir: &Path, address: &str) -> NodeResult<BalanceSummary> {
    let (_config, _blocks, chain) = load_validated_chain(config_path, data_dir)?;
    let balance = chain.state().balance_of(address);
    let next_nonce = chain.state().next_nonce_of(address);
    Ok(BalanceSummary {
        address: address.to_owned(),
        balance_atomic: balance.as_atomic(),
        exists: chain.state().balances().contains_key(address)
            || next_nonce > alvenqis_core::FIRST_ACCOUNT_NONCE,
        next_nonce,
    })
}

pub fn state(config_path: &Path, data_dir: &Path) -> NodeResult<StateSummary> {
    let (config, blocks, chain) = load_validated_chain(config_path, data_dir)?;
    state_summary(&config, &blocks, &chain)
}

pub fn format_status(report: &StatusReport) -> String {
    match report {
        StatusReport::Uninitialized {
            network_id,
            network_name,
            status,
            data_dir,
        } => format!(
            "network_id={} network={} status={} initialized=false data_dir={}",
            network_id, network_name, status, data_dir
        ),
        StatusReport::Ready(summary) => format!(
            "network_id={} network={} status={} initialized=true height={} blocks={} tip_hash={} emitted_supply_atomic={}",
            summary.network_id,
            summary.network_name,
            summary.status,
            summary.height,
            summary.block_count,
            summary.tip_hash,
            summary.emitted_supply_atomic
        ),
    }
}

pub(crate) fn build_validated_chain(
    config_path: &Path,
    config: &NetworkConfig,
    blocks: &[Block],
) -> NodeResult<Chain> {
    if config.network.requires_explicit_allow() {
        // Approval file check only — never re-mine genesis on the hot path
        // (create_block_template / RPC template would hang at difficulty 16).
        verify_existing_genesis(config_path, blocks)?;
    }
    Chain::from_blocks(config.network, blocks.iter().cloned()).map_err(NodeError::from)
}

pub(crate) fn summarize_validated_blocks(
    config_path: &Path,
    config: &NetworkConfig,
    data_dir: &Path,
    blocks: &[Block],
) -> NodeResult<ChainSummary> {
    if blocks.is_empty() {
        return Err(NodeError::ChainNotInitialized(storage::chain_file_path(
            data_dir,
        )));
    }

    let chain = build_validated_chain(config_path, config, blocks)?;

    summarize_chain(config, blocks, &chain)
}

pub(crate) fn load_validated_chain(
    config_path: &Path,
    data_dir: &Path,
) -> NodeResult<(NetworkConfig, Vec<Block>, Chain)> {
    let config = NetworkConfig::load_from_path(config_path)?;
    ensure_network_storage_path(config.network, data_dir)?;
    let fingerprint = storage::chain_storage_fingerprint(data_dir);
    let cache = VALIDATED_CHAIN_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    let previous = cache
        .lock()
        .map_err(|_| NodeError::Input("validated chain cache lock poisoned".to_owned()))?
        .get(data_dir)
        .cloned();
    if let Some(cached) = previous
        .as_ref()
        .filter(|cached| cached.fingerprint == fingerprint)
    {
        return Ok((config, cached.blocks.clone(), cached.chain.clone()));
    }
    // Do NOT call verified_mainnet_genesis_manifest / genesis_review_manifest here.
    // Those re-mine genesis at candidate difficulty (~16) on every template load and
    // freeze /mining/template for minutes on CPU-only validators.
    // Approval is enforced via verify_existing_genesis inside build_validated_chain.
    let blocks = storage::load_blocks(data_dir)?;
    let chain = if let Some(cached) = previous {
        if blocks.len() >= cached.blocks.len() && blocks[..cached.blocks.len()] == cached.blocks[..]
        {
            let mut chain = cached.chain;
            for block in &blocks[cached.blocks.len()..] {
                chain.append_block(block.clone())?;
            }
            chain
        } else {
            build_validated_chain(config_path, &config, &blocks)?
        }
    } else {
        build_validated_chain(config_path, &config, &blocks)?
    };
    cache
        .lock()
        .map_err(|_| NodeError::Input("validated chain cache lock poisoned".to_owned()))?
        .insert(
            data_dir.to_path_buf(),
            CachedValidatedChain {
                fingerprint,
                blocks: blocks.clone(),
                chain: chain.clone(),
            },
        );
    Ok((config, blocks, chain))
}

fn summarize_chain(
    config: &NetworkConfig,
    blocks: &[Block],
    chain: &Chain,
) -> NodeResult<ChainSummary> {
    let height = chain.height().unwrap_or(0);
    let tip_hash = chain
        .tip_hash()?
        .map(|hash| hash_to_hex(&hash))
        .unwrap_or_default();
    let emitted_supply_atomic = chain.emitted_supply().as_atomic();

    Ok(ChainSummary {
        network_id: config.network.network_id().to_owned(),
        network_name: config.human_name.clone(),
        status: config.status_label.clone(),
        block_count: blocks.len(),
        height,
        tip_hash,
        emitted_supply_atomic,
    })
}

fn state_summary(
    config: &NetworkConfig,
    _blocks: &[Block],
    chain: &Chain,
) -> NodeResult<StateSummary> {
    let latest_height = chain.height().unwrap_or(0);
    let latest_block_base_fee_atomic = chain
        .blocks()
        .last()
        .map(|block| block.header.base_fee_atomic)
        .unwrap_or_default();
    let latest_block_fees_atomic = chain
        .state()
        .block_fees()
        .get(&latest_height)
        .copied()
        .unwrap_or_default()
        .as_atomic();
    let latest_block_burned_fees_atomic = chain
        .state()
        .block_burned_fees()
        .get(&latest_height)
        .copied()
        .unwrap_or_default()
        .as_atomic();
    let latest_block_priority_fees_atomic = chain
        .state()
        .block_priority_fees()
        .get(&latest_height)
        .copied()
        .unwrap_or_default()
        .as_atomic();
    let latest_coinbase_reward_atomic = chain
        .state()
        .coinbase_rewards()
        .get(&latest_height)
        .copied()
        .unwrap_or_default()
        .as_atomic();

    Ok(StateSummary {
        status: prototype_mode(config.network),
        network_name: config.human_name.clone(),
        chain_status: config.status_label.clone(),
        height: latest_height,
        tip_hash: chain
            .tip_hash()?
            .map(|hash| hash_to_hex(&hash))
            .unwrap_or_default(),
        emitted_supply_atomic: chain.emitted_supply().as_atomic(),
        tracked_addresses: chain.state().balances().len(),
        latest_block_base_fee_atomic,
        latest_block_fees_atomic,
        latest_block_burned_fees_atomic,
        latest_block_priority_fees_atomic,
        latest_coinbase_reward_atomic,
        balances: chain
            .state()
            .balances()
            .iter()
            .map(|(address, balance)| StateBalanceEntry {
                address: address.clone(),
                balance_atomic: balance.as_atomic(),
                next_nonce: chain.state().next_nonce_of(address),
            })
            .collect(),
    })
}

pub(crate) fn prototype_mode(network: Network) -> String {
    format!("{} / Prototype", network.status_label())
}

pub(crate) fn ensure_network_storage_path(network: Network, path: &Path) -> NodeResult<()> {
    let allowed_roots = [network.default_data_root(), LOCAL_OPERATOR_ROOT];
    let matches_allowed_root = path.components().any(|component| {
        allowed_roots
            .iter()
            .any(|root| component.as_os_str() == OsStr::new(root))
    });
    if matches_allowed_root {
        return Ok(());
    }

    Err(NodeError::InvalidDataPath {
        network: network.network_id().to_owned(),
        expected_root: format!("{} or {}", network.default_data_root(), LOCAL_OPERATOR_ROOT),
        actual_path: path.display().to_string(),
    })
}

#[cfg(test)]
mod storage_path_tests {
    use super::ensure_network_storage_path;
    use alvenqis_core::Network;
    use std::path::Path;

    /// Regression CR-H07: import/force wipe only under allowlisted roots.
    #[test]
    fn network_storage_allowlist_rejects_arbitrary_paths() {
        let error = ensure_network_storage_path(
            Network::Devnet,
            Path::new("C:\\Windows\\Temp\\not-alvenqis-chain"),
        )
        .expect_err("must reject non-allowlisted path");
        let message = error.to_string().to_lowercase();
        assert!(
            message.contains("path") || message.contains("root") || message.contains("invalid"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn network_storage_allowlist_accepts_default_root_component() {
        let path = Path::new("/home/user/.alvenqis-dev/chain");
        // Devnet default root is typically `.alvenqis-dev` (component match).
        // If this environment uses a different root string, still accept local operator root.
        let result = ensure_network_storage_path(Network::Devnet, path);
        if result.is_err() {
            let local = Path::new("/tmp/.alvenqis-local/chain");
            ensure_network_storage_path(Network::Devnet, local)
                .or_else(|_| {
                    ensure_network_storage_path(
                        Network::MainnetCandidate,
                        Path::new("/data/.alvenqis-mainnet/chain"),
                    )
                })
                .expect("at least one allowlisted root pattern must pass");
        }
    }
}
