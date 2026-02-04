//! # Page Table Walker
//!
//! This module provides utilities for walking page tables and
//! translating virtual addresses to physical addresses.

use core::fmt;

use super::addresses::{PageSize, PhysicalAddress, VirtualAddress};
use super::entries::{PageFlags, PageTableEntry, PageTableLevel};
use super::table::PageTable;
use super::{is_5level_paging, phys_to_virt};

// =============================================================================
// Translation Error
// =============================================================================

/// Error type for page table translation failures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationError {
    /// Entry at the given level is not present
    NotPresent { level: PageTableLevel, index: u16 },

    /// Address is not canonical
    NonCanonical,

    /// Huge page at unexpected level
    UnexpectedHugePage { level: PageTableLevel },

    /// Table is null
    NullTable,
}

impl fmt::Display for TranslationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TranslationError::NotPresent { level, index } => {
                write!(f, "Entry not present at {} index {}", level.name(), index)
            },
            TranslationError::NonCanonical => {
                write!(f, "Address is not canonical")
            },
            TranslationError::UnexpectedHugePage { level } => {
                write!(f, "Unexpected huge page at level {}", level.name())
            },
            TranslationError::NullTable => {
                write!(f, "Null table pointer")
            },
        }
    }
}

// =============================================================================
// Mapping Information
// =============================================================================

/// Information about a virtual address mapping
#[derive(Debug, Clone, Copy)]
pub struct MappingInfo {
    /// Virtual address that was translated
    pub virtual_address: VirtualAddress,

    /// Physical address it maps to
    pub physical_address: PhysicalAddress,

    /// Page size
    pub page_size: PageSize,

    /// Page flags
    pub flags: PageFlags,

    /// Page table level where the mapping was found
    pub level: PageTableLevel,
}

impl MappingInfo {
    /// Check if the page is writable
    #[inline]
    pub const fn is_writable(&self) -> bool {
        self.flags.contains(PageFlags::WRITABLE)
    }

    /// Check if the page is user accessible
    #[inline]
    pub const fn is_user_accessible(&self) -> bool {
        self.flags.contains(PageFlags::USER_ACCESSIBLE)
    }

    /// Check if execution is allowed
    #[inline]
    pub const fn is_executable(&self) -> bool {
        !self.flags.contains(PageFlags::NO_EXECUTE)
    }

    /// Check if this is a huge page
    #[inline]
    pub const fn is_huge_page(&self) -> bool {
        !matches!(self.page_size, PageSize::Size4K)
    }
}

impl fmt::Display for MappingInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} -> {} ({}, {:?})",
            self.virtual_address, self.physical_address, self.page_size, self.flags
        )
    }
}

// =============================================================================
// Page Table Walker
// =============================================================================

/// Page table walker for address translation
///
/// This struct provides methods to walk page tables and translate
/// virtual addresses to physical addresses.
pub struct PageTableWalker {
    /// Physical address of the root page table (from CR3)
    root: PhysicalAddress,

    /// Whether 5-level paging is active
    five_level: bool,
}

impl PageTableWalker {
    /// Create a new page table walker from CR3
    ///
    /// # Safety
    ///
    /// The CR3 value must point to a valid page table.
    #[inline]
    pub unsafe fn from_cr3(cr3: u64) -> Self {
        Self {
            root: PhysicalAddress::new(cr3 & super::PHYS_ADDR_MASK),
            five_level: is_5level_paging(),
        }
    }

    /// Create a new page table walker from the current CR3
    ///
    /// # Safety
    ///
    /// The current page table must be valid.
    #[inline]
    pub unsafe fn current() -> Self {
        unsafe { Self::from_cr3(super::read_cr3()) }
    }

    /// Create a new page table walker from a root physical address
    ///
    /// # Safety
    ///
    /// The physical address must point to a valid page table.
    #[inline]
    pub const unsafe fn new(root: PhysicalAddress, five_level: bool) -> Self {
        Self { root, five_level }
    }

    /// Get the root physical address
    #[inline]
    pub const fn root(&self) -> PhysicalAddress {
        self.root
    }

    /// Check if using 5-level paging
    #[inline]
    pub const fn is_5level(&self) -> bool {
        self.five_level
    }

    /// Translate a virtual address to a physical address
    ///
    /// # Safety
    ///
    /// The page tables must be valid and mapped.
    pub unsafe fn translate(&self, virt: VirtualAddress) -> Result<MappingInfo, TranslationError> {
        // Check canonical address
        if self.five_level {
            if !virt.is_canonical_5level() {
                return Err(TranslationError::NonCanonical);
            }
        } else if !virt.is_canonical_4level() {
            return Err(TranslationError::NonCanonical);
        }

        // Start walking from the root
        let mut table_phys = self.root;
        let start_level = if self.five_level {
            PageTableLevel::Pml5
        } else {
            PageTableLevel::Pml4
        };

        let mut current_level = start_level;

        loop {
            // Get the table
            let table_virt = phys_to_virt(table_phys);
            let table = unsafe { &*(table_virt.as_ptr::<PageTable>()) };

            // Get the index for this level
            let index = virt.table_index(current_level as u8);
            let entry = table[index as usize];

            // Check if present
            if !entry.is_present() {
                return Err(TranslationError::NotPresent {
                    level: current_level,
                    index,
                });
            }

            // Check for huge page
            if entry.is_huge_page() && current_level.can_have_huge_pages() {
                let page_size = match current_level {
                    PageTableLevel::Pdpt => PageSize::Size1G,
                    PageTableLevel::Pd => PageSize::Size2M,
                    _ => {
                        return Err(TranslationError::UnexpectedHugePage {
                            level: current_level,
                        })
                    },
                };

                let frame_addr = entry.address();
                let offset = virt.as_u64() & (page_size.size() as u64 - 1);
                let phys_addr = PhysicalAddress::new(frame_addr.as_u64() + offset);

                return Ok(MappingInfo {
                    virtual_address: virt,
                    physical_address: phys_addr,
                    page_size,
                    flags: entry.flags(),
                    level: current_level,
                });
            }

            // Move to next level
            match current_level.next_lower() {
                Some(next_level) => {
                    table_phys = entry.address();
                    current_level = next_level;
                },
                None => {
                    // We're at PT level, this is the final mapping
                    let frame_addr = entry.address();
                    let offset = virt.page_offset() as u64;
                    let phys_addr = PhysicalAddress::new(frame_addr.as_u64() + offset);

                    return Ok(MappingInfo {
                        virtual_address: virt,
                        physical_address: phys_addr,
                        page_size: PageSize::Size4K,
                        flags: entry.flags(),
                        level: current_level,
                    });
                },
            }
        }
    }

    /// Get the page table entry for a virtual address at a specific level
    ///
    /// # Safety
    ///
    /// The page tables must be valid and mapped.
    pub unsafe fn get_entry(
        &self,
        virt: VirtualAddress,
        target_level: PageTableLevel,
    ) -> Result<(PhysicalAddress, PageTableEntry), TranslationError> {
        // Check canonical address
        if !virt.is_canonical_4level() && !self.five_level {
            return Err(TranslationError::NonCanonical);
        }

        let mut table_phys = self.root;
        let start_level = if self.five_level {
            PageTableLevel::Pml5
        } else {
            PageTableLevel::Pml4
        };

        let mut current_level = start_level;

        loop {
            let table_virt = phys_to_virt(table_phys);
            let table = unsafe { &*(table_virt.as_ptr::<PageTable>()) };

            let index = virt.table_index(current_level as u8);
            let entry = table[index as usize];

            if current_level == target_level {
                return Ok((table_phys, entry));
            }

            if !entry.is_present() {
                return Err(TranslationError::NotPresent {
                    level: current_level,
                    index,
                });
            }

            if entry.is_huge_page() {
                return Err(TranslationError::UnexpectedHugePage {
                    level: current_level,
                });
            }

            match current_level.next_lower() {
                Some(next_level) => {
                    table_phys = entry.address();
                    current_level = next_level;
                },
                None => {
                    return Err(TranslationError::NotPresent {
                        level: current_level,
                        index,
                    });
                },
            }
        }
    }

    /// Walk the page table and call a function for each mapping
    ///
    /// # Safety
    ///
    /// The page tables must be valid and mapped.
    pub unsafe fn walk<F>(&self, start: VirtualAddress, end: VirtualAddress, mut callback: F)
    where
        F: FnMut(MappingInfo),
    {
        let mut current = start.align_down(PageSize::Size4K);
        let end_aligned = end.align_up(PageSize::Size4K);

        while current < end_aligned {
            if let Ok(info) = unsafe { self.translate(current) } {
                callback(info);
                // Skip to the end of this page
                current = VirtualAddress::new(current.as_u64() + info.page_size.size() as u64);
            } else {
                // Skip one 4K page and try again
                current = VirtualAddress::new(current.as_u64() + super::PAGE_SIZE_4K as u64);
            }
        }
    }
}

impl fmt::Debug for PageTableWalker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PageTableWalker")
            .field("root", &self.root)
            .field("five_level", &self.five_level)
            .finish()
    }
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Translate a virtual address using the current page tables
///
/// # Safety
///
/// The current page tables must be valid.
#[inline]
pub unsafe fn translate(virt: VirtualAddress) -> Result<PhysicalAddress, TranslationError> {
    let walker = unsafe { PageTableWalker::current() };
    unsafe { walker.translate(virt) }.map(|info| info.physical_address)
}

/// Get full mapping information for a virtual address
///
/// # Safety
///
/// The current page tables must be valid.
#[inline]
pub unsafe fn get_mapping_info(virt: VirtualAddress) -> Result<MappingInfo, TranslationError> {
    let walker = unsafe { PageTableWalker::current() };
    unsafe { walker.translate(virt) }
}

/// Check if a virtual address is mapped
///
/// # Safety
///
/// The current page tables must be valid.
#[inline]
pub unsafe fn is_mapped(virt: VirtualAddress) -> bool {
    let walker = unsafe { PageTableWalker::current() };
    unsafe { walker.translate(virt) }.is_ok()
}

/// Check if a range of virtual addresses is mapped
///
/// # Safety
///
/// The current page tables must be valid.
pub unsafe fn is_range_mapped(start: VirtualAddress, size: usize) -> bool {
    let walker = unsafe { PageTableWalker::current() };
    let end = VirtualAddress::new(start.as_u64() + size as u64);

    let mut current = start.align_down(PageSize::Size4K);

    while current < end {
        match unsafe { walker.translate(current) } {
            Ok(info) => {
                current = VirtualAddress::new(current.as_u64() + info.page_size.size() as u64);
            },
            Err(_) => return false,
        }
    }

    true
}

// =============================================================================
// Debug/Dump Utilities
// =============================================================================

/// Dump page table entries for debugging
///
/// # Safety
///
/// The page tables must be valid.
#[cfg(feature = "debug")]
pub unsafe fn dump_page_table(root: PhysicalAddress, max_depth: u8) {
    // Pre-defined indentation strings for each depth level
    const INDENT: [&str; 6] = ["", "  ", "    ", "      ", "        ", "          "];

    fn dump_level(table_phys: PhysicalAddress, level: PageTableLevel, max_depth: u8, depth: usize) {
        if level as u8 > max_depth || depth >= INDENT.len() {
            return;
        }

        let prefix = INDENT[depth];
        let table_virt = phys_to_virt(table_phys);
        let table = unsafe { &*(table_virt.as_ptr::<PageTable>()) };

        for (idx, entry) in table.iter_present() {
            log::debug!(
                "{}{} [{}]: {} -> {:#x} {:?}",
                prefix,
                level.name(),
                idx,
                if entry.is_huge_page() { "HUGE" } else { "    " },
                entry.address().as_u64(),
                entry.flags()
            );

            if entry.is_present() && !entry.is_huge_page() {
                if let Some(next) = level.next_lower() {
                    dump_level(entry.address(), next, max_depth, depth + 1);
                }
            }
        }
    }

    let start_level = if is_5level_paging() {
        PageTableLevel::Pml5
    } else {
        PageTableLevel::Pml4
    };

    log::debug!("Page Table Dump (root: {:#x})", root.as_u64());
    dump_level(root, start_level, max_depth, 0);
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translation_error_display() {
        let err = TranslationError::NotPresent {
            level: PageTableLevel::Pml4,
            index: 42,
        };
        assert!(err.to_string().contains("PML4"));
        assert!(err.to_string().contains("42"));
    }

    #[test]
    fn test_mapping_info() {
        let info = MappingInfo {
            virtual_address: VirtualAddress::new(0x1000),
            physical_address: PhysicalAddress::new(0x2000),
            page_size: PageSize::Size4K,
            flags: PageFlags::PRESENT | PageFlags::WRITABLE,
            level: PageTableLevel::Pt,
        };

        assert!(info.is_writable());
        assert!(!info.is_user_accessible());
        assert!(info.is_executable()); // NX not set
        assert!(!info.is_huge_page());
    }
}
