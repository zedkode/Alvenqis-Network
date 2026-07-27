use crate::error::{WalletError, WalletResult};
use crate::rpc::{
    fetch_balance, submit_transaction as submit_signed_transaction, RpcBalanceResponse,
    RpcSubmitTransactionResponse,
};
use crate::storage::{
    default_signed_tx_dir, default_wallet_dir, ensure_signed_tx_dir, load_wallet,
    private_key_from_wallet, write_wallet_with_metadata, StoredWallet,
};
use alvenqis_core::{
    generate_mnemonic, hash_to_hex, launch_address_standard, launch_key_derivation_policy,
    launch_signing_standard, launch_wallet_seed_standard, next_base_fee, Address, Amount, Chain,
    MnemonicWordCount, Network, PrivateKey, Transaction, WalletDerivationPath,
};
use alvenqis_node::storage;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_RPC_BASE_URL: &str = "https://rpcnode.dohotstudio.com";
pub const DEFAULT_MAINNET_DATA_DIR: &str = ".alvenqis-mainnet/chain";
pub const DEFAULT_DEVNET_DATA_DIR: &str = DEFAULT_MAINNET_DATA_DIR;

pub fn default_rpc_base_url_for_network(network: Network) -> String {
    format!("http://127.0.0.1:{}", network.default_rpc_port())
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicWalletInfo {
    pub network_id: String,
    pub network_name: String,
    pub status_label: String,
    pub address: String,
    pub public_key_hex: String,
    pub address_standard_id: &'static str,
    pub signature_standard_id: &'static str,
    pub key_derivation_policy_id: &'static str,
    pub wallet_seed_standard_id: &'static str,
    pub key_origin: String,
    pub derivation_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct WalletStatus {
    pub mode: String,
    pub protocol_parameters_id: &'static str,
    pub active_network_id: String,
    pub active_network_name: String,
    pub active_status_label: String,
    pub address_standard_id: &'static str,
    pub address_prefix: &'static str,
    pub signature_standard_id: &'static str,
    pub tx_signing_domain: &'static str,
    pub key_derivation_policy_id: &'static str,
    pub wallet_seed_standard_id: &'static str,
    pub wallet_dir: String,
    pub signed_tx_dir: String,
    pub wallet_present: bool,
    pub address: Option<String>,
    pub key_origin: Option<String>,
    pub derivation_path: Option<String>,
    pub rpc_base_url: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CreatedWallet {
    pub network_id: String,
    pub network_name: String,
    pub status_label: String,
    pub address: String,
    pub public_key_hex: String,
    pub derivation_path: String,
    pub wallet_seed_standard_id: &'static str,
    pub key_derivation_policy_id: &'static str,
    pub mnemonic_word_count: usize,
    pub mnemonic: String,
    pub warning: &'static str,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SignedTxFile {
    pub mode: String,
    pub network_id: String,
    pub network_name: String,
    pub status_label: String,
    pub warning: &'static str,
    pub tx_hash: String,
    pub path: String,
    pub transaction: Transaction,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SubmittedTxResult {
    pub network_id: String,
    pub status: String,
    pub tx_hash: String,
    pub lifecycle_status: String,
    pub mempool_size: usize,
    pub source_path: String,
}

pub fn default_wallet_dir_path(network: Network) -> WalletResult<PathBuf> {
    default_wallet_dir(network)
}

pub fn default_signed_tx_dir_path(network: Network) -> WalletResult<PathBuf> {
    default_signed_tx_dir(network)
}

pub fn default_chain_data_dir(network: Network) -> PathBuf {
    crate::storage::default_network_root(network)
        .unwrap_or_else(|_| PathBuf::from(network.default_data_root()))
        .join("chain")
}

pub fn create_dev_wallet(wallet_dir: &Path, network: Network) -> WalletResult<StoredWallet> {
    let private_key = PrivateKey::generate();
    write_wallet_with_metadata(
        wallet_dir,
        &private_key,
        network,
        Some("raw-private-key"),
        None,
        None,
        Some(launch_key_derivation_policy().policy_id),
    )
}

pub fn import_dev_private_key(
    wallet_dir: &Path,
    private_key_hex: &str,
    network: Network,
) -> WalletResult<StoredWallet> {
    let private_key = PrivateKey::from_hex(private_key_hex)?;
    write_wallet_with_metadata(
        wallet_dir,
        &private_key,
        network,
        Some("raw-private-key-import"),
        None,
        None,
        Some(launch_key_derivation_policy().policy_id),
    )
}

pub fn create_wallet(
    wallet_dir: &Path,
    network: Network,
    word_count: MnemonicWordCount,
    path: WalletDerivationPath,
) -> WalletResult<CreatedWallet> {
    let mnemonic = generate_mnemonic(word_count)?;
    let private_key = PrivateKey::from_mnemonic(&mnemonic, "", path)?;
    let wallet = write_wallet_with_metadata(
        wallet_dir,
        &private_key,
        network,
        Some("bip39-mnemonic"),
        Some(&path.to_string()),
        Some(launch_wallet_seed_standard().standard_id),
        Some(launch_key_derivation_policy().policy_id),
    )?;
    Ok(CreatedWallet {
        network_id: wallet.network_id.clone(),
        network_name: wallet.network_name.clone(),
        status_label: wallet.status_label.clone(),
        address: wallet.address.clone(),
        public_key_hex: wallet.public_key_hex.clone(),
        derivation_path: path.to_string(),
        wallet_seed_standard_id: launch_wallet_seed_standard().standard_id,
        key_derivation_policy_id: launch_key_derivation_policy().policy_id,
        mnemonic_word_count: word_count.as_usize(),
        mnemonic,
        warning:
            "Back up this mnemonic now. It is shown once and is not stored in the repository or returned by later wallet commands.",
    })
}

pub fn import_mnemonic_wallet(
    wallet_dir: &Path,
    phrase: &str,
    passphrase: &str,
    network: Network,
    path: WalletDerivationPath,
) -> WalletResult<StoredWallet> {
    let private_key = PrivateKey::from_mnemonic(phrase, passphrase, path)?;
    write_wallet_with_metadata(
        wallet_dir,
        &private_key,
        network,
        Some("bip39-mnemonic-import"),
        Some(&path.to_string()),
        Some(launch_wallet_seed_standard().standard_id),
        Some(launch_key_derivation_policy().policy_id),
    )
}

pub fn wallet_address(wallet_dir: &Path) -> WalletResult<String> {
    Ok(load_wallet(wallet_dir)?.address)
}

pub fn export_public_info(wallet_dir: &Path) -> WalletResult<PublicWalletInfo> {
    let wallet = load_wallet(wallet_dir)?;
    let network = wallet.network()?;
    Ok(PublicWalletInfo {
        network_id: wallet.network_id,
        network_name: wallet.network_name,
        status_label: wallet.status_label,
        address: wallet.address,
        public_key_hex: wallet.public_key_hex,
        address_standard_id: launch_address_standard(network).standard_id,
        signature_standard_id: launch_signing_standard().standard_id,
        key_derivation_policy_id: launch_key_derivation_policy().policy_id,
        wallet_seed_standard_id: launch_wallet_seed_standard().standard_id,
        key_origin: wallet
            .key_origin
            .unwrap_or_else(|| "legacy-raw-private-key".to_owned()),
        derivation_path: wallet.derivation_path,
    })
}

pub fn wallet_status(
    selected_network: Network,
    wallet_dir: &Path,
    signed_tx_dir: &Path,
    rpc_base_url: &str,
) -> WalletResult<WalletStatus> {
    let wallet_path = crate::storage::wallet_file_path(wallet_dir);
    let wallet_present = wallet_path.exists();
    let (
        active_network,
        active_network_name,
        active_status_label,
        address,
        key_origin,
        derivation_path,
    ) = if wallet_present {
        let wallet = load_wallet(wallet_dir)?;
        (
            wallet.network()?,
            wallet.network_name,
            wallet.status_label,
            Some(wallet.address),
            wallet.key_origin,
            wallet.derivation_path,
        )
    } else {
        (
            selected_network,
            selected_network.human_name().to_owned(),
            selected_network.status_label().to_owned(),
            None,
            None,
            None,
        )
    };

    Ok(WalletStatus {
        mode: format!("{} / Prototype", active_status_label),
        protocol_parameters_id: alvenqis_core::launch_protocol_parameters(active_network)
            .parameters_id,
        active_network_id: active_network.network_id().to_owned(),
        active_network_name,
        active_status_label,
        address_standard_id: launch_address_standard(active_network).standard_id,
        address_prefix: launch_address_standard(active_network).address_prefix,
        signature_standard_id: launch_signing_standard().standard_id,
        tx_signing_domain: launch_signing_standard().tx_signing_domain,
        key_derivation_policy_id: launch_key_derivation_policy().policy_id,
        wallet_seed_standard_id: launch_wallet_seed_standard().standard_id,
        wallet_dir: wallet_dir.display().to_string(),
        signed_tx_dir: signed_tx_dir.display().to_string(),
        wallet_present,
        address,
        key_origin,
        derivation_path,
        rpc_base_url: rpc_base_url.to_owned(),
    })
}

pub fn balance(rpc_base_url: &str, address: &str) -> WalletResult<RpcBalanceResponse> {
    Address::parse(address)?;
    fetch_balance(rpc_base_url, address)
}

pub fn sign_tx(
    wallet_dir: &Path,
    signed_tx_dir: &Path,
    chain_data_dir: &Path,
    to: &str,
    amount: &str,
    fee: &str,
) -> WalletResult<SignedTxFile> {
    let recipient = Address::parse(to)?;
    let amount = Amount::parse_alve(amount)?;
    let fee = Amount::parse_alve(fee)?;
    if amount == Amount::ZERO {
        return Err(WalletError::Input(
            "transaction amount must be greater than zero".to_owned(),
        ));
    }

    let wallet = load_wallet(wallet_dir)?;
    let wallet_network = wallet.network()?;
    if recipient.network() != wallet_network {
        return Err(WalletError::Core(
            alvenqis_core::AlvenqisError::InvalidNetwork {
                expected: wallet_network.network_id().to_owned(),
                actual: recipient.network().network_id().to_owned(),
            },
        ));
    }
    let private_key = private_key_from_wallet(&wallet)?;
    let chain = load_chain(chain_data_dir)?;
    if chain.network() != wallet_network {
        return Err(WalletError::Input(format!(
            "wallet network {} does not match chain network {}",
            wallet_network.network_id(),
            chain.network().network_id()
        )));
    }
    let anticipated_base_fee = next_base_fee(chain.blocks().last());
    let balance = chain.state().balance_of(&wallet.address);
    let required = amount.checked_add(anticipated_base_fee)?.checked_add(fee)?;
    if balance < required {
        return Err(WalletError::Input(format!(
            "insufficient Mainnet Candidate balance: available {}, required {}",
            balance.as_atomic(),
            required.as_atomic()
        )));
    }

    let nonce = next_account_nonce(chain.blocks(), &wallet.address);
    let transaction = Transaction::new_signed(
        1,
        nonce,
        wallet_network,
        &private_key,
        to.to_owned(),
        amount,
        anticipated_base_fee.checked_add(fee)?,
        fee,
        None,
    )?;
    ensure_signed_tx_dir(signed_tx_dir)?;
    let tx_hash = hash_to_hex(&transaction.tx_hash());
    let path = signed_tx_dir.join(format!("{tx_hash}.json"));
    fs::write(&path, serde_json::to_string_pretty(&transaction)?)?;

    Ok(SignedTxFile {
        mode: format!("{} / Prototype", wallet.status_label),
        network_id: wallet.network_id,
        network_name: wallet.network_name,
        status_label: wallet.status_label,
        warning:
            "Saved signed transaction for local prototype use only. Submit it through the matching local RPC endpoint.",
        tx_hash,
        path: path.display().to_string(),
        transaction,
    })
}

pub fn verify_tx(tx_file: &Path) -> WalletResult<Transaction> {
    let content = fs::read_to_string(tx_file)?;
    let transaction: Transaction = serde_json::from_str(&content)?;
    transaction.verify()?;
    Ok(transaction)
}

pub fn submit_tx(rpc_base_url: &str, tx_file: &Path) -> WalletResult<SubmittedTxResult> {
    let transaction = verify_tx(tx_file)?;
    let network = transaction.network()?;
    let response: RpcSubmitTransactionResponse =
        submit_signed_transaction(rpc_base_url, &transaction)?;
    Ok(SubmittedTxResult {
        network_id: network.network_id().to_owned(),
        status: response.status,
        tx_hash: response.tx_hash,
        lifecycle_status: response.lifecycle_status,
        mempool_size: response.mempool_size,
        source_path: tx_file.display().to_string(),
    })
}

pub fn load_chain(chain_data_dir: &Path) -> WalletResult<Chain> {
    let blocks = storage::load_blocks(chain_data_dir)?;
    let first_block = blocks.first().ok_or_else(|| {
        alvenqis_node::NodeError::ChainNotInitialized(storage::chain_file_path(chain_data_dir))
    })?;
    let mut chain = Chain::new(first_block.network()?);
    for block in blocks {
        chain.append_block(block)?;
    }
    Ok(chain)
}

fn next_account_nonce(blocks: &[alvenqis_core::Block], address: &str) -> u64 {
    // Prefer ledger sequential state when the chain rebuilds cleanly; fall back to scan.
    if let Some(first) = blocks.first() {
        if let Ok(network) = first.network() {
            if let Ok(chain) = Chain::from_blocks(network, blocks.to_vec()) {
                return chain.state().next_nonce_of(address);
            }
        }
    }
    blocks
        .iter()
        .flat_map(|block| block.transactions.iter())
        .filter(|transaction| transaction.from.as_deref() == Some(address))
        .map(|transaction| transaction.nonce)
        .max()
        .map_or(alvenqis_core::FIRST_ACCOUNT_NONCE, |nonce| nonce + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alvenqis_core::{devnet_genesis_with_difficulty, Address, Network, WalletDerivationPath};
    use tempfile::tempdir;

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn creating_a_mnemonic_wallet_works() {
        let temp = tempdir().expect("tempdir");
        let created = create_wallet(
            temp.path(),
            Network::Devnet,
            MnemonicWordCount::TwentyFour,
            WalletDerivationPath::default(),
        )
        .expect("wallet");

        assert!(crate::storage::wallet_file_path(temp.path()).exists());
        assert_eq!(created.network_id, Network::Devnet.network_id());
        assert_eq!(created.mnemonic.split_whitespace().count(), 24);
        assert_eq!(
            created.wallet_seed_standard_id,
            "alvenqis-wallet-bip39-slip10-v1"
        );
        assert_eq!(created.derivation_path, "m/44'/7330'/0'/0'/0'");
    }

    #[test]
    fn importing_mnemonic_reproduces_same_address() {
        let temp = tempdir().expect("tempdir");
        let wallet_dir = temp.path().join("wallets");
        let imported = import_mnemonic_wallet(
            &wallet_dir,
            TEST_MNEMONIC,
            "",
            Network::Devnet,
            WalletDerivationPath::new(0, 0, 7),
        )
        .expect("import");
        let imported_again = import_mnemonic_wallet(
            &temp.path().join("wallets-2"),
            TEST_MNEMONIC,
            "",
            Network::Devnet,
            WalletDerivationPath::new(0, 0, 7),
        )
        .expect("import again");

        assert_eq!(imported.address, imported_again.address);
        assert_eq!(
            imported.derivation_path.as_deref(),
            Some("m/44'/7330'/0'/0'/7'")
        );
    }

    #[test]
    fn creating_a_dev_wallet_works() {
        let temp = tempdir().expect("tempdir");
        let wallet = create_dev_wallet(temp.path(), Network::Devnet).expect("wallet");
        assert!(crate::storage::wallet_file_path(temp.path()).exists());
        assert_eq!(wallet.network_id, Network::Devnet.network_id());
        assert!(wallet.address.starts_with("dalve1"));
    }

    #[test]
    fn address_derivation_and_format_work() {
        let temp = tempdir().expect("tempdir");
        let wallet = create_dev_wallet(temp.path(), Network::Devnet).expect("wallet");
        Address::parse(&wallet.address).expect("address parses");
    }

    #[test]
    fn signing_a_transaction_works() {
        let temp = tempdir().expect("tempdir");
        let wallet_dir = temp.path().join("wallets");
        let signed_tx_dir = temp.path().join("signed");
        let data_dir = temp.path().join(".alvenqis-dev/chain");
        let wallet = create_dev_wallet(&wallet_dir, Network::Devnet).expect("wallet");
        let recipient =
            create_dev_wallet(&temp.path().join("recipient"), Network::Devnet).expect("recipient");
        let genesis = devnet_genesis_with_difficulty(&wallet.address, 4).expect("genesis");
        storage::append_block(&data_dir, &genesis).expect("append");

        let signed = sign_tx(
            &wallet_dir,
            &signed_tx_dir,
            &data_dir,
            &recipient.address,
            "1.00000000",
            "0.00000001",
        )
        .expect("sign");

        assert!(Path::new(&signed.path).exists());
        assert_eq!(signed.transaction.to, recipient.address);
    }

    #[test]
    fn verifying_a_signed_transaction_works() {
        let temp = tempdir().expect("tempdir");
        let wallet_dir = temp.path().join("wallets");
        let signed_tx_dir = temp.path().join("signed");
        let data_dir = temp.path().join(".alvenqis-dev/chain");
        let wallet = create_dev_wallet(&wallet_dir, Network::Devnet).expect("wallet");
        let recipient =
            create_dev_wallet(&temp.path().join("recipient"), Network::Devnet).expect("recipient");
        let genesis = devnet_genesis_with_difficulty(&wallet.address, 4).expect("genesis");
        storage::append_block(&data_dir, &genesis).expect("append");
        let signed = sign_tx(
            &wallet_dir,
            &signed_tx_dir,
            &data_dir,
            &recipient.address,
            "1.00000000",
            "0.00000001",
        )
        .expect("sign");

        let verified = verify_tx(Path::new(&signed.path)).expect("verify");
        assert_eq!(verified.tx_hash(), signed.transaction.tx_hash());
    }

    #[test]
    fn tampered_signed_transaction_fails() {
        let temp = tempdir().expect("tempdir");
        let wallet_dir = temp.path().join("wallets");
        let signed_tx_dir = temp.path().join("signed");
        let data_dir = temp.path().join(".alvenqis-dev/chain");
        let wallet = create_dev_wallet(&wallet_dir, Network::Devnet).expect("wallet");
        let recipient =
            create_dev_wallet(&temp.path().join("recipient"), Network::Devnet).expect("recipient");
        let genesis = devnet_genesis_with_difficulty(&wallet.address, 4).expect("genesis");
        storage::append_block(&data_dir, &genesis).expect("append");
        let signed = sign_tx(
            &wallet_dir,
            &signed_tx_dir,
            &data_dir,
            &recipient.address,
            "1.00000000",
            "0.00000001",
        )
        .expect("sign");

        let mut transaction: Transaction =
            serde_json::from_str(&fs::read_to_string(&signed.path).expect("tx file"))
                .expect("json");
        transaction.amount = Amount::from_atomic(transaction.amount.as_atomic() + 1);
        fs::write(
            &signed.path,
            serde_json::to_string_pretty(&transaction).expect("json"),
        )
        .expect("write");

        let error = verify_tx(Path::new(&signed.path)).expect_err("tamper must fail");
        assert!(matches!(
            error,
            WalletError::Core(alvenqis_core::AlvenqisError::InvalidSignature(_))
        ));
    }

    #[test]
    fn wallet_refuses_invalid_address() {
        let temp = tempdir().expect("tempdir");
        let wallet_dir = temp.path().join("wallets");
        let signed_tx_dir = temp.path().join("signed");
        let data_dir = temp.path().join(".alvenqis-dev/chain");
        let wallet = create_dev_wallet(&wallet_dir, Network::Devnet).expect("wallet");
        let genesis = devnet_genesis_with_difficulty(&wallet.address, 4).expect("genesis");
        storage::append_block(&data_dir, &genesis).expect("append");

        let error = sign_tx(
            &wallet_dir,
            &signed_tx_dir,
            &data_dir,
            "not-an-address",
            "1.0",
            "0.1",
        )
        .expect_err("invalid address must fail");
        assert!(matches!(
            error,
            WalletError::Core(alvenqis_core::AlvenqisError::InvalidAddress(_))
        ));
    }

    #[test]
    fn wallet_handles_missing_rpc_gracefully() {
        let wallet =
            create_dev_wallet(tempdir().expect("tempdir").path(), Network::Devnet).expect("wallet");
        let error =
            balance("http://127.0.0.1:65534", &wallet.address).expect_err("missing RPC must fail");
        assert!(matches!(error, WalletError::RpcUnavailable(_)));
    }

    #[test]
    fn wallet_status_reports_active_network() {
        let temp = tempdir().expect("tempdir");
        let wallet_dir = temp.path().join("wallets");
        let signed_tx_dir = temp.path().join("signed");
        create_dev_wallet(&wallet_dir, Network::Devnet).expect("wallet");

        let status = wallet_status(
            Network::Devnet,
            &wallet_dir,
            &signed_tx_dir,
            DEFAULT_RPC_BASE_URL,
        )
        .expect("status");

        assert_eq!(status.active_network_id, Network::Devnet.network_id());
        assert_eq!(status.active_status_label, Network::Devnet.status_label());
        assert_eq!(
            status.address_standard_id,
            "alvenqis-address-bech32m-ed25519-v1"
        );
        assert_eq!(
            status.signature_standard_id,
            "alvenqis-signature-ed25519-v1"
        );
        assert_eq!(status.key_derivation_policy_id, "alvenqis-key-ed25519-v1");
        assert_eq!(
            status.wallet_seed_standard_id,
            "alvenqis-wallet-bip39-slip10-v1"
        );
    }

    #[test]
    fn export_public_info_reports_frozen_standards() {
        let temp = tempdir().expect("tempdir");
        let wallet_dir = temp.path().join("wallets");
        import_mnemonic_wallet(
            &wallet_dir,
            TEST_MNEMONIC,
            "",
            Network::Devnet,
            WalletDerivationPath::new(0, 1, 2),
        )
        .expect("wallet");

        let info = export_public_info(&wallet_dir).expect("public info");
        assert_eq!(
            info.address_standard_id,
            "alvenqis-address-bech32m-ed25519-v1"
        );
        assert_eq!(info.signature_standard_id, "alvenqis-signature-ed25519-v1");
        assert_eq!(info.key_derivation_policy_id, "alvenqis-key-ed25519-v1");
        assert_eq!(
            info.wallet_seed_standard_id,
            "alvenqis-wallet-bip39-slip10-v1"
        );
        assert_eq!(info.key_origin, "bip39-mnemonic-import");
        assert_eq!(
            info.derivation_path.as_deref(),
            Some("m/44'/7330'/0'/1'/2'")
        );
    }

    #[test]
    fn wallet_files_are_not_created_inside_tracked_source_folders() {
        let temp = tempdir().expect("tempdir");
        let wallet_dir = temp.path().join("wallets");
        create_dev_wallet(&wallet_dir, Network::Devnet).expect("wallet");

        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root");
        let wallet_file = crate::storage::wallet_file_path(&wallet_dir);
        assert!(!wallet_file.starts_with(workspace_root));
    }
}
