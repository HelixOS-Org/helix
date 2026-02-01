//! # Math Utilities for no_std
//!
//! Provides math functions for no_std environments using libm.

/// Extension trait for f64 math operations in no_std
pub trait F64Ext {
    fn sqrt(self) -> f64;
    fn ln(self) -> f64;
    fn log2(self) -> f64;
    fn log10(self) -> f64;
    fn exp(self) -> f64;
    fn powi(self, n: i32) -> f64;
    fn powf(self, n: f64) -> f64;
    fn sin(self) -> f64;
    fn cos(self) -> f64;
    fn tan(self) -> f64;
    fn tanh(self) -> f64;
    fn abs(self) -> f64;
    fn floor(self) -> f64;
    fn ceil(self) -> f64;
    fn round(self) -> f64;
}

impl F64Ext for f64 {
    #[inline]
    fn sqrt(self) -> f64 { libm::sqrt(self) }
    #[inline]
    fn ln(self) -> f64 { libm::log(self) }
    #[inline]
    fn log2(self) -> f64 { libm::log2(self) }
    #[inline]
    fn log10(self) -> f64 { libm::log10(self) }
    #[inline]
    fn exp(self) -> f64 { libm::exp(self) }
    #[inline]
    fn powi(self, n: i32) -> f64 { libm::pow(self, n as f64) }
    #[inline]
    fn powf(self, n: f64) -> f64 { libm::pow(self, n) }
    #[inline]
    fn sin(self) -> f64 { libm::sin(self) }
    #[inline]
    fn cos(self) -> f64 { libm::cos(self) }
    #[inline]
    fn tan(self) -> f64 { libm::tan(self) }
    #[inline]
    fn tanh(self) -> f64 { libm::tanh(self) }
    #[inline]
    fn abs(self) -> f64 { libm::fabs(self) }
    #[inline]
    fn floor(self) -> f64 { libm::floor(self) }
    #[inline]
    fn ceil(self) -> f64 { libm::ceil(self) }
    #[inline]
    fn round(self) -> f64 { libm::round(self) }
}

/// Extension trait for f32 math operations in no_std
pub trait F32Ext {
    fn sqrt(self) -> f32;
    fn ln(self) -> f32;
    fn exp(self) -> f32;
    fn tanh(self) -> f32;
    fn powf(self, n: f32) -> f32;
}

impl F32Ext for f32 {
    #[inline]
    fn sqrt(self) -> f32 { libm::sqrtf(self) }
    #[inline]
    fn ln(self) -> f32 { libm::logf(self) }
    #[inline]
    fn exp(self) -> f32 { libm::expf(self) }
    #[inline]
    fn tanh(self) -> f32 { libm::tanhf(self) }
    #[inline]
    fn powf(self, n: f32) -> f32 { libm::powf(self, n) }
}

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
