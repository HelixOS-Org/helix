//! # Frame Allocator Trait

use helix_hal::PageSize;

use crate::{Frame, MemResult};

/// Simple frame allocator trait for boot-time allocation
pub trait FrameAllocator {
    /// Allocate a frame
    fn allocate_frame(&mut self) -> MemResult<Frame>;

    /// Deallocate a frame
    fn deallocate_frame(&mut self, frame: Frame);
}

/// Bump allocator for early boot
pub struct BumpAllocator {
    /// Next free address
    next: u64,
    /// End of available memory
    end: u64,
    /// Page size
    page_size: PageSize,
}

impl BumpAllocator {
    /// Create a new bump allocator
    pub fn new(start: u64, end: u64, page_size: PageSize) -> Self {
        // Align start to page boundary
        let page_mask = page_size.size() - 1;
        let aligned_start = (start + page_mask) & !page_mask;

        Self {
            next: aligned_start,
            end,
            page_size,
        }
    }

    /// Get remaining frames
    pub fn remaining(&self) -> usize {
        if self.next >= self.end {
            0
        } else {
            ((self.end - self.next) / self.page_size.size()) as usize
        }
    }
}

impl FrameAllocator for BumpAllocator {
    fn allocate_frame(&mut self) -> MemResult<Frame> {
        let size = self.page_size.size();

        if self.next + size > self.end {
            return Err(crate::MemError::OutOfMemory);
        }

        let frame = Frame::new(helix_hal::PhysAddr::new(self.next), self.page_size);

        self.next += size;
        Ok(frame)
    }

    fn deallocate_frame(&mut self, _frame: Frame) {
        // Bump allocator doesn't support deallocation
        // This is fine for early boot
    }
}
