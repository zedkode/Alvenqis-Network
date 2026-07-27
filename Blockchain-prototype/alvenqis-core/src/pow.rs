//! Canonical Alvenqis Proof-of-Work — **FiroPoW 0.9.4** (AlvenqisPoW v1).
//!
//! Re-exports the FiroPoW module. Blake3 is **not** used for mining.
//! Future: Alvenqis PoLW v2 activates via fixed-height upgrade only.

pub use crate::firopow::{
    boundary_from_leading_zero_bits, check_pow, epoch_number, firopow_hash, firopow_prewarm,
    firopow_search_light, firopow_search_mt, firopow_verify, hash_meets_boundary, keccak256,
    leading_zero_bits_be, mine_firopow_solution, mining_header_hash, mining_seed_preimage,
    native_available, pow_hash, validate_pow, FiroPow, FiroPowOutput, PowValidation, PowVersion,
    EPOCH_LENGTH, FIROPOW_REVISION, FUTURE_POLW_V2_ALGORITHM_ID, FUTURE_POLW_V2_VERSION,
    PERIOD_LENGTH, POW_ALGORITHM_ID, POW_VERSION,
};

/// Backward-compatible name used by older call sites (now FiroPoW).
pub type Blake3LeadingZeroPow = FiroPow;
