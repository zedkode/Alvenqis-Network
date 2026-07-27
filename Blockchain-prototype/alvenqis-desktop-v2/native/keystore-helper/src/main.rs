use alvenqis_core::{
    generate_mnemonic, Address, Amount, MnemonicWordCount, Network, PrivateKey, Transaction,
    WalletDerivationPath,
};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};
use thiserror::Error;
use zeroize::Zeroize;

/// Current OS credential-manager service name (Alvenqis brand).
const SERVICE: &str = "Alvenqis Desktop";
/// Immediate predecessor brand service (Vireon). Must stay DIFFERENT from SERVICE so
/// load_private_key can find and re-home credentials after the rebrand.
const LEGACY_SERVICE: &str = "Vireon Desktop";
/// Older brand service still present on some machines (Veiron → Vireon → Alvenqis).
const LEGACY_SERVICE_VEIRON: &str = "Veiron Desktop";
const LEGACY_ACCOUNT: &str = "mainnet-candidate-default-wallet";
/// Current on-disk wallet metadata schema.
const METADATA_SCHEMA: &str = "alvenqis-desktop-wallet-metadata-v2";
/// Predecessor schema (Vireon). Must stay DIFFERENT from METADATA_SCHEMA so
/// load_wallets() upgrade-in-place actually detects old files.
const LEGACY_METADATA_SCHEMA: &str = "vireon-desktop-wallet-metadata-v2";
/// Oldest known desktop metadata schema (Veiron).
const LEGACY_METADATA_SCHEMA_VEIRON: &str = "veiron-desktop-wallet-metadata-v2";
/// Prior brand folder names under the OS data dir (migration sources only).
const LEGACY_BRAND_FOLDERS: &[&str] = &["Vireon", "Veiron"];

fn rpc_url() -> String {
    std::env::var("ALVENQIS_RPC_URL")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| alvenqis_sdk_rust::DEFAULT_MAINNET_CANDIDATE_RPC.to_owned())
}

fn rpc_client() -> Result<alvenqis_sdk_rust::BlockingRpcClient> {
    alvenqis_sdk_rust::BlockingRpcClient::new(alvenqis_sdk_rust::NetworkConfig::with_rpc(
        alvenqis_sdk_rust::Network::MainnetCandidate,
        rpc_url(),
    ))
    .map_err(|error| HelperError::Service(error.to_string()))
}

#[derive(Debug, Error)]
enum HelperError {
    #[error("invalid wallet input: {0}")]
    Input(String),
    #[error("secure credential storage failed: {0}")]
    Credential(String),
    #[error("local wallet metadata failed: {0}")]
    Metadata(String),
    #[error("local service failed: {0}")]
    Service(String),
    #[error("chain state changed; refresh the signing preview")]
    StalePreview,
}

type Result<T> = std::result::Result<T, HelperError>;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WalletMetadata {
    wallet_id: String,
    display_name: String,
    schema: String,
    network_id: String,
    address: String,
    public_key_hex: String,
    key_origin: String,
    derivation_path: String,
    credential_account: String,
}

#[derive(Deserialize)]
struct Request {
    command: String,
    /// Must match ALVENQIS_KEYSTORE_PARENT_TOKEN from the Control Center parent process.
    #[serde(default)]
    parent_token: Option<String>,
    #[serde(default)]
    wallet_id: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
    /// Optional recovery phrase for in-app import (parent-token protected only).
    #[serde(default)]
    recovery_phrase: Option<String>,
    #[serde(default)]
    workspace: Option<PathBuf>,
    #[serde(default)]
    recipient: Option<String>,
    #[serde(default)]
    amount: Option<String>,
    #[serde(default)]
    tip: Option<String>,
    #[serde(default)]
    prepared: Option<PreparedTransaction>,
}

#[derive(Serialize)]
struct CreateResult {
    metadata: WalletMetadata,
    /// Always false until the UI acknowledges backup (phrase is returned once).
    recovery_confirmed: bool,
    /// One-time recovery words for in-app reveal / copy (never logged).
    recovery_phrase: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PreparedTransaction {
    recipient: String,
    amount_atomic: String,
    tip_atomic: String,
    base_fee_atomic: String,
    total_atomic: String,
    available_atomic: String,
    nonce: u64,
    chain_tip: String,
}

#[derive(Serialize)]
struct SubmissionResult {
    tx_hash: String,
    lifecycle_status: String,
    mempool_size: usize,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--purge-uninstall")) {
        purge_for_uninstall()?;
        output(&serde_json::json!({ "purged": true }))?;
        return Ok(());
    }
    let mut bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut bytes)
        .map_err(metadata_error)?;
    let request: Request = serde_json::from_slice(&bytes).map_err(metadata_error)?;
    bytes.zeroize();
    verify_parent_token(request.parent_token.as_deref())?;
    match request.command.as_str() {
        "metadata" => output(&load_metadata()?),
        "list" => output(&load_wallets()?),
        "select" => output(&select_wallet(&required(request.wallet_id, "wallet_id")?)?),
        "create" => output(&create_wallet(validated_display_name(
            request.display_name,
        )?)?),
        "import" => output(&import_wallet_native(validated_display_name(
            request.display_name,
        )?)?),
        // In-app paste import (Control Center only; parent token required).
        "import_phrase" => {
            let display_name = validated_display_name(request.display_name)?;
            let phrase = request
                .recovery_phrase
                .ok_or_else(|| HelperError::Input("recovery_phrase is required".into()))?;
            output(&import_wallet_phrase(display_name, phrase)?)
        }
        "remove" => {
            remove_wallet()?;
            output(&serde_json::Value::Null)
        }
        "prepare" => output(&prepare_transaction(
            required_path(request.workspace, "workspace")?,
            required(request.recipient, "recipient")?,
            required(request.amount, "amount")?,
            required(request.tip, "tip")?,
        )?),
        "sign_submit" => output(&sign_and_submit(
            required_path(request.workspace, "workspace")?,
            request
                .prepared
                .ok_or_else(|| HelperError::Input("prepared transaction is required".into()))?,
        )?),
        _ => Err(HelperError::Input("unsupported helper command".into())),
    }
}

fn verify_parent_token(provided: Option<&str>) -> Result<()> {
    // Dev escape hatch for manual smoke tests only.
    if std::env::var_os("ALVENQIS_KEYSTORE_ALLOW_UNAUTHENTICATED")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        return Ok(());
    }
    let expected = std::env::var("ALVENQIS_KEYSTORE_PARENT_TOKEN").map_err(|_| {
        HelperError::Input(
            "keystore helper requires ALVENQIS_KEYSTORE_PARENT_TOKEN from Control Center".into(),
        )
    })?;
    let provided = provided.unwrap_or("");
    if provided.is_empty() || provided != expected {
        return Err(HelperError::Input(
            "invalid keystore parent token (spawn only via Control Center)".into(),
        ));
    }
    Ok(())
}

fn output(value: &impl Serialize) -> Result<()> {
    serde_json::to_writer(std::io::stdout(), value).map_err(metadata_error)
}

fn create_wallet(display_name: String) -> Result<CreateResult> {
    let mnemonic = generate_mnemonic(MnemonicWordCount::TwentyFour)
        .map_err(|error| HelperError::Input(error.to_string()))?;
    let key = PrivateKey::from_mnemonic(&mnemonic, "", WalletDerivationPath::default())
        .map_err(|error| HelperError::Input(error.to_string()))?;
    let metadata = persist_key(&key, "bip39-created", display_name)?;
    // Recovery phrase is revealed in the Control Center UI (hide/reveal + copy).
    // No Windows MessageBox / native prompt.
    Ok(CreateResult {
        metadata,
        recovery_confirmed: false,
        recovery_phrase: mnemonic,
    })
}

fn import_wallet_native(display_name: String) -> Result<WalletMetadata> {
    let mut phrase = prompt_recovery_phrase()?;
    let result = require_twenty_four_words(&phrase).and_then(|()| {
        PrivateKey::from_mnemonic(&phrase, "", WalletDerivationPath::default())
            .map_err(|_| HelperError::Input("recovery phrase is invalid".into()))
            .and_then(|key| persist_key(&key, "bip39-imported", display_name))
    });
    phrase.zeroize();
    result
}

fn import_wallet_phrase(display_name: String, mut phrase: String) -> Result<WalletMetadata> {
    let result = require_twenty_four_words(&phrase).and_then(|()| {
        PrivateKey::from_mnemonic(&phrase, "", WalletDerivationPath::default())
            .map_err(|_| HelperError::Input("recovery phrase is invalid".into()))
            .and_then(|key| persist_key(&key, "bip39-imported", display_name))
    });
    phrase.zeroize();
    result
}

fn persist_key(key: &PrivateKey, origin: &str, display_name: String) -> Result<WalletMetadata> {
    let public_key = key.public_key();
    let address = Address::from_public_key_for_network(&public_key, Network::MainnetCandidate);
    let wallet_id = address.to_string();
    let credential_account = format!("mainnet-candidate-wallet-{wallet_id}");
    let metadata = WalletMetadata {
        wallet_id,
        display_name,
        schema: METADATA_SCHEMA.into(),
        network_id: "alvenqis-mainnet-candidate".into(),
        address: address.to_string(),
        public_key_hex: public_key.to_hex(),
        key_origin: origin.into(),
        derivation_path: "m/44'/7330'/0'/0'/0'".into(),
        credential_account,
    };
    let mut secret = key.to_hex();
    let stored = credential(&metadata.credential_account)?
        .set_password(&secret)
        .map_err(credential_error);
    secret.zeroize();
    stored?;
    if let Err(error) = save_wallet(&metadata) {
        remove_private_key(&metadata.credential_account);
        return Err(error);
    }
    Ok(metadata)
}

fn prepare_transaction(
    _workspace: PathBuf,
    recipient: String,
    amount: String,
    tip: String,
) -> Result<PreparedTransaction> {
    let wallet = load_metadata()?
        .ok_or_else(|| HelperError::Input("create or import a wallet first".into()))?;
    let recipient = Address::parse(&recipient).map_err(|_| {
        HelperError::Input("recipient must be a valid Mainnet Candidate address".into())
    })?;
    if recipient.network() != Network::MainnetCandidate {
        return Err(HelperError::Input(
            "recipient is not a Mainnet Candidate address".into(),
        ));
    }
    let amount = Amount::parse_alve(&amount)
        .map_err(|_| HelperError::Input("amount must be a valid ALVE value".into()))?;
    let tip = Amount::parse_alve(&tip)
        .map_err(|_| HelperError::Input("tip must be a valid ALVE value".into()))?;
    if amount == Amount::ZERO {
        return Err(HelperError::Input(
            "amount must be greater than zero".into(),
        ));
    }

    // VPS-first: never require a local chain copy. Sign against the configured RPC tip.
    let account = fetch_remote_account(&wallet.address)?;
    let base_fee = Amount::from_atomic(account.anticipated_base_fee_atomic);
    let available = Amount::from_atomic(account.balance_atomic);
    let total = amount
        .checked_add(base_fee)
        .and_then(|value| value.checked_add(tip))
        .map_err(service_error)?;
    if available < total {
        return Err(HelperError::Input(format!(
            "insufficient balance: available {}, required {} atomic units",
            available.as_atomic(),
            total.as_atomic()
        )));
    }
    let chain_tip = account
        .tip_hash
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            HelperError::Service("RPC gateway has no tip hash; chain is not ready".into())
        })?;
    Ok(PreparedTransaction {
        recipient: recipient.to_string(),
        amount_atomic: amount.as_atomic().to_string(),
        tip_atomic: tip.as_atomic().to_string(),
        base_fee_atomic: base_fee.as_atomic().to_string(),
        total_atomic: total.as_atomic().to_string(),
        available_atomic: available.as_atomic().to_string(),
        nonce: account.next_nonce,
        chain_tip,
    })
}

#[derive(Debug, Deserialize)]
struct RemoteAccount {
    balance_atomic: u64,
    next_nonce: u64,
    tip_hash: Option<String>,
    anticipated_base_fee_atomic: u64,
}

fn fetch_remote_account(address: &str) -> Result<RemoteAccount> {
    let client = rpc_client()?;
    match client.account(address) {
        Ok(account) => Ok(RemoteAccount {
            balance_atomic: account.balance_atomic,
            next_nonce: account.next_nonce,
            tip_hash: account.tip_hash,
            anticipated_base_fee_atomic: account.anticipated_base_fee_atomic,
        }),
        Err(alvenqis_sdk_rust::SdkError::RpcHttp { status: 404, .. }) => {
            // Older gateways without /account - refuse invented nonces.
            fetch_remote_account_compat(address)
        }
        Err(error) => Err(HelperError::Service(format!(
            "cannot load account from {}: {error}",
            rpc_url()
        ))),
    }
}

fn fetch_remote_account_compat(address: &str) -> Result<RemoteAccount> {
    // Fail closed (audit A-M04): never invent nonce=1 for spenders on old gateways.
    let _ = address;
    Err(HelperError::Service(
        "RPC gateway is missing GET /addresses/{addr}/account. Upgrade the VPS gateway - refusing to invent next_nonce.".into(),
    ))
}

fn sign_and_submit(workspace: PathBuf, prepared: PreparedTransaction) -> Result<SubmissionResult> {
    let wallet = load_metadata()?
        .ok_or_else(|| HelperError::Input("create or import a wallet first".into()))?;
    let refreshed = prepare_transaction(
        workspace.clone(),
        prepared.recipient.clone(),
        format_atomic(parse_atomic(&prepared.amount_atomic)?),
        format_atomic(parse_atomic(&prepared.tip_atomic)?),
    )?;
    if refreshed.nonce != prepared.nonce
        || refreshed.base_fee_atomic != prepared.base_fee_atomic
        || refreshed.chain_tip != prepared.chain_tip
    {
        return Err(HelperError::StalePreview);
    }
    let mut secret = load_private_key(&wallet.credential_account)?;
    let result = PrivateKey::from_hex(&secret)
        .map_err(|_| HelperError::Credential("stored key is invalid".into()))
        .and_then(|key| {
            let address =
                Address::from_public_key_for_network(&key.public_key(), Network::MainnetCandidate);
            if address.to_string() != wallet.address {
                return Err(HelperError::Credential(
                    "stored key does not match public wallet metadata".into(),
                ));
            }
            let tip = Amount::from_atomic(parse_atomic(&prepared.tip_atomic)?);
            let base = Amount::from_atomic(parse_atomic(&prepared.base_fee_atomic)?);
            let max_fee = base.checked_add(tip).map_err(service_error)?;
            let transaction = Transaction::new_signed(
                1,
                prepared.nonce,
                Network::MainnetCandidate,
                &key,
                prepared.recipient,
                Amount::from_atomic(parse_atomic(&prepared.amount_atomic)?),
                max_fee,
                tip,
                None,
            )
            .map_err(service_error)?;
            let response = alvenqis_wallet::rpc::submit_transaction(&rpc_url(), &transaction)
                .map_err(service_error)?;
            Ok(SubmissionResult {
                tx_hash: response.tx_hash,
                lifecycle_status: response.lifecycle_status,
                mempool_size: response.mempool_size,
            })
        });
    secret.zeroize();
    result
}

fn wallet_root() -> Result<PathBuf> {
    let root = dirs::data_local_dir().ok_or_else(|| {
        HelperError::Metadata("local application data directory is unavailable".into())
    })?;
    let current = root.join("Alvenqis").join("Desktop");
    // Copy missing files from each prior-brand Desktop profile into Alvenqis/Desktop.
    // Self-copy is rejected inside copy_missing_tree (guard against rebrand regressions).
    for brand in LEGACY_BRAND_FOLDERS {
        let legacy = root.join(brand).join("Desktop");
        if legacy.exists() {
            copy_missing_tree(&legacy, &current)?;
        }
    }
    Ok(current)
}

fn legacy_metadata_path() -> Result<PathBuf> {
    Ok(wallet_root()?.join("wallet.json"))
}

fn wallets_dir() -> Result<PathBuf> {
    Ok(wallet_root()?.join("wallets"))
}

fn active_wallet_path() -> Result<PathBuf> {
    Ok(wallet_root()?.join("active-wallet"))
}

fn migrate_legacy_wallet() -> Result<()> {
    let legacy_path = legacy_metadata_path()?;
    if !legacy_path.exists() || wallets_dir()?.exists() {
        return Ok(());
    }
    #[derive(Deserialize)]
    struct LegacyMetadata {
        network_id: String,
        address: String,
        public_key_hex: String,
        key_origin: String,
        derivation_path: String,
    }
    let legacy: LegacyMetadata =
        serde_json::from_slice(&fs::read(&legacy_path).map_err(metadata_error)?)
            .map_err(metadata_error)?;
    // Accept prior mainnet-candidate wire IDs written under older brands; rewrite to Alvenqis.
    let network_id = match legacy.network_id.as_str() {
        "alvenqis-mainnet-candidate" | "vireon-mainnet-candidate" | "veiron-mainnet-candidate" => {
            "alvenqis-mainnet-candidate".to_owned()
        }
        other => {
            return Err(HelperError::Metadata(format!(
                "legacy wallet belongs to another network ({other})"
            )));
        }
    };
    let metadata = WalletMetadata {
        wallet_id: legacy.address.clone(),
        display_name: "Primary wallet".into(),
        schema: METADATA_SCHEMA.into(),
        network_id,
        address: legacy.address,
        public_key_hex: legacy.public_key_hex,
        key_origin: legacy.key_origin,
        derivation_path: legacy.derivation_path,
        credential_account: LEGACY_ACCOUNT.into(),
    };
    save_wallet(&metadata)?;
    // Keep the legacy metadata in place as a rollback source.
    Ok(())
}

fn load_wallets() -> Result<Vec<WalletMetadata>> {
    migrate_legacy_wallet()?;
    let directory = wallets_dir()?;
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut wallets = Vec::new();
    for entry in fs::read_dir(directory).map_err(metadata_error)? {
        let path = entry.map_err(metadata_error)?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        if !recover_completed_atomic_write(&path)? {
            continue;
        }
        let mut metadata: WalletMetadata =
            serde_json::from_slice(&fs::read(path).map_err(metadata_error)?)
                .map_err(metadata_error)?;
        let mut upgraded = false;
        if is_legacy_metadata_schema(&metadata.schema) {
            metadata.schema = METADATA_SCHEMA.into();
            upgraded = true;
        }
        if is_legacy_mainnet_candidate_network_id(&metadata.network_id) {
            metadata.network_id = "alvenqis-mainnet-candidate".into();
            upgraded = true;
        }
        if upgraded {
            save_wallet(&metadata)?;
        }
        validate_wallet_metadata(&metadata)?;
        wallets.push(metadata);
    }
    wallets.sort_by(|left, right| {
        left.display_name
            .cmp(&right.display_name)
            .then(left.wallet_id.cmp(&right.wallet_id))
    });
    Ok(wallets)
}

fn load_metadata() -> Result<Option<WalletMetadata>> {
    let wallets = load_wallets()?;
    if wallets.is_empty() {
        return Ok(None);
    }
    let active = fs::read_to_string(active_wallet_path()?).unwrap_or_default();
    Ok(wallets
        .iter()
        .find(|wallet| wallet.wallet_id == active.trim())
        .cloned()
        .or_else(|| wallets.first().cloned()))
}

fn select_wallet(wallet_id: &str) -> Result<WalletMetadata> {
    let wallet = load_wallets()?
        .into_iter()
        .find(|wallet| wallet.wallet_id == wallet_id)
        .ok_or_else(|| HelperError::Input("selected wallet does not exist".into()))?;
    write_atomic(&active_wallet_path()?, wallet.wallet_id.as_bytes())?;
    Ok(wallet)
}

fn save_wallet(metadata: &WalletMetadata) -> Result<()> {
    validate_wallet_metadata(metadata)?;
    let directory = wallets_dir()?;
    fs::create_dir_all(&directory).map_err(metadata_error)?;
    let path = directory.join(format!("{}.json", metadata.wallet_id));
    write_atomic(
        &path,
        &serde_json::to_vec_pretty(metadata).map_err(metadata_error)?,
    )?;
    write_atomic(&active_wallet_path()?, metadata.wallet_id.as_bytes())
}

fn is_legacy_metadata_schema(schema: &str) -> bool {
    schema == LEGACY_METADATA_SCHEMA || schema == LEGACY_METADATA_SCHEMA_VEIRON
}

fn is_supported_metadata_schema(schema: &str) -> bool {
    schema == METADATA_SCHEMA || is_legacy_metadata_schema(schema)
}

fn is_legacy_mainnet_candidate_network_id(network_id: &str) -> bool {
    matches!(
        network_id,
        "vireon-mainnet-candidate" | "veiron-mainnet-candidate"
    )
}

fn validate_wallet_metadata(metadata: &WalletMetadata) -> Result<()> {
    if !is_supported_metadata_schema(&metadata.schema)
        || metadata.network_id != "alvenqis-mainnet-candidate"
        || metadata.wallet_id != metadata.address
        || metadata.credential_account.trim().is_empty()
    {
        return Err(HelperError::Metadata(
            "unsupported wallet metadata or network".into(),
        ));
    }
    Ok(())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| HelperError::Metadata("invalid wallet metadata path".into()))?;
    fs::create_dir_all(parent).map_err(metadata_error)?;
    let _ = quarantine_directory_at_file_path(path)?;
    let temporary = path.with_extension("tmp");
    let _ = quarantine_directory_at_file_path(&temporary)?;
    fs::write(&temporary, bytes).map_err(metadata_error)?;
    if path.exists() {
        fs::remove_file(path).map_err(metadata_error)?;
    }
    fs::rename(temporary, path).map_err(metadata_error)
}

/// Older rebrand builds could leave a directory where a wallet metadata file belongs.
/// Preserve that unexpected directory as a sibling instead of deleting user data, then
/// allow the normal atomic write to recreate the required file.
fn quarantine_directory_at_file_path(path: &Path) -> Result<bool> {
    if !path.is_dir() {
        return Ok(false);
    }

    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| HelperError::Metadata("invalid wallet metadata file name".into()))?;
    let parent = path
        .parent()
        .ok_or_else(|| HelperError::Metadata("invalid wallet metadata path".into()))?;
    for suffix in 0..=1_000 {
        let backup_name = if suffix == 0 {
            format!("{file_name}.invalid-directory")
        } else {
            format!("{file_name}.invalid-directory-{suffix}")
        };
        let backup = parent.join(backup_name);
        if !backup.exists() {
            fs::rename(path, backup).map_err(metadata_error)?;
            return Ok(true);
        }
    }

    Err(HelperError::Metadata(
        "cannot preserve unexpected wallet metadata directory".into(),
    ))
}

/// If an interrupted old write left `path` as a directory and `path.tmp` as the
/// completed file, preserve the directory and promote the temporary file.
/// Returns false only when the collision had no recoverable temporary file.
fn recover_completed_atomic_write(path: &Path) -> Result<bool> {
    if !path.is_dir() {
        return Ok(true);
    }
    let _ = quarantine_directory_at_file_path(path)?;
    let temporary = path.with_extension("tmp");
    if !temporary.is_file() {
        return Ok(false);
    }
    fs::rename(temporary, path).map_err(metadata_error)?;
    Ok(true)
}

fn remove_wallet() -> Result<()> {
    let Some(wallet) = load_metadata()? else {
        return Ok(());
    };
    remove_private_key(&wallet.credential_account);
    let path = wallets_dir()?.join(format!("{}.json", wallet.wallet_id));
    if path.exists() {
        fs::remove_file(path).map_err(metadata_error)?;
    }
    let remaining = load_wallets()?;
    if let Some(next) = remaining.first() {
        write_atomic(&active_wallet_path()?, next.wallet_id.as_bytes())?;
    } else if active_wallet_path()?.exists() {
        fs::remove_file(active_wallet_path()?).map_err(metadata_error)?;
    }
    Ok(())
}

fn purge_for_uninstall() -> Result<()> {
    let mut accounts = load_wallets()
        .unwrap_or_default()
        .into_iter()
        .map(|wallet| wallet.credential_account)
        .collect::<Vec<_>>();
    accounts.push(LEGACY_ACCOUNT.to_owned());
    accounts.sort();
    accounts.dedup();

    for account in accounts {
        remove_credential_from_service(SERVICE, &account);
        for service in legacy_credential_services() {
            remove_credential_from_service(service, &account);
        }
    }

    let root = wallet_root()?;
    if root.exists() {
        fs::remove_dir_all(&root).map_err(metadata_error)?;
    }
    Ok(())
}

fn credential(account: &str) -> Result<Entry> {
    Entry::new(SERVICE, account).map_err(credential_error)
}

fn legacy_credential_services() -> &'static [&'static str] {
    &[LEGACY_SERVICE, LEGACY_SERVICE_VEIRON]
}

fn load_private_key(account: &str) -> Result<String> {
    match credential(account)?.get_password() {
        Ok(secret) => Ok(secret),
        Err(current_error) => {
            // Try each prior-brand credential service, then re-home under Alvenqis.
            let mut last_error = credential_error(current_error);
            for service in legacy_credential_services() {
                match Entry::new(service, account) {
                    Ok(entry) => match entry.get_password() {
                        Ok(secret) => {
                            credential(account)?
                                .set_password(&secret)
                                .map_err(credential_error)?;
                            return Ok(secret);
                        }
                        Err(error) => last_error = credential_error(error),
                    },
                    Err(error) => last_error = credential_error(error),
                }
            }
            Err(last_error)
        }
    }
}
fn remove_private_key(account: &str) {
    remove_credential_from_service(SERVICE, account);
}

fn remove_credential_from_service(service: &str, account: &str) {
    if let Ok(entry) = Entry::new(service, account) {
        let _ = entry.delete_credential();
    }
}

/// Returns true when `source` and `destination` resolve to the same filesystem path.
/// Prefer canonicalize when both sides exist; fall back to path equality.
fn same_path(source: &Path, destination: &Path) -> bool {
    if source == destination {
        return true;
    }
    match (source.canonicalize(), destination.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Copy files from `source` into `destination` without overwriting existing files.
/// No-op (Ok) when source and destination are the same path — never self-copy.
fn copy_missing_tree(source: &Path, destination: &Path) -> Result<()> {
    if same_path(source, destination) {
        return Ok(());
    }
    if source.is_file() {
        if !destination.exists() {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(metadata_error)?;
            }
            fs::copy(source, destination).map_err(metadata_error)?;
        }
        return Ok(());
    }
    if !source.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(destination).map_err(metadata_error)?;
    for entry in fs::read_dir(source).map_err(metadata_error)? {
        let entry = entry.map_err(metadata_error)?;
        copy_missing_tree(&entry.path(), &destination.join(entry.file_name()))?;
    }
    Ok(())
}

#[cfg(test)]
mod rebrand_migration_tests {
    use super::*;

    #[test]
    fn legacy_service_and_schema_differ_from_current() {
        assert_ne!(
            SERVICE, LEGACY_SERVICE,
            "LEGACY_SERVICE must remain the prior-brand OS credential service"
        );
        assert_ne!(
            SERVICE, LEGACY_SERVICE_VEIRON,
            "LEGACY_SERVICE_VEIRON must remain the oldest brand OS credential service"
        );
        assert_ne!(
            LEGACY_SERVICE, LEGACY_SERVICE_VEIRON,
            "predecessor brand services must stay distinct from each other"
        );
        assert_ne!(
            METADATA_SCHEMA, LEGACY_METADATA_SCHEMA,
            "LEGACY_METADATA_SCHEMA must remain the Vireon on-disk schema"
        );
        assert_ne!(
            METADATA_SCHEMA, LEGACY_METADATA_SCHEMA_VEIRON,
            "LEGACY_METADATA_SCHEMA_VEIRON must remain the Veiron on-disk schema"
        );
        assert_ne!(
            LEGACY_METADATA_SCHEMA, LEGACY_METADATA_SCHEMA_VEIRON,
            "prior schemas must stay distinct from each other"
        );
    }

    #[test]
    fn copy_missing_tree_is_noop_when_source_equals_destination() {
        let dir = tempfile_dir("same-path");
        let nested = dir.join("nested");
        fs::create_dir_all(&nested).expect("mkdir");
        let file = nested.join("wallet.json");
        fs::write(&file, b"keep-me").expect("write");

        // Direct equality (no canonicalize needed).
        copy_missing_tree(&dir, &dir).expect("self-copy by Path equality must be Ok");
        // Canonicalize equality (same directory after resolution).
        let a = dir.canonicalize().expect("canon a");
        let b = dir.canonicalize().expect("canon b");
        copy_missing_tree(&a, &b).expect("self-copy by canonicalize must be Ok");
        assert_eq!(
            fs::read_to_string(&file).expect("read"),
            "keep-me",
            "self-copy must not destroy existing files"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// Linux uses dirs::data_local_dir() → $XDG_DATA_HOME or ~/.local/share.
    /// The rebrand bug set legacy==current under that root; prove self-copy is a no-op
    /// with XDG-style path components (not Windows LocalAppData).
    #[test]
    fn linux_xdg_share_self_copy_alvenqis_desktop_is_noop() {
        let home = tempfile_dir("xdg-home");
        // Mimic Linux: $HOME/.local/share/Alvenqis/Desktop (XDG_DATA_HOME default).
        let xdg_data = home.join(".local").join("share");
        let current = xdg_data.join("Alvenqis").join("Desktop");
        let wallets = current.join("wallets");
        fs::create_dir_all(&wallets).expect("mkdir wallets");
        let marker = wallets.join("active.json");
        fs::write(&marker, b"linux-wallet").expect("write");

        // Exact rebrand regression: legacy and current both Alvenqis/Desktop under XDG.
        let legacy_broken = xdg_data.join("Alvenqis").join("Desktop");
        assert_eq!(legacy_broken, current);
        copy_missing_tree(&legacy_broken, &current)
            .expect("XDG self-copy must be Ok, not Access Denied");
        assert_eq!(
            fs::read_to_string(&marker).expect("read"),
            "linux-wallet",
            "self-copy must not corrupt wallet metadata under ~/.local/share"
        );

        // same_path must also treat canonicalize equals as identity on this layout.
        let a = current.canonicalize().expect("canon a");
        let b = legacy_broken.canonicalize().expect("canon b");
        assert!(same_path(&a, &b));
        copy_missing_tree(&a, &b).expect("canonical XDG self-copy");
        let _ = fs::remove_dir_all(&home);
    }

    /// AREA 7a: mock HOME + XDG_DATA_HOME so dirs::data_local_dir() resolves under a
    /// temp Linux layout, then exercise wallet_root() (the real migration entrypoint).
    /// Only meaningful on Linux where dirs honors XDG_* (Windows uses LocalAppData).
    #[test]
    #[cfg(target_os = "linux")]
    fn wallet_root_mocked_xdg_data_home_self_referential_is_noop() {
        use std::sync::Mutex;
        static ENV_LOCK: Mutex<()> = Mutex::new(());
        let _guard = ENV_LOCK.lock().expect("env lock");

        let home = tempfile_dir("xdg-env-home");
        let xdg_data = home.join("share");
        let desktop = xdg_data.join("Alvenqis").join("Desktop");
        let wallets = desktop.join("wallets");
        fs::create_dir_all(&wallets).expect("mkdir");
        let marker = wallets.join("marker.json");
        fs::write(&marker, b"xdg-mock-wallet").expect("write");

        let prev_home = std::env::var_os("HOME");
        let prev_xdg = std::env::var_os("XDG_DATA_HOME");
        // Safety: if HOME were used without XDG_DATA_HOME, still stay under temp.
        std::env::set_var("HOME", &home);
        std::env::set_var("XDG_DATA_HOME", &xdg_data);

        let resolved = wallet_root().expect("wallet_root under mocked XDG");
        assert_eq!(
            resolved.canonicalize().unwrap(),
            desktop.canonicalize().unwrap(),
            "wallet_root must resolve to $XDG_DATA_HOME/Alvenqis/Desktop"
        );
        // Migration loop only walks Vireon/Veiron; Alvenqis is never a source.
        // Calling wallet_root twice must remain a no-op (no Access Denied / corruption).
        let _ = wallet_root().expect("second wallet_root");
        assert_eq!(
            fs::read_to_string(&marker).expect("read marker"),
            "xdg-mock-wallet"
        );

        // Restore env for other tests.
        match prev_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match prev_xdg {
            Some(v) => std::env::set_var("XDG_DATA_HOME", v),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn linux_xdg_share_migrates_vireon_desktop_into_alvenqis() {
        let home = tempfile_dir("xdg-migrate");
        let xdg_data = home.join(".local").join("share");
        let legacy = xdg_data.join("Vireon").join("Desktop");
        let current = xdg_data.join("Alvenqis").join("Desktop");
        fs::create_dir_all(legacy.join("wallets")).expect("legacy");
        fs::create_dir_all(current.join("wallets")).expect("current");
        fs::write(legacy.join("wallets").join("from-vireon.json"), b"vireon").expect("write");
        fs::write(current.join("wallets").join("keep.json"), b"alvenqis").expect("write");

        assert!(LEGACY_BRAND_FOLDERS.contains(&"Vireon"));
        assert!(!LEGACY_BRAND_FOLDERS.iter().any(|b| *b == "Alvenqis"));
        copy_missing_tree(&legacy, &current).expect("migrate under XDG");
        assert_eq!(
            fs::read_to_string(current.join("wallets").join("from-vireon.json")).expect("read"),
            "vireon"
        );
        assert_eq!(
            fs::read_to_string(current.join("wallets").join("keep.json")).expect("read"),
            "alvenqis"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn secret_service_missing_message_is_actionable() {
        let msg = format_secret_service_hint(
            "Platform secure storage failure: D-Bus error: The name org.freedesktop.secrets was not provided",
        );
        let lower = msg.to_lowercase();
        assert!(
            lower.contains("secret service") || lower.contains("no secret service"),
            "must name Secret Service: {msg}"
        );
        assert!(
            lower.contains("gnome-keyring") || lower.contains("kwallet"),
            "must suggest install targets: {msg}"
        );
    }

    #[test]
    fn copy_missing_tree_copies_missing_and_preserves_existing() {
        let root = tempfile_dir("migrate");
        let legacy = root.join("Vireon").join("Desktop");
        let current = root.join("Alvenqis").join("Desktop");
        fs::create_dir_all(legacy.join("wallets")).expect("legacy wallets");
        fs::create_dir_all(current.join("wallets")).expect("current wallets");

        fs::write(
            legacy.join("wallets").join("from-legacy.json"),
            b"legacy-only",
        )
        .expect("write");
        fs::write(legacy.join("wallets").join("shared.json"), b"legacy-shared").expect("write");
        fs::write(current.join("wallets").join("shared.json"), b"current-wins").expect("write");
        fs::write(
            current.join("wallets").join("current-only.json"),
            b"current-only",
        )
        .expect("write");

        copy_missing_tree(&legacy, &current).expect("migrate");

        assert_eq!(
            fs::read_to_string(current.join("wallets").join("from-legacy.json")).expect("read"),
            "legacy-only"
        );
        assert_eq!(
            fs::read_to_string(current.join("wallets").join("shared.json")).expect("read"),
            "current-wins",
            "existing current files must not be overwritten"
        );
        assert_eq!(
            fs::read_to_string(current.join("wallets").join("current-only.json")).expect("read"),
            "current-only"
        );
        // Legacy source remains intact (non-destructive).
        assert_eq!(
            fs::read_to_string(legacy.join("wallets").join("from-legacy.json")).expect("read"),
            "legacy-only"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn write_atomic_preserves_directory_that_blocks_metadata_file() {
        let root = tempfile_dir("metadata-directory-collision");
        let path = root.join("active-wallet");
        fs::create_dir_all(&path).expect("blocking directory");
        fs::write(path.join("preserve-me"), b"legacy-data").expect("blocking directory data");

        write_atomic(&path, b"wallet-id").expect("recover metadata path");

        assert!(path.is_file(), "metadata path must become a regular file");
        assert_eq!(fs::read(&path).expect("metadata"), b"wallet-id");
        let backup = root.join("active-wallet.invalid-directory");
        assert!(backup.is_dir(), "unexpected directory must be preserved");
        assert_eq!(
            fs::read(backup.join("preserve-me")).expect("preserved data"),
            b"legacy-data"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn directory_collision_can_restore_completed_temporary_metadata() {
        let root = tempfile_dir("metadata-temporary-recovery");
        let path = root.join("wallet.json");
        fs::create_dir_all(&path).expect("blocking directory");
        fs::write(path.join("preserve-me"), b"legacy-data").expect("blocking directory data");
        let temporary = path.with_extension("tmp");
        fs::write(&temporary, b"completed-write").expect("temporary metadata");

        assert!(recover_completed_atomic_write(&path).expect("recover"));

        assert_eq!(fs::read(&path).expect("metadata"), b"completed-write");
        assert_eq!(
            fs::read(
                root.join("wallet.json.invalid-directory")
                    .join("preserve-me")
            )
            .expect("preserved data"),
            b"legacy-data"
        );
        let _ = fs::remove_dir_all(&root);
    }

    fn tempfile_dir(label: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "alvenqis-keystore-helper-test-{}-{}",
            label,
            std::process::id()
        ));
        // Unique-ish suffix to avoid parallel collisions.
        path.push(format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("tempdir");
        path
    }
}

fn parse_atomic(value: &str) -> Result<u64> {
    value
        .parse()
        .map_err(|_| HelperError::Input("invalid atomic value".into()))
}
fn format_atomic(value: u64) -> String {
    format!(
        "{}.{:08}",
        value / alvenqis_core::ATOMIC_UNITS_PER_ALVE,
        value % alvenqis_core::ATOMIC_UNITS_PER_ALVE
    )
}
fn required(value: Option<String>, name: &str) -> Result<String> {
    value.ok_or_else(|| HelperError::Input(format!("{name} is required")))
}
fn validated_display_name(value: Option<String>) -> Result<String> {
    let name = value
        .unwrap_or_else(|| "Primary wallet".into())
        .trim()
        .to_owned();
    if name.is_empty() || name.chars().count() > 48 || name.chars().any(char::is_control) {
        return Err(HelperError::Input(
            "wallet display name must contain 1 to 48 printable characters".into(),
        ));
    }
    Ok(name)
}
fn require_twenty_four_words(phrase: &str) -> Result<()> {
    if phrase.split_whitespace().count() != 24 {
        return Err(HelperError::Input(
            "recovery phrase must contain exactly 24 words".into(),
        ));
    }
    Ok(())
}
fn required_path(value: Option<PathBuf>, name: &str) -> Result<PathBuf> {
    value.ok_or_else(|| HelperError::Input(format!("{name} is required")))
}
fn credential_error(error: impl std::fmt::Display) -> HelperError {
    let text = error.to_string();
    HelperError::Credential(format_secret_service_hint(&text))
}

/// Map opaque keyring/libsecret failures into an actionable operator message on Linux.
fn format_secret_service_hint(raw: &str) -> String {
    let lower = raw.to_lowercase();
    let looks_like_missing_provider = lower.contains("secret service")
        || lower.contains("org.freedesktop.secrets")
        || lower.contains("no secret service")
        || lower.contains("secret_service")
        || lower.contains("dbus")
        || lower.contains("d-bus")
        || lower.contains("no collection")
        || lower.contains("collection doesn't exist")
        || lower.contains("platform secure storage failure")
        || lower.contains("secure storage failure")
        || lower.contains("no password store")
        || (lower.contains("could not connect")
            && (lower.contains("secret") || lower.contains("keyring")));
    if looks_like_missing_provider {
        format!(
            "no Secret Service provider found for wallet secrets ({raw}). \
Install and unlock a keyring daemon: gnome-keyring (GNOME/Ubuntu/Debian) or kwallet (KDE/Plasma). \
On minimal server images without a desktop session, Secret Service is unavailable — use a desktop environment or an unlocked keyring agent."
        )
    } else {
        raw.to_owned()
    }
}
fn metadata_error(error: impl std::fmt::Display) -> HelperError {
    HelperError::Metadata(error.to_string())
}
fn service_error(error: impl std::fmt::Display) -> HelperError {
    HelperError::Service(error.to_string())
}

#[cfg(windows)]
fn prompt_recovery_phrase() -> Result<String> {
    use std::sync::{
        atomic::{AtomicIsize, Ordering},
        Mutex, OnceLock,
    };
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::UpdateWindow,
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
            GetWindowTextLengthW, GetWindowTextW, LoadCursorW, PostQuitMessage, RegisterClassW,
            ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, ES_AUTOHSCROLL,
            HMENU, IDC_ARROW, MSG, SW_SHOW, WM_COMMAND, WM_CREATE, WM_DESTROY, WNDCLASSW,
            WS_BORDER, WS_CAPTION, WS_CHILD, WS_EX_CLIENTEDGE, WS_OVERLAPPED, WS_SYSMENU,
            WS_VISIBLE,
        },
    };

    static EDIT: AtomicIsize = AtomicIsize::new(0);
    static RESULT: OnceLock<Mutex<Option<String>>> = OnceLock::new();

    unsafe extern "system" fn window_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_CREATE => {
                let instance = GetModuleHandleW(std::ptr::null());
                CreateWindowExW(0, wide("STATIC").as_ptr(), wide("Enter the 24-word Alvenqis recovery phrase. It is passed directly to the Rust keystore and is not exposed to React.").as_ptr(),
                    WS_CHILD | WS_VISIBLE, 20, 20, 660, 36, window, std::ptr::null_mut(), instance, std::ptr::null());
                let edit = CreateWindowExW(
                    WS_EX_CLIENTEDGE,
                    wide("EDIT").as_ptr(),
                    wide("").as_ptr(),
                    WS_CHILD | WS_VISIBLE | WS_BORDER | ES_AUTOHSCROLL as u32,
                    20,
                    66,
                    660,
                    30,
                    window,
                    std::ptr::null_mut(),
                    instance,
                    std::ptr::null(),
                );
                EDIT.store(edit as isize, Ordering::SeqCst);
                CreateWindowExW(
                    0,
                    wide("BUTTON").as_ptr(),
                    wide("Import wallet").as_ptr(),
                    WS_CHILD | WS_VISIBLE,
                    438,
                    116,
                    116,
                    34,
                    window,
                    1usize as HMENU,
                    instance,
                    std::ptr::null(),
                );
                CreateWindowExW(
                    0,
                    wide("BUTTON").as_ptr(),
                    wide("Cancel").as_ptr(),
                    WS_CHILD | WS_VISIBLE,
                    564,
                    116,
                    116,
                    34,
                    window,
                    2usize as HMENU,
                    instance,
                    std::ptr::null(),
                );
                0
            }
            WM_COMMAND => {
                match wparam & 0xffff {
                    1 => {
                        let edit = EDIT.load(Ordering::SeqCst) as HWND;
                        let length = GetWindowTextLengthW(edit);
                        let mut buffer = vec![0u16; length as usize + 1];
                        GetWindowTextW(edit, buffer.as_mut_ptr(), buffer.len() as i32);
                        let phrase = String::from_utf16_lossy(&buffer[..length as usize]);
                        buffer.zeroize();
                        *RESULT
                            .get_or_init(|| Mutex::new(None))
                            .lock()
                            .expect("recovery result lock") = Some(phrase);
                        DestroyWindow(window);
                    }
                    2 => {
                        DestroyWindow(window);
                    }
                    _ => {}
                }
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }

    *RESULT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| HelperError::Service("recovery dialog lock failed".into()))? = None;
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let class_name = wide("AlvenqisRecoveryImportWindow");
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            lpszClassName: class_name.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&class);
        let window = CreateWindowExW(
            0,
            class_name.as_ptr(),
            wide("Import Alvenqis wallet").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            720,
            210,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        if window.is_null() {
            return Err(HelperError::Service(
                "could not create recovery import dialog".into(),
            ));
        }
        ShowWindow(window, SW_SHOW);
        UpdateWindow(window);
        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    RESULT
        .get()
        .and_then(|result| result.lock().ok()?.take())
        .filter(|phrase| !phrase.trim().is_empty())
        .ok_or_else(|| HelperError::Input("wallet import cancelled".into()))
}

#[cfg(target_os = "linux")]
fn prompt_recovery_phrase() -> Result<String> {
    use std::process::Command;
    let output = Command::new("zenity")
        .args([
            "--entry",
            "--hide-text",
            "--title=Import Alvenqis wallet",
            "--text=Enter the 24-word recovery phrase",
        ])
        .output()
        .map_err(|_| {
            HelperError::Service("install zenity to use the secure Linux import dialog".into())
        })?;
    if !output.status.success() {
        return Err(HelperError::Input("wallet import cancelled".into()));
    }
    let phrase = String::from_utf8(output.stdout)
        .map_err(metadata_error)?
        .trim()
        .to_owned();
    if phrase.is_empty() {
        return Err(HelperError::Input("wallet import cancelled".into()));
    }
    Ok(phrase)
}

#[cfg(not(any(windows, target_os = "linux")))]
fn prompt_recovery_phrase() -> Result<String> {
    Err(HelperError::Service(
        "wallet import is not implemented on this platform".into(),
    ))
}

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atomic_format_matches_desktop() {
        assert_eq!(format_atomic(100_000_001), "1.00000001");
        assert_eq!(format_atomic(1), "0.00000001");
    }
}
