// SPDX-License-Identifier: GPL-2.0
//! # no_std Math Extensions
//!
//! Provides `.sqrt()`, `.powi()`, `.ln()`, `.exp()`, `.ceil()`, `.floor()`,
//! `.log2()`, `.sin()`, `.cos()`, `.round()`, `.powf()`, `.log10()` methods
//! on `f32` and `f64` in `no_std` environments via `libm`.
//!
//! Import the traits `F32Ext` and `F64Ext` to use these methods.

/// Extension trait providing math methods for `f32` in `no_std`.
pub trait F32Ext {
    fn sqrt(self) -> f32;
    fn powi(self, n: i32) -> f32;
    fn ln(self) -> f32;
    fn exp(self) -> f32;
    fn ceil(self) -> f32;
    fn floor(self) -> f32;
    fn round(self) -> f32;
    fn log2(self) -> f32;
    fn log10(self) -> f32;
    fn sin(self) -> f32;
    fn cos(self) -> f32;
    fn powf(self, n: f32) -> f32;
    fn tan(self) -> f32;
    fn atan2(self, other: f32) -> f32;
    fn copied(self) -> f32;
}

impl F32Ext for f32 {
    #[inline(always)]
    fn sqrt(self) -> f32 { libm::sqrtf(self) }
    #[inline(always)]
    fn powi(self, n: i32) -> f32 {
        let mut result = 1.0f32;
        let mut base = self;
        let mut exp = if n < 0 { -n as u32 } else { n as u32 };
        while exp > 0 {
            if exp & 1 == 1 { result *= base; }
            base *= base;
            exp >>= 1;
        }
        if n < 0 { 1.0 / result } else { result }
    }
    #[inline(always)]
    fn ln(self) -> f32 { libm::logf(self) }
    #[inline(always)]
    fn exp(self) -> f32 { libm::expf(self) }
    #[inline(always)]
    fn ceil(self) -> f32 { libm::ceilf(self) }
    #[inline(always)]
    fn floor(self) -> f32 { libm::floorf(self) }
    #[inline(always)]
    fn round(self) -> f32 { libm::roundf(self) }
    #[inline(always)]
    fn log2(self) -> f32 { libm::log2f(self) }
    #[inline(always)]
    fn log10(self) -> f32 { libm::log10f(self) }
    #[inline(always)]
    fn sin(self) -> f32 { libm::sinf(self) }
    #[inline(always)]
    fn cos(self) -> f32 { libm::cosf(self) }
    #[inline(always)]
    fn powf(self, n: f32) -> f32 { libm::powf(self, n) }
    #[inline(always)]
    fn tan(self) -> f32 { libm::tanf(self) }
    #[inline(always)]
    fn atan2(self, other: f32) -> f32 { libm::atan2f(self, other) }
    #[inline(always)]
    fn copied(self) -> f32 { self }
}

/// Extension trait providing math methods for `f64` in `no_std`.
pub trait F64Ext {
    fn sqrt(self) -> f64;
    fn powi(self, n: i32) -> f64;
    fn ln(self) -> f64;
    fn exp(self) -> f64;
    fn ceil(self) -> f64;
    fn floor(self) -> f64;
    fn round(self) -> f64;
    fn log2(self) -> f64;
    fn log10(self) -> f64;
    fn sin(self) -> f64;
    fn cos(self) -> f64;
    fn powf(self, n: f64) -> f64;
    fn tan(self) -> f64;
    fn atan2(self, other: f64) -> f64;
    fn copied(self) -> f64;
}

impl F64Ext for f64 {
    #[inline(always)]
    fn sqrt(self) -> f64 { libm::sqrt(self) }
    #[inline(always)]
    fn powi(self, n: i32) -> f64 {
        let mut result = 1.0f64;
        let mut base = self;
        let mut exp = if n < 0 { -n as u32 } else { n as u32 };
        while exp > 0 {
            if exp & 1 == 1 { result *= base; }
            base *= base;
            exp >>= 1;
        }
        if n < 0 { 1.0 / result } else { result }
    }
    #[inline(always)]
    fn ln(self) -> f64 { libm::log(self) }
    #[inline(always)]
    fn exp(self) -> f64 { libm::exp(self) }
    #[inline(always)]
    fn ceil(self) -> f64 { libm::ceil(self) }
    #[inline(always)]
    fn floor(self) -> f64 { libm::floor(self) }
    #[inline(always)]
    fn round(self) -> f64 { libm::round(self) }
    #[inline(always)]
    fn log2(self) -> f64 { libm::log2(self) }
    #[inline(always)]
    fn log10(self) -> f64 { libm::log10(self) }
    #[inline(always)]
    fn sin(self) -> f64 { libm::sin(self) }
    #[inline(always)]
    fn cos(self) -> f64 { libm::cos(self) }
    #[inline(always)]
    fn powf(self, n: f64) -> f64 { libm::pow(self, n) }
    #[inline(always)]
    fn tan(self) -> f64 { libm::tan(self) }
    #[inline(always)]
    fn atan2(self, other: f64) -> f64 { libm::atan2(self, other) }
    #[inline(always)]
    fn copied(self) -> f64 { self }
}
