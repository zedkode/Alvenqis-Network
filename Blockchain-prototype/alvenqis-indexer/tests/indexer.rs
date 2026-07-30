use alvenqis_core::{Address, Network, PrivateKey};
use alvenqis_indexer::{
    default_index_dir, ensure_index_matches_chain, find_address, find_block, find_transaction,
    index_devnet, indexer_status, load_index, reset_index, IndexerError,
};
use alvenqis_node::{default_miner_address, init_devnet, mine_dev_blocks, send_dev_tx, storage};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn write_devnet_config(path: &Path) {
    let content = r#"
network = "devnet"
network_id = "alvenqis-devnet"
human_name = "Alvenqis Devnet"
status_label = "Draft / Private Devnet"
block_time_seconds = 60
difficulty_leading_zero_bits = 4
ticker = "ALVE"
address_prefix = "dalve"
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
    fs::create_dir_all(path.parent().expect("config path must have parent")).expect("config dir");
    fs::write(path, content.trim_start()).expect("config file");
}

fn setup_paths() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
    let temp_dir = tempdir().expect("tempdir");
    let config_path = temp_dir.path().join("alvenqis-devnet/config/devnet.toml");
    let data_dir = temp_dir.path().join(".alvenqis-dev/chain");
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    write_devnet_config(&config_path);
    (temp_dir, config_path, data_dir, index_dir)
}

#[test]
fn index_empty_chain_safely() {
    let (_temp_dir, _config_path, data_dir, index_dir) = setup_paths();
    let error = index_devnet(&data_dir, &index_dir).expect_err("empty chain should fail safely");
    assert!(matches!(
        error,
        IndexerError::Node(alvenqis_node::NodeError::ChainNotInitialized(_))
    ));
}

#[test]
fn index_devnet_genesis() {
    let (_temp_dir, config_path, data_dir, index_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init");

    let index = index_devnet(&data_dir, &index_dir).expect("index");
    assert_eq!(index.summary.indexed_height, Some(0));
    assert_eq!(index.summary.indexed_block_count, 1);
    assert_eq!(index.summary.transaction_count, 1);
}

#[test]
fn index_multiple_mined_blocks() {
    let (_temp_dir, config_path, data_dir, index_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    mine_dev_blocks(&config_path, &data_dir, &miner_address, 3).expect("mine");

    let index = index_devnet(&data_dir, &index_dir).expect("index");
    assert_eq!(index.summary.indexed_height, Some(3));
    assert_eq!(index.summary.indexed_block_count, 4);
    assert!(index.summary.tip_hash.is_some());
}

#[test]
fn find_block_by_height_works() {
    let (_temp_dir, config_path, data_dir, index_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    mine_dev_blocks(&config_path, &data_dir, &miner_address, 2).expect("mine");
    index_devnet(&data_dir, &index_dir).expect("index");

    let block = find_block(&index_dir, 1).expect("block");
    assert_eq!(block.height, 1);
}

#[test]
fn find_transaction_by_hash_works() {
    let (_temp_dir, config_path, data_dir, index_dir) = setup_paths();
    let miner = PrivateKey::generate();
    let recipient = PrivateKey::generate();
    let miner_address =
        Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
    let recipient_address =
        Address::from_public_key_for_network(&recipient.public_key(), Network::Devnet).to_string();
    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    let sent = send_dev_tx(
        &config_path,
        &data_dir,
        &miner.to_hex(),
        &recipient_address,
        100,
        1,
        &miner_address,
    )
    .expect("send");
    index_devnet(&data_dir, &index_dir).expect("index");

    let transaction = find_transaction(&index_dir, &sent.tx_hash).expect("tx");
    assert_eq!(transaction.hash, sent.tx_hash);
    assert_eq!(transaction.to, recipient_address);
}

#[test]
fn find_address_activity_works() {
    let (_temp_dir, config_path, data_dir, index_dir) = setup_paths();
    let miner = PrivateKey::generate();
    let recipient = PrivateKey::generate();
    let miner_address =
        Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
    let recipient_address =
        Address::from_public_key_for_network(&recipient.public_key(), Network::Devnet).to_string();
    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    send_dev_tx(
        &config_path,
        &data_dir,
        &miner.to_hex(),
        &recipient_address,
        100,
        1,
        &miner_address,
    )
    .expect("send");
    index_devnet(&data_dir, &index_dir).expect("index");

    let activity = find_address(&index_dir, &recipient_address).expect("address");
    assert_eq!(activity.balance_atomic, 100);
    assert_eq!(activity.received_tx_hashes.len(), 1);
}

#[test]
fn reset_index_works() {
    let (_temp_dir, config_path, data_dir, index_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    index_devnet(&data_dir, &index_dir).expect("index");

    reset_index(&index_dir).expect("reset");
    let status = indexer_status(&index_dir).expect("status");
    assert!(!status.initialized);
}

#[test]
fn reindex_produces_same_result() {
    let (_temp_dir, config_path, data_dir, index_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    mine_dev_blocks(&config_path, &data_dir, &miner_address, 2).expect("mine");

    let first = index_devnet(&data_dir, &index_dir).expect("first index");
    let second = index_devnet(&data_dir, &index_dir).expect("second index");
    assert_eq!(first, second);

    let loaded = load_index(&index_dir).expect("load");
    assert_eq!(loaded, second);
}

#[test]
fn ensure_index_rebuilds_when_chain_tip_advances() {
    let (_temp_dir, config_path, data_dir, index_dir) = setup_paths();
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    mine_dev_blocks(&config_path, &data_dir, &miner_address, 1).expect("mine");
    let first = ensure_index_matches_chain(&data_dir, &index_dir).expect("index h1");
    assert_eq!(first.summary.indexed_height, Some(1));

    // Cached hit when tip unchanged
    let cached = ensure_index_matches_chain(&data_dir, &index_dir).expect("cache");
    assert_eq!(cached.summary.tip_hash, first.summary.tip_hash);

    mine_dev_blocks(&config_path, &data_dir, &miner_address, 2).expect("mine more");
    let advanced = ensure_index_matches_chain(&data_dir, &index_dir).expect("reindex");
    assert_eq!(advanced.summary.indexed_height, Some(3));
    assert_ne!(advanced.summary.tip_hash, first.summary.tip_hash);

    let rebuilt =
        index_devnet(&data_dir, &index_dir.with_file_name("full-index")).expect("full rebuild");
    assert_eq!(advanced, rebuilt);
}

#[test]
fn ensure_index_rebuilds_after_chain_reset_reorg() {
    let (_temp_dir, config_path, data_dir, index_dir) = setup_paths();
    let miner_a = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_a).expect("init");
    mine_dev_blocks(&config_path, &data_dir, &miner_a, 3).expect("mine");
    let before = ensure_index_matches_chain(&data_dir, &index_dir).expect("index");
    assert_eq!(before.summary.indexed_height, Some(3));
    let tip_before = before.summary.tip_hash.clone().expect("tip");

    // Simulate reorg/replace by resetting chain and growing a shorter tip.
    storage::reset_data_dir(&data_dir).expect("reset chain");
    let miner_b =
        Address::from_public_key_for_network(&PrivateKey::generate().public_key(), Network::Devnet)
            .to_string();
    init_devnet(&config_path, &data_dir, &miner_b).expect("re-init");
    mine_dev_blocks(&config_path, &data_dir, &miner_b, 1).expect("mine short");

    let after = ensure_index_matches_chain(&data_dir, &index_dir).expect("reindex reorg");
    assert_eq!(after.summary.indexed_height, Some(1));
    assert_ne!(after.summary.tip_hash.as_deref(), Some(tip_before.as_str()));
    assert_eq!(after.summary.indexed_block_count, 2);
}

#[test]
fn invalid_chain_data_is_handled_safely() {
    let (_temp_dir, _config_path, data_dir, index_dir) = setup_paths();
    storage::ensure_data_dir(&data_dir).expect("ensure");
    fs::write(storage::chain_file_path(&data_dir), "{invalid}\n").expect("invalid chain");

    let error = index_devnet(&data_dir, &index_dir).expect_err("invalid chain must fail");
    assert!(matches!(
        error,
        IndexerError::Node(
            alvenqis_node::NodeError::Sqlite(_)
                | alvenqis_node::NodeError::InvalidChainDatabase { .. }
        )
    ));
}

#[test]
fn default_index_directory_is_outside_tracked_source_folders() {
    let path = default_index_dir();
    let path_text = path.to_string_lossy().to_lowercase();
    assert!(!path_text.contains("alvenqis-indexer/src"));
}
