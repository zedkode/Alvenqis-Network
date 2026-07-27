use crate::address::Address;
use crate::amount::Amount;
use crate::constants::{MAX_SUPPLY_ATOMIC, TX_SIGNING_DOMAIN_PREFIX};
use crate::crypto::{double_sha256, Hash};
use crate::errors::{AlvenqisError, Result};
use crate::network::Network;
use crate::signing::{PrivateKey, PublicKey, Signature};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsignedTransaction {
    pub version: u32,
    pub nonce: u64,
    pub from: Option<String>,
    pub to: String,
    pub amount: Amount,
    #[serde(rename = "max_fee", alias = "fee")]
    pub max_fee: Amount,
    #[serde(default)]
    pub priority_fee: Amount,
    pub memo_hash: Option<Hash>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub version: u32,
    pub nonce: u64,
    pub from: Option<String>,
    pub to: String,
    pub amount: Amount,
    #[serde(rename = "max_fee", alias = "fee")]
    pub max_fee: Amount,
    #[serde(default)]
    pub priority_fee: Amount,
    pub memo_hash: Option<Hash>,
    #[serde(default)]
    pub sender_public_key: Option<PublicKey>,
    #[serde(default)]
    pub signature: Option<Signature>,
}

impl Transaction {
    // Transaction fields are the wire shape; keep positional constructors for call-site clarity.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        version: u32,
        nonce: u64,
        from: Option<String>,
        to: String,
        amount: Amount,
        max_fee: Amount,
        priority_fee: Amount,
        memo_hash: Option<Hash>,
    ) -> Result<Self> {
        let tx = Self {
            version,
            nonce,
            from,
            to,
            amount,
            max_fee,
            priority_fee,
            memo_hash,
            sender_public_key: None,
            signature: None,
        };
        tx.validate_shape()?;
        Ok(tx)
    }

    // Same rationale as `new` — network + key + fee fields stay explicit at call sites.
    #[allow(clippy::too_many_arguments)]
    pub fn new_signed(
        version: u32,
        nonce: u64,
        network: Network,
        private_key: &PrivateKey,
        to: String,
        amount: Amount,
        max_fee: Amount,
        priority_fee: Amount,
        memo_hash: Option<Hash>,
    ) -> Result<Self> {
        let from =
            Address::from_public_key_for_network(&private_key.public_key(), network).to_string();
        let mut tx = Self::new(
            version,
            nonce,
            Some(from),
            to,
            amount,
            max_fee,
            priority_fee,
            memo_hash,
        )?;
        tx.sign(private_key)?;
        Ok(tx)
    }

    pub fn coinbase(height: u64, to: String, amount: Amount) -> Result<Self> {
        Self::new(
            1,
            height,
            None,
            to,
            amount,
            Amount::ZERO,
            Amount::ZERO,
            None,
        )
    }

    pub fn is_coinbase(&self) -> bool {
        self.from.is_none() && self.sender_public_key.is_none() && self.signature.is_none()
    }

    pub fn unsigned_payload(&self) -> UnsignedTransaction {
        UnsignedTransaction {
            version: self.version,
            nonce: self.nonce,
            from: self.from.clone(),
            to: self.to.clone(),
            amount: self.amount,
            max_fee: self.max_fee,
            priority_fee: self.priority_fee,
            memo_hash: self.memo_hash,
        }
    }

    pub fn unsigned_payload_bytes(&self) -> Vec<u8> {
        let payload = self.unsigned_payload();
        let mut bytes = Vec::new();
        let network_id = self
            .network()
            .map(Network::network_id)
            .unwrap_or("unknown-network");
        bytes.extend_from_slice(TX_SIGNING_DOMAIN_PREFIX);
        push_string(&mut bytes, network_id);
        bytes.extend_from_slice(&payload.version.to_le_bytes());
        bytes.extend_from_slice(&payload.nonce.to_le_bytes());
        push_optional_string(&mut bytes, payload.from.as_deref());
        push_string(&mut bytes, &payload.to);
        bytes.extend_from_slice(&payload.amount.as_atomic().to_le_bytes());
        bytes.extend_from_slice(&payload.max_fee.as_atomic().to_le_bytes());
        bytes.extend_from_slice(&payload.priority_fee.as_atomic().to_le_bytes());
        match payload.memo_hash {
            Some(hash) => {
                bytes.push(1);
                bytes.extend_from_slice(hash.as_bytes());
            }
            None => bytes.push(0),
        }
        bytes
    }

    pub fn encode(&self) -> Vec<u8> {
        if self.sender_public_key.is_none() && self.signature.is_none() {
            return self.legacy_encode();
        }

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.unsigned_payload_bytes());
        match &self.sender_public_key {
            Some(public_key) => {
                bytes.push(1);
                bytes.extend_from_slice(public_key.as_bytes());
            }
            None => bytes.push(0),
        }
        match &self.signature {
            Some(signature) => {
                bytes.push(1);
                bytes.extend_from_slice(signature.as_bytes());
            }
            None => bytes.push(0),
        }
        bytes
    }

    pub fn tx_hash(&self) -> Hash {
        double_sha256(&self.encode())
    }

    pub fn txid(&self) -> Hash {
        self.tx_hash()
    }

    pub fn effective_priority_fee(&self, base_fee: Amount) -> Result<Amount> {
        if self.is_coinbase() {
            return Ok(Amount::ZERO);
        }
        let available_tip = self.max_fee.checked_sub(base_fee).map_err(|_| {
            AlvenqisError::InvalidFee("max fee is below the block base fee".to_owned())
        })?;
        Ok(if self.priority_fee > available_tip {
            available_tip
        } else {
            self.priority_fee
        })
    }

    pub fn effective_fee(&self, base_fee: Amount) -> Result<Amount> {
        if self.is_coinbase() {
            return Ok(Amount::ZERO);
        }
        base_fee.checked_add(self.effective_priority_fee(base_fee)?)
    }

    pub fn total_debit(&self, base_fee: Amount) -> Result<Amount> {
        self.amount.checked_add(self.effective_fee(base_fee)?)
    }

    pub fn validate_fee_against_base_fee(&self, base_fee: Amount) -> Result<()> {
        if self.is_coinbase() {
            return Ok(());
        }
        if self.max_fee < base_fee {
            return Err(AlvenqisError::InvalidFee(
                "max fee is below the block base fee".to_owned(),
            ));
        }
        let _ = self.effective_priority_fee(base_fee)?;
        Ok(())
    }

    pub fn sign(&mut self, private_key: &PrivateKey) -> Result<()> {
        if self.from.is_none() {
            return Err(AlvenqisError::InvalidTransaction(
                "coinbase transactions cannot be signed".to_owned(),
            ));
        }

        let public_key = private_key.public_key();
        let sender_address = Address::parse(self.from.as_deref().ok_or_else(|| {
            AlvenqisError::InvalidTransaction("non-coinbase transaction requires sender".to_owned())
        })?)?;
        let expected_from =
            Address::from_public_key_for_network(&public_key, sender_address.network()).to_string();
        if self.from.as_deref() != Some(expected_from.as_str()) {
            return Err(AlvenqisError::InvalidTransaction(
                "sender address does not match signing key".to_owned(),
            ));
        }

        let signature = private_key.sign(&self.unsigned_payload_bytes());
        self.sender_public_key = Some(public_key);
        self.signature = Some(signature);
        self.validate_shape()?;
        Ok(())
    }

    pub fn verify(&self) -> Result<()> {
        self.validate_shape()?;
        if self.is_coinbase() {
            return Ok(());
        }

        let from = self.from.as_deref().ok_or_else(|| {
            AlvenqisError::InvalidTransaction("non-coinbase transaction requires sender".to_owned())
        })?;
        let sender_public_key = self.sender_public_key.as_ref().ok_or_else(|| {
            AlvenqisError::InvalidTransaction(
                "non-coinbase transaction requires sender public key".to_owned(),
            )
        })?;
        let signature = self.signature.as_ref().ok_or_else(|| {
            AlvenqisError::InvalidTransaction(
                "non-coinbase transaction requires signature".to_owned(),
            )
        })?;

        let sender_address = Address::parse(from)?;
        let recipient_address = Address::parse(&self.to)?;
        if sender_address.network() != recipient_address.network() {
            return Err(AlvenqisError::InvalidNetwork {
                expected: recipient_address.network().network_id().to_owned(),
                actual: sender_address.network().network_id().to_owned(),
            });
        }
        let derived_address =
            Address::from_public_key_for_network(sender_public_key, sender_address.network())
                .to_string();
        if derived_address != from {
            return Err(AlvenqisError::InvalidTransaction(
                "sender address does not match sender public key".to_owned(),
            ));
        }

        sender_public_key.verify(&self.unsigned_payload_bytes(), signature)
    }

    pub fn network(&self) -> Result<Network> {
        let recipient = Address::parse(&self.to)?;
        if let Some(from) = &self.from {
            let sender = Address::parse(from)?;
            if sender.network() != recipient.network() {
                return Err(AlvenqisError::InvalidNetwork {
                    expected: recipient.network().network_id().to_owned(),
                    actual: sender.network().network_id().to_owned(),
                });
            }
        }
        Ok(recipient.network())
    }

    fn validate_shape(&self) -> Result<()> {
        if self.to.trim().is_empty() {
            return Err(AlvenqisError::InvalidTransaction(
                "recipient cannot be empty".to_owned(),
            ));
        }
        Address::parse(&self.to)?;
        // Coinbase may be zero when subsidy is exhausted and no miner tips (audit CR-H09).
        // Non-coinbase transfers must always move a positive amount.
        if self.amount == Amount::ZERO && !self.is_coinbase() {
            return Err(AlvenqisError::ZeroAmountTransaction);
        }
        if self.max_fee.as_atomic() > MAX_SUPPLY_ATOMIC {
            return Err(AlvenqisError::InvalidFee(
                "max fee exceeds max supply bounds".to_owned(),
            ));
        }
        if self.priority_fee.as_atomic() > MAX_SUPPLY_ATOMIC {
            return Err(AlvenqisError::InvalidFee(
                "priority fee exceeds max supply bounds".to_owned(),
            ));
        }
        if self.is_coinbase() {
            if self.max_fee != Amount::ZERO || self.priority_fee != Amount::ZERO {
                return Err(AlvenqisError::InvalidTransaction(
                    "coinbase max fee and priority fee must be zero".to_owned(),
                ));
            }
            return Ok(());
        }
        if self.priority_fee > self.max_fee {
            return Err(AlvenqisError::InvalidFee(
                "priority fee cannot exceed max fee".to_owned(),
            ));
        }

        if self
            .from
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(AlvenqisError::InvalidTransaction(
                "sender cannot be empty".to_owned(),
            ));
        }
        if self.sender_public_key.is_some() ^ self.signature.is_some() {
            return Err(AlvenqisError::InvalidTransaction(
                "sender public key and signature must both be present or both be absent".to_owned(),
            ));
        }
        if let Some(from) = &self.from {
            let sender = Address::parse(from)?;
            let recipient = Address::parse(&self.to)?;
            if sender.network() != recipient.network() {
                return Err(AlvenqisError::InvalidNetwork {
                    expected: recipient.network().network_id().to_owned(),
                    actual: sender.network().network_id().to_owned(),
                });
            }
        }
        Ok(())
    }

    fn legacy_encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&self.nonce.to_le_bytes());
        push_optional_string(&mut bytes, self.from.as_deref());
        push_string(&mut bytes, &self.to);
        bytes.extend_from_slice(&self.amount.as_atomic().to_le_bytes());
        bytes.extend_from_slice(&self.max_fee.as_atomic().to_le_bytes());
        bytes.extend_from_slice(&self.priority_fee.as_atomic().to_le_bytes());
        match self.memo_hash {
            Some(hash) => {
                bytes.push(1);
                bytes.extend_from_slice(hash.as_bytes());
            }
            None => bytes.push(0),
        }
        bytes
    }
}

fn push_string(buffer: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    buffer.extend_from_slice(bytes);
}

fn push_optional_string(buffer: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(text) => {
            buffer.push(1);
            push_string(buffer, text);
        }
        None => buffer.push(0),
    }
}
