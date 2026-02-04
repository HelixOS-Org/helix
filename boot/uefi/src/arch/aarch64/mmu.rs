//! AArch64 MMU Configuration
//!
//! Memory Management Unit setup for ARM64.

use crate::error::{Error, Result};
use crate::raw::types::*;

// =============================================================================
// PAGE TABLE CONSTANTS
// =============================================================================

/// 4KB page size
pub const PAGE_SIZE_4K: u64 = 4096;
/// 16KB page size
pub const PAGE_SIZE_16K: u64 = 16384;
/// 64KB page size
pub const PAGE_SIZE_64K: u64 = 65536;

/// 2MB block (4KB granule)
pub const BLOCK_SIZE_2M: u64 = 2 * 1024 * 1024;
/// 1GB block (4KB granule)
pub const BLOCK_SIZE_1G: u64 = 1024 * 1024 * 1024;

/// Entries per 4KB page table
pub const ENTRIES_4K: usize = 512;
/// Entries per 16KB page table
pub const ENTRIES_16K: usize = 2048;
/// Entries per 64KB page table
pub const ENTRIES_64K: usize = 8192;

// =============================================================================
// TRANSLATION CONTROL REGISTER (TCR_EL1)
// =============================================================================

/// TCR_EL1 bits
pub mod tcr {
    /// T0SZ: Size offset of TTBR0_EL1 addressed region
    pub const T0SZ_SHIFT: u64 = 0;
    pub const T0SZ_MASK: u64 = 0x3F;

    /// EPD0: Translation table walk disable for TTBR0_EL1
    pub const EPD0: u64 = 1 << 7;

    /// IRGN0: Inner cacheability for TTBR0_EL1
    pub const IRGN0_NC: u64 = 0b00 << 8;
    pub const IRGN0_WB_WA: u64 = 0b01 << 8;
    pub const IRGN0_WT: u64 = 0b10 << 8;
    pub const IRGN0_WB_NWA: u64 = 0b11 << 8;

    /// ORGN0: Outer cacheability for TTBR0_EL1
    pub const ORGN0_NC: u64 = 0b00 << 10;
    pub const ORGN0_WB_WA: u64 = 0b01 << 10;
    pub const ORGN0_WT: u64 = 0b10 << 10;
    pub const ORGN0_WB_NWA: u64 = 0b11 << 10;

    /// SH0: Shareability for TTBR0_EL1
    pub const SH0_NS: u64 = 0b00 << 12;
    pub const SH0_OS: u64 = 0b10 << 12;
    pub const SH0_IS: u64 = 0b11 << 12;

    /// TG0: Granule size for TTBR0_EL1
    pub const TG0_4K: u64 = 0b00 << 14;
    pub const TG0_64K: u64 = 0b01 << 14;
    pub const TG0_16K: u64 = 0b10 << 14;

    /// T1SZ: Size offset of TTBR1_EL1 addressed region
    pub const T1SZ_SHIFT: u64 = 16;
    pub const T1SZ_MASK: u64 = 0x3F << 16;

    /// A1: ASID select (0 = TTBR0, 1 = TTBR1)
    pub const A1: u64 = 1 << 22;

    /// EPD1: Translation table walk disable for TTBR1_EL1
    pub const EPD1: u64 = 1 << 23;

    /// IRGN1: Inner cacheability for TTBR1_EL1
    pub const IRGN1_NC: u64 = 0b00 << 24;
    pub const IRGN1_WB_WA: u64 = 0b01 << 24;
    pub const IRGN1_WT: u64 = 0b10 << 24;
    pub const IRGN1_WB_NWA: u64 = 0b11 << 24;

    /// ORGN1: Outer cacheability for TTBR1_EL1
    pub const ORGN1_NC: u64 = 0b00 << 26;
    pub const ORGN1_WB_WA: u64 = 0b01 << 26;
    pub const ORGN1_WT: u64 = 0b10 << 26;
    pub const ORGN1_WB_NWA: u64 = 0b11 << 26;

    /// SH1: Shareability for TTBR1_EL1
    pub const SH1_NS: u64 = 0b00 << 28;
    pub const SH1_OS: u64 = 0b10 << 28;
    pub const SH1_IS: u64 = 0b11 << 28;

    /// TG1: Granule size for TTBR1_EL1
    pub const TG1_16K: u64 = 0b01 << 30;
    pub const TG1_4K: u64 = 0b10 << 30;
    pub const TG1_64K: u64 = 0b11 << 30;

    /// IPS: Intermediate Physical Address Size
    pub const IPS_32BIT: u64 = 0b000 << 32;
    pub const IPS_36BIT: u64 = 0b001 << 32;
    pub const IPS_40BIT: u64 = 0b010 << 32;
    pub const IPS_42BIT: u64 = 0b011 << 32;
    pub const IPS_44BIT: u64 = 0b100 << 32;
    pub const IPS_48BIT: u64 = 0b101 << 32;
    pub const IPS_52BIT: u64 = 0b110 << 32;

    /// AS: ASID Size (0 = 8 bit, 1 = 16 bit)
    pub const AS: u64 = 1 << 36;

    /// TBI0: Top Byte Ignored for TTBR0_EL1
    pub const TBI0: u64 = 1 << 37;

    /// TBI1: Top Byte Ignored for TTBR1_EL1
    pub const TBI1: u64 = 1 << 38;

    /// HA: Hardware Access flag update
    pub const HA: u64 = 1 << 39;

    /// HD: Hardware Dirty state management
    pub const HD: u64 = 1 << 40;

    /// HPD0: Hierarchical Permission Disables for TTBR0
    pub const HPD0: u64 = 1 << 41;

    /// HPD1: Hierarchical Permission Disables for TTBR1
    pub const HPD1: u64 = 1 << 42;

    /// HWU: Hardware Use fields
    pub const HWU059: u64 = 1 << 43;
    pub const HWU060: u64 = 1 << 44;
    pub const HWU061: u64 = 1 << 45;
    pub const HWU062: u64 = 1 << 46;
    pub const HWU159: u64 = 1 << 47;
    pub const HWU160: u64 = 1 << 48;
    pub const HWU161: u64 = 1 << 49;
    pub const HWU162: u64 = 1 << 50;

    /// TBID0: Top Byte Ignored for data only for TTBR0
    pub const TBID0: u64 = 1 << 51;

    /// TBID1: Top Byte Ignored for data only for TTBR1
    pub const TBID1: u64 = 1 << 52;

    /// NFD0: Non-Fault Disable for TTBR0
    pub const NFD0: u64 = 1 << 53;

    /// NFD1: Non-Fault Disable for TTBR1
    pub const NFD1: u64 = 1 << 54;

    /// E0PD0: E0 prevents data access from TTBR0
    pub const E0PD0: u64 = 1 << 55;

    /// E0PD1: E0 prevents data access from TTBR1
    pub const E0PD1: u64 = 1 << 56;

    /// TCMA0: Top Carry bits used for MTE tag for TTBR0
    pub const TCMA0: u64 = 1 << 57;

    /// TCMA1: Top Carry bits used for MTE tag for TTBR1
    pub const TCMA1: u64 = 1 << 58;

    /// DS: 52-bit address support
    pub const DS: u64 = 1 << 59;
}

// =============================================================================
// MEMORY ATTRIBUTE INDIRECTION REGISTER (MAIR_EL1)
// =============================================================================

/// Memory attributes
pub mod mair {
    /// Device-nGnRnE memory
    pub const DEVICE_NGNRNE: u8 = 0b0000_0000;
    /// Device-nGnRE memory
    pub const DEVICE_NGNRE: u8 = 0b0000_0100;
    /// Device-nGRE memory
    pub const DEVICE_NGRE: u8 = 0b0000_1000;
    /// Device-GRE memory
    pub const DEVICE_GRE: u8 = 0b0000_1100;

    /// Normal Non-Cacheable
    pub const NORMAL_NC: u8 = 0b0100_0100;

    /// Normal Write-Through
    pub const NORMAL_WT: u8 = 0b1011_1011;

    /// Normal Write-Back (typical RAM)
    pub const NORMAL_WB: u8 = 0b1111_1111;

    /// Attribute index in MAIR
    pub const ATTR_DEVICE_NGNRNE: u64 = 0;
    pub const ATTR_DEVICE_NGNRE: u64 = 1;
    pub const ATTR_NORMAL_NC: u64 = 2;
    pub const ATTR_NORMAL_WT: u64 = 3;
    pub const ATTR_NORMAL_WB: u64 = 4;
}

/// Build MAIR_EL1 value with standard attributes
pub fn build_mair() -> u64 {
    (mair::DEVICE_NGNRNE as u64) << (mair::ATTR_DEVICE_NGNRNE * 8)
        | (mair::DEVICE_NGNRE as u64) << (mair::ATTR_DEVICE_NGNRE * 8)
        | (mair::NORMAL_NC as u64) << (mair::ATTR_NORMAL_NC * 8)
        | (mair::NORMAL_WT as u64) << (mair::ATTR_NORMAL_WT * 8)
        | (mair::NORMAL_WB as u64) << (mair::ATTR_NORMAL_WB * 8)
}

// =============================================================================
// PAGE TABLE ENTRY DESCRIPTORS
// =============================================================================

/// Page/Block descriptor bits
pub mod desc {
    /// Valid entry
    pub const VALID: u64 = 1 << 0;

    /// Block/Page indicator (1 = table at L0-2, 1 = page at L3)
    pub const TABLE: u64 = 1 << 1;

    /// Memory attribute index (AttrIndx)
    pub const ATTR_SHIFT: u64 = 2;
    pub const ATTR_MASK: u64 = 0x7 << 2;

    /// Non-Secure bit
    pub const NS: u64 = 1 << 5;

    /// Access Permission (AP)
    pub const AP_SHIFT: u64 = 6;
    /// Read-write EL1, no access EL0
    pub const AP_RW_EL1: u64 = 0b00 << 6;
    /// Read-write EL1/EL0
    pub const AP_RW_EL1_EL0: u64 = 0b01 << 6;
    /// Read-only EL1, no access EL0
    pub const AP_RO_EL1: u64 = 0b10 << 6;
    /// Read-only EL1/EL0
    pub const AP_RO_EL1_EL0: u64 = 0b11 << 6;

    /// Shareability (SH)
    pub const SH_SHIFT: u64 = 8;
    pub const SH_NS: u64 = 0b00 << 8; // Non-shareable
    pub const SH_OS: u64 = 0b10 << 8; // Outer shareable
    pub const SH_IS: u64 = 0b11 << 8; // Inner shareable

    /// Access Flag
    pub const AF: u64 = 1 << 10;

    /// Not Global
    pub const NG: u64 = 1 << 11;

    /// Dirty Bit Modifier (DBM) - for HW dirty management
    pub const DBM: u64 = 1 << 51;

    /// Contiguous hint
    pub const CONTIGUOUS: u64 = 1 << 52;

    /// Privileged Execute Never
    pub const PXN: u64 = 1 << 53;

    /// User Execute Never / Execute Never
    pub const UXN: u64 = 1 << 54;
    pub const XN: u64 = 1 << 54;

    /// Output address mask for 4KB granule
    pub const ADDR_MASK_4K: u64 = 0x0000_FFFF_FFFF_F000;

    /// Output address mask for 16KB granule
    pub const ADDR_MASK_16K: u64 = 0x0000_FFFF_FFFF_C000;

    /// Output address mask for 64KB granule
    pub const ADDR_MASK_64K: u64 = 0x0000_FFFF_FFFF_0000;

    // Table descriptor bits (for non-leaf entries)

    /// NS table bit
    pub const NSTABLE: u64 = 1 << 63;

    /// AP table bits
    pub const APTABLE_SHIFT: u64 = 61;

    /// UXN table bit
    pub const UXNTABLE: u64 = 1 << 60;

    /// PXN table bit
    pub const PXNTABLE: u64 = 1 << 59;

    // Common attribute combinations

    /// Kernel code (read-only, executable)
    pub const KERNEL_CODE: u64 =
        VALID | AF | SH_IS | AP_RO_EL1 | UXN | (super::mair::ATTR_NORMAL_WB << ATTR_SHIFT);

    /// Kernel data (read-write, non-executable)
    pub const KERNEL_DATA: u64 =
        VALID | AF | SH_IS | AP_RW_EL1 | PXN | UXN | (super::mair::ATTR_NORMAL_WB << ATTR_SHIFT);

    /// Kernel read-only data
    pub const KERNEL_RODATA: u64 =
        VALID | AF | SH_IS | AP_RO_EL1 | PXN | UXN | (super::mair::ATTR_NORMAL_WB << ATTR_SHIFT);

    /// User code
    pub const USER_CODE: u64 =
        VALID | AF | SH_IS | AP_RO_EL1_EL0 | PXN | NG | (super::mair::ATTR_NORMAL_WB << ATTR_SHIFT);

    /// User data
    pub const USER_DATA: u64 = VALID
        | AF
        | SH_IS
        | AP_RW_EL1_EL0
        | PXN
        | UXN
        | NG
        | (super::mair::ATTR_NORMAL_WB << ATTR_SHIFT);

    /// Device MMIO
    pub const DEVICE: u64 =
        VALID | AF | SH_OS | AP_RW_EL1 | PXN | UXN | (super::mair::ATTR_DEVICE_NGNRE << ATTR_SHIFT);

    /// Table descriptor
    pub const TABLE_DESC: u64 = VALID | TABLE;
}

// =============================================================================
// PAGE TABLE ENTRY
// =============================================================================

/// Page table entry (64-bit)
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    /// Invalid entry
    pub const fn invalid() -> Self {
        Self(0)
    }

    /// Create table descriptor (points to next level)
    pub const fn table(addr: PhysicalAddress) -> Self {
        Self((addr.0 & desc::ADDR_MASK_4K) | desc::TABLE_DESC)
    }

    /// Create block descriptor (1GB or 2MB mapping)
    pub const fn block(addr: PhysicalAddress, attrs: u64) -> Self {
        Self((addr.0 & desc::ADDR_MASK_4K) | desc::VALID | attrs)
    }

    /// Create page descriptor (4KB mapping at L3)
    pub const fn page(addr: PhysicalAddress, attrs: u64) -> Self {
        Self((addr.0 & desc::ADDR_MASK_4K) | desc::VALID | desc::TABLE | attrs)
    }

    /// Get raw value
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Set raw value
    pub fn set_raw(&mut self, value: u64) {
        self.0 = value;
    }

    /// Check if valid
    pub const fn is_valid(&self) -> bool {
        (self.0 & desc::VALID) != 0
    }

    /// Check if table descriptor
    pub const fn is_table(&self) -> bool {
        (self.0 & (desc::VALID | desc::TABLE)) == (desc::VALID | desc::TABLE)
    }

    /// Check if block/page descriptor
    pub const fn is_block(&self) -> bool {
        (self.0 & desc::VALID) != 0 && (self.0 & desc::TABLE) == 0
    }

    /// Get output address
    pub const fn address(&self) -> PhysicalAddress {
        PhysicalAddress(self.0 & desc::ADDR_MASK_4K)
    }

    /// Set output address
    pub fn set_address(&mut self, addr: PhysicalAddress) {
        self.0 = (self.0 & !desc::ADDR_MASK_4K) | (addr.0 & desc::ADDR_MASK_4K);
    }

    /// Get attribute index
    pub const fn attr_index(&self) -> u64 {
        (self.0 >> desc::ATTR_SHIFT) & 0x7
    }

    /// Check if access flag set
    pub const fn is_accessed(&self) -> bool {
        (self.0 & desc::AF) != 0
    }

    /// Check if non-global
    pub const fn is_non_global(&self) -> bool {
        (self.0 & desc::NG) != 0
    }

    /// Check if execute never (user)
    pub const fn is_uxn(&self) -> bool {
        (self.0 & desc::UXN) != 0
    }

    /// Check if privileged execute never
    pub const fn is_pxn(&self) -> bool {
        (self.0 & desc::PXN) != 0
    }

    /// Clear entry
    pub fn clear(&mut self) {
        self.0 = 0;
    }
}

impl Default for PageTableEntry {
    fn default() -> Self {
        Self::invalid()
    }
}

impl core::fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if !self.is_valid() {
            write!(f, "PTE(invalid)")
        } else if self.is_table() {
            write!(f, "PTE(table -> {:#x})", self.address().0)
        } else {
            write!(f, "PTE(block {:#x}, ", self.address().0)?;
            if self.is_pxn() {
                write!(f, "PXN ")?;
            }
            if self.is_uxn() {
                write!(f, "UXN ")?;
            }
            if self.is_non_global() {
                write!(f, "nG ")?;
            }
            write!(f, "attr={})", self.attr_index())
        }
    }
}

// =============================================================================
// PAGE TABLE
// =============================================================================

/// Page table (4KB, 512 entries)
#[repr(C, align(4096))]
pub struct PageTable4K {
    entries: [PageTableEntry; ENTRIES_4K],
}

impl PageTable4K {
    /// Create empty page table
    pub const fn empty() -> Self {
        Self {
            entries: [PageTableEntry::invalid(); ENTRIES_4K],
        }
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            entry.clear();
        }
    }

    /// Get entry
    pub fn entry(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }

    /// Get entry (mutable)
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
}

impl core::ops::Index<usize> for PageTable4K {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl core::ops::IndexMut<usize> for PageTable4K {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

// =============================================================================
// VIRTUAL ADDRESS DECOMPOSITION
// =============================================================================

/// Virtual address components (4KB granule, 48-bit VA)
#[derive(Debug, Clone, Copy)]
pub struct VaComponents4K {
    /// Level 0 index (bits 39-47)
    pub l0: usize,
    /// Level 1 index (bits 30-38)
    pub l1: usize,
    /// Level 2 index (bits 21-29)
    pub l2: usize,
    /// Level 3 index (bits 12-20)
    pub l3: usize,
    /// Page offset (bits 0-11)
    pub offset: usize,
}

impl VaComponents4K {
    /// Decompose virtual address
    pub fn from_va(va: VirtualAddress) -> Self {
        let va = va.0;
        Self {
            l0: ((va >> 39) & 0x1FFu64) as usize,
            l1: ((va >> 30) & 0x1FFu64) as usize,
            l2: ((va >> 21) & 0x1FFu64) as usize,
            l3: ((va >> 12) & 0x1FFu64) as usize,
            offset: (va & 0xFFFu64) as usize,
        }
    }

    /// Compose virtual address
    pub fn to_va(&self) -> VirtualAddress {
        let mut va = 0u64;
        va |= (self.l0 as u64) << 39;
        va |= (self.l1 as u64) << 30;
        va |= (self.l2 as u64) << 21;
        va |= (self.l3 as u64) << 12;
        va |= self.offset as u64;
        VirtualAddress(va)
    }
}

// =============================================================================
// MMU CONTROL
// =============================================================================

/// Read SCTLR_EL1
pub fn read_sctlr_el1() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, SCTLR_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Write SCTLR_EL1
pub fn write_sctlr_el1(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr SCTLR_EL1, {}",
            "isb",
            in(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// SCTLR_EL1 bits
pub mod sctlr {
    /// MMU enable
    pub const M: u64 = 1 << 0;
    /// Alignment check enable
    pub const A: u64 = 1 << 1;
    /// Data cache enable
    pub const C: u64 = 1 << 2;
    /// Stack alignment check enable
    pub const SA: u64 = 1 << 3;
    /// SP alignment check for EL0
    pub const SA0: u64 = 1 << 4;
    /// Instruction cache enable
    pub const I: u64 = 1 << 12;
    /// Write permission implies XN
    pub const WXN: u64 = 1 << 19;
    /// Exception exit is context synchronizing
    pub const EOS: u64 = 1 << 11;
}

/// Read TCR_EL1
pub fn read_tcr_el1() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, TCR_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Write TCR_EL1
pub fn write_tcr_el1(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr TCR_EL1, {}",
            "isb",
            in(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Read MAIR_EL1
pub fn read_mair_el1() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, MAIR_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Write MAIR_EL1
pub fn write_mair_el1(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr MAIR_EL1, {}",
            "isb",
            in(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Read TTBR0_EL1
pub fn read_ttbr0_el1() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, TTBR0_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Write TTBR0_EL1
pub fn write_ttbr0_el1(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr TTBR0_EL1, {}",
            "isb",
            in(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Read TTBR1_EL1
pub fn read_ttbr1_el1() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, TTBR1_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Write TTBR1_EL1
pub fn write_ttbr1_el1(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr TTBR1_EL1, {}",
            "isb",
            in(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

// =============================================================================
// MMU CONFIGURATION
// =============================================================================

/// MMU configuration
#[derive(Debug, Clone)]
pub struct MmuConfig {
    /// T0SZ value (64 - VA bits for TTBR0)
    pub t0sz: u8,
    /// T1SZ value (64 - VA bits for TTBR1)
    pub t1sz: u8,
    /// Granule size for TTBR0
    pub tg0: u64,
    /// Granule size for TTBR1
    pub tg1: u64,
    /// Physical address size
    pub ips: u64,
    /// 16-bit ASID
    pub asid16: bool,
    /// Hardware access flag update
    pub ha: bool,
    /// Hardware dirty flag management
    pub hd: bool,
}

impl MmuConfig {
    /// Create default configuration for 48-bit VA, 4KB granule
    pub fn default_4k_48bit() -> Self {
        Self {
            t0sz: 16, // 48-bit VA
            t1sz: 16,
            tg0: tcr::TG0_4K,
            tg1: tcr::TG1_4K,
            ips: tcr::IPS_48BIT,
            asid16: true,
            ha: false,
            hd: false,
        }
    }

    /// Build TCR_EL1 value
    pub fn build_tcr(&self) -> u64 {
        let mut tcr_value = 0u64;

        // T0SZ
        tcr_value |= (self.t0sz as u64) & tcr::T0SZ_MASK;

        // T1SZ
        tcr_value |= ((self.t1sz as u64) << tcr::T1SZ_SHIFT) & tcr::T1SZ_MASK;

        // Granule sizes
        tcr_value |= self.tg0;
        tcr_value |= self.tg1;

        // Physical address size
        tcr_value |= self.ips;

        // Cacheability and shareability for both TTBR regions
        tcr_value |= tcr::IRGN0_WB_WA | tcr::ORGN0_WB_WA | tcr::SH0_IS;
        tcr_value |= tcr::IRGN1_WB_WA | tcr::ORGN1_WB_WA | tcr::SH1_IS;

        // ASID size
        if self.asid16 {
            tcr_value |= tcr::AS;
        }

        // Hardware flags
        if self.ha {
            tcr_value |= tcr::HA;
        }
        if self.hd {
            tcr_value |= tcr::HD;
        }

        tcr_value
    }
}

impl Default for MmuConfig {
    fn default() -> Self {
        Self::default_4k_48bit()
    }
}

/// Enable MMU with given page tables
///
/// # Safety
/// - Page tables must be properly set up
/// - Must be called with appropriate exception level
pub unsafe fn enable_mmu(ttbr0: PhysicalAddress, ttbr1: PhysicalAddress, config: &MmuConfig) {
    use super::{dsb, isb};

    // Configure MAIR
    write_mair_el1(build_mair());

    // Configure TCR
    write_tcr_el1(config.build_tcr());

    // Set page table bases
    write_ttbr0_el1(ttbr0.0);
    write_ttbr1_el1(ttbr1.0);

    // Barrier
    dsb();
    isb();

    // Enable MMU
    let sctlr = read_sctlr_el1();
    write_sctlr_el1(sctlr | sctlr::M | sctlr::C | sctlr::I);

    // Barrier
    isb();
}

/// Disable MMU
///
/// # Safety
/// Must ensure identity mapping is in place or PC is in uncached region
pub unsafe fn disable_mmu() {
    use super::{dsb, isb};

    let sctlr = read_sctlr_el1();
    write_sctlr_el1(sctlr & !sctlr::M);

    dsb();
    isb();
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
        assert_eq!(core::mem::size_of::<PageTable4K>(), 4096);
    }

    #[test]
    fn test_mair() {
        let mair = build_mair();
        assert_eq!((mair >> 0) & 0xFF, mair::DEVICE_NGNRNE as u64);
        assert_eq!((mair >> 8) & 0xFF, mair::DEVICE_NGNRE as u64);
        assert_eq!((mair >> 32) & 0xFF, mair::NORMAL_WB as u64);
    }

    #[test]
    fn test_va_components() {
        let va = 0x0000_0001_2345_6789u64;
        let comp = VaComponents4K::from_va(va);
        assert_eq!(comp.offset, 0x789);
        let reconstructed = comp.to_va();
        assert_eq!(reconstructed, va);
    }
}
