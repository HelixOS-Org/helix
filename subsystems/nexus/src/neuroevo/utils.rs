//! Utility functions for neuroevolution.

/// Linear congruential generator
#[inline]
pub fn lcg_next(state: u64) -> u64 {
    state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

/// Generate a random weight in [-2, 2]
#[inline(always)]
pub fn random_weight(seed: u64) -> f64 {
    (seed as f64 / u64::MAX as f64) * 4.0 - 2.0
}
