use alvenqis_core::{
    block_reward, firopow::mine_firopow_solution, hash_to_hex, initial_base_fee, Address, Block,
    Hash, Network, PrivateKey, Transaction,
};
use alvenqis_miner::{
    FileWorkSource, MiningSubmitRequest, MiningTemplate, SubmitStatus, WorkSource,
    MINING_PROTOCOL_VERSION,
};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::tempdir;

fn fixture() -> (String, MiningTemplate) {
    let key = PrivateKey::generate();
    let address =
        Address::from_public_key_for_network(&key.public_key(), Network::Devnet).to_string();
    let coinbase = Transaction::coinbase(1, address.clone(), block_reward(1)).expect("coinbase");
    let block = Block::new(
        Network::Devnet,
        1,
        Hash::zero(),
        initial_base_fee().as_atomic(),
        1_720_000_060,
        0,
        vec![coinbase],
    )
    .expect("block");
    let expires_at_unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs()
        + 60;
    let template = MiningTemplate {
        protocol: MINING_PROTOCOL_VERSION.to_owned(),
        template_id: "fixture-1".to_owned(),
        expires_at_unix_seconds,
        version: block.header.version,
        network_id: block.header.network_id.clone(),
        height: block.header.height,
        previous_hash: hash_to_hex(&block.header.previous_hash),
        merkle_root: hash_to_hex(&block.header.merkle_root),
        base_fee_atomic: block.header.base_fee_atomic,
        timestamp: block.header.timestamp,
        difficulty_leading_zero_bits: block.header.difficulty_leading_zero_bits,
        share_difficulty_leading_zero_bits: None,
        nonce_start: 41,
        transactions: block.transactions,
    };
    (address, template)
}

#[test]
fn template_builds_the_exact_pow_candidate() {
    let (address, template) = fixture();
    let block = template
        .validate_and_build(&address)
        .expect("valid template");

    assert_eq!(block.header.nonce, 41);
    assert_eq!(block.header.height, template.height);
    assert_eq!(hash_to_hex(&block.header.merkle_root), template.merkle_root);
}

#[test]
fn wrong_coinbase_recipient_is_rejected() {
    let (address, mut template) = fixture();
    template.transactions[0].to = "not-the-miner".to_owned();

    let error = template
        .validate_and_build(&address)
        .expect_err("wrong payout must fail");
    assert!(error.to_string().contains("coinbase paying miner_address"));
}

#[test]
fn parallel_nonce_search_finds_valid_firopow() {
    let (address, template) = fixture();
    let block = template.validate_and_build(&address).expect("template");
    // difficulty 0 in fixture → first valid FiroPoW evaluation should accept.
    let (nonce, result) = mine_firopow_solution(
        &block,
        block.header.difficulty_leading_zero_bits,
        100,
        10_000,
    )
    .expect("search")
    .expect("difficulty zero always solves");

    assert!(nonce >= 100);
    assert!(alvenqis_core::check_pow(
        &result.final_hash,
        block.header.difficulty_leading_zero_bits
    ));
    assert_ne!(result.mix_hash, Hash::zero());
}

#[test]
fn local_file_source_reads_work_and_writes_submission_atomically() {
    let directory = tempdir().expect("tempdir");
    let template_path = directory.path().join("template.json");
    let submission_path = directory.path().join("submission.json");
    let (address, template) = fixture();
    fs::write(
        &template_path,
        serde_json::to_vec_pretty(&template).expect("serialize template"),
    )
    .expect("write template");
    let source = FileWorkSource::new(template_path, submission_path.clone());

    let loaded = source.fetch_template(&address).expect("read template");
    let block = loaded.validate_and_build(&address).expect("validate");
    let pow = block.pow_result().expect("FiroPoW evaluation");
    let request = MiningSubmitRequest::from_solution(
        loaded.template_id,
        block.header.nonce,
        pow.final_hash,
        pow.mix_hash,
    );
    let response = source.submit(&request).expect("write submission");
    let written: MiningSubmitRequest =
        serde_json::from_slice(&fs::read(submission_path).expect("read submission"))
            .expect("parse submission");

    assert_eq!(response.status, SubmitStatus::PendingLocal);
    assert_eq!(written, request);
}
