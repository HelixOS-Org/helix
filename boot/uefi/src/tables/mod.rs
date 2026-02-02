//! System Tables Layer
//!
//! Advanced parsing and abstraction for system tables.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     System Tables Layer                         │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │  ┌─────────────────────────────────────────────────────────┐   │
//! │  │                  ACPI Tables Parser                      │   │
//! │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────┐│   │
//! │  │  │  RSDP   │ │  MADT   │ │  FADT   │ │  Other Tables   ││   │
//! │  │  └─────────┘ └─────────┘ └─────────┘ └─────────────────┘│   │
//! │  └─────────────────────────────────────────────────────────┘   │
//! │                                                                 │
//! │  ┌─────────────────────────────────────────────────────────┐   │
//! │  │                  SMBIOS Tables Parser                    │   │
//! │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────┐│   │
//! │  │  │ System  │ │  CPU    │ │ Memory  │ │  Other Types    ││   │
//! │  │  └─────────┘ └─────────┘ └─────────┘ └─────────────────┘│   │
//! │  └─────────────────────────────────────────────────────────┘   │
//! │                                                                 │
//! │  ┌─────────────────────────────────────────────────────────┐   │
//! │  │                  Device Tree Parser                      │   │
//! │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────┐│   │
//! │  │  │  Nodes  │ │ Props   │ │  Phandles │ │   FDT Blob    ││   │
//! │  │  └─────────┘ └─────────┘ └─────────┘ └─────────────────┘│   │
//! │  └─────────────────────────────────────────────────────────┘   │
//! │                                                                 │
//! │  ┌─────────────────────────────────────────────────────────┐   │
//! │  │                  Configuration Tables                    │   │
//! │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────┐│   │
//! │  │  │  GUID   │ │  Data   │ │ Vendor  │ │   Custom        ││   │
//! │  │  └─────────┘ └─────────┘ └─────────┘ └─────────────────┘│   │
//! │  └─────────────────────────────────────────────────────────┘   │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

pub mod acpi;
pub mod config;
pub mod smbios;

#[cfg(feature = "dtb")]
pub mod dtb;

use crate::error::Result;
use crate::raw::types::*;

extern crate alloc;
use alloc::vec::Vec;

// Re-exports
pub use self::acpi::*;
pub use self::config::*;
pub use self::smbios::*;

// =============================================================================
// SYSTEM INFO
// =============================================================================

/// Aggregated system information from all tables
#[derive(Debug, Clone)]
pub struct SystemInfo {
    /// Firmware vendor
    pub firmware_vendor: Option<alloc::string::String>,
    /// Firmware version
    pub firmware_version: u32,
    /// ACPI version
    pub acpi_version: Option<AcpiVersion>,
    /// SMBIOS version
    pub smbios_version: Option<SmbiosVersion>,
    /// System manufacturer
    pub manufacturer: Option<alloc::string::String>,
    /// System product name
    pub product_name: Option<alloc::string::String>,
    /// System serial number
    pub serial_number: Option<alloc::string::String>,
    /// System UUID
    pub uuid: Option<[u8; 16]>,
    /// BIOS vendor
    pub bios_vendor: Option<alloc::string::String>,
    /// BIOS version
    pub bios_version: Option<alloc::string::String>,
    /// Processor count
    pub processor_count: usize,
    /// Total memory (bytes)
    pub total_memory: u64,
    /// Chassis type
    pub chassis_type: Option<alloc::string::String>,
}

impl SystemInfo {
    /// Create empty system info
    pub fn new() -> Self {
        Self {
            firmware_vendor: None,
            firmware_version: 0,
            acpi_version: None,
            smbios_version: None,
            manufacturer: None,
            product_name: None,
            serial_number: None,
            uuid: None,
            bios_vendor: None,
            bios_version: None,
            processor_count: 0,
            total_memory: 0,
            chassis_type: None,
        }
    }

    /// Get UUID as string
    pub fn uuid_string(&self) -> Option<alloc::string::String> {
        self.uuid.map(|u| {
            alloc::format!(
                "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
                u[0], u[1], u[2], u[3],
                u[4], u[5],
                u[6], u[7],
                u[8], u[9],
                u[10], u[11], u[12], u[13], u[14], u[15]
            )
        })
    }

    /// Get total memory in human-readable format
    pub fn memory_string(&self) -> alloc::string::String {
        let bytes = self.total_memory;
        if bytes >= 1024 * 1024 * 1024 * 1024 {
            alloc::format!("{} TB", bytes / (1024 * 1024 * 1024 * 1024))
        } else if bytes >= 1024 * 1024 * 1024 {
            alloc::format!("{} GB", bytes / (1024 * 1024 * 1024))
        } else if bytes >= 1024 * 1024 {
            alloc::format!("{} MB", bytes / (1024 * 1024))
        } else if bytes >= 1024 {
            alloc::format!("{} KB", bytes / 1024)
        } else {
            alloc::format!("{} B", bytes)
        }
    }
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// ACPI VERSION
// =============================================================================

/// ACPI specification version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AcpiVersion {
    /// Major version
    pub major: u8,
    /// Minor version
    pub minor: u8,
}

impl AcpiVersion {
    /// Create new version
    pub const fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }

    /// ACPI 1.0
    pub const V1_0: Self = Self::new(1, 0);
    /// ACPI 2.0
    pub const V2_0: Self = Self::new(2, 0);
    /// ACPI 3.0
    pub const V3_0: Self = Self::new(3, 0);
    /// ACPI 4.0
    pub const V4_0: Self = Self::new(4, 0);
    /// ACPI 5.0
    pub const V5_0: Self = Self::new(5, 0);
    /// ACPI 5.1
    pub const V5_1: Self = Self::new(5, 1);
    /// ACPI 6.0
    pub const V6_0: Self = Self::new(6, 0);
    /// ACPI 6.1
    pub const V6_1: Self = Self::new(6, 1);
    /// ACPI 6.2
    pub const V6_2: Self = Self::new(6, 2);
    /// ACPI 6.3
    pub const V6_3: Self = Self::new(6, 3);
    /// ACPI 6.4
    pub const V6_4: Self = Self::new(6, 4);
    /// ACPI 6.5
    pub const V6_5: Self = Self::new(6, 5);
}

impl core::fmt::Display for AcpiVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

// =============================================================================
// SMBIOS VERSION (re-exported)
// =============================================================================

/// SMBIOS specification version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SmbiosVersion {
    /// Major version
    pub major: u8,
    /// Minor version
    pub minor: u8,
}

impl SmbiosVersion {
    /// Create new version
    pub const fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }
}

impl core::fmt::Display for SmbiosVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

// =============================================================================
// TABLE LOCATOR
// =============================================================================

/// Configuration table locator
pub struct TableLocator {
    /// Configuration tables
    tables: Vec<ConfigurationTableEntry>,
}

impl TableLocator {
    /// Create new table locator
    pub fn new() -> Self {
        Self { tables: Vec::new() }
    }

    /// Initialize from system table
    pub unsafe fn init_from_system_table(&mut self, _system_table: *const u8) -> Result<()> {
        // TODO: Parse configuration tables from EFI_SYSTEM_TABLE
        Ok(())
    }

    /// Find table by GUID
    pub fn find(&self, guid: &Guid) -> Option<PhysicalAddress> {
        self.tables
            .iter()
            .find(|t| &t.guid == guid)
            .map(|t| t.address)
    }

    /// Get ACPI 2.0+ RSDP address
    pub fn acpi_20_rsdp(&self) -> Option<PhysicalAddress> {
        self.find(&table_guids::ACPI_20)
    }

    /// Get ACPI 1.0 RSDP address
    pub fn acpi_10_rsdp(&self) -> Option<PhysicalAddress> {
        self.find(&table_guids::ACPI_10)
    }

    /// Get SMBIOS 3.0 entry point address
    pub fn smbios_30(&self) -> Option<PhysicalAddress> {
        self.find(&table_guids::SMBIOS3)
    }

    /// Get SMBIOS entry point address
    pub fn smbios(&self) -> Option<PhysicalAddress> {
        self.find(&table_guids::SMBIOS)
    }

    /// Get Device Tree Blob address
    pub fn dtb(&self) -> Option<PhysicalAddress> {
        self.find(&table_guids::DTB)
    }

    /// Get all configuration tables
    pub fn tables(&self) -> &[ConfigurationTableEntry] {
        &self.tables
    }
}

impl Default for TableLocator {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration table entry
#[derive(Debug, Clone)]
pub struct ConfigurationTableEntry {
    /// Vendor GUID
    pub guid: Guid,
    /// Table address
    pub address: PhysicalAddress,
}

// =============================================================================
// TABLE GUIDS
// =============================================================================

/// Well-known configuration table GUIDs
pub mod table_guids {
    use super::*;

    /// ACPI 1.0 RSDP
    pub const ACPI_10: Guid = Guid::new(0xEB9D2D30, 0x2D88, 0x11D3, [
        0x9A, 0x16, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    /// ACPI 2.0+ RSDP
    pub const ACPI_20: Guid = Guid::new(0x8868E871, 0xE4F1, 0x11D3, [
        0xBC, 0x22, 0x00, 0x80, 0xC7, 0x3C, 0x88, 0x81,
    ]);

    /// SMBIOS entry point
    pub const SMBIOS: Guid = Guid::new(0xEB9D2D31, 0x2D88, 0x11D3, [
        0x9A, 0x16, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    /// SMBIOS 3.0 entry point
    pub const SMBIOS3: Guid = Guid::new(0xF2FD1544, 0x9794, 0x4A2C, [
        0x99, 0x2E, 0xE5, 0xBB, 0xCF, 0x20, 0xE3, 0x94,
    ]);

    /// Device Tree Blob
    pub const DTB: Guid = Guid::new(0xB1B621D5, 0xF19C, 0x41A5, [
        0x83, 0x0B, 0xD9, 0x15, 0x2C, 0x69, 0xAA, 0xE0,
    ]);

    /// MPS (Intel Multi-Processor Specification)
    pub const MPS: Guid = Guid::new(0xEB9D2D2F, 0x2D88, 0x11D3, [
        0x9A, 0x16, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    /// SAL System Table
    pub const SAL: Guid = Guid::new(0xEB9D2D32, 0x2D88, 0x11D3, [
        0x9A, 0x16, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    /// EFI Properties Table
    pub const PROPERTIES: Guid = Guid::new(0x880AACA3, 0x4ADC, 0x4A04, [
        0x90, 0x79, 0xB7, 0x47, 0x34, 0x08, 0x25, 0xE5,
    ]);

    /// EFI Memory Attributes Table
    pub const MEMORY_ATTRIBUTES: Guid = Guid::new(0xDCFA911D, 0x26EB, 0x469F, [
        0xA2, 0x20, 0x38, 0xB7, 0xDC, 0x46, 0x12, 0x20,
    ]);

    /// RT Properties Table
    pub const RT_PROPERTIES: Guid = Guid::new(0xEB66918A, 0x7EEF, 0x402A, [
        0x84, 0x2E, 0x93, 0x1D, 0x21, 0xC3, 0x8A, 0xE9,
    ]);

    /// EFI System Resource Table
    pub const ESRT: Guid = Guid::new(0xB122A263, 0x3661, 0x4F68, [
        0x99, 0x29, 0x78, 0xF8, 0xB0, 0xD6, 0x21, 0x80,
    ]);

    /// Debug Image Info Table
    pub const DEBUG_IMAGE_INFO: Guid = Guid::new(0x49152E77, 0x1ADA, 0x4764, [
        0xB7, 0xA2, 0x7A, 0xFE, 0xFE, 0xD9, 0x5E, 0x8B,
    ]);

    /// DXE Services Table
    pub const DXE_SERVICES: Guid = Guid::new(0x05AD34BA, 0x6F02, 0x4214, [
        0x95, 0x2E, 0x4D, 0xA0, 0x39, 0x8E, 0x2B, 0xB9,
    ]);

    /// HOB List
    pub const HOB_LIST: Guid = Guid::new(0x7739F24C, 0x93D7, 0x11D4, [
        0x9A, 0x3A, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    /// LZMA Decompressor
    pub const LZMA_CUSTOM_DECOMPRESS: Guid = Guid::new(0xEE4E5898, 0x3914, 0x4259, [
        0x9D, 0x6E, 0xDC, 0x7B, 0xD7, 0x94, 0x03, 0xCF,
    ]);

    /// TCG Event Log (TPM 1.2)
    pub const TCG_EVENT_LOG: Guid = Guid::new(0xF14BBE01, 0xECB0, 0x4E27, [
        0xBC, 0x26, 0x7F, 0x97, 0xF6, 0x32, 0xC6, 0x09,
    ]);

    /// TCG Event Log (TPM 2.0)
    pub const TCG2_EVENT_LOG: Guid = Guid::new(0x1E2ED096, 0x30E2, 0x4254, [
        0xBD, 0x89, 0x86, 0x3B, 0xBE, 0xA8, 0x76, 0x26,
    ]);
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acpi_version() {
        assert!(AcpiVersion::V6_5 > AcpiVersion::V5_0);
        assert_eq!(AcpiVersion::new(6, 5), AcpiVersion::V6_5);
    }

    #[test]
    fn test_smbios_version() {
        let v = SmbiosVersion::new(3, 4);
        assert_eq!(v.major, 3);
        assert_eq!(v.minor, 4);
    }

    #[test]
    fn test_system_info() {
        let mut info = SystemInfo::new();
        info.total_memory = 16 * 1024 * 1024 * 1024; // 16 GB
        assert_eq!(info.memory_string(), "16 GB");
    }
}
