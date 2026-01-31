//! # AArch64 Memory Mapping
//!
//! This module provides high-level memory mapping utilities for AArch64.

use super::entries::{PageTableEntry, MemoryAttributes, MemoryAttributeIndex};
use super::tables::{PageTable, TranslationLevel, PageTableAllocator};
use super::tlb;
use super::asid::Asid;
use super::{PAGE_SIZE, PAGE_SHIFT, page_align_down, page_align_up};

// =============================================================================
// Map Flags
// =============================================================================

bitflags::bitflags! {
    /// Mapping flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MapFlags: u32 {
        /// Readable
        const READ = 1 << 0;
        /// Writable
        const WRITE = 1 << 1;
        /// Executable
        const EXECUTE = 1 << 2;
        /// User accessible
        const USER = 1 << 3;
        /// Global (not ASID-specific)
        const GLOBAL = 1 << 4;
        /// Device memory (non-cacheable, strongly ordered)
        const DEVICE = 1 << 5;
        /// Write-through caching
        const WRITE_THROUGH = 1 << 6;
        /// Non-cacheable
        const UNCACHED = 1 << 7;
        /// No execute (for data)
        const NO_EXECUTE = 1 << 8;
        /// Kernel mapping
        const KERNEL = 1 << 9;
        /// Huge page (2MB)
        const HUGE_2M = 1 << 10;
        /// Huge page (1GB)
        const HUGE_1G = 1 << 11;

        /// Kernel code: readable, executable
        const KERNEL_CODE = Self::READ.bits() | Self::EXECUTE.bits() | Self::KERNEL.bits() | Self::GLOBAL.bits();
        /// Kernel data: readable, writable
        const KERNEL_DATA = Self::READ.bits() | Self::WRITE.bits() | Self::NO_EXECUTE.bits() | Self::KERNEL.bits() | Self::GLOBAL.bits();
        /// Kernel read-only data
        const KERNEL_RODATA = Self::READ.bits() | Self::NO_EXECUTE.bits() | Self::KERNEL.bits() | Self::GLOBAL.bits();
        /// User code: readable, executable
        const USER_CODE = Self::READ.bits() | Self::EXECUTE.bits() | Self::USER.bits();
        /// User data: readable, writable
        const USER_DATA = Self::READ.bits() | Self::WRITE.bits() | Self::NO_EXECUTE.bits() | Self::USER.bits();
        /// MMIO: device memory for kernel
        const MMIO = Self::READ.bits() | Self::WRITE.bits() | Self::DEVICE.bits() | Self::KERNEL.bits() | Self::GLOBAL.bits();
    }
}

impl MapFlags {
    /// Convert to memory attributes
    pub fn to_memory_attrs(self) -> MemoryAttributes {
        let attr_index = if self.contains(Self::DEVICE) {
            MemoryAttributeIndex::DeviceNGnRnE
        } else if self.contains(Self::UNCACHED) {
            MemoryAttributeIndex::NormalNonCacheable
        } else if self.contains(Self::WRITE_THROUGH) {
            MemoryAttributeIndex::NormalWriteThrough
        } else {
            MemoryAttributeIndex::NormalWriteBack
        };

        use super::entries::{AccessPermission, Shareability};

        let permission = match (self.contains(Self::WRITE), self.contains(Self::USER)) {
            (true, true) => AccessPermission::KernelRwUserRw,
            (true, false) => AccessPermission::KernelRwUserNone,
            (false, true) => AccessPermission::KernelRoUserRo,
            (false, false) => AccessPermission::KernelRoUserNone,
        };

        let shareability = if self.contains(Self::DEVICE) {
            Shareability::NonShareable
        } else {
            Shareability::InnerShareable
        };

        MemoryAttributes {
            attr_index,
            shareability,
            permission,
            uxn: !self.contains(Self::EXECUTE) || self.contains(Self::NO_EXECUTE) || !self.contains(Self::USER),
            pxn: !self.contains(Self::EXECUTE) || self.contains(Self::NO_EXECUTE) || self.contains(Self::USER),
            not_global: !self.contains(Self::GLOBAL),
            access_flag: true,
        }
    }
}

// =============================================================================
// Mapping Error
// =============================================================================

/// Memory mapping error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    /// Virtual address already mapped
    AlreadyMapped,
    /// Out of memory (can't allocate page table)
    OutOfMemory,
    /// Invalid alignment
    InvalidAlignment,
    /// Invalid address
    InvalidAddress,
    /// Region too large
    RegionTooLarge,
    /// Not mapped
    NotMapped,
    /// Cannot unmap (would corrupt tables)
    CannotUnmap,
}

/// Result type for mapping operations
pub type MapResult<T> = Result<T, MapError>;

// =============================================================================
// Virtual Memory Mapper
// =============================================================================

/// Virtual memory mapper for managing page tables
pub struct VirtualMemoryMapper<'a, A: PageTableAllocator> {
    /// Root page table (L0)
    root: &'a mut PageTable,
    /// Page table allocator
    allocator: &'a mut A,
    /// ASID for this address space
    asid: Asid,
}

impl<'a, A: PageTableAllocator> VirtualMemoryMapper<'a, A> {
    /// Create a new mapper
    pub fn new(root: &'a mut PageTable, allocator: &'a mut A, asid: Asid) -> Self {
        Self { root, allocator, asid }
    }

    /// Map a single page
    pub fn map_page(&mut self, va: u64, pa: u64, flags: MapFlags) -> MapResult<()> {
        if !super::tables::is_canonical_va(va) {
            return Err(MapError::InvalidAddress);
        }

        if va & (PAGE_SIZE as u64 - 1) != 0 || pa & (PAGE_SIZE as u64 - 1) != 0 {
            return Err(MapError::InvalidAlignment);
        }

        let attrs = flags.to_memory_attrs();
        self.ensure_mapped(va, pa, attrs, TranslationLevel::L3)
    }

    /// Map a range of pages
    pub fn map_range(&mut self, va_start: u64, pa_start: u64, size: usize, flags: MapFlags) -> MapResult<()> {
        let va_end = va_start + size as u64;
        let mut va = page_align_down(va_start as usize) as u64;
        let mut pa = page_align_down(pa_start as usize) as u64;

        while va < va_end {
            self.map_page(va, pa, flags)?;
            va += PAGE_SIZE as u64;
            pa += PAGE_SIZE as u64;
        }

        Ok(())
    }

    /// Map a 2MB huge page
    pub fn map_huge_2m(&mut self, va: u64, pa: u64, flags: MapFlags) -> MapResult<()> {
        const HUGE_2M: u64 = 2 * 1024 * 1024;

        if va & (HUGE_2M - 1) != 0 || pa & (HUGE_2M - 1) != 0 {
            return Err(MapError::InvalidAlignment);
        }

        let attrs = flags.to_memory_attrs();
        self.ensure_mapped(va, pa, attrs, TranslationLevel::L2)
    }

    /// Map a 1GB huge page
    pub fn map_huge_1g(&mut self, va: u64, pa: u64, flags: MapFlags) -> MapResult<()> {
        const HUGE_1G: u64 = 1024 * 1024 * 1024;

        if va & (HUGE_1G - 1) != 0 || pa & (HUGE_1G - 1) != 0 {
            return Err(MapError::InvalidAlignment);
        }

        let attrs = flags.to_memory_attrs();
        self.ensure_mapped(va, pa, attrs, TranslationLevel::L1)
    }

    /// Unmap a page
    pub fn unmap_page(&mut self, va: u64) -> MapResult<u64> {
        if !super::tables::is_canonical_va(va) {
            return Err(MapError::InvalidAddress);
        }

        let indices = super::tables::va_to_indices(va);

        // Walk to L3
        let l0_entry = self.root[indices[0]];
        if !l0_entry.is_valid() {
            return Err(MapError::NotMapped);
        }

        let l1_table = unsafe { &mut *(l0_entry.phys_addr() as *mut PageTable) };
        let l1_entry = l1_table[indices[1]];
        if !l1_entry.is_valid() {
            return Err(MapError::NotMapped);
        }

        let l2_table = unsafe { &mut *(l1_entry.phys_addr() as *mut PageTable) };
        let l2_entry = l2_table[indices[2]];
        if !l2_entry.is_valid() {
            return Err(MapError::NotMapped);
        }

        let l3_table = unsafe { &mut *(l2_entry.phys_addr() as *mut PageTable) };
        let l3_entry = l3_table[indices[3]];
        if !l3_entry.is_valid() {
            return Err(MapError::NotMapped);
        }

        let pa = l3_entry.phys_addr();
        l3_table[indices[3]] = PageTableEntry::invalid();

        // Invalidate TLB
        tlb::tlb_flush_page_asid(va, self.asid);

        Ok(pa)
    }

    /// Unmap a range
    pub fn unmap_range(&mut self, va_start: u64, size: usize) -> MapResult<()> {
        let va_end = va_start + size as u64;
        let mut va = page_align_down(va_start as usize) as u64;

        while va < va_end {
            let _ = self.unmap_page(va); // Ignore errors for already-unmapped pages
            va += PAGE_SIZE as u64;
        }

        Ok(())
    }

    /// Change page protection flags
    pub fn protect(&mut self, va: u64, flags: MapFlags) -> MapResult<()> {
        if !super::tables::is_canonical_va(va) {
            return Err(MapError::InvalidAddress);
        }

        let indices = super::tables::va_to_indices(va);

        // Walk to L3 and update entry
        let l0_entry = self.root[indices[0]];
        if !l0_entry.is_valid() {
            return Err(MapError::NotMapped);
        }

        let l1_table = unsafe { &mut *(l0_entry.phys_addr() as *mut PageTable) };
        let l1_entry = l1_table[indices[1]];
        if !l1_entry.is_valid() {
            return Err(MapError::NotMapped);
        }

        let l2_table = unsafe { &mut *(l1_entry.phys_addr() as *mut PageTable) };
        let l2_entry = l2_table[indices[2]];
        if !l2_entry.is_valid() {
            return Err(MapError::NotMapped);
        }

        let l3_table = unsafe { &mut *(l2_entry.phys_addr() as *mut PageTable) };
        let l3_entry = l3_table[indices[3]];
        if !l3_entry.is_valid() {
            return Err(MapError::NotMapped);
        }

        // Update with new attributes
        let pa = l3_entry.phys_addr();
        let attrs = flags.to_memory_attrs();
        l3_table[indices[3]] = PageTableEntry::new_page(pa, attrs);

        // Invalidate TLB
        tlb::tlb_flush_page_asid(va, self.asid);

        Ok(())
    }

    /// Translate virtual to physical address
    pub fn translate(&self, va: u64) -> Option<u64> {
        let walker = unsafe { super::tables::PageTableWalker::new(self.root) };
        unsafe { walker.translate(va) }
    }

    /// Ensure mapping exists, creating page tables as needed
    fn ensure_mapped(
        &mut self,
        va: u64,
        pa: u64,
        attrs: MemoryAttributes,
        target_level: TranslationLevel,
    ) -> MapResult<()> {
        let indices = super::tables::va_to_indices(va);

        // L0 -> L1
        let l1_table = self.ensure_table_at(self.root, indices[0])?;

        if target_level == TranslationLevel::L1 {
            // Create 1GB block mapping
            l1_table[indices[1]] = PageTableEntry::from_bits(
                (pa & 0x0000_FFFF_C000_0000) | attrs.to_bits()
            );
            return Ok(());
        }

        // L1 -> L2
        let l2_table = self.ensure_table_at(l1_table, indices[1])?;

        if target_level == TranslationLevel::L2 {
            // Create 2MB block mapping
            l2_table[indices[2]] = PageTableEntry::from_bits(
                (pa & 0x0000_FFFF_FFE0_0000) | attrs.to_bits()
            );
            return Ok(());
        }

        // L2 -> L3
        let l3_table = self.ensure_table_at(l2_table, indices[2])?;

        // L3 entry (4KB page)
        if l3_table[indices[3]].is_valid() {
            return Err(MapError::AlreadyMapped);
        }

        l3_table[indices[3]] = PageTableEntry::new_page(pa, attrs);

        Ok(())
    }

    /// Ensure a table entry exists, allocating if needed
    fn ensure_table_at(&mut self, table: &mut PageTable, index: usize) -> MapResult<&mut PageTable> {
        let entry = table[index];

        if entry.is_valid() && entry.is_table() {
            // Table already exists
            let next_table = entry.phys_addr() as *mut PageTable;
            return Ok(unsafe { &mut *next_table });
        }

        if entry.is_valid() {
            // Entry exists but is a block, can't create table here
            return Err(MapError::AlreadyMapped);
        }

        // Allocate new table
        let new_table = self.allocator.allocate().ok_or(MapError::OutOfMemory)?;

        // Initialize to empty
        unsafe {
            (*new_table).clear();
        }

        // Create table entry
        table[index] = PageTableEntry::new_table(new_table as u64);

        Ok(unsafe { &mut *new_table })
    }
}

// =============================================================================
// Identity Mapping Helper
// =============================================================================

/// Create identity mapping (VA = PA)
pub fn create_identity_mapping<A: PageTableAllocator>(
    mapper: &mut VirtualMemoryMapper<A>,
    start: u64,
    end: u64,
    flags: MapFlags,
) -> MapResult<()> {
    let mut addr = page_align_down(start as usize) as u64;
    let end = page_align_up(end as usize) as u64;

    while addr < end {
        mapper.map_page(addr, addr, flags)?;
        addr += PAGE_SIZE as u64;
    }

    Ok(())
}

/// Create identity mapping with 2MB pages
pub fn create_identity_mapping_2m<A: PageTableAllocator>(
    mapper: &mut VirtualMemoryMapper<A>,
    start: u64,
    end: u64,
    flags: MapFlags,
) -> MapResult<()> {
    const HUGE_2M: u64 = 2 * 1024 * 1024;

    let mut addr = start & !(HUGE_2M - 1);
    let end = (end + HUGE_2M - 1) & !(HUGE_2M - 1);

    while addr < end {
        mapper.map_huge_2m(addr, addr, flags)?;
        addr += HUGE_2M;
    }

    Ok(())
}

// =============================================================================
// Higher-Half Kernel Mapping
// =============================================================================

/// Map kernel at higher half (0xFFFF_0000_0000_0000)
pub fn map_kernel_higher_half<A: PageTableAllocator>(
    mapper: &mut VirtualMemoryMapper<A>,
    phys_start: u64,
    phys_end: u64,
    virt_offset: u64,
    flags: MapFlags,
) -> MapResult<()> {
    let size = phys_end - phys_start;
    mapper.map_range(virt_offset + phys_start, phys_start, size as usize, flags)
}
