//! FiroPoW 0.9.4 — canonical AlvenqisPoW v1.
//!
//! Vendored from firoorg/firo (`ProgPoW` revision **0.9.4**, `period_length = 1`).
//! Nodes use this module **only** to verify GPU-found solutions (light context).
//! Continuous product mining is GPU-only and lives in `alvenqis-miner`.
//!
//! Future: `Alvenqis PoLW v2` activates via fixed-height upgrade (see [`crate::upgrade`]).

use crate::block::Block;
use crate::crypto::Hash;
use crate::errors::{AlvenqisError, Result};
use serde::{Deserialize, Serialize};
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::sync::OnceLock;

/// Consensus algorithm id for AlvenqisPoW v1.
pub const POW_ALGORITHM_ID: &str = "firopow-0.9.4";
/// Human revision string (must match progpow::revision).
pub const FIROPOW_REVISION: &str = "0.9.4";
/// Launch PoW version number (AlvenqisPoW v1).
pub const POW_VERSION: u32 = 1;
/// Reserved future algorithm id for energy-aware PoLW (not active).
pub const FUTURE_POLW_V2_ALGORITHM_ID: &str = "alvenqis-polw-v2";
pub const FUTURE_POLW_V2_VERSION: u32 = 2;

/// Firo epoch length (blocks) — ETHASH_EPOCH_LENGTH from vendored sources.
pub const EPOCH_LENGTH: i32 = 1300;
/// ProgPoW period length — Firo sets this to 1 (kernel changes every block).
pub const PERIOD_LENGTH: i32 = 1;

#[repr(C)]
struct NativeResult {
    final_hash: [u8; 32],
    mix_hash: [u8; 32],
}

extern "C" {
    fn alvenqis_firopow_revision(out: *mut c_char, out_len: c_int) -> c_int;
    fn alvenqis_firopow_period_length() -> c_int;
    fn alvenqis_firopow_epoch_length() -> c_int;
    fn alvenqis_firopow_epoch_number(block_number: c_int) -> c_int;
    fn alvenqis_keccak256(data: *const u8, len: usize, out: *mut u8);
    fn alvenqis_firopow_hash(
        block_number: c_int,
        header_hash: *const u8,
        nonce: u64,
        out: *mut NativeResult,
    ) -> c_int;
    fn alvenqis_firopow_verify(
        block_number: c_int,
        header_hash: *const u8,
        mix_hash: *const u8,
        nonce: u64,
        boundary: *const u8,
    ) -> c_int;
    fn alvenqis_firopow_search_light(
        block_number: c_int,
        header_hash: *const u8,
        boundary: *const u8,
        start_nonce: u64,
        iterations: usize,
        found_nonce: *mut u64,
        out: *mut NativeResult,
    ) -> c_int;
    fn alvenqis_firopow_search_mt(
        block_number: c_int,
        header_hash: *const u8,
        boundary: *const u8,
        start_nonce: u64,
        iterations: usize,
        threads: c_int,
        cancel_flag: *const c_int,
        found_nonce: *mut u64,
        out: *mut NativeResult,
        hashes_done: *mut u64,
    ) -> c_int;
    fn alvenqis_firopow_prewarm_full(block_number: c_int) -> c_int;
    fn alvenqis_firopow_export_full_dag(
        block_number: c_int,
        dag_out: *mut *const u8,
        dag_bytes_out: *mut u64,
        l1_out: *mut *const u32,
        l1_words_out: *mut u32,
        full_dataset_num_items_out: *mut c_int,
    ) -> c_int;
    fn alvenqis_firopow_export_light_cache(
        block_number: c_int,
        light_out: *mut *const u32,
        light_items_out: *mut u32,
        l1_out: *mut *const u32,
        l1_words_out: *mut u32,
        full_dataset_num_items_out: *mut c_int,
    ) -> c_int;
    fn alvenqis_firopow_dataset_item_1024(block_number: c_int, index: u32, out: *mut u8) -> c_int;
    fn alvenqis_firopow_full_dataset_bytes(block_number: c_int) -> u64;
}

/// Wire/documentation identity of the active PoW algorithm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowVersion {
    pub version: u32,
    pub algorithm_id: &'static str,
}

impl PowVersion {
    pub const fn launch() -> Self {
        Self {
            version: POW_VERSION,
            algorithm_id: POW_ALGORITHM_ID,
        }
    }

    pub const fn future_polw_v2() -> Self {
        Self {
            version: FUTURE_POLW_V2_VERSION,
            algorithm_id: FUTURE_POLW_V2_ALGORITHM_ID,
        }
    }
}

/// Result of a FiroPoW evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FiroPowOutput {
    pub final_hash: Hash,
    pub mix_hash: Hash,
}

/// Full PoW validation outcome for a block header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PowValidation {
    pub version: PowVersion,
    pub final_hash: Hash,
    pub mix_hash: Hash,
    pub header_hash: Hash,
    pub required_leading_zero_bits: u8,
    pub meets_target: bool,
}

/// Keccak-256 via vendored ethash keccak.
pub fn keccak256(data: &[u8]) -> Hash {
    let mut out = [0u8; 32];
    unsafe {
        alvenqis_keccak256(data.as_ptr(), data.len(), out.as_mut_ptr());
    }
    Hash::from_bytes(out)
}

/// Epoch number for a block height (height is treated as block_number).
pub fn epoch_number(block_height: u64) -> i32 {
    let bn = height_as_i32(block_height);
    unsafe { alvenqis_firopow_epoch_number(bn) }
}

/// Native FiroPoW epoch length (blocks). Must match [`EPOCH_LENGTH`].
pub fn epoch_length() -> i32 {
    unsafe { alvenqis_firopow_epoch_length() }
}

fn height_as_i32(height: u64) -> i32 {
    i32::try_from(height).unwrap_or(i32::MAX)
}

/// Boundary (big-endian 256-bit) requiring `bits` leading zero bits on the final hash.
///
/// Equivalent to `final_hash < 2^(256-bits)` when interpreted as big-endian integer.
pub fn boundary_from_leading_zero_bits(bits: u8) -> [u8; 32] {
    let mut boundary = [0xffu8; 32];
    let full_bytes = (bits / 8) as usize;
    let rem = bits % 8;
    for b in boundary.iter_mut().take(full_bytes.min(32)) {
        *b = 0x00;
    }
    if full_bytes < 32 && rem > 0 {
        boundary[full_bytes] = 0xffu8 >> rem;
    }
    if bits == 255 {
        // 255 leading zeros → only the least significant bit of the last byte may be 1.
        return [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1,
        ];
    }
    boundary
}

/// True if `hash` (big-endian) is strictly less than `boundary` (big-endian).
pub fn hash_meets_boundary(hash: &Hash, boundary: &[u8; 32]) -> bool {
    let hb = hash.as_bytes();
    for i in 0..32 {
        if hb[i] < boundary[i] {
            return true;
        }
        if hb[i] > boundary[i] {
            return false;
        }
    }
    // equal — ethash uses <= boundary; accept equal
    true
}

/// Leading zero bits of a big-endian hash (for telemetry / DAA display).
pub fn leading_zero_bits_be(hash: &Hash) -> u32 {
    let mut total = 0u32;
    for &byte in hash.as_bytes() {
        if byte == 0 {
            total += 8;
        } else {
            total += byte.leading_zeros();
            break;
        }
    }
    total
}

/// Serialize the mining seed preimage (header **without** nonce and mix_hash).
///
/// Layout (little-endian multi-byte fields):
/// `version_u32 | previous_hash_32 | merkle_root_32 | timestamp_u64 | difficulty_u8 | height_u64`
///
/// The seed is `keccak256(this preimage)`. Nonce is applied inside FiroPoW, not here.
pub fn mining_seed_preimage(block: &Block) -> Vec<u8> {
    let h = &block.header;
    let mut bytes = Vec::with_capacity(4 + 32 + 32 + 8 + 1 + 8);
    bytes.extend_from_slice(&h.version.to_le_bytes());
    bytes.extend_from_slice(h.previous_hash.as_bytes());
    bytes.extend_from_slice(h.merkle_root.as_bytes());
    bytes.extend_from_slice(&h.timestamp.to_le_bytes());
    bytes.push(h.difficulty_leading_zero_bits);
    bytes.extend_from_slice(&h.height.to_le_bytes());
    bytes
}

/// Header hash (seed) fed into FiroPoW.
pub fn mining_header_hash(block: &Block) -> Hash {
    keccak256(&mining_seed_preimage(block))
}

/// Compute FiroPoW for `(height, header_hash, nonce)`.
pub fn firopow_hash(block_height: u64, header_hash: &Hash, nonce: u64) -> Result<FiroPowOutput> {
    let mut out = NativeResult {
        final_hash: [0u8; 32],
        mix_hash: [0u8; 32],
    };
    let rc = unsafe {
        alvenqis_firopow_hash(
            height_as_i32(block_height),
            header_hash.as_bytes().as_ptr(),
            nonce,
            &mut out,
        )
    };
    if rc != 0 {
        return Err(AlvenqisError::PowEvaluationFailed(format!(
            "native FiroPoW evaluator returned status {rc} for height {block_height} nonce {nonce}"
        )));
    }
    Ok(FiroPowOutput {
        final_hash: Hash::from_bytes(out.final_hash),
        mix_hash: Hash::from_bytes(out.mix_hash),
    })
}

/// Verify a claimed solution against a boundary.
pub fn firopow_verify(
    block_height: u64,
    header_hash: &Hash,
    mix_hash: &Hash,
    nonce: u64,
    boundary: &[u8; 32],
) -> bool {
    unsafe {
        alvenqis_firopow_verify(
            height_as_i32(block_height),
            header_hash.as_bytes().as_ptr(),
            mix_hash.as_bytes().as_ptr(),
            nonce,
            boundary.as_ptr(),
        ) == 1
    }
}

/// Host light-context search (single-thread, no full DAG).
pub fn firopow_search_light(
    block_height: u64,
    header_hash: &Hash,
    boundary: &[u8; 32],
    start_nonce: u64,
    iterations: u64,
) -> Result<Option<(u64, FiroPowOutput)>> {
    let mut found = 0u64;
    let mut out = NativeResult {
        final_hash: [0u8; 32],
        mix_hash: [0u8; 32],
    };
    let rc = unsafe {
        alvenqis_firopow_search_light(
            height_as_i32(block_height),
            header_hash.as_bytes().as_ptr(),
            boundary.as_ptr(),
            start_nonce,
            iterations as usize,
            &mut found,
            &mut out,
        )
    };
    match rc {
        0 => Ok(None),
        1 => Ok(Some((
            found,
            FiroPowOutput {
                final_hash: Hash::from_bytes(out.final_hash),
                mix_hash: Hash::from_bytes(out.mix_hash),
            },
        ))),
        _ => Err(AlvenqisError::InvalidPow {
            required: 0,
            actual: 0,
        }),
    }
}

/// Multi-threaded immutable light-context FiroPoW search for tests and genesis tooling.
/// Returns `(nonce, output, hashes_done)` when a solution is found.
pub fn firopow_search_mt(
    block_height: u64,
    header_hash: &Hash,
    boundary: &[u8; 32],
    start_nonce: u64,
    iterations: u64,
    threads: i32,
    cancel: Option<&std::sync::atomic::AtomicI32>,
) -> Result<Option<(u64, FiroPowOutput, u64)>> {
    let mut found = 0u64;
    let mut hashes_done = 0u64;
    let mut out = NativeResult {
        final_hash: [0u8; 32],
        mix_hash: [0u8; 32],
    };
    let cancel_ptr = cancel
        .map(|c| c as *const std::sync::atomic::AtomicI32 as *const c_int)
        .unwrap_or(std::ptr::null());
    let rc = unsafe {
        alvenqis_firopow_search_mt(
            height_as_i32(block_height),
            header_hash.as_bytes().as_ptr(),
            boundary.as_ptr(),
            start_nonce,
            iterations as usize,
            threads,
            cancel_ptr,
            &mut found,
            &mut out,
            &mut hashes_done,
        )
    };
    match rc {
        0 => Ok(None),
        1 => Ok(Some((
            found,
            FiroPowOutput {
                final_hash: Hash::from_bytes(out.final_hash),
                mix_hash: Hash::from_bytes(out.mix_hash),
            },
            hashes_done,
        ))),
        _ => Err(AlvenqisError::InvalidPow {
            required: 0,
            actual: 0,
        }),
    }
}

/// Pre-allocate full epoch DAG (lazy slots) for faster subsequent searches.
pub fn firopow_prewarm(block_height: u64) -> Result<()> {
    let rc = unsafe { alvenqis_firopow_prewarm_full(height_as_i32(block_height)) };
    if rc == 0 {
        Ok(())
    } else {
        Err(AlvenqisError::InvalidPow {
            required: 0,
            actual: 0,
        })
    }
}

/// Host view of a fully materialized epoch DAG (pointers into native cache).
///
/// Safe to memcpy into CUDA device memory. Lifetime is process-static for the epoch.
#[derive(Clone, Copy, Debug)]
pub struct FiroPowDagView {
    pub dag_ptr: *const u8,
    pub dag_bytes: u64,
    pub l1_ptr: *const u32,
    pub l1_words: u32,
    /// Number of hash1024 items in the full dataset.
    pub full_dataset_num_items: i32,
}

/// Host view of the small epoch inputs required to generate the full DAG on a GPU.
#[derive(Clone, Copy, Debug)]
pub struct FiroPowLightCacheView {
    /// `light_cache_items * 16` little-endian `u32` words (hash512 entries).
    pub light_cache_ptr: *const u32,
    pub light_cache_items: u32,
    pub l1_ptr: *const u32,
    pub l1_words: u32,
    /// Number of hash1024 items in the full dataset.
    pub full_dataset_num_items: i32,
}

// SAFETY: native epoch contexts are process-global and immutable after creation.
unsafe impl Send for FiroPowLightCacheView {}
unsafe impl Sync for FiroPowLightCacheView {}

/// Build only the small host light context and expose it for CUDA DAG generation.
pub fn firopow_export_light_cache(block_height: u64) -> Result<FiroPowLightCacheView> {
    let mut light_cache_ptr: *const u32 = std::ptr::null();
    let mut light_cache_items = 0u32;
    let mut l1_ptr: *const u32 = std::ptr::null();
    let mut l1_words = 0u32;
    let mut full_dataset_num_items: c_int = 0;
    let rc = unsafe {
        alvenqis_firopow_export_light_cache(
            height_as_i32(block_height),
            &mut light_cache_ptr,
            &mut light_cache_items,
            &mut l1_ptr,
            &mut l1_words,
            &mut full_dataset_num_items,
        )
    };
    if rc != 0
        || light_cache_ptr.is_null()
        || light_cache_items == 0
        || l1_ptr.is_null()
        || l1_words == 0
        || full_dataset_num_items <= 0
    {
        return Err(AlvenqisError::InvalidPow {
            required: 0,
            actual: 0,
        });
    }
    Ok(FiroPowLightCacheView {
        light_cache_ptr,
        light_cache_items,
        l1_ptr,
        l1_words,
        full_dataset_num_items,
    })
}

/// Canonical host calculation of one hash1024 DAG item for GPU parity checks.
pub fn firopow_dataset_item_1024(block_height: u64, index: u32) -> Result<[u8; 128]> {
    let mut output = [0u8; 128];
    let rc = unsafe {
        alvenqis_firopow_dataset_item_1024(height_as_i32(block_height), index, output.as_mut_ptr())
    };
    if rc == 0 {
        Ok(output)
    } else {
        Err(AlvenqisError::InvalidPow {
            required: 0,
            actual: 0,
        })
    }
}

// SAFETY: native epoch contexts are process-global and immutable after materialize.
unsafe impl Send for FiroPowDagView {}
unsafe impl Sync for FiroPowDagView {}

/// Materialize full DAG + L1 cache and return host pointers for GPU upload.
pub fn firopow_export_full_dag(block_height: u64) -> Result<FiroPowDagView> {
    let mut dag_ptr: *const u8 = std::ptr::null();
    let mut dag_bytes: u64 = 0;
    let mut l1_ptr: *const u32 = std::ptr::null();
    let mut l1_words: u32 = 0;
    let mut num_items: c_int = 0;
    let rc = unsafe {
        alvenqis_firopow_export_full_dag(
            height_as_i32(block_height),
            &mut dag_ptr,
            &mut dag_bytes,
            &mut l1_ptr,
            &mut l1_words,
            &mut num_items,
        )
    };
    if rc != 0 || dag_ptr.is_null() || l1_ptr.is_null() || dag_bytes == 0 {
        return Err(AlvenqisError::InvalidPow {
            required: 0,
            actual: 0,
        });
    }
    Ok(FiroPowDagView {
        dag_ptr,
        dag_bytes,
        l1_ptr,
        l1_words,
        full_dataset_num_items: num_items,
    })
}

/// Full dataset size in bytes for the epoch of `block_height`.
pub fn firopow_full_dataset_bytes(block_height: u64) -> u64 {
    unsafe { alvenqis_firopow_full_dataset_bytes(height_as_i32(block_height)) }
}

/// Canonical FiroPoW engine for AlvenqisPoW v1.
#[derive(Clone, Copy, Debug, Default)]
pub struct FiroPow;

impl FiroPow {
    pub const fn version(self) -> PowVersion {
        PowVersion::launch()
    }

    pub fn hash_header(self, block: &Block, nonce: u64) -> Result<FiroPowOutput> {
        let seed = mining_header_hash(block);
        firopow_hash(block.header.height, &seed, nonce)
    }

    pub fn meets_target(self, final_hash: &Hash, required_leading_zero_bits: u8) -> bool {
        let boundary = boundary_from_leading_zero_bits(required_leading_zero_bits);
        hash_meets_boundary(final_hash, &boundary)
    }

    pub fn validate(
        self,
        block: &Block,
        nonce: u64,
        mix_hash: &Hash,
        required_leading_zero_bits: u8,
    ) -> Result<PowValidation> {
        let header_hash = mining_header_hash(block);
        let boundary = boundary_from_leading_zero_bits(required_leading_zero_bits);
        let ok = firopow_verify(
            block.header.height,
            &header_hash,
            mix_hash,
            nonce,
            &boundary,
        );
        let computed = firopow_hash(block.header.height, &header_hash, nonce)?;
        let meets = ok
            && computed.mix_hash == *mix_hash
            && self.meets_target(&computed.final_hash, required_leading_zero_bits);
        Ok(PowValidation {
            version: self.version(),
            final_hash: computed.final_hash,
            mix_hash: computed.mix_hash,
            header_hash,
            required_leading_zero_bits,
            meets_target: meets,
        })
    }

    pub fn validate_block_header(self, block: &Block) -> Result<PowValidation> {
        self.validate(
            block,
            block.header.nonce,
            &block.header.mix_hash,
            block.header.difficulty_leading_zero_bits,
        )
    }

    pub fn ensure_valid(self, block: &Block) -> Result<PowValidation> {
        let result = self.validate_block_header(block)?;
        if !result.meets_target {
            let actual = leading_zero_bits_be(&result.final_hash);
            return Err(AlvenqisError::InvalidPow {
                required: block.header.difficulty_leading_zero_bits,
                actual,
            });
        }
        Ok(result)
    }
}

/// Convenience: target check on a final hash.
#[inline]
pub fn check_pow(hash: &Hash, required_leading_zero_bits: u8) -> bool {
    FiroPow.meets_target(hash, required_leading_zero_bits)
}

/// Convenience: compute final hash for block at nonce (sets mix via computation).
#[inline]
pub fn pow_hash(block: &Block, nonce: u64) -> Result<Hash> {
    Ok(FiroPow.hash_header(block, nonce)?.final_hash)
}

/// Convenience: full validate at block's stored fields.
#[inline]
pub fn validate_pow(block: &Block) -> Result<PowValidation> {
    FiroPow.validate_block_header(block)
}

/// Mine a solution for tests/genesis (multi-thread immutable light-context search).
/// Product continuous mining is GPU-orchestrated in `alvenqis-miner`.
pub fn mine_firopow_solution(
    block: &Block,
    required_leading_zero_bits: u8,
    start_nonce: u64,
    max_iterations: u64,
) -> Result<Option<(u64, FiroPowOutput)>> {
    let header_hash = mining_header_hash(block);
    let boundary = boundary_from_leading_zero_bits(required_leading_zero_bits);
    let _ = firopow_prewarm(block.header.height);
    let threads = std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(4);
    Ok(firopow_search_mt(
        block.header.height,
        &header_hash,
        &boundary,
        start_nonce,
        max_iterations,
        threads,
        None,
    )?
    .map(|(n, o, _)| (n, o)))
}

static NATIVE_OK: OnceLock<bool> = OnceLock::new();

/// Returns true when the native FiroPoW library linked successfully.
pub fn native_available() -> bool {
    *NATIVE_OK.get_or_init(|| {
        // c_char is i8 on Windows, u8 on Android/Linux — do not hardcode i8.
        let mut buf = [0 as c_char; 16];
        let n = unsafe { alvenqis_firopow_revision(buf.as_mut_ptr(), buf.len() as c_int) };
        n > 0
            && unsafe { alvenqis_firopow_period_length() } == PERIOD_LENGTH
            && epoch_length() == EPOCH_LENGTH
    })
}

// silence unused import warning for c_void in some toolchains
const _: *const c_void = std::ptr::null();

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_hex32(s: &str) -> [u8; 32] {
        let clean = s.trim();
        let bytes = hex::decode(clean).expect("hex");
        assert_eq!(bytes.len(), 32);
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        out
    }

    #[test]
    fn multithreaded_search_returns_lowest_valid_nonce() {
        let header_hash = Hash::from_bytes([0x5a; 32]);
        let boundary = [0xff; 32];
        let expected = firopow_search_light(0, &header_hash, &boundary, 0, 64)
            .expect("single-threaded search")
            .expect("solution");

        assert_eq!(expected.0, 0);
        for threads in [1, 2, 4, 8] {
            let actual = firopow_search_mt(0, &header_hash, &boundary, 0, 64, threads, None)
                .expect("multi-threaded search")
                .expect("solution");
            assert_eq!(actual.0, expected.0);
            assert_eq!(actual.1, expected.1);
        }
    }

    #[test]
    fn native_reports_firopow_094_period_1() {
        assert!(native_available());
        assert_eq!(unsafe { alvenqis_firopow_period_length() }, 1);
        assert_eq!(unsafe { alvenqis_firopow_epoch_length() }, EPOCH_LENGTH);
    }

    #[test]
    fn official_vector_block_1() {
        // From firo firopow_test_vectors.hpp case {1, ...}
        let header =
            parse_hex32("2d794e900dcad779e658de9078d9a88eee87d75f7b09a8fdd270d3a8e76650c7");
        let boundary =
            parse_hex32("0001869e7a058e2aaf266cd2f166fb85c98d651e60eadbbe72bb0a36f8802805");
        let nonce = u64::from_str_radix("85f22c9b3cd2f123", 16).unwrap();
        let expect_mix =
            parse_hex32("cfab3766331d6c4e6913e6688a71e4c26b7f36c1581cdbec0f5b19db8956eb50");
        let expect_final =
            parse_hex32("00017c7de1fa499314f9e3dd3537546982073624f7d478592cf28a6d13929f2d");

        let header_hash = Hash::from_bytes(header);
        let out = firopow_hash(1, &header_hash, nonce).expect("hash");
        assert_eq!(out.mix_hash.as_bytes(), &expect_mix);
        assert_eq!(out.final_hash.as_bytes(), &expect_final);
        assert!(firopow_verify(
            1,
            &header_hash,
            &out.mix_hash,
            nonce,
            &boundary
        ));
    }

    #[test]
    fn concurrent_hashing_is_deterministic() {
        let header = Hash::from_bytes(parse_hex32(
            "2d794e900dcad779e658de9078d9a88eee87d75f7b09a8fdd270d3a8e76650c7",
        ));
        let nonce = u64::from_str_radix("85f22c9b3cd2f123", 16).unwrap();
        let expected = firopow_hash(1, &header, nonce).expect("reference hash");
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let expected = expected.clone();
                std::thread::spawn(move || {
                    for _ in 0..16 {
                        assert_eq!(
                            firopow_hash(1, &header, nonce).expect("concurrent hash"),
                            expected
                        );
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().expect("hash worker");
        }
    }

    #[test]
    fn boundary_leading_zeros_monotonic() {
        let h = Hash::from_bytes([
            0x00, 0x0f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff,
        ]);
        assert!(check_pow(&h, 8));
        assert!(check_pow(&h, 12));
        assert!(!check_pow(&h, 13));
    }

    #[test]
    fn pow_version_is_firopow_v1() {
        let v = FiroPow.version();
        assert_eq!(v.version, 1);
        assert_eq!(v.algorithm_id, "firopow-0.9.4");
        assert_eq!(PowVersion::future_polw_v2().version, 2);
    }
}
