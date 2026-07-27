use alvenqis_core as core;
use alvenqis_sdk_rust as sdk;

#[test]
fn frozen_protocol_constants_match_core() {
    assert_eq!(sdk::TICKER, core::TICKER);
    assert_eq!(sdk::DECIMALS, core::DECIMALS);
    assert_eq!(sdk::MAX_SUPPLY_ATOMIC, core::MAX_SUPPLY_ATOMIC);
    assert_eq!(sdk::BLOCK_TIME_SECONDS, core::BLOCK_TIME_SECONDS);
    assert_eq!(
        sdk::protocol::MAX_FUTURE_BLOCK_DRIFT_SECONDS,
        core::MAX_FUTURE_BLOCK_DRIFT_SECONDS
    );
    assert_eq!(
        sdk::protocol::MEDIAN_TIME_PAST_WINDOW,
        core::MEDIAN_TIME_PAST_WINDOW
    );
    assert_eq!(sdk::protocol::POW_HASH_ALGORITHM, core::POW_HASH_ALGORITHM);
    assert_eq!(
        sdk::protocol::MAX_TRANSACTIONS_PER_BLOCK,
        core::MAX_TRANSACTIONS_PER_BLOCK
    );
    assert_eq!(
        sdk::protocol::MAX_TRANSACTION_WIRE_BYTES,
        core::MAX_TRANSACTION_WIRE_BYTES
    );
}

#[test]
fn deterministic_address_and_signed_transaction_match_core() {
    let private_key_bytes = [7_u8; 32];
    let core_key = core::PrivateKey::from_bytes(private_key_bytes);
    let sdk_key = sdk::PrivateKey::from_bytes(private_key_bytes);
    let recipient_key_bytes = [11_u8; 32];
    let core_recipient_key = core::PrivateKey::from_bytes(recipient_key_bytes);
    let sdk_recipient_key = sdk::PrivateKey::from_bytes(recipient_key_bytes);

    let core_sender = core::Address::from_public_key_for_network(
        &core_key.public_key(),
        core::Network::MainnetCandidate,
    );
    let sdk_sender = sdk::Address::from_public_key_for_network(
        &sdk_key.public_key(),
        sdk::Network::MainnetCandidate,
    );
    let core_recipient = core::Address::from_public_key_for_network(
        &core_recipient_key.public_key(),
        core::Network::MainnetCandidate,
    );
    let sdk_recipient = sdk::Address::from_public_key_for_network(
        &sdk_recipient_key.public_key(),
        sdk::Network::MainnetCandidate,
    );
    assert_eq!(sdk_sender.to_string(), core_sender.to_string());
    assert_eq!(sdk_recipient.to_string(), core_recipient.to_string());

    let core_transaction = core::Transaction::new_signed(
        1,
        9,
        core::Network::MainnetCandidate,
        &core_key,
        core_recipient.to_string(),
        core::Amount::from_atomic(42_000),
        core::Amount::from_atomic(17),
        core::Amount::from_atomic(5),
        Some(core::Hash::from_bytes([3_u8; 32])),
    )
    .expect("core transaction");
    let sdk_transaction = sdk::Transaction::new_signed(
        1,
        9,
        sdk::Network::MainnetCandidate,
        &sdk_key,
        sdk_recipient.to_string(),
        sdk::Amount::from_atomic(42_000),
        sdk::Amount::from_atomic(17),
        sdk::Amount::from_atomic(5),
        Some(sdk::protocol::Hash::from_bytes([3_u8; 32])),
    )
    .expect("SDK transaction");

    assert_eq!(
        sdk_transaction.unsigned_payload_bytes(),
        core_transaction.unsigned_payload_bytes()
    );
    assert_eq!(
        sdk::hash_to_hex(&sdk_transaction.tx_hash()),
        core::hash_to_hex(&core_transaction.tx_hash())
    );
    assert_eq!(
        serde_json::to_value(&sdk_transaction).expect("SDK JSON"),
        serde_json::to_value(&core_transaction).expect("core JSON")
    );
    sdk_transaction.verify().expect("SDK signature");
    core_transaction.verify().expect("core signature");
}

#[test]
fn post_subsidy_zero_coinbase_shape_matches_core() {
    let recipient_key_bytes = [19_u8; 32];
    let core_key = core::PrivateKey::from_bytes(recipient_key_bytes);
    let sdk_key = sdk::PrivateKey::from_bytes(recipient_key_bytes);
    let core_recipient = core::Address::from_public_key_for_network(
        &core_key.public_key(),
        core::Network::MainnetCandidate,
    );
    let sdk_recipient = sdk::Address::from_public_key_for_network(
        &sdk_key.public_key(),
        sdk::Network::MainnetCandidate,
    );

    let core_coinbase =
        core::Transaction::coinbase(100, core_recipient.to_string(), core::Amount::ZERO)
            .expect("core zero coinbase");
    let sdk_coinbase =
        sdk::Transaction::coinbase(100, sdk_recipient.to_string(), sdk::Amount::ZERO)
            .expect("SDK zero coinbase");

    assert_eq!(
        serde_json::to_value(sdk_coinbase).expect("SDK JSON"),
        serde_json::to_value(core_coinbase).expect("core JSON")
    );
}
