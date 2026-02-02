//! Page Table Management
//!
//! x86_64 and aarch64 page table setup for kernel transition.

use crate::error::Result;
use crate::raw::types::*;

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ptr;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Page size (4 KiB)
pub const PAGE_SIZE: u64 = 0x1000;

/// Large page size (2 MiB)
pub const LARGE_PAGE_SIZE: u64 = 0x20_0000;

/// Huge page size (1 GiB)
pub const HUGE_PAGE_SIZE: u64 = 0x4000_0000;

/// Number of entries per page table level
pub const ENTRIES_PER_TABLE: usize = 512;

/// Page table entry present flag
pub const PTE_PRESENT: u64 = 1 << 0;

/// Page table entry writable flag
pub const PTE_WRITABLE: u64 = 1 << 1;

/// Page table entry user accessible flag
pub const PTE_USER: u64 = 1 << 2;

/// Page table entry write-through flag
pub const PTE_WRITE_THROUGH: u64 = 1 << 3;

/// Page table entry cache disable flag
pub const PTE_CACHE_DISABLE: u64 = 1 << 4;

/// Page table entry accessed flag
pub const PTE_ACCESSED: u64 = 1 << 5;

/// Page table entry dirty flag
pub const PTE_DIRTY: u64 = 1 << 6;

/// Page table entry page size flag (for large/huge pages)
pub const PTE_PAGE_SIZE: u64 = 1 << 7;

/// Page table entry global flag
pub const PTE_GLOBAL: u64 = 1 << 8;

/// Page table entry no-execute flag (requires NX bit in EFER)
pub const PTE_NO_EXECUTE: u64 = 1 << 63;

/// Address mask for page table entries
pub const PTE_ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;

// =============================================================================
// PAGE TABLE MANAGER
// =============================================================================

/// Page table manager for boot environment
pub struct PageTableManager {
    /// PML4 table physical address
    pml4_address: PhysicalAddress,
    /// Allocated page tables
    allocated_tables: Vec<PhysicalAddress>,
    /// Whether large pages are used
    use_large_pages: bool,
    /// Whether huge pages are used
    use_huge_pages: bool,
    /// Whether NX bit is available
    nx_available: bool,
    /// Current mappings for tracking
    mappings: Vec<PageMapping>,
}

impl PageTableManager {
    /// Create new page table manager
    pub fn new() -> Self {
        Self {
            pml4_address: PhysicalAddress(0),
            allocated_tables: Vec::new(),
            use_large_pages: true,
            use_huge_pages: false,
            nx_available: false,
            mappings: Vec::new(),
        }
    }

    /// Initialize page tables
    pub unsafe fn init(&mut self) -> Result<()> {
        // Allocate PML4
        self.pml4_address = self.allocate_table()?;

        // Zero the table
        let pml4 = self.pml4_address.0 as *mut PageTableEntry;
        for i in 0..ENTRIES_PER_TABLE {
            ptr::write_volatile(pml4.add(i), PageTableEntry(0));
        }

        Ok(())
    }

    /// Allocate a new page table
    unsafe fn allocate_table(&mut self) -> Result<PhysicalAddress> {
        // In real implementation, this would use UEFI's AllocatePages
        // For now, we track allocations
        let table = Box::new([PageTableEntry(0); ENTRIES_PER_TABLE]);
        let addr = PhysicalAddress(Box::into_raw(table) as u64);
        self.allocated_tables.push(addr);
        Ok(addr)
    }

    /// Get PML4 address
    pub fn pml4_address(&self) -> PhysicalAddress {
        self.pml4_address
    }

    /// Map a page
    pub unsafe fn map_page(
        &mut self,
        virt: VirtualAddress,
        phys: PhysicalAddress,
        flags: PageFlags,
    ) -> Result<()> {
        self.map_page_with_size(virt, phys, PAGE_SIZE, flags)
    }

    /// Map a page with specific size
    pub unsafe fn map_page_with_size(
        &mut self,
        virt: VirtualAddress,
        phys: PhysicalAddress,
        size: u64,
        flags: PageFlags,
    ) -> Result<()> {
        let pml4_idx = (virt >> 39) & 0x1FFu64;
        let pdpt_idx = (virt >> 30) & 0x1FFu64;
        let pd_idx = (virt >> 21) & 0x1FFu64;
        let pt_idx = (virt >> 12) & 0x1FFu64;

        let pte_flags = flags.to_pte_flags();

        // Get or create PDPT
        let pml4 = self.pml4_address.0 as *mut PageTableEntry;
        let pml4_entry = &mut *pml4.add(pml4_idx as usize);

        let pdpt_addr = if pml4_entry.is_present() {
            pml4_entry.address()
        } else {
            let new_table = self.allocate_table()?;
            *pml4_entry = PageTableEntry::new(new_table, PTE_PRESENT | PTE_WRITABLE | PTE_USER);
            new_table
        };

        // 1 GiB huge page?
        if size >= HUGE_PAGE_SIZE && self.use_huge_pages {
            let pdpt = pdpt_addr.0 as *mut PageTableEntry;
            let pdpt_entry = &mut *pdpt.add(pdpt_idx as usize);
            *pdpt_entry = PageTableEntry::new(phys, pte_flags | PTE_PAGE_SIZE);

            self.mappings.push(PageMapping {
                virtual_address: virt,
                physical_address: phys,
                size: HUGE_PAGE_SIZE,
                flags,
            });

            return Ok(());
        }

        // Get or create PD
        let pdpt = pdpt_addr.0 as *mut PageTableEntry;
        let pdpt_entry = &mut *pdpt.add(pdpt_idx as usize);

        let pd_addr = if pdpt_entry.is_present() && !pdpt_entry.is_page_size() {
            pdpt_entry.address()
        } else {
            let new_table = self.allocate_table()?;
            *pdpt_entry = PageTableEntry::new(new_table, PTE_PRESENT | PTE_WRITABLE | PTE_USER);
            new_table
        };

        // 2 MiB large page?
        if size >= LARGE_PAGE_SIZE && self.use_large_pages {
            let pd = pd_addr.0 as *mut PageTableEntry;
            let pd_entry = &mut *pd.add(pd_idx as usize);
            *pd_entry = PageTableEntry::new(phys, pte_flags | PTE_PAGE_SIZE);

            self.mappings.push(PageMapping {
                virtual_address: virt,
                physical_address: phys,
                size: LARGE_PAGE_SIZE,
                flags,
            });

            return Ok(());
        }

        // Get or create PT
        let pd = pd_addr.0 as *mut PageTableEntry;
        let pd_entry = &mut *pd.add(pd_idx as usize);

        let pt_addr = if pd_entry.is_present() && !pd_entry.is_page_size() {
            pd_entry.address()
        } else {
            let new_table = self.allocate_table()?;
            *pd_entry = PageTableEntry::new(new_table, PTE_PRESENT | PTE_WRITABLE | PTE_USER);
            new_table
        };

        // Map 4 KiB page
        let pt = pt_addr.0 as *mut PageTableEntry;
        let pt_entry = &mut *pt.add(pt_idx as usize);
        *pt_entry = PageTableEntry::new(phys, pte_flags);

        self.mappings.push(PageMapping {
            virtual_address: virt,
            physical_address: phys,
            size: PAGE_SIZE,
            flags,
        });

        Ok(())
    }

    /// Map a range of physical memory
    pub unsafe fn map_range(
        &mut self,
        virt_start: VirtualAddress,
        phys_start: PhysicalAddress,
        size: u64,
        flags: PageFlags,
    ) -> Result<()> {
        let mut offset = 0u64;

        while offset < size {
            let remaining = size - offset;
            let virt = virt_start + offset;
            let phys = phys_start + offset;

            // Try to use largest page size possible
            if self.use_huge_pages
                && remaining >= HUGE_PAGE_SIZE
                && (virt & (HUGE_PAGE_SIZE - 1)) == 0
                && (phys & (HUGE_PAGE_SIZE - 1)) == 0
            {
                self.map_page_with_size(virt, phys, HUGE_PAGE_SIZE, flags)?;
                offset += HUGE_PAGE_SIZE;
            } else if self.use_large_pages
                && remaining >= LARGE_PAGE_SIZE
                && (virt & (LARGE_PAGE_SIZE - 1)) == 0
                && (phys & (LARGE_PAGE_SIZE - 1)) == 0
            {
                self.map_page_with_size(virt, phys, LARGE_PAGE_SIZE, flags)?;
                offset += LARGE_PAGE_SIZE;
            } else {
                self.map_page_with_size(virt, phys, PAGE_SIZE, flags)?;
                offset += PAGE_SIZE;
            }
        }

        Ok(())
    }

    /// Map identity (physical = virtual)
    pub unsafe fn map_identity(
        &mut self,
        start: PhysicalAddress,
        size: u64,
        flags: PageFlags,
    ) -> Result<()> {
        self.map_range(VirtualAddress(start.0), start, size, flags)
    }

    /// Unmap a page
    pub unsafe fn unmap_page(&mut self, virt: VirtualAddress) -> Result<()> {
        let pml4_idx = (virt >> 39) & 0x1FFu64;
        let pdpt_idx = (virt >> 30) & 0x1FFu64;
        let pd_idx = (virt >> 21) & 0x1FFu64;
        let pt_idx = (virt >> 12) & 0x1FFu64;

        let pml4 = self.pml4_address.0 as *mut PageTableEntry;
        let pml4_entry = &*pml4.add(pml4_idx as usize);

        if !pml4_entry.is_present() {
            return Ok(());
        }

        let pdpt = pml4_entry.address().0 as *mut PageTableEntry;
        let pdpt_entry = &*pdpt.add(pdpt_idx as usize);

        if !pdpt_entry.is_present() {
            return Ok(());
        }

        if pdpt_entry.is_page_size() {
            // Huge page - clear it
            let pdpt_entry_mut = &mut *pdpt.add(pdpt_idx as usize);
            *pdpt_entry_mut = PageTableEntry(0);
            return Ok(());
        }

        let pd = pdpt_entry.address().0 as *mut PageTableEntry;
        let pd_entry = &*pd.add(pd_idx as usize);

        if !pd_entry.is_present() {
            return Ok(());
        }

        if pd_entry.is_page_size() {
            // Large page - clear it
            let pd_entry_mut = &mut *pd.add(pd_idx as usize);
            *pd_entry_mut = PageTableEntry(0);
            return Ok(());
        }

        let pt = pd_entry.address().0 as *mut PageTableEntry;
        let pt_entry_mut = &mut *pt.add(pt_idx as usize);
        *pt_entry_mut = PageTableEntry(0);

        // Remove from tracking
        self.mappings.retain(|m| m.virtual_address != virt);

        Ok(())
    }

    /// Unmap a range of pages
    pub unsafe fn unmap_range(&mut self, virt: VirtualAddress, size: u64) -> Result<()> {
        let mut offset = 0u64;
        while offset < size {
            let current_virt = VirtualAddress(virt.0 + offset);
            self.unmap_page(current_virt)?;
            offset += PAGE_SIZE;
        }
        Ok(())
    }
    /// Get mapping at virtual address
    pub fn get_mapping(&self, virt: VirtualAddress) -> Option<&PageMapping> {
        self.mappings
            .iter()
            .find(|m| virt >= m.virtual_address && virt < m.virtual_address + m.size)
    }

    /// Get all mappings
    pub fn mappings(&self) -> &[PageMapping] {
        &self.mappings
    }

    /// Get allocated table count
    pub fn table_count(&self) -> usize {
        self.allocated_tables.len()
    }

    /// Set whether to use large pages
    pub fn set_use_large_pages(&mut self, use_large: bool) {
        self.use_large_pages = use_large;
    }

    /// Set whether to use huge pages
    pub fn set_use_huge_pages(&mut self, use_huge: bool) {
        self.use_huge_pages = use_huge;
    }

    /// Set NX availability
    pub fn set_nx_available(&mut self, available: bool) {
        self.nx_available = available;
    }

    /// Translate virtual to physical
    pub unsafe fn translate(&self, virt: VirtualAddress) -> Option<PhysicalAddress> {
        let pml4_idx = (virt >> 39) & 0x1FFu64;
        let pdpt_idx = (virt >> 30) & 0x1FFu64;
        let pd_idx = (virt >> 21) & 0x1FFu64;
        let pt_idx = (virt >> 12) & 0x1FFu64;
        let offset = virt & 0xFFF;

        let pml4 = self.pml4_address.0 as *const PageTableEntry;
        let pml4_entry = &*pml4.add(pml4_idx as usize);

        if !pml4_entry.is_present() {
            return None;
        }

        let pdpt = pml4_entry.address().0 as *const PageTableEntry;
        let pdpt_entry = &*pdpt.add(pdpt_idx as usize);

        if !pdpt_entry.is_present() {
            return None;
        }

        if pdpt_entry.is_page_size() {
            // 1 GiB page
            let offset_1g = virt & (HUGE_PAGE_SIZE - 1);
            return Some(pdpt_entry.address() + offset_1g);
        }

        let pd = pdpt_entry.address().0 as *const PageTableEntry;
        let pd_entry = &*pd.add(pd_idx as usize);

        if !pd_entry.is_present() {
            return None;
        }

        if pd_entry.is_page_size() {
            // 2 MiB page
            let offset_2m = virt & (LARGE_PAGE_SIZE - 1);
            return Some(pd_entry.address() + offset_2m);
        }

        let pt = pd_entry.address().0 as *const PageTableEntry;
        let pt_entry = &*pt.add(pt_idx as usize);

        if !pt_entry.is_present() {
            return None;
        }

        Some(pt_entry.address() + offset)
    }
}

impl Default for PageTableManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// PAGE TABLE ENTRY
// =============================================================================

/// Page table entry
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PageTableEntry(pub u64);

impl PageTableEntry {
    /// Create new entry
    pub fn new(address: PhysicalAddress, flags: u64) -> Self {
        Self((address & PTE_ADDR_MASK) | flags)
    }

    /// Get raw value
    pub fn raw(&self) -> u64 {
        self.0
    }

    /// Check if present
    pub fn is_present(&self) -> bool {
        (self.0 & PTE_PRESENT) != 0
    }

    /// Check if writable
    pub fn is_writable(&self) -> bool {
        (self.0 & PTE_WRITABLE) != 0
    }

    /// Check if user accessible
    pub fn is_user(&self) -> bool {
        (self.0 & PTE_USER) != 0
    }

    /// Check if page size (large/huge page)
    pub fn is_page_size(&self) -> bool {
        (self.0 & PTE_PAGE_SIZE) != 0
    }

    /// Check if no-execute
    pub fn is_no_execute(&self) -> bool {
        (self.0 & PTE_NO_EXECUTE) != 0
    }

    /// Get physical address
    pub fn address(&self) -> PhysicalAddress {
        PhysicalAddress(self.0 & PTE_ADDR_MASK)
    }

    /// Set physical address
    pub fn set_address(&mut self, address: PhysicalAddress) {
        self.0 = (self.0 & !PTE_ADDR_MASK) | (address.0 & PTE_ADDR_MASK);
    }

    /// Set flags
    pub fn set_flags(&mut self, flags: u64) {
        self.0 = (self.0 & PTE_ADDR_MASK) | flags;
    }

    /// Clear entry
    pub fn clear(&mut self) {
        self.0 = 0;
    }
}

// =============================================================================
// PAGE FLAGS
// =============================================================================

/// Page mapping flags
#[derive(Debug, Clone, Copy, Default)]
pub struct PageFlags {
    /// Present
    pub present: bool,
    /// Writable
    pub writable: bool,
    /// User accessible
    pub user: bool,
    /// Write-through caching
    pub write_through: bool,
    /// Cache disabled
    pub cache_disable: bool,
    /// Global page
    pub global: bool,
    /// No-execute
    pub no_execute: bool,
}

impl PageFlags {
    /// Create empty flags (all false)
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create flags for kernel code
    pub fn kernel_code() -> Self {
        Self {
            present: true,
            writable: false,
            user: false,
            write_through: false,
            cache_disable: false,
            global: true,
            no_execute: false,
        }
    }

    /// Create flags for kernel data
    pub fn kernel_data() -> Self {
        Self {
            present: true,
            writable: true,
            user: false,
            write_through: false,
            cache_disable: false,
            global: true,
            no_execute: true,
        }
    }

    /// Create flags for kernel read-only data
    pub fn kernel_rodata() -> Self {
        Self {
            present: true,
            writable: false,
            user: false,
            write_through: false,
            cache_disable: false,
            global: true,
            no_execute: true,
        }
    }

    /// Create flags for device memory (MMIO)
    pub fn device() -> Self {
        Self {
            present: true,
            writable: true,
            user: false,
            write_through: true,
            cache_disable: true,
            global: true,
            no_execute: true,
        }
    }

    /// Create flags for user code
    pub fn user_code() -> Self {
        Self {
            present: true,
            writable: false,
            user: true,
            write_through: false,
            cache_disable: false,
            global: false,
            no_execute: false,
        }
    }

    /// Create flags for user data
    pub fn user_data() -> Self {
        Self {
            present: true,
            writable: true,
            user: true,
            write_through: false,
            cache_disable: false,
            global: false,
            no_execute: true,
        }
    }

    /// Convert to PTE flags
    pub fn to_pte_flags(&self) -> u64 {
        let mut flags = 0u64;
        if self.present {
            flags |= PTE_PRESENT;
        }
        if self.writable {
            flags |= PTE_WRITABLE;
        }
        if self.user {
            flags |= PTE_USER;
        }
        if self.write_through {
            flags |= PTE_WRITE_THROUGH;
        }
        if self.cache_disable {
            flags |= PTE_CACHE_DISABLE;
        }
        if self.global {
            flags |= PTE_GLOBAL;
        }
        if self.no_execute {
            flags |= PTE_NO_EXECUTE;
        }
        flags
    }
}

// =============================================================================
// PAGE MAPPING
// =============================================================================

/// Tracked page mapping
#[derive(Debug, Clone)]
pub struct PageMapping {
    /// Virtual address
    pub virtual_address: VirtualAddress,
    /// Physical address
    pub physical_address: PhysicalAddress,
    /// Mapping size
    pub size: u64,
    /// Flags
    pub flags: PageFlags,
}

impl PageMapping {
    /// Get end virtual address
    pub fn virtual_end(&self) -> VirtualAddress {
        self.virtual_address + self.size
    }

    /// Get end physical address
    pub fn physical_end(&self) -> PhysicalAddress {
        self.physical_address + self.size
    }

    /// Check if contains virtual address
    pub fn contains_virtual(&self, addr: VirtualAddress) -> bool {
        addr >= self.virtual_address && addr < self.virtual_end()
    }

    /// Check if contains physical address
    pub fn contains_physical(&self, addr: PhysicalAddress) -> bool {
        addr >= self.physical_address && addr < self.physical_end()
    }
}

// =============================================================================
// VIRTUAL ADDRESS SPACE LAYOUT
// =============================================================================

/// Virtual address space layout for x86_64
pub mod layout_x86_64 {
    use super::*;

    /// User space start
    pub const USER_START: VirtualAddress = VirtualAddress(0x0000_0000_0000_0000);

    /// User space end (canonical hole start)
    pub const USER_END: VirtualAddress = VirtualAddress(0x0000_7FFF_FFFF_FFFF);

    /// Kernel space start (after canonical hole)
    pub const KERNEL_START: VirtualAddress = VirtualAddress(0xFFFF_8000_0000_0000);

    /// Kernel space end
    pub const KERNEL_END: VirtualAddress = VirtualAddress(0xFFFF_FFFF_FFFF_FFFF);

    /// Direct physical memory map base
    pub const PHYS_MAP_BASE: VirtualAddress = VirtualAddress(0xFFFF_8800_0000_0000);

    /// Direct physical memory map size (512 GB)
    pub const PHYS_MAP_SIZE: u64 = 512 * 1024 * 1024 * 1024;

    /// Kernel image base
    pub const KERNEL_IMAGE_BASE: VirtualAddress = VirtualAddress(0xFFFF_FFFF_8000_0000);

    /// Kernel stack top
    pub const KERNEL_STACK_TOP: VirtualAddress = VirtualAddress(0xFFFF_FFFF_FF00_0000);

    /// Kernel heap base
    pub const KERNEL_HEAP_BASE: VirtualAddress = VirtualAddress(0xFFFF_C000_0000_0000);

    /// MMIO mapping base
    pub const MMIO_BASE: VirtualAddress = VirtualAddress(0xFFFF_A000_0000_0000);

    /// Check if address is in user space
    pub const fn is_user_address(addr: VirtualAddress) -> bool {
        addr.0 <= USER_END.0
    }

    /// Check if address is in kernel space
    pub const fn is_kernel_address(addr: VirtualAddress) -> bool {
        addr.0 >= KERNEL_START.0
    }

    /// Check if address is canonical
    pub const fn is_canonical(addr: VirtualAddress) -> bool {
        is_user_address(addr) || is_kernel_address(addr)
    }

    /// Convert physical to virtual (direct map)
    pub const fn phys_to_virt(phys: PhysicalAddress) -> VirtualAddress {
        VirtualAddress(PHYS_MAP_BASE.0 + phys.0)
    }

    /// Convert virtual (direct map) to physical
    pub const fn virt_to_phys(virt: VirtualAddress) -> PhysicalAddress {
        PhysicalAddress(virt.0 - PHYS_MAP_BASE.0)
    }
}

// =============================================================================
// PAGE TABLE BUILDER
// =============================================================================

/// Builder for setting up page tables
pub struct PageTableBuilder {
    /// Page table manager
    manager: PageTableManager,
    /// Identity map lower memory
    identity_lower: bool,
    /// Direct physical memory map
    direct_map: bool,
    /// Direct map base address
    direct_map_base: VirtualAddress,
    /// Kernel mappings
    kernel_mappings: Vec<(VirtualAddress, PhysicalAddress, u64, PageFlags)>,
}

impl PageTableBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            manager: PageTableManager::new(),
            identity_lower: false,
            direct_map: false,
            direct_map_base: VirtualAddress(0),
            kernel_mappings: Vec::new(),
        }
    }

    /// Enable identity mapping of lower memory
    pub fn identity_map_lower(mut self, enabled: bool) -> Self {
        self.identity_lower = enabled;
        self
    }

    /// Enable direct physical memory map
    pub fn direct_physical_map(mut self, base: VirtualAddress) -> Self {
        self.direct_map = true;
        self.direct_map_base = base;
        self
    }

    /// Add kernel mapping
    pub fn map_kernel(
        mut self,
        virt: VirtualAddress,
        phys: PhysicalAddress,
        size: u64,
        flags: PageFlags,
    ) -> Self {
        self.kernel_mappings.push((virt, phys, size, flags));
        self
    }

    /// Use large pages
    pub fn use_large_pages(mut self, enabled: bool) -> Self {
        self.manager.set_use_large_pages(enabled);
        self
    }

    /// Use huge pages
    pub fn use_huge_pages(mut self, enabled: bool) -> Self {
        self.manager.set_use_huge_pages(enabled);
        self
    }

    /// Build page tables
    pub unsafe fn build(mut self, memory_size: u64) -> Result<PageTableManager> {
        self.manager.init()?;

        // Identity map lower memory if requested
        if self.identity_lower {
            // Map first 4 GB identity
            let lower_size = memory_size.min(4 * 1024 * 1024 * 1024);
            self.manager
                .map_identity(PhysicalAddress(0), lower_size, PageFlags::kernel_data())?;
        }

        // Create direct physical memory map if requested
        if self.direct_map {
            self.manager.map_range(
                self.direct_map_base,
                PhysicalAddress(0),
                memory_size,
                PageFlags::kernel_data(),
            )?;
        }

        // Map kernel
        for (virt, phys, size, flags) in self.kernel_mappings {
            self.manager.map_range(virt, phys, size, flags)?;
        }

        Ok(self.manager)
    }
}

impl Default for PageTableBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_table_entry() {
        let entry = PageTableEntry::new(0x1000, PTE_PRESENT | PTE_WRITABLE);
        assert!(entry.is_present());
        assert!(entry.is_writable());
        assert!(!entry.is_user());
        assert_eq!(entry.address(), 0x1000);
    }

    #[test]
    fn test_page_flags() {
        let flags = PageFlags::kernel_code();
        assert!(flags.present);
        assert!(!flags.writable);
        assert!(!flags.no_execute);

        let pte = flags.to_pte_flags();
        assert_eq!(pte & PTE_PRESENT, PTE_PRESENT);
        assert_eq!(pte & PTE_WRITABLE, 0);
    }

    #[test]
    fn test_virtual_layout() {
        assert!(layout_x86_64::is_user_address(0x1000));
        assert!(!layout_x86_64::is_user_address(0xFFFF_8000_0000_0000));
        assert!(layout_x86_64::is_kernel_address(0xFFFF_8000_0000_0000));
        assert!(layout_x86_64::is_canonical(0x1000));
        assert!(layout_x86_64::is_canonical(0xFFFF_FFFF_8000_0000));
    }
}
