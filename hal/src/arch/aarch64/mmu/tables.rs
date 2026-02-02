//! # AArch64 Page Tables
//!
//! This module implements the page table structures for AArch64 translation.

use super::entries::{BlockDescriptor, MemoryAttributes, PageTableEntry, TableDescriptor};
use super::{PAGE_SHIFT, PAGE_SIZE};

// =============================================================================
// Translation Granule
// =============================================================================

/// Translation granule size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationGranule {
    /// 4KB pages
    Granule4K,
    /// 16KB pages
    Granule16K,
    /// 64KB pages
    Granule64K,
}

impl TranslationGranule {
    /// Get page size in bytes
    pub const fn page_size(self) -> usize {
        match self {
            Self::Granule4K => 4096,
            Self::Granule16K => 16384,
            Self::Granule64K => 65536,
        }
    }

    /// Get page shift (log2 of page size)
    pub const fn page_shift(self) -> usize {
        match self {
            Self::Granule4K => 12,
            Self::Granule16K => 14,
            Self::Granule64K => 16,
        }
    }

    /// Get entries per table
    pub const fn entries_per_table(self) -> usize {
        match self {
            Self::Granule4K => 512,   // 9 bits
            Self::Granule16K => 2048, // 11 bits
            Self::Granule64K => 8192, // 13 bits
        }
    }

    /// Get index bits per level
    pub const fn index_bits(self) -> usize {
        match self {
            Self::Granule4K => 9,
            Self::Granule16K => 11,
            Self::Granule64K => 13,
        }
    }

    /// Get starting level for 48-bit VA
    pub const fn start_level(self) -> TranslationLevel {
        match self {
            Self::Granule4K => TranslationLevel::L0,
            Self::Granule16K => TranslationLevel::L0,
            Self::Granule64K => TranslationLevel::L1,
        }
    }
}

// =============================================================================
// Translation Level
// =============================================================================

/// Translation table level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum TranslationLevel {
    /// Level 0 (512GB per entry for 4KB granule)
    L0 = 0,
    /// Level 1 (1GB per entry for 4KB granule)
    L1 = 1,
    /// Level 2 (2MB per entry for 4KB granule)
    L2 = 2,
    /// Level 3 (4KB per entry for 4KB granule)
    L3 = 3,
}

impl TranslationLevel {
    /// Get next (deeper) level
    pub const fn next(self) -> Option<Self> {
        match self {
            Self::L0 => Some(Self::L1),
            Self::L1 => Some(Self::L2),
            Self::L2 => Some(Self::L3),
            Self::L3 => None,
        }
    }

    /// Get previous (shallower) level
    pub const fn prev(self) -> Option<Self> {
        match self {
            Self::L0 => None,
            Self::L1 => Some(Self::L0),
            Self::L2 => Some(Self::L1),
            Self::L3 => Some(Self::L2),
        }
    }

    /// Check if this level supports block mappings (for 4KB granule)
    pub const fn supports_blocks(self) -> bool {
        matches!(self, Self::L1 | Self::L2)
    }

    /// Get block size at this level (for 4KB granule)
    pub const fn block_size(self) -> Option<usize> {
        match self {
            Self::L0 => None,
            Self::L1 => Some(1024 * 1024 * 1024), // 1GB
            Self::L2 => Some(2 * 1024 * 1024),    // 2MB
            Self::L3 => Some(4096),               // 4KB (page, not block)
        }
    }

    /// Get the VA index shift for this level (4KB granule)
    pub const fn index_shift(self) -> usize {
        match self {
            Self::L0 => 39, // bits [47:39]
            Self::L1 => 30, // bits [38:30]
            Self::L2 => 21, // bits [29:21]
            Self::L3 => 12, // bits [20:12]
        }
    }

    /// Get the index from a virtual address (4KB granule)
    pub const fn index_from_va(self, va: u64) -> usize {
        ((va >> self.index_shift()) & 0x1FF) as usize
    }
}

// =============================================================================
// Page Table
// =============================================================================

/// Number of entries in a 4KB granule page table
pub const PAGE_TABLE_ENTRIES: usize = 512;

/// A page table (4KB granule, 512 entries)
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; PAGE_TABLE_ENTRIES],
}

impl PageTable {
    /// Create a new empty page table
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::invalid(); PAGE_TABLE_ENTRIES],
        }
    }

    /// Get number of entries
    pub const fn len(&self) -> usize {
        PAGE_TABLE_ENTRIES
    }

    /// Check if empty (all invalid)
    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|e| !e.is_valid())
    }

    /// Get entry at index
    pub fn get(&self, index: usize) -> Option<PageTableEntry> {
        self.entries.get(index).copied()
    }

    /// Get mutable entry at index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut PageTableEntry> {
        self.entries.get_mut(index)
    }

    /// Set entry at index
    pub fn set(&mut self, index: usize, entry: PageTableEntry) {
        if index < PAGE_TABLE_ENTRIES {
            self.entries[index] = entry;
        }
    }

    /// Get entry for virtual address at given level
    pub fn entry_for_va(&self, va: u64, level: TranslationLevel) -> PageTableEntry {
        let index = level.index_from_va(va);
        self.entries[index]
    }

    /// Set entry for virtual address at given level
    pub fn set_entry_for_va(&mut self, va: u64, level: TranslationLevel, entry: PageTableEntry) {
        let index = level.index_from_va(va);
        self.entries[index] = entry;
    }

    /// Get physical address of this table
    pub fn phys_addr(&self) -> u64 {
        self as *const Self as u64
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            *entry = PageTableEntry::invalid();
        }
    }

    /// Count valid entries
    pub fn count_valid(&self) -> usize {
        self.entries.iter().filter(|e| e.is_valid()).count()
    }

    /// Count table entries (pointers to next level)
    pub fn count_tables(&self) -> usize {
        self.entries.iter().filter(|e| e.is_table()).count()
    }

    /// Iterate over entries
    pub fn iter(&self) -> impl Iterator<Item = &PageTableEntry> {
        self.entries.iter()
    }

    /// Iterate mutably over entries
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PageTableEntry> {
        self.entries.iter_mut()
    }

    /// Map a 4KB page
    pub fn map_page(&mut self, index: usize, phys_addr: u64, attrs: MemoryAttributes) {
        if index < PAGE_TABLE_ENTRIES {
            self.entries[index] = PageTableEntry::new_page(phys_addr, attrs);
        }
    }

    /// Map a table pointer
    pub fn map_table(&mut self, index: usize, next_table_addr: u64) {
        if index < PAGE_TABLE_ENTRIES {
            self.entries[index] = PageTableEntry::new_table(next_table_addr);
        }
    }

    /// Unmap entry
    pub fn unmap(&mut self, index: usize) {
        if index < PAGE_TABLE_ENTRIES {
            self.entries[index] = PageTableEntry::invalid();
        }
    }
}

impl Default for PageTable {
    fn default() -> Self {
        Self::new()
    }
}

impl core::ops::Index<usize> for PageTable {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl core::ops::IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

// =============================================================================
// Page Table Walker
// =============================================================================

/// Result of page table walk
#[derive(Debug, Clone, Copy)]
pub enum WalkResult {
    /// Found a valid mapping
    Mapped {
        phys_addr: u64,
        level: TranslationLevel,
        is_block: bool,
    },
    /// No mapping at this address
    NotMapped { level: TranslationLevel },
    /// Invalid entry encountered
    Invalid { level: TranslationLevel },
}

/// Page table walker for 4KB granule
pub struct PageTableWalker {
    root: *const PageTable,
}

impl PageTableWalker {
    /// Create a new walker with root table
    ///
    /// # Safety
    /// Root must point to a valid page table
    pub unsafe fn new(root: *const PageTable) -> Self {
        Self { root }
    }

    /// Walk the page tables to translate a virtual address
    ///
    /// # Safety
    /// All page tables must be valid and mapped
    pub unsafe fn walk(&self, va: u64) -> WalkResult {
        let mut table = self.root;

        for level in [
            TranslationLevel::L0,
            TranslationLevel::L1,
            TranslationLevel::L2,
            TranslationLevel::L3,
        ] {
            let index = level.index_from_va(va);
            let entry = (*table).entries[index];

            if !entry.is_valid() {
                return WalkResult::NotMapped { level };
            }

            // Check for block mapping (L1 or L2)
            if level != TranslationLevel::L3 && !entry.is_table() {
                // Block descriptor
                let block_size = level.block_size().unwrap_or(0);
                let offset = (va as usize) & (block_size - 1);
                return WalkResult::Mapped {
                    phys_addr: entry.phys_addr() + offset as u64,
                    level,
                    is_block: true,
                };
            }

            if level == TranslationLevel::L3 {
                // L3 entry is a page
                let offset = (va as usize) & (PAGE_SIZE - 1);
                return WalkResult::Mapped {
                    phys_addr: entry.phys_addr() + offset as u64,
                    level,
                    is_block: false,
                };
            }

            // Table descriptor - follow to next level
            table = entry.phys_addr() as *const PageTable;
        }

        WalkResult::Invalid {
            level: TranslationLevel::L3,
        }
    }

    /// Translate virtual address to physical
    ///
    /// # Safety
    /// All page tables must be valid
    pub unsafe fn translate(&self, va: u64) -> Option<u64> {
        match self.walk(va) {
            WalkResult::Mapped { phys_addr, .. } => Some(phys_addr),
            _ => None,
        }
    }
}

// =============================================================================
// Page Table Allocator Trait
// =============================================================================

/// Trait for allocating page tables
pub trait PageTableAllocator {
    /// Allocate a new zeroed page table
    fn allocate(&mut self) -> Option<*mut PageTable>;

    /// Deallocate a page table
    ///
    /// # Safety
    /// The table must have been allocated by this allocator
    unsafe fn deallocate(&mut self, table: *mut PageTable);
}

// =============================================================================
// Index Calculations
// =============================================================================

/// Calculate page table indices for a virtual address (4KB granule)
pub fn va_to_indices(va: u64) -> [usize; 4] {
    [
        TranslationLevel::L0.index_from_va(va),
        TranslationLevel::L1.index_from_va(va),
        TranslationLevel::L2.index_from_va(va),
        TranslationLevel::L3.index_from_va(va),
    ]
}

/// Check if virtual address is canonical (valid)
pub fn is_canonical_va(va: u64) -> bool {
    // Upper 16 bits must be all 0s or all 1s
    let upper = va >> 48;
    upper == 0 || upper == 0xFFFF
}

/// Check if virtual address is in user space (TTBR0)
pub fn is_user_va(va: u64) -> bool {
    va < 0x0001_0000_0000_0000
}

/// Check if virtual address is in kernel space (TTBR1)
pub fn is_kernel_va(va: u64) -> bool {
    va >= 0xFFFF_0000_0000_0000
}
