//! # RISC-V SATP Register
//!
//! This module provides control over the SATP (Supervisor Address Translation
//! and Protection) register.
//!
//! ## SATP Layout (RV64)
//!
//! ```text
//! 63    60 59         44 43                            0
//! +-------+-------------+-------------------------------+
//! | MODE  |    ASID     |             PPN               |
//! +-------+-------------+-------------------------------+
//!   4 bits   16 bits              44 bits
//! ```
//!
//! ## Modes
//!
//! - 0: Bare (no translation)
//! - 8: Sv39 (39-bit virtual address, 3-level page table)
//! - 9: Sv48 (48-bit virtual address, 4-level page table)
//! - 10: Sv57 (57-bit virtual address, 5-level page table)

use super::super::core::csr;

// ============================================================================
// SATP Mode Definitions
// ============================================================================

/// SATP paging modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SatpMode {
    /// No translation (bare mode)
    Bare = 0,
    /// Sv39: 39-bit virtual address, 3-level page table
    Sv39 = 8,
    /// Sv48: 48-bit virtual address, 4-level page table
    Sv48 = 9,
    /// Sv57: 57-bit virtual address, 5-level page table
    Sv57 = 10,
}

impl SatpMode {
    /// Create from raw value
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Bare),
            8 => Some(Self::Sv39),
            9 => Some(Self::Sv48),
            10 => Some(Self::Sv57),
            _ => None,
        }
    }

    /// Get the number of page table levels
    pub const fn levels(self) -> usize {
        match self {
            Self::Bare => 0,
            Self::Sv39 => 3,
            Self::Sv48 => 4,
            Self::Sv57 => 5,
        }
    }

    /// Get the virtual address width in bits
    pub const fn va_bits(self) -> usize {
        match self {
            Self::Bare => 0,
            Self::Sv39 => 39,
            Self::Sv48 => 48,
            Self::Sv57 => 57,
        }
    }

    /// Get the mode name
    pub const fn name(self) -> &'static str {
        match self {
            Self::Bare => "Bare",
            Self::Sv39 => "Sv39",
            Self::Sv48 => "Sv48",
            Self::Sv57 => "Sv57",
        }
    }
}

impl Default for SatpMode {
    fn default() -> Self {
        Self::Bare
    }
}

// ============================================================================
// SATP Register Representation
// ============================================================================

/// SATP register value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Satp {
    /// Raw register value
    bits: u64,
}

impl Satp {
    /// Mode field shift
    pub const MODE_SHIFT: u64 = 60;
    /// Mode field mask
    pub const MODE_MASK: u64 = 0xF << Self::MODE_SHIFT;
    /// ASID field shift
    pub const ASID_SHIFT: u64 = 44;
    /// ASID field mask
    pub const ASID_MASK: u64 = 0xFFFF << Self::ASID_SHIFT;
    /// PPN field mask
    pub const PPN_MASK: u64 = (1 << 44) - 1;

    /// Create from raw value
    pub const fn from_bits(bits: u64) -> Self {
        Self { bits }
    }

    /// Get raw value
    pub const fn bits(self) -> u64 {
        self.bits
    }

    /// Create new SATP value
    pub const fn new(mode: SatpMode, asid: u16, root_ppn: usize) -> Self {
        let bits = ((mode as u64) << Self::MODE_SHIFT)
            | ((asid as u64) << Self::ASID_SHIFT)
            | ((root_ppn as u64 >> 12) & Self::PPN_MASK);
        Self { bits }
    }

    /// Create SATP for bare mode
    pub const fn bare() -> Self {
        Self { bits: 0 }
    }

    /// Create SATP for Sv39
    pub const fn sv39(asid: u16, root_table_addr: usize) -> Self {
        Self::new(SatpMode::Sv39, asid, root_table_addr)
    }

    /// Create SATP for Sv48
    pub const fn sv48(asid: u16, root_table_addr: usize) -> Self {
        Self::new(SatpMode::Sv48, asid, root_table_addr)
    }

    /// Get the mode
    pub const fn mode(self) -> SatpMode {
        let mode_bits = ((self.bits & Self::MODE_MASK) >> Self::MODE_SHIFT) as u8;
        match SatpMode::from_u8(mode_bits) {
            Some(mode) => mode,
            None => SatpMode::Bare,
        }
    }

    /// Get the ASID
    pub const fn asid(self) -> u16 {
        ((self.bits & Self::ASID_MASK) >> Self::ASID_SHIFT) as u16
    }

    /// Get the PPN (physical page number of root table)
    pub const fn ppn(self) -> u64 {
        self.bits & Self::PPN_MASK
    }

    /// Get the physical address of the root page table
    pub const fn root_table_addr(self) -> usize {
        (self.ppn() << 12) as usize
    }

    /// Check if paging is enabled
    pub const fn is_paging_enabled(self) -> bool {
        !matches!(self.mode(), SatpMode::Bare)
    }

    /// Set the ASID
    pub const fn with_asid(self, asid: u16) -> Self {
        let bits = (self.bits & !Self::ASID_MASK) | ((asid as u64) << Self::ASID_SHIFT);
        Self { bits }
    }

    /// Set the root table address
    pub const fn with_root_table(self, addr: usize) -> Self {
        let ppn = (addr as u64 >> 12) & Self::PPN_MASK;
        let bits = (self.bits & !Self::PPN_MASK) | ppn;
        Self { bits }
    }

    /// Set the mode
    pub const fn with_mode(self, mode: SatpMode) -> Self {
        let bits = (self.bits & !Self::MODE_MASK) | ((mode as u64) << Self::MODE_SHIFT);
        Self { bits }
    }
}

impl Default for Satp {
    fn default() -> Self {
        Self::bare()
    }
}

impl From<u64> for Satp {
    fn from(bits: u64) -> Self {
        Self::from_bits(bits)
    }
}

impl From<Satp> for u64 {
    fn from(satp: Satp) -> u64 {
        satp.bits()
    }
}

// ============================================================================
// SATP Register Access
// ============================================================================

/// Read the current SATP value
#[inline]
pub fn read_satp() -> Satp {
    Satp::from_bits(csr::read_satp())
}

/// Write the SATP register
#[inline]
pub fn write_satp(satp: Satp) {
    csr::write_satp(satp.bits());
}

/// Get the current SATP value
#[inline]
pub fn get_satp() -> Satp {
    read_satp()
}

/// Set the SATP register
#[inline]
pub fn set_satp(satp: Satp) {
    write_satp(satp);
}

/// Enable paging with specified mode and root table
pub fn enable_paging(mode: SatpMode, root_table_addr: usize, asid: u16) {
    let satp = Satp::new(mode, asid, root_table_addr);
    write_satp(satp);
    // Ensure the SATP write completes before continuing
    super::super::core::barriers::fence_rw_rw();
}

/// Disable paging (switch to bare mode)
pub fn disable_paging() {
    write_satp(Satp::bare());
    super::super::core::barriers::fence_rw_rw();
}

/// Get the current paging mode
#[inline]
pub fn get_paging_mode() -> SatpMode {
    read_satp().mode()
}

/// Check if paging is currently enabled
#[inline]
pub fn is_paging_enabled() -> bool {
    read_satp().is_paging_enabled()
}

/// Get the current root page table address
#[inline]
pub fn get_root_table_addr() -> usize {
    read_satp().root_table_addr()
}

/// Get the current ASID
#[inline]
pub fn get_current_asid() -> u16 {
    read_satp().asid()
}

/// Set the current ASID without changing other fields
pub fn set_current_asid(asid: u16) {
    let satp = read_satp().with_asid(asid);
    write_satp(satp);
}

/// Switch to a different address space
pub fn switch_address_space(root_table_addr: usize, asid: u16) {
    let satp = read_satp()
        .with_root_table(root_table_addr)
        .with_asid(asid);
    write_satp(satp);
}

// ============================================================================
// SATP Validation
// ============================================================================

/// Check if a mode is supported
pub fn is_mode_supported(mode: SatpMode) -> bool {
    // Try to write the mode and read it back
    let original = read_satp();
    let test = Satp::new(mode, 0, 0);
    write_satp(test);
    let result = read_satp();
    write_satp(original);

    result.mode() == mode
}

/// Probe supported paging modes
pub fn probe_modes() -> (bool, bool, bool) {
    let sv39 = is_mode_supported(SatpMode::Sv39);
    let sv48 = is_mode_supported(SatpMode::Sv48);
    let sv57 = is_mode_supported(SatpMode::Sv57);
    (sv39, sv48, sv57)
}

/// Get the maximum supported paging mode
pub fn max_supported_mode() -> SatpMode {
    if is_mode_supported(SatpMode::Sv57) {
        SatpMode::Sv57
    } else if is_mode_supported(SatpMode::Sv48) {
        SatpMode::Sv48
    } else if is_mode_supported(SatpMode::Sv39) {
        SatpMode::Sv39
    } else {
        SatpMode::Bare
    }
}
