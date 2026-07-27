use crate::protocol::network::Network;
use serde::Serialize;

pub const ADDRESS_STANDARD_ID: &str = "alvenqis-address-bech32m-ed25519-v1";
pub const SIGNATURE_STANDARD_ID: &str = "alvenqis-signature-ed25519-v1";
pub const KEY_DERIVATION_POLICY_ID: &str = "alvenqis-key-ed25519-v1";
pub const WALLET_SEED_STANDARD_ID: &str = "alvenqis-wallet-bip39-slip10-v1";
pub const ADDRESS_ENCODING: &str = "Bech32m";
pub const ADDRESS_CHECKSUM_RULE: &str = "Bech32m canonical lowercase checksum";
pub const ADDRESS_PAYLOAD_LAYOUT: &str = "version-byte || ed25519-public-key";
pub const ADDRESS_CANONICAL_CASE: &str = "lowercase";
pub const ADDRESS_VERSION: u8 = 0;
pub const ADDRESS_PAYLOAD_SIZE: usize = 33;
pub const PUBLIC_KEY_SCHEME: &str = "ed25519";
pub const PRIVATE_KEY_FORMAT: &str = "raw 32-byte ed25519 signing seed";
pub const SIGNATURE_SCHEME: &str = "ed25519";
pub const SIGNATURE_IMPLEMENTATION_STATUS: &str =
    "Prototype implementation on frozen launch standard";
pub const TX_SIGNING_DOMAIN: &str = "alvenqis-tx-ed25519-v1";
pub const KEY_DERIVATION_RULE: &str =
    "raw ed25519 signing seed -> ed25519 public key -> Bech32m address payload";
pub const HD_DERIVATION_DIRECTION: &str =
    "SLIP-0010 ed25519 with BIP44-compatible all-hardened account paths";
pub const HD_DERIVATION_SCHEME: &str = "BIP39 mnemonic -> BIP39 seed -> SLIP-0010 ed25519";
pub const HD_DERIVATION_PURPOSE: u32 = 44;
pub const HD_DERIVATION_COIN_TYPE: u32 = 7_330;
pub const HD_COIN_TYPE_POLICY: &str =
    "coin_type 7330 is the current provisional launch constant until a final SLIP-44 assignment exists";
pub const HD_DERIVATION_PATH_TEMPLATE: &str = "m/44'/7330'/account'/change'/address_index'";
pub const HD_DERIVATION_PATH_POLICY: &str =
    "all path segments are hardened for ed25519 compatibility";
pub const WALLET_SEED_POLICY_STATUS: &str =
    "Frozen launch wallet seed and derivation rule under TM-111";
pub const MNEMONIC_LANGUAGE: &str = "English";
pub const MNEMONIC_WORD_COUNTS: &str = "12 or 24 words";
pub const MNEMONIC_PASSPHRASE_POLICY: &str =
    "BIP39 passphrase supported and optional; empty string is the default";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct AddressStandard {
    pub standard_id: &'static str,
    pub network_id: &'static str,
    pub network_name: &'static str,
    pub address_prefix: &'static str,
    pub encoding: &'static str,
    pub checksum_rule: &'static str,
    pub canonical_case: &'static str,
    pub payload_version: u8,
    pub payload_size_bytes: usize,
    pub payload_layout: &'static str,
    pub public_key_scheme: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SigningStandard {
    pub standard_id: &'static str,
    pub public_key_scheme: &'static str,
    pub private_key_format: &'static str,
    pub signature_scheme: &'static str,
    pub public_key_size_bytes: usize,
    pub signature_size_bytes: usize,
    pub tx_signing_domain: &'static str,
    pub implementation_status: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct WalletSeedStandard {
    pub standard_id: &'static str,
    pub mnemonic_language: &'static str,
    pub mnemonic_word_counts: &'static str,
    pub mnemonic_passphrase_policy: &'static str,
    pub derivation_scheme: &'static str,
    pub derivation_path_template: &'static str,
    pub derivation_path_policy: &'static str,
    pub coin_type: u32,
    pub coin_type_policy: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct KeyDerivationPolicy {
    pub policy_id: &'static str,
    pub direct_key_rule: &'static str,
    pub wallet_seed_standard_id: &'static str,
    pub hd_derivation_direction: &'static str,
    pub hd_derivation_scheme: &'static str,
    pub hd_derivation_path_template: &'static str,
    pub hd_derivation_path_policy: &'static str,
    pub hd_derivation_coin_type: u32,
    pub hd_coin_type_policy: &'static str,
    pub mnemonic_language: &'static str,
    pub mnemonic_word_counts: &'static str,
    pub mnemonic_passphrase_policy: &'static str,
    pub wallet_seed_policy_status: &'static str,
}

pub const fn launch_address_standard(network: Network) -> AddressStandard {
    AddressStandard {
        standard_id: ADDRESS_STANDARD_ID,
        network_id: network.network_id(),
        network_name: network.human_name(),
        address_prefix: network.address_prefix(),
        encoding: ADDRESS_ENCODING,
        checksum_rule: ADDRESS_CHECKSUM_RULE,
        canonical_case: ADDRESS_CANONICAL_CASE,
        payload_version: ADDRESS_VERSION,
        payload_size_bytes: ADDRESS_PAYLOAD_SIZE,
        payload_layout: ADDRESS_PAYLOAD_LAYOUT,
        public_key_scheme: PUBLIC_KEY_SCHEME,
    }
}

pub const fn launch_signing_standard() -> SigningStandard {
    SigningStandard {
        standard_id: SIGNATURE_STANDARD_ID,
        public_key_scheme: PUBLIC_KEY_SCHEME,
        private_key_format: PRIVATE_KEY_FORMAT,
        signature_scheme: SIGNATURE_SCHEME,
        public_key_size_bytes: 32,
        signature_size_bytes: 64,
        tx_signing_domain: TX_SIGNING_DOMAIN,
        implementation_status: SIGNATURE_IMPLEMENTATION_STATUS,
    }
}

pub const fn launch_wallet_seed_standard() -> WalletSeedStandard {
    WalletSeedStandard {
        standard_id: WALLET_SEED_STANDARD_ID,
        mnemonic_language: MNEMONIC_LANGUAGE,
        mnemonic_word_counts: MNEMONIC_WORD_COUNTS,
        mnemonic_passphrase_policy: MNEMONIC_PASSPHRASE_POLICY,
        derivation_scheme: HD_DERIVATION_SCHEME,
        derivation_path_template: HD_DERIVATION_PATH_TEMPLATE,
        derivation_path_policy: HD_DERIVATION_PATH_POLICY,
        coin_type: HD_DERIVATION_COIN_TYPE,
        coin_type_policy: HD_COIN_TYPE_POLICY,
    }
}

pub const fn launch_key_derivation_policy() -> KeyDerivationPolicy {
    KeyDerivationPolicy {
        policy_id: KEY_DERIVATION_POLICY_ID,
        direct_key_rule: KEY_DERIVATION_RULE,
        wallet_seed_standard_id: WALLET_SEED_STANDARD_ID,
        hd_derivation_direction: HD_DERIVATION_DIRECTION,
        hd_derivation_scheme: HD_DERIVATION_SCHEME,
        hd_derivation_path_template: HD_DERIVATION_PATH_TEMPLATE,
        hd_derivation_path_policy: HD_DERIVATION_PATH_POLICY,
        hd_derivation_coin_type: HD_DERIVATION_COIN_TYPE,
        hd_coin_type_policy: HD_COIN_TYPE_POLICY,
        mnemonic_language: MNEMONIC_LANGUAGE,
        mnemonic_word_counts: MNEMONIC_WORD_COUNTS,
        mnemonic_passphrase_policy: MNEMONIC_PASSPHRASE_POLICY,
        wallet_seed_policy_status: WALLET_SEED_POLICY_STATUS,
    }
}
