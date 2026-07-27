use crate::protocol::errors::{AlvenqisError, Result};
use crate::protocol::signing::PrivateKey;
use crate::protocol::standards::{
    launch_key_derivation_policy, launch_wallet_seed_standard, KeyDerivationPolicy,
    WalletSeedStandard, HD_DERIVATION_COIN_TYPE, HD_DERIVATION_PURPOSE,
};
use bip39::{Language, Mnemonic};
use hmac::{Hmac, Mac};
use sha2::Sha512;
use std::fmt;

type HmacSha512 = Hmac<Sha512>;

const SLIP10_MASTER_KEY: &[u8] = b"ed25519 seed";
const HARDENED_OFFSET: u32 = 1 << 31;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MnemonicWordCount {
    Twelve,
    TwentyFour,
}

impl MnemonicWordCount {
    pub const fn as_usize(self) -> usize {
        match self {
            Self::Twelve => 12,
            Self::TwentyFour => 24,
        }
    }

    pub fn from_u16(value: u16) -> Result<Self> {
        match value {
            12 => Ok(Self::Twelve),
            24 => Ok(Self::TwentyFour),
            _ => Err(AlvenqisError::InvalidMnemonic(format!(
                "unsupported mnemonic word count {value}; expected 12 or 24"
            ))),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WalletDerivationPath {
    pub account: u32,
    pub change: u32,
    pub address_index: u32,
}

impl WalletDerivationPath {
    pub const fn new(account: u32, change: u32, address_index: u32) -> Self {
        Self {
            account,
            change,
            address_index,
        }
    }

    pub const fn standard() -> WalletSeedStandard {
        launch_wallet_seed_standard()
    }

    pub const fn key_policy() -> KeyDerivationPolicy {
        launch_key_derivation_policy()
    }

    pub fn hardened_segments(&self) -> Result<[u32; 5]> {
        Ok([
            hardened(HD_DERIVATION_PURPOSE)?,
            hardened(HD_DERIVATION_COIN_TYPE)?,
            hardened(self.account)?,
            hardened(self.change)?,
            hardened(self.address_index)?,
        ])
    }
}

impl Default for WalletDerivationPath {
    fn default() -> Self {
        Self::new(0, 0, 0)
    }
}

impl fmt::Debug for WalletDerivationPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl fmt::Display for WalletDerivationPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "m/{}'/{}'/{}'/{}'/{}'",
            HD_DERIVATION_PURPOSE,
            HD_DERIVATION_COIN_TYPE,
            self.account,
            self.change,
            self.address_index
        )
    }
}

pub fn generate_mnemonic(word_count: MnemonicWordCount) -> Result<String> {
    let mnemonic = Mnemonic::generate_in(Language::English, word_count.as_usize())
        .map_err(|error| AlvenqisError::InvalidMnemonic(error.to_string()))?;
    Ok(mnemonic.to_string())
}

pub fn normalize_mnemonic(phrase: &str) -> Result<String> {
    let mnemonic = Mnemonic::parse_in_normalized(Language::English, phrase)
        .map_err(|error| AlvenqisError::InvalidMnemonic(error.to_string()))?;
    Ok(mnemonic.to_string())
}

pub fn derive_private_key_from_mnemonic(
    phrase: &str,
    passphrase: &str,
    path: WalletDerivationPath,
) -> Result<PrivateKey> {
    let mnemonic = Mnemonic::parse_in_normalized(Language::English, phrase)
        .map_err(|error| AlvenqisError::InvalidMnemonic(error.to_string()))?;
    let seed = mnemonic.to_seed_normalized(passphrase);
    let mut node = derive_master_node(&seed)?;
    for segment in path.hardened_segments()? {
        node = derive_child_node(&node, segment)?;
    }
    Ok(PrivateKey::from_seed_bytes(node.key))
}

fn hardened(index: u32) -> Result<u32> {
    index.checked_add(HARDENED_OFFSET).ok_or_else(|| {
        AlvenqisError::InvalidDerivationPath(format!("path segment {index} is too large"))
    })
}

fn derive_master_node(seed: &[u8; 64]) -> Result<DerivedNode> {
    derive_node(SLIP10_MASTER_KEY, seed)
}

fn derive_child_node(node: &DerivedNode, index: u32) -> Result<DerivedNode> {
    let mut data = [0_u8; 1 + 32 + 4];
    data[1..33].copy_from_slice(&node.key);
    data[33..].copy_from_slice(&index.to_be_bytes());
    derive_node(&node.chain_code, &data)
}

fn derive_node(key: &[u8], data: &[u8]) -> Result<DerivedNode> {
    let mut mac = HmacSha512::new_from_slice(key)
        .map_err(|error| AlvenqisError::InvalidDerivationPath(error.to_string()))?;
    mac.update(data);
    let digest = mac.finalize().into_bytes();
    let mut derived_key = [0_u8; 32];
    let mut chain_code = [0_u8; 32];
    derived_key.copy_from_slice(&digest[..32]);
    chain_code.copy_from_slice(&digest[32..]);
    Ok(DerivedNode {
        key: derived_key,
        chain_code,
    })
}

struct DerivedNode {
    key: [u8; 32],
    chain_code: [u8; 32],
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn same_mnemonic_and_path_produce_same_private_key() {
        let path = WalletDerivationPath::default();
        let first =
            derive_private_key_from_mnemonic(TEST_MNEMONIC, "", path).expect("first derivation");
        let second =
            derive_private_key_from_mnemonic(TEST_MNEMONIC, "", path).expect("second derivation");
        assert_eq!(first.to_hex(), second.to_hex());
    }

    #[test]
    fn different_path_produces_different_private_key() {
        let base =
            derive_private_key_from_mnemonic(TEST_MNEMONIC, "", WalletDerivationPath::default())
                .expect("base derivation");
        let alternate =
            derive_private_key_from_mnemonic(TEST_MNEMONIC, "", WalletDerivationPath::new(0, 0, 1))
                .expect("alternate derivation");
        assert_ne!(base.to_hex(), alternate.to_hex());
    }

    #[test]
    fn invalid_mnemonic_is_rejected() {
        let error = derive_private_key_from_mnemonic(
            "not a valid alvenqis mnemonic",
            "",
            WalletDerivationPath::default(),
        )
        .expect_err("invalid mnemonic must fail");
        assert!(matches!(error, AlvenqisError::InvalidMnemonic(_)));
    }
}
