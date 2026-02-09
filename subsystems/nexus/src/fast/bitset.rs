// SPDX-License-Identifier: GPL-2.0
//! # BitSet — Compact Boolean Array
//!
//! For tracking per-CPU, per-task, or per-category boolean flags,
//! a bitset is 64× more memory-efficient than `[bool; N]` and allows
//! bulk operations (popcount, find-first-set) in single instructions.
//!
//! ## Performance
//!
//! | Operation        | [bool; N]  | BitSet      | Speedup |
//! |-----------------|-----------|-------------|---------|
//! | Set/Clear       | ~1ns      | ~1ns        | same    |
//! | Count set bits  | O(n)      | O(n/64)     | ×64     |
//! | Find first set  | O(n)      | O(n/64)     | ×64     |
//! | Memory (256)    | 256 bytes | 32 bytes    | ×8      |
//! | Memory (1024)   | 1024 B    | 128 bytes   | ×8      |

/// Fixed-size bitset stored as an array of u64 words.
///
/// - `N` = number of bits (rounded up to multiple of 64 internally).
/// - Total memory: `ceil(N/64) * 8` bytes.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BitSet<const N: usize>
where
    [(); (N + 63) / 64]: Sized,
{
    words: [u64; (N + 63) / 64],
}

impl<const N: usize> BitSet<N>
where
    [(); (N + 63) / 64]: Sized,
{
    /// Number of u64 words needed.
    const WORDS: usize = (N + 63) / 64;

    /// Create an empty bitset (all zeros).
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            words: [0u64; (N + 63) / 64],
        }
    }

    /// Set bit at position `idx`.
    #[inline(always)]
    pub fn set(&mut self, idx: usize) {
        debug_assert!(idx < N, "BitSet::set out of bounds");
        self.words[idx / 64] |= 1u64 << (idx % 64);
    }

    /// Clear bit at position `idx`.
    #[inline(always)]
    pub fn clear(&mut self, idx: usize) {
        debug_assert!(idx < N, "BitSet::clear out of bounds");
        self.words[idx / 64] &= !(1u64 << (idx % 64));
    }

    /// Toggle bit at position `idx`.
    #[inline(always)]
    pub fn toggle(&mut self, idx: usize) {
        debug_assert!(idx < N, "BitSet::toggle out of bounds");
        self.words[idx / 64] ^= 1u64 << (idx % 64);
    }

    /// Test if bit at `idx` is set.
    #[inline(always)]
    pub fn test(&self, idx: usize) -> bool {
        debug_assert!(idx < N, "BitSet::test out of bounds");
        (self.words[idx / 64] & (1u64 << (idx % 64))) != 0
    }

    /// Count the number of set bits (population count).
    /// Uses hardware `popcnt` instruction on x86.
    #[inline]
    pub fn count_ones(&self) -> usize {
        let mut total = 0usize;
        let mut i = 0;
        while i < Self::WORDS {
            total += self.words[i].count_ones() as usize;
            i += 1;
        }
        total
    }

    /// Count the number of clear bits.
    #[inline]
    pub fn count_zeros(&self) -> usize {
        N - self.count_ones()
    }

    /// Find the index of the first set bit, or `None`.
    /// Uses hardware `ctz` (count trailing zeros) for O(1) per word.
    #[inline]
    pub fn first_set(&self) -> Option<usize> {
        let mut i = 0;
        while i < Self::WORDS {
            if self.words[i] != 0 {
                let bit = self.words[i].trailing_zeros() as usize;
                let idx = i * 64 + bit;
                if idx < N {
                    return Some(idx);
                }
            }
            i += 1;
        }
        None
    }

    /// Find the index of the first clear bit, or `None`.
    #[inline]
    pub fn first_clear(&self) -> Option<usize> {
        let mut i = 0;
        while i < Self::WORDS {
            let inv = !self.words[i];
            if inv != 0 {
                let bit = inv.trailing_zeros() as usize;
                let idx = i * 64 + bit;
                if idx < N {
                    return Some(idx);
                }
            }
            i += 1;
        }
        None
    }

    /// Check if all bits are set.
    #[inline]
    pub fn all(&self) -> bool {
        self.count_ones() == N
    }

    /// Check if no bits are set.
    #[inline]
    pub fn none(&self) -> bool {
        let mut i = 0;
        while i < Self::WORDS {
            if self.words[i] != 0 {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Check if any bit is set.
    #[inline]
    pub fn any(&self) -> bool {
        !self.none()
    }

    /// Clear all bits.
    #[inline]
    pub fn clear_all(&mut self) {
        self.words = [0u64; (N + 63) / 64];
    }

    /// Set all bits.
    #[inline]
    pub fn set_all(&mut self) {
        self.words = [u64::MAX; (N + 63) / 64];
        // Mask off excess bits in last word
        let excess = N % 64;
        if excess > 0 && Self::WORDS > 0 {
            self.words[Self::WORDS - 1] = (1u64 << excess) - 1;
        }
    }

    /// Bitwise OR with another bitset (union).
    #[inline]
    pub fn union(&mut self, other: &Self) {
        let mut i = 0;
        while i < Self::WORDS {
            self.words[i] |= other.words[i];
            i += 1;
        }
    }

    /// Bitwise AND with another bitset (intersection).
    #[inline]
    pub fn intersect(&mut self, other: &Self) {
        let mut i = 0;
        while i < Self::WORDS {
            self.words[i] &= other.words[i];
            i += 1;
        }
    }

    /// Bitwise XOR with another bitset (symmetric difference).
    #[inline]
    pub fn symmetric_diff(&mut self, other: &Self) {
        let mut i = 0;
        while i < Self::WORDS {
            self.words[i] ^= other.words[i];
            i += 1;
        }
    }

    /// Total capacity in bits.
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Iterate over indices of set bits.
    #[inline]
    pub fn iter_set(&self) -> BitSetIter<'_, N> {
        BitSetIter {
            bitset: self,
            word_idx: 0,
            remaining: self.words[0],
        }
    }
}

impl<const N: usize> Default for BitSet<N>
where
    [(); (N + 63) / 64]: Sized,
{
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> core::fmt::Debug for BitSet<N>
where
    [(); (N + 63) / 64]: Sized,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BitSet")
            .field("capacity", &N)
            .field("set_bits", &self.count_ones())
            .finish()
    }
}

/// Iterator over set bit indices.
pub struct BitSetIter<'a, const N: usize>
where
    [(); (N + 63) / 64]: Sized,
{
    bitset: &'a BitSet<N>,
    word_idx: usize,
    remaining: u64,
}

impl<'a, const N: usize> Iterator for BitSetIter<'a, N>
where
    [(); (N + 63) / 64]: Sized,
{
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        loop {
            if self.remaining != 0 {
                let bit = self.remaining.trailing_zeros() as usize;
                self.remaining &= self.remaining - 1; // Clear lowest set bit
                let idx = self.word_idx * 64 + bit;
                if idx < N {
                    return Some(idx);
                }
            }
            self.word_idx += 1;
            if self.word_idx >= BitSet::<N>::WORDS {
                return None;
            }
            self.remaining = self.bitset.words[self.word_idx];
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut bs = BitSet::<64>::new();
        assert!(bs.none());
        bs.set(0);
        bs.set(63);
        assert!(bs.test(0));
        assert!(bs.test(63));
        assert!(!bs.test(1));
        assert_eq!(bs.count_ones(), 2);
    }

    #[test]
    fn test_large() {
        let mut bs = BitSet::<256>::new();
        bs.set(0);
        bs.set(127);
        bs.set(255);
        assert_eq!(bs.count_ones(), 3);
        assert_eq!(bs.first_set(), Some(0));
        bs.clear(0);
        assert_eq!(bs.first_set(), Some(127));
    }

    #[test]
    fn test_first_clear() {
        let mut bs = BitSet::<128>::new();
        bs.set_all();
        assert!(bs.all());
        bs.clear(42);
        assert_eq!(bs.first_clear(), Some(42));
    }

    #[test]
    fn test_union_intersect() {
        let mut a = BitSet::<64>::new();
        let mut b = BitSet::<64>::new();
        a.set(0);
        a.set(1);
        b.set(1);
        b.set(2);

        let mut u = a;
        u.union(&b);
        assert_eq!(u.count_ones(), 3); // 0, 1, 2

        let mut i = a;
        i.intersect(&b);
        assert_eq!(i.count_ones(), 1); // 1
        assert!(i.test(1));
    }

    #[test]
    fn test_iter_set() {
        let mut bs = BitSet::<128>::new();
        bs.set(5);
        bs.set(64);
        bs.set(100);

        let indices: alloc::vec::Vec<_> = bs.iter_set().collect();
        assert_eq!(indices, alloc::vec![5, 64, 100]);
    }

    #[test]
    fn test_size() {
        // 64 bits = 8 bytes
        assert_eq!(core::mem::size_of::<BitSet<64>>(), 8);
        // 128 bits = 16 bytes
        assert_eq!(core::mem::size_of::<BitSet<128>>(), 16);
        // 256 bits = 32 bytes
        assert_eq!(core::mem::size_of::<BitSet<256>>(), 32);
    }
}
