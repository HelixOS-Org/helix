//! Constants and basic configuration for federated learning.

/// Maximum number of federated clients
pub const MAX_CLIENTS: usize = 1000;

/// Default local epochs
pub const DEFAULT_LOCAL_EPOCHS: usize = 5;

/// Default batch size
pub const DEFAULT_BATCH_SIZE: usize = 32;

/// Default learning rate
pub const DEFAULT_LR: f64 = 0.01;

/// Noise multiplier for differential privacy
pub const DEFAULT_NOISE_MULTIPLIER: f64 = 0.1;

/// Clipping bound for gradients
pub const DEFAULT_CLIP_BOUND: f64 = 1.0;

/// LCG random number generator
#[inline]
pub fn lcg_next(state: u64) -> u64 {
    state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

/// Box-Muller transform for Gaussian noise
#[inline]
pub fn box_muller(seed: u64) -> f64 {
    let u1 = (seed as f64 / u64::MAX as f64).max(1e-10);
    let seed2 = lcg_next(seed);
    let u2 = seed2 as f64 / u64::MAX as f64;

    libm::sqrt(-2.0 * libm::log(u1)) * libm::cos(2.0 * core::f64::consts::PI * u2)
}
