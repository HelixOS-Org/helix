//! # Virtual Memory Management
//!
//! Framework for virtual memory and address space management.

pub mod address_space;
pub mod mapper;

use bitflags::bitflags;
use helix_hal::{PhysAddr, VirtAddr};

use crate::{Frame, MemResult, Page};

bitflags! {
    /// Page mapping flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageFlags: u64 {
        /// Page is present
        const PRESENT = 1 << 0;
        /// Page is writable
        const WRITABLE = 1 << 1;
        /// Page is accessible from user mode
        const USER = 1 << 2;
        /// Write-through caching
        const WRITE_THROUGH = 1 << 3;
        /// Disable caching
        const NO_CACHE = 1 << 4;
        /// Page has been accessed
        const ACCESSED = 1 << 5;
        /// Page has been written to
        const DIRTY = 1 << 6;
        /// Huge page
        const HUGE = 1 << 7;
        /// Global page (not flushed on context switch)
        const GLOBAL = 1 << 8;
        /// No execute
        const NO_EXECUTE = 1 << 63;
    }
}

impl PageFlags {
    /// Read-only kernel page
    pub const KERNEL_READ: Self = Self::PRESENT;

    /// Read-write kernel page
    pub const KERNEL_WRITE: Self = Self::PRESENT.union(Self::WRITABLE);

    /// Read-only user page
    pub const USER_READ: Self = Self::PRESENT.union(Self::USER);

    /// Read-write user page
    pub const USER_WRITE: Self = Self::PRESENT.union(Self::USER).union(Self::WRITABLE);

    /// Kernel code (read + execute)
    pub const KERNEL_CODE: Self = Self::PRESENT;

    /// User code (read + execute)
    pub const USER_CODE: Self = Self::PRESENT.union(Self::USER);
}

/// Virtual memory mapper trait
pub trait VirtualMapper: Send + Sync {
    /// Map a page to a frame
    fn map(&self, page: Page, frame: Frame, flags: PageFlags) -> MemResult<()>;

    /// Unmap a page
    fn unmap(&self, page: Page) -> MemResult<Frame>;

    /// Change page flags
    fn update_flags(&self, page: Page, flags: PageFlags) -> MemResult<()>;

    /// Translate virtual to physical address
    fn translate(&self, virt: VirtAddr) -> Option<PhysAddr>;

    /// Flush TLB for a page
    fn flush(&self, page: Page);

    /// Flush entire TLB
    fn flush_all(&self);
}

/// Address space identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AddressSpaceId(u64);

impl AddressSpaceId {
    /// Kernel address space
    pub const KERNEL: Self = Self(0);

    /// Create a new ID
    pub fn new() -> Self {
        use core::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for AddressSpaceId {
    fn default() -> Self {
        Self::new()
    }
}

/// Virtual memory region
#[derive(Debug, Clone)]
pub struct VmRegion {
    /// Start address
    pub start: VirtAddr,
    /// Size in bytes
    pub size: u64,
    /// Flags
    pub flags: PageFlags,
    /// Region type
    pub region_type: VmRegionType,
}

/// Virtual memory region types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmRegionType {
    /// Anonymous memory (heap, stack)
    Anonymous,
    /// File-backed memory
    File,
    /// Device memory (MMIO)
    Device,
    /// Kernel memory
    Kernel,
    /// Guard page
    Guard,
    /// Reserved
    Reserved,
}

impl VmRegion {
    /// Get end address
    pub fn end(&self) -> VirtAddr {
        VirtAddr::new(self.start.as_u64() + self.size)
    }

    /// Check if contains address
    pub fn contains(&self, addr: VirtAddr) -> bool {
        addr >= self.start && addr < self.end()
    }

    /// Check if overlaps with another region
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end() && other.start < self.end()
    }
}
