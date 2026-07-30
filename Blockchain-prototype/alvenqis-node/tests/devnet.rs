use alvenqis_core::{
    check_pow, devnet_child_block_with_difficulty, devnet_genesis_with_difficulty,
    genesis_with_timestamp_for_network, mine_block as mine_core_block, Address, AlvenqisError,
    Amount, Hash, Network, PrivateKey, Transaction, BLOCK_TIME_SECONDS, INITIAL_BASE_FEE_ATOMIC,
    INITIAL_BLOCK_REWARD_ATOMIC, MAX_SUPPLY_ATOMIC,
};
use alvenqis_node::{
    adopt_candidate_chain, approve_genesis, balance, create_block_template, default_miner_address,
    genesis_approval_status, genesis_hash_hex_from_config, genesis_review_manifest, init_devnet,
    load_pending_transactions, local_p2p_handshake, mempool_status, mine_dev_blocks,
    mine_pending_block, node_status, reset_devnet, send_dev_tx, state, status, storage,
    submit_mined_block, submit_transaction, validate_chain, validate_p2p_handshake,
    write_genesis_review_manifest, NetworkConfig, NodeError, P2pHandshake, StatusReport,
    MAX_BLOCK_TEMPLATE_TRANSACTIONS,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

#[test]
fn higher_work_branch_is_atomically_adopted() {
    let (temp, config_path, data_dir, mempool_dir) = setup_paths();
    let candidate_dir = temp.path().join(".alvenqis-dev/candidate-chain");
    let current_miner = generate_address().1;
    let candidate_miner = generate_address().1;
    init_devnet(&config_path, &data_dir, &current_miner).expect("current genesis");
    clone_genesis_fixture(&data_dir, &candidate_dir);
    mine_dev_blocks(&config_path, &data_dir, &current_miner, 1).expect("current branch");
    mine_dev_blocks(&config_path, &candidate_dir, &candidate_miner, 2).expect("higher-work branch");

    let candidate = storage::load_blocks(&candidate_dir).expect("candidate blocks");
    let summary = adopt_candidate_chain(&config_path, &data_dir, &mempool_dir, &candidate)
        .expect("adopt higher-work branch");
    assert_eq!(summary.common_ancestor_height, 0);
    assert_eq!(summary.detached_blocks, 1);
    assert_eq!(summary.attached_blocks, 2);
    assert!(summary.new_chain_work > summary.previous_chain_work);
    assert_eq!(
        storage::load_blocks(&data_dir).expect("adopted blocks"),
        candidate
    );
}

#[test]
fn invalid_reorg_candidate_never_changes_canonical_storage() {
    let (temp, config_path, data_dir, mempool_dir) = setup_paths();
    let candidate_dir = temp.path().join(".alvenqis-dev/invalid-candidate-chain");
    let miner = generate_address().1;
    init_devnet(&config_path, &data_dir, &miner).expect("current genesis");
    clone_genesis_fixture(&data_dir, &candidate_dir);
    mine_dev_blocks(&config_path, &data_dir, &miner, 1).expect("current branch");
    mine_dev_blocks(&config_path, &candidate_dir, &miner, 2).expect("candidate branch");
    let before = fs::read(storage::chain_file_path(&data_dir)).expect("canonical bytes");
    let mut candidate = storage::load_blocks(&candidate_dir).expect("candidate blocks");
    candidate[1].header.merkle_root = Hash::zero();

    adopt_candidate_chain(&config_path, &data_dir, &mempool_dir, &candidate)
        .expect_err("invalid branch must fail");
    assert_eq!(
        fs::read(storage::chain_file_path(&data_dir)).expect("canonical bytes after rejection"),
        before
    );
}

#[test]
fn detached_valid_transaction_returns_to_mempool_after_reorg() {
    let (temp, config_path, data_dir, mempool_dir) = setup_paths();
    let candidate_dir = temp.path().join(".alvenqis-dev/reorg-candidate-chain");
    let sender_key = PrivateKey::generate();
    let sender =
        Address::from_public_key_for_network(&sender_key.public_key(), Network::Devnet).to_string();
    let recipient = generate_address().1;
    let candidate_miner = generate_address().1;
    init_devnet(&config_path, &data_dir, &sender).expect("current genesis");
    clone_genesis_fixture(&data_dir, &candidate_dir);
    let transaction = Transaction::new_signed(
        1,
        1,
        Network::Devnet,
        &sender_key,
        recipient,
        Amount::from_atomic(100),
        Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 1),
        Amount::from_atomic(1),
        None,
    )
    .expect("signed transaction");
    let tx_hash = alvenqis_core::hash_to_hex(&transaction.tx_hash());
    submit_transaction(&data_dir, &mempool_dir, 8, &transaction).expect("submit transaction");
    mine_pending_block(&config_path, &data_dir, &mempool_dir, &sender)
        .expect("mine transaction on detached branch");
    mine_dev_blocks(&config_path, &candidate_dir, &candidate_miner, 2)
        .expect("mine higher-work candidate");

    let candidate = storage::load_blocks(&candidate_dir).expect("candidate blocks");
    adopt_candidate_chain(&config_path, &data_dir, &mempool_dir, &candidate)
        .expect("adopt candidate");
    let pending = load_pending_transactions(&mempool_dir).expect("restored mempool");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].tx_hash, tx_hash);
    assert_eq!(pending[0].transaction, transaction);
}

fn clone_genesis_fixture(source_dir: &Path, candidate_dir: &Path) {
    fs::create_dir_all(candidate_dir).expect("create candidate chain directory");
    fs::copy(
        storage::chain_file_path(source_dir),
        storage::chain_file_path(candidate_dir),
    )
    .expect("copy canonical genesis into candidate chain");
}

fn write_network_config(path: &Path, network: Network, genesis_approval_path: Option<&Path>) {
    let approval_path_text =
        genesis_approval_path.map(|path| path.to_string_lossy().replace('\\', "/"));
    let network_name = match network {
        Network::Devnet => "devnet",
        Network::Testnet => "testnet",
        Network::MainnetCandidate => "mainnet-candidate",
    };
    let (difficulty, allow_mainnet_candidate) = match network {
        Network::Devnet => (4, false),
        Network::Testnet => (12, false),
        Network::MainnetCandidate => (16, true),
    };
    let content = format!(
        r#"
network = "{network_name}"
network_id = "{network_id}"
human_name = "{human_name}"
status_label = "{status_label}"
block_time_seconds = 60
difficulty_leading_zero_bits = {difficulty}
ticker = "ALVE"
address_prefix = "{address_prefix}"
max_supply = "60000000"
halving_interval = 1576800
initial_block_reward = "19.02587519"
default_rpc_port = {rpc_port}
default_p2p_port = {p2p_port}
max_mempool_transactions = 8
genesis_config_path = "{genesis_path}"
{genesis_approval_line}chain_magic_hex = "{chain_magic_hex}"
allow_mainnet_candidate = {allow_mainnet_candidate}
"#,
        network_name = network_name,
        network_id = network.network_id(),
        human_name = network.human_name(),
        status_label = network.status_label(),
        difficulty = difficulty,
        address_prefix = network.address_prefix(),
        rpc_port = network.default_rpc_port(),
        p2p_port = network.default_p2p_port(),
        genesis_path = network.genesis_config_path(),
        genesis_approval_line = approval_path_text
            .as_deref()
            .map(|path| format!("genesis_approval_path = \"{path}\"\n"))
            .unwrap_or_default(),
        chain_magic_hex = network
            .chain_magic_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        allow_mainnet_candidate = allow_mainnet_candidate,
    );
    fs::create_dir_all(path.parent().expect("config path must have parent")).expect("config dir");
    fs::write(path, content.trim_start()).expect("config file");
}

fn setup_paths() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
    let temp_dir = tempdir().expect("tempdir");
    let config_path = temp_dir.path().join("alvenqis-devnet/config/devnet.toml");
    let data_dir = temp_dir.path().join(".alvenqis-dev/chain");
    let mempool_dir = temp_dir.path().join(".alvenqis-dev/mempool");
    write_network_config(&config_path, Network::Devnet, None);
    (temp_dir, config_path, data_dir, mempool_dir)
}

fn setup_mainnet_candidate_paths() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf, PathBuf) {
    let temp_dir = tempdir().expect("tempdir");
    let config_path = temp_dir.path().join("configs/mainnet-candidate.toml");
    let data_dir = temp_dir.path().join(".alvenqis-mainnet/chain");
    let mempool_dir = temp_dir.path().join(".alvenqis-mainnet/mempool");
    let approval_path = temp_dir
        .path()
        .join("docs/release/GENESIS_APPROVAL.mainnet-candidate.json");
    write_network_config(
        &config_path,
        Network::MainnetCandidate,
        Some(&approval_path),
    );
    let workspace_genesis_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("configs/genesis.mainnet-candidate.toml");
    let temp_genesis_path = temp_dir
        .path()
        .join("configs/genesis.mainnet-candidate.toml");
    fs::create_dir_all(temp_genesis_path.parent().expect("temp genesis parent"))
        .expect("temp genesis dir");
    fs::copy(&workspace_genesis_path, &temp_genesis_path).expect("copy genesis config");
    (temp_dir, config_path, data_dir, mempool_dir, approval_path)
}

fn append_candidate_genesis(data_dir: &Path) {
    storage::ensure_data_dir(data_dir).expect("ensure dir");
    let genesis = genesis_with_timestamp_for_network(
        Network::MainnetCandidate,
        &default_miner_address(Network::MainnetCandidate),
        1_720_000_000,
        16,
    )
    .expect("candidate genesis");
    storage::append_block(data_dir, &genesis).expect("append genesis");
}

fn generate_address() -> (PrivateKey, String) {
    let private_key = PrivateKey::generate();
    let address = Address::from_public_key_for_network(&private_key.public_key(), Network::Devnet)
        .to_string();
    (private_key, address)
}

#[test]
fn devnet_genesis_creation_works() {
    let (_temp_dir, config_path, data_dir, _mempool_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    let summary = init_devnet(&config_path, &data_dir, &miner_address).expect("init devnet");

    assert_eq!(summary.height, 0);
    assert_eq!(summary.block_count, 1);
    assert!(storage::chain_file_path(&data_dir).exists());
}

#[test]
fn appending_mined_dev_blocks_works() {
    let (_temp_dir, config_path, data_dir, _mempool_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init devnet");
    let summary = mine_dev_blocks(&config_path, &data_dir, &miner_address, 3).expect("mine blocks");

    assert_eq!(summary.height, 3);
    assert_eq!(summary.block_count, 4);
}

#[test]
fn persisted_chain_can_be_loaded_again() {
    let (_temp_dir, config_path, data_dir, _mempool_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init devnet");
    mine_dev_blocks(&config_path, &data_dir, &miner_address, 2).expect("mine blocks");

    let summary = validate_chain(&config_path, &data_dir).expect("validate");
    let report = status(&config_path, &data_dir).expect("status");

    match report {
        StatusReport::Ready(ready) => {
            assert_eq!(ready.height, summary.height);
            assert_eq!(ready.tip_hash, summary.tip_hash);
        }
        StatusReport::Uninitialized { .. } => panic!("devnet should be initialized"),
    }
}

#[test]
fn invalid_chain_file_is_rejected() {
    let (_temp_dir, config_path, data_dir, _mempool_dir) = setup_paths();
    storage::ensure_data_dir(&data_dir).expect("ensure dir");
    fs::write(storage::chain_file_path(&data_dir), "{not-json}\n").expect("invalid chain file");

    let error = validate_chain(&config_path, &data_dir).expect_err("invalid chain should fail");
    assert!(matches!(
        error,
        NodeError::Sqlite(_) | NodeError::InvalidChainDatabase { .. }
    ));
}

#[test]
fn reset_removes_devnet_local_data_only() {
    let (_temp_dir, config_path, data_dir, mempool_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init devnet");
    let sibling_file = data_dir
        .parent()
        .expect("data dir parent")
        .join("keep-me.txt");
    fs::write(&sibling_file, "preserve").expect("sibling file");
    fs::create_dir_all(&mempool_dir).expect("mempool dir");
    fs::write(mempool_dir.join("pending.json"), "[]").expect("mempool file");

    let summary = reset_devnet(&config_path, &data_dir, &mempool_dir, true).expect("reset");

    assert!(!storage::chain_file_path(&data_dir).exists());
    assert!(data_dir.exists());
    assert!(!mempool_dir.join("pending.json").exists());
    assert!(mempool_dir.exists());
    assert!(sibling_file.exists());
    assert!(summary.backup_dir.is_some());
}

#[test]
fn persisted_chain_can_rebuild_ledger_state() {
    let (_temp_dir, config_path, data_dir, _mempool_dir) = setup_paths();
    let (miner_key, miner_address) = generate_address();
    let (_recipient_key, recipient_address) = generate_address();

    init_devnet(&config_path, &data_dir, &miner_address).expect("init devnet");
    send_dev_tx(
        &config_path,
        &data_dir,
        &miner_key.to_hex(),
        &recipient_address,
        100,
        1,
        &miner_address,
    )
    .expect("send dev tx");

    let summary = state(&config_path, &data_dir).expect("state");
    let recipient_balance = balance(&config_path, &data_dir, &recipient_address).expect("balance");

    assert_eq!(summary.height, 1);
    assert_eq!(recipient_balance.balance_atomic, 100);
    assert!(summary.tracked_addresses >= 2);
}

#[test]
fn valid_transaction_enters_mempool() {
    let (_temp_dir, config_path, data_dir, mempool_dir) = setup_paths();
    let (miner_key, miner_address) = generate_address();
    let (_recipient_key, recipient_address) = generate_address();
    init_devnet(&config_path, &data_dir, &miner_address).expect("init devnet");

    let transaction = alvenqis_core::Transaction::new_signed(
        1,
        1,
        Network::Devnet,
        &miner_key,
        recipient_address,
        alvenqis_core::Amount::from_atomic(100),
        alvenqis_core::Amount::from_atomic(alvenqis_core::INITIAL_BASE_FEE_ATOMIC + 1),
        alvenqis_core::Amount::from_atomic(1),
        None,
    )
    .expect("signed tx");
    let summary = submit_transaction(&data_dir, &mempool_dir, 8, &transaction).expect("submit tx");

    assert_eq!(summary.lifecycle_status, "pending");
    assert_eq!(summary.mempool_size, 1);
    assert_eq!(
        load_pending_transactions(&mempool_dir).expect("load").len(),
        1
    );
}

#[test]
fn duplicate_transaction_is_rejected() {
    let (_temp_dir, config_path, data_dir, mempool_dir) = setup_paths();
    let (miner_key, miner_address) = generate_address();
    let (_recipient_key, recipient_address) = generate_address();
    init_devnet(&config_path, &data_dir, &miner_address).expect("init devnet");

    let transaction = alvenqis_core::Transaction::new_signed(
        1,
        1,
        Network::Devnet,
        &miner_key,
        recipient_address,
        alvenqis_core::Amount::from_atomic(100),
        alvenqis_core::Amount::from_atomic(alvenqis_core::INITIAL_BASE_FEE_ATOMIC + 1),
        alvenqis_core::Amount::from_atomic(1),
        None,
    )
    .expect("signed tx");
    submit_transaction(&data_dir, &mempool_dir, 8, &transaction).expect("submit tx");
    let error = submit_transaction(&data_dir, &mempool_dir, 8, &transaction)
        .expect_err("duplicate tx must fail");
    assert!(matches!(error, NodeError::Input(_)));
}

#[test]
fn mempool_capacity_limit_is_enforced() {
    let (_temp_dir, config_path, data_dir, mempool_dir) = setup_paths();
    let (miner_key, miner_address) = generate_address();
    let (_recipient_key, recipient_address) = generate_address();
    init_devnet(&config_path, &data_dir, &miner_address).expect("init devnet");

    let first = Transaction::new_signed(
        1,
        1,
        Network::Devnet,
        &miner_key,
        recipient_address.clone(),
        Amount::from_atomic(100),
        Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 1),
        Amount::from_atomic(1),
        None,
    )
    .expect("first");
    let second = Transaction::new_signed(
        1,
        2,
        Network::Devnet,
        &miner_key,
        recipient_address,
        Amount::from_atomic(100),
        Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 1),
        Amount::from_atomic(1),
        None,
    )
    .expect("second");

    submit_transaction(&data_dir, &mempool_dir, 1, &first).expect("submit first");
    let error = submit_transaction(&data_dir, &mempool_dir, 1, &second)
        .expect_err("second transaction must exceed mempool capacity");
    assert!(matches!(error, NodeError::MempoolFull { limit: 1 }));
}

#[test]
fn invalid_signature_is_rejected_by_mempool() {
    let (_temp_dir, config_path, data_dir, mempool_dir) = setup_paths();
    let (miner_key, miner_address) = generate_address();
    let (_recipient_key, recipient_address) = generate_address();
    init_devnet(&config_path, &data_dir, &miner_address).expect("init devnet");

    let mut transaction = alvenqis_core::Transaction::new_signed(
        1,
        1,
        Network::Devnet,
        &miner_key,
        recipient_address,
        alvenqis_core::Amount::from_atomic(100),
        alvenqis_core::Amount::from_atomic(alvenqis_core::INITIAL_BASE_FEE_ATOMIC + 1),
        alvenqis_core::Amount::from_atomic(1),
        None,
    )
    .expect("signed tx");
    transaction.signature = Some(PrivateKey::generate().sign(b"wrong"));

    let error = submit_transaction(&data_dir, &mempool_dir, 8, &transaction)
        .expect_err("invalid signature must fail");
    assert!(matches!(
        error,
        NodeError::Core(alvenqis_core::AlvenqisError::InvalidSignature(_))
    ));
}

#[test]
fn insufficient_balance_is_rejected_by_mempool() {
    let (_temp_dir, config_path, data_dir, mempool_dir) = setup_paths();
    let (miner_key, miner_address) = generate_address();
    let (_recipient_key, recipient_address) = generate_address();
    init_devnet(&config_path, &data_dir, &miner_address).expect("init devnet");

    let transaction = alvenqis_core::Transaction::new_signed(
        1,
        1,
        Network::Devnet,
        &miner_key,
        recipient_address,
        alvenqis_core::Amount::from_atomic(alvenqis_core::INITIAL_BLOCK_REWARD_ATOMIC + 1),
        alvenqis_core::Amount::from_atomic(alvenqis_core::INITIAL_BASE_FEE_ATOMIC + 1),
        alvenqis_core::Amount::from_atomic(1),
        None,
    )
    .expect("signed tx");

    let error = submit_transaction(&data_dir, &mempool_dir, 8, &transaction)
        .expect_err("insufficient balance must fail");
    assert!(matches!(
        error,
        NodeError::Core(alvenqis_core::AlvenqisError::InsufficientBalance { .. })
    ));
}

#[test]
fn mining_pending_transactions_updates_chain_and_clears_mempool() {
    let (_temp_dir, config_path, data_dir, mempool_dir) = setup_paths();
    let (miner_key, miner_address) = generate_address();
    let (_recipient_key, recipient_address) = generate_address();
    init_devnet(&config_path, &data_dir, &miner_address).expect("init devnet");

    let transaction = alvenqis_core::Transaction::new_signed(
        1,
        1,
        Network::Devnet,
        &miner_key,
        recipient_address.clone(),
        alvenqis_core::Amount::from_atomic(100),
        alvenqis_core::Amount::from_atomic(alvenqis_core::INITIAL_BASE_FEE_ATOMIC + 7),
        alvenqis_core::Amount::from_atomic(7),
        None,
    )
    .expect("signed tx");
    let tx_hash = alvenqis_core::hash_to_hex(&transaction.tx_hash());
    submit_transaction(&data_dir, &mempool_dir, 8, &transaction).expect("submit tx");

    let mined = mine_pending_block(&config_path, &data_dir, &mempool_dir, &miner_address)
        .expect("mine pending block");
    let recipient_balance = balance(&config_path, &data_dir, &recipient_address).expect("balance");
    let mempool = mempool_status(&data_dir, &mempool_dir).expect("mempool");
    let ledger_state = state(&config_path, &data_dir).expect("state");

    assert_eq!(mined.included_tx_hashes, vec![tx_hash]);
    assert_eq!(recipient_balance.balance_atomic, 100);
    assert_eq!(mempool.pending_count, 0);
    assert_eq!(
        ledger_state.latest_block_base_fee_atomic,
        INITIAL_BASE_FEE_ATOMIC
    );
    assert_eq!(ledger_state.latest_block_fees_atomic, 8);
    assert_eq!(ledger_state.latest_block_priority_fees_atomic, 7);
    assert_eq!(
        ledger_state.latest_block_burned_fees_atomic,
        INITIAL_BASE_FEE_ATOMIC
    );
}

#[test]
fn remote_block_template_can_be_mined_and_submitted() {
    let (_temp_dir, config_path, data_dir, mempool_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init devnet");

    let template = create_block_template(&config_path, &data_dir, &mempool_dir, &miner_address, 8)
        .expect("create template");
    assert_eq!(template.block.header.height, 1);
    assert_eq!(template.block.header.nonce, 0);
    assert_eq!(template.block.transactions.len(), 1);

    let mut mined_block = template.block;
    mine_core_block(&mut mined_block);
    let submitted = submit_mined_block(&config_path, &data_dir, &mempool_dir, &mined_block)
        .expect("submit mined block");
    let summary = validate_chain(&config_path, &data_dir).expect("validate persisted chain");

    assert_eq!(submitted.block_height, 1);
    assert!(submitted.accepted_tx_hashes.is_empty());
    assert!(submitted.mempool_cleanup_complete);
    assert_eq!(summary.height, 1);
}

#[test]
fn rejected_remote_block_does_not_change_persisted_chain() {
    let (_temp_dir, config_path, data_dir, mempool_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init devnet");
    let mut candidate =
        create_block_template(&config_path, &data_dir, &mempool_dir, &miner_address, 8)
            .expect("create template")
            .block;
    candidate.header.merkle_root = Hash::zero();
    mine_core_block(&mut candidate);

    let error = submit_mined_block(&config_path, &data_dir, &mempool_dir, &candidate)
        .expect_err("invalid block must fail");
    let summary = validate_chain(&config_path, &data_dir).expect("chain remains valid");

    assert!(matches!(
        error,
        NodeError::Core(AlvenqisError::InvalidMerkleRoot)
    ));
    assert_eq!(summary.height, 0);
    assert_eq!(summary.block_count, 1);
}

#[test]
fn stale_remote_block_cannot_be_committed_twice() {
    let (_temp_dir, config_path, data_dir, mempool_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init devnet");
    let mut candidate =
        create_block_template(&config_path, &data_dir, &mempool_dir, &miner_address, 8)
            .expect("create template")
            .block;
    mine_core_block(&mut candidate);
    submit_mined_block(&config_path, &data_dir, &mempool_dir, &candidate)
        .expect("first submission");

    submit_mined_block(&config_path, &data_dir, &mempool_dir, &candidate)
        .expect_err("stale submission must fail");
    let summary = validate_chain(&config_path, &data_dir).expect("chain remains valid");
    assert_eq!(summary.height, 1);
    assert_eq!(summary.block_count, 2);
}

#[test]
fn remote_block_template_request_is_bounded() {
    let (_temp_dir, config_path, data_dir, mempool_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init devnet");

    for invalid_limit in [0, MAX_BLOCK_TEMPLATE_TRANSACTIONS + 1] {
        let error = create_block_template(
            &config_path,
            &data_dir,
            &mempool_dir,
            &miner_address,
            invalid_limit,
        )
        .expect_err("invalid bound must fail");
        assert!(matches!(error, NodeError::Input(_)));
    }
}

#[test]
fn mainnet_candidate_reset_is_rejected() {
    let temp_dir = tempdir().expect("tempdir");
    let config_path = temp_dir
        .path()
        .join("alvenqis-devnet/config/mainnet-candidate.toml");
    let data_dir = temp_dir.path().join(".alvenqis-mainnet/chain");
    let mempool_dir = temp_dir.path().join(".alvenqis-mainnet/mempool");
    let approval_path = temp_dir
        .path()
        .join("docs/release/GENESIS_APPROVAL.mainnet-candidate.json");
    write_network_config(
        &config_path,
        Network::MainnetCandidate,
        Some(&approval_path),
    );

    let error = reset_devnet(&config_path, &data_dir, &mempool_dir, true)
        .expect_err("mainnet candidate reset must fail");
    assert!(matches!(error, NodeError::ResetNotAllowed(_)));
}

#[test]
fn reset_requires_confirmation() {
    let (_temp_dir, config_path, data_dir, mempool_dir) = setup_paths();
    let error = reset_devnet(&config_path, &data_dir, &mempool_dir, false)
        .expect_err("reset without confirmation must fail");
    assert!(matches!(error, NodeError::ResetConfirmationRequired(_)));
}

#[test]
fn wrong_genesis_is_rejected() {
    let (_temp_dir, config_path, data_dir, _mempool_dir, approval_path) =
        setup_mainnet_candidate_paths();
    let review_path = approval_path.with_file_name("GENESIS_REVIEW.mainnet-candidate.json");
    write_genesis_review_manifest(&config_path, &review_path).expect("write review");
    approve_genesis(
        &config_path,
        &review_path,
        "integration-test",
        Some("wrong genesis coverage"),
        Some(&approval_path),
    )
    .expect("approve genesis");
    storage::ensure_data_dir(&data_dir).expect("ensure dir");

    let wrong_address = Address::from_public_key_for_network(
        &PrivateKey::generate().public_key(),
        Network::MainnetCandidate,
    )
    .to_string();
    let wrong_genesis =
        genesis_with_timestamp_for_network(Network::MainnetCandidate, &wrong_address, 1, 16)
            .expect("wrong genesis");
    storage::append_block(&data_dir, &wrong_genesis).expect("append wrong genesis");

    let error = validate_chain(&config_path, &data_dir).expect_err("wrong genesis must fail");
    assert!(matches!(error, NodeError::ConfigMismatch(_)));
}

#[test]
fn mainnet_candidate_requires_genesis_approval_before_validation() {
    let (_temp_dir, config_path, data_dir, _mempool_dir, _approval_path) =
        setup_mainnet_candidate_paths();
    append_candidate_genesis(&data_dir);

    let error = validate_chain(&config_path, &data_dir).expect_err("missing approval must fail");
    assert!(matches!(error, NodeError::Input(_)));
}

#[test]
fn broken_previous_hash_is_rejected() {
    let (_temp_dir, config_path, data_dir, _mempool_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    let genesis = devnet_genesis_with_difficulty(&miner_address, 4).expect("genesis");
    storage::ensure_data_dir(&data_dir).expect("ensure dir");
    storage::append_block(&data_dir, &genesis).expect("append genesis");

    let mut child = devnet_child_block_with_difficulty(
        &genesis,
        &miner_address,
        genesis.header.timestamp + BLOCK_TIME_SECONDS,
        vec![],
        4,
    )
    .expect("child");
    child.header.previous_hash = Hash::zero();
    // Production append refuses broken tip links (A-H04); fixtures use unchecked.
    let refuse = storage::append_block(&data_dir, &child).expect_err("tip link must refuse");
    assert!(matches!(refuse, NodeError::StaleChainTip { .. }));
    storage::append_block_unchecked(&data_dir, &child).expect("append invalid child fixture");

    let error = validate_chain(&config_path, &data_dir).expect_err("invalid previous hash");
    // Storage structural load now rejects broken previous_hash links before full consensus.
    assert!(
        matches!(error, NodeError::InvalidChainFile { .. })
            || matches!(
                error,
                NodeError::Core(AlvenqisError::InvalidPreviousHash { .. })
            ),
        "unexpected error: {error}"
    );
}

#[test]
fn invalid_merkle_root_is_rejected() {
    let (_temp_dir, config_path, data_dir, _mempool_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    let genesis = devnet_genesis_with_difficulty(&miner_address, 4).expect("genesis");
    storage::ensure_data_dir(&data_dir).expect("ensure dir");
    storage::append_block(&data_dir, &genesis).expect("append genesis");

    let mut child = devnet_child_block_with_difficulty(
        &genesis,
        &miner_address,
        genesis.header.timestamp + BLOCK_TIME_SECONDS,
        vec![],
        4,
    )
    .expect("child");
    child.header.merkle_root = Hash::zero();
    storage::append_block(&data_dir, &child).expect("append invalid child");

    let error = validate_chain(&config_path, &data_dir).expect_err("invalid merkle root");
    assert!(
        matches!(error, NodeError::InvalidChainDatabase { .. })
            || matches!(error, NodeError::Core(AlvenqisError::InvalidMerkleRoot)),
        "unexpected error: {error}"
    );
}

#[test]
fn invalid_pow_is_rejected() {
    let (_temp_dir, config_path, data_dir, _mempool_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    let genesis = devnet_genesis_with_difficulty(&miner_address, 4).expect("genesis");
    storage::ensure_data_dir(&data_dir).expect("ensure dir");
    storage::append_block(&data_dir, &genesis).expect("append genesis");

    let mut child = devnet_child_block_with_difficulty(
        &genesis,
        &miner_address,
        genesis.header.timestamp + BLOCK_TIME_SECONDS,
        vec![],
        4,
    )
    .expect("child");
    while check_pow(
        &child.pow_hash().expect("pow eval"),
        child.header.difficulty_leading_zero_bits,
    ) {
        child.header.nonce = child.header.nonce.wrapping_add(1);
    }
    storage::append_block(&data_dir, &child).expect("append invalid child");

    let error = validate_chain(&config_path, &data_dir).expect_err("invalid pow");
    assert!(matches!(
        error,
        NodeError::Core(AlvenqisError::InvalidPow { .. })
    ));
}

#[test]
fn zero_amount_is_rejected() {
    let (miner_key, _miner_address) = generate_address();
    let (_recipient_key, recipient_address) = generate_address();
    let error = Transaction::new_signed(
        1,
        1,
        Network::Devnet,
        &miner_key,
        recipient_address,
        Amount::ZERO,
        Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 1),
        Amount::from_atomic(1),
        None,
    )
    .expect_err("zero amount must fail");
    assert!(matches!(error, AlvenqisError::ZeroAmountTransaction));
}

#[test]
fn invalid_fee_is_rejected() {
    let (miner_key, _miner_address) = generate_address();
    let (_recipient_key, recipient_address) = generate_address();
    let error = Transaction::new_signed(
        1,
        1,
        Network::Devnet,
        &miner_key,
        recipient_address,
        Amount::from_atomic(1),
        Amount::from_atomic(MAX_SUPPLY_ATOMIC + 1),
        Amount::from_atomic(1),
        None,
    )
    .expect_err("fee over max supply bound must fail");
    assert!(matches!(error, AlvenqisError::InvalidFee(_)));
}

#[test]
fn coinbase_reward_above_allowed_reward_is_rejected() {
    let (_temp_dir, config_path, data_dir, _mempool_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    let genesis = devnet_genesis_with_difficulty(&miner_address, 4).expect("genesis");
    storage::ensure_data_dir(&data_dir).expect("ensure dir");
    storage::append_block(&data_dir, &genesis).expect("append genesis");

    let excessive_coinbase = Transaction::coinbase(
        1,
        miner_address.clone(),
        Amount::from_atomic(INITIAL_BLOCK_REWARD_ATOMIC + 1),
    )
    .expect("coinbase");
    let mut invalid_block = alvenqis_core::Block::new(
        Network::Devnet,
        1,
        genesis.hash().expect("genesis hash"),
        alvenqis_core::initial_base_fee().as_atomic(),
        genesis.header.timestamp + BLOCK_TIME_SECONDS,
        4,
        vec![excessive_coinbase],
    )
    .expect("invalid block");
    // Proper FiroPoW mine so validation fails on coinbase amount, not PoW/mix.
    alvenqis_core::mine_block(&mut invalid_block);
    storage::append_block(&data_dir, &invalid_block).expect("append invalid block");

    let error = validate_chain(&config_path, &data_dir).expect_err("reward overflow");
    assert!(
        matches!(
            error,
            NodeError::Core(AlvenqisError::InvalidCoinbaseReward { .. })
                | NodeError::Core(AlvenqisError::InvalidCoinbaseAmount { .. })
                | NodeError::Core(AlvenqisError::InvalidDifficultyAdjustment { .. })
        ),
        "unexpected error: {error:?}"
    );
}

#[test]
fn wrong_network_p2p_handshake_is_rejected() {
    let (_temp_dir, config_path, _data_dir, _mempool_dir) = setup_paths();
    let config = alvenqis_node::NetworkConfig::load_from_path(&config_path).expect("config");
    let mut remote = local_p2p_handshake(&config);
    remote.network_id = Network::Testnet.network_id().to_owned();

    let error = validate_p2p_handshake(&config, &remote).expect_err("network mismatch must fail");
    assert!(matches!(error, NodeError::NetworkMismatch { .. }));
}

#[test]
fn wrong_chain_magic_p2p_handshake_is_rejected() {
    let (_temp_dir, config_path, _data_dir, _mempool_dir) = setup_paths();
    let config = alvenqis_node::NetworkConfig::load_from_path(&config_path).expect("config");
    let remote = P2pHandshake {
        network_id: Network::Devnet.network_id().to_owned(),
        chain_magic_hex: "00000000".to_owned(),
        p2p_port: Network::Devnet.default_p2p_port(),
    };

    let error =
        validate_p2p_handshake(&config, &remote).expect_err("chain magic mismatch must fail");
    assert!(matches!(error, NodeError::ChainMagicMismatch { .. }));
}

#[test]
fn genesis_hash_is_deterministic() {
    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("configs/mainnet-candidate.toml");

    let first = genesis_hash_hex_from_config(&config_path).expect("first hash");
    let second = genesis_hash_hex_from_config(&config_path).expect("second hash");

    assert_eq!(first, second);
    assert_eq!(
        first,
        "0000c29213014578ac41a748c2be3489859f1e0b1f3555bd89b7e5301632a4c5"
    );
}

#[test]
fn genesis_review_manifest_is_deterministic() {
    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("configs/mainnet-candidate.toml");

    let first = genesis_review_manifest(&config_path).expect("first manifest");
    let second = genesis_review_manifest(&config_path).expect("second manifest");

    assert_eq!(first, second);
    assert_eq!(
        first.deterministic_genesis_hash,
        "0000c29213014578ac41a748c2be3489859f1e0b1f3555bd89b7e5301632a4c5"
    );
    assert_eq!(first.human_name, "Alvenqis Mainnet Candidate");
    // Must match Blockchain-docs/human/release/GENESIS_REVIEW.mainnet-candidate.json
    // (and GENESIS_APPROVAL approved_review_hash).
    assert_eq!(
        first.review_hash,
        "3b245dfb0004603200d804996e24375728b12cf209a88ce61ced23e38e5a602c"
    );

    let active = alvenqis_node::NetworkConfig::load_from_path(&config_path).expect("config");
    assert_eq!(active.human_name, "Alvenqis Mainnet Candidate");
}

#[test]
fn mainnet_candidate_imports_the_pinned_premined_genesis() {
    // Pinned genesis + approval live under monorepo Blockchain-docs (layout split
    // from the older in-tree docs/release path). Stage a workspace-shaped temp
    // tree so resolve_config_path can still find relative approval/genesis inputs.
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let monorepo = workspace.parent().expect("monorepo root");
    let release_docs = monorepo
        .join("Blockchain-docs")
        .join("human")
        .join("release");
    let genesis_path = release_docs.join("genesis.mainnet-candidate.block.json");
    let approval_src = release_docs.join("GENESIS_APPROVAL.mainnet-candidate.json");
    assert!(
        genesis_path.is_file(),
        "pinned premined genesis missing at {}",
        genesis_path.display()
    );
    assert!(
        approval_src.is_file(),
        "pinned genesis approval missing at {}",
        approval_src.display()
    );

    let temp_dir = tempdir().expect("tempdir");
    let data_dir = temp_dir.path().join(".alvenqis-mainnet/chain");
    let config_path = temp_dir.path().join("configs/mainnet-candidate.toml");
    let genesis_config_path = temp_dir
        .path()
        .join("configs/genesis.mainnet-candidate.toml");
    let approval_path = temp_dir
        .path()
        .join("docs/release/GENESIS_APPROVAL.mainnet-candidate.json");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("config dir");
    fs::create_dir_all(approval_path.parent().expect("approval parent")).expect("approval dir");
    fs::copy(
        workspace.join("configs/mainnet-candidate.toml"),
        &config_path,
    )
    .expect("stage mainnet-candidate config");
    fs::copy(
        workspace.join("configs/genesis.mainnet-candidate.toml"),
        &genesis_config_path,
    )
    .expect("stage genesis config");
    fs::copy(&approval_src, &approval_path).expect("stage genesis approval");

    let imported =
        alvenqis_node::import_genesis_block(&config_path, &data_dir, &genesis_path, false)
            .expect("import pinned genesis");

    assert_eq!(
        imported,
        "0000c29213014578ac41a748c2be3489859f1e0b1f3555bd89b7e5301632a4c5"
    );
    let summary = validate_chain(&config_path, &data_dir).expect("validate imported chain");
    assert_eq!(summary.height, 0);
    assert_eq!(summary.tip_hash, imported);
}

#[test]
fn mainnet_candidate_genesis_approval_status_is_valid() {
    let (_temp_dir, config_path, data_dir, _mempool_dir, approval_path) =
        setup_mainnet_candidate_paths();
    let review_path = approval_path.with_file_name("GENESIS_REVIEW.mainnet-candidate.json");
    let manifest =
        write_genesis_review_manifest(&config_path, &review_path).expect("write review manifest");
    let approval = approve_genesis(
        &config_path,
        &review_path,
        "integration-test",
        Some("approval status coverage"),
        Some(&approval_path),
    )
    .expect("approve genesis");
    append_candidate_genesis(&data_dir);
    let summary = validate_chain(&config_path, &data_dir).expect("validate chain");
    let status = genesis_approval_status(&config_path).expect("approval status");

    assert!(approval.approved);
    assert!(status.approved);
    assert_eq!(status.review_hash, manifest.review_hash);
    assert_eq!(summary.height, 0);
}

#[test]
fn tampered_genesis_approval_hash_is_rejected() {
    let (_temp_dir, config_path, data_dir, _mempool_dir, approval_path) =
        setup_mainnet_candidate_paths();
    let review_path = approval_path.with_file_name("GENESIS_REVIEW.mainnet-candidate.json");
    write_genesis_review_manifest(&config_path, &review_path).expect("write review manifest");
    approve_genesis(
        &config_path,
        &review_path,
        "integration-test",
        Some("tamper coverage"),
        Some(&approval_path),
    )
    .expect("approve genesis");
    let mut approval_value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&approval_path).expect("read approval"))
            .expect("approval json");
    // validate_chain / startup use the pinned-approval hot path: they bind the
    // stored genesis to approval.deterministic_genesis_hash without re-mining
    // the review manifest (re-mine is reserved for genesis_approval_status /
    // approve-genesis). Tamper the hash that path actually enforces.
    approval_value["deterministic_genesis_hash"] = serde_json::Value::String("00".repeat(32));
    fs::write(
        &approval_path,
        serde_json::to_string_pretty(&approval_value).expect("serialize approval"),
    )
    .expect("write tampered approval");
    append_candidate_genesis(&data_dir);

    let error = validate_chain(&config_path, &data_dir).expect_err("tampered approval must fail");
    assert!(matches!(error, NodeError::ConfigMismatch(_)));
}

#[test]
fn wrong_network_config_is_rejected() {
    let temp_dir = tempdir().expect("tempdir");
    let config_path = temp_dir.path().join("alvenqis-devnet/config/devnet.toml");
    let invalid = r#"
network = "devnet"
network_id = "alvenqis-devnet"
human_name = "Alvenqis Devnet"
status_label = "Draft / Private Devnet"
block_time_seconds = 60
difficulty_leading_zero_bits = 4
ticker = "ALVE"
address_prefix = "alve"
max_supply = "60000000"
halving_interval = 1576800
initial_block_reward = "19.02587519"
default_rpc_port = 8787
default_p2p_port = 18787
max_mempool_transactions = 8
genesis_config_path = "alvenqis-devnet/config/genesis-devnet.json"
chain_magic_hex = "414c4456"
allow_mainnet_candidate = false
"#;
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("config dir");
    fs::write(&config_path, invalid.trim_start()).expect("config");

    let error = NetworkConfig::load_from_path(&config_path).expect_err("config must fail");
    assert!(matches!(error, NodeError::ConfigMismatch(_)));
}

#[test]
fn chain_validates_on_startup() {
    let (_temp_dir, config_path, data_dir, mempool_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init devnet");

    let summary = node_status(&config_path, &data_dir, &mempool_dir).expect("node status");

    assert!(summary.chain_initialized);
    assert_eq!(summary.height, Some(0));
    assert_eq!(summary.block_count, 1);
}

#[test]
fn local_operator_root_is_accepted_for_mainnet_candidate_status() {
    let temp_dir = tempdir().expect("tempdir");
    let config_path = temp_dir.path().join("configs/local.toml");
    let data_dir = temp_dir.path().join(".alvenqis-local/chain");
    let approval_path = temp_dir
        .path()
        .join("docs/release/GENESIS_APPROVAL.mainnet-candidate.json");
    write_network_config(
        &config_path,
        Network::MainnetCandidate,
        Some(&approval_path),
    );

    let report = status(&config_path, &data_dir).expect("status");
    match report {
        StatusReport::Uninitialized {
            data_dir: actual_path,
            ..
        } => {
            assert!(actual_path.contains(".alvenqis-local"));
        }
        StatusReport::Ready(_) => panic!("local operator chain should not be initialized"),
    }
}

#[test]
fn forbidden_files_are_not_tracked() {
    let output = Command::new("git")
        .args(["ls-files"])
        .output()
        .expect("git ls-files");
    assert!(
        output.status.success(),
        "git ls-files failed with status {:?}",
        output.status.code()
    );

    let tracked = String::from_utf8(output.stdout).expect("utf8");
    let forbidden_patterns = [
        ".env",
        ".alvenqis-dev/",
        ".alvenqis-testnet/",
        ".alvenqis-mainnet/",
        ".alvenqis-local/",
        ".wallet",
        ".seed",
        ".pem",
        ".key",
        "target/",
        "node_modules/",
    ];

    for pattern in forbidden_patterns {
        assert!(
            !tracked.lines().any(|line| line.contains(pattern)),
            "tracked files contain forbidden pattern {pattern}"
        );
    }
}
