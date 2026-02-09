//! Accelerator registry.

use super::atomic::AtomicOps;
use super::crypto::CryptoAccel;
use super::simd::SimdType;
use super::vector::VectorOps;

/// Registry of all accelerators
pub struct AcceleratorRegistry {
    /// Vector operations
    pub vector: VectorOps,
    /// Crypto acceleration
    pub crypto: CryptoAccel,
    /// Atomic operations
    pub atomic: AtomicOps,
}

impl AcceleratorRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            vector: VectorOps::new(),
            crypto: CryptoAccel::new(),
            atomic: AtomicOps::new(),
        }
    }

    /// Get capabilities summary
    #[inline]
    pub fn capabilities(&self) -> AcceleratorCapabilities {
        AcceleratorCapabilities {
            simd_type: self.vector.simd_type(),
            has_aes: self.crypto.has_aes(),
            has_sha: self.crypto.has_sha(),
            has_crc32: self.crypto.has_crc32(),
            has_cmpxchg16b: self.atomic.has_cmpxchg16b(),
            has_wait: self.atomic.has_wait(),
        }
    }
}

impl Default for AcceleratorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Accelerator capabilities summary
#[derive(Debug, Clone)]
pub struct AcceleratorCapabilities {
    /// SIMD type available
    pub simd_type: SimdType,
    /// Has AES acceleration
    pub has_aes: bool,
    /// Has SHA acceleration
    pub has_sha: bool,
    /// Has CRC32C acceleration
    pub has_crc32: bool,
    /// Has 128-bit CAS
    pub has_cmpxchg16b: bool,
    /// Has wait/wake
    pub has_wait: bool,
}
