use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

pub const HASH_SIZE: usize = 32;

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash(pub [u8; HASH_SIZE]);

impl Hash {
    pub const fn zero() -> Self {
        Self([0; HASH_SIZE])
    }

    pub const fn as_bytes(&self) -> &[u8; HASH_SIZE] {
        &self.0
    }

    pub fn to_hex(self) -> String {
        hash_to_hex(&self)
    }

    pub fn from_hex(input: &str) -> Result<Self, String> {
        let trimmed = input.trim();
        if trimmed.len() != HASH_SIZE * 2 {
            return Err(format!(
                "expected {} hex chars, got {}",
                HASH_SIZE * 2,
                trimmed.len()
            ));
        }

        let mut bytes = [0_u8; HASH_SIZE];
        let encoded = trimmed.as_bytes();
        for index in 0..HASH_SIZE {
            bytes[index] = (decode_hex_nibble(encoded[index * 2])? << 4)
                | decode_hex_nibble(encoded[index * 2 + 1])?;
        }
        Ok(Self(bytes))
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

pub fn sha256(data: &[u8]) -> Hash {
    let digest = Sha256::digest(data);
    let mut bytes = [0_u8; HASH_SIZE];
    bytes.copy_from_slice(&digest);
    Hash(bytes)
}

pub fn blake3_hash(data: &[u8]) -> Hash {
    let digest = blake3::hash(data);
    let mut bytes = [0_u8; HASH_SIZE];
    bytes.copy_from_slice(digest.as_bytes());
    Hash(bytes)
}

pub fn double_sha256(data: &[u8]) -> Hash {
    let first = sha256(data);
    sha256(first.as_bytes())
}

pub fn hash_to_hex(hash: &Hash) -> String {
    let mut output = String::with_capacity(HASH_SIZE * 2);
    for byte in hash.0 {
        output.push(hex_char(byte >> 4));
        output.push(hex_char(byte & 0x0f));
    }
    output
}

pub fn leading_zero_bits(hash: &Hash) -> u32 {
    let mut total = 0_u32;
    for byte in hash.0 {
        if byte == 0 {
            total += 8;
        } else {
            total += byte.leading_zeros();
            break;
        }
    }
    total
}

fn hex_char(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => unreachable!("nibble value must be between 0 and 15"),
    }
}

fn decode_hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("invalid hex character {}", byte as char)),
    }
}
