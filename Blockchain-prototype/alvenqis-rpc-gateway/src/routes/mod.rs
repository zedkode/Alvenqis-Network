mod addresses;
mod chain;
mod health;
mod indexer;
mod mempool;
mod mining;

use addresses::{address_account, address_balance, addresses};
use chain::{
    blocks_by_hash, blocks_by_height, blocks_latest, chain_height, chain_tip, state_snapshot,
    status, supply, sync_status, transactions_by_hash,
};
use health::{health, network, p2p_status};
use indexer::{
    indexer_address, indexer_addresses_page, indexer_blocks_by_hash, indexer_blocks_by_height,
    indexer_blocks_latest, indexer_blocks_page, indexer_overview, indexer_status, indexer_summary,
    indexer_transaction_by_hash, indexer_transactions_page,
};
use mempool::{mempool, mempool_status, submit_transaction};
use mining::{mining_submit, mining_template};

use crate::state::RpcState;
use axum::routing::{get, post};
use axum::Router;
use http::header::CONTENT_TYPE;
use http::{HeaderValue, Method, StatusCode};
use std::path::PathBuf;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::{ServeDir, ServeFile};

pub fn router(state: RpcState) -> Router {
    let configured_origins = state.config.effective_cors_origins();
    let cors = if configured_origins.contains(&"*") {
        CorsLayer::new().allow_origin(Any)
    } else {
        // Invalid origins are skipped rather than panicking the gateway process.
        let origins: Vec<HeaderValue> = configured_origins
            .into_iter()
            .filter_map(|origin| HeaderValue::from_str(origin).ok())
            .collect();
        if origins.is_empty() {
            // Restrictive default: same-origin / non-browser clients only.
            CorsLayer::new()
        } else {
            CorsLayer::new().allow_origin(origins)
        }
    }
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([CONTENT_TYPE]);

    let mut router = Router::new()
        .route("/health", get(health))
        .route("/network", get(network))
        .route("/status", get(status))
        .route("/sync/status", get(sync_status))
        .route("/chain/tip", get(chain_tip))
        .route("/chain/height", get(chain_height))
        .route("/addresses/:address", get(addresses))
        .route("/addresses/:address/balance", get(address_balance))
        .route("/addresses/:address/account", get(address_account))
        .route("/state", get(state_snapshot))
        .route("/supply", get(supply))
        .route("/blocks/latest", get(blocks_latest))
        .route("/blocks/:height", get(blocks_by_height))
        .route("/blocks/hash/:hash", get(blocks_by_hash))
        .route("/transactions/:hash", get(transactions_by_hash))
        .route("/mempool", get(mempool))
        .route("/mempool/status", get(mempool_status))
        .route("/indexer/status", get(indexer_status))
        .route("/indexer/overview", get(indexer_overview))
        .route("/indexer/summary", get(indexer_summary))
        .route("/indexer/blocks", get(indexer_blocks_page))
        .route("/indexer/blocks/latest", get(indexer_blocks_latest))
        .route("/indexer/blocks/hash/:hash", get(indexer_blocks_by_hash))
        .route("/indexer/blocks/:height", get(indexer_blocks_by_height))
        .route("/indexer/transactions", get(indexer_transactions_page))
        .route("/indexer/addresses", get(indexer_addresses_page))
        .route("/indexer/tx/:hash", get(indexer_transaction_by_hash))
        .route("/indexer/address/:address", get(indexer_address))
        // Read-only network telemetry; public nodes expose it so clients can
        // render peer and sync state regardless of the configured endpoint.
        .route("/p2p/status", get(p2p_status));
    if state.config.access_mode.allows_transaction_submission() {
        router = router.route("/transactions", post(submit_transaction));
    }
    if state.config.mining_endpoints_enabled() {
        router = router
            .route("/mining/template", get(mining_template))
            .route("/mining/submit", post(mining_submit));
    } else {
        router = router
            .route("/mining/template", get(mining_unavailable))
            .route("/mining/submit", post(mining_unavailable));
    }
    if !state.config.explorer_static_path.trim().is_empty() {
        let root = PathBuf::from(&state.config.explorer_static_path);
        router = router.fallback_service(
            ServeDir::new(&root).fallback(ServeFile::new(root.join("index.html"))),
        );
    }
    router
        .layer(RequestBodyLimitLayer::new(
            state.config.max_request_body_bytes,
        ))
        .layer(cors)
        .with_state(state)
}

async fn mining_unavailable() -> StatusCode {
    StatusCode::GONE
}
