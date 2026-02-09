// SPDX-License-Identifier: GPL-2.0
//! # NEXUS Fast Primitives
//!
//! Ultra-high-performance data structures and functions designed for kernel
//! hot paths. Every operation in this module targets **nanosecond** latency.
//!
//! ## Key Primitives
//!
//! - [`RingBuffer`] — O(1) fixed-capacity circular buffer (replaces Vec::remove(0))
//! - [`FastEma`] — Cache-line aligned EMA with #[inline(always)]
//! - [`FlatMap`] — O(1) array-backed map for small key ranges
//! - [`FastHasher`] — Stack-allocated FNV-1a without heap allocation
//! - [`ArrayMap`] — O(1) fixed-size array indexed by usize (replaces BTreeMap<u32, _>)
//! - [`LinearMap`] — Cache-friendly small map for <128 entries (replaces BTreeMap<u64, _>)
//! - [`InlineStr`] — Stack-allocated 64-byte string (replaces String clones)
//! - [`SlabPool`] — O(1) fixed-size slab allocator (replaces Vec<Option<T>>)
//! - [`BitSet`] — Compact boolean array with hardware popcnt/ctz
//!
//! ## Performance Guarantees
//!
//! | Primitive    | Insert | Lookup | Remove | Memory |
//! |-------------|--------|--------|--------|--------|
//! | RingBuffer  | O(1)   | O(1)   | O(1)   | Stack  |
//! | FastEma     | O(1)   | O(1)   | N/A    | Stack  |
//! | FlatMap     | O(1)   | O(1)   | O(1)   | Stack  |
//! | FastHasher  | O(n)   | N/A    | N/A    | Stack  |
//! | ArrayMap    | O(1)   | O(1)   | O(1)   | Stack  |
//! | LinearMap   | O(n)   | O(n)   | O(1)   | Stack  |
//! | InlineStr   | O(n)   | O(1)   | N/A    | Stack  |
//! | SlabPool    | O(1)   | O(1)   | O(1)   | Stack  |
//! | BitSet      | O(1)   | O(1)   | O(1)   | Stack  |

pub mod ring_buffer;
pub mod fast_ema;
pub mod flat_map;
pub mod fast_hash;
pub mod array_map;
pub mod linear_map;
pub mod inline_str;
pub mod slab_pool;
pub mod bitset;
