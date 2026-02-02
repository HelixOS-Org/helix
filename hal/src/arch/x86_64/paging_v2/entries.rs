//! # Page Table Entry
//!
//! This module defines the 64-bit page table entry format and flags.
//!
//! ## Page Table Entry Format (64 bits)
//!
//! ```text
//! Bits      Field                   Description
//! ─────────────────────────────────────────────────────────────
//! 0         Present (P)             Entry is valid
//! 1         Read/Write (R/W)        Page is writable
//! 2         User/Supervisor (U/S)   Page is accessible from user mode
//! 3         Page-Write Through (PWT) Write-through caching
//! 4         Page Cache Disable (PCD) Disable caching
//! 5         Accessed (A)            Page was accessed
//! 6         Dirty (D)               Page was written
//! 7         Page Size (PS)          Huge page (2MB/1GB) if set
//! 8         Global (G)              Global page (not flushed on CR3 write)
//! 9-11      Available               Available for OS use
//! 12-51     Physical Address        Physical frame address (40 bits)
//! 52-58     Available               Available for OS use
//! 59-62     Protection Key          Memory protection key (if PKU enabled)
//! 63        No Execute (NX)         Execute-disable if NXE in EFER
//! ```

use core::fmt;

use bitflags::bitflags;

use super::addresses::PhysicalAddress;
use super::PHYS_ADDR_MASK;

// =============================================================================
// Page Table Level
// =============================================================================

/// Page table level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum PageTableLevel {
    /// Page Table (PT) - Level 1
    Pt   = 1,
    /// Page Directory (PD) - Level 2
    Pd   = 2,
    /// Page Directory Pointer Table (PDPT) - Level 3
    Pdpt = 3,
    /// Page Map Level 4 (PML4) - Level 4
    Pml4 = 4,
    /// Page Map Level 5 (PML5) - Level 5 (LA57)
    Pml5 = 5,
}

impl PageTableLevel {
    /// Get the next lower level
    #[inline]
    pub const fn next_lower(self) -> Option<Self> {
        match self {
            PageTableLevel::Pml5 => Some(PageTableLevel::Pml4),
            PageTableLevel::Pml4 => Some(PageTableLevel::Pdpt),
            PageTableLevel::Pdpt => Some(PageTableLevel::Pd),
            PageTableLevel::Pd => Some(PageTableLevel::Pt),
            PageTableLevel::Pt => None,
        }
    }

    /// Get the next higher level
    #[inline]
    pub const fn next_higher(self) -> Option<Self> {
        match self {
            PageTableLevel::Pt => Some(PageTableLevel::Pd),
            PageTableLevel::Pd => Some(PageTableLevel::Pdpt),
            PageTableLevel::Pdpt => Some(PageTableLevel::Pml4),
            PageTableLevel::Pml4 => Some(PageTableLevel::Pml5),
            PageTableLevel::Pml5 => None,
        }
    }

    /// Check if this level can have huge pages
    #[inline]
    pub const fn can_have_huge_pages(self) -> bool {
        matches!(self, PageTableLevel::Pd | PageTableLevel::Pdpt)
    }

    /// Get the page size for huge pages at this level
    #[inline]
    pub const fn huge_page_size(self) -> Option<usize> {
        match self {
            PageTableLevel::Pd => Some(super::PAGE_SIZE_2M),
            PageTableLevel::Pdpt => Some(super::PAGE_SIZE_1G),
            _ => None,
        }
    }

    /// Get the shift amount for address extraction
    #[inline]
    pub const fn address_shift(self) -> u8 {
        12 + (self as u8 - 1) * 9
    }

    /// Get the level name
    #[inline]
    pub const fn name(self) -> &'static str {
        match self {
            PageTableLevel::Pt => "PT",
            PageTableLevel::Pd => "PD",
            PageTableLevel::Pdpt => "PDPT",
            PageTableLevel::Pml4 => "PML4",
            PageTableLevel::Pml5 => "PML5",
        }
    }
}

impl TryFrom<u8> for PageTableLevel {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(PageTableLevel::Pt),
            2 => Ok(PageTableLevel::Pd),
            3 => Ok(PageTableLevel::Pdpt),
            4 => Ok(PageTableLevel::Pml4),
            5 => Ok(PageTableLevel::Pml5),
            _ => Err(()),
        }
    }
}

// =============================================================================
// Page Flags
// =============================================================================

bitflags! {
    /// Page table entry flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageFlags: u64 {
        /// Page is present in memory
        const PRESENT = 1 << 0;

        /// Page is writable (otherwise read-only)
        const WRITABLE = 1 << 1;

        /// Page is accessible from user mode
        const USER_ACCESSIBLE = 1 << 2;

        /// Page uses write-through caching
        const WRITE_THROUGH = 1 << 3;

        /// Page caching is disabled
        const NO_CACHE = 1 << 4;

        /// CPU has accessed this page
        const ACCESSED = 1 << 5;

        /// CPU has written to this page
        const DIRTY = 1 << 6;

        /// Entry maps a huge page (2MB or 1GB)
        const HUGE_PAGE = 1 << 7;

        /// Page is global (not flushed on CR3 change)
        const GLOBAL = 1 << 8;

        /// Available for OS use (bit 9)
        const OS_BIT_9 = 1 << 9;

        /// Available for OS use (bit 10)
        const OS_BIT_10 = 1 << 10;

        /// Available for OS use (bit 11)
        const OS_BIT_11 = 1 << 11;

        /// Available for OS use (bit 52)
        const OS_BIT_52 = 1 << 52;

        /// Available for OS use (bit 53)
        const OS_BIT_53 = 1 << 53;

        /// Available for OS use (bit 54)
        const OS_BIT_54 = 1 << 54;

        /// Available for OS use (bit 55)
        const OS_BIT_55 = 1 << 55;

        /// Available for OS use (bit 56)
        const OS_BIT_56 = 1 << 56;

        /// Available for OS use (bit 57)
        const OS_BIT_57 = 1 << 57;

        /// Available for OS use (bit 58)
        const OS_BIT_58 = 1 << 58;

        /// Code execution is disabled (requires NXE bit in EFER)
        const NO_EXECUTE = 1 << 63;
    }
}

impl PageFlags {
    /// Kernel read-only data flags
    pub const KERNEL_RO: Self = Self::PRESENT.union(Self::NO_EXECUTE);

    /// Kernel read-write data flags
    pub const KERNEL_RW: Self = Self::PRESENT.union(Self::WRITABLE).union(Self::NO_EXECUTE);

    /// Kernel executable code flags
    pub const KERNEL_CODE: Self = Self::PRESENT;

    /// User read-only data flags
    pub const USER_RO: Self = Self::PRESENT
        .union(Self::USER_ACCESSIBLE)
        .union(Self::NO_EXECUTE);

    /// User read-write data flags
    pub const USER_RW: Self = Self::PRESENT
        .union(Self::WRITABLE)
        .union(Self::USER_ACCESSIBLE)
        .union(Self::NO_EXECUTE);

    /// User executable code flags
    pub const USER_CODE: Self = Self::PRESENT.union(Self::USER_ACCESSIBLE);

    /// Page table (intermediate level) flags
    pub const TABLE: Self = Self::PRESENT
        .union(Self::WRITABLE)
        .union(Self::USER_ACCESSIBLE);

    /// Check if the page is present
    #[inline]
    pub const fn is_present(self) -> bool {
        self.contains(Self::PRESENT)
    }

    /// Check if the page is writable
    #[inline]
    pub const fn is_writable(self) -> bool {
        self.contains(Self::WRITABLE)
    }

    /// Check if the page is user accessible
    #[inline]
    pub const fn is_user_accessible(self) -> bool {
        self.contains(Self::USER_ACCESSIBLE)
    }

    /// Check if this is a huge page
    #[inline]
    pub const fn is_huge_page(self) -> bool {
        self.contains(Self::HUGE_PAGE)
    }

    /// Check if execution is disabled
    #[inline]
    pub const fn is_no_execute(self) -> bool {
        self.contains(Self::NO_EXECUTE)
    }

    /// Check if the page is global
    #[inline]
    pub const fn is_global(self) -> bool {
        self.contains(Self::GLOBAL)
    }
}

impl Default for PageFlags {
    fn default() -> Self {
        Self::empty()
    }
}

// =============================================================================
// Page Table Entry
// =============================================================================

/// A 64-bit page table entry
#[derive(Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    /// Create an empty (not present) entry
    #[inline]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create a new entry with the given address and flags
    #[inline]
    pub const fn new(addr: PhysicalAddress, flags: PageFlags) -> Self {
        Self((addr.as_u64() & PHYS_ADDR_MASK) | flags.bits())
    }

    /// Create an entry pointing to a page table
    #[inline]
    pub const fn table(addr: PhysicalAddress) -> Self {
        Self::new(addr, PageFlags::TABLE)
    }

    /// Create an entry for a 4KB page
    #[inline]
    pub const fn page_4k(addr: PhysicalAddress, flags: PageFlags) -> Self {
        Self::new(addr, flags)
    }

    /// Create an entry for a 2MB huge page
    #[inline]
    pub const fn page_2m(addr: PhysicalAddress, flags: PageFlags) -> Self {
        Self::new(addr, flags.union(PageFlags::HUGE_PAGE))
    }

    /// Create an entry for a 1GB huge page
    #[inline]
    pub const fn page_1g(addr: PhysicalAddress, flags: PageFlags) -> Self {
        Self::new(addr, flags.union(PageFlags::HUGE_PAGE))
    }

    /// Check if the entry is present
    #[inline]
    pub const fn is_present(&self) -> bool {
        self.0 & PageFlags::PRESENT.bits() != 0
    }

    /// Check if the entry is unused (zero)
    #[inline]
    pub const fn is_unused(&self) -> bool {
        self.0 == 0
    }

    /// Check if this is a huge page entry
    #[inline]
    pub const fn is_huge_page(&self) -> bool {
        self.0 & PageFlags::HUGE_PAGE.bits() != 0
    }

    /// Get the flags
    #[inline]
    pub const fn flags(&self) -> PageFlags {
        PageFlags::from_bits_truncate(self.0)
    }

    /// Get the physical address
    #[inline]
    pub const fn address(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.0 & PHYS_ADDR_MASK)
    }

    /// Get the raw entry value
    #[inline]
    pub const fn bits(&self) -> u64 {
        self.0
    }

    /// Set the entry value
    #[inline]
    pub fn set(&mut self, addr: PhysicalAddress, flags: PageFlags) {
        self.0 = (addr.as_u64() & PHYS_ADDR_MASK) | flags.bits();
    }

    /// Set the flags
    #[inline]
    pub fn set_flags(&mut self, flags: PageFlags) {
        let addr = self.address();
        self.0 = (addr.as_u64() & PHYS_ADDR_MASK) | flags.bits();
    }

    /// Set the address
    #[inline]
    pub fn set_address(&mut self, addr: PhysicalAddress) {
        let flags = self.flags();
        self.0 = (addr.as_u64() & PHYS_ADDR_MASK) | flags.bits();
    }

    /// Add flags
    #[inline]
    pub fn add_flags(&mut self, flags: PageFlags) {
        self.0 |= flags.bits();
    }

    /// Remove flags
    #[inline]
    pub fn remove_flags(&mut self, flags: PageFlags) {
        self.0 &= !flags.bits();
    }

    /// Clear the entry (set to zero)
    #[inline]
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// Check if the accessed flag is set and clear it
    #[inline]
    pub fn check_and_clear_accessed(&mut self) -> bool {
        let accessed = self.0 & PageFlags::ACCESSED.bits() != 0;
        if accessed {
            self.0 &= !PageFlags::ACCESSED.bits();
        }
        accessed
    }

    /// Check if the dirty flag is set and clear it
    #[inline]
    pub fn check_and_clear_dirty(&mut self) -> bool {
        let dirty = self.0 & PageFlags::DIRTY.bits() != 0;
        if dirty {
            self.0 &= !PageFlags::DIRTY.bits();
        }
        dirty
    }

    /// Get the protection key (if PKU is enabled)
    #[inline]
    pub const fn protection_key(&self) -> u8 {
        ((self.0 >> 59) & 0xF) as u8
    }

    /// Set the protection key
    #[inline]
    pub fn set_protection_key(&mut self, key: u8) {
        self.0 = (self.0 & !(0xF << 59)) | ((key as u64 & 0xF) << 59);
    }
}

impl fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_unused() {
            write!(f, "PageTableEntry(UNUSED)")
        } else if self.is_present() {
            f.debug_struct("PageTableEntry")
                .field("addr", &format_args!("{:#x}", self.address().as_u64()))
                .field("flags", &self.flags())
                .field("huge", &self.is_huge_page())
                .finish()
        } else {
            f.debug_struct("PageTableEntry")
                .field("present", &false)
                .field("bits", &format_args!("{:#x}", self.0))
                .finish()
        }
    }
}

impl From<u64> for PageTableEntry {
    #[inline]
    fn from(bits: u64) -> Self {
        Self(bits)
    }
}

impl From<PageTableEntry> for u64 {
    #[inline]
    fn from(entry: PageTableEntry) -> Self {
        entry.0
    }
}

// =============================================================================
// Compile-time Assertions
// =============================================================================

const _: () = {
    use core::mem::size_of;

    // Entry must be 8 bytes
    assert!(size_of::<PageTableEntry>() == 8);

    // Verify flag positions
    assert!(PageFlags::PRESENT.bits() == 1);
    assert!(PageFlags::WRITABLE.bits() == 2);
    assert!(PageFlags::USER_ACCESSIBLE.bits() == 4);
    assert!(PageFlags::HUGE_PAGE.bits() == 128);
    assert!(PageFlags::NO_EXECUTE.bits() == 1 << 63);
};

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_creation() {
        let addr = PhysicalAddress::new(0x1000);
        let flags = PageFlags::PRESENT | PageFlags::WRITABLE;
        let entry = PageTableEntry::new(addr, flags);

        assert!(entry.is_present());
        assert_eq!(entry.address().as_u64(), 0x1000);
        assert!(entry.flags().contains(PageFlags::WRITABLE));
    }

    #[test]
    fn test_huge_page() {
        let addr = PhysicalAddress::new(0x200000);
        let entry = PageTableEntry::page_2m(addr, PageFlags::PRESENT | PageFlags::WRITABLE);

        assert!(entry.is_huge_page());
        assert!(entry.is_present());
    }

    #[test]
    fn test_flags_modification() {
        let mut entry = PageTableEntry::new(PhysicalAddress::new(0x1000), PageFlags::PRESENT);

        assert!(!entry.flags().contains(PageFlags::WRITABLE));
        entry.add_flags(PageFlags::WRITABLE);
        assert!(entry.flags().contains(PageFlags::WRITABLE));
        entry.remove_flags(PageFlags::WRITABLE);
        assert!(!entry.flags().contains(PageFlags::WRITABLE));
    }
}
