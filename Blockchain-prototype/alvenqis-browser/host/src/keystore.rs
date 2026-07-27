use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use alvenqis_sdk_rust::{Address, Network, PrivateKey, WalletAccount};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use zeroize::Zeroize;

pub const SCHEMA_ENCRYPTED_V1: &str = "alvenqis-browser-host-encrypted-v1";
pub const DEFAULT_KEYSTORE_FILE: &str = "browser-host-wallet.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedSecret {
    pub kdf: String,
    pub salt_b64: String,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredHostWallet {
    pub schema: String,
    pub network_id: String,
    pub network_name: String,
    pub status_label: String,
    pub address: String,
    pub public_key_hex: String,
    pub encrypted: EncryptedSecret,
    pub derivation_path: Option<String>,
    pub warning: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicWalletView {
    pub network_id: String,
    pub network_name: String,
    pub status_label: String,
    pub address: String,
    pub public_key_hex: String,
    pub derivation_path: Option<String>,
    pub keystore_path: String,
    pub schema: String,
}

impl StoredHostWallet {
    pub fn public_view(&self, path: &Path) -> PublicWalletView {
        PublicWalletView {
            network_id: self.network_id.clone(),
            network_name: self.network_name.clone(),
            status_label: self.status_label.clone(),
            address: self.address.clone(),
            public_key_hex: self.public_key_hex.clone(),
            derivation_path: self.derivation_path.clone(),
            keystore_path: path.display().to_string(),
            schema: self.schema.clone(),
        }
    }
}

pub fn default_keystore_dir(network: Network) -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "home directory unavailable".to_owned())?;
    Ok(home
        .join(network.default_data_root())
        .join("browser-host")
        .join("wallets"))
}

pub fn keystore_path(dir: &Path) -> PathBuf {
    dir.join(DEFAULT_KEYSTORE_FILE)
}

pub fn keystore_exists(dir: &Path) -> bool {
    keystore_path(dir).exists()
}

pub fn load_stored(dir: &Path) -> Result<StoredHostWallet, String> {
    let path = keystore_path(dir);
    if !path.exists() {
        return Err(format!("keystore not found at {}", path.display()));
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

pub fn export_public(dir: &Path) -> Result<PublicWalletView, String> {
    let path = keystore_path(dir);
    let stored = load_stored(dir)?;
    Ok(stored.public_view(&path))
}

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let params = Params::new(19_456, 2, 1, Some(32)).map_err(|e| format!("argon2 params: {e}"))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| format!("argon2 derive failed: {e}"))?;
    Ok(key)
}

fn encrypt_secret(passphrase: &str, plaintext: &[u8]) -> Result<EncryptedSecret, String> {
    let mut salt = [0u8; 16];
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let mut key = derive_key(passphrase, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("aes key: {e}"))?;
    key.zeroize();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| "encryption failed".to_owned())?;
    Ok(EncryptedSecret {
        kdf: "argon2id".into(),
        salt_b64: B64.encode(salt),
        nonce_b64: B64.encode(nonce_bytes),
        ciphertext_b64: B64.encode(ciphertext),
    })
}

fn decrypt_secret(passphrase: &str, secret: &EncryptedSecret) -> Result<Vec<u8>, String> {
    if secret.kdf != "argon2id" {
        return Err(format!("unsupported kdf {}", secret.kdf));
    }
    let salt = B64
        .decode(&secret.salt_b64)
        .map_err(|e| format!("salt: {e}"))?;
    let nonce_bytes = B64
        .decode(&secret.nonce_b64)
        .map_err(|e| format!("nonce: {e}"))?;
    let ciphertext = B64
        .decode(&secret.ciphertext_b64)
        .map_err(|e| format!("ciphertext: {e}"))?;
    if nonce_bytes.len() != 12 {
        return Err("invalid nonce length".to_owned());
    }
    let mut key = derive_key(passphrase, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("aes key: {e}"))?;
    key.zeroize();
    let nonce = Nonce::from_slice(&nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| "decryption failed (wrong passphrase?)".to_owned())
}

fn validate_passphrase(passphrase: &str) -> Result<(), String> {
    if passphrase.trim().len() < 8 {
        return Err("passphrase must be at least 8 characters".to_owned());
    }
    Ok(())
}

fn write_stored(dir: &Path, stored: &StoredHostWallet) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = keystore_path(dir);
    let content = serde_json::to_string_pretty(stored).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true).mode(0o600);
        let mut file = opts.open(&path).map_err(|e| e.to_string())?;
        file.write_all(content.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(unix))]
    {
        fs::write(&path, content).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn stored_from_account(
    network: Network,
    passphrase: &str,
    account: &WalletAccount,
) -> Result<StoredHostWallet, String> {
    validate_passphrase(passphrase)?;
    let mut secret_hex = account.private_key().to_hex();
    let encrypted = encrypt_secret(passphrase, secret_hex.as_bytes())?;
    secret_hex.zeroize();
    Ok(StoredHostWallet {
        schema: SCHEMA_ENCRYPTED_V1.to_owned(),
        network_id: network.network_id().to_owned(),
        network_name: network.human_name().to_owned(),
        status_label: network.status_label().to_owned(),
        address: account.address_string(),
        public_key_hex: account.public_key().to_hex(),
        encrypted,
        derivation_path: account.derivation_path().map(ToOwned::to_owned),
        warning: "Encrypted browser-host wallet (AES-256-GCM + Argon2id). Private key is never stored in plaintext and is never returned to the extension. Mnemonic recovery is CLI-only.".to_owned(),
    })
}

/// Create a new encrypted keystore. Fails if a file already exists.
pub fn create_encrypted_wallet(
    dir: &Path,
    network: Network,
    passphrase: &str,
    account: &WalletAccount,
) -> Result<StoredHostWallet, String> {
    let path = keystore_path(dir);
    if path.exists() {
        return Err(format!(
            "keystore already exists at {}; delete_wallet first or unlock",
            path.display()
        ));
    }
    let stored = stored_from_account(network, passphrase, account)?;
    write_stored(dir, &stored)?;
    Ok(stored)
}

pub fn unlock_wallet(
    dir: &Path,
    network: Network,
    passphrase: &str,
) -> Result<WalletAccount, String> {
    let stored = load_stored(dir)?;
    if stored.network_id != network.network_id() {
        return Err(format!(
            "keystore network {} does not match host network {}",
            stored.network_id,
            network.network_id()
        ));
    }
    let mut plain = decrypt_secret(passphrase, &stored.encrypted)?;
    let hex = std::str::from_utf8(&plain)
        .map_err(|_| "decrypted key is not utf8".to_owned())?
        .to_owned();
    plain.zeroize();
    let private_key = PrivateKey::from_hex(&hex).map_err(|e| e.to_string())?;
    let account =
        WalletAccount::from_private_key(network, private_key, stored.derivation_path.clone())
            .map_err(|e| e.to_string())?;
    if account.address_string() != stored.address {
        return Err("unlocked address does not match keystore metadata".to_owned());
    }
    let _ = Address::parse(&stored.address).map_err(|e| e.to_string())?;
    Ok(account)
}

/// Re-encrypt the keystore under a new passphrase (same key material).
pub fn change_passphrase(
    dir: &Path,
    network: Network,
    old_passphrase: &str,
    new_passphrase: &str,
) -> Result<PublicWalletView, String> {
    validate_passphrase(new_passphrase)?;
    let account = unlock_wallet(dir, network, old_passphrase)?;
    let stored = stored_from_account(network, new_passphrase, &account)?;
    write_stored(dir, &stored)?;
    Ok(stored.public_view(&keystore_path(dir)))
}

/// Verify passphrase, then delete the keystore file. Clears nothing else.
pub fn delete_wallet(dir: &Path, network: Network, passphrase: &str) -> Result<(), String> {
    let _account = unlock_wallet(dir, network, passphrase)?;
    let path = keystore_path(dir);
    fs::remove_file(&path).map_err(|e| format!("failed to remove {}: {e}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alvenqis_sdk_rust::{MnemonicWordCount, Network, WalletAccount, WalletDerivationPath};
    use tempfile::tempdir;

    #[test]
    fn create_unlock_roundtrip() {
        let dir = tempdir().expect("temp");
        let (account, _) =
            WalletAccount::generate(Network::MainnetCandidate, MnemonicWordCount::Twelve)
                .expect("gen");
        let address = account.address_string();
        create_encrypted_wallet(
            dir.path(),
            Network::MainnetCandidate,
            "test-pass-123",
            &account,
        )
        .expect("create");
        let unlocked =
            unlock_wallet(dir.path(), Network::MainnetCandidate, "test-pass-123").expect("unlock");
        assert_eq!(unlocked.address_string(), address);
        assert!(unlock_wallet(dir.path(), Network::MainnetCandidate, "wrong-pass").is_err());
    }

    #[test]
    fn change_passphrase_and_delete() {
        let dir = tempdir().expect("temp");
        let (account, mnemonic) =
            WalletAccount::generate(Network::MainnetCandidate, MnemonicWordCount::Twelve)
                .expect("gen");
        let address = account.address_string();
        create_encrypted_wallet(
            dir.path(),
            Network::MainnetCandidate,
            "old-pass-123",
            &account,
        )
        .expect("create");

        change_passphrase(
            dir.path(),
            Network::MainnetCandidate,
            "old-pass-123",
            "new-pass-456",
        )
        .expect("change");
        assert!(unlock_wallet(dir.path(), Network::MainnetCandidate, "old-pass-123").is_err());
        let unlocked =
            unlock_wallet(dir.path(), Network::MainnetCandidate, "new-pass-456").expect("new");
        assert_eq!(unlocked.address_string(), address);

        // CLI-style recovery: re-import same mnemonic after delete.
        delete_wallet(dir.path(), Network::MainnetCandidate, "new-pass-456").expect("delete");
        assert!(!keystore_exists(dir.path()));
        let recovered = WalletAccount::from_mnemonic(
            Network::MainnetCandidate,
            &mnemonic.phrase,
            "",
            WalletDerivationPath::default(),
        )
        .expect("import mnemonic");
        create_encrypted_wallet(
            dir.path(),
            Network::MainnetCandidate,
            "recover-pass-1",
            &recovered,
        )
        .expect("recreate");
        assert_eq!(
            unlock_wallet(dir.path(), Network::MainnetCandidate, "recover-pass-1")
                .expect("unlock")
                .address_string(),
            address
        );
    }

    #[test]
    fn export_public_has_no_secrets() {
        let dir = tempdir().expect("temp");
        let (account, _) =
            WalletAccount::generate(Network::MainnetCandidate, MnemonicWordCount::Twelve)
                .expect("gen");
        create_encrypted_wallet(
            dir.path(),
            Network::MainnetCandidate,
            "test-pass-123",
            &account,
        )
        .expect("create");
        let public = export_public(dir.path()).expect("export");
        let json = serde_json::to_string(&public).expect("json");
        assert!(json.contains(&account.address_string()));
        assert!(!json.contains("private"));
        assert!(!json.contains("mnemonic"));
        assert!(!json.contains("ciphertext"));
    }
}
