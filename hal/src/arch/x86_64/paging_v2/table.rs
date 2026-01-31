//! # Page Table Structure
//!
//! This module provides the page table structure and indexing utilities.

use core::fmt;
use core::ops::{Index, IndexMut};

use super::entries::PageTableEntry;
use super::addresses::{VirtualAddress, PhysicalAddress};
use super::{ENTRIES_PER_TABLE, TABLE_SIZE};

// =============================================================================
// Page Table Index
// =============================================================================

/// A validated index into a page table (0-511)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PageTableIndex(u16);

impl PageTableIndex {
    /// Create a new page table index
    ///
    /// # Panics
    ///
    /// Panics if the index is >= 512.
    #[inline]
    pub const fn new(index: u16) -> Self {
        assert!(index < ENTRIES_PER_TABLE as u16);
        Self(index)
    }
    
    /// Create a new page table index, truncating to valid range
    #[inline]
    pub const fn new_truncate(index: u16) -> Self {
        Self(index & 0x1FF)
    }
    
    /// Get the index value
    #[inline]
    pub const fn as_u16(self) -> u16 {
        self.0
    }
    
    /// Get the index as usize
    #[inline]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
    
    /// Get the PT index from a virtual address
    #[inline]
    pub const fn pt_index(addr: VirtualAddress) -> Self {
        Self::new_truncate(addr.pt_index())
    }
    
    /// Get the PD index from a virtual address
    #[inline]
    pub const fn pd_index(addr: VirtualAddress) -> Self {
        Self::new_truncate(addr.pd_index())
    }
    
    /// Get the PDPT index from a virtual address
    #[inline]
    pub const fn pdpt_index(addr: VirtualAddress) -> Self {
        Self::new_truncate(addr.pdpt_index())
    }
    
    /// Get the PML4 index from a virtual address
    #[inline]
    pub const fn pml4_index(addr: VirtualAddress) -> Self {
        Self::new_truncate(addr.pml4_index())
    }
    
    /// Get the PML5 index from a virtual address
    #[inline]
    pub const fn pml5_index(addr: VirtualAddress) -> Self {
        Self::new_truncate(addr.pml5_index())
    }
}

impl fmt::Debug for PageTableIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PageTableIndex({})", self.0)
    }
}

impl fmt::Display for PageTableIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u16> for PageTableIndex {
    #[inline]
    fn from(index: u16) -> Self {
        Self::new_truncate(index)
    }
}

impl From<PageTableIndex> for u16 {
    #[inline]
    fn from(index: PageTableIndex) -> Self {
        index.0
    }
}

impl From<PageTableIndex> for usize {
    #[inline]
    fn from(index: PageTableIndex) -> Self {
        index.0 as usize
    }
}

// =============================================================================
// Page Table
// =============================================================================

/// A page table (512 entries, 4096 bytes)
///
/// This represents any level of page table: PML5, PML4, PDPT, PD, or PT.
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; ENTRIES_PER_TABLE],
}

impl PageTable {
    /// Create a new empty page table
    #[inline]
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::empty(); ENTRIES_PER_TABLE],
        }
    }
    
    /// Clear all entries
    #[inline]
    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.clear();
        }
    }
    
    /// Get the number of present entries
    #[inline]
    pub fn count_present(&self) -> usize {
        self.entries.iter().filter(|e| e.is_present()).count()
    }
    
    /// Check if the table is empty (all entries not present)
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|e| !e.is_present())
    }
    
    /// Get a reference to an entry by index
    #[inline]
    pub fn get(&self, index: PageTableIndex) -> &PageTableEntry {
        &self.entries[index.as_usize()]
    }
    
    /// Get a mutable reference to an entry by index
    #[inline]
    pub fn get_mut(&mut self, index: PageTableIndex) -> &mut PageTableEntry {
        &mut self.entries[index.as_usize()]
    }
    
    /// Get a reference to an entry by raw index
    #[inline]
    pub fn get_raw(&self, index: usize) -> Option<&PageTableEntry> {
        self.entries.get(index)
    }
    
    /// Get a mutable reference to an entry by raw index
    #[inline]
    pub fn get_raw_mut(&mut self, index: usize) -> Option<&mut PageTableEntry> {
        self.entries.get_mut(index)
    }
    
    /// Iterate over all entries
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &PageTableEntry> {
        self.entries.iter()
    }
    
    /// Iterate over all entries mutably
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PageTableEntry> {
        self.entries.iter_mut()
    }
    
    /// Iterate over all entries with their indices
    #[inline]
    pub fn iter_indexed(&self) -> impl Iterator<Item = (PageTableIndex, &PageTableEntry)> {
        self.entries
            .iter()
            .enumerate()
            .map(|(i, e)| (PageTableIndex::new_truncate(i as u16), e))
    }
    
    /// Iterate over present entries with their indices
    #[inline]
    pub fn iter_present(&self) -> impl Iterator<Item = (PageTableIndex, &PageTableEntry)> {
        self.iter_indexed().filter(|(_, e)| e.is_present())
    }
    
    /// Get the physical address of this table (assuming it's mapped at its virtual address)
    ///
    /// # Safety
    ///
    /// The table must be identity-mapped or the caller must convert appropriately.
    #[inline]
    pub fn physical_address(&self) -> PhysicalAddress {
        PhysicalAddress::new(self as *const _ as u64)
    }
    
    /// Get a raw pointer to the entries
    #[inline]
    pub fn as_ptr(&self) -> *const PageTableEntry {
        self.entries.as_ptr()
    }
    
    /// Get a raw mutable pointer to the entries
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut PageTableEntry {
        self.entries.as_mut_ptr()
    }
    
    /// Zero-initialize the table
    ///
    /// This is equivalent to clear() but may be faster for full initialization.
    #[inline]
    pub fn zero(&mut self) {
        // Safety: PageTableEntry is just a u64, so zeroing is valid
        unsafe {
            core::ptr::write_bytes(self.entries.as_mut_ptr(), 0, ENTRIES_PER_TABLE);
        }
    }
    
    /// Copy entries from another table
    #[inline]
    pub fn copy_from(&mut self, other: &PageTable) {
        self.entries.copy_from_slice(&other.entries);
    }
    
    /// Copy a range of entries from another table
    #[inline]
    pub fn copy_range(&mut self, start: PageTableIndex, end: PageTableIndex, from: &PageTable) {
        let start = start.as_usize();
        let end = end.as_usize();
        self.entries[start..=end].copy_from_slice(&from.entries[start..=end]);
    }
}

impl Default for PageTable {
    fn default() -> Self {
        Self::new()
    }
}

impl Index<PageTableIndex> for PageTable {
    type Output = PageTableEntry;
    
    #[inline]
    fn index(&self, index: PageTableIndex) -> &Self::Output {
        &self.entries[index.as_usize()]
    }
}

impl IndexMut<PageTableIndex> for PageTable {
    #[inline]
    fn index_mut(&mut self, index: PageTableIndex) -> &mut Self::Output {
        &mut self.entries[index.as_usize()]
    }
}

impl Index<usize> for PageTable {
    type Output = PageTableEntry;
    
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for PageTable {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl fmt::Debug for PageTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PageTable")
            .field("present_entries", &self.count_present())
            .field("address", &format_args!("{:p}", self))
            .finish()
    }
}

// =============================================================================
// Page Table Reference
// =============================================================================

/// A reference to a page table at a physical address
///
/// This is useful for walking page tables where you need to
/// convert physical addresses to virtual addresses.
pub struct PageTableRef {
    /// Physical address of the page table
    phys: PhysicalAddress,
    /// Virtual address (for accessing the table)
    virt: *mut PageTable,
}

impl PageTableRef {
    /// Create a new page table reference
    ///
    /// # Safety
    ///
    /// The physical address must point to a valid page table,
    /// and the virtual address must be a valid mapping of that physical address.
    #[inline]
    pub const unsafe fn new(phys: PhysicalAddress, virt: *mut PageTable) -> Self {
        Self { phys, virt }
    }
    
    /// Create from a physical address using direct mapping
    ///
    /// # Safety
    ///
    /// Physical memory must be directly mapped.
    #[inline]
    pub unsafe fn from_phys(phys: PhysicalAddress) -> Self {
        let virt = super::phys_to_virt(phys).as_mut_ptr();
        Self { phys, virt }
    }
    
    /// Get the physical address
    #[inline]
    pub const fn physical_address(&self) -> PhysicalAddress {
        self.phys
    }
    
    /// Get a reference to the page table
    ///
    /// # Safety
    ///
    /// The virtual address must be valid.
    #[inline]
    pub unsafe fn as_ref(&self) -> &PageTable {
        unsafe { &*self.virt }
    }
    
    /// Get a mutable reference to the page table
    ///
    /// # Safety
    ///
    /// The virtual address must be valid and the caller must have exclusive access.
    #[inline]
    pub unsafe fn as_mut(&mut self) -> &mut PageTable {
        unsafe { &mut *self.virt }
    }
    
    /// Get an entry
    ///
    /// # Safety
    ///
    /// The virtual address must be valid.
    #[inline]
    pub unsafe fn get(&self, index: PageTableIndex) -> PageTableEntry {
        unsafe { (*self.virt)[index] }
    }
    
    /// Set an entry
    ///
    /// # Safety
    ///
    /// The virtual address must be valid and the caller must have exclusive access.
    #[inline]
    pub unsafe fn set(&mut self, index: PageTableIndex, entry: PageTableEntry) {
        unsafe { (*self.virt)[index] = entry; }
    }
}

impl fmt::Debug for PageTableRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PageTableRef")
            .field("phys", &self.phys)
            .field("virt", &self.virt)
            .finish()
    }
}

// =============================================================================
// Compile-time Assertions
// =============================================================================

const _: () = {
    use core::mem::{size_of, align_of};
    
    // Page table must be exactly 4KB
    assert!(size_of::<PageTable>() == TABLE_SIZE);
    
    // Page table must be page-aligned
    assert!(align_of::<PageTable>() == TABLE_SIZE);
    
    // Must have exactly 512 entries
    assert!(ENTRIES_PER_TABLE == 512);
};

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::entries::PageFlags;
    
    #[test]
    fn test_page_table_index() {
        let idx = PageTableIndex::new(100);
        assert_eq!(idx.as_u16(), 100);
        
        let idx_trunc = PageTableIndex::new_truncate(600);
        assert_eq!(idx_trunc.as_u16(), 600 & 0x1FF);
    }
    
    #[test]
    fn test_page_table_empty() {
        let table = PageTable::new();
        assert!(table.is_empty());
        assert_eq!(table.count_present(), 0);
    }
    
    #[test]
    fn test_page_table_operations() {
        let mut table = PageTable::new();
        let idx = PageTableIndex::new(42);
        
        let entry = PageTableEntry::new(
            PhysicalAddress::new(0x1000),
            PageFlags::PRESENT | PageFlags::WRITABLE,
        );
        
        table[idx] = entry;
        
        assert!(!table.is_empty());
        assert_eq!(table.count_present(), 1);
        assert!(table[idx].is_present());
    }
}
