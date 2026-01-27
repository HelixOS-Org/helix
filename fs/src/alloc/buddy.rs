//! Buddy allocator for contiguous block allocation.
//!
//! Uses power-of-2 sized chunks for efficient coalescing.
//! Good for large contiguous allocations.

use crate::core::types::*;
use crate::core::error::{HfsError, HfsResult};
use core::sync::atomic::{AtomicU64, AtomicU32, Ordering};

// ============================================================================
// Constants
// ============================================================================

/// Maximum order (2^MAX_ORDER blocks per allocation)
pub const MAX_ORDER: usize = 20; // 2^20 = 1M blocks max

/// Minimum order
pub const MIN_ORDER: usize = 0; // 1 block minimum

/// Number of orders
pub const NUM_ORDERS: usize = MAX_ORDER + 1;

// ============================================================================
// Buddy Block
// ============================================================================

/// Free list entry for a buddy block.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BuddyBlock {
    /// Block number
    pub block: u64,
    /// Next free block in list (0 = end)
    pub next: u64,
    /// Previous free block in list (0 = none)
    pub prev: u64,
}

impl BuddyBlock {
    /// Create new block entry
    pub const fn new(block: u64) -> Self {
        Self {
            block,
            next: 0,
            prev: 0,
        }
    }
    
    /// Check if end of list
    #[inline]
    pub fn is_end(&self) -> bool {
        self.next == 0
    }
}

// ============================================================================
// Free List Head
// ============================================================================

/// Atomic free list head for buddy allocator.
#[derive(Default)]
pub struct FreeListHead {
    /// First block in list (0 = empty)
    head: AtomicU64,
    /// Number of entries
    count: AtomicU32,
}

impl FreeListHead {
    /// Create new empty list
    pub const fn new() -> Self {
        Self {
            head: AtomicU64::new(0),
            count: AtomicU32::new(0),
        }
    }
    
    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Relaxed) == 0
    }
    
    /// Get count
    #[inline]
    pub fn count(&self) -> u32 {
        self.count.load(Ordering::Relaxed)
    }
    
    /// Get head block
    #[inline]
    pub fn head(&self) -> u64 {
        self.head.load(Ordering::Relaxed)
    }
    
    /// Push block to front (requires external locking)
    pub fn push_front(&self, block: u64) {
        self.head.store(block, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Pop block from front (requires external locking)
    pub fn pop_front(&self, new_head: u64) -> Option<u64> {
        let old = self.head.swap(new_head, Ordering::Relaxed);
        if old != 0 {
            self.count.fetch_sub(1, Ordering::Relaxed);
            Some(old)
        } else {
            None
        }
    }
    
    /// Update head
    pub fn set_head(&self, head: u64) {
        self.head.store(head, Ordering::Relaxed);
    }
    
    /// Decrement count
    pub fn dec_count(&self) {
        self.count.fetch_sub(1, Ordering::Relaxed);
    }
}

// ============================================================================
// Buddy Allocator State
// ============================================================================

/// Block state in buddy allocator
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum BuddyState {
    /// Block is free
    Free = 0,
    /// Block is split (children exist)
    Split = 1,
    /// Block is allocated
    Allocated = 2,
}

/// Order statistics
#[derive(Clone, Copy, Debug, Default)]
pub struct OrderStats {
    /// Free blocks at this order
    pub free_count: u32,
    /// Allocated blocks at this order
    pub alloc_count: u32,
    /// Total allocations from this order
    pub total_allocs: u64,
    /// Total frees to this order
    pub total_frees: u64,
}

/// Buddy allocator state and free lists.
pub struct BuddyAllocState {
    /// Free lists for each order
    free_lists: [FreeListHead; NUM_ORDERS],
    /// Total blocks managed
    total_blocks: u64,
    /// Free blocks
    free_blocks: AtomicU64,
    /// Statistics per order
    order_stats: [OrderStats; NUM_ORDERS],
}

impl BuddyAllocState {
    /// Create new buddy allocator state
    pub fn new(total_blocks: u64) -> Self {
        const EMPTY_LIST: FreeListHead = FreeListHead::new();
        const EMPTY_STATS: OrderStats = OrderStats {
            free_count: 0,
            alloc_count: 0,
            total_allocs: 0,
            total_frees: 0,
        };
        
        Self {
            free_lists: [EMPTY_LIST; NUM_ORDERS],
            total_blocks,
            free_blocks: AtomicU64::new(0),
            order_stats: [EMPTY_STATS; NUM_ORDERS],
        }
    }
    
    /// Get free list for order
    #[inline]
    pub fn free_list(&self, order: usize) -> &FreeListHead {
        &self.free_lists[order]
    }
    
    /// Get total blocks
    #[inline]
    pub fn total_blocks(&self) -> u64 {
        self.total_blocks
    }
    
    /// Get free blocks
    #[inline]
    pub fn free_blocks(&self) -> u64 {
        self.free_blocks.load(Ordering::Relaxed)
    }
    
    /// Add to free blocks
    pub fn add_free(&self, count: u64) {
        self.free_blocks.fetch_add(count, Ordering::Relaxed);
    }
    
    /// Remove from free blocks
    pub fn remove_free(&self, count: u64) {
        self.free_blocks.fetch_sub(count, Ordering::Relaxed);
    }
}

// ============================================================================
// Buddy Utilities
// ============================================================================

/// Get size in blocks for order.
#[inline]
pub const fn order_size(order: usize) -> u64 {
    1u64 << order
}

/// Get order needed for block count (rounded up).
#[inline]
pub fn size_to_order(blocks: u64) -> usize {
    if blocks == 0 {
        return 0;
    }
    if blocks == 1 {
        return 0;
    }
    
    // Find ceiling log2
    let leading_zeros = (blocks - 1).leading_zeros();
    (64 - leading_zeros) as usize
}

/// Get buddy block number.
///
/// For a block at address A with order O, buddy is at A ^ (1 << O).
#[inline]
pub const fn buddy_of(block: u64, order: usize) -> u64 {
    block ^ (1u64 << order)
}

/// Check if block is properly aligned for order.
#[inline]
pub const fn is_aligned(block: u64, order: usize) -> bool {
    (block & ((1u64 << order) - 1)) == 0
}

/// Get parent block (for coalescing).
#[inline]
pub const fn parent_of(block: u64, order: usize) -> u64 {
    block & !(1u64 << order)
}

/// Check if block is left child of parent.
#[inline]
pub const fn is_left_child(block: u64, order: usize) -> bool {
    (block & (1u64 << order)) == 0
}

// ============================================================================
// Buddy Bitmap (tracks state of buddy pairs)
// ============================================================================

/// Bitmap tracking buddy pair states.
///
/// Uses 2 bits per pair:
/// - 00: Both free (can coalesce)
/// - 01: Left allocated, right free
/// - 10: Left free, right allocated
/// - 11: Both allocated (or parent is allocated)
pub struct BuddyBitmap {
    /// Bitmaps for each order level
    /// order_bitmaps[o] has bits for blocks at order o
    order_bitmaps: [*mut u64; NUM_ORDERS],
    /// Number of u64 words per order
    words_per_order: [usize; NUM_ORDERS],
    /// Total blocks covered
    total_blocks: u64,
}

impl BuddyBitmap {
    /// Calculate words needed for order
    #[inline]
    fn words_for_order(total_blocks: u64, order: usize) -> usize {
        let pairs = total_blocks / (1 << (order + 1));
        let bits = pairs * 2; // 2 bits per pair
        ((bits + 63) / 64) as usize
    }
    
    /// Get state of block at order
    pub fn get_state(&self, block: u64, order: usize) -> BuddyState {
        if order >= NUM_ORDERS {
            return BuddyState::Allocated;
        }
        
        // Calculate bit position
        let pair_idx = block / (1 << (order + 1));
        let is_right = (block / (1 << order)) % 2 == 1;
        
        let word_idx = (pair_idx / 32) as usize;
        let bit_idx = ((pair_idx % 32) * 2) as u32;
        
        if word_idx >= self.words_per_order[order] {
            return BuddyState::Allocated;
        }
        
        // SAFETY: We've checked bounds
        let word = unsafe { *self.order_bitmaps[order].add(word_idx) };
        let pair_bits = (word >> bit_idx) & 0b11;
        
        match (pair_bits, is_right) {
            (0b00, _) => BuddyState::Free,
            (0b01, false) => BuddyState::Allocated,
            (0b01, true) => BuddyState::Free,
            (0b10, false) => BuddyState::Free,
            (0b10, true) => BuddyState::Allocated,
            (0b11, _) => BuddyState::Allocated,
            _ => BuddyState::Allocated,
        }
    }
    
    /// Set state of block at order
    pub fn set_state(&mut self, block: u64, order: usize, state: BuddyState) {
        if order >= NUM_ORDERS {
            return;
        }
        
        let pair_idx = block / (1 << (order + 1));
        let is_right = (block / (1 << order)) % 2 == 1;
        
        let word_idx = (pair_idx / 32) as usize;
        let bit_idx = ((pair_idx % 32) * 2) as u32;
        
        if word_idx >= self.words_per_order[order] {
            return;
        }
        
        // SAFETY: We've checked bounds
        let word = unsafe { &mut *self.order_bitmaps[order].add(word_idx) };
        
        // Clear the bit for this block
        let clear_mask = if is_right { 0b10 } else { 0b01 };
        let set_mask = if is_right { 0b10 } else { 0b01 };
        
        *word &= !(clear_mask << bit_idx);
        
        if state == BuddyState::Allocated {
            *word |= set_mask << bit_idx;
        }
    }
    
    /// Check if buddy pair can coalesce
    pub fn can_coalesce(&self, block: u64, order: usize) -> bool {
        if order >= NUM_ORDERS - 1 {
            return false;
        }
        
        let pair_idx = block / (1 << (order + 1));
        let word_idx = (pair_idx / 32) as usize;
        let bit_idx = ((pair_idx % 32) * 2) as u32;
        
        if word_idx >= self.words_per_order[order] {
            return false;
        }
        
        // SAFETY: We've checked bounds
        let word = unsafe { *self.order_bitmaps[order].add(word_idx) };
        let pair_bits = (word >> bit_idx) & 0b11;
        
        // Both free = can coalesce
        pair_bits == 0b00
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_order_size() {
        assert_eq!(order_size(0), 1);
        assert_eq!(order_size(1), 2);
        assert_eq!(order_size(10), 1024);
        assert_eq!(order_size(20), 1 << 20);
    }
    
    #[test]
    fn test_size_to_order() {
        assert_eq!(size_to_order(0), 0);
        assert_eq!(size_to_order(1), 0);
        assert_eq!(size_to_order(2), 1);
        assert_eq!(size_to_order(3), 2);
        assert_eq!(size_to_order(4), 2);
        assert_eq!(size_to_order(5), 3);
        assert_eq!(size_to_order(1024), 10);
        assert_eq!(size_to_order(1025), 11);
    }
    
    #[test]
    fn test_buddy_of() {
        // Order 0: buddies are adjacent
        assert_eq!(buddy_of(0, 0), 1);
        assert_eq!(buddy_of(1, 0), 0);
        assert_eq!(buddy_of(2, 0), 3);
        
        // Order 1: buddies are 2 apart
        assert_eq!(buddy_of(0, 1), 2);
        assert_eq!(buddy_of(2, 1), 0);
        assert_eq!(buddy_of(4, 1), 6);
        
        // Order 2: buddies are 4 apart
        assert_eq!(buddy_of(0, 2), 4);
        assert_eq!(buddy_of(4, 2), 0);
    }
    
    #[test]
    fn test_alignment() {
        assert!(is_aligned(0, 0));
        assert!(is_aligned(0, 10));
        assert!(is_aligned(1024, 10));
        assert!(!is_aligned(1, 1));
        assert!(!is_aligned(3, 2));
    }
    
    #[test]
    fn test_parent() {
        assert_eq!(parent_of(0, 0), 0);
        assert_eq!(parent_of(1, 0), 0);
        assert_eq!(parent_of(2, 1), 0);
        assert_eq!(parent_of(6, 1), 4);
    }
    
    #[test]
    fn test_free_list_head() {
        let list = FreeListHead::new();
        
        assert!(list.is_empty());
        assert_eq!(list.count(), 0);
        
        list.push_front(100);
        assert!(!list.is_empty());
        assert_eq!(list.count(), 1);
        assert_eq!(list.head(), 100);
        
        list.push_front(200);
        assert_eq!(list.count(), 2);
        assert_eq!(list.head(), 200);
    }
    
    #[test]
    fn test_buddy_state() {
        let state = BuddyAllocState::new(1024);
        
        assert_eq!(state.total_blocks(), 1024);
        assert_eq!(state.free_blocks(), 0);
        
        state.add_free(512);
        assert_eq!(state.free_blocks(), 512);
        
        state.remove_free(100);
        assert_eq!(state.free_blocks(), 412);
    }
}
