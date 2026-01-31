//! # Math Utilities for no_std
//!
//! Provides math functions for no_std environments using libm.

/// Square root for f64
#[inline]
pub fn sqrt(x: f64) -> f64 {
    libm::sqrt(x)
}

/// Power function: base^exp for f64
#[inline]
pub fn pow(base: f64, exp: f64) -> f64 {
    libm::pow(base, exp)
}

/// Integer power: base^exp for f64
#[inline]
pub fn powi(base: f64, exp: i32) -> f64 {
    libm::pow(base, exp as f64)
}

/// Natural logarithm
#[inline]
pub fn ln(x: f64) -> f64 {
    libm::log(x)
}

/// Log base 2
#[inline]
pub fn log2(x: f64) -> f64 {
    libm::log2(x)
}

/// Log base 10
#[inline]
pub fn log10(x: f64) -> f64 {
    libm::log10(x)
}

/// Exponential e^x
#[inline]
pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}

/// Absolute value
#[inline]
pub fn abs(x: f64) -> f64 {
    libm::fabs(x)
}

/// Floor
#[inline]
pub fn floor(x: f64) -> f64 {
    libm::floor(x)
}

/// Ceiling
#[inline]
pub fn ceil(x: f64) -> f64 {
    libm::ceil(x)
}

/// Round
#[inline]
pub fn round(x: f64) -> f64 {
    libm::round(x)
}

/// Sine
#[inline]
pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}

/// Cosine
#[inline]
pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}

/// Tangent
#[inline]
pub fn tan(x: f64) -> f64 {
    libm::tan(x)
}

/// Hyperbolic tangent (for neural networks)
#[inline]
pub fn tanh(x: f64) -> f64 {
    libm::tanh(x)
}

/// Minimum of two values
#[inline]
pub fn min(a: f64, b: f64) -> f64 {
    libm::fmin(a, b)
}

/// Maximum of two values
#[inline]
pub fn max(a: f64, b: f64) -> f64 {
    libm::fmax(a, b)
}

// ============================================================================
// F32 VARIANTS
// ============================================================================

/// Square root for f32
#[inline]
pub fn sqrtf(x: f32) -> f32 {
    libm::sqrtf(x)
}

/// Power for f32
#[inline]
pub fn powf(base: f32, exp: f32) -> f32 {
    libm::powf(base, exp)
}

/// Exp for f32
#[inline]
pub fn expf(x: f32) -> f32 {
    libm::expf(x)
}

/// Tanh for f32
#[inline]
pub fn tanhf(x: f32) -> f32 {
    libm::tanhf(x)
}
