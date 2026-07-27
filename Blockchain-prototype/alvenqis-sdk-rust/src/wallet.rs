use crate::error::{Result, SdkError};
use crate::protocol::{
    generate_mnemonic, Address, MnemonicWordCount, Network, PrivateKey, PublicKey,
    WalletDerivationPath,
};

/// In-memory wallet account. Keys are never written to disk by this type.
///
/// For browser product flows, prefer a native messaging host that owns the keystore
/// rather than placing mnemonics in page JavaScript.
///
/// Note: not `Clone` because `PrivateKey` intentionally does not implement `Clone`.
pub struct WalletAccount {
    network: Network,
    private_key: PrivateKey,
    address: Address,
    derivation_path: Option<String>,
}

impl WalletAccount {
    /// Generate a new mnemonic-backed account (mnemonic returned once to the caller).
    pub fn generate(
        network: Network,
        word_count: MnemonicWordCount,
    ) -> Result<(Self, GeneratedMnemonic)> {
        Self::generate_with_path(network, word_count, WalletDerivationPath::default())
    }

    pub fn generate_with_path(
        network: Network,
        word_count: MnemonicWordCount,
        path: WalletDerivationPath,
    ) -> Result<(Self, GeneratedMnemonic)> {
        let mnemonic = generate_mnemonic(word_count)?;
        let account = Self::from_mnemonic(network, &mnemonic, "", path)?;
        Ok((
            account,
            GeneratedMnemonic {
                phrase: mnemonic,
                word_count: word_count.as_usize(),
                derivation_path: path.to_string(),
                warning: "Back up this mnemonic now. The SDK keeps keys in memory only and does not persist them.",
            },
        ))
    }

    pub fn from_mnemonic(
        network: Network,
        phrase: &str,
        passphrase: &str,
        path: WalletDerivationPath,
    ) -> Result<Self> {
        let private_key = PrivateKey::from_mnemonic(phrase, passphrase, path)?;
        Self::from_private_key(network, private_key, Some(path.to_string()))
    }

    pub fn from_private_key(
        network: Network,
        private_key: PrivateKey,
        derivation_path: Option<String>,
    ) -> Result<Self> {
        let address = Address::from_public_key_for_network(&private_key.public_key(), network);
        Ok(Self {
            network,
            private_key,
            address,
            derivation_path,
        })
    }

    pub fn from_private_key_hex(
        network: Network,
        private_key_hex: &str,
        derivation_path: Option<String>,
    ) -> Result<Self> {
        let private_key = PrivateKey::from_hex(private_key_hex)?;
        Self::from_private_key(network, private_key, derivation_path)
    }

    pub fn network(&self) -> Network {
        self.network
    }

    pub fn address(&self) -> &Address {
        &self.address
    }

    pub fn address_string(&self) -> String {
        self.address.to_string()
    }

    pub fn public_key(&self) -> PublicKey {
        self.private_key.public_key()
    }

    pub fn derivation_path(&self) -> Option<&str> {
        self.derivation_path.as_deref()
    }

    /// Borrow the private key for signing. Prefer `TransferBuilder::sign`.
    pub fn private_key(&self) -> &PrivateKey {
        &self.private_key
    }
}

/// Mnemonic material returned once at generation time.
#[derive(Clone, Debug)]
pub struct GeneratedMnemonic {
    pub phrase: String,
    pub word_count: usize,
    pub derivation_path: String,
    pub warning: &'static str,
}

impl WalletAccount {
    pub fn ensure_network(&self, expected: Network) -> Result<()> {
        if self.network != expected {
            return Err(SdkError::input(format!(
                "wallet network {} does not match expected {}",
                self.network.network_id(),
                expected.network_id()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Network;

    #[test]
    fn generate_and_reimport_roundtrip() {
        let (account, mnemonic) =
            WalletAccount::generate(Network::MainnetCandidate, MnemonicWordCount::Twelve)
                .expect("generate");
        assert!(account.address_string().starts_with("alve1"));
        let restored = WalletAccount::from_mnemonic(
            Network::MainnetCandidate,
            &mnemonic.phrase,
            "",
            WalletDerivationPath::default(),
        )
        .expect("import");
        assert_eq!(account.address_string(), restored.address_string());
    }
}
