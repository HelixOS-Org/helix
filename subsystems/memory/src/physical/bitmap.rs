//! # Bitmap Allocator
//!
//! A simple bitmap-based physical page allocator.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use helix_hal::{PageSize, PhysAddr};
use spin::Mutex;

use super::{AllocatorStats, PhysicalAllocator, PhysicalRegion};
use crate::{Frame, MemError, MemResult, MemoryZone};

/// Bitmap allocator
pub struct BitmapAllocator {
    /// Bitmap data (1 = used, 0 = free)
    bitmap: Mutex<Vec<u64>>,
    /// Base address
    base: PhysAddr,
    /// Total number of frames
    total_frames: usize,
    /// Free frame count
    free_count: AtomicU64,
    /// Statistics
    allocations: AtomicU64,
    deallocations: AtomicU64,
}

impl BitmapAllocator {
    /// Create a new bitmap allocator
    pub fn new() -> Self {
        Self {
            bitmap: Mutex::new(Vec::new()),
            base: PhysAddr::new(0),
            total_frames: 0,
            free_count: AtomicU64::new(0),
            allocations: AtomicU64::new(0),
            deallocations: AtomicU64::new(0),
        }
    }

    /// Find first free bit
    fn find_free(&self) -> Option<usize> {
        let bitmap = self.bitmap.lock();
        for (word_idx, &word) in bitmap.iter().enumerate() {
            if word != u64::MAX {
                // Find the first zero bit
                let bit = (!word).trailing_zeros() as usize;
                let frame_idx = word_idx * 64 + bit;
                if frame_idx < self.total_frames {
                    return Some(frame_idx);
                }
            }
        }
        None
    }

    /// Find contiguous free frames
    fn find_contiguous(&self, count: usize) -> Option<usize> {
        let bitmap = self.bitmap.lock();
        let mut start = 0;
        let mut found = 0;

        for frame_idx in 0..self.total_frames {
            let word_idx = frame_idx / 64;
            let bit_idx = frame_idx % 64;

            if bitmap[word_idx] & (1 << bit_idx) == 0 {
                if found == 0 {
                    start = frame_idx;
                }
                found += 1;
                if found >= count {
                    return Some(start);
                }
            } else {
                found = 0;
            }
        }

        None
    }

    /// Set frame as used
    fn set_used(&self, frame_idx: usize) {
        let mut bitmap = self.bitmap.lock();
        let word_idx = frame_idx / 64;
        let bit_idx = frame_idx % 64;
        bitmap[word_idx] |= 1 << bit_idx;
    }

    /// Set frame as free
    fn set_free(&self, frame_idx: usize) {
        let mut bitmap = self.bitmap.lock();
        let word_idx = frame_idx / 64;
        let bit_idx = frame_idx % 64;
        bitmap[word_idx] &= !(1 << bit_idx);
    }

    /// Get frame index from address
    fn frame_index(&self, addr: PhysAddr) -> usize {
        ((addr.as_u64() - self.base.as_u64()) / PageSize::Size4KiB.size()) as usize
    }

    /// Get address from frame index
    fn frame_address(&self, idx: usize) -> PhysAddr {
        PhysAddr::new(self.base.as_u64() + (idx as u64 * PageSize::Size4KiB.size()))
    }
}

impl Default for BitmapAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicalAllocator for BitmapAllocator {
    fn name(&self) -> &'static str {
        "Bitmap Allocator"
    }

    fn init(&mut self, regions: &[PhysicalRegion]) -> MemResult<()> {
        // Find usable memory
        let mut min_addr = u64::MAX;
        let mut max_addr = 0u64;

        for region in regions {
            if region.is_usable() {
                min_addr = min_addr.min(region.start.as_u64());
                max_addr = max_addr.max(region.end().as_u64());
            }
        }

        if min_addr >= max_addr {
            return Err(MemError::InvalidRegion);
        }

        // Calculate bitmap size
        let page_size = PageSize::Size4KiB.size();
        let total_frames = ((max_addr - min_addr) / page_size) as usize;
        let bitmap_words = (total_frames + 63) / 64;

        // Initialize bitmap (all used initially)
        let mut bitmap = self.bitmap.lock();
        bitmap.resize(bitmap_words, u64::MAX);

        self.base = PhysAddr::new(min_addr);
        self.total_frames = total_frames;

        // Mark usable regions as free
        let mut free = 0u64;
        for region in regions {
            if region.is_usable() {
                let start_frame = ((region.start.as_u64() - min_addr) / page_size) as usize;
                let end_frame = ((region.end().as_u64() - min_addr) / page_size) as usize;

                for frame_idx in start_frame..end_frame {
                    if frame_idx < total_frames {
                        let word_idx = frame_idx / 64;
                        let bit_idx = frame_idx % 64;
                        bitmap[word_idx] &= !(1 << bit_idx);
                        free += 1;
                    }
                }
            }
        }

        drop(bitmap);
        self.free_count.store(free, Ordering::SeqCst);

        log::info!(
            "Bitmap allocator initialized: {} frames, {} free",
            total_frames,
            free
        );

        Ok(())
    }

    fn allocate(&self, size: PageSize) -> MemResult<Frame> {
        if size != PageSize::Size4KiB {
            // For now, only support normal pages
            // Large/huge pages would need different handling
            return Err(MemError::InvalidSize);
        }

        let frame_idx = self.find_free().ok_or(MemError::OutOfMemory)?;

        self.set_used(frame_idx);
        self.free_count.fetch_sub(1, Ordering::SeqCst);
        self.allocations.fetch_add(1, Ordering::Relaxed);

        Ok(Frame::new(self.frame_address(frame_idx), size))
    }

    fn allocate_contiguous(&self, count: usize, size: PageSize) -> MemResult<Frame> {
        if size != PageSize::Size4KiB {
            return Err(MemError::InvalidSize);
        }

        let start_idx = self.find_contiguous(count).ok_or(MemError::OutOfMemory)?;

        for i in 0..count {
            self.set_used(start_idx + i);
        }

        self.free_count.fetch_sub(count as u64, Ordering::SeqCst);
        self.allocations.fetch_add(1, Ordering::Relaxed);

        Ok(Frame::new(self.frame_address(start_idx), size))
    }

    fn allocate_zone(&self, size: PageSize, _zone: MemoryZone) -> MemResult<Frame> {
        // Simple implementation: just allocate from anywhere
        // A proper implementation would track zones separately
        self.allocate(size)
    }

    fn deallocate(&self, frame: Frame) -> MemResult<()> {
        let frame_idx = self.frame_index(frame.address());

        if frame_idx >= self.total_frames {
            return Err(MemError::InvalidAddress);
        }

        self.set_free(frame_idx);
        self.free_count.fetch_add(1, Ordering::SeqCst);
        self.deallocations.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    fn free_frames(&self) -> usize {
        self.free_count.load(Ordering::Relaxed) as usize
    }

    fn total_frames(&self) -> usize {
        self.total_frames
    }

    fn stats(&self) -> AllocatorStats {
        let allocations = self.allocations.load(Ordering::Relaxed);
        let deallocations = self.deallocations.load(Ordering::Relaxed);

        AllocatorStats {
            allocations,
            deallocations,
            current_allocations: allocations - deallocations,
            peak_allocations: allocations, // Not tracked accurately
            bytes_allocated: allocations * PageSize::Size4KiB.size(),
            bytes_freed: deallocations * PageSize::Size4KiB.size(),
            fragmentation: 0, // Would need more complex tracking
        }
    }
}
