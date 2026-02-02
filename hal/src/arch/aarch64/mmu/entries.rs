//! # AArch64 Page Table Entry Definitions
//!
//! This module defines the page table entry formats for AArch64 translation tables.

use core::fmt;

// =============================================================================
// Page Table Entry Constants
// =============================================================================

/// Entry present/valid bit
pub const PTE_VALID: u64 = 1 << 0;

/// Table/Page descriptor (0 = block, 1 = table/page)
pub const PTE_TABLE: u64 = 1 << 1;

/// Memory attribute index (bits 4:2)
pub const PTE_ATTR_INDEX_SHIFT: u64 = 2;
pub const PTE_ATTR_INDEX_MASK: u64 = 0x7 << PTE_ATTR_INDEX_SHIFT;

/// Non-secure bit
pub const PTE_NS: u64 = 1 << 5;

/// Access permission bits (bits 7:6)
pub const PTE_AP_SHIFT: u64 = 6;
pub const PTE_AP_MASK: u64 = 0x3 << PTE_AP_SHIFT;

/// Shareability bits (bits 9:8)
pub const PTE_SH_SHIFT: u64 = 8;
pub const PTE_SH_MASK: u64 = 0x3 << PTE_SH_SHIFT;

/// Access flag
pub const PTE_AF: u64 = 1 << 10;

/// Not global (for TLB ASID matching)
pub const PTE_NG: u64 = 1 << 11;

/// Dirty bit (if HAFDBS enabled)
pub const PTE_DBM: u64 = 1 << 51;

/// Contiguous hint
pub const PTE_CONTIGUOUS: u64 = 1 << 52;

/// Privileged execute never
pub const PTE_PXN: u64 = 1 << 53;

/// Unprivileged execute never
pub const PTE_UXN: u64 = 1 << 54;

/// Execute never alias (for both PXN and UXN)
pub const PTE_XN: u64 = PTE_PXN | PTE_UXN;

/// Software bits (bits 58:55) - available for OS use
pub const PTE_SW_SHIFT: u64 = 55;
pub const PTE_SW_MASK: u64 = 0xF << PTE_SW_SHIFT;

/// Address mask for 4KB granule (bits 47:12)
pub const PTE_ADDR_MASK_4K: u64 = 0x0000_FFFF_FFFF_F000;

/// Address mask for 16KB granule (bits 47:14)
pub const PTE_ADDR_MASK_16K: u64 = 0x0000_FFFF_FFFF_C000;

/// Address mask for 64KB granule (bits 47:16)
pub const PTE_ADDR_MASK_64K: u64 = 0x0000_FFFF_FFFF_0000;

// =============================================================================
// Access Permissions
// =============================================================================

/// Access permission values (AP[2:1])
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AccessPermission {
    /// Kernel read/write, user no access
    KernelRwUserNone = 0b00,
    /// Kernel read/write, user read/write
    KernelRwUserRw   = 0b01,
    /// Kernel read-only, user no access
    KernelRoUserNone = 0b10,
    /// Kernel read-only, user read-only
    KernelRoUserRo   = 0b11,
}

impl AccessPermission {
    /// Convert to PTE bits
    pub const fn to_bits(self) -> u64 {
        (self as u64) << PTE_AP_SHIFT
    }

    /// Create from PTE bits
    pub const fn from_bits(bits: u64) -> Self {
        match (bits >> PTE_AP_SHIFT) & 0x3 {
            0b00 => Self::KernelRwUserNone,
            0b01 => Self::KernelRwUserRw,
            0b10 => Self::KernelRoUserNone,
            0b11 => Self::KernelRoUserRo,
            _ => Self::KernelRwUserNone,
        }
    }

    /// Check if writable by kernel
    pub const fn is_kernel_writable(self) -> bool {
        matches!(self, Self::KernelRwUserNone | Self::KernelRwUserRw)
    }

    /// Check if accessible by user
    pub const fn is_user_accessible(self) -> bool {
        matches!(self, Self::KernelRwUserRw | Self::KernelRoUserRo)
    }
}

// =============================================================================
// Shareability Domain
// =============================================================================

/// Shareability domain (SH[1:0])
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Shareability {
    /// Non-shareable
    NonShareable   = 0b00,
    /// Reserved
    Reserved       = 0b01,
    /// Outer shareable
    OuterShareable = 0b10,
    /// Inner shareable
    InnerShareable = 0b11,
}

impl Shareability {
    /// Convert to PTE bits
    pub const fn to_bits(self) -> u64 {
        (self as u64) << PTE_SH_SHIFT
    }

    /// Create from PTE bits
    pub const fn from_bits(bits: u64) -> Self {
        match (bits >> PTE_SH_SHIFT) & 0x3 {
            0b00 => Self::NonShareable,
            0b01 => Self::Reserved,
            0b10 => Self::OuterShareable,
            0b11 => Self::InnerShareable,
            _ => Self::NonShareable,
        }
    }
}

// =============================================================================
// Memory Attributes
// =============================================================================

/// Memory attribute index (indexes into MAIR_EL1)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MemoryAttributeIndex {
    /// Device-nGnRnE (strongly ordered)
    DeviceNGnRnE       = 0,
    /// Device-nGnRE
    DeviceNGnRE        = 1,
    /// Normal Non-cacheable
    NormalNonCacheable = 2,
    /// Normal Write-Through
    NormalWriteThrough = 3,
    /// Normal Write-Back
    NormalWriteBack    = 4,
    /// Normal Write-Back Non-transient
    NormalWriteBackNt  = 5,
}

impl MemoryAttributeIndex {
    /// Convert to PTE bits
    pub const fn to_bits(self) -> u64 {
        (self as u64) << PTE_ATTR_INDEX_SHIFT
    }

    /// Create from PTE bits
    pub const fn from_bits(bits: u64) -> Self {
        match (bits >> PTE_ATTR_INDEX_SHIFT) & 0x7 {
            0 => Self::DeviceNGnRnE,
            1 => Self::DeviceNGnRE,
            2 => Self::NormalNonCacheable,
            3 => Self::NormalWriteThrough,
            4 => Self::NormalWriteBack,
            5 => Self::NormalWriteBackNt,
            _ => Self::DeviceNGnRnE,
        }
    }
}

/// Complete memory attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryAttributes {
    /// Attribute index
    pub attr_index: MemoryAttributeIndex,
    /// Shareability
    pub shareability: Shareability,
    /// Access permission
    pub permission: AccessPermission,
    /// Execute never (unprivileged)
    pub uxn: bool,
    /// Privileged execute never
    pub pxn: bool,
    /// Not global (ASID-specific)
    pub not_global: bool,
    /// Access flag (set to 1 for most cases)
    pub access_flag: bool,
}

impl MemoryAttributes {
    /// Create attributes for kernel code
    pub const fn kernel_code() -> Self {
        Self {
            attr_index: MemoryAttributeIndex::NormalWriteBack,
            shareability: Shareability::InnerShareable,
            permission: AccessPermission::KernelRoUserNone,
            uxn: true,
            pxn: false,
            not_global: false,
            access_flag: true,
        }
    }

    /// Create attributes for kernel data
    pub const fn kernel_data() -> Self {
        Self {
            attr_index: MemoryAttributeIndex::NormalWriteBack,
            shareability: Shareability::InnerShareable,
            permission: AccessPermission::KernelRwUserNone,
            uxn: true,
            pxn: true,
            not_global: false,
            access_flag: true,
        }
    }

    /// Create attributes for kernel read-only data
    pub const fn kernel_rodata() -> Self {
        Self {
            attr_index: MemoryAttributeIndex::NormalWriteBack,
            shareability: Shareability::InnerShareable,
            permission: AccessPermission::KernelRoUserNone,
            uxn: true,
            pxn: true,
            not_global: false,
            access_flag: true,
        }
    }

    /// Create attributes for user code
    pub const fn user_code() -> Self {
        Self {
            attr_index: MemoryAttributeIndex::NormalWriteBack,
            shareability: Shareability::InnerShareable,
            permission: AccessPermission::KernelRoUserRo,
            uxn: false,
            pxn: true,
            not_global: true,
            access_flag: true,
        }
    }

    /// Create attributes for user data
    pub const fn user_data() -> Self {
        Self {
            attr_index: MemoryAttributeIndex::NormalWriteBack,
            shareability: Shareability::InnerShareable,
            permission: AccessPermission::KernelRwUserRw,
            uxn: true,
            pxn: true,
            not_global: true,
            access_flag: true,
        }
    }

    /// Create attributes for device memory
    pub const fn device() -> Self {
        Self {
            attr_index: MemoryAttributeIndex::DeviceNGnRnE,
            shareability: Shareability::NonShareable,
            permission: AccessPermission::KernelRwUserNone,
            uxn: true,
            pxn: true,
            not_global: false,
            access_flag: true,
        }
    }

    /// Create attributes for MMIO
    pub const fn mmio() -> Self {
        Self::device()
    }

    /// Convert to PTE bits
    pub const fn to_bits(self) -> u64 {
        let mut bits = PTE_VALID | PTE_AF;

        bits |= self.attr_index.to_bits();
        bits |= self.shareability.to_bits();
        bits |= self.permission.to_bits();

        if self.uxn {
            bits |= PTE_UXN;
        }
        if self.pxn {
            bits |= PTE_PXN;
        }
        if self.not_global {
            bits |= PTE_NG;
        }

        bits
    }
}

// =============================================================================
// Page Table Entry
// =============================================================================

/// Page table entry (4KB page or table pointer)
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    /// Create an invalid (empty) entry
    pub const fn invalid() -> Self {
        Self(0)
    }

    /// Create a new page entry
    pub const fn new_page(phys_addr: u64, attrs: MemoryAttributes) -> Self {
        let addr = phys_addr & PTE_ADDR_MASK_4K;
        Self(addr | attrs.to_bits() | PTE_TABLE | PTE_VALID)
    }

    /// Create a new table entry (pointer to next level)
    pub const fn new_table(next_table_addr: u64) -> Self {
        let addr = next_table_addr & PTE_ADDR_MASK_4K;
        Self(addr | PTE_TABLE | PTE_VALID)
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
        (self.0 & PTE_VALID) != 0
    }

    /// Check if entry is a table pointer
    pub const fn is_table(self) -> bool {
        self.is_valid() && (self.0 & PTE_TABLE) != 0
    }

    /// Check if entry is a page/block
    pub const fn is_page(self) -> bool {
        self.is_valid() && (self.0 & PTE_TABLE) != 0
    }

    /// Get physical address
    pub const fn phys_addr(self) -> u64 {
        self.0 & PTE_ADDR_MASK_4K
    }

    /// Get access permission
    pub const fn permission(self) -> AccessPermission {
        AccessPermission::from_bits(self.0)
    }

    /// Get shareability
    pub const fn shareability(self) -> Shareability {
        Shareability::from_bits(self.0)
    }

    /// Get attribute index
    pub const fn attr_index(self) -> MemoryAttributeIndex {
        MemoryAttributeIndex::from_bits(self.0)
    }

    /// Check if execute never (user)
    pub const fn is_uxn(self) -> bool {
        (self.0 & PTE_UXN) != 0
    }

    /// Check if execute never (privileged)
    pub const fn is_pxn(self) -> bool {
        (self.0 & PTE_PXN) != 0
    }

    /// Check if not global
    pub const fn is_not_global(self) -> bool {
        (self.0 & PTE_NG) != 0
    }

    /// Check if access flag is set
    pub const fn is_accessed(self) -> bool {
        (self.0 & PTE_AF) != 0
    }

    /// Set access flag
    pub fn set_accessed(&mut self) {
        self.0 |= PTE_AF;
    }

    /// Clear valid bit (invalidate)
    pub fn invalidate(&mut self) {
        self.0 &= !PTE_VALID;
    }

    /// Get software bits
    pub const fn sw_bits(self) -> u8 {
        ((self.0 >> PTE_SW_SHIFT) & 0xF) as u8
    }

    /// Set software bits
    pub fn set_sw_bits(&mut self, bits: u8) {
        self.0 = (self.0 & !PTE_SW_MASK) | (((bits & 0xF) as u64) << PTE_SW_SHIFT);
    }
}

impl fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.is_valid() {
            write!(f, "PTE::Invalid")
        } else if self.is_table() {
            write!(f, "PTE::Table({:#x})", self.phys_addr())
        } else {
            write!(
                f,
                "PTE::Page({:#x}, {:?}, {:?})",
                self.phys_addr(),
                self.permission(),
                self.attr_index()
            )
        }
    }
}

// =============================================================================
// Block Descriptor (L1/L2)
// =============================================================================

/// Block descriptor for 1GB (L1) or 2MB (L2) mappings
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct BlockDescriptor(u64);

impl BlockDescriptor {
    /// Create an invalid entry
    pub const fn invalid() -> Self {
        Self(0)
    }

    /// Create a new 1GB block (L1)
    pub const fn new_1g(phys_addr: u64, attrs: MemoryAttributes) -> Self {
        // 1GB aligned (bits 29:0 must be zero in address)
        let addr = phys_addr & 0x0000_FFFF_C000_0000;
        // Block descriptor: bit 1 = 0
        Self(addr | attrs.to_bits() | PTE_VALID)
    }

    /// Create a new 2MB block (L2)
    pub const fn new_2m(phys_addr: u64, attrs: MemoryAttributes) -> Self {
        // 2MB aligned (bits 20:0 must be zero in address)
        let addr = phys_addr & 0x0000_FFFF_FFE0_0000;
        // Block descriptor: bit 1 = 0
        Self(addr | attrs.to_bits() | PTE_VALID)
    }

    /// Get raw bits
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Check if valid
    pub const fn is_valid(self) -> bool {
        (self.0 & PTE_VALID) != 0
    }

    /// Check if this is a block (not table)
    pub const fn is_block(self) -> bool {
        self.is_valid() && (self.0 & PTE_TABLE) == 0
    }

    /// Get physical address (base of block)
    pub const fn phys_addr(self) -> u64 {
        self.0 & 0x0000_FFFF_FFFF_F000
    }
}

impl fmt::Debug for BlockDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.is_valid() {
            write!(f, "Block::Invalid")
        } else {
            write!(f, "Block({:#x})", self.phys_addr())
        }
    }
}

// =============================================================================
// Table Descriptor (L0/L1/L2)
// =============================================================================

/// Table descriptor pointing to next-level table
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct TableDescriptor(u64);

impl TableDescriptor {
    /// Create an invalid entry
    pub const fn invalid() -> Self {
        Self(0)
    }

    /// Create a new table descriptor
    pub const fn new(next_level_addr: u64) -> Self {
        let addr = next_level_addr & PTE_ADDR_MASK_4K;
        Self(addr | PTE_TABLE | PTE_VALID)
    }

    /// Create with hierarchical attributes
    pub const fn with_attrs(
        next_level_addr: u64,
        ns: bool,
        ap_table: u8,
        xn: bool,
        pxn: bool,
    ) -> Self {
        let addr = next_level_addr & PTE_ADDR_MASK_4K;
        let mut bits = addr | PTE_TABLE | PTE_VALID;

        if ns {
            bits |= 1 << 63; // NSTable
        }
        bits |= ((ap_table & 0x3) as u64) << 61; // APTable
        if xn {
            bits |= 1 << 60; // XNTable
        }
        if pxn {
            bits |= 1 << 59; // PXNTable
        }

        Self(bits)
    }

    /// Get raw bits
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Check if valid
    pub const fn is_valid(self) -> bool {
        (self.0 & PTE_VALID) != 0
    }

    /// Check if this is a table descriptor
    pub const fn is_table(self) -> bool {
        self.is_valid() && (self.0 & PTE_TABLE) != 0
    }

    /// Get address of next-level table
    pub const fn next_table_addr(self) -> u64 {
        self.0 & PTE_ADDR_MASK_4K
    }

    /// Get next table as pointer
    pub const fn next_table_ptr<T>(self) -> *mut T {
        self.next_table_addr() as *mut T
    }
}

impl fmt::Debug for TableDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.is_valid() {
            write!(f, "Table::Invalid")
        } else {
            write!(f, "Table({:#x})", self.next_table_addr())
        }
    }
}
