//! # Page Table Mapper
//!
//! Architecture-agnostic page table manipulation.

use helix_hal::{PhysAddr, VirtAddr};
use spin::Mutex;

use super::{PageFlags, VirtualMapper};
use crate::physical::frame_allocator::FrameAllocator;
use crate::{Frame, MemError, MemResult, Page};

/// Generic page table mapper
pub struct PageTableMapper<A: FrameAllocator> {
    /// Root page table physical address
    root: PhysAddr,
    /// Frame allocator for page tables
    allocator: Mutex<A>,
}

impl<A: FrameAllocator> PageTableMapper<A> {
    /// Create a new mapper
    pub fn new(root: PhysAddr, allocator: A) -> Self {
        Self {
            root,
            allocator: Mutex::new(allocator),
        }
    }

    /// Get root table address
    pub fn root(&self) -> PhysAddr {
        self.root
    }
}

impl<A: FrameAllocator + Send + Sync> VirtualMapper for PageTableMapper<A> {
    fn map(&self, page: Page, frame: Frame, flags: PageFlags) -> MemResult<()> {
        // This would be architecture-specific
        // Walk the page table, allocating intermediate tables as needed
        // Set the final entry to point to the frame with the given flags

        log::trace!(
            "Mapping {:?} -> {:?} with flags {:?}",
            page.address(),
            frame.address(),
            flags
        );

        // Placeholder - real implementation would manipulate page tables
        Ok(())
    }

    fn unmap(&self, page: Page) -> MemResult<Frame> {
        log::trace!("Unmapping {:?}", page.address());

        // Placeholder - would return the previously mapped frame
        Err(MemError::NotMapped)
    }

    fn update_flags(&self, page: Page, flags: PageFlags) -> MemResult<()> {
        log::trace!("Updating flags for {:?} to {:?}", page.address(), flags);

        // Placeholder
        Ok(())
    }

    fn translate(&self, virt: VirtAddr) -> Option<PhysAddr> {
        // Walk page tables to find physical address
        // Placeholder
        None
    }

    fn flush(&self, page: Page) {
        // Architecture-specific TLB flush
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!(
                "invlpg [{}]",
                in(reg) page.address().as_u64(),
                options(nostack, preserves_flags)
            );
        }
    }

    fn flush_all(&self) {
        // Architecture-specific full TLB flush
        #[cfg(target_arch = "x86_64")]
        unsafe {
            // Reload CR3
            let cr3: u64;
            core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack));
            core::arch::asm!("mov cr3, {}", in(reg) cr3, options(nomem, nostack));
        }
    }
}
