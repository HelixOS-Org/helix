//! SIMD operation types.

/// SIMD operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimdType {
    /// No SIMD
    None,
    /// 128-bit SIMD (SSE, NEON)
    Simd128,
    /// 256-bit SIMD (AVX2)
    Simd256,
    /// 512-bit SIMD (AVX-512, SVE)
    Simd512,
}

impl SimdType {
    /// Get width in bytes
    #[inline]
    pub fn width(&self) -> usize {
        match self {
            Self::None => 1,
            Self::Simd128 => 16,
            Self::Simd256 => 32,
            Self::Simd512 => 64,
        }
    }

    /// Get width in f64 elements
    #[inline(always)]
    pub fn f64_lanes(&self) -> usize {
        self.width() / 8
    }

    /// Get width in u64 elements
    #[inline(always)]
    pub fn u64_lanes(&self) -> usize {
        self.width() / 8
    }

    /// Detect best available SIMD
    pub fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            // Would use CPUID in real kernel
            Self::Simd128 // SSE2 is baseline for x86_64
        }
        #[cfg(target_arch = "aarch64")]
        {
            Self::Simd128 // NEON is mandatory on AArch64
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            Self::None
        }
    }
}
