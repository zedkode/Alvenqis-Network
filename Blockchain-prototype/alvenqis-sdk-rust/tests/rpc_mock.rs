#![cfg(feature = "native")]

use alvenqis_sdk_rust::{Network, NetworkConfig, RpcClient};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn rpc_status_and_account_decode() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "network_id": "alvenqis-mainnet-candidate",
            "network_name": "Alvenqis Mainnet Candidate",
            "status_label": "Planned / Mainnet Candidate",
            "initialized": true,
            "block_count": 2,
            "height": 1,
            "tip_hash": "abc",
            "emitted_supply_atomic": 100
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(
            "/addresses/alve1qr4y5mrru2w9yz4774g8kyewchue23mk46ltu7ujgg0w56g5gmfzcnfqv0q/account",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "address": "alve1qr4y5mrru2w9yz4774g8kyewchue23mk46ltu7ujgg0w56g5gmfzcnfqv0q",
            "exists": true,
            "balance_atomic": 50,
            "next_nonce": 3,
            "tip_hash": "abc",
            "tip_height": 1,
            "anticipated_base_fee_atomic": 1
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/sync/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "network_id": "alvenqis-mainnet-candidate",
            "sync_state": "synced",
            "local_height": 1,
            "network_height": 1,
            "remaining_blocks": 0,
            "progress_percent": 100.0,
            "connected_peer_count": 2,
            "validated_peer_count": 1,
            "detail": "ok"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/supply"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "emitted_supply_atomic": 100,
            "max_supply_atomic": 6000000000000000_u64,
            "remaining_supply_atomic": 5999999999999900_u64
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/mempool/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok",
            "pending_count": 2,
            "anticipated_base_fee_atomic": 1,
            "total_fees_atomic": 4,
            "total_burned_fees_atomic": 2,
            "total_priority_fees_atomic": 2
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/chain/tip"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "height": 1,
            "hash": "tiphash"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/blocks/latest"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "network_id": "alvenqis-mainnet-candidate",
            "height": 1,
            "hash": "block1",
            "previous_hash": "block0",
            "merkle_root": "merkle",
            "base_fee_atomic": 1,
            "timestamp": 1720000060_u64,
            "nonce": 9,
            "difficulty_leading_zero_bits": 16,
            "transaction_count": 1,
            "transactions": [{
                "lifecycle_status": "mined",
                "hash": "tx1",
                "block_height": 1,
                "block_hash": "block1",
                "version": 1,
                "nonce": 0,
                "from": null,
                "to": "alve1qr4y5mrru2w9yz4774g8kyewchue23mk46ltu7ujgg0w56g5gmfzcnfqv0q",
                "amount_atomic": 1902587519_u64,
                "authorization_state": "coinbase"
            }]
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/blocks/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "network_id": "alvenqis-mainnet-candidate",
            "height": 1,
            "hash": "block1",
            "previous_hash": "block0",
            "merkle_root": "merkle",
            "base_fee_atomic": 1,
            "timestamp": 1720000060_u64,
            "nonce": 9,
            "difficulty_leading_zero_bits": 16,
            "transaction_count": 1,
            "transactions": []
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/blocks/0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "network_id": "alvenqis-mainnet-candidate",
            "height": 0,
            "hash": "block0",
            "previous_hash": "0000",
            "merkle_root": "merkle0",
            "base_fee_atomic": 1,
            "timestamp": 1720000000_u64,
            "nonce": 1,
            "difficulty_leading_zero_bits": 16,
            "transaction_count": 1,
            "transactions": []
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/indexer/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "mode": "Draft / Local Indexer / Prototype",
            "network_id": "alvenqis-mainnet-candidate",
            "status_label": "Planned / Mainnet Candidate",
            "initialized": true,
            "index_dir": "redacted",
            "indexed_height": 1,
            "indexed_block_count": 2,
            "transaction_count": 2,
            "address_count": 1,
            "tip_hash": "block1",
            "chain_height": 1,
            "in_sync": true,
            "lag_blocks": 0
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/indexer/summary"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "summary": {
                "mode": "Draft / Local Indexer / Prototype",
                "network": "alvenqis-mainnet-candidate",
                "status": "ready",
                "indexed_height": 1,
                "indexed_block_count": 2,
                "transaction_count": 2,
                "address_count": 1,
                "tip_hash": "block1",
                "latest_block_hash": "block1",
                "latest_block_timestamp": 1720000060_u64,
                "supply": {
                    "emitted_supply_atomic": 100,
                    "max_supply_atomic": 6000000000000000_u64,
                    "remaining_supply_atomic": 5999999999999900_u64
                }
            },
            "blocks_by_height": { "0": { "height": 0 }, "1": { "height": 1 } },
            "transactions_by_hash": {},
            "addresses": {}
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/indexer/blocks/latest"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "height": 1,
            "hash": "block1",
            "previous_hash": "block0",
            "merkle_root": "merkle",
            "timestamp": 1720000060_u64,
            "nonce": 9,
            "difficulty_leading_zero_bits": 16,
            "transaction_count": 1,
            "miner_address": "alve1miner",
            "coinbase_payout_atomic": 1902587519_u64,
            "miner_reward_atomic": 1902587519_u64,
            "fees_atomic": 0,
            "burned_fees_atomic": 0,
            "priority_fees_atomic": 0,
            "base_fee_atomic": 1,
            "transaction_hashes": ["tx1"]
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/p2p/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "local_peer_id": "12D3KooWtest",
            "connected_peer_count": 3,
            "validated_peer_count": 1,
            "syncing": false
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/v1/pool/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "pool_name": "test-pool",
            "block_maturity_confirmations": 12,
            "connected_workers": 1,
            "recent_blocks": [{
                "height": 1,
                "hash": "block1",
                "status": "immature",
                "reward_atomic": "100"
            }]
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/v1/pool/history"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "blocks": [{
                "height": 1,
                "hash": "block1",
                "status": "immature",
                "reward_atomic": 100
            }]
        })))
        .mount(&server)
        .await;

    // Same mock server URI for both RPC and pool bases.
    let client = RpcClient::new(NetworkConfig::with_rpc_and_pool(
        Network::MainnetCandidate,
        server.uri(),
        server.uri(),
    ))
    .expect("client");

    let status = client.status().await.expect("status");
    assert_eq!(status.network_id, "alvenqis-mainnet-candidate");
    assert_eq!(status.height, Some(1));

    let account = client
        .account("alve1qr4y5mrru2w9yz4774g8kyewchue23mk46ltu7ujgg0w56g5gmfzcnfqv0q")
        .await
        .expect("account");
    assert_eq!(account.next_nonce, 3);
    assert_eq!(account.balance_atomic, 50);

    let sync = client.sync_status().await.expect("sync");
    assert_eq!(sync.sync_state, "synced");
    assert_eq!(sync.remaining_blocks, Some(0));

    let supply = client.supply().await.expect("supply");
    assert_eq!(supply.emitted_supply_atomic, 100);

    let mempool = client.mempool_status().await.expect("mempool");
    assert_eq!(mempool.pending_count, 2);
    assert_eq!(mempool.anticipated_base_fee_atomic, 1);

    let latest = client.block_latest().await.expect("latest");
    assert_eq!(latest.height, 1);
    assert_eq!(latest.coinbase_amount_atomic(), Some(1_902_587_519));

    let recent = client.recent_blocks(2).await.expect("recent");
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].height, 1);
    assert_eq!(recent[1].height, 0);

    let idx = client.indexer_status().await.expect("indexer status");
    assert_eq!(idx.indexed_height, Some(1));
    assert_eq!(idx.in_sync, Some(true));

    let summary = client.indexer_summary().await.expect("indexer summary");
    assert_eq!(summary.summary.indexed_block_count, 2);
    assert_eq!(summary.summary.supply.emitted_supply_atomic, 100);

    let ib = client.indexer_block_latest().await.expect("indexed latest");
    assert_eq!(ib.miner_reward_atomic, 1_902_587_519);

    let p2p = client.p2p_status().await.expect("p2p");
    assert_eq!(p2p.connected_peer_count, Some(3));
    assert_eq!(p2p.local_peer_id.as_deref(), Some("12D3KooWtest"));

    let pool = client.pool_status().await.expect("pool");
    assert_eq!(pool.block_maturity_confirmations, Some(12));
    assert_eq!(pool.recent_blocks.len(), 1);

    let matured = client.pool_blocks_with_maturity().await.expect("maturity");
    assert_eq!(matured.len(), 1);
    assert_eq!(matured[0].height, 1);
    // tip height 1, block 1, required 12 → immature
    assert_eq!(
        matured[0].maturity.status,
        alvenqis_sdk_rust::MaturityStatus::Immature
    );
    assert_eq!(matured[0].maturity.confirmations, 0);
}
