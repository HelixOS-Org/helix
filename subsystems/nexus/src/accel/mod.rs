//! # Architecture Accelerators
//!
//! Architecture-specific hardware acceleration.
//!
//! ## Key Features
//!
//! - **SIMD Operations**: Vector processing acceleration
//! - **Cryptographic Acceleration**: Hardware crypto
//! - **Memory Operations**: Optimized memory operations
//! - **Atomic Operations**: Lock-free primitives

#![allow(dead_code)]

extern crate alloc;

mod atomic;
mod crypto;
mod registry;
mod simd;
mod vector;

// Re-export SIMD types
// Re-export atomic ops
pub use atomic::{AtomicOps, Prefetch};
// Re-export crypto
pub use crypto::CryptoAccel;
// Re-export registry
pub use registry::{AcceleratorCapabilities, AcceleratorRegistry};
pub use simd::SimdType;
// Re-export vector ops
pub use vector::VectorOps;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_type() {
        let simd = SimdType::detect();
        assert!(simd.width() >= 1);
    }

    #[test]
    fn test_vector_ops() {
        let ops = VectorOps::new();

        let mut dst = [0u8; 16];
        ops.memset(&mut dst, 0xFF);
        assert!(dst.iter().all(|&b| b == 0xFF));

        let src = [1u8, 2, 3, 4];
        let mut dst = [0u8; 4];
        ops.memcpy(&mut dst, &src);
        assert_eq!(dst, src);
    }

    #[test]
    fn test_sum() {
        let ops = VectorOps::new();

        let data = [1u64, 2, 3, 4, 5];
        assert_eq!(ops.sum_u64(&data), 15);

        let data = [1.0f64, 2.0, 3.0, 4.0, 5.0];
        assert!((ops.sum_f64(&data) - 15.0).abs() < 0.001);
    }

    #[test]
    fn test_dot_product() {
        let ops = VectorOps::new();

        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];
        // 1*4 + 2*5 + 3*6 = 32
        assert!((ops.dot_product_f64(&a, &b) - 32.0).abs() < 0.001);
    }

    #[test]
    fn test_crypto() {
        let crypto = CryptoAccel::new();

        let data = b"hello world";
        let crc = crypto.crc32c(data, 0);
        assert!(crc != 0);

        let hash = crypto.fnv1a(data);
        assert!(hash != 0);

        let xxh = crypto.xxhash(data, 0);
        assert!(xxh != 0);
    }

    #[test]
    fn test_registry() {
        let registry = AcceleratorRegistry::new();
        let caps = registry.capabilities();

        assert!(caps.simd_type.width() >= 1);
    }
}
