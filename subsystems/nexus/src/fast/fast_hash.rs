// SPDX-License-Identifier: GPL-2.0
//! # Stack-Allocated Fast Hasher — Zero Heap Allocation
//!
//! FNV-1a hashing without `format!()` or `String`. All operations work
//! directly on bytes, avoiding heap allocation entirely.
//!
//! ## Performance Comparison
//!
//! | Method                    | Heap Alloc | Latency |
//! |--------------------------|-----------|---------|
//! | `format!("{}-{}", a, b)` | YES (~100ns) | ~150-300ns |
//! | `fnv1a_hash(bytes)`      | NO           | ~5-20ns    |
//! | `FastHasher::new().u64().u64().finish()` | NO | ~10-30ns |
//!
//! The `format!()` approach allocates a `String` on the heap just to hash it
//! and immediately discard it. This wastes ~100-300ns per call. The FastHasher
//! feeds bytes directly into FNV-1a with zero allocation.

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

/// Stack-allocated incremental FNV-1a hasher.
///
/// Feed multiple values without heap allocation:
/// ```rust,ignore
/// // BEFORE (heap allocation):
/// let key = format!("{}-{}-{}", module_id, event_type, timestamp);
/// let hash = fnv1a_hash(key.as_bytes());
///
/// // AFTER (zero allocation):
/// let hash = FastHasher::new()
///     .feed_u64(module_id)
///     .feed_u32(event_type)
///     .feed_u64(timestamp)
///     .finish();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct FastHasher {
    state: u64,
}

impl FastHasher {
    /// Create a new hasher with FNV-1a offset basis. **O(1)**.
    #[inline(always)]
    pub const fn new() -> Self {
        Self { state: FNV_OFFSET }
    }

    /// Create a hasher seeded with a custom value. **O(1)**.
    #[inline(always)]
    pub const fn with_seed(seed: u64) -> Self {
        Self {
            state: FNV_OFFSET ^ seed,
        }
    }

    /// Feed a single byte. **O(1)**.
    #[inline(always)]
    pub fn feed_byte(mut self, byte: u8) -> Self {
        self.state ^= byte as u64;
        self.state = self.state.wrapping_mul(FNV_PRIME);
        self
    }

    /// Feed a u16. **O(1)**.
    #[inline(always)]
    pub fn feed_u16(self, value: u16) -> Self {
        let bytes = value.to_le_bytes();
        self.feed_byte(bytes[0]).feed_byte(bytes[1])
    }

    /// Feed a u32. **O(1)**.
    #[inline(always)]
    pub fn feed_u32(self, value: u32) -> Self {
        let bytes = value.to_le_bytes();
        self.feed_byte(bytes[0])
            .feed_byte(bytes[1])
            .feed_byte(bytes[2])
            .feed_byte(bytes[3])
    }

    /// Feed a u64. **O(1)**.
    #[inline(always)]
    pub fn feed_u64(self, value: u64) -> Self {
        let bytes = value.to_le_bytes();
        self.feed_byte(bytes[0])
            .feed_byte(bytes[1])
            .feed_byte(bytes[2])
            .feed_byte(bytes[3])
            .feed_byte(bytes[4])
            .feed_byte(bytes[5])
            .feed_byte(bytes[6])
            .feed_byte(bytes[7])
    }

    /// Feed a usize. **O(1)**.
    #[inline(always)]
    pub fn feed_usize(self, value: usize) -> Self {
        self.feed_u64(value as u64)
    }

    /// Feed a byte slice. **O(n)** where n = slice length.
    #[inline]
    pub fn feed_bytes(mut self, data: &[u8]) -> Self {
        for &byte in data {
            self.state ^= byte as u64;
            self.state = self.state.wrapping_mul(FNV_PRIME);
        }
        self
    }

    /// Feed a str without allocation. **O(n)**.
    #[inline]
    pub fn feed_str(self, s: &str) -> Self {
        self.feed_bytes(s.as_bytes())
    }

    /// Feed a bool. **O(1)**.
    #[inline(always)]
    pub fn feed_bool(self, value: bool) -> Self {
        self.feed_byte(value as u8)
    }

    /// Feed an f32 (bit pattern). **O(1)**.
    #[inline(always)]
    pub fn feed_f32(self, value: f32) -> Self {
        self.feed_u32(value.to_bits())
    }

    /// Finalize and return the hash. **O(1)**.
    #[inline(always)]
    pub const fn finish(self) -> u64 {
        self.state
    }

    /// Finalize as a usize (for use as array index). **O(1)**.
    #[inline(always)]
    pub const fn finish_usize(self) -> usize {
        self.state as usize
    }

    /// Finalize modulo N (for direct array indexing). **O(1)**.
    #[inline(always)]
    pub const fn finish_mod(self, n: usize) -> usize {
        (self.state as usize) % n
    }
}

/// Standalone FNV-1a hash function for byte slices. **O(n)**.
///
/// Use this when you have a single byte slice to hash.
/// For combining multiple values, use [`FastHasher`] instead.
#[inline]
pub fn fnv1a(data: &[u8]) -> u64 {
    FastHasher::new().feed_bytes(data).finish()
}

/// Hash two u64 values together without allocation. **O(1)**.
///
/// Replaces: `FastHasher::new().feed_u64(a as u64).feed_str("-").feed_u64(b as u64).finish()`
#[inline(always)]
pub fn hash_pair(a: u64, b: u64) -> u64 {
    FastHasher::new().feed_u64(a).feed_u64(b).finish()
}

/// Hash three u64 values together without allocation. **O(1)**.
#[inline(always)]
pub fn hash_triple(a: u64, b: u64, c: u64) -> u64 {
    FastHasher::new()
        .feed_u64(a)
        .feed_u64(b)
        .feed_u64(c)
        .finish()
}

/// Xorshift64 PRNG — deterministic, ~1ns per call.
///
/// Use for jitter, noise injection, random selection.
/// NOT cryptographically secure.
#[inline(always)]
pub fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Xorshift64 returning f32 in [0.0, 1.0). **O(1), ~2ns**.
#[inline(always)]
pub fn xorshift64_f32(state: &mut u64) -> f32 {
    let x = xorshift64(state);
    (x >> 40) as f32 / (1u64 << 24) as f32
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hasher_deterministic() {
        let h1 = FastHasher::new().feed_u64(42).feed_u32(7).finish();
        let h2 = FastHasher::new().feed_u64(42).feed_u32(7).finish();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hasher_different_inputs() {
        let h1 = FastHasher::new().feed_u64(1).finish();
        let h2 = FastHasher::new().feed_u64(2).finish();
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_fnv1a_matches_manual() {
        let data = b"hello";
        let h1 = fnv1a(data);
        let h2 = FastHasher::new().feed_bytes(data).finish();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_pair_no_alloc() {
        // This should produce the same hash as feeding two u64s
        let h1 = hash_pair(100, 200);
        let h2 = FastHasher::new().feed_u64(100).feed_u64(200).finish();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_xorshift_range() {
        let mut state = 12345u64;
        for _ in 0..1000 {
            let f = xorshift64_f32(&mut state);
            assert!(f >= 0.0 && f < 1.0);
        }
    }

    #[test]
    fn test_finish_mod() {
        let idx = FastHasher::new().feed_u64(42).finish_mod(256);
        assert!(idx < 256);
    }
}
