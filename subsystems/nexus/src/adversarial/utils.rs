//! Utility functions for the adversarial defense module.

use core::f64::consts::PI;

/// Sign function
#[inline]
pub fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// LCG random number generator
#[inline]
pub fn lcg_next(state: u64) -> u64 {
    state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

/// Box-Muller transform
#[inline]
pub fn box_muller(seed: u64) -> f64 {
    let u1 = (seed as f64 / u64::MAX as f64).max(1e-10);
    let seed2 = lcg_next(seed);
    let u2 = seed2 as f64 / u64::MAX as f64;

    libm::sqrt(-2.0 * libm::log(u1)) * libm::cos(2.0 * PI * u2)
}

/// Inverse normal CDF approximation
pub fn inv_normal_cdf(p: f64) -> f64 {
    // Approximation using Abramowitz and Stegun formula 26.2.23
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }

    let sign = if p < 0.5 { -1.0 } else { 1.0 };
    let p = if p < 0.5 { p } else { 1.0 - p };

    let t = libm::sqrt(-2.0 * libm::log(p));

    // Coefficients
    let c0 = 2.515517;
    let c1 = 0.802853;
    let c2 = 0.010328;
    let d1 = 1.432788;
    let d2 = 0.189269;
    let d3 = 0.001308;

    let num = c0 + c1 * t + c2 * t * t;
    let den = 1.0 + d1 * t + d2 * t * t + d3 * t * t * t;

    sign * (t - num / den)
}
