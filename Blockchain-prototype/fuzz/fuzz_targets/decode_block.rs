#![no_main]

use alvenqis_core::Block;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(block) = serde_json::from_slice::<Block>(data) {
        let _ = std::hint::black_box(block.network());
        let _ = std::hint::black_box(block.recompute_merkle_root());
        let _ = std::hint::black_box(block.header_bytes());
        let _ = std::hint::black_box(serde_json::to_vec(&block));
    }
});
