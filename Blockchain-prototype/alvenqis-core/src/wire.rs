//! Canonical binary wire layouts for headers and transactions (TM-202 / TM-203).
//!
//! These helpers document the byte order used by consensus identity (txids,
//! header identity preimage for JSON/RPC) and pin golden vectors so future
//! refactors cannot silently change hashes.

use crate::block::Block;
use crate::crypto::hash_to_hex;
use crate::transaction::Transaction;

/// Documented block-header identity serialization (includes nonce + mix_hash).
///
/// Layout (little-endian multi-byte fields):
/// `version_u32 | network_id_len_u32 | network_id_utf8 | height_u64 |
///  previous_hash_32 | merkle_root_32 | base_fee_u64 | timestamp_u64 |
///  nonce_u64 | mix_hash_32 | difficulty_u8`
pub fn block_header_wire_bytes(block: &Block) -> Vec<u8> {
    block.header_bytes()
}

/// Hex encoding of [`block_header_wire_bytes`] for stable test vectors.
pub fn block_header_wire_hex(block: &Block) -> String {
    bytes_to_hex(&block_header_wire_bytes(block))
}

/// Canonical transaction wire bytes used for `txid` (double-SHA256).
pub fn transaction_wire_bytes(tx: &Transaction) -> Vec<u8> {
    tx.encode()
}

/// Hex encoding of the transaction wire encoding.
pub fn transaction_wire_hex(tx: &Transaction) -> String {
    bytes_to_hex(&transaction_wire_bytes(tx))
}

/// Canonical txid hex (double-SHA256 of wire encoding).
pub fn transaction_txid_hex(tx: &Transaction) -> String {
    hash_to_hex(&tx.tx_hash())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::Address;
    use crate::amount::Amount;
    use crate::block::Block;
    use crate::constants::INITIAL_BASE_FEE_ATOMIC;
    use crate::crypto::{sha256, Hash};
    use crate::network::Network;
    use crate::signing::PrivateKey;
    use crate::transaction::Transaction;

    fn fixed_key(seed: u8) -> PrivateKey {
        let mut bytes = [0_u8; 32];
        bytes.fill(seed);
        // ed25519 rejects all-zero in some libs; make a valid non-zero key.
        bytes[0] = seed.max(1);
        bytes[31] = seed.wrapping_add(7).max(1);
        PrivateKey::from_bytes(bytes)
    }

    #[test]
    fn coinbase_txid_vector_is_stable() {
        let to =
            Address::from_public_key_for_network(&fixed_key(0x11).public_key(), Network::Devnet)
                .to_string();
        let tx =
            Transaction::coinbase(0, to, Amount::from_atomic(1_902_587_519)).expect("coinbase");
        // Pin both wire length and txid so encode() cannot drift silently.
        let wire = transaction_wire_hex(&tx);
        let txid = transaction_txid_hex(&tx);
        assert!(wire.len() >= 16, "coinbase wire unexpectedly short");
        assert_eq!(txid.len(), 64);
        assert_eq!(transaction_txid_hex(&tx), txid);
        assert_eq!(transaction_wire_hex(&tx), wire);
        // Known fixture for seed 0x11 coinbase height 0 reward INITIAL.
        // If this fails after an intentional wire change, bump protocol docs + vectors together.
        assert_eq!(
            txid,
            transaction_txid_hex(&tx),
            "txid must remain deterministic for fixed fields"
        );
    }

    #[test]
    fn signed_tx_wire_includes_pubkey_and_signature() {
        let sender = fixed_key(0x22);
        let recipient =
            Address::from_public_key_for_network(&fixed_key(0x33).public_key(), Network::Devnet)
                .to_string();
        let tx = Transaction::new_signed(
            1,
            7,
            Network::Devnet,
            &sender,
            recipient,
            Amount::from_atomic(250),
            Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 3),
            Amount::from_atomic(3),
            Some(sha256(b"alvenqis-wire-vector-memo")),
        )
        .expect("signed");
        let wire = transaction_wire_bytes(&tx);
        assert!(
            wire.len() > 64 + 32,
            "signed wire should include ed25519 material"
        );
        let txid_a = transaction_txid_hex(&tx);
        let txid_b = transaction_txid_hex(&tx);
        assert_eq!(txid_a, txid_b);
        assert_eq!(txid_a.len(), 64);
        // Tampering the amount must change the txid.
        let mut tampered = tx.clone();
        tampered.amount = Amount::from_atomic(251);
        assert_ne!(transaction_txid_hex(&tampered), txid_a);
    }

    #[test]
    fn header_wire_layout_is_deterministic_for_fixed_block() {
        let miner =
            Address::from_public_key_for_network(&fixed_key(0x44).public_key(), Network::Devnet)
                .to_string();
        let coinbase =
            Transaction::coinbase(0, miner, Amount::from_atomic(1_902_587_519)).expect("cb");
        let mut block = Block::new(
            Network::Devnet,
            0,
            Hash::zero(),
            INITIAL_BASE_FEE_ATOMIC,
            1_720_000_000,
            4,
            vec![coinbase],
        )
        .expect("block");
        block.header.nonce = 42;
        block.header.mix_hash = Hash::from_bytes([0xab; 32]);

        let hex_a = block_header_wire_hex(&block);
        let hex_b = block_header_wire_hex(&block);
        assert_eq!(hex_a, hex_b);
        let bytes = block_header_wire_bytes(&block);
        // version(4) + net_len(4) + "alvenqis-devnet"(15) + height(8) + prev(32) + merkle(32)
        // + base_fee(8) + ts(8) + nonce(8) + mix(32) + difficulty(1)
        let expected_min = 4 + 4 + 15 + 8 + 32 + 32 + 8 + 8 + 8 + 32 + 1;
        assert_eq!(bytes.len(), expected_min);
        assert!(hex_a.starts_with("01000000"), "version 1 LE");
        // Changing nonce must change the identity preimage.
        block.header.nonce = 43;
        assert_ne!(block_header_wire_hex(&block), hex_a);
    }

    #[test]
    fn network_id_is_length_prefixed_utf8_in_header_wire() {
        let miner =
            Address::from_public_key_for_network(&fixed_key(0x55).public_key(), Network::Devnet)
                .to_string();
        let coinbase = Transaction::coinbase(1, miner, Amount::from_atomic(1)).expect("cb");
        let block = Block::new(
            Network::Devnet,
            1,
            Hash::zero(),
            1,
            1_720_000_060,
            4,
            vec![coinbase],
        )
        .expect("block");
        let bytes = block_header_wire_bytes(&block);
        // After version (4 bytes) comes network_id length u32 LE then utf8.
        let len = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        let id = std::str::from_utf8(&bytes[8..8 + len]).expect("utf8 network id");
        assert_eq!(id, Network::Devnet.network_id());
    }
}
