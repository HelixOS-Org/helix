//! # Buddy Allocator
//!
//! A buddy system allocator for efficient physical memory management.

use crate::{Frame, MemResult, MemError, MemoryZone};
use super::{PhysicalAllocator, PhysicalRegion, AllocatorStats};
use helix_hal::{PhysAddr, PageSize};
use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use spin::Mutex;
use core::sync::atomic::{AtomicU64, Ordering};

/// Maximum order (2^MAX_ORDER pages)
const MAX_ORDER: usize = 11; // Up to 8MB blocks (2^11 * 4KB)

/// Buddy allocator
pub struct BuddyAllocator {
    /// Free lists for each order
    free_lists: Mutex<[BTreeSet<u64>; MAX_ORDER]>,
    /// Base address
    base: PhysAddr,
    /// Total size
    total_size: u64,
    /// Page size
    page_size: usize,
    /// Statistics
    stats: AllocatorStats,
    /// Allocation count
    alloc_count: AtomicU64,
    /// Deallocation count
    dealloc_count: AtomicU64,
}

impl BuddyAllocator {
    /// Create a new buddy allocator
    pub fn new() -> Self {
        Self {
            free_lists: Mutex::new(core::array::from_fn(|_| BTreeSet::new())),
            base: PhysAddr::new(0),
            total_size: 0,
            page_size: PageSize::Size4KiB.size() as usize,
            stats: AllocatorStats::default(),
            alloc_count: AtomicU64::new(0),
            dealloc_count: AtomicU64::new(0),
        }
    }

    /// Get order for a size
    fn size_to_order(&self, size: usize) -> usize {
        let pages = (size + self.page_size - 1) / self.page_size;
        let order = (usize::BITS - pages.leading_zeros()) as usize;
        if 1 << order == pages {
            order
        } else {
            order
        }
    }

    /// Get buddy address
    fn buddy_addr(&self, addr: u64, order: usize) -> u64 {
        addr ^ (1 << (order + 12)) // Assuming 4KB pages (12 bits)
    }

    /// Split a block into two buddies
    fn split(&self, lists: &mut [BTreeSet<u64>; MAX_ORDER], addr: u64, order: usize) {
        if order == 0 {
            return;
        }
        
        let buddy = self.buddy_addr(addr, order - 1);
        lists[order - 1].insert(addr);
        lists[order - 1].insert(buddy);
    }

    /// Try to merge with buddy
    fn try_merge(&self, lists: &mut [BTreeSet<u64>; MAX_ORDER], addr: u64, order: usize) -> bool {
        if order >= MAX_ORDER - 1 {
            return false;
        }

        let buddy = self.buddy_addr(addr, order);
        
        if lists[order].remove(&buddy) {
            // Merge successful, add to higher order
            let merged = addr.min(buddy);
            lists[order].remove(&addr);
            
            // Try to merge further
            if !self.try_merge(lists, merged, order + 1) {
                lists[order + 1].insert(merged);
            }
            true
        } else {
            false
        }
    }
}

impl Default for BuddyAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicalAllocator for BuddyAllocator {
    fn name(&self) -> &'static str {
        "Buddy Allocator"
    }

    fn init(&mut self, regions: &[PhysicalRegion]) -> MemResult<()> {
        let mut lists = self.free_lists.lock();
        
        for region in regions {
            if !region.is_usable() {
                continue;
            }

            if self.base.as_u64() == 0 {
                self.base = region.start;
            }

            // Add region to free lists
            let mut addr = region.start.as_u64();
            let end = region.end().as_u64();

            while addr < end {
                // Find the largest order that fits
                let mut order = MAX_ORDER - 1;
                while order > 0 {
                    let block_size = (1 << order) * self.page_size as u64;
                    if addr + block_size <= end && addr % block_size == 0 {
                        break;
                    }
                    order -= 1;
                }

                let block_size = (1 << order) * self.page_size as u64;
                lists[order].insert(addr);
                addr += block_size;
                self.total_size += block_size;
            }
        }

        log::info!(
            "Buddy allocator initialized: {} bytes total",
            self.total_size
        );

        Ok(())
    }

    fn allocate(&self, size: PageSize) -> MemResult<Frame> {
        let order = match size {
            PageSize::Size4KiB => 0,
            PageSize::Size2MiB => 9,  // 2MB = 512 * 4KB
            PageSize::Size1GiB => 18,  // 1GB (not typically supported)
        };

        if order >= MAX_ORDER {
            return Err(MemError::InvalidSize);
        }

        let mut lists = self.free_lists.lock();

        // Find a block of sufficient size
        for o in order..MAX_ORDER {
            if let Some(&addr) = lists[o].iter().next() {
                lists[o].remove(&addr);

                // Split if necessary
                for split_order in (order..o).rev() {
                    let buddy = self.buddy_addr(addr, split_order);
                    lists[split_order].insert(buddy);
                }

                self.alloc_count.fetch_add(1, Ordering::Relaxed);
                return Ok(Frame::new(PhysAddr::new(addr), size));
            }
        }

        Err(MemError::OutOfMemory)
    }

    fn allocate_contiguous(&self, count: usize, size: PageSize) -> MemResult<Frame> {
        // For buddy allocator, we allocate a block of appropriate order
        let total_size = count * size.size() as usize;
        let order = self.size_to_order(total_size);

        if order >= MAX_ORDER {
            return Err(MemError::InvalidSize);
        }

        let mut lists = self.free_lists.lock();

        for o in order..MAX_ORDER {
            if let Some(&addr) = lists[o].iter().next() {
                lists[o].remove(&addr);

                // Split excess
                for split_order in (order..o).rev() {
                    let buddy = self.buddy_addr(addr, split_order);
                    lists[split_order].insert(buddy);
                }

                self.alloc_count.fetch_add(1, Ordering::Relaxed);
                return Ok(Frame::new(PhysAddr::new(addr), size));
            }
        }

        Err(MemError::OutOfMemory)
    }

    fn allocate_zone(&self, size: PageSize, _zone: MemoryZone) -> MemResult<Frame> {
        // Simple implementation
        self.allocate(size)
    }

    fn deallocate(&self, frame: Frame) -> MemResult<()> {
        let addr = frame.address().as_u64();
        let order = match frame.size() {
            PageSize::Size4KiB => 0,
            PageSize::Size2MiB => 9,
            PageSize::Size1GiB => 18,
        };

        if order >= MAX_ORDER {
            return Err(MemError::InvalidSize);
        }

        let mut lists = self.free_lists.lock();

        // Try to merge with buddy
        if !self.try_merge(&mut lists, addr, order) {
            lists[order].insert(addr);
        }

        self.dealloc_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn free_frames(&self) -> usize {
        let lists = self.free_lists.lock();
        lists.iter()
            .enumerate()
            .map(|(order, set)| set.len() * (1 << order))
            .sum()
    }

    fn total_frames(&self) -> usize {
        (self.total_size / self.page_size as u64) as usize
    }

    fn stats(&self) -> AllocatorStats {
        let allocs = self.alloc_count.load(Ordering::Relaxed);
        let deallocs = self.dealloc_count.load(Ordering::Relaxed);

        AllocatorStats {
            allocations: allocs,
            deallocations: deallocs,
            current_allocations: allocs - deallocs,
            peak_allocations: allocs,
            bytes_allocated: allocs * self.page_size as u64,
            bytes_freed: deallocs * self.page_size as u64,
            fragmentation: 0,
        }
    }
}
