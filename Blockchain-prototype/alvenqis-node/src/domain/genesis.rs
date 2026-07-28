use crate::config::NetworkConfig;
use crate::dev_helpers::default_miner_address;
use crate::domain::chain::{ensure_network_storage_path, summarize_validated_blocks, ChainSummary};
use crate::error::{NodeError, NodeResult};
use crate::mempool::current_unix_seconds;
use crate::storage;
use alvenqis_core::{
    blake3_hash, genesis_with_difficulty_for_network, genesis_with_timestamp_for_network,
    hash_to_hex, Address, Block, Network,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_MAINNET_CANDIDATE_CONFIG_PATH: &str = "configs/mainnet-candidate.toml";
pub const DEFAULT_CONFIG_PATH: &str = DEFAULT_MAINNET_CANDIDATE_CONFIG_PATH;
pub const GENESIS_REVIEW_STANDARD_ID: &str = "alvenqis-genesis-review-v1";
pub const GENESIS_APPROVAL_STANDARD_ID: &str = "alvenqis-genesis-approval-v1";
pub(crate) const GENESIS_MARKER_FILE_NAME: &str = "genesis-info.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisConfig {
    pub network: Network,
    pub network_id: String,
    pub human_name: String,
    pub status_label: String,
    pub address_prefix: String,
    pub timestamp: u64,
    pub difficulty_leading_zero_bits: u8,
    pub recipient_strategy: String,
    #[serde(default)]
    pub recipient_address: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisMarker {
    pub network_id: String,
    pub genesis_hash: String,
    pub genesis_height: u64,
    pub status_label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisReviewManifest {
    pub review_standard_id: String,
    pub network_id: String,
    pub human_name: String,
    pub status_label: String,
    pub address_prefix: String,
    pub block_time_seconds: u64,
    pub difficulty_leading_zero_bits: u8,
    pub chain_magic_hex: String,
    pub genesis_timestamp: u64,
    pub recipient_strategy: String,
    pub recipient_address: Option<String>,
    pub resolved_recipient_address: String,
    pub deterministic_genesis_hash: String,
    pub review_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisApprovalRecord {
    pub approval_standard_id: String,
    pub review_standard_id: String,
    pub network_id: String,
    pub human_name: String,
    pub status_label: String,
    pub deterministic_genesis_hash: String,
    pub approved_review_hash: String,
    pub approved_by: String,
    #[serde(default)]
    pub approval_notes: Option<String>,
    pub approved_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct GenesisApprovalStatus {
    pub network_id: String,
    pub human_name: String,
    pub status_label: String,
    pub approval_required: bool,
    pub approval_path: Option<String>,
    pub approved: bool,
    pub deterministic_genesis_hash: String,
    pub approved_genesis_hash: Option<String>,
    pub review_hash: String,
    pub approved_review_hash: Option<String>,
    pub approved_by: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct GenesisReviewPayload {
    review_standard_id: String,
    network_id: String,
    human_name: String,
    status_label: String,
    address_prefix: String,
    block_time_seconds: u64,
    difficulty_leading_zero_bits: u8,
    chain_magic_hex: String,
    genesis_timestamp: u64,
    recipient_strategy: String,
    recipient_address: Option<String>,
    resolved_recipient_address: String,
    deterministic_genesis_hash: String,
}

pub fn default_config_path(network: Network) -> PathBuf {
    match network {
        Network::Devnet => PathBuf::from("alvenqis-devnet/config/devnet.toml"),
        Network::Testnet => PathBuf::from("alvenqis-devnet/config/testnet.toml"),
        Network::MainnetCandidate => PathBuf::from(DEFAULT_MAINNET_CANDIDATE_CONFIG_PATH),
    }
}

pub fn init_devnet(
    config_path: &Path,
    data_dir: &Path,
    miner_address: &str,
) -> NodeResult<ChainSummary> {
    let config = NetworkConfig::load_from_path(config_path)?;
    ensure_network_storage_path(config.network, data_dir)?;
    storage::ensure_data_dir(data_dir)?;

    match storage::load_blocks(data_dir) {
        Ok(existing_blocks) => {
            summarize_validated_blocks(config_path, &config, data_dir, &existing_blocks)
        }
        Err(NodeError::ChainNotInitialized(_)) => {
            let genesis = genesis_with_difficulty_for_network(
                config.network,
                miner_address,
                config.difficulty_leading_zero_bits,
            )?;
            storage::append_block(data_dir, &genesis)?;
            summarize_validated_blocks(config_path, &config, data_dir, &[genesis])
        }
        Err(error) => Err(error),
    }
}

pub fn load_genesis_config(path: &Path) -> NodeResult<GenesisConfig> {
    let content = fs::read_to_string(path)?;
    let config: GenesisConfig = toml::from_str(&content)?;
    config.validate()?;
    Ok(config)
}

pub fn genesis_hash_hex_from_config(config_path: &Path) -> NodeResult<String> {
    Ok(hash_to_hex(
        &deterministic_genesis_from_config(config_path)?.hash()?,
    ))
}

pub fn genesis_review_manifest(config_path: &Path) -> NodeResult<GenesisReviewManifest> {
    let (network_config, genesis_config) = load_matching_genesis_inputs(config_path)?;
    let recipient = resolve_genesis_recipient(&network_config, &genesis_config)?;
    let genesis = genesis_with_timestamp_for_network(
        network_config.network,
        &recipient,
        genesis_config.timestamp,
        genesis_config.difficulty_leading_zero_bits,
    )?;
    let deterministic_genesis_hash = hash_to_hex(&genesis.hash()?);
    let payload = GenesisReviewPayload {
        review_standard_id: GENESIS_REVIEW_STANDARD_ID.to_owned(),
        network_id: network_config.network_id.clone(),
        human_name: network_config.genesis_review_human_name().to_owned(),
        status_label: network_config.status_label.clone(),
        address_prefix: network_config.address_prefix.clone(),
        block_time_seconds: network_config.block_time_seconds,
        difficulty_leading_zero_bits: genesis_config.difficulty_leading_zero_bits,
        chain_magic_hex: network_config.chain_magic_hex.clone(),
        genesis_timestamp: genesis_config.timestamp,
        recipient_strategy: genesis_config.recipient_strategy.clone(),
        recipient_address: genesis_config.recipient_address.clone(),
        resolved_recipient_address: recipient,
        deterministic_genesis_hash,
    };
    let review_hash = hash_to_hex(&blake3_hash(
        serde_json::to_string(&payload)
            .map_err(NodeError::from)?
            .as_bytes(),
    ));

    Ok(GenesisReviewManifest {
        review_standard_id: payload.review_standard_id,
        network_id: payload.network_id,
        human_name: payload.human_name,
        status_label: payload.status_label,
        address_prefix: payload.address_prefix,
        block_time_seconds: payload.block_time_seconds,
        difficulty_leading_zero_bits: payload.difficulty_leading_zero_bits,
        chain_magic_hex: payload.chain_magic_hex,
        genesis_timestamp: payload.genesis_timestamp,
        recipient_strategy: payload.recipient_strategy,
        recipient_address: payload.recipient_address,
        resolved_recipient_address: payload.resolved_recipient_address,
        deterministic_genesis_hash: payload.deterministic_genesis_hash,
        review_hash,
    })
}

pub fn write_genesis_review_manifest(
    config_path: &Path,
    output_path: &Path,
) -> NodeResult<GenesisReviewManifest> {
    let manifest = genesis_review_manifest(config_path)?;
    write_json_file(output_path, &manifest)?;
    Ok(manifest)
}

pub fn approve_genesis(
    config_path: &Path,
    review_path: &Path,
    approved_by: &str,
    approval_notes: Option<&str>,
    output_path: Option<&Path>,
) -> NodeResult<GenesisApprovalStatus> {
    let approved_by = approved_by.trim();
    if approved_by.is_empty() {
        return Err(NodeError::Input("approved_by cannot be empty".to_owned()));
    }

    let manifest = genesis_review_manifest(config_path)?;
    let review_content = fs::read_to_string(review_path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            NodeError::Input(format!(
                "genesis review file is missing at {}",
                review_path.display()
            ))
        } else {
            NodeError::Io(error)
        }
    })?;
    let review_manifest: GenesisReviewManifest = serde_json::from_str(&review_content)?;
    if review_manifest != manifest {
        return Err(NodeError::ConfigMismatch(
            "genesis review file does not match the active deterministic genesis inputs".to_owned(),
        ));
    }

    let network_config = NetworkConfig::load_from_path(config_path)?;
    let approval_path = output_path
        .map(PathBuf::from)
        .unwrap_or(genesis_approval_output_path(config_path, &network_config)?);
    let record = GenesisApprovalRecord {
        approval_standard_id: GENESIS_APPROVAL_STANDARD_ID.to_owned(),
        review_standard_id: manifest.review_standard_id.clone(),
        network_id: manifest.network_id.clone(),
        human_name: manifest.human_name.clone(),
        status_label: manifest.status_label.clone(),
        deterministic_genesis_hash: manifest.deterministic_genesis_hash.clone(),
        approved_review_hash: manifest.review_hash.clone(),
        approved_by: approved_by.to_owned(),
        approval_notes: approval_notes.map(str::to_owned),
        approved_at_unix_seconds: current_unix_seconds(),
    };
    write_json_file(&approval_path, &record)?;
    genesis_approval_status(config_path)
}

pub fn genesis_approval_status(config_path: &Path) -> NodeResult<GenesisApprovalStatus> {
    let config = NetworkConfig::load_from_path(config_path)?;
    let manifest = genesis_review_manifest(config_path)?;
    let approval_required = config.network.requires_explicit_allow();
    let approval_path = config
        .genesis_approval_path
        .as_deref()
        .map(|path| resolve_config_path(config_path, path));

    if !approval_required {
        return Ok(GenesisApprovalStatus {
            network_id: manifest.network_id,
            human_name: manifest.human_name,
            status_label: manifest.status_label,
            approval_required,
            approval_path: approval_path.map(|path| path.display().to_string()),
            approved: false,
            deterministic_genesis_hash: manifest.deterministic_genesis_hash,
            approved_genesis_hash: None,
            review_hash: manifest.review_hash,
            approved_review_hash: None,
            approved_by: None,
        });
    }

    let approval_path = genesis_approval_output_path(config_path, &config)?;
    let approval = load_genesis_approval_record(&approval_path)?;
    validate_genesis_approval_record(&manifest, &approval)?;
    Ok(GenesisApprovalStatus {
        network_id: manifest.network_id,
        human_name: manifest.human_name,
        status_label: manifest.status_label,
        approval_required,
        approval_path: Some(approval_path.display().to_string()),
        approved: true,
        deterministic_genesis_hash: manifest.deterministic_genesis_hash.clone(),
        approved_genesis_hash: Some(approval.deterministic_genesis_hash),
        review_hash: manifest.review_hash.clone(),
        approved_review_hash: Some(approval.approved_review_hash),
        approved_by: Some(approval.approved_by),
    })
}

/// Export the deterministic genesis block JSON (mines once locally). Use for VPS import without re-mine.
pub fn export_genesis_block(config_path: &Path, output: &Path) -> NodeResult<Block> {
    let genesis = deterministic_genesis_from_config(config_path)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, serde_json::to_string_pretty(&genesis)?)?;
    Ok(genesis)
}

/// Import a pre-mined genesis block into an empty data_dir (no mining on server).
pub fn import_genesis_block(
    config_path: &Path,
    data_dir: &Path,
    genesis_file: &Path,
    force: bool,
) -> NodeResult<String> {
    let config = NetworkConfig::load_from_path(config_path)?;
    // Audit CR-H07: refuse destructive force wipe outside allowed network storage roots.
    ensure_network_storage_path(config.network, data_dir)?;
    let marker_path = genesis_marker_path(data_dir);
    if (marker_path.exists() || storage::chain_storage_exists(data_dir)) && !force {
        return Err(NodeError::Input(
            "genesis marker or chain database already exists; pass --force to replace chain root"
                .to_owned(),
        ));
    }
    let content = fs::read_to_string(genesis_file)?;
    let genesis: Block = serde_json::from_str(&content)?;
    if config.network.requires_explicit_allow() {
        // Candidate imports must stay fast on non-mining VPS hosts. The
        // published approval hash is authoritative, so validating the supplied
        // block against it is both stricter and cheaper than re-mining genesis.
        verify_existing_genesis(config_path, std::slice::from_ref(&genesis))?;
    } else {
        let expected = deterministic_genesis_from_config(config_path)?;
        let genesis_hash = genesis.hash()?;
        let expected_hash = expected.hash()?;
        if genesis_hash != expected_hash {
            return Err(NodeError::Input(format!(
                "imported genesis hash {} does not match config-deterministic hash {}",
                hash_to_hex(&genesis_hash),
                hash_to_hex(&expected_hash)
            )));
        }
    }
    // Wipe existing chain root when forcing — only after path allowlist check above.
    if force {
        // Prefer removing known chain artifacts; fall back to directory wipe only
        // for the validated network storage path.
        let _ = fs::remove_dir_all(data_dir);
    }
    storage::ensure_data_dir(data_dir)?;
    // Any pre-existing database or legacy JSONL requires --force above. This
    // prevents an import from silently replacing or auto-migrating chain data.
    let tip_path = data_dir.join("chain-tip.json");
    if tip_path.exists() {
        fs::remove_file(&tip_path)?;
    }
    storage::append_block(data_dir, &genesis)?;
    let marker = GenesisMarker {
        network_id: config.network_id,
        genesis_hash: hash_to_hex(&genesis.hash()?),
        genesis_height: genesis.header.height,
        status_label: config.status_label,
    };
    fs::write(marker_path, serde_json::to_string_pretty(&marker)?)?;
    Ok(marker.genesis_hash)
}

impl GenesisConfig {
    pub(crate) fn validate(&self) -> NodeResult<()> {
        if self.network_id != self.network.network_id() {
            return Err(NodeError::ConfigMismatch(format!(
                "genesis network_id must be {}",
                self.network.network_id()
            )));
        }
        if self.human_name != self.network.human_name() {
            return Err(NodeError::ConfigMismatch(format!(
                "genesis human_name must be {}",
                self.network.human_name()
            )));
        }
        if self.status_label != self.network.status_label() {
            return Err(NodeError::ConfigMismatch(format!(
                "genesis status_label must be {}",
                self.network.status_label()
            )));
        }
        if self.address_prefix != self.network.address_prefix() {
            return Err(NodeError::ConfigMismatch(format!(
                "genesis address_prefix must be {}",
                self.network.address_prefix()
            )));
        }
        if self.recipient_strategy.trim().is_empty() {
            return Err(NodeError::ConfigMismatch(
                "genesis recipient_strategy cannot be empty".to_owned(),
            ));
        }
        Ok(())
    }
}

fn load_matching_genesis_inputs(config_path: &Path) -> NodeResult<(NetworkConfig, GenesisConfig)> {
    let network_config = NetworkConfig::load_from_path(config_path)?;
    let genesis_path = resolve_config_path(config_path, &network_config.genesis_config_path);
    let genesis_config = load_genesis_config(&genesis_path)?;
    if genesis_config.network != network_config.network {
        return Err(NodeError::ConfigMismatch(format!(
            "genesis network {} does not match node network {}",
            genesis_config.network.network_id(),
            network_config.network.network_id()
        )));
    }
    if genesis_config.difficulty_leading_zero_bits != network_config.difficulty_leading_zero_bits {
        return Err(NodeError::ConfigMismatch(
            "genesis difficulty must match node difficulty".to_owned(),
        ));
    }
    Ok((network_config, genesis_config))
}

fn resolve_genesis_recipient(
    network_config: &NetworkConfig,
    genesis_config: &GenesisConfig,
) -> NodeResult<String> {
    match genesis_config.recipient_strategy.as_str() {
        "default_miner_address" => Ok(default_miner_address(network_config.network)),
        "fixed_address" => {
            let address = genesis_config.recipient_address.clone().ok_or_else(|| {
                NodeError::ConfigMismatch("recipient_address is required".to_owned())
            })?;
            let parsed =
                Address::parse(&address).map_err(|error| NodeError::Input(error.to_string()))?;
            if parsed.network() != network_config.network {
                return Err(NodeError::NetworkMismatch {
                    expected: network_config.network.network_id().to_owned(),
                    actual: parsed.network().network_id().to_owned(),
                });
            }
            Ok(address)
        }
        other => Err(NodeError::ConfigMismatch(format!(
            "unsupported genesis recipient_strategy {other}"
        ))),
    }
}

fn deterministic_genesis_from_config(config_path: &Path) -> NodeResult<Block> {
    let (network_config, genesis_config) = load_matching_genesis_inputs(config_path)?;
    let recipient = resolve_genesis_recipient(&network_config, &genesis_config)?;

    genesis_with_timestamp_for_network(
        network_config.network,
        &recipient,
        genesis_config.timestamp,
        genesis_config.difficulty_leading_zero_bits,
    )
    .map_err(NodeError::from)
}

fn genesis_approval_output_path(config_path: &Path, config: &NetworkConfig) -> NodeResult<PathBuf> {
    let configured_path = config.genesis_approval_path.as_deref().ok_or_else(|| {
        NodeError::ConfigMismatch("mainnet candidate requires genesis_approval_path".to_owned())
    })?;
    Ok(resolve_config_path(config_path, configured_path))
}

fn load_genesis_approval_record(path: &Path) -> NodeResult<GenesisApprovalRecord> {
    let content = fs::read_to_string(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            NodeError::Input(format!(
                "genesis approval file is missing at {}",
                path.display()
            ))
        } else {
            NodeError::Io(error)
        }
    })?;
    serde_json::from_str(&content).map_err(NodeError::from)
}

fn validate_pinned_genesis_approval(
    config: &NetworkConfig,
    approval: &GenesisApprovalRecord,
) -> NodeResult<()> {
    if approval.approval_standard_id != GENESIS_APPROVAL_STANDARD_ID {
        return Err(NodeError::ConfigMismatch(format!(
            "genesis approval standard must be {}",
            GENESIS_APPROVAL_STANDARD_ID
        )));
    }
    if approval.review_standard_id != GENESIS_REVIEW_STANDARD_ID {
        return Err(NodeError::ConfigMismatch(format!(
            "genesis approval review standard must be {}",
            GENESIS_REVIEW_STANDARD_ID
        )));
    }
    if approval.network_id != config.network_id {
        return Err(NodeError::NetworkMismatch {
            expected: config.network_id.clone(),
            actual: approval.network_id.clone(),
        });
    }
    if approval.human_name != config.genesis_review_human_name()
        || approval.status_label != config.status_label
    {
        return Err(NodeError::ConfigMismatch(
            "genesis approval metadata does not match the active network config".to_owned(),
        ));
    }
    let valid_hash = |value: &str| {
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    };
    if !valid_hash(&approval.deterministic_genesis_hash)
        || !valid_hash(&approval.approved_review_hash)
    {
        return Err(NodeError::ConfigMismatch(
            "genesis approval contains an invalid canonical hash".to_owned(),
        ));
    }
    if approval.approved_by.trim().is_empty() {
        return Err(NodeError::ConfigMismatch(
            "genesis approval approved_by cannot be empty".to_owned(),
        ));
    }
    Ok(())
}

/// Reads the already reviewed approval record without re-mining genesis.
/// Runtime telemetry calls this on every chain/mempool change; the explicit
/// governance command still recomputes the deterministic review manifest.
pub(crate) fn pinned_genesis_approval_status(
    config_path: &Path,
) -> NodeResult<GenesisApprovalStatus> {
    let (config, genesis_config) = load_matching_genesis_inputs(config_path)?;
    if !config.network.requires_explicit_allow() {
        return genesis_approval_status(config_path);
    }
    resolve_genesis_recipient(&config, &genesis_config)?;
    let approval_path = genesis_approval_output_path(config_path, &config)?;
    let approval = load_genesis_approval_record(&approval_path)?;
    validate_pinned_genesis_approval(&config, &approval)?;
    Ok(GenesisApprovalStatus {
        network_id: config.network_id,
        human_name: config.human_name,
        status_label: config.status_label,
        approval_required: true,
        approval_path: Some(approval_path.display().to_string()),
        approved: true,
        deterministic_genesis_hash: approval.deterministic_genesis_hash.clone(),
        approved_genesis_hash: Some(approval.deterministic_genesis_hash),
        review_hash: approval.approved_review_hash.clone(),
        approved_review_hash: Some(approval.approved_review_hash),
        approved_by: Some(approval.approved_by),
    })
}

fn validate_genesis_approval_record(
    manifest: &GenesisReviewManifest,
    approval: &GenesisApprovalRecord,
) -> NodeResult<()> {
    if approval.approval_standard_id != GENESIS_APPROVAL_STANDARD_ID {
        return Err(NodeError::ConfigMismatch(format!(
            "genesis approval standard must be {}",
            GENESIS_APPROVAL_STANDARD_ID
        )));
    }
    if approval.review_standard_id != manifest.review_standard_id {
        return Err(NodeError::ConfigMismatch(format!(
            "genesis approval review standard must be {}",
            manifest.review_standard_id
        )));
    }
    if approval.network_id != manifest.network_id {
        return Err(NodeError::NetworkMismatch {
            expected: manifest.network_id.clone(),
            actual: approval.network_id.clone(),
        });
    }
    if approval.deterministic_genesis_hash != manifest.deterministic_genesis_hash {
        return Err(NodeError::ConfigMismatch(format!(
            "approved genesis hash mismatch: expected {}, got {}",
            manifest.deterministic_genesis_hash, approval.deterministic_genesis_hash
        )));
    }
    if approval.approved_review_hash != manifest.review_hash {
        return Err(NodeError::ConfigMismatch(format!(
            "approved genesis review hash mismatch: expected {}, got {}",
            manifest.review_hash, approval.approved_review_hash
        )));
    }
    if approval.approved_by.trim().is_empty() {
        return Err(NodeError::ConfigMismatch(
            "genesis approval approved_by cannot be empty".to_owned(),
        ));
    }
    Ok(())
}

fn verified_mainnet_genesis_manifest(
    config_path: &Path,
) -> NodeResult<Option<(GenesisReviewManifest, GenesisApprovalRecord, PathBuf)>> {
    let config = NetworkConfig::load_from_path(config_path)?;
    if !config.network.requires_explicit_allow() {
        return Ok(None);
    }

    let manifest = genesis_review_manifest(config_path)?;
    let approval_path = genesis_approval_output_path(config_path, &config)?;
    let approval = load_genesis_approval_record(&approval_path)?;
    validate_genesis_approval_record(&manifest, &approval)?;
    Ok(Some((manifest, approval, approval_path)))
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> NodeResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

pub(crate) fn verify_existing_genesis(config_path: &Path, blocks: &[Block]) -> NodeResult<()> {
    let genesis = blocks
        .first()
        .ok_or_else(|| NodeError::Input("expected genesis block to exist".to_owned()))?;
    let actual_hash = genesis.hash()?;
    let actual_hex = hash_to_hex(&actual_hash);
    let (config, genesis_config) = load_matching_genesis_inputs(config_path)?;

    // Hot path: compare against GENESIS_APPROVAL only. Never call
    // genesis_review_manifest / deterministic_genesis_from_config here — those re-mine
    // at difficulty 16 and freeze public RPC (/mining/template → 504).
    if config.network.requires_explicit_allow() {
        let approval_path = genesis_approval_output_path(config_path, &config)?;
        let approval = load_genesis_approval_record(&approval_path)?;
        validate_pinned_genesis_approval(&config, &approval)?;
        let recipient = resolve_genesis_recipient(&config, &genesis_config)?;
        let coinbase_matches = genesis
            .transactions
            .first()
            .is_some_and(|transaction| transaction.is_coinbase() && transaction.to == recipient);
        if genesis.header.height != 0
            || genesis.header.network_id != config.network_id
            || genesis.header.timestamp != genesis_config.timestamp
            || genesis.header.difficulty_leading_zero_bits
                != genesis_config.difficulty_leading_zero_bits
            || !coinbase_matches
        {
            return Err(NodeError::ConfigMismatch(
                "stored genesis fields do not match the active pinned inputs".to_owned(),
            ));
        }
        let expected_hex = approval
            .deterministic_genesis_hash
            .trim()
            .to_ascii_lowercase();
        if actual_hex != expected_hex {
            return Err(NodeError::ConfigMismatch(format!(
                "genesis hash mismatch vs approval: expected {expected_hex}, got {actual_hex}"
            )));
        }
        return Ok(());
    }

    // Dev/test networks only: rebuild is cheap at low difficulty.
    let expected_hash = deterministic_genesis_from_config(config_path)?.hash()?;
    if actual_hash != expected_hash {
        return Err(NodeError::ConfigMismatch(format!(
            "genesis hash mismatch: expected {}, got {actual_hex}",
            hash_to_hex(&expected_hash),
        )));
    }
    Ok(())
}

pub(crate) fn initialize_deterministic_genesis(
    config_path: &Path,
    data_dir: &Path,
    force_genesis: bool,
) -> NodeResult<()> {
    let config = NetworkConfig::load_from_path(config_path)?;
    let marker_path = genesis_marker_path(data_dir);
    if !config.network.is_resettable() && marker_path.exists() && !force_genesis {
        return Err(NodeError::Input(
            "genesis marker already exists; pass --force-genesis to recreate the chain root"
                .to_owned(),
        ));
    }

    if config.network.requires_explicit_allow() {
        verified_mainnet_genesis_manifest(config_path)?;
    }
    storage::ensure_data_dir(data_dir)?;
    let genesis = deterministic_genesis_from_config(config_path)?;
    let marker = GenesisMarker {
        network_id: config.network_id,
        genesis_hash: hash_to_hex(&genesis.hash()?),
        genesis_height: genesis.header.height,
        status_label: config.status_label,
    };
    storage::append_block(data_dir, &genesis)?;
    fs::write(marker_path, serde_json::to_string_pretty(&marker)?)?;
    Ok(())
}

pub(crate) fn load_genesis_marker(data_dir: &Path) -> NodeResult<GenesisMarker> {
    let content = fs::read_to_string(genesis_marker_path(data_dir))?;
    serde_json::from_str(&content).map_err(NodeError::from)
}

fn genesis_marker_path(data_dir: &Path) -> PathBuf {
    data_dir
        .parent()
        .unwrap_or(data_dir)
        .join(GENESIS_MARKER_FILE_NAME)
}

fn resolve_config_path(config_path: &Path, configured_path: &str) -> PathBuf {
    let candidate = PathBuf::from(configured_path);
    if candidate.is_absolute() || candidate.exists() {
        return candidate;
    }

    if let Some(parent) = config_path.parent() {
        let joined = parent.join(&candidate);
        if joined.exists() {
            return joined;
        }

        if let Some(grandparent) = parent.parent() {
            let grandparent_joined = grandparent.join(&candidate);
            if grandparent_joined.exists() {
                return grandparent_joined;
            }
            return grandparent_joined;
        }

        return joined;
    }

    candidate
}
