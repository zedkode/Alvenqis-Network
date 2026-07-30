pub mod config;
pub mod dev_helpers;
pub mod domain;
pub mod error;
pub mod mempool;
pub mod p2p;
pub mod peer_reputation;
#[cfg(feature = "storage-rocksdb")]
pub mod state_store;
pub mod storage;

pub use config::NetworkConfig;
pub use dev_helpers::{
    default_miner_address, format_verified_transaction, generate_dev_address, sign_dev_transaction,
    verify_dev_transaction, GeneratedDevAddress,
};
pub use domain::chain::{
    adopt_candidate_chain, balance, format_status, print_chain, state, status, validate_chain,
    BalanceSummary, ChainReorgSummary, ChainSummary, StateSummary, StatusReport,
};
pub use domain::genesis::{
    approve_genesis, default_config_path, export_genesis_block, genesis_approval_status,
    genesis_hash_hex_from_config, genesis_review_manifest, import_genesis_block, init_devnet,
    load_genesis_config, write_genesis_review_manifest, GenesisApprovalRecord,
    GenesisApprovalStatus, GenesisConfig, GenesisReviewManifest, DEFAULT_CONFIG_PATH,
    DEFAULT_MAINNET_CANDIDATE_CONFIG_PATH, GENESIS_APPROVAL_STANDARD_ID,
    GENESIS_REVIEW_STANDARD_ID,
};
pub use domain::mining::{
    create_block_template, mine_block, mine_dev_block, mine_dev_blocks, mine_pending_block,
    submit_mined_block, BlockTemplate, MinePendingBlockSummary, SubmittedMinedBlock,
    MAX_BLOCK_TEMPLATE_TRANSACTIONS,
};
pub use domain::runtime::{
    default_data_dir, default_runtime_dir, node_status, peers, reset_devnet,
    runtime_dir_for_data_dir, shutdown, start_node, NodeRuntimeStatus, PeersSummary, ResetSummary,
    DEFAULT_DATA_DIR, DEFAULT_STORAGE_INTEGRITY_INTERVAL_SECONDS,
};
pub use domain::transactions::{
    mempool_status, send_dev_tx, submit_transaction, MempoolStatusSummary, SendTransactionSummary,
    SubmitTransactionSummary,
};
pub use error::{NodeError, NodeResult};
pub use mempool::{
    clear_mempool, default_mempool_dir, default_network_root, load_pending_transactions,
    load_pending_transactions_for_chain, lowest_fee_sender_package, mempool_runtime_fingerprint,
    reconcile_after_reorg, reconcile_after_reorg_for_chain, select_pending_for_template,
    select_pending_for_template_for_chain, write_pending_transactions_for_chain,
    write_pending_transactions_for_chain_in_lock, PendingTransactionRecord,
    DEFAULT_MEMPOOL_MAX_AGE_SECONDS, MAX_PENDING_TXS_PER_SENDER, MEMPOOL_FILE_NAME,
};
pub use p2p::{
    load_p2p_status, local_p2p_handshake, queue_peer_ban, queue_peer_unban, run_p2p_service,
    validate_p2p_handshake, ConnectedPeer, NetworkMinerPresence, P2pHandshake, P2pStatus,
    PeerHello, P2P_PROTOCOL_VERSION, P2P_STATUS_FILE_NAME,
};
pub use peer_reputation::{
    PeerAdminAction, PeerAdminRequest, ReputationStore, DEFAULT_BAN_SECONDS, DEFAULT_SCORE,
    PEER_ADMIN_QUEUE_FILE_NAME, REPUTATION_FILE_NAME, SEVERE_BAN_SECONDS,
};
#[cfg(feature = "storage-rocksdb")]
pub use state_store::{
    backup_state_database, load_persisted_chain_state, load_persisted_mempool, persist_chain_state,
    replace_persisted_mempool, restore_latest_state_database, state_database_path,
    verify_state_database, PersistedChainState, RocksBackupInfo, RocksStateStatus,
    STATE_DATABASE_DIR_NAME, STATE_DATABASE_LOCK_FILE_NAME,
};
pub use storage::{
    append_block, append_block_unchecked, backup_chain_database, chain_database_path,
    chain_storage_exists, chain_storage_fingerprint, existing_transaction_hashes, load_blocks,
    load_transaction_by_hash, verify_chain_structure, verify_database_integrity, BlockStore,
    ChainStorageFingerprint, FileFingerprint, SqliteBlockStore, StorageIntegrityReport,
    StoredTransaction, CHAIN_DATABASE_FILE_NAME, CHAIN_FILE_NAME, CHAIN_LOCK_FILE_NAME,
    LEGACY_CHAIN_FILE_NAME,
};
