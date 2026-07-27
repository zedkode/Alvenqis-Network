use crate::error::{Result, SdkError};
use crate::protocol::{
    hash_to_hex, Address, Amount, Hash, Network, Transaction, INITIAL_BASE_FEE_ATOMIC,
};
use crate::wallet::WalletAccount;

/// Builder for signed account-based transfers.
#[derive(Clone, Debug)]
pub struct TransferBuilder {
    network: Network,
    version: u32,
    to: Option<String>,
    amount: Option<Amount>,
    nonce: Option<u64>,
    max_fee: Option<Amount>,
    priority_fee: Amount,
    memo_hash: Option<Hash>,
}

impl TransferBuilder {
    pub fn new(network: Network) -> Self {
        Self {
            network,
            version: 1,
            to: None,
            amount: None,
            nonce: None,
            max_fee: None,
            priority_fee: Amount::ZERO,
            memo_hash: None,
        }
    }

    pub fn version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    pub fn to(mut self, to: impl AsRef<str>) -> Result<Self> {
        let address = Address::parse(to.as_ref())?;
        if address.network() != self.network {
            return Err(SdkError::input(format!(
                "recipient network {} does not match builder network {}",
                address.network().network_id(),
                self.network.network_id()
            )));
        }
        self.to = Some(address.to_string());
        Ok(self)
    }

    pub fn amount(mut self, amount: Amount) -> Result<Self> {
        if amount == Amount::ZERO {
            return Err(SdkError::input(
                "transaction amount must be greater than zero",
            ));
        }
        self.amount = Some(amount);
        Ok(self)
    }

    pub fn amount_alve(self, value: &str) -> Result<Self> {
        self.amount(Amount::parse_alve(value)?)
    }

    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(nonce);
        self
    }

    /// Set nonce from a remote account snapshot (`GET /addresses/:addr/account`).
    pub fn nonce_from_account(self, account: &crate::rpc::AddressAccountResponse) -> Self {
        self.nonce(account.next_nonce)
    }

    pub fn max_fee(mut self, max_fee: Amount) -> Self {
        self.max_fee = Some(max_fee);
        self
    }

    pub fn priority_fee(mut self, priority_fee: Amount) -> Self {
        self.priority_fee = priority_fee;
        self
    }

    /// Convenience: max_fee = base_fee + priority_fee (EIP-1559-like shape used by the node).
    pub fn fees(mut self, base_fee: Amount, priority_fee: Amount) -> Result<Self> {
        self.priority_fee = priority_fee;
        self.max_fee = Some(base_fee.checked_add(priority_fee)?);
        Ok(self)
    }

    pub fn memo_hash(mut self, memo_hash: Hash) -> Self {
        self.memo_hash = Some(memo_hash);
        self
    }

    pub fn sign(self, account: &WalletAccount) -> Result<SignedTransfer> {
        account.ensure_network(self.network)?;
        let to = self
            .to
            .ok_or_else(|| SdkError::input("transfer recipient is required"))?;
        let amount = self
            .amount
            .ok_or_else(|| SdkError::input("transfer amount is required"))?;
        let nonce = self
            .nonce
            .ok_or_else(|| SdkError::input("transfer nonce is required"))?;
        let max_fee = self.max_fee.unwrap_or_else(|| {
            Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC).saturating_add_for_fee(self.priority_fee)
        });

        let transaction = Transaction::new_signed(
            self.version,
            nonce,
            self.network,
            account.private_key(),
            to,
            amount,
            max_fee,
            self.priority_fee,
            self.memo_hash,
        )?;
        transaction.verify()?;
        let tx_hash = hash_to_hex(&transaction.tx_hash());
        Ok(SignedTransfer {
            network: self.network,
            tx_hash,
            transaction,
        })
    }
}

/// Helper trait for fee defaulting without panicking on overflow in the common tiny-fee path.
trait AmountFeeExt {
    fn saturating_add_for_fee(self, other: Amount) -> Amount;
}

impl AmountFeeExt for Amount {
    fn saturating_add_for_fee(self, other: Amount) -> Amount {
        self.checked_add(other)
            .unwrap_or_else(|_| Amount::from_atomic(u64::MAX))
    }
}

/// Signed transfer ready for RPC submit.
#[derive(Clone, Debug)]
pub struct SignedTransfer {
    pub network: Network,
    pub tx_hash: String,
    pub transaction: Transaction,
}

impl SignedTransfer {
    pub fn transaction(&self) -> &Transaction {
        &self.transaction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{MnemonicWordCount, Network};
    use crate::wallet::WalletAccount;

    #[test]
    fn builds_and_verifies_signed_transfer() {
        let (sender, _) =
            WalletAccount::generate(Network::MainnetCandidate, MnemonicWordCount::Twelve)
                .expect("sender");
        let (recipient, _) =
            WalletAccount::generate(Network::MainnetCandidate, MnemonicWordCount::Twelve)
                .expect("recipient");

        let signed = TransferBuilder::new(Network::MainnetCandidate)
            .to(recipient.address_string())
            .expect("to")
            .amount(Amount::from_atomic(1_000))
            .expect("amount")
            .nonce(1)
            .fees(Amount::from_atomic(1), Amount::from_atomic(1))
            .expect("fees")
            .sign(&sender)
            .expect("sign");

        assert!(!signed.tx_hash.is_empty());
        signed.transaction.verify().expect("verify");
        assert_eq!(
            signed.transaction.from.as_deref(),
            Some(sender.address_string().as_str())
        );
    }

    #[test]
    fn rejects_cross_network_recipient() {
        let err = TransferBuilder::new(Network::MainnetCandidate)
            .to(
                WalletAccount::generate(Network::Devnet, MnemonicWordCount::Twelve)
                    .expect("dev")
                    .0
                    .address_string(),
            )
            .expect_err("cross network");
        assert!(matches!(err, SdkError::Input(_)));
    }
}
