//! Canonical validation for CUDA-produced FiroPoW candidates.
//!
//! This module deliberately contains no nonce search. Continuous mining in the
//! product is CUDA-only; CPU FiroPoW remains in `alvenqis-core` solely for node
//! consensus validation and tightly scoped genesis/test utilities.

#[cfg(feature = "gpu-cuda")]
use crate::Result;
#[cfg(feature = "gpu-cuda")]
use alvenqis_core::{firopow, Block, Hash};

#[cfg(feature = "gpu-cuda")]
pub(crate) fn revalidate_solution(
    block: &Block,
    nonce: u64,
    mix_hash: &Hash,
    difficulty_leading_zero_bits: u8,
) -> Result<bool> {
    let mut probe = block.clone();
    probe.header.nonce = nonce;
    probe.header.mix_hash = *mix_hash;
    match firopow::FiroPow.validate(&probe, nonce, mix_hash, difficulty_leading_zero_bits) {
        Ok(validation) => Ok(validation.meets_target),
        Err(_) => Ok(false),
    }
}
