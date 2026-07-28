use crate::config::NetworkConfig;
use crate::devnet::{
    deterministic_genesis_from_config, ensure_network_storage_path, genesis_approval_output_path,
    genesis_marker_path, load_genesis_approval_record, load_matching_genesis_inputs,
    resolve_config_path, resolve_genesis_recipient, summarize_validated_blocks,
    validate_genesis_approval_record, verify_existing_genesis, write_json_file, ChainSummary,
};
use crate::error::{NodeError, NodeResult};
use crate::mempool::current_unix_seconds;
use crate::storage;
use alvenqis_core::{
    blake3_hash, genesis_with_difficulty_for_network, genesis_with_timestamp_for_network,
    hash_to_hex, Block, Network,
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
