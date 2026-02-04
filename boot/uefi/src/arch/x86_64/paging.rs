//! x86_64 Paging
//!
//! 4-level and 5-level page table implementation.

use core::ops::{Index, IndexMut};

use crate::error::{Error, Result};
use crate::raw::types::*;

// =============================================================================
// PAGE TABLE CONSTANTS
// =============================================================================

/// Page size (4 KiB)
pub const PAGE_SIZE: u64 = 4096;

/// Large page size (2 MiB)
pub const LARGE_PAGE_SIZE: u64 = 2 * 1024 * 1024;

/// Huge page size (1 GiB)
pub const HUGE_PAGE_SIZE: u64 = 1024 * 1024 * 1024;

/// Entries per page table
pub const ENTRIES_PER_TABLE: usize = 512;

/// Page table entry flags
pub mod flags {
    /// Present
    pub const PRESENT: u64 = 1 << 0;
    /// Writable
    pub const WRITABLE: u64 = 1 << 1;
    /// User accessible
    pub const USER: u64 = 1 << 2;
    /// Write-through
    pub const WRITE_THROUGH: u64 = 1 << 3;
    /// Cache disable
    pub const CACHE_DISABLE: u64 = 1 << 4;
    /// Accessed
    pub const ACCESSED: u64 = 1 << 5;
    /// Dirty (only in leaf entries)
    pub const DIRTY: u64 = 1 << 6;
    /// Huge/large page (PS bit)
    pub const HUGE_PAGE: u64 = 1 << 7;
    /// Global (not flushed on CR3 reload)
    pub const GLOBAL: u64 = 1 << 8;
    /// Available bit 1
    pub const AVAILABLE_1: u64 = 1 << 9;
    /// Available bit 2
    pub const AVAILABLE_2: u64 = 1 << 10;
    /// Available bit 3
    pub const AVAILABLE_3: u64 = 1 << 11;
    /// PAT bit for 4K pages
    pub const PAT_4K: u64 = 1 << 7;
    /// PAT bit for large pages
    pub const PAT_LARGE: u64 = 1 << 12;
    /// No execute
    pub const NO_EXECUTE: u64 = 1 << 63;

    /// Address mask (bits 12-51)
    pub const ADDRESS_MASK: u64 = 0x000F_FFFF_FFFF_F000;

    /// Common flag combinations
    pub const KERNEL_CODE: u64 = PRESENT | GLOBAL;
    pub const KERNEL_DATA: u64 = PRESENT | WRITABLE | GLOBAL | NO_EXECUTE;
    pub const KERNEL_RODATA: u64 = PRESENT | GLOBAL | NO_EXECUTE;
    pub const USER_CODE: u64 = PRESENT | USER;
    pub const USER_DATA: u64 = PRESENT | WRITABLE | USER | NO_EXECUTE;
    pub const MMIO: u64 = PRESENT | WRITABLE | CACHE_DISABLE | NO_EXECUTE;
}

// =============================================================================
// PAGE TABLE ENTRY
// =============================================================================

/// Page table entry
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    /// Create empty (not present) entry
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create entry with address and flags
    pub const fn new(addr: PhysicalAddress, entry_flags: u64) -> Self {
        Self((addr.0 & flags::ADDRESS_MASK) | entry_flags)
    }

    /// Create entry pointing to next table
    pub const fn table(addr: PhysicalAddress) -> Self {
        Self::new(addr, flags::PRESENT | flags::WRITABLE | flags::USER)
    }

    /// Create 4K page mapping
    pub const fn page(addr: PhysicalAddress, entry_flags: u64) -> Self {
        Self::new(addr, entry_flags | flags::PRESENT)
    }

    /// Create 2M large page mapping
    pub const fn large_page(addr: PhysicalAddress, entry_flags: u64) -> Self {
        Self::new(addr, entry_flags | flags::PRESENT | flags::HUGE_PAGE)
    }

    /// Create 1G huge page mapping
    pub const fn huge_page(addr: PhysicalAddress, entry_flags: u64) -> Self {
        Self::new(addr, entry_flags | flags::PRESENT | flags::HUGE_PAGE)
    }

    /// Get raw value
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Set raw value
    pub fn set_raw(&mut self, value: u64) {
        self.0 = value;
    }

    /// Check if present
    pub const fn is_present(&self) -> bool {
        (self.0 & flags::PRESENT) != 0
    }

    /// Check if writable
    pub const fn is_writable(&self) -> bool {
        (self.0 & flags::WRITABLE) != 0
    }

    /// Check if user accessible
    pub const fn is_user(&self) -> bool {
        (self.0 & flags::USER) != 0
    }

    /// Check if huge/large page
    pub const fn is_huge(&self) -> bool {
        (self.0 & flags::HUGE_PAGE) != 0
    }

    /// Check if global
    pub const fn is_global(&self) -> bool {
        (self.0 & flags::GLOBAL) != 0
    }

    /// Check if accessed
    pub const fn is_accessed(&self) -> bool {
        (self.0 & flags::ACCESSED) != 0
    }

    /// Check if dirty
    pub const fn is_dirty(&self) -> bool {
        (self.0 & flags::DIRTY) != 0
    }

    /// Check if no-execute
    pub const fn is_no_execute(&self) -> bool {
        (self.0 & flags::NO_EXECUTE) != 0
    }

    /// Get physical address
    pub const fn address(&self) -> PhysicalAddress {
        PhysicalAddress(self.0 & flags::ADDRESS_MASK)
    }

    /// Set address
    pub fn set_address(&mut self, addr: PhysicalAddress) {
        self.0 = (self.0 & !flags::ADDRESS_MASK) | (addr.0 & flags::ADDRESS_MASK);
    }

    /// Get flags
    pub const fn flags(&self) -> u64 {
        self.0 & !flags::ADDRESS_MASK
    }

    /// Set flags
    pub fn set_flags(&mut self, entry_flags: u64) {
        self.0 = (self.0 & flags::ADDRESS_MASK) | (entry_flags & !flags::ADDRESS_MASK);
    }

    /// Add flags
    pub fn add_flags(&mut self, entry_flags: u64) {
        self.0 |= entry_flags & !flags::ADDRESS_MASK;
    }

    /// Remove flags
    pub fn remove_flags(&mut self, entry_flags: u64) {
        self.0 &= !(entry_flags & !flags::ADDRESS_MASK);
    }

    /// Clear entry
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// Clear accessed flag
    pub fn clear_accessed(&mut self) {
        self.0 &= !flags::ACCESSED;
    }

    /// Clear dirty flag
    pub fn clear_dirty(&mut self) {
        self.0 &= !flags::DIRTY;
    }
}

impl Default for PageTableEntry {
    fn default() -> Self {
        Self::empty()
    }
}

impl core::fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if !self.is_present() {
            write!(f, "PageTableEntry(not present)")
        } else {
            write!(f, "PageTableEntry(addr={:#x}, ", self.address().0)?;
            if self.is_writable() {
                write!(f, "W")?;
            } else {
                write!(f, "-")?;
            }
            if self.is_user() {
                write!(f, "U")?;
            } else {
                write!(f, "-")?;
            }
            if self.is_huge() {
                write!(f, "H")?;
            } else {
                write!(f, "-")?;
            }
            if self.is_global() {
                write!(f, "G")?;
            } else {
                write!(f, "-")?;
            }
            if self.is_no_execute() {
                write!(f, "X")?;
            } else {
                write!(f, "-")?;
            }
            write!(f, ")")
        }
    }
}

// =============================================================================
// PAGE TABLE
// =============================================================================

/// Page table (512 entries, 4KiB aligned)
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; ENTRIES_PER_TABLE],
}

impl PageTable {
    /// Create empty page table
    pub const fn empty() -> Self {
        Self {
            entries: [PageTableEntry::empty(); ENTRIES_PER_TABLE],
        }
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            entry.clear();
        }
    }

    /// Get entry at index
    pub fn entry(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }

    /// Get entry at index (mutable)
    pub fn entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }

    /// Iterate entries
    pub fn iter(&self) -> impl Iterator<Item = &PageTableEntry> {
        self.entries.iter()
    }

    /// Iterate entries (mutable)
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PageTableEntry> {
        self.entries.iter_mut()
    }

    /// Iterate present entries with index
    pub fn present_entries(&self) -> impl Iterator<Item = (usize, &PageTableEntry)> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_present())
    }

    /// Count present entries
    pub fn present_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_present()).count()
    }

    /// Check if empty (no present entries)
    pub fn is_empty(&self) -> bool {
        self.present_count() == 0
    }
}

impl Index<usize> for PageTable {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl Default for PageTable {
    fn default() -> Self {
        Self::empty()
    }
}

// =============================================================================
// VIRTUAL ADDRESS DECOMPOSITION
// =============================================================================

/// Virtual address components for 4-level paging
#[derive(Debug, Clone, Copy)]
pub struct VirtualAddressComponents {
    /// PML4 index (bits 39-47)
    pub pml4: usize,
    /// PDPT index (bits 30-38)
    pub pdpt: usize,
    /// PD index (bits 21-29)
    pub pd: usize,
    /// PT index (bits 12-20)
    pub pt: usize,
    /// Page offset (bits 0-11)
    pub offset: usize,
}

impl VirtualAddressComponents {
    /// Decompose virtual address
    pub fn from_address(addr: VirtualAddress) -> Self {
        let addr = addr.0;
        Self {
            pml4: ((addr >> 39) & 0x1FFu64) as usize,
            pdpt: ((addr >> 30) & 0x1FFu64) as usize,
            pd: ((addr >> 21) & 0x1FFu64) as usize,
            pt: ((addr >> 12) & 0x1FFu64) as usize,
            offset: (addr & 0xFFFu64) as usize,
        }
    }

    /// Compose virtual address
    pub fn to_address(&self) -> VirtualAddress {
        let mut addr = 0u64;
        addr |= (self.pml4 as u64) << 39;
        addr |= (self.pdpt as u64) << 30;
        addr |= (self.pd as u64) << 21;
        addr |= (self.pt as u64) << 12;
        addr |= self.offset as u64;

        // Sign extend for canonical address
        if (addr & (1 << 47)) != 0 {
            addr |= 0xFFFF_0000_0000_0000;
        }

        VirtualAddress(addr)
    }
}

/// Virtual address components for 5-level paging
#[derive(Debug, Clone, Copy)]
pub struct VirtualAddressComponents5 {
    /// PML5 index (bits 48-56)
    pub pml5: usize,
    /// PML4 index (bits 39-47)
    pub pml4: usize,
    /// PDPT index (bits 30-38)
    pub pdpt: usize,
    /// PD index (bits 21-29)
    pub pd: usize,
    /// PT index (bits 12-20)
    pub pt: usize,
    /// Page offset (bits 0-11)
    pub offset: usize,
}

impl VirtualAddressComponents5 {
    /// Decompose virtual address
    pub fn from_address(addr: VirtualAddress) -> Self {
        let addr = addr.0;
        Self {
            pml5: ((addr >> 48) & 0x1FFu64) as usize,
            pml4: ((addr >> 39) & 0x1FFu64) as usize,
            pdpt: ((addr >> 30) & 0x1FFu64) as usize,
            pd: ((addr >> 21) & 0x1FFu64) as usize,
            pt: ((addr >> 12) & 0x1FFu64) as usize,
            offset: (addr & 0xFFFu64) as usize,
        }
    }
}

// =============================================================================
// PAGE TABLE WALKER
// =============================================================================

/// Page table walker for address translation
pub struct PageTableWalker {
    /// Physical to virtual offset
    phys_offset: u64,
    /// Use 5-level paging
    level5: bool,
}

impl PageTableWalker {
    /// Create new walker with physical memory offset
    pub fn new(phys_offset: u64) -> Self {
        Self {
            phys_offset,
            level5: false,
        }
    }

    /// Create walker for 5-level paging
    pub fn new_level5(phys_offset: u64) -> Self {
        Self {
            phys_offset,
            level5: true,
        }
    }

    /// Get page table at physical address
    unsafe fn get_table(&self, phys: PhysicalAddress) -> &'static PageTable {
        let virt = phys + self.phys_offset;
        &*(virt.0 as *const PageTable)
    }

    /// Get page table at physical address (mutable)
    unsafe fn get_table_mut(&self, phys: PhysicalAddress) -> &'static mut PageTable {
        let virt = phys + self.phys_offset;
        &mut *(virt.0 as *mut PageTable)
    }

    /// Translate virtual to physical address
    pub fn translate(
        &self,
        pml4: PhysicalAddress,
        virt: VirtualAddress,
    ) -> Option<PhysicalAddress> {
        let components = VirtualAddressComponents::from_address(virt);

        // Walk PML4
        let pml4_table = unsafe { self.get_table(pml4) };
        let pml4_entry = &pml4_table[components.pml4];
        if !pml4_entry.is_present() {
            return None;
        }

        // Walk PDPT
        let pdpt_table = unsafe { self.get_table(pml4_entry.address()) };
        let pdpt_entry = &pdpt_table[components.pdpt];
        if !pdpt_entry.is_present() {
            return None;
        }
        if pdpt_entry.is_huge() {
            // 1GB page
            return Some(pdpt_entry.address() + (virt & (HUGE_PAGE_SIZE - 1)));
        }

        // Walk PD
        let pd_table = unsafe { self.get_table(pdpt_entry.address()) };
        let pd_entry = &pd_table[components.pd];
        if !pd_entry.is_present() {
            return None;
        }
        if pd_entry.is_huge() {
            // 2MB page
            return Some(pd_entry.address() + (virt & (LARGE_PAGE_SIZE - 1)));
        }

        // Walk PT
        let pt_table = unsafe { self.get_table(pd_entry.address()) };
        let pt_entry = &pt_table[components.pt];
        if !pt_entry.is_present() {
            return None;
        }

        // 4KB page
        Some(pt_entry.address() + components.offset as u64)
    }
}

// =============================================================================
// PAGE TABLE BUILDER
// =============================================================================

/// Page table allocator trait
pub trait PageTableAllocator {
    /// Allocate a page table (returns physical address)
    fn allocate(&mut self) -> Result<PhysicalAddress>;

    /// Deallocate a page table
    fn deallocate(&mut self, addr: PhysicalAddress);
}

/// Page table builder
pub struct PageTableBuilder<A: PageTableAllocator> {
    /// Page table allocator
    allocator: A,
    /// Physical to virtual offset
    phys_offset: u64,
    /// Root table (PML4) address
    root: PhysicalAddress,
    /// Use 5-level paging
    level5: bool,
}

impl<A: PageTableAllocator> PageTableBuilder<A> {
    /// Create new builder
    pub fn new(mut allocator: A, phys_offset: u64) -> Result<Self> {
        let root = allocator.allocate()?;

        // Clear root table
        let root_table = unsafe { &mut *((root + phys_offset).0 as *mut PageTable) };
        root_table.clear();

        Ok(Self {
            allocator,
            phys_offset,
            root,
            level5: false,
        })
    }

    /// Get root table address
    pub fn root(&self) -> PhysicalAddress {
        self.root
    }

    /// Get page table at physical address (mutable)
    unsafe fn get_table_mut(&self, phys: PhysicalAddress) -> &'static mut PageTable {
        let virt = phys + self.phys_offset;
        &mut *(virt.0 as *mut PageTable)
    }

    /// Ensure table exists at entry, creating if needed
    fn ensure_table(&mut self, entry: &mut PageTableEntry) -> Result<PhysicalAddress> {
        if entry.is_present() {
            Ok(entry.address())
        } else {
            let new_table = self.allocator.allocate()?;

            // Clear new table
            let table = unsafe { self.get_table_mut(new_table) };
            table.clear();

            // Set entry to point to new table
            *entry = PageTableEntry::table(new_table);

            Ok(new_table)
        }
    }

    /// Map 4KB page
    pub fn map_4k(
        &mut self,
        virt: VirtualAddress,
        phys: PhysicalAddress,
        entry_flags: u64,
    ) -> Result<()> {
        let components = VirtualAddressComponents::from_address(virt);

        // Get/create tables
        let pml4 = unsafe { self.get_table_mut(self.root) };
        let pdpt_addr = self.ensure_table(&mut pml4[components.pml4])?;

        let pdpt = unsafe { self.get_table_mut(pdpt_addr) };
        let pd_addr = self.ensure_table(&mut pdpt[components.pdpt])?;

        let pd = unsafe { self.get_table_mut(pd_addr) };
        let pt_addr = self.ensure_table(&mut pd[components.pd])?;

        let pt = unsafe { self.get_table_mut(pt_addr) };
        pt[components.pt] = PageTableEntry::page(phys, entry_flags);

        Ok(())
    }

    /// Map 2MB large page
    pub fn map_2m(
        &mut self,
        virt: VirtualAddress,
        phys: PhysicalAddress,
        entry_flags: u64,
    ) -> Result<()> {
        let components = VirtualAddressComponents::from_address(virt);

        // Ensure alignment
        if (virt & (LARGE_PAGE_SIZE - 1)) != 0 || (phys & (LARGE_PAGE_SIZE - 1)) != 0 {
            return Err(Error::InvalidParameter);
        }

        // Get/create tables
        let pml4 = unsafe { self.get_table_mut(self.root) };
        let pdpt_addr = self.ensure_table(&mut pml4[components.pml4])?;

        let pdpt = unsafe { self.get_table_mut(pdpt_addr) };
        let pd_addr = self.ensure_table(&mut pdpt[components.pdpt])?;

        let pd = unsafe { self.get_table_mut(pd_addr) };
        pd[components.pd] = PageTableEntry::large_page(phys, entry_flags);

        Ok(())
    }

    /// Map 1GB huge page
    pub fn map_1g(
        &mut self,
        virt: VirtualAddress,
        phys: PhysicalAddress,
        entry_flags: u64,
    ) -> Result<()> {
        let components = VirtualAddressComponents::from_address(virt);

        // Ensure alignment
        if (virt & (HUGE_PAGE_SIZE - 1)) != 0 || (phys & (HUGE_PAGE_SIZE - 1)) != 0 {
            return Err(Error::InvalidParameter);
        }

        // Get/create tables
        let pml4 = unsafe { self.get_table_mut(self.root) };
        let pdpt_addr = self.ensure_table(&mut pml4[components.pml4])?;

        let pdpt = unsafe { self.get_table_mut(pdpt_addr) };
        pdpt[components.pdpt] = PageTableEntry::huge_page(phys, entry_flags);

        Ok(())
    }

    /// Map range with appropriate page sizes
    pub fn map_range(
        &mut self,
        virt: VirtualAddress,
        phys: PhysicalAddress,
        size: u64,
        entry_flags: u64,
    ) -> Result<()> {
        let mut virt = virt;
        let mut phys = phys;
        let mut remaining = size;

        while remaining > 0 {
            // Try 1GB page
            if remaining >= HUGE_PAGE_SIZE
                && (virt & (HUGE_PAGE_SIZE - 1)) == 0
                && (phys & (HUGE_PAGE_SIZE - 1)) == 0
            {
                self.map_1g(virt, phys, entry_flags)?;
                virt += HUGE_PAGE_SIZE;
                phys += HUGE_PAGE_SIZE;
                remaining -= HUGE_PAGE_SIZE;
            }
            // Try 2MB page
            else if remaining >= LARGE_PAGE_SIZE
                && (virt & (LARGE_PAGE_SIZE - 1)) == 0
                && (phys & (LARGE_PAGE_SIZE - 1)) == 0
            {
                self.map_2m(virt, phys, entry_flags)?;
                virt += LARGE_PAGE_SIZE;
                phys += LARGE_PAGE_SIZE;
                remaining -= LARGE_PAGE_SIZE;
            }
            // Use 4KB page
            else {
                self.map_4k(virt, phys, entry_flags)?;
                virt += PAGE_SIZE;
                phys += PAGE_SIZE;
                remaining = remaining.saturating_sub(PAGE_SIZE);
            }
        }

        Ok(())
    }

    /// Identity map range
    pub fn identity_map(
        &mut self,
        phys: PhysicalAddress,
        size: u64,
        entry_flags: u64,
    ) -> Result<()> {
        self.map_range(VirtualAddress(phys.0), phys, size, entry_flags)
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_page_table_entry_size() {
        assert_eq!(core::mem::size_of::<PageTableEntry>(), 8);
    }

    #[test]
    fn test_page_table_size() {
        assert_eq!(core::mem::size_of::<PageTable>(), 4096);
    }

    #[test]
    fn test_entry_creation() {
        let entry = PageTableEntry::page(0x1000, flags::WRITABLE | flags::USER);
        assert!(entry.is_present());
        assert!(entry.is_writable());
        assert!(entry.is_user());
        assert!(!entry.is_huge());
        assert_eq!(entry.address(), 0x1000);
    }

    #[test]
    fn test_large_page() {
        let entry = PageTableEntry::large_page(0x200000, flags::WRITABLE);
        assert!(entry.is_present());
        assert!(entry.is_huge());
        assert_eq!(entry.address(), 0x200000);
    }

    #[test]
    fn test_virtual_address_decomposition() {
        let addr = 0xFFFF_8000_0010_1234u64;
        let components = VirtualAddressComponents::from_address(addr);

        assert_eq!(components.offset, 0x234);
        assert_eq!(components.pt, 0x101);
        // Verify reconstruction
        let reconstructed = components.to_address();
        assert_eq!(
            reconstructed & 0xFFFF_FFFF_FFFF_F000,
            addr & 0xFFFF_FFFF_FFFF_F000
        );
    }
}
