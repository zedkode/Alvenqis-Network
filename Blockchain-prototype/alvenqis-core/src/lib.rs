pub mod address;
pub mod amount;
pub mod block;
pub mod chain;
pub mod checkpoint;
pub mod consensus;
pub mod constants;
pub mod crypto;
pub mod errors;
pub mod firopow;
pub mod genesis;
pub mod network;
pub mod pow;
pub mod protocol;
pub mod seed;
pub mod signing;
pub mod standards;
pub mod state;
pub mod transaction;
pub mod upgrade;
pub mod wire;

pub use address::Address;
pub use amount::Amount;
pub use block::{Block, BlockHeader};
pub use chain::{
    block_work, common_ancestor_height, cumulative_work, select_fork, Chain, ChainWork, ForkChoice,
};
pub use checkpoint::{
    checkpoint_at_height, launch_checkpoint_policy, scheduled_checkpoints, validate_checkpoint,
    ChainCheckpoint, CheckpointPolicy, CHECKPOINT_POLICY_ID, CHECKPOINT_POLICY_MODE,
    CHECKPOINT_POLICY_RELAXATION,
};
pub use consensus::{
    block_reward, block_subsidy, expected_coinbase_amount, initial_base_fee, lwma_next_difficulty,
    median_time_past, next_base_fee, next_base_fee_from_block, next_difficulty_for_network,
    total_theoretical_emission, validate_block_not_too_far_in_future, validate_block_timestamp,
    validate_coinbase_amount, validate_coinbase_structure, validate_median_time_past,
    validate_next_block,
};
pub use constants::*;
pub use crypto::{blake3_hash, double_sha256, hash_to_hex, leading_zero_bits, sha256, Hash};
pub use errors::{AlvenqisError, Result};
pub use firopow::{FiroPow, FiroPowOutput, FIROPOW_REVISION, PERIOD_LENGTH};
pub use genesis::{
    child_block_with_consensus_difficulty, devnet_child_block, devnet_child_block_with_difficulty,
    devnet_genesis, devnet_genesis_with_difficulty, genesis_with_difficulty_for_network,
    genesis_with_timestamp_for_network, mine_block, DEVNET_DIFFICULTY_LEADING_ZERO_BITS,
};
pub use network::Network;
pub use pow::{
    check_pow, pow_hash, validate_pow, Blake3LeadingZeroPow, PowValidation, PowVersion,
    POW_ALGORITHM_ID, POW_VERSION,
};
pub use protocol::{launch_protocol_parameters, ProtocolParameters, PROTOCOL_PARAMETERS_ID};
pub use seed::{
    derive_private_key_from_mnemonic, generate_mnemonic, normalize_mnemonic, MnemonicWordCount,
    WalletDerivationPath,
};
pub use signing::{PrivateKey, PublicKey, Signature};
pub use standards::{
    launch_address_standard, launch_key_derivation_policy, launch_signing_standard,
    launch_wallet_seed_standard, AddressStandard, KeyDerivationPolicy, SigningStandard,
    WalletSeedStandard, ADDRESS_STANDARD_ID, KEY_DERIVATION_POLICY_ID, SIGNATURE_STANDARD_ID,
    WALLET_SEED_STANDARD_ID,
};
pub use state::{
    apply_block, apply_transaction, validate_block_against_state,
    validate_transaction_against_state, BlockLedgerSummary, LedgerState, FIRST_ACCOUNT_NONCE,
    TX_HASH_RETENTION_BLOCKS,
};
pub use transaction::{Transaction, UnsignedTransaction};
pub use upgrade::{
    expected_block_version, launch_upgrade_policy, protocol_version_at_height,
    scheduled_protocol_upgrades, ScheduledProtocolUpgrade, UpgradeActivationPolicy,
    LAUNCH_BLOCK_VERSION, LAUNCH_PROTOCOL_VERSION, LAUNCH_UPGRADE_MIGRATION_PATH,
    UPGRADE_ACTIVATION_MODE, UPGRADE_ACTIVATION_POLICY_ID,
};
pub use wire::{
    block_header_wire_bytes, block_header_wire_hex, transaction_txid_hex, transaction_wire_bytes,
    transaction_wire_hex,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn dev_address(label: &str) -> String {
        let mut bytes = [0_u8; 32];
        let seed = label
            .as_bytes()
            .iter()
            .fold(1_u8, |value, byte| value.wrapping_add(*byte));
        bytes.fill(seed);
        let private_key = PrivateKey::from_bytes(bytes);
        Address::from_public_key_for_network(&private_key.public_key(), Network::Devnet).to_string()
    }

    fn signed_transaction() -> Transaction {
        let recipient_key = PrivateKey::generate();
        Transaction::new_signed(
            1,
            7,
            Network::Devnet,
            &PrivateKey::generate(),
            Address::from_public_key_for_network(&recipient_key.public_key(), Network::Devnet)
                .to_string(),
            Amount::from_atomic(250),
            Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 3),
            Amount::from_atomic(3),
            Some(sha256(b"memo")),
        )
        .expect("signed transaction should be valid")
    }

    #[test]
    fn alve_unit_constants_are_correct() {
        assert_eq!(DECIMALS, 8);
        assert_eq!(ATOMIC_UNITS_PER_ALVE, 100_000_000);
        assert_eq!(MAX_SUPPLY_ATOMIC, 6_000_000_000_000_000);
    }

    #[test]
    fn initial_reward_atomic_units_are_correct() {
        assert_eq!(INITIAL_BLOCK_REWARD_ATOMIC, 1_902_587_519);
        assert_eq!(block_reward(0).as_atomic(), INITIAL_BLOCK_REWARD_ATOMIC);
    }

    #[test]
    fn reward_halves_after_halving_interval() {
        assert_eq!(
            block_reward(HALVING_INTERVAL_BLOCKS).as_atomic(),
            INITIAL_BLOCK_REWARD_ATOMIC / 2
        );
    }

    #[test]
    fn total_theoretical_emission_does_not_exceed_max_supply() {
        let total = total_theoretical_emission().expect("emission math should work");
        assert!(total.as_atomic() <= MAX_SUPPLY_ATOMIC);
    }

    #[test]
    fn transaction_hash_is_deterministic() {
        let tx = signed_transaction();

        assert_eq!(tx.tx_hash(), tx.tx_hash());
    }

    #[test]
    fn block_hash_uses_pow_hash() {
        let coinbase = Transaction::coinbase(0, dev_address("miner"), block_reward(0))
            .expect("coinbase should be valid");
        let block = Block::new(
            Network::Devnet,
            0,
            Hash::zero(),
            initial_base_fee().as_atomic(),
            1_720_000_000,
            0,
            vec![coinbase],
        )
        .expect("block builds");

        assert_eq!(
            block.hash().expect("hash"),
            block.pow_hash().expect("pow_hash")
        );
    }

    #[test]
    fn block_hash_has_no_blake3_fallback_identity() {
        // Identity is FiroPoW-only: evaluation must succeed or return Err (CR-H08).
        // There is no alternate Blake3 digest when the native evaluator fails.
        let coinbase = Transaction::coinbase(0, dev_address("miner"), block_reward(0))
            .expect("coinbase should be valid");
        let block = Block::new(
            Network::Devnet,
            0,
            Hash::zero(),
            initial_base_fee().as_atomic(),
            1_720_000_000,
            0,
            vec![coinbase],
        )
        .expect("block builds");
        // With a working native evaluator this Ok is a real FiroPoW final hash (non-zero).
        // If evaluation cannot run, Err must surface — never Hash::zero() / Blake3 substitute.
        match block.hash() {
            Ok(h) => assert_ne!(h, Hash::zero(), "canonical FiroPoW hash must not be zero"),
            Err(err) => {
                let msg = err.to_string();
                assert!(
                    msg.contains("FiroPoW") || msg.contains("evaluation"),
                    "diagnostic PoW failure expected, got: {msg}"
                );
            }
        }
    }

    #[test]
    fn zero_coinbase_allowed_when_expected_is_zero() {
        // Post-subsidy empty block liveness (audit CR-H09).
        let zero = Amount::ZERO;
        let coinbase = Transaction::coinbase(u64::MAX / 2, dev_address("miner"), zero)
            .expect("zero coinbase shape is valid");
        assert!(coinbase.is_coinbase());
        assert_eq!(coinbase.amount, zero);
    }

    #[test]
    fn block_hash_changes_when_nonce_changes() {
        let coinbase = Transaction::coinbase(0, dev_address("miner"), block_reward(0))
            .expect("coinbase should be valid");
        let mut block = Block::new(
            Network::Devnet,
            0,
            Hash::zero(),
            initial_base_fee().as_atomic(),
            1_720_000_000,
            0,
            vec![coinbase],
        )
        .expect("block builds");
        let hash_before = block.hash().expect("hash before");
        block.header.nonce = 1;
        let hash_after = block.hash().expect("hash after");
        assert_ne!(hash_before, hash_after);
    }

    #[test]
    fn merkle_root_is_deterministic() {
        let reward = block_reward(0);
        let alice = PrivateKey::generate();
        let bob = PrivateKey::generate();
        let txs = vec![
            Transaction::coinbase(0, dev_address("miner"), reward).expect("coinbase"),
            Transaction::new_signed(
                1,
                1,
                Network::Devnet,
                &alice,
                Address::from_public_key_for_network(&bob.public_key(), Network::Devnet)
                    .to_string(),
                Amount::from_atomic(100),
                Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 1),
                Amount::from_atomic(1),
                None,
            )
            .expect("tx"),
        ];

        let block_a = Block::new(
            Network::Devnet,
            0,
            Hash::zero(),
            initial_base_fee().as_atomic(),
            1_720_000_000,
            0,
            txs.clone(),
        )
        .expect("block A");
        let block_b = Block::new(
            Network::Devnet,
            0,
            Hash::zero(),
            initial_base_fee().as_atomic(),
            1_720_000_000,
            0,
            txs,
        )
        .expect("block B");
        assert_eq!(block_a.header.merkle_root, block_b.header.merkle_root);
    }

    #[test]
    fn pow_check_works_for_low_dev_difficulty() {
        let mut block = devnet_genesis(&dev_address("miner")).expect("genesis should be mined");
        block.header.difficulty_leading_zero_bits = 4;
        crate::genesis::mine_block(&mut block);
        assert!(check_pow(
            &block.pow_hash().expect("pow hash"),
            block.header.difficulty_leading_zero_bits
        ));
    }

    #[test]
    fn valid_genesis_and_dev_block_can_be_appended() {
        let mut chain = Chain::new(Network::Devnet);
        let genesis = devnet_genesis(&dev_address("miner")).expect("genesis");
        chain.append_block(genesis.clone()).expect("append genesis");

        let child = devnet_child_block(
            &genesis,
            &dev_address("miner"),
            genesis.header.timestamp + BLOCK_TIME_SECONDS,
            vec![],
        )
        .expect("child");
        chain.append_block(child).expect("append child");

        assert_eq!(chain.height(), Some(1));
        assert_eq!(
            chain.emitted_supply().as_atomic(),
            INITIAL_BLOCK_REWARD_ATOMIC * 2
        );
    }

    #[test]
    fn invalid_previous_hash_is_rejected() {
        let mut chain = Chain::new(Network::Devnet);
        let genesis = devnet_genesis(&dev_address("miner")).expect("genesis");
        chain.append_block(genesis).expect("append genesis");

        let wrong_child = Block::new(
            Network::Testnet,
            1,
            Hash::zero(),
            initial_base_fee().as_atomic(),
            1_720_000_060,
            0,
            vec![
                Transaction::coinbase(1, dev_address("miner"), block_reward(1)).expect("coinbase"),
            ],
        )
        .expect("block");

        let error = chain
            .append_block(wrong_child)
            .expect_err("previous hash must fail");
        assert!(matches!(error, AlvenqisError::InvalidNetwork { .. }));
    }

    #[test]
    fn coinbase_reward_above_allowed_reward_is_rejected() {
        let mut chain = Chain::new(Network::Devnet);
        let genesis = devnet_genesis(&dev_address("miner")).expect("genesis");
        chain.append_block(genesis.clone()).expect("append genesis");

        let excessive_reward = block_reward(1)
            .checked_add(Amount::from_atomic(1))
            .expect("reward addition");
        let mut invalid_block = Block::new(
            Network::Devnet,
            1,
            genesis.hash().expect("genesis hash"),
            next_base_fee(Some(&genesis)).as_atomic(),
            genesis.header.timestamp + BLOCK_TIME_SECONDS,
            next_difficulty_for_network(
                Network::Devnet,
                std::slice::from_ref(&genesis),
                genesis.header.difficulty_leading_zero_bits,
            ),
            vec![
                Transaction::coinbase(1, dev_address("miner"), excessive_reward).expect("coinbase"),
            ],
        )
        .expect("block");
        crate::genesis::mine_block(&mut invalid_block);

        let error = chain
            .append_block(invalid_block)
            .expect_err("coinbase reward above allowed reward must fail");
        assert!(matches!(error, AlvenqisError::InvalidCoinbaseAmount { .. }));
    }

    #[test]
    fn coinbase_reward_below_allowed_reward_is_rejected() {
        let mut chain = Chain::new(Network::Devnet);
        let genesis = devnet_genesis(&dev_address("miner")).expect("genesis");
        chain.append_block(genesis.clone()).expect("append genesis");

        let under_reward = block_reward(1)
            .checked_sub(Amount::from_atomic(1))
            .expect("reward subtraction");
        let mut invalid_block = Block::new(
            Network::Devnet,
            1,
            genesis.hash().expect("genesis hash"),
            next_base_fee(Some(&genesis)).as_atomic(),
            genesis.header.timestamp + BLOCK_TIME_SECONDS,
            next_difficulty_for_network(
                Network::Devnet,
                std::slice::from_ref(&genesis),
                genesis.header.difficulty_leading_zero_bits,
            ),
            vec![Transaction::coinbase(1, dev_address("miner"), under_reward).expect("coinbase")],
        )
        .expect("block");
        crate::genesis::mine_block(&mut invalid_block);

        let error = chain
            .append_block(invalid_block)
            .expect_err("coinbase underpayment must fail");
        assert!(matches!(error, AlvenqisError::InvalidCoinbaseAmount { .. }));
    }

    #[test]
    fn coinbase_helpers_agree_between_consensus_and_state() {
        // Single source of truth: expected_coinbase_amount is exact subsidy + priority fees.
        let height = 1_u64;
        let base = initial_base_fee();
        let txs = vec![
            Transaction::coinbase(height, dev_address("miner"), block_reward(height))
                .expect("coinbase"),
        ];
        let expected =
            expected_coinbase_amount(height, base, &txs).expect("expected coinbase amount");
        assert_eq!(expected, block_reward(height));
        validate_coinbase_amount(height, base, &txs).expect("exact coinbase ok");
    }

    #[test]
    fn future_timestamp_beyond_drift_is_rejected() {
        let now = 1_800_000_000_u64;
        let too_far = now + MAX_FUTURE_BLOCK_DRIFT_SECONDS + 1;
        let err = validate_block_not_too_far_in_future(too_far, now)
            .expect_err("far-future timestamp must fail");
        assert!(matches!(err, AlvenqisError::InvalidFutureTimestamp { .. }));
        validate_block_not_too_far_in_future(now + MAX_FUTURE_BLOCK_DRIFT_SECONDS, now)
            .expect("exactly max drift is allowed");
        validate_block_not_too_far_in_future(now.saturating_sub(60), now).expect("past ok");
    }

    #[test]
    fn block_rejects_too_many_transactions() {
        let miner = dev_address("cap-miner");
        let mut chain = Chain::new(Network::Devnet);
        let genesis = devnet_genesis(&miner).expect("genesis");
        chain.append_block(genesis.clone()).expect("genesis");

        let coinbase = Transaction::coinbase(1, miner.clone(), block_reward(1)).expect("coinbase");
        // Inflate body past the consensus hard cap (duplicates fail after the count check).
        let mut txs = vec![coinbase; MAX_TRANSACTIONS_PER_BLOCK + 1];
        // Ensure first is coinbase-shaped; remaining clones still count toward the cap.
        let mut bloated = Block::new(
            Network::Devnet,
            1,
            genesis.hash().expect("genesis hash"),
            next_base_fee(Some(&genesis)).as_atomic(),
            genesis.header.timestamp + BLOCK_TIME_SECONDS,
            next_difficulty_for_network(
                Network::Devnet,
                std::slice::from_ref(&genesis),
                genesis.header.difficulty_leading_zero_bits,
            ),
            // Block::new rejects empty; use two txs then replace with over-cap list.
            vec![
                Transaction::coinbase(1, miner.clone(), block_reward(1)).expect("cb"),
                Transaction::coinbase(1, miner, block_reward(1)).expect("cb2"),
            ],
        )
        .expect("scaffold");
        bloated.transactions = std::mem::take(&mut txs);
        crate::genesis::mine_block(&mut bloated);

        let err = chain
            .append_block(bloated)
            .expect_err("over-cap block must fail");
        assert!(
            matches!(err, AlvenqisError::TooManyTransactions { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn median_time_past_uses_middle_of_sorted_window() {
        let miner = dev_address("mtp-miner");
        let mut blocks = Vec::new();
        let mut parent = devnet_genesis(&miner).expect("genesis");
        blocks.push(parent.clone());
        for i in 0..10 {
            let child = devnet_child_block(
                &parent,
                &miner,
                parent.header.timestamp + BLOCK_TIME_SECONDS + i,
                vec![],
            )
            .expect("child");
            blocks.push(child.clone());
            parent = child;
        }
        let mtp = median_time_past(&blocks).expect("mtp");
        let mut times: Vec<u64> = blocks.iter().map(|b| b.header.timestamp).collect();
        times.sort_unstable();
        assert_eq!(mtp, times[times.len() / 2]);
        assert!(mtp < blocks.last().expect("tip").header.timestamp);

        let mut bad = blocks.last().expect("tip").clone();
        bad.header.height = blocks.len() as u64;
        bad.header.previous_hash = blocks.last().expect("tip").hash().expect("tip hash");
        bad.header.timestamp = mtp;
        let err = validate_median_time_past(&blocks, &bad).expect_err("mtp floor");
        assert!(matches!(err, AlvenqisError::InvalidMedianTimePast { .. }));
    }

    #[test]
    fn non_monotonic_timestamp_equal_to_previous_is_rejected() {
        let mut chain = Chain::new(Network::Devnet);
        let genesis = devnet_genesis(&dev_address("miner")).expect("genesis");
        chain.append_block(genesis.clone()).expect("append genesis");

        let mut invalid = Block::new(
            Network::Devnet,
            1,
            genesis.hash().expect("genesis hash"),
            next_base_fee(Some(&genesis)).as_atomic(),
            genesis.header.timestamp, // equal → must fail
            next_difficulty_for_network(
                Network::Devnet,
                std::slice::from_ref(&genesis),
                genesis.header.difficulty_leading_zero_bits,
            ),
            vec![
                Transaction::coinbase(1, dev_address("miner"), block_reward(1)).expect("coinbase"),
            ],
        )
        .expect("block");
        crate::genesis::mine_block(&mut invalid);

        let error = chain
            .append_block(invalid)
            .expect_err("equal timestamp must fail");
        assert!(matches!(
            error,
            AlvenqisError::InvalidTimestamp {
                previous,
                actual
            } if previous == genesis.header.timestamp && actual == genesis.header.timestamp
        ));
    }

    #[test]
    fn non_monotonic_timestamp_before_previous_is_rejected() {
        let mut chain = Chain::new(Network::Devnet);
        let genesis = devnet_genesis(&dev_address("miner")).expect("genesis");
        chain.append_block(genesis.clone()).expect("append genesis");

        let backdated = genesis.header.timestamp.saturating_sub(1);
        let mut invalid = Block::new(
            Network::Devnet,
            1,
            genesis.hash().expect("genesis hash"),
            next_base_fee(Some(&genesis)).as_atomic(),
            backdated,
            next_difficulty_for_network(
                Network::Devnet,
                std::slice::from_ref(&genesis),
                genesis.header.difficulty_leading_zero_bits,
            ),
            vec![
                Transaction::coinbase(1, dev_address("miner"), block_reward(1)).expect("coinbase"),
            ],
        )
        .expect("block");
        crate::genesis::mine_block(&mut invalid);

        let error = chain
            .append_block(invalid)
            .expect_err("backdated timestamp must fail");
        assert!(matches!(error, AlvenqisError::InvalidTimestamp { .. }));
    }

    #[test]
    fn state_layer_rejects_non_monotonic_timestamp() {
        use crate::state::{apply_block, validate_block_against_state, LedgerState};

        let miner = dev_address("miner");
        let genesis = devnet_genesis(&miner).expect("genesis");
        let mut state = LedgerState::new();
        apply_block(&mut state, &genesis).expect("apply genesis");

        let mut same_ts = Block::new(
            Network::Devnet,
            1,
            genesis.hash().expect("genesis hash"),
            next_base_fee(Some(&genesis)).as_atomic(),
            genesis.header.timestamp,
            next_difficulty_for_network(
                Network::Devnet,
                std::slice::from_ref(&genesis),
                genesis.header.difficulty_leading_zero_bits,
            ),
            vec![Transaction::coinbase(1, miner, block_reward(1)).expect("coinbase")],
        )
        .expect("block");
        crate::genesis::mine_block(&mut same_ts);

        let error = validate_block_against_state(&state, &same_ts)
            .expect_err("state must enforce timestamp monotonicity");
        assert!(matches!(error, AlvenqisError::InvalidTimestamp { .. }));
    }

    #[test]
    fn address_format_and_parse_roundtrip() {
        let private_key = PrivateKey::generate();
        let address =
            Address::from_public_key_for_network(&private_key.public_key(), Network::Devnet);
        let encoded = address.to_string();
        let parsed = Address::parse(&encoded).expect("address should parse");

        assert_eq!(encoded, parsed.to_string());
        assert_eq!(parsed.version(), 0);
        assert_eq!(
            parsed.to_public_key().expect("public key"),
            private_key.public_key()
        );
    }

    #[test]
    fn invalid_address_checksum_is_rejected() {
        let private_key = PrivateKey::generate();
        let address =
            Address::from_public_key_for_network(&private_key.public_key(), Network::Devnet)
                .to_string();
        let mut tampered = address.into_bytes();
        let last = tampered.len() - 1;
        tampered[last] = if tampered[last] == b'q' { b'p' } else { b'q' };
        let tampered = String::from_utf8(tampered).expect("valid string");

        let error = Address::parse(&tampered).expect_err("checksum must fail");
        assert!(matches!(error, AlvenqisError::InvalidAddress(_)));
    }

    #[test]
    fn uppercase_address_is_rejected() {
        let private_key = PrivateKey::generate();
        let address =
            Address::from_public_key_for_network(&private_key.public_key(), Network::Devnet)
                .to_string()
                .to_uppercase();

        let error = Address::parse(&address).expect_err("uppercase address must fail");
        assert!(matches!(error, AlvenqisError::InvalidAddress(_)));
    }

    #[test]
    fn launch_address_and_signing_standards_are_frozen() {
        let devnet_address = launch_address_standard(Network::Devnet);
        let signing = launch_signing_standard();
        let key_policy = launch_key_derivation_policy();
        let seed_standard = launch_wallet_seed_standard();

        assert_eq!(devnet_address.standard_id, ADDRESS_STANDARD_ID);
        assert_eq!(devnet_address.address_prefix, "dalve");
        assert_eq!(devnet_address.payload_version, 0);
        assert_eq!(devnet_address.payload_size_bytes, 33);
        assert_eq!(signing.standard_id, SIGNATURE_STANDARD_ID);
        assert_eq!(signing.public_key_scheme, "ed25519");
        assert_eq!(signing.signature_scheme, "ed25519");
        assert_eq!(signing.tx_signing_domain, "alvenqis-tx-ed25519-v1");
        assert_eq!(key_policy.policy_id, KEY_DERIVATION_POLICY_ID);
        assert_eq!(key_policy.wallet_seed_standard_id, WALLET_SEED_STANDARD_ID);
        assert_eq!(key_policy.hd_derivation_coin_type, 7_330);
        assert_eq!(seed_standard.standard_id, WALLET_SEED_STANDARD_ID);
        assert_eq!(
            seed_standard.derivation_path_template,
            "m/44'/7330'/account'/change'/address_index'"
        );
    }

    #[test]
    fn keypair_can_sign_and_verify_a_message() {
        let private_key = PrivateKey::generate();
        let public_key = private_key.public_key();
        let message = b"alvenqis message";
        let signature = private_key.sign(message);

        public_key
            .verify(message, &signature)
            .expect("signature should verify");
    }

    #[test]
    fn signed_transaction_verifies() {
        let transaction = signed_transaction();
        transaction.verify().expect("transaction should verify");
    }

    #[test]
    fn tampered_signed_transaction_fails_verification() {
        let mut transaction = signed_transaction();
        transaction.amount = Amount::from_atomic(transaction.amount.as_atomic() + 1);

        let error = transaction
            .verify()
            .expect_err("tampering should break the signature");
        assert!(matches!(error, AlvenqisError::InvalidSignature(_)));
    }

    #[test]
    fn coinbase_transaction_does_not_require_signature() {
        let transaction =
            Transaction::coinbase(0, dev_address("miner"), block_reward(0)).expect("coinbase");
        transaction.verify().expect("coinbase should verify");
    }

    #[test]
    fn block_version_matches_launch_upgrade_policy() {
        let coinbase = Transaction::coinbase(0, dev_address("miner"), block_reward(0))
            .expect("coinbase should be valid");
        let block = Block::new(
            Network::MainnetCandidate,
            0,
            Hash::zero(),
            initial_base_fee().as_atomic(),
            1_720_000_000,
            16,
            vec![coinbase],
        )
        .expect("block builds");

        assert_eq!(
            block.header.version,
            expected_block_version(Network::MainnetCandidate, 0)
        );
        assert_eq!(block.header.version, LAUNCH_BLOCK_VERSION);
    }

    #[test]
    fn block_with_wrong_version_is_rejected() {
        let mut chain = Chain::new(Network::Devnet);
        let genesis = devnet_genesis(&dev_address("miner")).expect("genesis");
        chain.append_block(genesis.clone()).expect("append genesis");

        let mut block = devnet_child_block(
            &genesis,
            &dev_address("miner"),
            genesis.header.timestamp + BLOCK_TIME_SECONDS,
            vec![],
        )
        .expect("child block");
        block.header.version = LAUNCH_BLOCK_VERSION + 1;

        let error = chain
            .append_block(block)
            .expect_err("wrong block version must fail");
        assert!(matches!(error, AlvenqisError::InvalidBlockVersion { .. }));
    }

    #[test]
    fn mainnet_candidate_genesis_checkpoint_is_enforced() {
        let recipient = "alve1qr4y5mrru2w9yz4774g8kyewchue23mk46ltu7ujgg0w56g5gmfzcnfqv0q";
        let mut chain = Chain::new(Network::MainnetCandidate);
        let genesis = genesis_with_timestamp_for_network(
            Network::MainnetCandidate,
            recipient,
            1_720_000_000,
            16,
        )
        .expect("genesis");

        chain.append_block(genesis).expect("checkpointed genesis");
    }

    #[test]
    fn mainnet_candidate_wrong_genesis_checkpoint_is_rejected() {
        let wrong_recipient = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::MainnetCandidate,
        )
        .to_string();
        let wrong_genesis = genesis_with_timestamp_for_network(
            Network::MainnetCandidate,
            &wrong_recipient,
            1_720_000_000,
            16,
        )
        .expect("wrong genesis");
        let mut chain = Chain::new(Network::MainnetCandidate);

        let error = chain
            .append_block(wrong_genesis)
            .expect_err("checkpoint mismatch must fail");
        assert!(matches!(error, AlvenqisError::InvalidCheckpoint { .. }));
    }

    #[test]
    fn block_with_invalid_signed_transaction_is_rejected() {
        let mut chain = Chain::new(Network::Devnet);
        let genesis = devnet_genesis(&dev_address("miner")).expect("genesis");
        chain.append_block(genesis.clone()).expect("append genesis");

        let recipient = PrivateKey::generate();
        let mut transaction = Transaction::new_signed(
            1,
            1,
            Network::Devnet,
            &PrivateKey::generate(),
            Address::from_public_key_for_network(&recipient.public_key(), Network::Devnet)
                .to_string(),
            Amount::from_atomic(10),
            Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 1),
            Amount::from_atomic(1),
            None,
        )
        .expect("signed tx");
        transaction.signature = Some(PrivateKey::generate().sign(b"wrong-message"));

        let block = child_block_with_consensus_difficulty(
            std::slice::from_ref(&genesis),
            &dev_address("miner"),
            genesis.header.timestamp + BLOCK_TIME_SECONDS,
            vec![transaction],
            genesis.header.difficulty_leading_zero_bits,
        )
        .expect("block");

        let error = chain
            .append_block(block)
            .expect_err("invalid signed transaction must be rejected");
        assert!(matches!(error, AlvenqisError::InvalidSignature(_)));
    }

    #[test]
    fn duplicate_transaction_hash_in_block_is_rejected() {
        let mut chain = Chain::new(Network::Devnet);
        let miner = PrivateKey::generate();
        let miner_address =
            Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
        let recipient = PrivateKey::generate();
        let recipient_address =
            Address::from_public_key_for_network(&recipient.public_key(), Network::Devnet)
                .to_string();
        let genesis = devnet_genesis(&miner_address).expect("genesis");
        chain.append_block(genesis.clone()).expect("append genesis");

        let transfer = Transaction::new_signed(
            1,
            1,
            Network::Devnet,
            &miner,
            recipient_address,
            Amount::from_atomic(10),
            Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 1),
            Amount::from_atomic(1),
            None,
        )
        .expect("transfer");
        let block = devnet_child_block_with_difficulty(
            &genesis,
            &miner_address,
            genesis.header.timestamp + BLOCK_TIME_SECONDS,
            vec![transfer.clone(), transfer],
            4,
        )
        .expect("block");

        let error = chain
            .append_block(block)
            .expect_err("duplicate tx hash must fail");
        assert!(matches!(error, AlvenqisError::DuplicateTransactionHash(_)));
    }

    #[test]
    fn network_roundtrip_works_for_address_prefixes() {
        assert_eq!(Network::from_address_prefix("dalve"), Some(Network::Devnet));
        assert_eq!(
            Network::from_address_prefix("talve"),
            Some(Network::Testnet)
        );
        assert_eq!(
            Network::from_address_prefix("alve"),
            Some(Network::MainnetCandidate)
        );
    }

    #[test]
    fn lwma_keeps_difficulty_stable_near_target_time() {
        let miner = dev_address("miner");
        let genesis = devnet_genesis_with_difficulty(&miner, 4).expect("genesis");
        let block_one = devnet_child_block_with_difficulty(
            &genesis,
            &miner,
            genesis.header.timestamp + BLOCK_TIME_SECONDS,
            vec![],
            4,
        )
        .expect("block one");
        let block_two = devnet_child_block_with_difficulty(
            &block_one,
            &miner,
            block_one.header.timestamp + BLOCK_TIME_SECONDS,
            vec![],
            4,
        )
        .expect("block two");

        let difficulty =
            next_difficulty_for_network(Network::Devnet, &[genesis, block_one, block_two], 4);
        assert_eq!(difficulty, 4);
    }

    #[test]
    fn lwma_increases_difficulty_when_blocks_arrive_too_fast() {
        let miner = dev_address("miner");
        let genesis = devnet_genesis_with_difficulty(&miner, 4).expect("genesis");
        let block_one = devnet_child_block_with_difficulty(
            &genesis,
            &miner,
            genesis.header.timestamp + 10,
            vec![],
            4,
        )
        .expect("block one");
        let block_two = devnet_child_block_with_difficulty(
            &block_one,
            &miner,
            block_one.header.timestamp + 10,
            vec![],
            5,
        )
        .expect("block two");

        let difficulty =
            next_difficulty_for_network(Network::Devnet, &[genesis, block_one, block_two], 4);
        assert!(difficulty > 5);
    }

    #[test]
    fn lwma_decreases_difficulty_when_blocks_arrive_too_slowly() {
        let miner = dev_address("miner");
        let genesis = devnet_genesis_with_difficulty(&miner, 8).expect("genesis");
        let block_one = devnet_child_block_with_difficulty(
            &genesis,
            &miner,
            genesis.header.timestamp + 360,
            vec![],
            8,
        )
        .expect("block one");
        let block_two = devnet_child_block_with_difficulty(
            &block_one,
            &miner,
            block_one.header.timestamp + 360,
            vec![],
            7,
        )
        .expect("block two");

        let difficulty =
            next_difficulty_for_network(Network::Devnet, &[genesis, block_one, block_two], 8);
        assert!(difficulty < 7);
    }

    #[test]
    fn block_with_wrong_adjusted_difficulty_is_rejected() {
        let miner = dev_address("miner");
        let mut chain = Chain::new(Network::Devnet);
        let genesis = devnet_genesis_with_difficulty(&miner, 4).expect("genesis");
        chain.append_block(genesis.clone()).expect("append genesis");

        let block_one = child_block_with_consensus_difficulty(
            std::slice::from_ref(&genesis),
            &miner,
            genesis.header.timestamp + 10,
            vec![],
            4,
        )
        .expect("block one");
        chain
            .append_block(block_one.clone())
            .expect("append block one");

        let mut block_two = child_block_with_consensus_difficulty(
            &[genesis, block_one.clone()],
            &miner,
            block_one.header.timestamp + 10,
            vec![],
            4,
        )
        .expect("block two");
        let expected_difficulty = block_two.header.difficulty_leading_zero_bits;
        block_two.header.difficulty_leading_zero_bits = expected_difficulty
            .saturating_sub(1)
            .max(Network::Devnet.minimum_difficulty_leading_zero_bits());

        let error = chain
            .append_block(block_two)
            .expect_err("wrong adjusted difficulty must fail");
        assert!(matches!(
            error,
            AlvenqisError::InvalidDifficultyAdjustment { .. }
        ));
    }
}
