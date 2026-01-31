//! Cryptographic accelerator.

use core::sync::atomic::{AtomicU64, Ordering};

/// Cryptographic accelerator
pub struct CryptoAccel {
    /// Has AES-NI (x86) or similar
    has_aes: bool,
    /// Has SHA extensions
    has_sha: bool,
    /// Has CRC32C acceleration
    has_crc32: bool,
    /// Operations counter
    ops_count: AtomicU64,
}

impl CryptoAccel {
    /// Create new crypto accelerator
    pub fn new() -> Self {
        Self {
            has_aes: Self::detect_aes(),
            has_sha: Self::detect_sha(),
            has_crc32: Self::detect_crc32(),
            ops_count: AtomicU64::new(0),
        }
    }

    fn detect_aes() -> bool {
        // Would use CPUID/feature registers in real kernel
        cfg!(target_arch = "x86_64") || cfg!(target_arch = "aarch64")
    }

    fn detect_sha() -> bool {
        cfg!(target_arch = "x86_64") || cfg!(target_arch = "aarch64")
    }

    fn detect_crc32() -> bool {
        cfg!(target_arch = "x86_64") || cfg!(target_arch = "aarch64")
    }

    /// Has AES acceleration?
    pub fn has_aes(&self) -> bool {
        self.has_aes
    }

    /// Has SHA acceleration?
    pub fn has_sha(&self) -> bool {
        self.has_sha
    }

    /// Has CRC32 acceleration?
    pub fn has_crc32(&self) -> bool {
        self.has_crc32
    }

    /// Fast CRC32C
    pub fn crc32c(&self, data: &[u8], initial: u32) -> u32 {
        self.ops_count.fetch_add(1, Ordering::Relaxed);

        // Software fallback (real impl would use hardware)
        let mut crc = !initial;
        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0x82F63B78
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    /// Fast FNV-1a hash
    pub fn fnv1a(&self, data: &[u8]) -> u64 {
        self.ops_count.fetch_add(1, Ordering::Relaxed);

        let mut hash = 0xcbf29ce484222325u64;
        for &byte in data {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// Fast xxHash-like
    pub fn xxhash(&self, data: &[u8], seed: u64) -> u64 {
        self.ops_count.fetch_add(1, Ordering::Relaxed);

        // Simplified xxHash-like algorithm
        const PRIME1: u64 = 11400714785074694791;
        const PRIME2: u64 = 14029467366897019727;
        const PRIME3: u64 = 1609587929392839161;

        let mut acc = seed.wrapping_add(PRIME3);

        for chunk in data.chunks(8) {
            let mut val = 0u64;
            for (i, &byte) in chunk.iter().enumerate() {
                val |= (byte as u64) << (i * 8);
            }
            acc = acc.wrapping_add(val.wrapping_mul(PRIME2));
            acc = acc.rotate_left(31);
            acc = acc.wrapping_mul(PRIME1);
        }

        acc ^= acc >> 33;
        acc = acc.wrapping_mul(PRIME2);
        acc ^= acc >> 29;
        acc = acc.wrapping_mul(PRIME3);
        acc ^= acc >> 32;

        acc
    }

    /// Get operations count
    pub fn ops_count(&self) -> u64 {
        self.ops_count.load(Ordering::Relaxed)
    }
}

impl Default for CryptoAccel {
    fn default() -> Self {
        Self::new()
    }
}
