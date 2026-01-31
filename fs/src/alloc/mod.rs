//! Block allocator module for HelixFS.
//!
//! This module provides block allocation with:
//! - Multi-zone allocation for NUMA awareness
//! - Best-fit and buddy allocation strategies
//! - Free space tracking with bitmaps and extent trees
//! - Copy-on-Write support
//! - Defragmentation hints

pub mod bitmap;
pub mod buddy;
pub mod zone;
pub mod freelist;
pub mod cow;

pub use bitmap::*;
pub use buddy::*;
pub use zone::*;
pub use freelist::*;
pub use cow::*;

use crate::core::types::*;
use crate::core::error::HfsResult;
use core::sync::atomic::{AtomicU64, AtomicU32, Ordering};

// ============================================================================
// Allocation Statistics
// ============================================================================

/// Block allocator statistics.
#[derive(Default)]
pub struct AllocStats {
    /// Total allocations
    pub total_allocs: AtomicU64,
    /// Total deallocations
    pub total_frees: AtomicU64,
    /// Total blocks allocated
    pub blocks_allocated: AtomicU64,
    /// Total blocks freed
    pub blocks_freed: AtomicU64,
    /// Failed allocations
    pub failed_allocs: AtomicU64,
    /// Fragmentation events
    pub fragmentation_events: AtomicU64,
    /// CoW allocations
    pub cow_allocs: AtomicU64,
    /// Largest allocation
    pub largest_alloc: AtomicU32,
    /// Smallest allocation
    pub smallest_alloc: AtomicU32,
}

impl AllocStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            total_allocs: AtomicU64::new(0),
            total_frees: AtomicU64::new(0),
            blocks_allocated: AtomicU64::new(0),
            blocks_freed: AtomicU64::new(0),
            failed_allocs: AtomicU64::new(0),
            fragmentation_events: AtomicU64::new(0),
            cow_allocs: AtomicU64::new(0),
            largest_alloc: AtomicU32::new(0),
            smallest_alloc: AtomicU32::new(u32::MAX),
        }
    }
    
    /// Record allocation
    pub fn record_alloc(&self, blocks: u32) {
        self.total_allocs.fetch_add(1, Ordering::Relaxed);
        self.blocks_allocated.fetch_add(blocks as u64, Ordering::Relaxed);
        
        // Update largest
        let mut current = self.largest_alloc.load(Ordering::Relaxed);
        while blocks > current {
            match self.largest_alloc.compare_exchange_weak(
                current,
                blocks,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(c) => current = c,
            }
        }
        
        // Update smallest
        let mut current = self.smallest_alloc.load(Ordering::Relaxed);
        while blocks < current {
            match self.smallest_alloc.compare_exchange_weak(
                current,
                blocks,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(c) => current = c,
            }
        }
    }
    
    /// Record free
    pub fn record_free(&self, blocks: u32) {
        self.total_frees.fetch_add(1, Ordering::Relaxed);
        self.blocks_freed.fetch_add(blocks as u64, Ordering::Relaxed);
    }
    
    /// Record failed allocation
    pub fn record_failed_alloc(&self) {
        self.failed_allocs.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record CoW allocation
    pub fn record_cow_alloc(&self) {
        self.cow_allocs.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Get snapshot
    pub fn snapshot(&self) -> AllocStatsSnapshot {
        AllocStatsSnapshot {
            total_allocs: self.total_allocs.load(Ordering::Relaxed),
            total_frees: self.total_frees.load(Ordering::Relaxed),
            blocks_allocated: self.blocks_allocated.load(Ordering::Relaxed),
            blocks_freed: self.blocks_freed.load(Ordering::Relaxed),
            failed_allocs: self.failed_allocs.load(Ordering::Relaxed),
            fragmentation_events: self.fragmentation_events.load(Ordering::Relaxed),
            cow_allocs: self.cow_allocs.load(Ordering::Relaxed),
            largest_alloc: self.largest_alloc.load(Ordering::Relaxed),
            smallest_alloc: self.smallest_alloc.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of allocation statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct AllocStatsSnapshot {
    pub total_allocs: u64,
    pub total_frees: u64,
    pub blocks_allocated: u64,
    pub blocks_freed: u64,
    pub failed_allocs: u64,
    pub fragmentation_events: u64,
    pub cow_allocs: u64,
    pub largest_alloc: u32,
    pub smallest_alloc: u32,
}

impl AllocStatsSnapshot {
    /// Get current blocks in use
    #[inline]
    pub fn blocks_in_use(&self) -> u64 {
        self.blocks_allocated.saturating_sub(self.blocks_freed)
    }
    
    /// Average allocation size
    pub fn avg_alloc_size(&self) -> f64 {
        if self.total_allocs == 0 {
            0.0
        } else {
            self.blocks_allocated as f64 / self.total_allocs as f64
        }
    }
}

// ============================================================================
// Allocation Flags
// ============================================================================

/// Allocation request flags.
#[derive(Clone, Copy, Default)]
#[repr(transparent)]
pub struct AllocFlags(pub u32);

impl AllocFlags {
    /// Normal allocation
    pub const NORMAL: u32 = 0;
    /// Contiguous allocation required
    pub const CONTIGUOUS: u32 = 1 << 0;
    /// Zero the allocated blocks
    pub const ZERO: u32 = 1 << 1;
    /// Preallocation (not written yet)
    pub const PREALLOC: u32 = 1 << 2;
    /// Metadata allocation
    pub const METADATA: u32 = 1 << 3;
    /// Journal allocation
    pub const JOURNAL: u32 = 1 << 4;
    /// CoW allocation
    pub const COW: u32 = 1 << 5;
    /// High priority
    pub const HIGH_PRIORITY: u32 = 1 << 6;
    /// Best effort (allow partial allocation)
    pub const BEST_EFFORT: u32 = 1 << 7;
    /// Near specific block (locality hint)
    pub const NEAR: u32 = 1 << 8;
    /// In specific zone
    pub const ZONE_PINNED: u32 = 1 << 9;
    
    /// Create new flags
    #[inline]
    pub const fn new(bits: u32) -> Self {
        Self(bits)
    }
    
    /// Check flag
    #[inline]
    pub fn has(&self, flag: u32) -> bool {
        (self.0 & flag) != 0
    }
    
    /// Set flag
    #[inline]
    pub fn set(&mut self, flag: u32) {
        self.0 |= flag;
    }
    
    /// Clear flag
    #[inline]
    pub fn clear(&mut self, flag: u32) {
        self.0 &= !flag;
    }
}

// ============================================================================
// Allocation Request
// ============================================================================

/// Block allocation request.
#[derive(Clone, Copy)]
pub struct AllocRequest {
    /// Number of blocks requested
    pub count: u32,
    /// Minimum acceptable count
    pub min_count: u32,
    /// Flags
    pub flags: AllocFlags,
    /// Preferred starting block (hint)
    pub goal: Option<BlockNum>,
    /// Zone to allocate from (if zone-pinned)
    pub zone_id: Option<u32>,
    /// Inode making the allocation (for locality)
    pub inode: Option<InodeNum>,
}

impl AllocRequest {
    /// Create simple allocation request
    pub const fn new(count: u32) -> Self {
        Self {
            count,
            min_count: count,
            flags: AllocFlags(AllocFlags::NORMAL),
            goal: None,
            zone_id: None,
            inode: None,
        }
    }
    
    /// Create with goal block
    pub const fn with_goal(mut self, goal: BlockNum) -> Self {
        self.goal = Some(goal);
        self.flags.0 |= AllocFlags::NEAR;
        self
    }
    
    /// Create with zone
    pub const fn with_zone(mut self, zone_id: u32) -> Self {
        self.zone_id = Some(zone_id);
        self.flags.0 |= AllocFlags::ZONE_PINNED;
        self
    }
    
    /// Create with flags
    pub const fn with_flags(mut self, flags: u32) -> Self {
        self.flags.0 |= flags;
        self
    }
    
    /// Create with minimum count (allows partial allocation)
    pub const fn with_min(mut self, min: u32) -> Self {
        self.min_count = min;
        self.flags.0 |= AllocFlags::BEST_EFFORT;
        self
    }
    
    /// For metadata
    pub const fn metadata(count: u32) -> Self {
        Self::new(count).with_flags(AllocFlags::METADATA)
    }
    
    /// For CoW
    pub const fn cow(count: u32, original: BlockNum) -> Self {
        Self::new(count)
            .with_goal(original)
            .with_flags(AllocFlags::COW)
    }
}

// ============================================================================
// Allocation Result
// ============================================================================

/// Successful allocation result.
#[derive(Clone, Copy, Debug)]
pub struct AllocResult {
    /// Starting block of allocation
    pub start: BlockNum,
    /// Number of blocks allocated
    pub count: u32,
    /// Zone that was used
    pub zone_id: u32,
}

impl AllocResult {
    /// Create new result
    #[inline]
    pub const fn new(start: BlockNum, count: u32, zone_id: u32) -> Self {
        Self { start, count, zone_id }
    }
    
    /// Get end block (exclusive)
    #[inline]
    pub fn end(&self) -> BlockNum {
        BlockNum::new(self.start.get() + self.count as u64)
    }
    
    /// Convert to extent
    pub fn to_extent(&self, logical: BlockNum) -> Extent {
        Extent::new(logical.get(), self.start.get(), self.count)
    }
}

// ============================================================================
// Block Allocator Trait
// ============================================================================

/// Block allocator interface.
pub trait BlockAllocator: Send + Sync {
    /// Allocate blocks
    fn allocate(&self, request: &AllocRequest) -> HfsResult<AllocResult>;
    
    /// Free blocks
    fn free(&self, start: BlockNum, count: u32) -> HfsResult<()>;
    
    /// Get free block count
    fn free_blocks(&self) -> u64;
    
    /// Get total block count
    fn total_blocks(&self) -> u64;
    
    /// Get used block count
    fn used_blocks(&self) -> u64 {
        self.total_blocks() - self.free_blocks()
    }
    
    /// Get usage percentage
    fn usage_percent(&self) -> f64 {
        let total = self.total_blocks();
        if total == 0 {
            return 0.0;
        }
        (self.used_blocks() as f64 / total as f64) * 100.0
    }
    
    /// Check if block is free
    fn is_free(&self, block: BlockNum) -> bool;
    
    /// Reserve blocks (mark as used but not allocated)
    fn reserve(&self, start: BlockNum, count: u32) -> HfsResult<()>;
    
    /// Unreserve blocks
    fn unreserve(&self, start: BlockNum, count: u32) -> HfsResult<()>;
    
    /// Sync allocator state
    fn sync(&self) -> HfsResult<()>;
}

// ============================================================================
// Space Manager (combines zones with unified interface)
// ============================================================================

/// Maximum allocation zones
pub const MAX_ZONES: usize = 32;

/// Space manager combining multiple allocation zones.
pub struct SpaceManager {
    /// Total blocks in filesystem
    total_blocks: u64,
    /// Free blocks
    free_blocks: AtomicU64,
    /// Reserved blocks (metadata, journal, etc.)
    reserved_blocks: AtomicU64,
    /// Block size
    block_size: u32,
    /// Allocation statistics
    stats: AllocStats,
    /// Low space threshold (percent * 10)
    low_space_threshold: u32,
}

impl SpaceManager {
    /// Create new space manager
    pub fn new(total_blocks: u64, block_size: u32) -> Self {
        Self {
            total_blocks,
            free_blocks: AtomicU64::new(total_blocks),
            reserved_blocks: AtomicU64::new(0),
            block_size,
            stats: AllocStats::new(),
            low_space_threshold: 50, // 5%
        }
    }
    
    /// Get total capacity in bytes
    #[inline]
    pub fn total_bytes(&self) -> u64 {
        self.total_blocks * self.block_size as u64
    }
    
    /// Get free bytes
    #[inline]
    pub fn free_bytes(&self) -> u64 {
        self.free_blocks.load(Ordering::Relaxed) * self.block_size as u64
    }
    
    /// Get used bytes
    #[inline]
    pub fn used_bytes(&self) -> u64 {
        self.total_bytes() - self.free_bytes()
    }
    
    /// Check if space is low
    #[inline]
    pub fn is_low_space(&self) -> bool {
        let free = self.free_blocks.load(Ordering::Relaxed);
        let threshold = (self.total_blocks * self.low_space_threshold as u64) / 1000;
        free < threshold
    }
    
    /// Get stats
    #[inline]
    pub fn stats(&self) -> &AllocStats {
        &self.stats
    }
    
    /// Decrease free block count
    pub fn consume(&self, blocks: u32) -> bool {
        let mut current = self.free_blocks.load(Ordering::Relaxed);
        loop {
            if current < blocks as u64 {
                return false;
            }
            match self.free_blocks.compare_exchange_weak(
                current,
                current - blocks as u64,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    self.stats.record_alloc(blocks);
                    return true;
                }
                Err(c) => current = c,
            }
        }
    }
    
    /// Increase free block count
    pub fn release(&self, blocks: u32) {
        self.free_blocks.fetch_add(blocks as u64, Ordering::Relaxed);
        self.stats.record_free(blocks);
    }
    
    /// Reserve blocks for system use
    pub fn reserve(&self, blocks: u64) {
        self.reserved_blocks.fetch_add(blocks, Ordering::Relaxed);
        // Decrease effective free blocks
        let mut current = self.free_blocks.load(Ordering::Relaxed);
        loop {
            let new_val = current.saturating_sub(blocks);
            match self.free_blocks.compare_exchange_weak(
                current,
                new_val,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(c) => current = c,
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_alloc_flags() {
        let mut flags = AllocFlags::new(AllocFlags::CONTIGUOUS);
        
        assert!(flags.has(AllocFlags::CONTIGUOUS));
        assert!(!flags.has(AllocFlags::ZERO));
        
        flags.set(AllocFlags::ZERO);
        assert!(flags.has(AllocFlags::ZERO));
        
        flags.clear(AllocFlags::CONTIGUOUS);
        assert!(!flags.has(AllocFlags::CONTIGUOUS));
    }
    
    #[test]
    fn test_alloc_request() {
        let req = AllocRequest::new(10)
            .with_goal(BlockNum::new(1000))
            .with_flags(AllocFlags::METADATA);
        
        assert_eq!(req.count, 10);
        assert!(req.goal.is_some());
        assert!(req.flags.has(AllocFlags::NEAR));
        assert!(req.flags.has(AllocFlags::METADATA));
    }
    
    #[test]
    fn test_space_manager() {
        let manager = SpaceManager::new(10000, 4096);
        
        assert_eq!(manager.free_bytes(), 10000 * 4096);
        
        assert!(manager.consume(100));
        assert_eq!(
            manager.free_blocks.load(Ordering::Relaxed),
            9900
        );
        
        manager.release(50);
        assert_eq!(
            manager.free_blocks.load(Ordering::Relaxed),
            9950
        );
    }
    
    #[test]
    fn test_alloc_stats() {
        let stats = AllocStats::new();
        
        stats.record_alloc(10);
        stats.record_alloc(100);
        stats.record_free(50);
        
        let snap = stats.snapshot();
        
        assert_eq!(snap.total_allocs, 2);
        assert_eq!(snap.blocks_allocated, 110);
        assert_eq!(snap.blocks_freed, 50);
        assert_eq!(snap.largest_alloc, 100);
        assert_eq!(snap.smallest_alloc, 10);
    }
}
