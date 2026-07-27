//! FiroPoW layout / GPU parity regression vectors.
//!
//! Consensus rules live in alvenqis-core. Product continuous mining is GPU-only;
//! these tests cover seed preimage layout, core hash/target, and optional GPU parity.

use alvenqis_core::{
    check_pow, firopow, leading_zero_bits, Amount, Block, Hash, Network, PrivateKey, Transaction,
};
use alvenqis_miner::{BackendConfig, CudaGpuBackend, MiningBackend, MiningJob, MiningMode};
use std::time::Duration;

fn sample_block(network: Network, height: u64, difficulty: u8) -> Block {
    let address = alvenqis_core::Address::from_public_key_for_network(
        &PrivateKey::generate().public_key(),
        network,
    )
    .to_string();
    let tx = Transaction::coinbase(height, address, Amount::from_atomic(1)).expect("coinbase");
    let mut block = Block::new(
        network,
        height,
        Hash::zero(),
        1_700_000_000,
        1,
        42,
        vec![tx],
    )
    .expect("block");
    block.header.difficulty_leading_zero_bits = difficulty;
    block
}

#[test]
fn mining_seed_preimage_is_stable_and_excludes_nonce_mix() {
    for network in [Network::Devnet, Network::MainnetCandidate] {
        let block = sample_block(network, 650, 8);
        let seed_a = firopow::mining_seed_preimage(&block);
        let seed_b = firopow::mining_seed_preimage(&block);
        assert_eq!(seed_a, seed_b);
        // version(4) + prev(32) + merkle(32) + timestamp(8) + difficulty(1) + height(8)
        assert_eq!(seed_a.len(), 4 + 32 + 32 + 8 + 1 + 8);
        let header_hash = firopow::mining_header_hash(&block);
        assert_ne!(header_hash, Hash::zero());
    }
}

#[test]
fn core_firopow_hash_and_target_boundary() {
    let block = sample_block(Network::Devnet, 1, 4);
    let out = block.pow_hash_with_nonce(0).expect("FiroPoW");
    assert!(check_pow(&out.final_hash, 0));
    let bits = leading_zero_bits(&out.final_hash) as u8;
    assert!(check_pow(&out.final_hash, bits));
    if bits < 255 {
        assert!(!check_pow(&out.final_hash, bits + 1));
    }
    assert_ne!(out.mix_hash, Hash::zero());
}

#[test]
fn deterministic_nonces_produce_stable_firopow_hashes() {
    let block = sample_block(Network::Devnet, 100, 12);
    let h0 = block.pow_hash_with_nonce(0).expect("FiroPoW");
    let h_max = block.pow_hash_with_nonce(u64::MAX).expect("FiroPoW");
    assert_ne!(h0.final_hash, h_max.final_hash);
    assert_eq!(
        h0.final_hash,
        block.pow_hash_with_nonce(0).expect("FiroPoW").final_hash
    );
    assert_eq!(
        h_max.mix_hash,
        block
            .pow_hash_with_nonce(u64::MAX)
            .expect("FiroPoW")
            .mix_hash
    );
}

#[test]
fn product_rejects_cpu_modes_gpu_only_policy() {
    // Continuous CPU mining is not a product feature.
    assert!(MiningMode::parse("cpu").is_err());
    assert!(MiningMode::parse("cpu-gpu").is_err());
    assert!(MiningMode::parse("auto").is_ok());
    assert!(MiningMode::parse("gpu").is_ok());
    assert!(MiningMode::parse("cuda").is_ok());
}

#[test]
fn mining_seed_length_is_within_gpu_kernel_limit() {
    for network in [Network::Devnet, Network::MainnetCandidate] {
        let block = sample_block(network, 650, 20);
        let seed = firopow::mining_seed_preimage(&block);
        assert!(
            seed.len() <= 256,
            "seed len {} exceeds GPU kernel buffer for {network:?}",
            seed.len()
        );
        assert!(seed.len() >= 64, "expected multi-field FiroPoW seed");
    }
}

#[test]
fn cuda_gpu_hashes_match_core_when_device_present() {
    let require_cuda = std::env::var_os("ALVENQIS_REQUIRE_CUDA_TEST").is_some();
    let block = sample_block(Network::Devnet, 7, 0);
    let mut gpu = CudaGpuBackend::default();
    if let Err(error) = gpu.initialize(BackendConfig {
        batch_size: 256,
        device_ids: Vec::new(),
        intensity: 100,
    }) {
        assert!(!require_cuda, "required CUDA backend failed: {error}");
        eprintln!("skipping CUDA parity: {error}");
        return;
    }

    let devices = match gpu.available_devices() {
        Ok(d) if !d.is_empty() => d,
        Ok(_) => {
            assert!(!require_cuda, "required CUDA test found no devices");
            eprintln!("skipping CUDA parity: no devices");
            return;
        }
        Err(error) => {
            assert!(!require_cuda, "required CUDA enumeration failed: {error}");
            eprintln!("skipping CUDA parity: {error}");
            return;
        }
    };
    let _ = devices;

    for start in [0_u64, 1, 12_345, u64::MAX] {
        let job = MiningJob {
            template_id: format!("gpu-parity-{start}"),
            block: block.clone(),
            difficulty_leading_zero_bits: 0,
            start_nonce: start,
            max_nonces: 1,
        };
        let solution = gpu
            .mine_batch(&job)
            .unwrap_or_else(|error| panic!("CUDA mine_batch failed at nonce {start}: {error}"))
            .expect("difficulty-zero CUDA batch must return a solution");
        let core = block
            .pow_hash_with_nonce(solution.nonce)
            .expect("core recompute");
        assert_eq!(
            solution.final_hash, core.final_hash,
            "GPU final_hash must match core for nonce {}",
            solution.nonce
        );
        assert_eq!(
            solution.mix_hash, core.mix_hash,
            "GPU mix_hash must match core for nonce {}",
            solution.nonce
        );
        assert_eq!(solution.nonce, start);
    }

    let benchmark_job = MiningJob {
        template_id: "gpu-benchmark".into(),
        block,
        difficulty_leading_zero_bits: 255,
        start_nonce: 0,
        max_nonces: 256,
    };
    let benchmark = gpu
        .benchmark(&benchmark_job, Duration::from_millis(200))
        .expect("CUDA benchmark");
    assert!(benchmark.hashes > 0);
    assert!(benchmark.hashrate_hs > 0.0);
}
