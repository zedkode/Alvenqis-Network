#![no_main]

use alvenqis_core::Transaction;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(transaction) = serde_json::from_slice::<Transaction>(data) {
        let _ = std::hint::black_box(transaction.network());
        let _ = std::hint::black_box(transaction.verify());
        let _ = std::hint::black_box(transaction.encode());
        let _ = std::hint::black_box(transaction.txid());
        let _ = std::hint::black_box(serde_json::to_vec(&transaction));
    }
});
