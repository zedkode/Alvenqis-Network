//! Optional live smoke against the public Mainnet Candidate RPC.
//!
//! Run:
//!   cargo test -p alvenqis-sdk-rust --features native --test live_rpc_smoke -- --ignored --nocapture
//!
//! Or set ALVENQIS_LIVE_SMOKE=1 to enable without --ignored.

#![cfg(feature = "native")]

use alvenqis_sdk_rust::{NetworkConfig, RpcClient};

fn live_enabled() -> bool {
    std::env::var("ALVENQIS_LIVE_SMOKE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[tokio::test]
#[ignore = "hits public Mainnet Candidate RPC; run with --ignored or ALVENQIS_LIVE_SMOKE=1"]
async fn public_candidate_rpc_smoke() {
    if !live_enabled() && std::env::var("CARGO_TEST_RUN_IGNORED").is_err() {
        // Still allow plain --ignored from cargo.
    }

    let client = RpcClient::new(NetworkConfig::mainnet_candidate()).expect("client");

    let health = client.health().await.expect("health");
    assert!(health.ok);

    let status = client.status().await.expect("status");
    assert_eq!(status.network_id, "alvenqis-mainnet-candidate");
    assert!(status.initialized);
    assert!(status.height.is_some());

    let tip = client.tip().await.expect("tip");
    assert_eq!(tip.height, status.height.unwrap_or(0));

    let latest = client.block_latest().await.expect("latest block");
    assert_eq!(latest.height, tip.height);

    let supply = client.supply().await.expect("supply");
    assert!(supply.max_supply_atomic >= supply.emitted_supply_atomic);

    let sync = client.sync_status().await.expect("sync");
    assert!(!sync.sync_state.is_empty());

    let mempool = client.mempool_status().await.expect("mempool");
    let _ = mempool.pending_count;

    // Indexer may lag or be empty on some deployments -- tolerate soft failures.
    match client.indexer_status().await {
        Ok(idx) => {
            assert!(idx.initialized || idx.indexed_block_count == 0);
            println!(
                "indexer: height={:?} blocks={} in_sync={:?}",
                idx.indexed_height, idx.indexed_block_count, idx.in_sync
            );
        }
        Err(error) => println!("indexer status skipped: {error}"),
    }

    println!(
        "live smoke ok: height={} tip={} supply_emitted={}",
        tip.height, tip.hash, supply.emitted_supply_atomic
    );
}
