use crate::protocol::errors::{AlvenqisError, Result};
use crate::protocol::seed::{derive_private_key_from_mnemonic, WalletDerivationPath};
use crate::protocol::standards::{
    launch_key_derivation_policy, launch_signing_standard, launch_wallet_seed_standard,
    KeyDerivationPolicy, SigningStandard, WalletSeedStandard,
};
use ed25519_dalek::{Signature as DalekSignature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PublicKey([u8; 32]);

#[derive(Clone, PartialEq, Eq)]
pub struct Signature([u8; 64]);

pub struct PrivateKey([u8; 32]);

impl PublicKey {
    pub const SIZE: usize = 32;

    pub fn from_bytes(bytes: [u8; Self::SIZE]) -> Result<Self> {
        VerifyingKey::from_bytes(&bytes)
            .map_err(|error| AlvenqisError::InvalidKey(error.to_string()))?;
        Ok(Self(bytes))
    }

    pub fn from_hex(input: &str) -> Result<Self> {
        Self::from_bytes(parse_hex_array::<{ Self::SIZE }>(input)?)
    }

    pub const fn as_bytes(&self) -> &[u8; Self::SIZE] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        bytes_to_hex(&self.0)
    }

    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<()> {
        let verifying_key = VerifyingKey::from_bytes(&self.0)
            .map_err(|error| AlvenqisError::InvalidKey(error.to_string()))?;
        let dalek_signature = signature.to_dalek()?;
        verifying_key
            .verify(message, &dalek_signature)
            .map_err(|error| AlvenqisError::InvalidSignature(error.to_string()))
    }

    pub const fn standard() -> SigningStandard {
        launch_signing_standard()
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_hex(&value).map_err(D::Error::custom)
    }
}

impl Signature {
    pub const SIZE: usize = 64;

    pub fn from_bytes(bytes: [u8; Self::SIZE]) -> Result<Self> {
        DalekSignature::from_bytes(&bytes);
        Ok(Self(bytes))
    }

    pub fn from_hex(input: &str) -> Result<Self> {
        Self::from_bytes(parse_hex_array::<{ Self::SIZE }>(input)?)
    }

    pub const fn as_bytes(&self) -> &[u8; Self::SIZE] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        bytes_to_hex(&self.0)
    }

    fn to_dalek(&self) -> Result<DalekSignature> {
        Ok(DalekSignature::from_bytes(&self.0))
    }

    pub const fn standard() -> SigningStandard {
        launch_signing_standard()
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_hex(&value).map_err(D::Error::custom)
    }
}

impl PrivateKey {
    pub const SIZE: usize = 32;

    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self(signing_key.to_bytes())
    }

    pub fn from_bytes(bytes: [u8; Self::SIZE]) -> Self {
        Self(bytes)
    }

    pub fn from_seed_bytes(bytes: [u8; Self::SIZE]) -> Self {
        Self::from_bytes(bytes)
    }

    pub fn from_hex(input: &str) -> Result<Self> {
        Ok(Self::from_bytes(parse_hex_array::<{ Self::SIZE }>(input)?))
    }

    pub fn from_mnemonic(
        phrase: &str,
        passphrase: &str,
        path: WalletDerivationPath,
    ) -> Result<Self> {
        derive_private_key_from_mnemonic(phrase, passphrase, path)
    }

    pub const fn as_bytes(&self) -> &[u8; Self::SIZE] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        bytes_to_hex(&self.0)
    }

    pub fn public_key(&self) -> PublicKey {
        let signing_key = SigningKey::from_bytes(&self.0);
        PublicKey(signing_key.verifying_key().to_bytes())
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        let signing_key = SigningKey::from_bytes(&self.0);
        Signature(signing_key.sign(message).to_bytes())
    }

    pub const fn standard() -> SigningStandard {
        launch_signing_standard()
    }

    pub const fn derivation_policy() -> KeyDerivationPolicy {
        launch_key_derivation_policy()
    }

    pub const fn seed_standard() -> WalletSeedStandard {
        launch_wallet_seed_standard()
    }
}

impl fmt::Debug for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PrivateKey(REDACTED)")
    }
}

fn parse_hex_array<const N: usize>(input: &str) -> Result<[u8; N]> {
    let trimmed = input.trim();
    if trimmed.len() != N * 2 {
        return Err(AlvenqisError::InvalidHex(format!(
            "expected {} hex chars, got {}",
            N * 2,
            trimmed.len()
        )));
    }

    let mut output = [0_u8; N];
    let bytes = trimmed.as_bytes();
    for index in 0..N {
        output[index] =
            (decode_hex_nibble(bytes[index * 2])? << 4) | decode_hex_nibble(bytes[index * 2 + 1])?;
    }
    Ok(output)
}

fn decode_hex_nibble(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(AlvenqisError::InvalidHex(format!(
            "invalid hex character {}",
            byte as char
        ))),
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(hex_char(byte >> 4));
        output.push(hex_char(byte & 0x0f));
    }
    output
}

fn hex_char(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => unreachable!("nibble value must be between 0 and 15"),
    }
}
