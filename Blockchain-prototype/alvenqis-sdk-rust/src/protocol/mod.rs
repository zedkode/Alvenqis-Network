//! Self-contained protocol primitives for the SDK (no `alvenqis-core` dependency).
//!
//! Kept in sync with monorepo `alvenqis-core` for wire compatibility. Crates.io
//! consumers only need `alvenqis-sdk-rust`.

pub mod address;
pub mod amount;
pub mod constants;
pub mod crypto;
pub mod errors;
pub mod network;
pub mod seed;
pub mod signing;
pub mod standards;
pub mod transaction;

pub use address::Address;
pub use amount::Amount;
pub use constants::*;
pub use crypto::{blake3_hash, hash_to_hex, Hash};
pub use errors::{AlvenqisError, Result as ProtocolResult};
pub use network::Network;
pub use seed::{generate_mnemonic, MnemonicWordCount, WalletDerivationPath};
pub use signing::{PrivateKey, PublicKey, Signature};
pub use transaction::{Transaction, UnsignedTransaction};
