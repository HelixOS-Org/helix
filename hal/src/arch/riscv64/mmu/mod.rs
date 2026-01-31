//! # RISC-V Memory Management Unit (MMU)
//!
//! This module provides comprehensive MMU support for RISC-V Sv39/Sv48/Sv57.
//!
//! ## Submodules
//!
//! - `satp`: SATP register control
//! - `entries`: Page table entry definitions
//! - `tables`: Page table management
//! - `tlb`: TLB operations
//! - `asid`: Address Space ID management

pub mod satp;
pub mod entries;
pub mod tables;
pub mod tlb;
pub mod asid;

// Re-export commonly used items
pub use satp::{SatpMode, Satp, set_satp, get_satp, enable_paging, disable_paging};
pub use entries::{PageTableEntry, PageFlags, PageSize};
pub use tables::{PageTable, PageTableLevel};
pub use tlb::{flush_tlb, flush_tlb_addr, flush_tlb_asid, flush_tlb_all};
pub use asid::{Asid, AsidAllocator, get_current_asid};

// ============================================================================
// MMU Constants
// ============================================================================

/// Page size (4 KiB)
pub const PAGE_SIZE: usize = 4096;
/// Page size shift (log2 of page size)
pub const PAGE_SHIFT: usize = 12;
/// Page offset mask
pub const PAGE_OFFSET_MASK: usize = PAGE_SIZE - 1;

/// Mega page size (2 MiB) for Sv39/Sv48
pub const MEGA_PAGE_SIZE: usize = 2 * 1024 * 1024;
/// Mega page shift
pub const MEGA_PAGE_SHIFT: usize = 21;

/// Giga page size (1 GiB) for Sv39/Sv48
pub const GIGA_PAGE_SIZE: usize = 1024 * 1024 * 1024;
/// Giga page shift
pub const GIGA_PAGE_SHIFT: usize = 30;

/// Tera page size (512 GiB) for Sv48/Sv57
pub const TERA_PAGE_SIZE: usize = 512 * 1024 * 1024 * 1024;
/// Tera page shift
pub const TERA_PAGE_SHIFT: usize = 39;

/// Number of entries in a page table
pub const PAGE_TABLE_ENTRIES: usize = 512;

/// Physical page number mask (44 bits for Sv39/Sv48)
pub const PPN_MASK: u64 = (1 << 44) - 1;

// ============================================================================
// Virtual Address Layouts
// ============================================================================

/// Sv39 virtual address layout
pub mod sv39 {
    /// Virtual address bits
    pub const VA_BITS: usize = 39;
    /// Number of page table levels
    pub const LEVELS: usize = 3;
    /// Maximum virtual address
    pub const MAX_VA: usize = (1 << VA_BITS) - 1;
    /// Kernel space start (high half)
    pub const KERNEL_START: usize = 0xFFFF_FFC0_0000_0000;
    /// User space end
    pub const USER_END: usize = 0x0000_003F_FFFF_FFFF;

    /// VPN[0] shift (page table level 0)
    pub const VPN0_SHIFT: usize = 12;
    /// VPN[1] shift (page table level 1)
    pub const VPN1_SHIFT: usize = 21;
    /// VPN[2] shift (page table level 2)
    pub const VPN2_SHIFT: usize = 30;

    /// VPN mask (9 bits)
    pub const VPN_MASK: usize = 0x1FF;
}

/// Sv48 virtual address layout
pub mod sv48 {
    /// Virtual address bits
    pub const VA_BITS: usize = 48;
    /// Number of page table levels
    pub const LEVELS: usize = 4;
    /// Maximum virtual address
    pub const MAX_VA: usize = (1 << VA_BITS) - 1;
    /// Kernel space start (high half)
    pub const KERNEL_START: usize = 0xFFFF_8000_0000_0000;
    /// User space end
    pub const USER_END: usize = 0x0000_7FFF_FFFF_FFFF;

    /// VPN[0] shift
    pub const VPN0_SHIFT: usize = 12;
    /// VPN[1] shift
    pub const VPN1_SHIFT: usize = 21;
    /// VPN[2] shift
    pub const VPN2_SHIFT: usize = 30;
    /// VPN[3] shift
    pub const VPN3_SHIFT: usize = 39;

    /// VPN mask (9 bits)
    pub const VPN_MASK: usize = 0x1FF;
}

/// Sv57 virtual address layout
pub mod sv57 {
    /// Virtual address bits
    pub const VA_BITS: usize = 57;
    /// Number of page table levels
    pub const LEVELS: usize = 5;
    /// Maximum virtual address
    pub const MAX_VA: usize = (1 << VA_BITS) - 1;
    /// Kernel space start (high half)
    pub const KERNEL_START: usize = 0xFF00_0000_0000_0000;
    /// User space end
    pub const USER_END: usize = 0x00FF_FFFF_FFFF_FFFF;

    /// VPN[0] shift
    pub const VPN0_SHIFT: usize = 12;
    /// VPN[1] shift
    pub const VPN1_SHIFT: usize = 21;
    /// VPN[2] shift
    pub const VPN2_SHIFT: usize = 30;
    /// VPN[3] shift
    pub const VPN3_SHIFT: usize = 39;
    /// VPN[4] shift
    pub const VPN4_SHIFT: usize = 48;

    /// VPN mask (9 bits)
    pub const VPN_MASK: usize = 0x1FF;
}

// ============================================================================
// Address Conversion Helpers
// ============================================================================

/// Convert virtual address to page-aligned address
#[inline]
pub const fn page_align_down(addr: usize) -> usize {
    addr & !PAGE_OFFSET_MASK
}

/// Round up to next page boundary
#[inline]
pub const fn page_align_up(addr: usize) -> usize {
    (addr + PAGE_SIZE - 1) & !PAGE_OFFSET_MASK
}

/// Get page offset from address
#[inline]
pub const fn page_offset(addr: usize) -> usize {
    addr & PAGE_OFFSET_MASK
}

/// Convert address to page frame number
#[inline]
pub const fn addr_to_pfn(addr: usize) -> usize {
    addr >> PAGE_SHIFT
}

/// Convert page frame number to address
#[inline]
pub const fn pfn_to_addr(pfn: usize) -> usize {
    pfn << PAGE_SHIFT
}

/// Check if address is page-aligned
#[inline]
pub const fn is_page_aligned(addr: usize) -> bool {
    addr & PAGE_OFFSET_MASK == 0
}

// ============================================================================
// Virtual Address Helpers
// ============================================================================

/// Extract VPN indices from virtual address (Sv39)
#[inline]
pub const fn va_to_vpn_sv39(va: usize) -> [usize; 3] {
    [
        (va >> sv39::VPN0_SHIFT) & sv39::VPN_MASK,
        (va >> sv39::VPN1_SHIFT) & sv39::VPN_MASK,
        (va >> sv39::VPN2_SHIFT) & sv39::VPN_MASK,
    ]
}

/// Extract VPN indices from virtual address (Sv48)
#[inline]
pub const fn va_to_vpn_sv48(va: usize) -> [usize; 4] {
    [
        (va >> sv48::VPN0_SHIFT) & sv48::VPN_MASK,
        (va >> sv48::VPN1_SHIFT) & sv48::VPN_MASK,
        (va >> sv48::VPN2_SHIFT) & sv48::VPN_MASK,
        (va >> sv48::VPN3_SHIFT) & sv48::VPN_MASK,
    ]
}

/// Check if address is in user space (Sv39)
#[inline]
pub const fn is_user_addr_sv39(addr: usize) -> bool {
    addr <= sv39::USER_END
}

/// Check if address is in kernel space (Sv39)
#[inline]
pub const fn is_kernel_addr_sv39(addr: usize) -> bool {
    addr >= sv39::KERNEL_START
}

/// Check if address is in user space (Sv48)
#[inline]
pub const fn is_user_addr_sv48(addr: usize) -> bool {
    addr <= sv48::USER_END
}

/// Check if address is in kernel space (Sv48)
#[inline]
pub const fn is_kernel_addr_sv48(addr: usize) -> bool {
    addr >= sv48::KERNEL_START
}

// ============================================================================
// Physical Address Helpers
// ============================================================================

/// Maximum physical address (56 bits for standard implementations)
pub const MAX_PHYS_ADDR: usize = (1 << 56) - 1;

/// Check if physical address is valid
#[inline]
pub const fn is_valid_phys_addr(addr: usize) -> bool {
    addr <= MAX_PHYS_ADDR
}

// ============================================================================
// MMU Initialization
// ============================================================================

/// MMU configuration
#[derive(Debug, Clone, Copy)]
pub struct MmuConfig {
    /// Paging mode (Sv39, Sv48, Sv57)
    pub mode: satp::SatpMode,
    /// Physical address of root page table
    pub root_table: usize,
    /// ASID for the address space
    pub asid: u16,
}

impl MmuConfig {
    /// Create new MMU configuration
    pub const fn new(mode: satp::SatpMode, root_table: usize, asid: u16) -> Self {
        Self {
            mode,
            root_table,
            asid,
        }
    }

    /// Apply this MMU configuration
    pub fn apply(&self) {
        let satp = satp::Satp::new(self.mode, self.asid, self.root_table);
        satp::set_satp(satp);
    }
}

/// Initialize MMU with Sv39
pub fn init_sv39(root_table: usize, asid: u16) {
    let config = MmuConfig::new(satp::SatpMode::Sv39, root_table, asid);
    config.apply();
    tlb::flush_tlb_all();
}

/// Initialize MMU with Sv48
pub fn init_sv48(root_table: usize, asid: u16) {
    let config = MmuConfig::new(satp::SatpMode::Sv48, root_table, asid);
    config.apply();
    tlb::flush_tlb_all();
}

/// Disable MMU (bare mode)
pub fn disable_mmu() {
    satp::disable_paging();
    tlb::flush_tlb_all();
}

// ============================================================================
// Page Table Walker
// ============================================================================

/// Result of page table walk
#[derive(Debug, Clone, Copy)]
pub enum WalkResult {
    /// Found a valid mapping
    Mapped {
        /// Physical address
        phys_addr: usize,
        /// Page size
        page_size: PageSize,
        /// Page flags
        flags: PageFlags,
    },
    /// Page not present
    NotMapped {
        /// Level where walk stopped
        level: usize,
    },
    /// Invalid entry encountered
    Invalid {
        /// Level where invalid entry was found
        level: usize,
    },
}

/// Walk page table to translate virtual address
///
/// # Safety
/// The page table must be valid and accessible.
pub unsafe fn walk_page_table(
    root: *const PageTable,
    va: usize,
    levels: usize,
) -> WalkResult {
    let vpns = match levels {
        3 => {
            let v = va_to_vpn_sv39(va);
            [v[2], v[1], v[0], 0, 0]
        }
        4 => {
            let v = va_to_vpn_sv48(va);
            [v[3], v[2], v[1], v[0], 0]
        }
        _ => return WalkResult::Invalid { level: levels },
    };

    let mut table = root;

    for level in (0..levels).rev() {
        let vpn = vpns[levels - 1 - level];
        let entry = (*table).entries[vpn];

        if !entry.is_valid() {
            return WalkResult::NotMapped { level };
        }

        if entry.is_leaf() {
            // Found a leaf entry
            let page_size = match level {
                0 => PageSize::Page4K,
                1 => PageSize::Page2M,
                2 => PageSize::Page1G,
                3 => PageSize::Page512G,
                _ => return WalkResult::Invalid { level },
            };

            let phys_base = entry.ppn() << PAGE_SHIFT;
            let offset = va & (page_size.size() - 1);

            return WalkResult::Mapped {
                phys_addr: phys_base | offset,
                page_size,
                flags: entry.flags(),
            };
        }

        // Not a leaf, follow to next level
        table = (entry.ppn() << PAGE_SHIFT) as *const PageTable;
    }

    WalkResult::NotMapped { level: 0 }
}
