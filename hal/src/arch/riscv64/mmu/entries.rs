//! # RISC-V Page Table Entries
//!
//! This module defines the page table entry format for RISC-V Sv39/Sv48/Sv57.
//!
//! ## PTE Format
//!
//! ```text
//! 63    54 53    28 27    19 18    10 9  8 7 6 5 4 3 2 1 0
//! +-------+--------+--------+--------+----+-+-+-+-+-+-+-+-+
//! |Reserved| PPN[2] | PPN[1] | PPN[0] |RSW |D|A|G|U|X|W|R|V|
//! +-------+--------+--------+--------+----+-+-+-+-+-+-+-+-+
//!  10 bits  26 bits   9 bits   9 bits  2b  1 1 1 1 1 1 1 1
//! ```
//!
//! ## Permission Encoding
//!
//! | R | W | X | Meaning                          |
//! |---|---|---|----------------------------------|
//! | 0 | 0 | 0 | Pointer to next level            |
//! | 0 | 0 | 1 | Execute-only page                |
//! | 0 | 1 | 0 | Reserved (invalid)               |
//! | 0 | 1 | 1 | Read-Execute page                |
//! | 1 | 0 | 0 | Read-only page                   |
//! | 1 | 0 | 1 | Read-Execute page                |
//! | 1 | 1 | 0 | Read-Write page                  |
//! | 1 | 1 | 1 | Read-Write-Execute page          |

use core::fmt;

// ============================================================================
// Page Table Entry
// ============================================================================

/// Page table entry (64-bit)
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    // PTE bit positions
    /// Valid bit
    pub const V_BIT: u64 = 1 << 0;
    /// Read permission
    pub const R_BIT: u64 = 1 << 1;
    /// Write permission
    pub const W_BIT: u64 = 1 << 2;
    /// Execute permission
    pub const X_BIT: u64 = 1 << 3;
    /// User mode accessible
    pub const U_BIT: u64 = 1 << 4;
    /// Global mapping
    pub const G_BIT: u64 = 1 << 5;
    /// Accessed
    pub const A_BIT: u64 = 1 << 6;
    /// Dirty
    pub const D_BIT: u64 = 1 << 7;

    // Reserved for software (RSW) bits
    /// RSW bit 0
    pub const RSW0_BIT: u64 = 1 << 8;
    /// RSW bit 1
    pub const RSW1_BIT: u64 = 1 << 9;

    // PPN fields
    /// PPN[0] shift (bits 10-18)
    pub const PPN0_SHIFT: u64 = 10;
    /// PPN[1] shift (bits 19-27)
    pub const PPN1_SHIFT: u64 = 19;
    /// PPN[2] shift (bits 28-53)
    pub const PPN2_SHIFT: u64 = 28;

    /// Full PPN shift
    pub const PPN_SHIFT: u64 = 10;
    /// Full PPN mask (44 bits)
    pub const PPN_MASK: u64 = ((1u64 << 44) - 1) << Self::PPN_SHIFT;

    /// RWX permission mask
    pub const RWX_MASK: u64 = Self::R_BIT | Self::W_BIT | Self::X_BIT;

    /// All flags mask (bits 0-9)
    pub const FLAGS_MASK: u64 = (1 << 10) - 1;

    // Svpbmt extension bits (bits 61-62)
    /// PMA: Normal memory (default)
    pub const PBMT_PMA: u64 = 0 << 61;
    /// NC: Non-cacheable, idempotent, weakly-ordered
    pub const PBMT_NC: u64 = 1 << 61;
    /// IO: Non-cacheable, non-idempotent, strongly-ordered
    pub const PBMT_IO: u64 = 2 << 61;
    /// PBMT mask
    pub const PBMT_MASK: u64 = 3 << 61;

    // Svnapot extension bit (bit 63)
    /// N bit for naturally aligned power-of-2 pages
    pub const N_BIT: u64 = 1 << 63;

    /// Create an invalid (zero) entry
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create from raw bits
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Get raw bits
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Check if entry is valid
    pub const fn is_valid(self) -> bool {
        self.0 & Self::V_BIT != 0
    }

    /// Check if entry is a leaf (has R, W, or X set)
    pub const fn is_leaf(self) -> bool {
        self.0 & Self::RWX_MASK != 0
    }

    /// Check if entry is a pointer to next level
    pub const fn is_pointer(self) -> bool {
        self.is_valid() && !self.is_leaf()
    }

    /// Check if readable
    pub const fn is_readable(self) -> bool {
        self.0 & Self::R_BIT != 0
    }

    /// Check if writable
    pub const fn is_writable(self) -> bool {
        self.0 & Self::W_BIT != 0
    }

    /// Check if executable
    pub const fn is_executable(self) -> bool {
        self.0 & Self::X_BIT != 0
    }

    /// Check if user accessible
    pub const fn is_user(self) -> bool {
        self.0 & Self::U_BIT != 0
    }

    /// Check if global
    pub const fn is_global(self) -> bool {
        self.0 & Self::G_BIT != 0
    }

    /// Check if accessed
    pub const fn is_accessed(self) -> bool {
        self.0 & Self::A_BIT != 0
    }

    /// Check if dirty
    pub const fn is_dirty(self) -> bool {
        self.0 & Self::D_BIT != 0
    }

    /// Get the physical page number
    pub const fn ppn(self) -> u64 {
        (self.0 & Self::PPN_MASK) >> Self::PPN_SHIFT
    }

    /// Get the physical address (PPN << 12)
    pub const fn phys_addr(self) -> usize {
        (self.ppn() << 12) as usize
    }

    /// Get the flags
    pub const fn flags(self) -> PageFlags {
        PageFlags::from_bits_truncate(self.0 as u16)
    }

    /// Create a valid page entry
    pub const fn new_page(phys_addr: usize, flags: PageFlags) -> Self {
        let ppn = (phys_addr as u64 >> 12) & ((1 << 44) - 1);
        Self((ppn << Self::PPN_SHIFT) | flags.bits() as u64 | Self::V_BIT)
    }

    /// Create a pointer to next level page table
    pub const fn new_table(table_addr: usize) -> Self {
        let ppn = (table_addr as u64 >> 12) & ((1 << 44) - 1);
        Self((ppn << Self::PPN_SHIFT) | Self::V_BIT)
    }

    /// Set the valid bit
    pub const fn set_valid(self) -> Self {
        Self(self.0 | Self::V_BIT)
    }

    /// Clear the valid bit
    pub const fn clear_valid(self) -> Self {
        Self(self.0 & !Self::V_BIT)
    }

    /// Set accessed bit
    pub const fn set_accessed(self) -> Self {
        Self(self.0 | Self::A_BIT)
    }

    /// Clear accessed bit
    pub const fn clear_accessed(self) -> Self {
        Self(self.0 & !Self::A_BIT)
    }

    /// Set dirty bit
    pub const fn set_dirty(self) -> Self {
        Self(self.0 | Self::D_BIT)
    }

    /// Clear dirty bit
    pub const fn clear_dirty(self) -> Self {
        Self(self.0 & !Self::D_BIT)
    }

    /// Set physical address
    pub const fn with_phys_addr(self, phys_addr: usize) -> Self {
        let ppn = (phys_addr as u64 >> 12) & ((1 << 44) - 1);
        Self((self.0 & !Self::PPN_MASK) | (ppn << Self::PPN_SHIFT))
    }

    /// Set flags
    pub const fn with_flags(self, flags: PageFlags) -> Self {
        Self((self.0 & !Self::FLAGS_MASK) | flags.bits() as u64)
    }
}

impl Default for PageTableEntry {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PageTableEntry")
            .field("valid", &self.is_valid())
            .field("leaf", &self.is_leaf())
            .field("ppn", &format_args!("{:#x}", self.ppn()))
            .field("flags", &self.flags())
            .finish()
    }
}

// ============================================================================
// Page Flags
// ============================================================================

bitflags::bitflags! {
    /// Page table entry flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageFlags: u16 {
        /// Valid
        const VALID = 1 << 0;
        /// Readable
        const READ = 1 << 1;
        /// Writable
        const WRITE = 1 << 2;
        /// Executable
        const EXEC = 1 << 3;
        /// User accessible
        const USER = 1 << 4;
        /// Global mapping
        const GLOBAL = 1 << 5;
        /// Accessed
        const ACCESSED = 1 << 6;
        /// Dirty
        const DIRTY = 1 << 7;
        /// Reserved for software 0
        const RSW0 = 1 << 8;
        /// Reserved for software 1
        const RSW1 = 1 << 9;
    }
}

impl PageFlags {
    /// Read-only page
    pub const RO: Self = Self::READ;
    /// Read-write page
    pub const RW: Self = Self::READ.union(Self::WRITE);
    /// Read-execute page
    pub const RX: Self = Self::READ.union(Self::EXEC);
    /// Read-write-execute page
    pub const RWX: Self = Self::READ.union(Self::WRITE).union(Self::EXEC);
    /// Execute-only page
    pub const XO: Self = Self::EXEC;

    /// Kernel code (RX, global)
    pub const KERNEL_CODE: Self = Self::RX.union(Self::GLOBAL).union(Self::VALID);
    /// Kernel data (RW, global)
    pub const KERNEL_DATA: Self = Self::RW.union(Self::GLOBAL).union(Self::VALID);
    /// Kernel read-only data (R, global)
    pub const KERNEL_RODATA: Self = Self::RO.union(Self::GLOBAL).union(Self::VALID);

    /// User code (RX, user)
    pub const USER_CODE: Self = Self::RX.union(Self::USER).union(Self::VALID);
    /// User data (RW, user)
    pub const USER_DATA: Self = Self::RW.union(Self::USER).union(Self::VALID);
    /// User read-only data (R, user)
    pub const USER_RODATA: Self = Self::RO.union(Self::USER).union(Self::VALID);

    /// Check if flags represent a leaf entry
    pub const fn is_leaf(self) -> bool {
        self.intersects(Self::READ.union(Self::WRITE).union(Self::EXEC))
    }
}

impl Default for PageFlags {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// Page Sizes
// ============================================================================

/// Page sizes supported by RISC-V
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageSize {
    /// 4 KiB page (level 0)
    Page4K,
    /// 2 MiB mega page (level 1)
    Page2M,
    /// 1 GiB giga page (level 2)
    Page1G,
    /// 512 GiB tera page (level 3, Sv48+)
    Page512G,
}

impl PageSize {
    /// Get the size in bytes
    pub const fn size(self) -> usize {
        match self {
            Self::Page4K => 4 * 1024,
            Self::Page2M => 2 * 1024 * 1024,
            Self::Page1G => 1024 * 1024 * 1024,
            Self::Page512G => 512 * 1024 * 1024 * 1024,
        }
    }

    /// Get the shift (log2 of size)
    pub const fn shift(self) -> usize {
        match self {
            Self::Page4K => 12,
            Self::Page2M => 21,
            Self::Page1G => 30,
            Self::Page512G => 39,
        }
    }

    /// Get the alignment mask
    pub const fn mask(self) -> usize {
        self.size() - 1
    }

    /// Get the page table level for this size
    pub const fn level(self) -> usize {
        match self {
            Self::Page4K => 0,
            Self::Page2M => 1,
            Self::Page1G => 2,
            Self::Page512G => 3,
        }
    }

    /// Get page size from level
    pub const fn from_level(level: usize) -> Option<Self> {
        match level {
            0 => Some(Self::Page4K),
            1 => Some(Self::Page2M),
            2 => Some(Self::Page1G),
            3 => Some(Self::Page512G),
            _ => None,
        }
    }

    /// Check if an address is aligned to this page size
    pub const fn is_aligned(self, addr: usize) -> bool {
        addr & self.mask() == 0
    }

    /// Align address down to this page size
    pub const fn align_down(self, addr: usize) -> usize {
        addr & !self.mask()
    }

    /// Align address up to this page size
    pub const fn align_up(self, addr: usize) -> usize {
        (addr + self.mask()) & !self.mask()
    }
}

impl Default for PageSize {
    fn default() -> Self {
        Self::Page4K
    }
}

// ============================================================================
// Memory Types (Svpbmt Extension)
// ============================================================================

/// Page-based memory types (Svpbmt extension)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MemoryType {
    /// Normal memory (default, uses PMA attributes)
    Normal = 0,
    /// Non-cacheable, idempotent, weakly-ordered (NC)
    NonCacheable = 1,
    /// Non-cacheable, non-idempotent, strongly-ordered (IO)
    DeviceMemory = 2,
}

impl MemoryType {
    /// Convert to PBMT bits
    pub const fn to_pbmt_bits(self) -> u64 {
        (self as u64) << 61
    }

    /// Get from PBMT bits
    pub const fn from_pbmt_bits(bits: u64) -> Self {
        match (bits >> 61) & 3 {
            0 => Self::Normal,
            1 => Self::NonCacheable,
            2 => Self::DeviceMemory,
            _ => Self::Normal,
        }
    }
}

// ============================================================================
// Entry Builder
// ============================================================================

/// Builder for page table entries
#[derive(Debug, Clone, Copy)]
pub struct PageEntryBuilder {
    bits: u64,
}

impl PageEntryBuilder {
    /// Create a new builder
    pub const fn new() -> Self {
        Self { bits: 0 }
    }

    /// Set physical address
    pub const fn phys_addr(mut self, addr: usize) -> Self {
        let ppn = (addr as u64 >> 12) & ((1 << 44) - 1);
        self.bits = (self.bits & !PageTableEntry::PPN_MASK) | (ppn << PageTableEntry::PPN_SHIFT);
        self
    }

    /// Set valid bit
    pub const fn valid(mut self) -> Self {
        self.bits |= PageTableEntry::V_BIT;
        self
    }

    /// Set readable
    pub const fn readable(mut self) -> Self {
        self.bits |= PageTableEntry::R_BIT;
        self
    }

    /// Set writable
    pub const fn writable(mut self) -> Self {
        self.bits |= PageTableEntry::W_BIT;
        self
    }

    /// Set executable
    pub const fn executable(mut self) -> Self {
        self.bits |= PageTableEntry::X_BIT;
        self
    }

    /// Set user accessible
    pub const fn user(mut self) -> Self {
        self.bits |= PageTableEntry::U_BIT;
        self
    }

    /// Set global
    pub const fn global(mut self) -> Self {
        self.bits |= PageTableEntry::G_BIT;
        self
    }

    /// Set accessed
    pub const fn accessed(mut self) -> Self {
        self.bits |= PageTableEntry::A_BIT;
        self
    }

    /// Set dirty
    pub const fn dirty(mut self) -> Self {
        self.bits |= PageTableEntry::D_BIT;
        self
    }

    /// Set memory type (Svpbmt)
    pub const fn memory_type(mut self, mt: MemoryType) -> Self {
        self.bits = (self.bits & !PageTableEntry::PBMT_MASK) | mt.to_pbmt_bits();
        self
    }

    /// Build the entry
    pub const fn build(self) -> PageTableEntry {
        PageTableEntry(self.bits)
    }
}

impl Default for PageEntryBuilder {
    fn default() -> Self {
        Self::new()
    }
}
