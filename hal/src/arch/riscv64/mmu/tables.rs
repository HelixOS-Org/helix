//! # RISC-V Page Tables
//!
//! This module provides page table management for RISC-V Sv39/Sv48/Sv57.

use super::entries::{PageTableEntry, PageFlags, PageSize};
use super::{PAGE_SIZE, PAGE_TABLE_ENTRIES, PAGE_SHIFT};

// ============================================================================
// Page Table Structure
// ============================================================================

/// A page table (512 entries, 4 KiB aligned)
#[repr(C, align(4096))]
#[derive(Clone)]
pub struct PageTable {
    /// Page table entries
    pub entries: [PageTableEntry; PAGE_TABLE_ENTRIES],
}

impl PageTable {
    /// Create a new empty page table
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::empty(); PAGE_TABLE_ENTRIES],
        }
    }

    /// Get a reference to an entry
    #[inline]
    pub fn get(&self, index: usize) -> Option<&PageTableEntry> {
        self.entries.get(index)
    }

    /// Get a mutable reference to an entry
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut PageTableEntry> {
        self.entries.get_mut(index)
    }

    /// Set an entry
    #[inline]
    pub fn set(&mut self, index: usize, entry: PageTableEntry) {
        if index < PAGE_TABLE_ENTRIES {
            self.entries[index] = entry;
        }
    }

    /// Clear an entry
    #[inline]
    pub fn clear(&mut self, index: usize) {
        if index < PAGE_TABLE_ENTRIES {
            self.entries[index] = PageTableEntry::empty();
        }
    }

    /// Clear all entries
    pub fn clear_all(&mut self) {
        for entry in &mut self.entries {
            *entry = PageTableEntry::empty();
        }
    }

    /// Check if the table is empty (no valid entries)
    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|e| !e.is_valid())
    }

    /// Count valid entries
    pub fn count_valid(&self) -> usize {
        self.entries.iter().filter(|e| e.is_valid()).count()
    }

    /// Get the physical address of this table
    pub fn phys_addr(&self) -> usize {
        self as *const Self as usize
    }

    /// Iterate over valid entries
    pub fn iter_valid(&self) -> impl Iterator<Item = (usize, &PageTableEntry)> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_valid())
    }
}

impl Default for PageTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Page Table Level
// ============================================================================

/// Page table levels for different paging modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageTableLevel {
    /// Level 0: 4 KiB pages (PT)
    L0 = 0,
    /// Level 1: 2 MiB mega pages (PD)
    L1 = 1,
    /// Level 2: 1 GiB giga pages (PDPT in Sv39)
    L2 = 2,
    /// Level 3: 512 GiB tera pages (PML4 in Sv48)
    L3 = 3,
    /// Level 4: Root level for Sv57 (PML5)
    L4 = 4,
}

impl PageTableLevel {
    /// Get the page size for this level
    pub const fn page_size(self) -> PageSize {
        match self {
            Self::L0 => PageSize::Page4K,
            Self::L1 => PageSize::Page2M,
            Self::L2 => PageSize::Page1G,
            Self::L3 | Self::L4 => PageSize::Page512G,
        }
    }

    /// Get the VPN shift for this level
    pub const fn vpn_shift(self) -> usize {
        match self {
            Self::L0 => 12,
            Self::L1 => 21,
            Self::L2 => 30,
            Self::L3 => 39,
            Self::L4 => 48,
        }
    }

    /// Get the next lower level
    pub const fn next(self) -> Option<Self> {
        match self {
            Self::L0 => None,
            Self::L1 => Some(Self::L0),
            Self::L2 => Some(Self::L1),
            Self::L3 => Some(Self::L2),
            Self::L4 => Some(Self::L3),
        }
    }

    /// Get the root level for a paging mode
    pub const fn root_for_levels(levels: usize) -> Self {
        match levels {
            3 => Self::L2,
            4 => Self::L3,
            5 => Self::L4,
            _ => Self::L2,
        }
    }

    /// Extract VPN index from virtual address
    pub const fn vpn_index(self, va: usize) -> usize {
        (va >> self.vpn_shift()) & 0x1FF
    }
}

// ============================================================================
// Page Table Operations
// ============================================================================

/// Allocator trait for page tables
pub trait PageTableAllocator {
    /// Allocate a new page table
    fn allocate(&mut self) -> Option<&'static mut PageTable>;
    /// Free a page table
    fn free(&mut self, table: &'static mut PageTable);
}

/// Mapper for page tables
pub struct PageTableMapper<A: PageTableAllocator> {
    /// Root page table
    root: *mut PageTable,
    /// Number of levels (3 for Sv39, 4 for Sv48, 5 for Sv57)
    levels: usize,
    /// Allocator for new page tables
    allocator: A,
}

impl<A: PageTableAllocator> PageTableMapper<A> {
    /// Create a new mapper
    ///
    /// # Safety
    /// The root table must be valid and properly aligned.
    pub unsafe fn new(root: *mut PageTable, levels: usize, allocator: A) -> Self {
        Self {
            root,
            levels,
            allocator,
        }
    }

    /// Get the root table
    pub fn root(&self) -> &PageTable {
        unsafe { &*self.root }
    }

    /// Get the root table mutably
    pub fn root_mut(&mut self) -> &mut PageTable {
        unsafe { &mut *self.root }
    }

    /// Map a page
    ///
    /// # Safety
    /// The physical address must be valid.
    pub unsafe fn map(
        &mut self,
        va: usize,
        pa: usize,
        size: PageSize,
        flags: PageFlags,
    ) -> Result<(), MapError> {
        let target_level = size.level();

        // Check alignment
        if !size.is_aligned(va) || !size.is_aligned(pa) {
            return Err(MapError::Misaligned);
        }

        // Walk to the target level, creating tables as needed
        let mut table = self.root;

        for level_idx in (target_level + 1..self.levels).rev() {
            let level = PageTableLevel::root_for_levels(self.levels);
            let actual_level = match level_idx {
                2 => PageTableLevel::L2,
                1 => PageTableLevel::L1,
                0 => PageTableLevel::L0,
                3 => PageTableLevel::L3,
                4 => PageTableLevel::L4,
                _ => return Err(MapError::InvalidLevel),
            };

            let vpn = actual_level.vpn_index(va);
            let entry = &mut (*table).entries[vpn];

            if !entry.is_valid() {
                // Allocate new table
                let new_table = self.allocator.allocate().ok_or(MapError::OutOfMemory)?;
                new_table.clear_all();
                *entry = PageTableEntry::new_table(new_table.phys_addr());
            } else if entry.is_leaf() {
                // Already mapped at higher level
                return Err(MapError::AlreadyMapped);
            }

            table = entry.phys_addr() as *mut PageTable;
        }

        // Insert the final mapping
        let level = PageTableLevel::root_for_levels(target_level + 1);
        let target = match target_level {
            0 => PageTableLevel::L0,
            1 => PageTableLevel::L1,
            2 => PageTableLevel::L2,
            _ => return Err(MapError::InvalidLevel),
        };
        let vpn = target.vpn_index(va);
        let entry = &mut (*table).entries[vpn];

        if entry.is_valid() {
            return Err(MapError::AlreadyMapped);
        }

        *entry = PageTableEntry::new_page(pa, flags);

        Ok(())
    }

    /// Unmap a page
    pub unsafe fn unmap(&mut self, va: usize) -> Result<(usize, PageSize), MapError> {
        // Walk the page table to find the mapping
        let mut table = self.root;

        for level_idx in (0..self.levels).rev() {
            let level = match level_idx {
                0 => PageTableLevel::L0,
                1 => PageTableLevel::L1,
                2 => PageTableLevel::L2,
                3 => PageTableLevel::L3,
                4 => PageTableLevel::L4,
                _ => return Err(MapError::InvalidLevel),
            };

            let vpn = level.vpn_index(va);
            let entry = &mut (*table).entries[vpn];

            if !entry.is_valid() {
                return Err(MapError::NotMapped);
            }

            if entry.is_leaf() {
                // Found the mapping
                let pa = entry.phys_addr();
                let size = level.page_size();
                *entry = PageTableEntry::empty();
                return Ok((pa, size));
            }

            table = entry.phys_addr() as *mut PageTable;
        }

        Err(MapError::NotMapped)
    }

    /// Translate a virtual address to physical
    pub fn translate(&self, va: usize) -> Option<usize> {
        unsafe {
            let mut table = self.root;

            for level_idx in (0..self.levels).rev() {
                let level = match level_idx {
                    0 => PageTableLevel::L0,
                    1 => PageTableLevel::L1,
                    2 => PageTableLevel::L2,
                    3 => PageTableLevel::L3,
                    4 => PageTableLevel::L4,
                    _ => return None,
                };

                let vpn = level.vpn_index(va);
                let entry = (*table).entries[vpn];

                if !entry.is_valid() {
                    return None;
                }

                if entry.is_leaf() {
                    let page_size = level.page_size();
                    let offset = va & page_size.mask();
                    return Some(entry.phys_addr() | offset);
                }

                table = entry.phys_addr() as *mut PageTable;
            }

            None
        }
    }

    /// Change flags on an existing mapping
    pub unsafe fn remap(&mut self, va: usize, new_flags: PageFlags) -> Result<(), MapError> {
        let mut table = self.root;

        for level_idx in (0..self.levels).rev() {
            let level = match level_idx {
                0 => PageTableLevel::L0,
                1 => PageTableLevel::L1,
                2 => PageTableLevel::L2,
                3 => PageTableLevel::L3,
                4 => PageTableLevel::L4,
                _ => return Err(MapError::InvalidLevel),
            };

            let vpn = level.vpn_index(va);
            let entry = &mut (*table).entries[vpn];

            if !entry.is_valid() {
                return Err(MapError::NotMapped);
            }

            if entry.is_leaf() {
                *entry = entry.with_flags(new_flags);
                return Ok(());
            }

            table = entry.phys_addr() as *mut PageTable;
        }

        Err(MapError::NotMapped)
    }
}

// ============================================================================
// Map Error
// ============================================================================

/// Errors that can occur during mapping
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    /// Address or size not aligned
    Misaligned,
    /// Page already mapped
    AlreadyMapped,
    /// Page not mapped
    NotMapped,
    /// Out of memory for page tables
    OutOfMemory,
    /// Invalid page table level
    InvalidLevel,
    /// Invalid address range
    InvalidRange,
}

// ============================================================================
// Identity Mapping Helpers
// ============================================================================

/// Create identity mapping for a range
pub unsafe fn identity_map_range<A: PageTableAllocator>(
    mapper: &mut PageTableMapper<A>,
    start: usize,
    size: usize,
    flags: PageFlags,
) -> Result<(), MapError> {
    let end = start + size;
    let mut addr = start & !(PAGE_SIZE - 1);

    while addr < end {
        mapper.map(addr, addr, PageSize::Page4K, flags)?;
        addr += PAGE_SIZE;
    }

    Ok(())
}

/// Create identity mapping using large pages where possible
pub unsafe fn identity_map_range_large<A: PageTableAllocator>(
    mapper: &mut PageTableMapper<A>,
    start: usize,
    size: usize,
    flags: PageFlags,
) -> Result<(), MapError> {
    let end = start + size;
    let mut addr = start;

    while addr < end {
        let remaining = end - addr;

        // Try 1 GiB pages first
        if PageSize::Page1G.is_aligned(addr) && remaining >= PageSize::Page1G.size() {
            mapper.map(addr, addr, PageSize::Page1G, flags)?;
            addr += PageSize::Page1G.size();
            continue;
        }

        // Try 2 MiB pages
        if PageSize::Page2M.is_aligned(addr) && remaining >= PageSize::Page2M.size() {
            mapper.map(addr, addr, PageSize::Page2M, flags)?;
            addr += PageSize::Page2M.size();
            continue;
        }

        // Fall back to 4 KiB pages
        mapper.map(addr, addr, PageSize::Page4K, flags)?;
        addr += PAGE_SIZE;
    }

    Ok(())
}

// ============================================================================
// Page Table Walker
// ============================================================================

/// Callback for page table walks
pub trait PageTableVisitor {
    /// Called for each valid leaf entry
    fn visit_leaf(
        &mut self,
        level: PageTableLevel,
        va: usize,
        entry: &PageTableEntry,
    ) -> bool;

    /// Called for each table entry (non-leaf)
    fn visit_table(
        &mut self,
        level: PageTableLevel,
        va: usize,
        entry: &PageTableEntry,
    ) -> bool;
}

/// Walk a page table recursively
pub unsafe fn walk_page_table_with_visitor<V: PageTableVisitor>(
    table: *const PageTable,
    levels: usize,
    level: usize,
    va_base: usize,
    visitor: &mut V,
) {
    let current_level = match level {
        0 => PageTableLevel::L0,
        1 => PageTableLevel::L1,
        2 => PageTableLevel::L2,
        3 => PageTableLevel::L3,
        4 => PageTableLevel::L4,
        _ => return,
    };

    let entry_size = 1 << current_level.vpn_shift();

    for (i, entry) in (*table).entries.iter().enumerate() {
        if !entry.is_valid() {
            continue;
        }

        let va = va_base + (i * entry_size);

        if entry.is_leaf() {
            if !visitor.visit_leaf(current_level, va, entry) {
                return;
            }
        } else {
            if !visitor.visit_table(current_level, va, entry) {
                return;
            }

            if level > 0 {
                let next_table = entry.phys_addr() as *const PageTable;
                walk_page_table_with_visitor(next_table, levels, level - 1, va, visitor);
            }
        }
    }
}
