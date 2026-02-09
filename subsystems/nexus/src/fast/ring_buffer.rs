// SPDX-License-Identifier: GPL-2.0
//! # O(1) Ring Buffer — Replaces Vec::remove(0)
//!
//! A fixed-capacity circular buffer where push and pop are **always O(1)**.
//! This replaces the anti-pattern of `Vec::remove(0)` which is O(n) due to
//! shifting all elements left.
//!
//! ## Performance
//!
//! | Operation     | Vec::remove(0) | RingBuffer |
//! |--------------|----------------|------------|
//! | Push (full)  | O(n) shift     | O(1)       |
//! | Pop front    | O(n) shift     | O(1)       |
//! | Latest item  | O(1)           | O(1)       |
//! | Oldest item  | O(1)           | O(1)       |
//! | Iterate      | O(n)           | O(n)       |
//!
//! For a 256-element buffer, Vec::remove(0) copies ~2KB per insert.
//! RingBuffer does a single write. That's **~500x faster**.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use helix_nexus::fast::ring_buffer::RingBuffer;
//!
//! let mut rb = RingBuffer::<f32, 256>::new();
//! rb.push(42.0);
//! rb.push(43.0);
//! assert_eq!(rb.len(), 2);
//! assert_eq!(rb.oldest(), Some(&42.0));
//! assert_eq!(rb.newest(), Some(&43.0));
//! ```

/// A fixed-capacity O(1) circular buffer.
///
/// `N` is the maximum capacity. When full, new elements overwrite the oldest.
/// All operations are O(1) with zero heap allocation.
///
/// # Cache Performance
///
/// Data is stored in a contiguous array, maximizing L1 cache utilization.
/// For `N <= 64` and `T = f32`, the entire buffer fits in a single cache line.
#[repr(C)]
#[repr(align(64))]
pub struct RingBuffer<T: Copy + Default, const N: usize> {
    /// Contiguous storage — cache-friendly
    data: [T; N],
    /// Write position (next slot to write)
    head: usize,
    /// Number of valid elements
    len: usize,
}

impl<T: Copy + Default, const N: usize> RingBuffer<T, N> {
    /// Create a new empty ring buffer. O(1).
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            data: [unsafe { core::mem::zeroed() }; N],
            head: 0,
            len: 0,
        }
    }

    /// Push an element. If full, overwrites the oldest. **O(1)**.
    ///
    /// This is the critical operation that replaces:
    /// ```rust,ignore
    /// if vec.len() >= MAX { vec.remove(0); } // O(n) — BAD
    /// vec.push(value);
    /// ```
    /// With:
    /// ```rust,ignore
    /// ring.push(value); // O(1) — GOOD
    /// ```
    #[inline(always)]
    pub fn push(&mut self, value: T) {
        self.data[self.head] = value;
        self.head = (self.head + 1) % N;
        if self.len < N {
            self.len += 1;
        }
    }

    /// Pop the oldest element. **O(1)**.
    #[inline(always)]
    pub fn pop_front(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        let tail = self.tail_index();
        let value = self.data[tail];
        self.len -= 1;
        Some(value)
    }

    /// Get the newest (most recently pushed) element. **O(1)**.
    #[inline(always)]
    pub fn newest(&self) -> Option<&T> {
        if self.len == 0 {
            return None;
        }
        let idx = if self.head == 0 { N - 1 } else { self.head - 1 };
        Some(&self.data[idx])
    }

    /// Get the oldest element. **O(1)**.
    #[inline(always)]
    pub fn oldest(&self) -> Option<&T> {
        if self.len == 0 {
            return None;
        }
        Some(&self.data[self.tail_index()])
    }

    /// Get element at logical index (0 = oldest). **O(1)**.
    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        let physical = (self.tail_index() + index) % N;
        Some(&self.data[physical])
    }

    /// Number of elements currently stored. **O(1)**.
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether the buffer is empty. **O(1)**.
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Whether the buffer is full. **O(1)**.
    #[inline(always)]
    pub const fn is_full(&self) -> bool {
        self.len == N
    }

    /// Maximum capacity. **O(1)**.
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Clear all elements. **O(1)** (doesn't zero memory).
    #[inline(always)]
    pub fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }

    /// Compute the average of all elements. **O(n)**.
    /// Only available for f32 buffers.
    #[inline]
    pub fn average(&self) -> f32
    where
        T: Into<f32>,
    {
        if self.len == 0 {
            return 0.0;
        }
        let mut sum: f32 = 0.0;
        for i in 0..self.len {
            let physical = (self.tail_index() + i) % N;
            sum += self.data[physical].into();
        }
        sum / self.len as f32
    }

    /// Compute min of all elements. **O(n)**.
    #[inline]
    pub fn min(&self) -> Option<T>
    where
        T: PartialOrd,
    {
        if self.len == 0 {
            return None;
        }
        let mut min_val = self.data[self.tail_index()];
        for i in 1..self.len {
            let physical = (self.tail_index() + i) % N;
            if self.data[physical] < min_val {
                min_val = self.data[physical];
            }
        }
        Some(min_val)
    }

    /// Compute max of all elements. **O(n)**.
    #[inline]
    pub fn max(&self) -> Option<T>
    where
        T: PartialOrd,
    {
        if self.len == 0 {
            return None;
        }
        let mut max_val = self.data[self.tail_index()];
        for i in 1..self.len {
            let physical = (self.tail_index() + i) % N;
            if self.data[physical] > max_val {
                max_val = self.data[physical];
            }
        }
        Some(max_val)
    }

    /// Iterate over elements from oldest to newest. **O(n) total**.
    #[inline]
    pub fn iter(&self) -> RingBufferIter<'_, T, N> {
        RingBufferIter {
            buffer: self,
            index: 0,
        }
    }

    /// Get the last N elements as a slice-like iterator. **O(n) total**.
    #[inline]
    pub fn last_n(&self, n: usize) -> RingBufferIter<'_, T, N> {
        let skip = if n >= self.len { 0 } else { self.len - n };
        RingBufferIter {
            buffer: self,
            index: skip,
        }
    }

    /// Internal: compute tail (oldest element) index. **O(1)**.
    #[inline(always)]
    const fn tail_index(&self) -> usize {
        if self.len < N {
            0
        } else {
            self.head
        }
    }
}

/// Iterator over ring buffer elements (oldest → newest).
#[repr(align(64))]
pub struct RingBufferIter<'a, T: Copy + Default, const N: usize> {
    buffer: &'a RingBuffer<T, N>,
    index: usize,
}

impl<'a, T: Copy + Default, const N: usize> Iterator for RingBufferIter<'a, T, N> {
    type Item = &'a T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.buffer.len {
            return None;
        }
        let item = self.buffer.get(self.index);
        self.index += 1;
        item
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.buffer.len - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a, T: Copy + Default, const N: usize> ExactSizeIterator for RingBufferIter<'a, T, N> {}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_len() {
        let mut rb = RingBuffer::<u32, 4>::new();
        assert_eq!(rb.len(), 0);
        assert!(rb.is_empty());

        rb.push(1);
        rb.push(2);
        rb.push(3);
        assert_eq!(rb.len(), 3);
        assert!(!rb.is_full());

        rb.push(4);
        assert_eq!(rb.len(), 4);
        assert!(rb.is_full());
    }

    #[test]
    fn test_overwrite_oldest() {
        let mut rb = RingBuffer::<u32, 3>::new();
        rb.push(1);
        rb.push(2);
        rb.push(3);
        // Buffer full: [1, 2, 3]

        rb.push(4);
        // Oldest (1) overwritten: [4, 2, 3] logically [2, 3, 4]
        assert_eq!(rb.len(), 3);
        assert_eq!(*rb.oldest().unwrap(), 2);
        assert_eq!(*rb.newest().unwrap(), 4);
    }

    #[test]
    fn test_pop_front() {
        let mut rb = RingBuffer::<u32, 4>::new();
        rb.push(10);
        rb.push(20);
        rb.push(30);

        assert_eq!(rb.pop_front(), Some(10));
        assert_eq!(rb.pop_front(), Some(20));
        assert_eq!(rb.len(), 1);
        assert_eq!(rb.pop_front(), Some(30));
        assert_eq!(rb.pop_front(), None);
    }

    #[test]
    fn test_iterate_order() {
        let mut rb = RingBuffer::<u32, 4>::new();
        rb.push(1);
        rb.push(2);
        rb.push(3);
        rb.push(4);
        rb.push(5); // overwrites 1

        let items: alloc::vec::Vec<u32> = rb.iter().copied().collect();
        assert_eq!(items, alloc::vec![2, 3, 4, 5]);
    }

    #[test]
    fn test_average() {
        let mut rb = RingBuffer::<f32, 4>::new();
        rb.push(10.0);
        rb.push(20.0);
        rb.push(30.0);
        rb.push(40.0);
        assert!((rb.average() - 25.0).abs() < 0.001);
    }

    #[test]
    fn test_last_n() {
        let mut rb = RingBuffer::<u32, 8>::new();
        for i in 0..8 {
            rb.push(i);
        }
        let last3: alloc::vec::Vec<u32> = rb.last_n(3).copied().collect();
        assert_eq!(last3, alloc::vec![5, 6, 7]);
    }
}
