//! Configuration Table Management
//!
//! Unified interface for accessing and managing UEFI configuration tables.

use crate::error::Result;
use crate::raw::types::*;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// CONFIGURATION TABLE MANAGER
// =============================================================================

/// Configuration table manager
pub struct ConfigTableManager {
    /// Parsed tables
    tables: Vec<ConfigTable>,
    /// ACPI info
    acpi: Option<AcpiConfig>,
    /// SMBIOS info
    smbios: Option<SmbiosConfig>,
    /// Device tree info
    dtb: Option<DtbConfig>,
    /// Debug configuration
    debug: Option<DebugConfig>,
}

impl ConfigTableManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            tables: Vec::new(),
            acpi: None,
            smbios: None,
            dtb: None,
            debug: None,
        }
    }

    /// Initialize from system table
    ///
    /// # Safety
    ///
    /// The caller must ensure system is in a valid state for initialization.
    pub unsafe fn init_from_system_table(
        &mut self,
        config_table: *const EfiConfigurationTable,
        count: usize,
    ) -> Result<()> {
        for i in 0..count {
            let entry = &*config_table.add(i);
            self.add_table(
                entry.vendor_guid,
                PhysicalAddress(entry.vendor_table as u64),
            )?;
        }

        // Parse known tables
        self.parse_acpi()?;
        self.parse_smbios()?;
        self.parse_dtb()?;
        self.parse_debug()?;

        Ok(())
    }

    /// Add table
    #[allow(clippy::unnecessary_wraps)]
    fn add_table(&mut self, guid: Guid, address: PhysicalAddress) -> Result<()> {
        let table_type = ConfigTableType::from_guid(&guid);

        self.tables.push(ConfigTable {
            guid,
            address,
            table_type,
        });

        Ok(())
    }

    /// Parse ACPI configuration
    #[allow(clippy::unnecessary_wraps)]
    unsafe fn parse_acpi(&mut self) -> Result<()> {
        // Try ACPI 2.0+ first
        let rsdp_addr = self
            .find_table_by_type(ConfigTableType::Acpi20)
            .or_else(|| self.find_table_by_type(ConfigTableType::Acpi10));

        if let Some(addr) = rsdp_addr {
            let rsdp = &*(addr.0 as *const Rsdp);

            if &rsdp.signature != b"RSD PTR " {
                return Ok(());
            }

            let version = if rsdp.revision >= 2 {
                super::AcpiVersion::new(2, rsdp.revision - 1)
            } else {
                super::AcpiVersion::V1_0
            };

            let mut oem_id = [0u8; 6];
            oem_id.copy_from_slice(&rsdp.oem_id);

            let rsdt_address = rsdp.rsdt_address as u64;

            let xsdt_address = if rsdp.revision >= 2 {
                let rsdp2 = &*(addr.0 as *const Rsdp2);
                Some(rsdp2.xsdt_address)
            } else {
                None
            };

            self.acpi = Some(AcpiConfig {
                version,
                rsdp_address: addr,
                rsdt_address,
                xsdt_address,
                oem_id,
            });
        }

        Ok(())
    }

    /// Parse SMBIOS configuration
    #[allow(clippy::unnecessary_wraps)]
    unsafe fn parse_smbios(&mut self) -> Result<()> {
        // Try SMBIOS 3.x first
        let entry_addr = self
            .find_table_by_type(ConfigTableType::Smbios3)
            .or_else(|| self.find_table_by_type(ConfigTableType::Smbios));

        if let Some(addr) = entry_addr {
            let ptr = addr.0 as *const u8;
            let anchor = core::slice::from_raw_parts(ptr, 5);

            if anchor == b"_SM3_" {
                let ep = &*(addr.0 as *const SmbiosEntryPoint3);
                self.smbios = Some(SmbiosConfig {
                    version: super::SmbiosVersion::new(ep.major_version, ep.minor_version),
                    entry_point_address: addr,
                    table_address: ep.structure_table_address,
                    table_length: ep.structure_table_max_size as usize,
                    is_64bit: true,
                });
            } else {
                let anchor4 = core::slice::from_raw_parts(ptr, 4);
                if anchor4 == b"_SM_" {
                    let ep = &*(addr.0 as *const SmbiosEntryPoint);
                    self.smbios = Some(SmbiosConfig {
                        version: super::SmbiosVersion::new(ep.major_version, ep.minor_version),
                        entry_point_address: addr,
                        table_address: ep.structure_table_address as u64,
                        table_length: ep.structure_table_length as usize,
                        is_64bit: false,
                    });
                }
            }
        }

        Ok(())
    }

    /// Parse device tree blob configuration
    #[allow(clippy::unnecessary_wraps)]
    unsafe fn parse_dtb(&mut self) -> Result<()> {
        if let Some(addr) = self.find_table_by_type(ConfigTableType::Dtb) {
            let header = &*(addr.0 as *const DtbHeader);

            // Check magic (big-endian: 0xd00d_feed)
            if header.magic == 0xd00d_feed || header.magic.swap_bytes() == 0xd00d_feed {
                let size = if header.magic == 0xd00d_feed {
                    header.totalsize
                } else {
                    header.totalsize.swap_bytes()
                };

                let version = if header.magic == 0xd00d_feed {
                    header.version
                } else {
                    header.version.swap_bytes()
                };

                self.dtb = Some(DtbConfig {
                    address: addr,
                    size: size as usize,
                    version,
                });
            }
        }

        Ok(())
    }

    /// Parse debug configuration
    #[allow(clippy::unnecessary_wraps)]
    unsafe fn parse_debug(&mut self) -> Result<()> {
        if let Some(addr) = self.find_table_by_type(ConfigTableType::DebugImageInfo) {
            self.debug = Some(DebugConfig {
                debug_info_address: addr,
            });
        }

        Ok(())
    }

    /// Find table by type
    pub fn find_table_by_type(&self, table_type: ConfigTableType) -> Option<PhysicalAddress> {
        self.tables
            .iter()
            .find(|t| t.table_type == table_type)
            .map(|t| t.address)
    }

    /// Find table by GUID
    pub fn find_table_by_guid(&self, guid: &Guid) -> Option<PhysicalAddress> {
        self.tables
            .iter()
            .find(|t| &t.guid == guid)
            .map(|t| t.address)
    }

    /// Get all tables
    pub fn tables(&self) -> &[ConfigTable] {
        &self.tables
    }

    /// Get table count
    pub fn table_count(&self) -> usize {
        self.tables.len()
    }

    /// Get ACPI configuration
    pub fn acpi(&self) -> Option<&AcpiConfig> {
        self.acpi.as_ref()
    }

    /// Get SMBIOS configuration
    pub fn smbios(&self) -> Option<&SmbiosConfig> {
        self.smbios.as_ref()
    }

    /// Get DTB configuration
    pub fn dtb(&self) -> Option<&DtbConfig> {
        self.dtb.as_ref()
    }

    /// Get debug configuration
    pub fn debug(&self) -> Option<&DebugConfig> {
        self.debug.as_ref()
    }

    /// Check if ACPI is available
    pub fn has_acpi(&self) -> bool {
        self.acpi.is_some()
    }

    /// Check if SMBIOS is available
    pub fn has_smbios(&self) -> bool {
        self.smbios.is_some()
    }

    /// Check if DTB is available
    pub fn has_dtb(&self) -> bool {
        self.dtb.is_some()
    }
}

impl Default for ConfigTableManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// RAW STRUCTURES
// =============================================================================

/// EFI Configuration Table entry
#[repr(C)]
pub struct EfiConfigurationTable {
    pub vendor_guid: Guid,
    pub vendor_table: *const core::ffi::c_void,
}

/// RSDP (ACPI 1.0)
#[repr(C, packed)]
struct Rsdp {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
}

/// RSDP (ACPI 2.0+)
#[repr(C, packed)]
struct Rsdp2 {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

/// SMBIOS Entry Point (2.x)
#[repr(C, packed)]
struct SmbiosEntryPoint {
    anchor_string: [u8; 4],
    entry_point_checksum: u8,
    entry_point_length: u8,
    major_version: u8,
    minor_version: u8,
    max_structure_size: u16,
    entry_point_revision: u8,
    formatted_area: [u8; 5],
    intermediate_anchor: [u8; 5],
    intermediate_checksum: u8,
    structure_table_length: u16,
    structure_table_address: u32,
    number_of_structures: u16,
    bcd_revision: u8,
}

/// SMBIOS Entry Point (3.x)
#[repr(C, packed)]
struct SmbiosEntryPoint3 {
    anchor_string: [u8; 5],
    entry_point_checksum: u8,
    entry_point_length: u8,
    major_version: u8,
    minor_version: u8,
    docrev: u8,
    entry_point_revision: u8,
    reserved: u8,
    structure_table_max_size: u32,
    structure_table_address: u64,
}

/// DTB Header
#[repr(C)]
struct DtbHeader {
    magic: u32,
    totalsize: u32,
    off_dt_struct: u32,
    off_dt_strings: u32,
    off_mem_rsvmap: u32,
    version: u32,
    last_comp_version: u32,
    boot_cpuid_phys: u32,
    size_dt_strings: u32,
    size_dt_struct: u32,
}

// =============================================================================
// PARSED STRUCTURES
// =============================================================================

/// Configuration table entry
#[derive(Debug, Clone)]
pub struct ConfigTable {
    /// GUID
    pub guid: Guid,
    /// Table address
    pub address: PhysicalAddress,
    /// Table type
    pub table_type: ConfigTableType,
}

impl ConfigTable {
    /// Get GUID as string
    pub fn guid_string(&self) -> String {
        alloc::format!(
            "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            self.guid.data1,
            self.guid.data2,
            self.guid.data3,
            self.guid.data4[0],
            self.guid.data4[1],
            self.guid.data4[2],
            self.guid.data4[3],
            self.guid.data4[4],
            self.guid.data4[5],
            self.guid.data4[6],
            self.guid.data4[7],
        )
    }
}

/// Configuration table type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigTableType {
    /// ACPI 1.0 RSDP
    Acpi10,
    /// ACPI 2.0+ RSDP
    Acpi20,
    /// SMBIOS 2.x
    Smbios,
    /// SMBIOS 3.x
    Smbios3,
    /// SAL System Table
    Sal,
    /// MPS Table
    Mps,
    /// Device Tree Blob
    Dtb,
    /// Debug Image Info Table
    DebugImageInfo,
    /// LZMA Compressed Filesystem
    LzmaCompress,
    /// DXE Services Table
    DxeServices,
    /// HOB List
    HobList,
    /// Memory Type Information
    MemoryTypeInfo,
    /// Memory Attributes Table
    MemoryAttributesTable,
    /// Conformance Profiles
    ConformanceProfiles,
    /// RT Properties Table
    RtProperties,
    /// JSON Capsule Data
    JsonConfigData,
    /// JSON Capsule Result
    JsonConfigResult,
    /// Memory Range Capsule
    MemoryRangeCapsule,
    /// TCG Event Log (EFI_TCG2_EVENT_LOG_FORMAT_TCG_2)
    Tcg2EventLog,
    /// Unknown vendor table
    Unknown,
}

impl ConfigTableType {
    /// Determine type from GUID
    pub fn from_guid(guid: &Guid) -> Self {
        // Well-known UEFI configuration table GUIDs
        match (guid.data1, guid.data2, guid.data3, &guid.data4[..]) {
            // ACPI 1.0
            (0xeb9d_2d30, 0x2d88, 0x11d3, &[0x9a, 0x16, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d]) => {
                Self::Acpi10
            },
            // ACPI 2.0
            (0x8868_e871, 0xe4f1, 0x11d3, &[0xbc, 0x22, 0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81]) => {
                Self::Acpi20
            },
            // SMBIOS
            (0xeb9d_2d31, 0x2d88, 0x11d3, &[0x9a, 0x16, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d]) => {
                Self::Smbios
            },
            // SMBIOS 3.x
            (0xf2fd_1544, 0x9794, 0x4a2c, &[0x99, 0x2e, 0xe5, 0xbb, 0xcf, 0x20, 0xe3, 0x94]) => {
                Self::Smbios3
            },
            // SAL
            (0xeb9d_2d32, 0x2d88, 0x11d3, &[0x9a, 0x16, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d]) => {
                Self::Sal
            },
            // MPS
            (0xeb9d_2d2f, 0x2d88, 0x11d3, &[0x9a, 0x16, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d]) => {
                Self::Mps
            },
            // DTB
            (0xb1b6_21d5, 0xf19c, 0x41a5, &[0x83, 0x0b, 0xd9, 0x15, 0x2c, 0x69, 0xaa, 0xe0]) => {
                Self::Dtb
            },
            // Debug Image Info
            (0x4915_2e77, 0x1ada, 0x4764, &[0xb7, 0xa2, 0x7a, 0xfe, 0xfe, 0xd9, 0x5e, 0x8b]) => {
                Self::DebugImageInfo
            },
            // LZMA Compress
            (0xee4e_5898, 0x3914, 0x4259, &[0x9d, 0x6e, 0xdc, 0x7b, 0xd7, 0x94, 0x03, 0xcf]) => {
                Self::LzmaCompress
            },
            // DXE Services
            (0x05ad_34ba, 0x6f02, 0x4214, &[0x95, 0x2e, 0x4d, 0xa0, 0x39, 0x8e, 0x2b, 0xb9]) => {
                Self::DxeServices
            },
            // HOB List
            (0x7739_f24c, 0x93d7, 0x11d4, &[0x9a, 0x3a, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d]) => {
                Self::HobList
            },
            // Memory Type Info
            (0x4c19_049f, 0x4137, 0x4dd3, &[0x9c, 0x10, 0x8b, 0x97, 0xa8, 0x3f, 0xfd, 0xfa]) => {
                Self::MemoryTypeInfo
            },
            // Memory Attributes Table
            (0xdcfa_911d, 0x26eb, 0x469f, &[0xa2, 0x20, 0x38, 0xb7, 0xdc, 0x46, 0x12, 0x20]) => {
                Self::MemoryAttributesTable
            },
            // Conformance Profiles
            (0x3612_2546, 0xf7ef, 0x4c8f, &[0xbd, 0x9b, 0xeb, 0x85, 0x25, 0xb5, 0x0c, 0x0b]) => {
                Self::ConformanceProfiles
            },
            // RT Properties
            (0xeb66_918a, 0x7eef, 0x402a, &[0x84, 0x2e, 0x93, 0x1d, 0x21, 0xc3, 0x8a, 0xe9]) => {
                Self::RtProperties
            },
            // JSON Config Data
            (0x8736_7f87, 0x1119, 0x41ce, &[0xaa, 0xec, 0x8b, 0xe0, 0x11, 0x1f, 0x55, 0x8a]) => {
                Self::JsonConfigData
            },
            // JSON Config Result
            (0xdbc4_61c3, 0xb3de, 0x422a, &[0xb9, 0xb4, 0x98, 0x86, 0xfd, 0x49, 0xa1, 0xe5]) => {
                Self::JsonConfigResult
            },
            // Memory Range Capsule
            (0x0de9_f0ec, 0x88b6, 0x428f, &[0x97, 0x7a, 0x25, 0x8f, 0x1d, 0x0e, 0x5e, 0x72]) => {
                Self::MemoryRangeCapsule
            },
            _ => Self::Unknown,
        }
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Acpi10 => "ACPI 1.0 RSDP",
            Self::Acpi20 => "ACPI 2.0+ RSDP",
            Self::Smbios => "SMBIOS 2.x",
            Self::Smbios3 => "SMBIOS 3.x",
            Self::Sal => "SAL System Table",
            Self::Mps => "MPS Table",
            Self::Dtb => "Device Tree Blob",
            Self::DebugImageInfo => "Debug Image Info",
            Self::LzmaCompress => "LZMA Compress",
            Self::DxeServices => "DXE Services",
            Self::HobList => "HOB List",
            Self::MemoryTypeInfo => "Memory Type Info",
            Self::MemoryAttributesTable => "Memory Attributes Table",
            Self::ConformanceProfiles => "Conformance Profiles",
            Self::RtProperties => "RT Properties",
            Self::JsonConfigData => "JSON Config Data",
            Self::JsonConfigResult => "JSON Config Result",
            Self::MemoryRangeCapsule => "Memory Range Capsule",
            Self::Tcg2EventLog => "TCG2 Event Log",
            Self::Unknown => "Unknown",
        }
    }
}

/// ACPI configuration
#[derive(Debug, Clone)]
pub struct AcpiConfig {
    /// ACPI version
    pub version: super::AcpiVersion,
    /// RSDP address
    pub rsdp_address: PhysicalAddress,
    /// RSDT address
    pub rsdt_address: u64,
    /// XSDT address (ACPI 2.0+)
    pub xsdt_address: Option<u64>,
    /// OEM ID
    pub oem_id: [u8; 6],
}

impl AcpiConfig {
    /// Get OEM ID as string
    pub fn oem_id_string(&self) -> &str {
        core::str::from_utf8(&self.oem_id).unwrap_or("").trim()
    }

    /// Check if ACPI 2.0+
    pub fn is_acpi2(&self) -> bool {
        self.xsdt_address.is_some()
    }
}

/// SMBIOS configuration
#[derive(Debug, Clone)]
pub struct SmbiosConfig {
    /// SMBIOS version
    pub version: super::SmbiosVersion,
    /// Entry point address
    pub entry_point_address: PhysicalAddress,
    /// Table address
    pub table_address: u64,
    /// Table length
    pub table_length: usize,
    /// 64-bit entry point (SMBIOS 3.x)
    pub is_64bit: bool,
}

impl SmbiosConfig {
    /// Check if SMBIOS 3.x
    pub fn is_smbios3(&self) -> bool {
        self.is_64bit
    }
}

/// Device Tree Blob configuration
#[derive(Debug, Clone)]
pub struct DtbConfig {
    /// DTB address
    pub address: PhysicalAddress,
    /// DTB size
    pub size: usize,
    /// FDT version
    pub version: u32,
}

/// Debug configuration
#[derive(Debug, Clone)]
pub struct DebugConfig {
    /// Debug info table address
    pub debug_info_address: PhysicalAddress,
}

// =============================================================================
// TABLE ITERATOR
// =============================================================================

/// Iterator over configuration tables
pub struct ConfigTableIterator<'a> {
    tables: &'a [ConfigTable],
    index: usize,
}

impl<'a> ConfigTableIterator<'a> {
    /// Create new iterator
    pub fn new(tables: &'a [ConfigTable]) -> Self {
        Self { tables, index: 0 }
    }
}

impl<'a> Iterator for ConfigTableIterator<'a> {
    type Item = &'a ConfigTable;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.tables.len() {
            let table = &self.tables[self.index];
            self.index += 1;
            Some(table)
        } else {
            None
        }
    }
}

// =============================================================================
// TABLE REGISTRY
// =============================================================================

/// Table registration and lookup
pub struct TableRegistry {
    /// Registered tables
    entries: Vec<TableEntry>,
}

impl TableRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Register a table
    pub fn register(&mut self, name: &str, guid: Guid, address: PhysicalAddress) {
        self.entries.push(TableEntry {
            name: String::from(name),
            guid,
            address,
        });
    }

    /// Lookup table by name
    pub fn lookup_by_name(&self, name: &str) -> Option<&TableEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// Lookup table by GUID
    pub fn lookup_by_guid(&self, guid: &Guid) -> Option<&TableEntry> {
        self.entries.iter().find(|e| &e.guid == guid)
    }

    /// Get all entries
    pub fn entries(&self) -> &[TableEntry] {
        &self.entries
    }
}

impl Default for TableRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry entry
#[derive(Debug, Clone)]
pub struct TableEntry {
    /// Table name
    pub name: String,
    /// GUID
    pub guid: Guid,
    /// Address
    pub address: PhysicalAddress,
}

// =============================================================================
// SYSTEM INFORMATION AGGREGATOR
// =============================================================================

/// Aggregated system information from all tables
#[derive(Debug, Clone)]
pub struct SystemInfoAggregator {
    /// Firmware vendor
    pub firmware_vendor: Option<String>,
    /// Firmware version
    pub firmware_version: Option<String>,
    /// System manufacturer
    pub system_manufacturer: Option<String>,
    /// System product
    pub system_product: Option<String>,
    /// System version
    pub system_version: Option<String>,
    /// System serial
    pub system_serial: Option<String>,
    /// System UUID
    pub system_uuid: Option<[u8; 16]>,
    /// BIOS vendor
    pub bios_vendor: Option<String>,
    /// BIOS version
    pub bios_version: Option<String>,
    /// BIOS date
    pub bios_date: Option<String>,
    /// Baseboard manufacturer
    pub baseboard_manufacturer: Option<String>,
    /// Baseboard product
    pub baseboard_product: Option<String>,
    /// Total installed memory (MB)
    pub total_memory_mb: u64,
    /// CPU count
    pub cpu_count: u32,
    /// Total cores
    pub total_cores: u32,
    /// Total threads
    pub total_threads: u32,
    /// ACPI version
    pub acpi_version: Option<super::AcpiVersion>,
    /// SMBIOS version
    pub smbios_version: Option<super::SmbiosVersion>,
}

impl SystemInfoAggregator {
    /// Create empty aggregator
    pub fn new() -> Self {
        Self {
            firmware_vendor: None,
            firmware_version: None,
            system_manufacturer: None,
            system_product: None,
            system_version: None,
            system_serial: None,
            system_uuid: None,
            bios_vendor: None,
            bios_version: None,
            bios_date: None,
            baseboard_manufacturer: None,
            baseboard_product: None,
            total_memory_mb: 0,
            cpu_count: 0,
            total_cores: 0,
            total_threads: 0,
            acpi_version: None,
            smbios_version: None,
        }
    }

    /// Populate from SMBIOS parser
    pub fn populate_from_smbios(&mut self, smbios: &super::smbios::SmbiosParser) {
        self.smbios_version = Some(smbios.version());

        if let Some(bios) = smbios.bios_info() {
            self.bios_vendor = Some(bios.vendor.clone());
            self.bios_version = Some(bios.version.clone());
            self.bios_date = Some(bios.release_date.clone());
        }

        if let Some(system) = smbios.system_info() {
            self.system_manufacturer = Some(system.manufacturer.clone());
            self.system_product = Some(system.product_name.clone());
            self.system_version = Some(system.version.clone());
            self.system_serial = Some(system.serial_number.clone());
            self.system_uuid = system.uuid;
        }

        if let Some(baseboard) = smbios.baseboard_info() {
            self.baseboard_manufacturer = Some(baseboard.manufacturer.clone());
            self.baseboard_product = Some(baseboard.product.clone());
        }

        self.total_memory_mb = smbios.total_memory_mb();
        self.cpu_count = smbios.cpu_count() as u32;
        self.total_cores = smbios.total_cores();
        self.total_threads = smbios.total_threads();
    }

    /// Populate from ACPI parser
    pub fn populate_from_acpi(&mut self, acpi: &super::acpi::AcpiParser) {
        self.acpi_version = Some(acpi.version());

        // MADT can provide CPU info
        if let Some(madt) = acpi.madt() {
            let cpu_count = madt.enabled_cpu_count() as u32;
            if self.cpu_count == 0 {
                self.cpu_count = cpu_count;
                self.total_cores = cpu_count;
                self.total_threads = cpu_count;
            }
        }
    }
}

impl Default for SystemInfoAggregator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_config_table_type() {
        // ACPI 2.0
        let acpi2_guid = Guid {
            data1: 0x8868_e871,
            data2: 0xe4f1,
            data3: 0x11d3,
            data4: [0xbc, 0x22, 0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81],
        };
        assert_eq!(
            ConfigTableType::from_guid(&acpi2_guid),
            ConfigTableType::Acpi20
        );

        // SMBIOS 3
        let smbios3_guid = Guid {
            data1: 0xf2fd_1544,
            data2: 0x9794,
            data3: 0x4a2c,
            data4: [0x99, 0x2e, 0xe5, 0xbb, 0xcf, 0x20, 0xe3, 0x94],
        };
        assert_eq!(
            ConfigTableType::from_guid(&smbios3_guid),
            ConfigTableType::Smbios3
        );
    }

    #[test]
    fn test_table_type_name() {
        assert_eq!(ConfigTableType::Acpi20.name(), "ACPI 2.0+ RSDP");
        assert_eq!(ConfigTableType::Smbios3.name(), "SMBIOS 3.x");
    }
}
