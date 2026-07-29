use alvenqis_core::{
    firopow::mine_firopow_solution, Address, Network, PrivateKey, Transaction,
    INITIAL_BASE_FEE_ATOMIC,
};
use alvenqis_indexer::index_devnet;
use alvenqis_miner::{MiningSubmitRequest, MiningTemplate};
use alvenqis_node::{
    default_miner_address, init_devnet, mine_dev_blocks, mine_pending_block, storage,
    submit_transaction,
};
use alvenqis_rpc_gateway::{load_chain, router, RpcAccessMode, RpcConfig, RpcState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use tower::util::ServiceExt;

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

fn setup_paths() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp_dir = tempdir().expect("tempdir");
    let config_path = temp_dir.path().join("alvenqis-devnet/config/devnet.toml");
    let data_dir = temp_dir.path().join(".alvenqis-dev/chain");
    write_devnet_config(&config_path);
    (temp_dir, config_path, data_dir)
}

fn rpc_state(data_dir: &Path, index_dir: &Path) -> RpcState {
    let mempool_dir = data_dir
        .parent()
        .expect("devnet dir")
        .parent()
        .expect("workspace temp root")
        .join(".alvenqis-dev/mempool");
    RpcState::new(RpcConfig {
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 8787,
        network: Network::Devnet,
        network_id: Network::Devnet.network_id().to_owned(),
        human_name: Network::Devnet.human_name().to_owned(),
        status_label: Network::Devnet.status_label().to_owned(),
        address_prefix: Network::Devnet.address_prefix().to_owned(),
        chain_data_path: data_dir.display().to_string(),
        indexer_data_path: index_dir.display().to_string(),
        mempool_data_path: mempool_dir.display().to_string(),
        public_rpc_allowed: false,
        access_mode: RpcAccessMode::Local,
        expose_mining_endpoints: None,
        max_mempool_transactions: 8,
        max_request_body_bytes: 65_536,
        cors_allowed_origin: "http://127.0.0.1:4173".to_owned(),
        cors_allowed_origins: Vec::new(),
        explorer_static_path: String::new(),
        allow_mainnet_candidate: false,
        api_token: String::new(),
        write_rate_limit_per_minute: 0,
        mining_template_rate_limit_per_minute: 0,
    })
}

#[tokio::test]
async fn cors_preflight_allows_configured_origins_only() {
    let (temp_dir, _config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let mut state = rpc_state_with_access_mode(&data_dir, &index_dir, RpcAccessMode::PublicSubmit);
    state.config.cors_allowed_origins = vec![
        "https://alvenqis.network".to_owned(),
        "http://127.0.0.1:4173".to_owned(),
    ];
    let app = router(state);

    let allowed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/transactions")
                .header("origin", "https://alvenqis.network")
                .header("access-control-request-method", "POST")
                .header("access-control-request-headers", "content-type")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(allowed.status(), StatusCode::OK);
    assert_eq!(
        allowed.headers().get("access-control-allow-origin"),
        Some(&"https://alvenqis.network".parse().expect("header"))
    );

    let rejected = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/transactions")
                .header("origin", "https://untrusted.example")
                .header("access-control-request-method", "POST")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert!(rejected
        .headers()
        .get("access-control-allow-origin")
        .is_none());
}

fn rpc_state_with_access_mode(
    data_dir: &Path,
    index_dir: &Path,
    access_mode: RpcAccessMode,
) -> RpcState {
    let mut state = rpc_state(data_dir, index_dir);
    state.config.access_mode = access_mode;
    state
}

#[tokio::test]
async fn public_read_profile_hides_mutating_and_operator_routes() {
    let (temp_dir, _config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let app = router(rpc_state_with_access_mode(
        &data_dir,
        &index_dir,
        RpcAccessMode::PublicRead,
    ));

    for (method, uri, expected) in [
        ("POST", "/transactions", StatusCode::NOT_FOUND),
        ("GET", "/mining/template", StatusCode::GONE),
        ("POST", "/mining/submit", StatusCode::GONE),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), expected, "{method} {uri}");
    }
}

#[tokio::test]
async fn public_read_profile_exposes_p2p_status() {
    let (temp_dir, config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let app = router(
        rpc_state_with_access_mode(&data_dir, &index_dir, RpcAccessMode::PublicRead)
            .with_node_config_path(config_path),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/p2p/status")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn public_submit_profile_returns_gone_for_mining() {
    let (temp_dir, _config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let app = router(rpc_state_with_access_mode(
        &data_dir,
        &index_dir,
        RpcAccessMode::PublicSubmit,
    ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/mining/template")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::GONE);
}

#[test]
fn public_submit_cannot_opt_in_to_mining() {
    let (temp_dir, _config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let mut state = rpc_state_with_access_mode(&data_dir, &index_dir, RpcAccessMode::PublicSubmit);
    state.config.expose_mining_endpoints = Some(true);
    let error = state
        .config
        .validate()
        .expect_err("public mining must fail");
    assert!(error
        .to_string()
        .contains("public RPC profiles cannot expose mining endpoints"));
}

#[tokio::test]
async fn health_response_works() {
    let (temp_dir, _config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let app = router(rpc_state(&data_dir, &index_dir));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn p2p_status_reports_the_local_network_view() {
    let (temp_dir, config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let app = router(rpc_state(&data_dir, &index_dir).with_node_config_path(config_path));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/p2p/status")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(json["network_id"], Value::from("alvenqis-devnet"));
    assert_eq!(json["connected_peer_count"], Value::from(0));
    assert_eq!(json["validated_peer_count"], Value::from(0));
}

#[tokio::test]
async fn public_sync_status_exposes_aggregates_without_peer_details() {
    let (temp_dir, config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    let app = router(
        rpc_state_with_access_mode(&data_dir, &index_dir, RpcAccessMode::PublicRead)
            .with_node_config_path(config_path),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/sync/status")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(json["sync_state"], Value::from("discovering"));
    assert_eq!(json["local_height"], Value::from(0_u64));
    assert_eq!(json["validated_peer_count"], Value::from(0_u64));
    assert!(json.get("peers").is_none());
}

#[tokio::test]
async fn status_response_works() {
    let (temp_dir, config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    let app = router(rpc_state(&data_dir, &index_dir));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/status")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&body).expect("json");

    assert_eq!(json["initialized"], Value::Bool(true));
    assert_eq!(json["network_id"], Value::from("alvenqis-devnet"));
    assert_eq!(json["height"], Value::from(0_u64));
}

#[tokio::test]
async fn network_response_exposes_frozen_standards() {
    let (temp_dir, _config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let app = router(rpc_state(&data_dir, &index_dir));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/network")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&body).expect("json");

    assert_eq!(
        json["address_standard_id"],
        Value::from("alvenqis-address-bech32m-ed25519-v1")
    );
    assert_eq!(
        json["signature_standard_id"],
        Value::from("alvenqis-signature-ed25519-v1")
    );
    assert_eq!(
        json["key_derivation_policy_id"],
        Value::from("alvenqis-key-ed25519-v1")
    );
    assert_eq!(
        json["tx_signing_domain"],
        Value::from("alvenqis-tx-ed25519-v1")
    );
    assert_eq!(
        json["protocol_parameters_id"],
        Value::from("alvenqis-launch-parameters-v1")
    );
    assert_eq!(
        json["max_supply_atomic"],
        Value::from(6_000_000_000_000_000_u64)
    );
    assert_eq!(json["pow_hash_algorithm"], Value::from("FiroPoW-0.9.4"));
}

#[tokio::test]
async fn mining_template_and_submission_append_a_block() {
    let (temp_dir, config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let miner = PrivateKey::generate();
    let miner_address =
        Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    let state = rpc_state(&data_dir, &index_dir).with_node_config_path(config_path);
    let app = router(state);

    let template_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/mining/template?miner_address={miner_address}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("template response");
    assert_eq!(template_response.status(), StatusCode::OK);
    let template_body = axum::body::to_bytes(template_response.into_body(), usize::MAX)
        .await
        .expect("template body");
    let template: MiningTemplate = serde_json::from_slice(&template_body).expect("mining template");
    let block = template
        .validate_and_build(&miner_address)
        .expect("valid template");
    let (nonce, solution) =
        mine_firopow_solution(&block, template.difficulty_leading_zero_bits, 0, 1_000_000)
            .expect("mining")
            .expect("low-difficulty FiroPoW solution");
    let submission = MiningSubmitRequest::from_solution(
        template.template_id,
        nonce,
        solution.final_hash,
        solution.mix_hash,
    );

    let submit_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mining/submit")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&submission).expect("submission JSON"),
                ))
                .expect("request"),
        )
        .await
        .expect("submit response");
    assert_eq!(submit_response.status(), StatusCode::OK);
    let submit_body = axum::body::to_bytes(submit_response.into_body(), usize::MAX)
        .await
        .expect("submit body");
    let submit_json: Value = serde_json::from_slice(&submit_body).expect("submit JSON");
    assert_eq!(submit_json["status"], Value::from("accepted"));

    let blocks = storage::load_blocks(&data_dir).expect("blocks");
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks.last().expect("tip").header.height, 1);
}

#[tokio::test]
async fn parallel_losing_template_is_reported_as_stale_not_rejected() {
    let (temp_dir, config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let first_miner = PrivateKey::generate();
    let second_miner = PrivateKey::generate();
    let first_address =
        Address::from_public_key_for_network(&first_miner.public_key(), Network::Devnet)
            .to_string();
    let second_address =
        Address::from_public_key_for_network(&second_miner.public_key(), Network::Devnet)
            .to_string();
    init_devnet(&config_path, &data_dir, &first_address).expect("init");
    let app = router(rpc_state(&data_dir, &index_dir).with_node_config_path(config_path));

    let first_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/mining/template?miner_address={first_address}"))
                .body(Body::empty())
                .expect("first template request"),
        )
        .await
        .expect("first template response");
    let second_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/mining/template?miner_address={second_address}"))
                .body(Body::empty())
                .expect("second template request"),
        )
        .await
        .expect("second template response");
    assert_eq!(first_response.status(), StatusCode::OK);
    assert_eq!(second_response.status(), StatusCode::OK);

    let first_body = axum::body::to_bytes(first_response.into_body(), usize::MAX)
        .await
        .expect("first template body");
    let second_body = axum::body::to_bytes(second_response.into_body(), usize::MAX)
        .await
        .expect("second template body");
    let first_template: MiningTemplate =
        serde_json::from_slice(&first_body).expect("first mining template");
    let second_template: MiningTemplate =
        serde_json::from_slice(&second_body).expect("second mining template");
    assert_eq!(first_template.height, second_template.height);
    assert_ne!(first_template.template_id, second_template.template_id);

    let first_block = first_template
        .validate_and_build(&first_address)
        .expect("first valid template");
    let second_block = second_template
        .validate_and_build(&second_address)
        .expect("second valid template");
    let (first_nonce, first_solution) = mine_firopow_solution(
        &first_block,
        first_template.difficulty_leading_zero_bits,
        0,
        1_000_000,
    )
    .expect("first mining")
    .expect("first low-difficulty solution");
    let (second_nonce, second_solution) = mine_firopow_solution(
        &second_block,
        second_template.difficulty_leading_zero_bits,
        0,
        1_000_000,
    )
    .expect("second mining")
    .expect("second low-difficulty solution");

    let first_submission = MiningSubmitRequest::from_solution(
        first_template.template_id,
        first_nonce,
        first_solution.final_hash,
        first_solution.mix_hash,
    );
    let second_submission = MiningSubmitRequest::from_solution(
        second_template.template_id,
        second_nonce,
        second_solution.final_hash,
        second_solution.mix_hash,
    );
    let first_submit_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mining/submit")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&first_submission).expect("first submission JSON"),
                ))
                .expect("first submit request"),
        )
        .await
        .expect("first submit response");
    let first_submit_body = axum::body::to_bytes(first_submit_response.into_body(), usize::MAX)
        .await
        .expect("first submit body");
    let first_submit_json: Value =
        serde_json::from_slice(&first_submit_body).expect("first submit JSON");
    assert_eq!(first_submit_json["status"], Value::from("accepted"));

    let second_submit_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mining/submit")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&second_submission).expect("second submission JSON"),
                ))
                .expect("second submit request"),
        )
        .await
        .expect("second submit response");
    let second_submit_body = axum::body::to_bytes(second_submit_response.into_body(), usize::MAX)
        .await
        .expect("second submit body");
    let second_submit_json: Value =
        serde_json::from_slice(&second_submit_body).expect("second submit JSON");
    assert_eq!(second_submit_json["status"], Value::from("stale"));
    assert!(second_submit_json["reason"]
        .as_str()
        .is_some_and(|reason| reason.contains("chain tip advanced")));
}

#[tokio::test]
async fn chain_tip_response_works() {
    let (temp_dir, config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    mine_dev_blocks(&config_path, &data_dir, &miner_address, 2).expect("mine");
    let app = router(rpc_state(&data_dir, &index_dir));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/chain/tip")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&body).expect("json");

    assert_eq!(json["height"], Value::from(2_u64));
    assert!(json["hash"].as_str().is_some());
}

#[tokio::test]
async fn missing_block_response_is_404() {
    let (temp_dir, config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    let app = router(rpc_state(&data_dir, &index_dir));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/blocks/99")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn loading_devnet_data_works() {
    let (temp_dir, config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    mine_dev_blocks(&config_path, &data_dir, &miner_address, 1).expect("mine");

    let loaded = load_chain(&rpc_state(&data_dir, &index_dir)).expect("load");
    assert_eq!(loaded.blocks.len(), 2);
    assert_eq!(loaded.chain.height(), Some(1));
}

#[tokio::test]
async fn invalid_devnet_data_handling_returns_500() {
    let (temp_dir, _config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    storage::ensure_data_dir(&data_dir).expect("ensure data dir");
    fs::write(storage::chain_file_path(&data_dir), "{invalid}\n").expect("invalid chain file");
    let app = router(rpc_state(&data_dir, &index_dir));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/chain/height")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn address_balance_endpoint_works() {
    let (temp_dir, config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let miner = PrivateKey::generate();
    let recipient = PrivateKey::generate();
    let miner_address =
        Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
    let recipient_address =
        Address::from_public_key_for_network(&recipient.public_key(), Network::Devnet).to_string();

    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    let mempool_dir = temp_dir.path().join(".alvenqis-dev/mempool");
    let transaction = Transaction::new_signed(
        1,
        1,
        Network::Devnet,
        &miner,
        recipient_address.clone(),
        alvenqis_core::Amount::from_atomic(100),
        alvenqis_core::Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 1),
        alvenqis_core::Amount::from_atomic(1),
        None,
    )
    .expect("transaction");
    submit_transaction(&data_dir, &mempool_dir, 8, &transaction).expect("submit");
    mine_pending_block(&config_path, &data_dir, &mempool_dir, &miner_address).expect("mine");
    let app = router(rpc_state(&data_dir, &index_dir));

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/addresses/{recipient_address}/balance"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&body).expect("json");

    assert_eq!(json["balance_atomic"], Value::from(100_u64));
}

#[tokio::test]
async fn state_and_supply_endpoints_work() {
    let (temp_dir, config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let miner = PrivateKey::generate();
    let miner_address =
        Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    mine_dev_blocks(&config_path, &data_dir, &miner_address, 1).expect("mine");
    let app = router(rpc_state(&data_dir, &index_dir));

    let state_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/state")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let state_body = axum::body::to_bytes(state_response.into_body(), usize::MAX)
        .await
        .expect("body");
    let state_json: Value = serde_json::from_slice(&state_body).expect("json");

    assert_eq!(state_json["height"], Value::from(1_u64));
    assert_eq!(state_json["tracked_addresses"], Value::from(1_u64));

    let supply_response = app
        .oneshot(
            Request::builder()
                .uri("/supply")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let supply_body = axum::body::to_bytes(supply_response.into_body(), usize::MAX)
        .await
        .expect("body");
    let supply_json: Value = serde_json::from_slice(&supply_body).expect("json");

    assert_eq!(
        supply_json["emitted_supply_atomic"],
        Value::from(alvenqis_core::INITIAL_BLOCK_REWARD_ATOMIC * 2)
    );
}

#[tokio::test]
async fn indexer_status_and_summary_endpoints_work() {
    let (temp_dir, config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let miner = PrivateKey::generate();
    let recipient = PrivateKey::generate();
    let miner_address =
        Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
    let recipient_address =
        Address::from_public_key_for_network(&recipient.public_key(), Network::Devnet).to_string();

    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    let mempool_dir = temp_dir.path().join(".alvenqis-dev/mempool");
    let transaction = Transaction::new_signed(
        1,
        1,
        Network::Devnet,
        &miner,
        recipient_address.clone(),
        alvenqis_core::Amount::from_atomic(100),
        alvenqis_core::Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 1),
        alvenqis_core::Amount::from_atomic(1),
        None,
    )
    .expect("transaction");
    submit_transaction(&data_dir, &mempool_dir, 8, &transaction).expect("submit");
    mine_pending_block(&config_path, &data_dir, &mempool_dir, &miner_address).expect("mine");
    index_devnet(&data_dir, &index_dir).expect("index");
    let app = router(rpc_state(&data_dir, &index_dir));

    let status_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/indexer/status")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(status_response.status(), StatusCode::OK);
    let status_body = axum::body::to_bytes(status_response.into_body(), usize::MAX)
        .await
        .expect("body");
    let status_json: Value = serde_json::from_slice(&status_body).expect("json");
    assert_eq!(status_json["initialized"], Value::Bool(true));
    assert_eq!(status_json["indexed_height"], Value::from(1_u64));

    let summary_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/indexer/summary")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(summary_response.status(), StatusCode::OK);
    let summary_body = axum::body::to_bytes(summary_response.into_body(), usize::MAX)
        .await
        .expect("body");
    let summary_json: Value = serde_json::from_slice(&summary_body).expect("json");
    assert_eq!(
        summary_json["summary"]["indexed_height"],
        Value::from(1_u64)
    );
    assert_eq!(
        summary_json["summary"]["transaction_count"],
        Value::from(3_u64)
    );

    let overview_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/indexer/overview?blocks=1&transactions=2")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(overview_response.status(), StatusCode::OK);
    let overview_body = axum::body::to_bytes(overview_response.into_body(), usize::MAX)
        .await
        .expect("body");
    let overview_json: Value = serde_json::from_slice(&overview_body).expect("json");
    assert_eq!(
        overview_json["recent_blocks"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(
        overview_json["recent_transactions"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );

    let blocks_page_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/indexer/blocks?offset=0&limit=1")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(blocks_page_response.status(), StatusCode::OK);
    let blocks_page_body = axum::body::to_bytes(blocks_page_response.into_body(), usize::MAX)
        .await
        .expect("body");
    let blocks_page_json: Value = serde_json::from_slice(&blocks_page_body).expect("json");
    assert_eq!(blocks_page_json["total"], Value::from(2_u64));
    assert_eq!(blocks_page_json["items"].as_array().map(Vec::len), Some(1));
    let indexed_block_hash = blocks_page_json["items"][0]["hash"]
        .as_str()
        .expect("block hash");

    let block_hash_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/indexer/blocks/hash/{indexed_block_hash}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(block_hash_response.status(), StatusCode::OK);

    let transactions_page_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/indexer/transactions?offset=1&limit=1")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(transactions_page_response.status(), StatusCode::OK);
    let transactions_page_body =
        axum::body::to_bytes(transactions_page_response.into_body(), usize::MAX)
            .await
            .expect("body");
    let transactions_page_json: Value =
        serde_json::from_slice(&transactions_page_body).expect("json");
    assert_eq!(transactions_page_json["total"], Value::from(3_u64));
    assert_eq!(
        transactions_page_json["items"].as_array().map(Vec::len),
        Some(1)
    );

    let addresses_page_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/indexer/addresses?offset=0&limit=1000")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(addresses_page_response.status(), StatusCode::OK);
    let addresses_page_body = axum::body::to_bytes(addresses_page_response.into_body(), usize::MAX)
        .await
        .expect("body");
    let addresses_page_json: Value = serde_json::from_slice(&addresses_page_body).expect("json");
    assert_eq!(addresses_page_json["limit"], Value::from(100_u64));
    assert!(addresses_page_json["total"].as_u64().unwrap_or_default() >= 2);

    let address_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/indexer/address/{recipient_address}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(address_response.status(), StatusCode::OK);

    let latest_block_response = app
        .oneshot(
            Request::builder()
                .uri("/indexer/blocks/latest")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(latest_block_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn transaction_submission_and_mempool_endpoints_work() {
    let (temp_dir, config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let mempool_dir = temp_dir.path().join(".alvenqis-dev/mempool");
    let miner = PrivateKey::generate();
    let recipient = PrivateKey::generate();
    let miner_address =
        Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
    let recipient_address =
        Address::from_public_key_for_network(&recipient.public_key(), Network::Devnet).to_string();
    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    let app = router(rpc_state(&data_dir, &index_dir));

    let transaction = Transaction::new_signed(
        1,
        1,
        Network::Devnet,
        &miner,
        recipient_address,
        alvenqis_core::Amount::from_atomic(100),
        alvenqis_core::Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 1),
        alvenqis_core::Amount::from_atomic(1),
        None,
    )
    .expect("transaction");
    let payload = serde_json::to_vec(&transaction).expect("json");
    let tx_hash = alvenqis_core::hash_to_hex(&transaction.tx_hash());

    let submit_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/transactions")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(submit_response.status(), StatusCode::OK);

    let mempool_status_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mempool/status")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let mempool_status_body = axum::body::to_bytes(mempool_status_response.into_body(), usize::MAX)
        .await
        .expect("body");
    let mempool_status_json: Value = serde_json::from_slice(&mempool_status_body).expect("json");
    assert_eq!(mempool_status_json["pending_count"], Value::from(1_u64));

    let tx_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/transactions/{tx_hash}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let tx_body = axum::body::to_bytes(tx_response.into_body(), usize::MAX)
        .await
        .expect("body");
    let tx_json: Value = serde_json::from_slice(&tx_body).expect("json");
    assert_eq!(tx_json["lifecycle_status"], Value::from("pending"));

    mine_pending_block(&config_path, &data_dir, &mempool_dir, &miner_address).expect("mine");

    let mined_tx_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/transactions/{tx_hash}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let mined_tx_body = axum::body::to_bytes(mined_tx_response.into_body(), usize::MAX)
        .await
        .expect("body");
    let mined_tx_json: Value = serde_json::from_slice(&mined_tx_body).expect("json");
    assert_eq!(mined_tx_json["lifecycle_status"], Value::from("mined"));
    assert_eq!(mined_tx_json["block_height"], Value::from(1_u64));
}

#[tokio::test]
async fn invalid_transaction_submission_is_rejected() {
    let (temp_dir, config_path, data_dir) = setup_paths();
    let index_dir = temp_dir.path().join(".alvenqis-dev/indexer");
    let miner = PrivateKey::generate();
    let recipient = PrivateKey::generate();
    let miner_address =
        Address::from_public_key_for_network(&miner.public_key(), Network::Devnet).to_string();
    let recipient_address =
        Address::from_public_key_for_network(&recipient.public_key(), Network::Devnet).to_string();
    init_devnet(&config_path, &data_dir, &miner_address).expect("init");
    let app = router(rpc_state(&data_dir, &index_dir));

    let mut transaction = Transaction::new_signed(
        1,
        1,
        Network::Devnet,
        &miner,
        recipient_address,
        alvenqis_core::Amount::from_atomic(100),
        alvenqis_core::Amount::from_atomic(INITIAL_BASE_FEE_ATOMIC + 1),
        alvenqis_core::Amount::from_atomic(1),
        None,
    )
    .expect("transaction");
    transaction.signature = Some(PrivateKey::generate().sign(b"tampered"));
    let payload = serde_json::to_vec(&transaction).expect("json");

    let submit_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/transactions")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(submit_response.status(), StatusCode::BAD_REQUEST);
}
