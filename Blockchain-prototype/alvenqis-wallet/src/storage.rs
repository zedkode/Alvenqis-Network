use crate::error::{WalletError, WalletResult};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use alvenqis_core::{Address, Network, PrivateKey};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use zeroize::Zeroize;

pub const DEFAULT_WALLET_FILE_NAME: &str = "default-wallet.json";
pub const SCHEMA_PLAINTEXT_LEGACY: &str = "alvenqis-wallet-plaintext-v0";
pub const SCHEMA_ENCRYPTED_V1: &str = "alvenqis-wallet-encrypted-v1";

/// Env var for the passphrase that encrypts mainnet-candidate wallet files.
pub const WALLET_PASSPHRASE_ENV: &str = "ALVENQIS_WALLET_PASSPHRASE";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedSecret {
    pub kdf: String,
    pub salt_b64: String,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredWallet {
    #[serde(default = "default_schema_legacy")]
    pub schema: String,
    pub network_id: String,
    pub network_name: String,
    pub status_label: String,
    pub warning: String,
    pub address: String,
    pub public_key_hex: String,
    /// Present only on legacy plaintext files. Prefer `encrypted`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_key_hex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted: Option<EncryptedSecret>,
    #[serde(default)]
    pub key_origin: Option<String>,
    #[serde(default)]
    pub derivation_path: Option<String>,
    #[serde(default)]
    pub wallet_seed_standard_id: Option<String>,
    #[serde(default)]
    pub key_derivation_policy_id: Option<String>,
}

fn default_schema_legacy() -> String {
    SCHEMA_PLAINTEXT_LEGACY.to_owned()
}

impl StoredWallet {
    pub fn network(&self) -> WalletResult<Network> {
        self.network_id
            .parse()
            .map_err(|error: alvenqis_core::AlvenqisError| WalletError::Core(error))
    }

    pub fn is_encrypted(&self) -> bool {
        self.encrypted.is_some()
            || self.schema == SCHEMA_ENCRYPTED_V1
            || self
                .private_key_hex
                .as_ref()
                .map(|s| s.is_empty())
                .unwrap_or(true)
                && self.encrypted.is_some()
    }
}

pub fn default_network_root(network: Network) -> WalletResult<PathBuf> {
    dirs::home_dir()
        .map(|home| home.join(network.default_data_root()))
        .ok_or(WalletError::StorageUnavailable)
}

pub fn default_wallet_dir(network: Network) -> WalletResult<PathBuf> {
    Ok(default_network_root(network)?.join("wallets"))
}

pub fn default_signed_tx_dir(network: Network) -> WalletResult<PathBuf> {
    Ok(default_network_root(network)?.join("signed-txs"))
}

pub fn wallet_file_path(wallet_dir: &Path) -> PathBuf {
    wallet_dir.join(DEFAULT_WALLET_FILE_NAME)
}

pub fn ensure_wallet_dir(wallet_dir: &Path) -> WalletResult<()> {
    fs::create_dir_all(wallet_dir)?;
    Ok(())
}

pub fn ensure_signed_tx_dir(signed_tx_dir: &Path) -> WalletResult<()> {
    fs::create_dir_all(signed_tx_dir)?;
    Ok(())
}

fn requires_encryption(network: Network) -> bool {
    matches!(network, Network::MainnetCandidate)
}

fn resolve_passphrase(explicit: Option<&str>) -> WalletResult<String> {
    if let Some(p) = explicit {
        if !p.is_empty() {
            return Ok(p.to_owned());
        }
    }
    match std::env::var(WALLET_PASSPHRASE_ENV) {
        Ok(p) if !p.trim().is_empty() => Ok(p),
        _ => Err(WalletError::Input(format!(
            "mainnet-candidate wallets must be encrypted: set {WALLET_PASSPHRASE_ENV} or pass an encryption passphrase"
        ))),
    }
}

fn derive_key(passphrase: &str, salt: &[u8]) -> WalletResult<[u8; 32]> {
    let params = Params::new(19_456, 2, 1, Some(32))
        .map_err(|e| WalletError::Input(format!("argon2 params: {e}")))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| WalletError::Input(format!("argon2 derive failed: {e}")))?;
    Ok(key)
}

fn encrypt_secret(passphrase: &str, plaintext: &[u8]) -> WalletResult<EncryptedSecret> {
    let mut salt = [0u8; 16];
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let mut key = derive_key(passphrase, &salt)?;
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| WalletError::Input(format!("aes key: {e}")))?;
    key.zeroize();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| WalletError::Input("encryption failed".into()))?;
    Ok(EncryptedSecret {
        kdf: "argon2id".into(),
        salt_b64: B64.encode(salt),
        nonce_b64: B64.encode(nonce_bytes),
        ciphertext_b64: B64.encode(ciphertext),
    })
}

fn decrypt_secret(passphrase: &str, secret: &EncryptedSecret) -> WalletResult<Vec<u8>> {
    if secret.kdf != "argon2id" {
        return Err(WalletError::Input(format!(
            "unsupported kdf {}",
            secret.kdf
        )));
    }
    let salt = B64
        .decode(&secret.salt_b64)
        .map_err(|e| WalletError::Input(format!("salt: {e}")))?;
    let nonce_bytes = B64
        .decode(&secret.nonce_b64)
        .map_err(|e| WalletError::Input(format!("nonce: {e}")))?;
    let ciphertext = B64
        .decode(&secret.ciphertext_b64)
        .map_err(|e| WalletError::Input(format!("ciphertext: {e}")))?;
    if nonce_bytes.len() != 12 {
        return Err(WalletError::Input("invalid nonce length".into()));
    }
    let mut key = derive_key(passphrase, &salt)?;
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| WalletError::Input(format!("aes key: {e}")))?;
    key.zeroize();
    let nonce = Nonce::from_slice(&nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| WalletError::Input("decryption failed (wrong passphrase?)".into()))
}

pub fn write_wallet(
    wallet_dir: &Path,
    private_key: &PrivateKey,
    network: Network,
) -> WalletResult<StoredWallet> {
    write_wallet_with_metadata(wallet_dir, private_key, network, None, None, None, None)
}

pub fn write_wallet_with_metadata(
    wallet_dir: &Path,
    private_key: &PrivateKey,
    network: Network,
    key_origin: Option<&str>,
    derivation_path: Option<&str>,
    wallet_seed_standard_id: Option<&str>,
    key_derivation_policy_id: Option<&str>,
) -> WalletResult<StoredWallet> {
    write_wallet_with_metadata_passphrase(
        wallet_dir,
        private_key,
        network,
        key_origin,
        derivation_path,
        wallet_seed_standard_id,
        key_derivation_policy_id,
        None,
    )
}

pub fn write_wallet_with_metadata_passphrase(
    wallet_dir: &Path,
    private_key: &PrivateKey,
    network: Network,
    key_origin: Option<&str>,
    derivation_path: Option<&str>,
    wallet_seed_standard_id: Option<&str>,
    key_derivation_policy_id: Option<&str>,
    encryption_passphrase: Option<&str>,
) -> WalletResult<StoredWallet> {
    ensure_wallet_dir(wallet_dir)?;
    let public_key = private_key.public_key();
    let address = Address::from_public_key_for_network(&public_key, network).to_string();
    let public_key_hex = public_key.to_hex();

    let (schema, private_key_hex, encrypted, warning) = if requires_encryption(network) {
        let pass = resolve_passphrase(encryption_passphrase)?;
        let mut secret_hex = private_key.to_hex();
        let encrypted = encrypt_secret(&pass, secret_hex.as_bytes())?;
        secret_hex.zeroize();
        (
            SCHEMA_ENCRYPTED_V1.to_owned(),
            None,
            Some(encrypted),
            "Encrypted wallet file (AES-256-GCM + Argon2id). Private key is never stored in plaintext."
                .to_owned(),
        )
    } else {
        // Local/dev fixtures only — never for mainnet-candidate.
        (
            SCHEMA_PLAINTEXT_LEGACY.to_owned(),
            Some(private_key.to_hex()),
            None,
            "Local prototype wallet material (devnet/test fixtures). Mainnet-candidate wallets are encrypted."
                .to_owned(),
        )
    };

    let wallet = StoredWallet {
        schema,
        network_id: network.network_id().to_owned(),
        network_name: network.human_name().to_owned(),
        status_label: network.status_label().to_owned(),
        warning,
        address,
        public_key_hex,
        private_key_hex,
        encrypted,
        key_origin: key_origin.map(ToOwned::to_owned),
        derivation_path: derivation_path.map(ToOwned::to_owned),
        wallet_seed_standard_id: wallet_seed_standard_id.map(ToOwned::to_owned),
        key_derivation_policy_id: key_derivation_policy_id.map(ToOwned::to_owned),
    };
    let path = wallet_file_path(wallet_dir);
    let content = serde_json::to_string_pretty(&wallet)?;
    // Best-effort restrictive mode on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true).mode(0o600);
        use std::io::Write;
        let mut file = opts.open(&path)?;
        file.write_all(content.as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        fs::write(&path, content)?;
    }
    Ok(wallet)
}

pub fn load_wallet(wallet_dir: &Path) -> WalletResult<StoredWallet> {
    let path = wallet_file_path(wallet_dir);
    if !path.exists() {
        return Err(WalletError::WalletNotFound(path.display().to_string()));
    }
    let content = fs::read_to_string(path)?;
    let mut wallet: StoredWallet = serde_json::from_str(&content)?;
    // Backward compat: older files had private_key_hex as required string.
    if wallet.private_key_hex.is_none() && wallet.encrypted.is_none() {
        // Try re-parse legacy with required string field via Value
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(hex) = value.get("private_key_hex").and_then(|v| v.as_str()) {
                wallet.private_key_hex = Some(hex.to_owned());
                wallet.schema = SCHEMA_PLAINTEXT_LEGACY.to_owned();
            }
        }
    }
    Ok(wallet)
}

/// Unlock the private key. Encrypted wallets need `passphrase` or `ALVENQIS_WALLET_PASSPHRASE`.
pub fn private_key_from_wallet(wallet: &StoredWallet) -> WalletResult<PrivateKey> {
    private_key_from_wallet_passphrase(wallet, None)
}

pub fn private_key_from_wallet_passphrase(
    wallet: &StoredWallet,
    encryption_passphrase: Option<&str>,
) -> WalletResult<PrivateKey> {
    if let Some(secret) = &wallet.encrypted {
        let pass = resolve_passphrase(encryption_passphrase)?;
        let mut plain = decrypt_secret(&pass, secret)?;
        let hex = std::str::from_utf8(&plain)
            .map_err(|_| WalletError::Input("decrypted key is not utf8".into()))?
            .to_owned();
        plain.zeroize();
        let key = PrivateKey::from_hex(&hex).map_err(WalletError::from);
        // hex is dropped (not zeroized string fully, but ok for this pass)
        return key;
    }
    if let Some(hex) = &wallet.private_key_hex {
        if !hex.is_empty() {
            // Refuse plaintext mainnet-candidate material for signing after upgrade path.
            if wallet.network_id == Network::MainnetCandidate.network_id() {
                return Err(WalletError::Input(
                    "legacy plaintext mainnet-candidate wallet detected; re-import with encryption (ALVENQIS_WALLET_PASSPHRASE)".into(),
                ));
            }
            return PrivateKey::from_hex(hex).map_err(WalletError::from);
        }
    }
    Err(WalletError::Input(
        "wallet file has no usable private key material".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn mainnet_candidate_wallet_is_encrypted_not_plaintext() {
        let dir = tempdir().expect("temp");
        std::env::set_var(WALLET_PASSPHRASE_ENV, "test-passphrase-p0-audit");
        let key = PrivateKey::generate();
        let stored = write_wallet(dir.path(), &key, Network::MainnetCandidate).expect("write");
        assert_eq!(stored.schema, SCHEMA_ENCRYPTED_V1);
        assert!(stored.encrypted.is_some());
        assert!(stored.private_key_hex.is_none());

        let raw = fs::read_to_string(wallet_file_path(dir.path())).expect("read file");
        assert!(
            !raw.contains(&key.to_hex()),
            "file must not contain private key hex"
        );

        let loaded = load_wallet(dir.path()).expect("load");
        let unlocked = private_key_from_wallet(&loaded).expect("unlock");
        assert_eq!(unlocked.to_hex(), key.to_hex());
        std::env::remove_var(WALLET_PASSPHRASE_ENV);
    }

    #[test]
    fn devnet_may_still_use_plaintext_for_fixtures() {
        let dir = tempdir().expect("temp");
        let key = PrivateKey::generate();
        let stored = write_wallet(dir.path(), &key, Network::Devnet).expect("write");
        assert!(stored.private_key_hex.is_some());
        let unlocked = private_key_from_wallet(&stored).expect("unlock");
        assert_eq!(unlocked.to_hex(), key.to_hex());
    }
}
