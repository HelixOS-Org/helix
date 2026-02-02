//! # MMU Abstraction
//!
//! This module defines traits for Memory Management Unit operations.

use bitflags::bitflags;

use crate::{HalResult, PageSize, PhysAddr, VirtAddr};

bitflags! {
    /// Page table entry flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageFlags: u64 {
        /// Page is present in memory
        const PRESENT = 1 << 0;
        /// Page is writable
        const WRITABLE = 1 << 1;
        /// Page is accessible from user mode
        const USER = 1 << 2;
        /// Page is write-through cached
        const WRITE_THROUGH = 1 << 3;
        /// Page caching is disabled
        const NO_CACHE = 1 << 4;
        /// Page has been accessed
        const ACCESSED = 1 << 5;
        /// Page has been written to
        const DIRTY = 1 << 6;
        /// Page is a large/huge page
        const HUGE_PAGE = 1 << 7;
        /// Page is global (not flushed on context switch)
        const GLOBAL = 1 << 8;
        /// Page is not executable (NX bit)
        const NO_EXECUTE = 1 << 63;
    }
}

impl PageFlags {
    /// Create flags for kernel code
    pub const fn kernel_code() -> Self {
        Self::PRESENT.union(Self::GLOBAL)
    }

    /// Create flags for kernel data
    pub const fn kernel_data() -> Self {
        Self::PRESENT
            .union(Self::WRITABLE)
            .union(Self::NO_EXECUTE)
            .union(Self::GLOBAL)
    }

    /// Create flags for kernel read-only data
    pub const fn kernel_rodata() -> Self {
        Self::PRESENT.union(Self::NO_EXECUTE).union(Self::GLOBAL)
    }

    /// Create flags for user code
    pub const fn user_code() -> Self {
        Self::PRESENT.union(Self::USER)
    }

    /// Create flags for user data
    pub const fn user_data() -> Self {
        Self::PRESENT
            .union(Self::WRITABLE)
            .union(Self::USER)
            .union(Self::NO_EXECUTE)
    }

    /// Create flags for user read-only data
    pub const fn user_rodata() -> Self {
        Self::PRESENT.union(Self::USER).union(Self::NO_EXECUTE)
    }
}

/// MMU abstraction trait
pub trait MmuAbstraction: Send + Sync {
    /// Page table type
    type PageTable: PageTable;

    /// Address Space ID type
    type Asid: Copy + Eq + core::fmt::Debug;

    /// Get the page sizes supported by this architecture
    fn supported_page_sizes(&self) -> &[PageSize];

    /// Get the default page size
    fn default_page_size(&self) -> PageSize;

    /// Get the maximum virtual address
    fn max_virtual_address(&self) -> VirtAddr;

    /// Get the maximum physical address
    fn max_physical_address(&self) -> PhysAddr;

    /// Get the number of address space IDs available
    fn max_asid(&self) -> usize;

    /// Allocate a new Address Space ID
    fn allocate_asid(&self) -> HalResult<Self::Asid>;

    /// Free an Address Space ID
    fn free_asid(&self, asid: Self::Asid);

    /// Create a new page table
    fn create_page_table(&self) -> HalResult<Self::PageTable>;

    /// Get the kernel page table
    fn kernel_page_table(&self) -> &Self::PageTable;

    /// Get the current page table
    fn current_page_table(&self) -> &Self::PageTable;

    /// Switch to a different page table
    ///
    /// # Safety
    /// The page table must be valid and properly initialized.
    unsafe fn switch_page_table(&self, table: &Self::PageTable, asid: Self::Asid);

    /// Translate a virtual address to physical
    fn translate(&self, table: &Self::PageTable, virt: VirtAddr) -> Option<PhysAddr>;

    /// Invalidate a TLB entry for a specific address
    fn invalidate_tlb(&self, virt: VirtAddr);

    /// Invalidate all TLB entries
    fn invalidate_tlb_all(&self);

    /// Invalidate TLB entries for a specific ASID
    fn invalidate_tlb_asid(&self, asid: Self::Asid);

    /// Invalidate TLB on all CPUs (for SMP)
    fn invalidate_tlb_broadcast(&self, virt: VirtAddr);
}

/// Page table trait
pub trait PageTable: Send + Sync {
    /// Map a virtual address to a physical address
    ///
    /// # Safety
    /// Incorrect mappings can cause memory corruption.
    unsafe fn map(
        &mut self,
        virt: VirtAddr,
        phys: PhysAddr,
        size: PageSize,
        flags: PageFlags,
    ) -> HalResult<()>;

    /// Map a range of pages
    ///
    /// # Safety
    /// Incorrect mappings can cause memory corruption.
    unsafe fn map_range(
        &mut self,
        virt_start: VirtAddr,
        phys_start: PhysAddr,
        size: usize,
        page_size: PageSize,
        flags: PageFlags,
    ) -> HalResult<()>;

    /// Unmap a virtual address
    fn unmap(&mut self, virt: VirtAddr, size: PageSize) -> HalResult<PhysAddr>;

    /// Unmap a range of pages
    fn unmap_range(
        &mut self,
        virt_start: VirtAddr,
        size: usize,
        page_size: PageSize,
    ) -> HalResult<()>;

    /// Change the flags for a mapped page
    fn update_flags(&mut self, virt: VirtAddr, size: PageSize, flags: PageFlags) -> HalResult<()>;

    /// Query the mapping for a virtual address
    fn query(&self, virt: VirtAddr) -> Option<(PhysAddr, PageSize, PageFlags)>;

    /// Check if an address is mapped
    fn is_mapped(&self, virt: VirtAddr) -> bool {
        self.query(virt).is_some()
    }

    /// Get the physical address of the page table root
    fn root_physical_address(&self) -> PhysAddr;

    /// Clone the page table (for fork-like operations)
    fn clone_table(&self) -> HalResult<Self>
    where
        Self: Sized;
}

/// Memory region descriptor
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    /// Starting physical address
    pub start: PhysAddr,
    /// Size in bytes
    pub size: u64,
    /// Region type
    pub kind: MemoryRegionKind,
}

/// Types of memory regions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionKind {
    /// Usable RAM
    Usable,
    /// Reserved by firmware
    Reserved,
    /// ACPI reclaimable
    AcpiReclaimable,
    /// ACPI NVS
    AcpiNvs,
    /// Bad memory
    BadMemory,
    /// Bootloader reserved
    BootloaderReserved,
    /// Kernel code/data
    Kernel,
    /// Framebuffer
    Framebuffer,
    /// MMIO region
    Mmio,
}
