//! Vector operations accelerator.

use core::sync::atomic::{AtomicU64, Ordering};

use super::simd::SimdType;

/// Vector operations accelerator
pub struct VectorOps {
    /// SIMD type available
    simd: SimdType,
    /// Operations counter
    ops_count: AtomicU64,
}

impl VectorOps {
    /// Create new vector ops
    pub fn new() -> Self {
        Self {
            simd: SimdType::detect(),
            ops_count: AtomicU64::new(0),
        }
    }

    /// Get SIMD type
    #[inline(always)]
    pub fn simd_type(&self) -> SimdType {
        self.simd
    }

    /// Vectorized memset
    #[inline]
    pub fn memset(&self, dst: &mut [u8], value: u8) {
        self.ops_count.fetch_add(1, Ordering::Relaxed);

        // In a real implementation, this would use SIMD instructions
        // For now, use standard fill
        dst.fill(value);
    }

    /// Vectorized memcpy
    #[inline]
    pub fn memcpy(&self, dst: &mut [u8], src: &[u8]) {
        self.ops_count.fetch_add(1, Ordering::Relaxed);

        let len = dst.len().min(src.len());
        dst[..len].copy_from_slice(&src[..len]);
    }

    /// Vectorized memcmp
    #[inline]
    pub fn memcmp(&self, a: &[u8], b: &[u8]) -> core::cmp::Ordering {
        self.ops_count.fetch_add(1, Ordering::Relaxed);

        let len = a.len().min(b.len());
        match a[..len].cmp(&b[..len]) {
            core::cmp::Ordering::Equal => a.len().cmp(&b.len()),
            other => other,
        }
    }

    /// Vectorized sum of u64
    #[inline]
    pub fn sum_u64(&self, data: &[u64]) -> u64 {
        self.ops_count.fetch_add(1, Ordering::Relaxed);

        // In real implementation, use SIMD horizontal add
        data.iter().sum()
    }

    /// Vectorized sum of f64
    #[inline]
    pub fn sum_f64(&self, data: &[f64]) -> f64 {
        self.ops_count.fetch_add(1, Ordering::Relaxed);

        data.iter().sum()
    }

    /// Vectorized min of u64
    #[inline]
    pub fn min_u64(&self, data: &[u64]) -> Option<u64> {
        self.ops_count.fetch_add(1, Ordering::Relaxed);

        data.iter().copied().min()
    }

    /// Vectorized max of u64
    #[inline]
    pub fn max_u64(&self, data: &[u64]) -> Option<u64> {
        self.ops_count.fetch_add(1, Ordering::Relaxed);

        data.iter().copied().max()
    }

    /// Vectorized dot product
    #[inline]
    pub fn dot_product_f64(&self, a: &[f64], b: &[f64]) -> f64 {
        self.ops_count.fetch_add(1, Ordering::Relaxed);

        let len = a.len().min(b.len());
        a[..len]
            .iter()
            .zip(b[..len].iter())
            .map(|(x, y)| x * y)
            .sum()
    }

    /// Vectorized XOR
    #[inline]
    pub fn xor(&self, dst: &mut [u8], src: &[u8]) {
        self.ops_count.fetch_add(1, Ordering::Relaxed);

        let len = dst.len().min(src.len());
        for i in 0..len {
            dst[i] ^= src[i];
        }
    }

    /// Find first non-zero byte
    #[inline]
    pub fn find_nonzero(&self, data: &[u8]) -> Option<usize> {
        self.ops_count.fetch_add(1, Ordering::Relaxed);

        // In real implementation, use SIMD to check multiple bytes at once
        data.iter().position(|&b| b != 0)
    }

    /// Count zero bytes
    #[inline]
    pub fn count_zero(&self, data: &[u8]) -> usize {
        self.ops_count.fetch_add(1, Ordering::Relaxed);

        data.iter().filter(|&&b| b == 0).count()
    }

    /// Get operations count
    #[inline(always)]
    pub fn ops_count(&self) -> u64 {
        self.ops_count.load(Ordering::Relaxed)
    }
}

impl Default for VectorOps {
    fn default() -> Self {
        Self::new()
    }
}
