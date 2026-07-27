use crate::errors::{AlvenqisError, Result};
use crate::network::Network;
use crate::signing::PublicKey;
use crate::standards::{
    launch_address_standard, AddressStandard, ADDRESS_PAYLOAD_SIZE, ADDRESS_VERSION,
};
use bech32::{self, FromBase32, ToBase32, Variant};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Address {
    network: Network,
    payload: [u8; ADDRESS_PAYLOAD_SIZE],
}

impl Address {
    pub fn from_public_key_for_network(public_key: &PublicKey, network: Network) -> Self {
        let mut payload = [0_u8; ADDRESS_PAYLOAD_SIZE];
        payload[0] = ADDRESS_VERSION;
        payload[1..].copy_from_slice(public_key.as_bytes());
        Self { network, payload }
    }

    pub fn parse(input: &str) -> Result<Self> {
        if input != input.to_lowercase() {
            return Err(AlvenqisError::InvalidAddress(
                "address must use lowercase canonical encoding".to_owned(),
            ));
        }
        let (hrp, data, variant) = bech32::decode(input)
            .map_err(|error| AlvenqisError::InvalidAddress(error.to_string()))?;
        let network = Network::from_address_prefix(&hrp).ok_or_else(|| {
            AlvenqisError::InvalidAddress(format!("unsupported address prefix {hrp}"))
        })?;
        if variant != Variant::Bech32m {
            return Err(AlvenqisError::InvalidAddress(
                "address must use Bech32m checksum".to_owned(),
            ));
        }

        let bytes = Vec::<u8>::from_base32(&data)
            .map_err(|error| AlvenqisError::InvalidAddress(error.to_string()))?;
        if bytes.len() != ADDRESS_PAYLOAD_SIZE {
            return Err(AlvenqisError::InvalidAddress(format!(
                "expected {ADDRESS_PAYLOAD_SIZE} payload bytes, got {}",
                bytes.len()
            )));
        }
        if bytes[0] != ADDRESS_VERSION {
            return Err(AlvenqisError::InvalidAddress(format!(
                "unsupported address version {}",
                bytes[0]
            )));
        }

        let mut payload = [0_u8; ADDRESS_PAYLOAD_SIZE];
        payload.copy_from_slice(&bytes);
        Ok(Self { network, payload })
    }

    pub fn to_public_key(&self) -> Result<PublicKey> {
        let mut bytes = [0_u8; PublicKey::SIZE];
        bytes.copy_from_slice(&self.payload[1..]);
        PublicKey::from_bytes(bytes)
    }

    pub const fn version(&self) -> u8 {
        self.payload[0]
    }

    pub const fn network(&self) -> Network {
        self.network
    }

    pub const fn payload(&self) -> &[u8; ADDRESS_PAYLOAD_SIZE] {
        &self.payload
    }

    pub const fn standard(&self) -> AddressStandard {
        launch_address_standard(self.network)
    }

    pub fn matches_public_key(&self, public_key: &PublicKey) -> bool {
        &self.payload[1..] == public_key.as_bytes()
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let encoded = bech32::encode(
            self.network.address_prefix(),
            self.payload.to_base32(),
            Variant::Bech32m,
        )
        .map_err(|_| fmt::Error)?;
        write!(f, "{encoded}")
    }
}

impl FromStr for Address {
    type Err = AlvenqisError;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}
