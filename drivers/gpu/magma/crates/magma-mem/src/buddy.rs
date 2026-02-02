//! # Buddy Allocator
//!
//! High-performance power-of-2 buddy allocator for VRAM.

use alloc::vec::Vec;
use core::cmp::min;

use magma_core::{ByteSize, Error, GpuAddr, Result};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Minimum allocation size (4KB)
pub const MIN_BLOCK_SIZE: u64 = 4 * 1024;

/// Maximum allocation size (2GB)
pub const MAX_BLOCK_SIZE: u64 = 2 * 1024 * 1024 * 1024;

/// Number of order levels (4KB to 2GB = 20 levels)
pub const MAX_ORDER: usize = 20;

// =============================================================================
// BUDDY BLOCK
// =============================================================================

/// A block in the buddy allocator
#[derive(Debug, Clone, Copy)]
pub struct BuddyBlock {
    /// Starting GPU address
    pub addr: GpuAddr,
    /// Size in bytes
    pub size: ByteSize,
    /// Order (log2(size) - log2(MIN_BLOCK_SIZE))
    pub order: u8,
    /// Is this block free?
    pub free: bool,
}

impl BuddyBlock {
    /// Create a new block
    pub const fn new(addr: GpuAddr, order: u8) -> Self {
        Self {
            addr,
            size: ByteSize::from_bytes(MIN_BLOCK_SIZE << order),
            order,
            free: true,
        }
    }

    /// Get buddy address
    pub fn buddy_addr(&self) -> GpuAddr {
        let size = self.size.as_bytes();
        GpuAddr(self.addr.0 ^ size)
    }

    /// Check if this block can be merged with its buddy
    pub fn can_merge(&self, buddy: &BuddyBlock) -> bool {
        self.order == buddy.order && self.free && buddy.free
    }
}

// =============================================================================
// FREE LIST
// =============================================================================

/// Free list for a specific order
#[derive(Debug)]
struct FreeList {
    /// Blocks at this order level
    blocks: Vec<BuddyBlock>,
}

impl FreeList {
    fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    fn push(&mut self, block: BuddyBlock) {
        self.blocks.push(block);
    }

    fn pop(&mut self) -> Option<BuddyBlock> {
        self.blocks.pop()
    }

    fn remove(&mut self, addr: GpuAddr) -> Option<BuddyBlock> {
        if let Some(idx) = self.blocks.iter().position(|b| b.addr == addr) {
            Some(self.blocks.swap_remove(idx))
        } else {
            None
        }
    }

    fn find_buddy(&self, addr: GpuAddr, size: u64) -> Option<usize> {
        let buddy_addr = GpuAddr(addr.0 ^ size);
        self.blocks.iter().position(|b| b.addr == buddy_addr)
    }

    fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    fn len(&self) -> usize {
        self.blocks.len()
    }
}

// =============================================================================
// BUDDY ALLOCATOR
// =============================================================================

/// Buddy allocator for GPU memory
#[derive(Debug)]
pub struct BuddyAllocator {
    /// Base address of managed region
    base: GpuAddr,
    /// Total size
    size: ByteSize,
    /// Free lists for each order
    free_lists: [FreeList; MAX_ORDER],
    /// Statistics
    stats: AllocatorStats,
}

/// Allocator statistics
#[derive(Debug, Clone, Default)]
pub struct AllocatorStats {
    /// Total allocations
    pub total_allocs: u64,
    /// Total frees
    pub total_frees: u64,
    /// Current allocated bytes
    pub allocated_bytes: u64,
    /// Peak allocated bytes
    pub peak_allocated: u64,
    /// Number of splits performed
    pub splits: u64,
    /// Number of merges performed
    pub merges: u64,
    /// Failed allocations
    pub failed_allocs: u64,
}

impl BuddyAllocator {
    /// Create a new buddy allocator
    pub fn new(base: GpuAddr, size: ByteSize) -> Result<Self> {
        let size_bytes = size.as_bytes();

        // Validate size is power of 2 and within bounds
        if size_bytes < MIN_BLOCK_SIZE || size_bytes > MAX_BLOCK_SIZE {
            return Err(Error::InvalidParameter);
        }

        if !size_bytes.is_power_of_two() {
            return Err(Error::InvalidParameter);
        }

        // Calculate initial order
        let order = (size_bytes.trailing_zeros() - MIN_BLOCK_SIZE.trailing_zeros()) as u8;

        // Create free lists
        let free_lists = core::array::from_fn(|_| FreeList::new());

        let mut allocator = Self {
            base,
            size,
            free_lists,
            stats: AllocatorStats::default(),
        };

        // Add initial block
        let initial_block = BuddyBlock::new(base, order);
        allocator.free_lists[order as usize].push(initial_block);

        Ok(allocator)
    }

    /// Calculate order for a given size
    fn size_to_order(size: u64) -> Option<u8> {
        if size < MIN_BLOCK_SIZE {
            return Some(0);
        }

        if size > MAX_BLOCK_SIZE {
            return None;
        }

        // Round up to power of 2
        let rounded = size.next_power_of_two();
        let order = (rounded.trailing_zeros() - MIN_BLOCK_SIZE.trailing_zeros()) as u8;

        if (order as usize) < MAX_ORDER {
            Some(order)
        } else {
            None
        }
    }

    /// Order to size
    const fn order_to_size(order: u8) -> u64 {
        MIN_BLOCK_SIZE << order
    }

    /// Allocate a block of the given size
    pub fn allocate(&mut self, size: ByteSize) -> Result<BuddyBlock> {
        let size_bytes = size.as_bytes();

        // Get required order
        let order = Self::size_to_order(size_bytes).ok_or(Error::InvalidParameter)?;

        // Find smallest available block
        let mut found_order = None;
        for o in (order as usize)..MAX_ORDER {
            if !self.free_lists[o].is_empty() {
                found_order = Some(o);
                break;
            }
        }

        let found_order = found_order.ok_or_else(|| {
            self.stats.failed_allocs += 1;
            Error::OutOfMemory
        })?;

        // Split down to required size
        let mut block = self.free_lists[found_order].pop().unwrap();

        while (block.order as usize) > (order as usize) {
            // Split block
            let new_order = block.order - 1;
            let half_size = Self::order_to_size(new_order);

            // Create buddy (upper half)
            let buddy = BuddyBlock::new(block.addr + half_size, new_order);
            self.free_lists[new_order as usize].push(buddy);

            // Shrink block (lower half)
            block.order = new_order;
            block.size = ByteSize::from_bytes(half_size);

            self.stats.splits += 1;
        }

        // Mark as allocated
        block.free = false;

        // Update stats
        self.stats.total_allocs += 1;
        self.stats.allocated_bytes += block.size.as_bytes();
        self.stats.peak_allocated = self.stats.peak_allocated.max(self.stats.allocated_bytes);

        Ok(block)
    }

    /// Free a previously allocated block
    pub fn free(&mut self, mut block: BuddyBlock) -> Result<()> {
        if block.free {
            return Err(Error::InvalidParameter);
        }

        block.free = true;
        self.stats.total_frees += 1;
        self.stats.allocated_bytes -= block.size.as_bytes();

        // Try to merge with buddies
        self.merge_block(block);

        Ok(())
    }

    /// Merge block with buddies recursively
    fn merge_block(&mut self, mut block: BuddyBlock) {
        loop {
            let order = block.order as usize;

            // Can't merge at max order
            if order >= MAX_ORDER - 1 {
                self.free_lists[order].push(block);
                return;
            }

            // Check for buddy
            let buddy_addr = block.buddy_addr();
            let block_size = block.size.as_bytes();

            if let Some(buddy_idx) = self.free_lists[order].find_buddy(block.addr, block_size) {
                // Remove buddy
                let buddy = self.free_lists[order].blocks.swap_remove(buddy_idx);

                // Merge: take lower address
                let merged_addr = min(block.addr, buddy.addr);
                block = BuddyBlock::new(merged_addr, block.order + 1);

                self.stats.merges += 1;
            } else {
                // No buddy available, add to free list
                self.free_lists[order].push(block);
                return;
            }
        }
    }

    /// Get allocator statistics
    pub fn stats(&self) -> &AllocatorStats {
        &self.stats
    }

    /// Get fragmentation ratio (0.0 = no fragmentation, 1.0 = fully fragmented)
    pub fn fragmentation(&self) -> f32 {
        if self.stats.allocated_bytes == 0 {
            return 0.0;
        }

        let total_free: u64 = self
            .free_lists
            .iter()
            .enumerate()
            .map(|(order, list)| list.len() as u64 * Self::order_to_size(order as u8))
            .sum();

        let total = self.size.as_bytes();
        let free_blocks: usize = self.free_lists.iter().map(|l| l.len()).sum();

        if free_blocks <= 1 {
            0.0
        } else {
            // Fragmentation = 1 - (largest_free / total_free)
            let largest = self
                .free_lists
                .iter()
                .enumerate()
                .rev()
                .find(|(_, l)| !l.is_empty())
                .map(|(o, _)| Self::order_to_size(o as u8))
                .unwrap_or(0);

            if total_free == 0 {
                0.0
            } else {
                1.0 - (largest as f32 / total_free as f32)
            }
        }
    }

    /// Get base address
    pub fn base(&self) -> GpuAddr {
        self.base
    }

    /// Get total size
    pub fn size(&self) -> ByteSize {
        self.size
    }

    /// Get free space
    pub fn free_space(&self) -> ByteSize {
        ByteSize::from_bytes(self.size.as_bytes() - self.stats.allocated_bytes)
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocator_creation() {
        let alloc = BuddyAllocator::new(GpuAddr(0), ByteSize::from_mib(1)).unwrap();
        assert_eq!(alloc.free_space(), ByteSize::from_mib(1));
        assert_eq!(alloc.fragmentation(), 0.0);
    }

    #[test]
    fn test_simple_alloc_free() {
        let mut alloc = BuddyAllocator::new(GpuAddr(0), ByteSize::from_mib(1)).unwrap();

        let block = alloc.allocate(ByteSize::from_kib(4)).unwrap();
        assert!(!block.free);

        alloc.free(block).unwrap();
        assert_eq!(alloc.stats().total_allocs, 1);
        assert_eq!(alloc.stats().total_frees, 1);
    }

    #[test]
    fn test_split_and_merge() {
        let mut alloc = BuddyAllocator::new(GpuAddr(0), ByteSize::from_mib(1)).unwrap();

        // Allocate small block (causes splits)
        let block1 = alloc.allocate(ByteSize::from_kib(4)).unwrap();
        assert!(alloc.stats().splits > 0);

        // Free should merge
        let splits_before = alloc.stats().splits;
        alloc.free(block1).unwrap();
        assert!(alloc.stats().merges > 0);
    }
}
