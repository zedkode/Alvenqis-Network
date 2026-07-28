mod chain_service;
mod indexer_service;
mod mempool_service;
mod mining_service;

pub(crate) use chain_service::{
    address_account, address_balance, addresses, blocks_by_hash, blocks_by_height, blocks_latest,
    chain_height, chain_tip, state_snapshot, status, supply, sync_status, transactions_by_hash,
};
pub(crate) use indexer_service::{
    cached_indexer_status_async, indexer_address, indexer_addresses_page, indexer_blocks_by_hash,
    indexer_blocks_by_height, indexer_blocks_latest, indexer_blocks_page, indexer_overview,
    indexer_status, indexer_summary, indexer_transaction_by_hash, indexer_transactions_page,
    IndexerOverviewQuery, IndexerPageQuery,
};
pub(crate) use mempool_service::{
    load_mempool_transactions, mempool, mempool_status, submit_transaction,
};
pub(crate) use mining_service::{
    mining_submit, mining_template, MiningTemplateQuery, StoredMiningTemplate,
};

pub(crate) fn map_submission_error(error: alvenqis_node::NodeError) -> crate::error::RpcError {
    match error {
        alvenqis_node::NodeError::Input(message) => crate::error::RpcError::BadRequest(message),
        alvenqis_node::NodeError::Core(core_error) => {
            crate::error::RpcError::BadRequest(core_error.to_string())
        }
        other => crate::error::RpcError::Node(other),
    }
}
