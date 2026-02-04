//! SMBIOS Protocol
//!
//! High-level SMBIOS table access abstraction.

use super::Protocol;
use crate::error::{Error, Result};
use crate::raw::types::*;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use core::ptr;

// =============================================================================
// SMBIOS TABLES
// =============================================================================

/// SMBIOS table accessor
pub struct SmbiosTables {
    /// Handle
    handle: Handle,
    /// Entry point address
    entry_point_address: PhysicalAddress,
    /// SMBIOS version
    version: SmbiosVersion,
    /// Table address
    table_address: PhysicalAddress,
    /// Table length
    table_length: u32,
    /// Number of structures
    structure_count: u16,
    /// Cached structures
    structures: Vec<SmbiosStructure>,
}

impl SmbiosTables {
    /// Create new SMBIOS tables accessor
    pub fn new(handle: Handle) -> Self {
        Self {
            handle,
            entry_point_address: PhysicalAddress(0),
            version: SmbiosVersion::new(0, 0),
            table_address: PhysicalAddress(0),
            table_length: 0,
            structure_count: 0,
            structures: Vec::new(),
        }
    }

    /// Get SMBIOS version
    pub fn version(&self) -> SmbiosVersion {
        self.version
    }

    /// Get table address
    pub fn table_address(&self) -> PhysicalAddress {
        self.table_address
    }

    /// Get table length
    pub fn table_length(&self) -> u32 {
        self.table_length
    }

    /// Get structure count
    pub fn structure_count(&self) -> u16 {
        self.structure_count
    }

    /// Initialize from SMBIOS entry point (32-bit)
    pub unsafe fn init_from_entry_point(&mut self, entry: *const SmbiosEntryPoint) -> Result<()> {
        if entry.is_null() {
            return Err(Error::InvalidParameter);
        }

        let entry_ref = &*entry;

        // Validate signature
        if &entry_ref.anchor != b"_SM_" {
            return Err(Error::InvalidParameter);
        }

        // Validate checksum
        if !entry_ref.validate_checksum() {
            return Err(Error::CrcError);
        }

        self.entry_point_address = PhysicalAddress(entry as u64);
        self.version = SmbiosVersion::new(entry_ref.major_version, entry_ref.minor_version);
        self.table_address = PhysicalAddress(entry_ref.table_address as u64);
        self.table_length = entry_ref.table_length as u32;
        self.structure_count = entry_ref.structure_count;

        // Parse structures
        self.parse_structures()?;

        Ok(())
    }

    /// Initialize from SMBIOS 3.0 entry point (64-bit)
    pub unsafe fn init_from_entry_point3(&mut self, entry: *const SmbiosEntryPoint3) -> Result<()> {
        if entry.is_null() {
            return Err(Error::InvalidParameter);
        }

        let entry_ref = &*entry;

        // Validate signature
        if &entry_ref.anchor != b"_SM3_" {
            return Err(Error::InvalidParameter);
        }

        // Validate checksum
        if !entry_ref.validate_checksum() {
            return Err(Error::CrcError);
        }

        self.entry_point_address = PhysicalAddress(entry as u64);
        self.version = SmbiosVersion::new(entry_ref.major_version, entry_ref.minor_version);
        self.table_address = PhysicalAddress(entry_ref.table_address);
        self.table_length = entry_ref.table_max_size;
        self.structure_count = 0; // Unknown in 3.0

        // Parse structures
        self.parse_structures()?;

        Ok(())
    }

    /// Parse structures from table
    unsafe fn parse_structures(&mut self) -> Result<()> {
        let end = PhysicalAddress(self.table_address.0 + self.table_length as u64);
        let mut ptr = self.table_address;

        while ptr < end {
            let header = &*(ptr.0 as *const SmbiosHeader);

            // End of table marker
            if header.structure_type == 127 {
                // Add the end structure
                self.structures.push(SmbiosStructure {
                    structure_type: header.structure_type,
                    handle: header.handle,
                    address: ptr,
                    length: header.length,
                    strings: Vec::new(),
                });
                break;
            }

            // Find strings section (after header.length bytes)
            let strings_start = ptr + header.length as u64;
            let strings = self.parse_strings(strings_start, end)?;
            let strings_len = self.calculate_strings_length(strings_start, end);

            self.structures.push(SmbiosStructure {
                structure_type: header.structure_type,
                handle: header.handle,
                address: ptr,
                length: header.length,
                strings,
            });

            // Move to next structure
            ptr = strings_start + strings_len as u64;
        }

        Ok(())
    }

    /// Parse strings from structure
    unsafe fn parse_strings(
        &self,
        start: PhysicalAddress,
        end: PhysicalAddress,
    ) -> Result<Vec<String>> {
        let mut strings = Vec::new();
        let mut ptr = start;

        while ptr < end {
            let byte = *(ptr.0 as *const u8);

            if byte == 0 {
                ptr += 1;
                // Double null = end of strings
                if ptr < end && *(ptr.0 as *const u8) == 0 {
                    break;
                }
                continue;
            }

            // Find end of string
            let mut str_end = ptr;
            while str_end < end && *(str_end.0 as *const u8) != 0 {
                str_end += 1;
            }

            let len = (str_end - ptr) as usize;
            let slice = core::slice::from_raw_parts(ptr.0 as *const u8, len);
            if let Ok(s) = core::str::from_utf8(slice) {
                strings.push(String::from(s));
            }

            ptr = str_end;
        }

        Ok(strings)
    }

    /// Calculate strings section length
    unsafe fn calculate_strings_length(
        &self,
        start: PhysicalAddress,
        end: PhysicalAddress,
    ) -> usize {
        let mut ptr = start;

        while ptr < end {
            let byte = *(ptr.0 as *const u8);
            ptr += 1;

            if byte == 0 {
                if ptr < end && *(ptr.0 as *const u8) == 0 {
                    ptr += 1;
                    break;
                }
            }
        }

        (ptr - start) as usize
    }

    /// Get all structures
    pub fn structures(&self) -> &[SmbiosStructure] {
        &self.structures
    }

    /// Find structures by type
    pub fn find_structures(&self, structure_type: u8) -> Vec<&SmbiosStructure> {
        self.structures
            .iter()
            .filter(|s| s.structure_type == structure_type)
            .collect()
    }

    /// Find first structure by type
    pub fn find_structure(&self, structure_type: u8) -> Option<&SmbiosStructure> {
        self.structures
            .iter()
            .find(|s| s.structure_type == structure_type)
    }

    /// Get BIOS information (Type 0)
    pub fn bios_info(&self) -> Option<BiosInfo> {
        self.find_structure(0)
            .map(|s| unsafe { BiosInfo::from_structure(s) })
    }

    /// Get system information (Type 1)
    pub fn system_info(&self) -> Option<SystemInfo> {
        self.find_structure(1)
            .map(|s| unsafe { SystemInfo::from_structure(s) })
    }

    /// Get baseboard information (Type 2)
    pub fn baseboard_info(&self) -> Option<BaseboardInfo> {
        self.find_structure(2)
            .map(|s| unsafe { BaseboardInfo::from_structure(s) })
    }

    /// Get chassis information (Type 3)
    pub fn chassis_info(&self) -> Option<ChassisInfo> {
        self.find_structure(3)
            .map(|s| unsafe { ChassisInfo::from_structure(s) })
    }

    /// Get processor information (Type 4)
    pub fn processor_info(&self) -> Vec<ProcessorInfo> {
        self.find_structures(4)
            .iter()
            .map(|s| unsafe { ProcessorInfo::from_structure(s) })
            .collect()
    }

    /// Get memory device information (Type 17)
    pub fn memory_devices(&self) -> Vec<MemoryDeviceInfo> {
        self.find_structures(17)
            .iter()
            .map(|s| unsafe { MemoryDeviceInfo::from_structure(s) })
            .collect()
    }

    /// Get total installed memory in bytes
    pub fn total_memory(&self) -> u64 {
        self.memory_devices().iter().filter_map(|d| d.size).sum()
    }
}

impl Protocol for SmbiosTables {
    const GUID: Guid = Guid::new(0xF2FD1544, 0x9794, 0x4A2C, [
        0x99, 0x2E, 0xE5, 0xBB, 0xCF, 0x20, 0xE3, 0x94,
    ]);

    fn open(handle: Handle) -> Result<Self> {
        Ok(Self::new(handle))
    }
}

// =============================================================================
// SMBIOS VERSION
// =============================================================================

/// SMBIOS version
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

    /// SMBIOS 2.0
    pub const V2_0: Self = Self::new(2, 0);
    /// SMBIOS 2.1
    pub const V2_1: Self = Self::new(2, 1);
    /// SMBIOS 2.4
    pub const V2_4: Self = Self::new(2, 4);
    /// SMBIOS 2.7
    pub const V2_7: Self = Self::new(2, 7);
    /// SMBIOS 3.0
    pub const V3_0: Self = Self::new(3, 0);
    /// SMBIOS 3.2
    pub const V3_2: Self = Self::new(3, 2);
    /// SMBIOS 3.4
    pub const V3_4: Self = Self::new(3, 4);
}

impl core::fmt::Display for SmbiosVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

// =============================================================================
// ENTRY POINTS
// =============================================================================

/// SMBIOS 2.1 Entry Point (32-bit)
#[repr(C, packed)]
pub struct SmbiosEntryPoint {
    /// Anchor string ("_SM_")
    pub anchor: [u8; 4],
    /// Checksum
    pub checksum: u8,
    /// Entry point length
    pub length: u8,
    /// Major version
    pub major_version: u8,
    /// Minor version
    pub minor_version: u8,
    /// Maximum structure size
    pub max_structure_size: u16,
    /// Entry point revision
    pub revision: u8,
    /// Formatted area
    pub formatted_area: [u8; 5],
    /// Intermediate anchor ("_DMI_")
    pub intermediate_anchor: [u8; 5],
    /// Intermediate checksum
    pub intermediate_checksum: u8,
    /// Table length
    pub table_length: u16,
    /// Table address
    pub table_address: u32,
    /// Number of structures
    pub structure_count: u16,
    /// BCD revision
    pub bcd_revision: u8,
}

impl SmbiosEntryPoint {
    /// Validate checksum
    pub fn validate_checksum(&self) -> bool {
        let bytes = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, self.length as usize)
        };

        bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b)) == 0
    }
}

/// SMBIOS 3.0 Entry Point (64-bit)
#[repr(C, packed)]
pub struct SmbiosEntryPoint3 {
    /// Anchor string ("_SM3_")
    pub anchor: [u8; 5],
    /// Checksum
    pub checksum: u8,
    /// Entry point length
    pub length: u8,
    /// Major version
    pub major_version: u8,
    /// Minor version
    pub minor_version: u8,
    /// Docrev
    pub docrev: u8,
    /// Entry point revision
    pub revision: u8,
    /// Reserved
    pub reserved: u8,
    /// Maximum table size
    pub table_max_size: u32,
    /// Table address
    pub table_address: u64,
}

impl SmbiosEntryPoint3 {
    /// Validate checksum
    pub fn validate_checksum(&self) -> bool {
        let bytes = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, self.length as usize)
        };

        bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b)) == 0
    }
}

// =============================================================================
// SMBIOS HEADER
// =============================================================================

/// SMBIOS structure header
#[repr(C, packed)]
pub struct SmbiosHeader {
    /// Structure type
    pub structure_type: u8,
    /// Length (excluding strings)
    pub length: u8,
    /// Handle
    pub handle: u16,
}

// =============================================================================
// SMBIOS STRUCTURE
// =============================================================================

/// Parsed SMBIOS structure
#[derive(Debug, Clone)]
pub struct SmbiosStructure {
    /// Structure type
    pub structure_type: u8,
    /// Handle
    pub handle: u16,
    /// Address
    pub address: PhysicalAddress,
    /// Length (excluding strings)
    pub length: u8,
    /// Strings
    pub strings: Vec<String>,
}

impl SmbiosStructure {
    /// Get structure type name
    pub fn type_name(&self) -> &'static str {
        match self.structure_type {
            0 => "BIOS Information",
            1 => "System Information",
            2 => "Baseboard Information",
            3 => "System Enclosure",
            4 => "Processor Information",
            5 => "Memory Controller Information",
            6 => "Memory Module Information",
            7 => "Cache Information",
            8 => "Port Connector Information",
            9 => "System Slots",
            10 => "On Board Devices Information",
            11 => "OEM Strings",
            12 => "System Configuration Options",
            13 => "BIOS Language Information",
            14 => "Group Associations",
            15 => "System Event Log",
            16 => "Physical Memory Array",
            17 => "Memory Device",
            18 => "32-bit Memory Error Information",
            19 => "Memory Array Mapped Address",
            20 => "Memory Device Mapped Address",
            21 => "Built-in Pointing Device",
            22 => "Portable Battery",
            23 => "System Reset",
            24 => "Hardware Security",
            25 => "System Power Controls",
            26 => "Voltage Probe",
            27 => "Cooling Device",
            28 => "Temperature Probe",
            29 => "Electrical Current Probe",
            30 => "Out-of-Band Remote Access",
            31 => "Boot Integrity Services",
            32 => "System Boot Information",
            33 => "64-bit Memory Error Information",
            34 => "Management Device",
            35 => "Management Device Component",
            36 => "Management Device Threshold Data",
            37 => "Memory Channel",
            38 => "IPMI Device Information",
            39 => "System Power Supply",
            40 => "Additional Information",
            41 => "Onboard Devices Extended Information",
            42 => "Management Controller Host Interface",
            43 => "TPM Device",
            44 => "Processor Additional Information",
            45 => "Firmware Inventory Information",
            46 => "String Property",
            127 => "End-of-Table",
            128..=255 => "OEM-specific",
            _ => "Unknown",
        }
    }

    /// Get string by index (1-based)
    pub fn get_string(&self, index: u8) -> Option<&str> {
        if index == 0 {
            None
        } else {
            self.strings.get(index as usize - 1).map(|s| s.as_str())
        }
    }

    /// Get raw data
    pub unsafe fn data(&self) -> &[u8] {
        core::slice::from_raw_parts(
            (self.address.0 + core::mem::size_of::<SmbiosHeader>() as u64) as *const u8,
            self.length as usize - core::mem::size_of::<SmbiosHeader>(),
        )
    }
}

// =============================================================================
// TYPE 0: BIOS INFORMATION
// =============================================================================

/// BIOS Information (Type 0)
#[derive(Debug, Clone)]
pub struct BiosInfo {
    /// Vendor
    pub vendor: Option<String>,
    /// Version
    pub version: Option<String>,
    /// Release date
    pub release_date: Option<String>,
    /// Starting address segment
    pub address_segment: u16,
    /// ROM size (64KB units - 1, or extended)
    pub rom_size: u8,
    /// Characteristics
    pub characteristics: u64,
    /// Extended ROM size (MB)
    pub extended_rom_size: Option<u16>,
    /// System BIOS major release
    pub major_release: Option<u8>,
    /// System BIOS minor release
    pub minor_release: Option<u8>,
    /// EC firmware major release
    pub ec_major_release: Option<u8>,
    /// EC firmware minor release
    pub ec_minor_release: Option<u8>,
}

impl BiosInfo {
    /// Parse from structure
    pub unsafe fn from_structure(s: &SmbiosStructure) -> Self {
        let data = (s.address.0 + 4) as *const u8;

        let vendor_idx = *data;
        let version_idx = *data.add(1);
        let address_segment = ptr::read_unaligned(data.add(2) as *const u16);
        let release_date_idx = *data.add(4);
        let rom_size = *data.add(5);
        let characteristics = ptr::read_unaligned(data.add(6) as *const u64);

        let mut info = Self {
            vendor: s.get_string(vendor_idx).map(String::from),
            version: s.get_string(version_idx).map(String::from),
            release_date: s.get_string(release_date_idx).map(String::from),
            address_segment,
            rom_size,
            characteristics,
            extended_rom_size: None,
            major_release: None,
            minor_release: None,
            ec_major_release: None,
            ec_minor_release: None,
        };

        // Extended fields (SMBIOS 2.4+)
        if s.length >= 0x18 {
            info.extended_rom_size = Some(ptr::read_unaligned(data.add(16) as *const u16));
            info.major_release = Some(*data.add(18));
            info.minor_release = Some(*data.add(19));
            info.ec_major_release = Some(*data.add(20));
            info.ec_minor_release = Some(*data.add(21));
        }

        info
    }

    /// Get ROM size in bytes
    pub fn rom_size_bytes(&self) -> u64 {
        if let Some(ext) = self.extended_rom_size {
            if (ext & 0xC000) == 0 {
                (ext as u64) * 1024 * 1024 // MB
            } else {
                (ext as u64 & 0x3FFF) * 1024 * 1024 * 1024 // GB
            }
        } else if self.rom_size == 0xFF {
            0 // Use extended size
        } else {
            ((self.rom_size as u64) + 1) * 64 * 1024
        }
    }
}

// =============================================================================
// TYPE 1: SYSTEM INFORMATION
// =============================================================================

/// System Information (Type 1)
#[derive(Debug, Clone)]
pub struct SystemInfo {
    /// Manufacturer
    pub manufacturer: Option<String>,
    /// Product name
    pub product_name: Option<String>,
    /// Version
    pub version: Option<String>,
    /// Serial number
    pub serial_number: Option<String>,
    /// UUID
    pub uuid: Option<[u8; 16]>,
    /// Wakeup type
    pub wakeup_type: WakeupType,
    /// SKU number
    pub sku_number: Option<String>,
    /// Family
    pub family: Option<String>,
}

impl SystemInfo {
    /// Parse from structure
    pub unsafe fn from_structure(s: &SmbiosStructure) -> Self {
        let data = (s.address.0 + 4) as *const u8;

        let manufacturer_idx = *data;
        let product_idx = *data.add(1);
        let version_idx = *data.add(2);
        let serial_idx = *data.add(3);

        let mut uuid = None;
        let mut wakeup_type = WakeupType::Unknown;
        let mut sku_idx = 0;
        let mut family_idx = 0;

        if s.length >= 0x19 {
            let mut uuid_bytes = [0u8; 16];
            for i in 0..16 {
                uuid_bytes[i] = *data.add(4 + i);
            }
            uuid = Some(uuid_bytes);
            wakeup_type = WakeupType::from_byte(*data.add(20));
        }

        if s.length >= 0x1B {
            sku_idx = *data.add(21);
            family_idx = *data.add(22);
        }

        Self {
            manufacturer: s.get_string(manufacturer_idx).map(String::from),
            product_name: s.get_string(product_idx).map(String::from),
            version: s.get_string(version_idx).map(String::from),
            serial_number: s.get_string(serial_idx).map(String::from),
            uuid,
            wakeup_type,
            sku_number: s.get_string(sku_idx).map(String::from),
            family: s.get_string(family_idx).map(String::from),
        }
    }

    /// Get UUID as string
    pub fn uuid_string(&self) -> Option<String> {
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
}

/// Wakeup type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeupType {
    /// Reserved
    Reserved,
    /// Other
    Other,
    /// Unknown
    Unknown,
    /// APM Timer
    ApmTimer,
    /// Modem Ring
    ModemRing,
    /// LAN Remote
    LanRemote,
    /// Power Switch
    PowerSwitch,
    /// PCI PME
    PciPme,
    /// AC Power Restored
    AcPowerRestored,
}

impl WakeupType {
    /// Create from byte
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => Self::Reserved,
            1 => Self::Other,
            2 => Self::Unknown,
            3 => Self::ApmTimer,
            4 => Self::ModemRing,
            5 => Self::LanRemote,
            6 => Self::PowerSwitch,
            7 => Self::PciPme,
            8 => Self::AcPowerRestored,
            _ => Self::Unknown,
        }
    }
}

// =============================================================================
// TYPE 2: BASEBOARD INFORMATION
// =============================================================================

/// Baseboard Information (Type 2)
#[derive(Debug, Clone)]
pub struct BaseboardInfo {
    /// Manufacturer
    pub manufacturer: Option<String>,
    /// Product
    pub product: Option<String>,
    /// Version
    pub version: Option<String>,
    /// Serial number
    pub serial_number: Option<String>,
    /// Asset tag
    pub asset_tag: Option<String>,
    /// Features
    pub feature_flags: u8,
    /// Location in chassis
    pub location: Option<String>,
    /// Chassis handle
    pub chassis_handle: u16,
    /// Board type
    pub board_type: BoardType,
}

impl BaseboardInfo {
    /// Parse from structure
    pub unsafe fn from_structure(s: &SmbiosStructure) -> Self {
        let data = (s.address.0 + 4) as *const u8;

        let manufacturer_idx = *data;
        let product_idx = *data.add(1);
        let version_idx = *data.add(2);
        let serial_idx = *data.add(3);
        let asset_idx = *data.add(4);
        let feature_flags = *data.add(5);
        let location_idx = *data.add(6);
        let chassis_handle = ptr::read_unaligned(data.add(7) as *const u16);
        let board_type = BoardType::from_byte(*data.add(9));

        Self {
            manufacturer: s.get_string(manufacturer_idx).map(String::from),
            product: s.get_string(product_idx).map(String::from),
            version: s.get_string(version_idx).map(String::from),
            serial_number: s.get_string(serial_idx).map(String::from),
            asset_tag: s.get_string(asset_idx).map(String::from),
            feature_flags,
            location: s.get_string(location_idx).map(String::from),
            chassis_handle,
            board_type,
        }
    }

    /// Check if hosting board
    pub fn is_hosting_board(&self) -> bool {
        (self.feature_flags & 0x01) != 0
    }

    /// Check if requires daughterboard
    pub fn requires_daughterboard(&self) -> bool {
        (self.feature_flags & 0x02) != 0
    }

    /// Check if removable
    pub fn is_removable(&self) -> bool {
        (self.feature_flags & 0x04) != 0
    }

    /// Check if replaceable
    pub fn is_replaceable(&self) -> bool {
        (self.feature_flags & 0x08) != 0
    }

    /// Check if hot-swappable
    pub fn is_hot_swappable(&self) -> bool {
        (self.feature_flags & 0x10) != 0
    }
}

/// Board type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardType {
    /// Unknown
    Unknown,
    /// Other
    Other,
    /// Server Blade
    ServerBlade,
    /// Connectivity Switch
    ConnectivitySwitch,
    /// System Management Module
    SystemManagementModule,
    /// Processor Module
    ProcessorModule,
    /// I/O Module
    IoModule,
    /// Memory Module
    MemoryModule,
    /// Daughterboard
    Daughterboard,
    /// Motherboard
    Motherboard,
    /// Processor/Memory Module
    ProcessorMemoryModule,
    /// Processor/IO Module
    ProcessorIoModule,
    /// Interconnect Board
    InterconnectBoard,
}

impl BoardType {
    /// Create from byte
    pub fn from_byte(value: u8) -> Self {
        match value {
            1 => Self::Unknown,
            2 => Self::Other,
            3 => Self::ServerBlade,
            4 => Self::ConnectivitySwitch,
            5 => Self::SystemManagementModule,
            6 => Self::ProcessorModule,
            7 => Self::IoModule,
            8 => Self::MemoryModule,
            9 => Self::Daughterboard,
            10 => Self::Motherboard,
            11 => Self::ProcessorMemoryModule,
            12 => Self::ProcessorIoModule,
            13 => Self::InterconnectBoard,
            _ => Self::Unknown,
        }
    }
}

// =============================================================================
// TYPE 3: CHASSIS INFORMATION
// =============================================================================

/// Chassis Information (Type 3)
#[derive(Debug, Clone)]
pub struct ChassisInfo {
    /// Manufacturer
    pub manufacturer: Option<String>,
    /// Type
    pub chassis_type: ChassisType,
    /// Version
    pub version: Option<String>,
    /// Serial number
    pub serial_number: Option<String>,
    /// Asset tag
    pub asset_tag: Option<String>,
    /// Boot-up state
    pub boot_state: ChassisState,
    /// Power supply state
    pub power_state: ChassisState,
    /// Thermal state
    pub thermal_state: ChassisState,
    /// Security status
    pub security_status: ChassisSecurityStatus,
}

impl ChassisInfo {
    /// Parse from structure
    pub unsafe fn from_structure(s: &SmbiosStructure) -> Self {
        let data = (s.address.0 + 4) as *const u8;

        let manufacturer_idx = *data;
        let chassis_type = ChassisType::from_byte(*data.add(1));
        let version_idx = *data.add(2);
        let serial_idx = *data.add(3);
        let asset_idx = *data.add(4);
        let boot_state = ChassisState::from_byte(*data.add(5));
        let power_state = ChassisState::from_byte(*data.add(6));
        let thermal_state = ChassisState::from_byte(*data.add(7));
        let security_status = ChassisSecurityStatus::from_byte(*data.add(8));

        Self {
            manufacturer: s.get_string(manufacturer_idx).map(String::from),
            chassis_type,
            version: s.get_string(version_idx).map(String::from),
            serial_number: s.get_string(serial_idx).map(String::from),
            asset_tag: s.get_string(asset_idx).map(String::from),
            boot_state,
            power_state,
            thermal_state,
            security_status,
        }
    }
}

/// Chassis type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChassisType {
    /// Other
    Other,
    /// Unknown
    Unknown,
    /// Desktop
    Desktop,
    /// Low Profile Desktop
    LowProfileDesktop,
    /// Pizza Box
    PizzaBox,
    /// Mini Tower
    MiniTower,
    /// Tower
    Tower,
    /// Portable
    Portable,
    /// Laptop
    Laptop,
    /// Notebook
    Notebook,
    /// Hand Held
    HandHeld,
    /// Docking Station
    DockingStation,
    /// All in One
    AllInOne,
    /// Sub Notebook
    SubNotebook,
    /// Space-saving
    SpaceSaving,
    /// Lunch Box
    LunchBox,
    /// Main Server Chassis
    MainServerChassis,
    /// Expansion Chassis
    ExpansionChassis,
    /// Sub Chassis
    SubChassis,
    /// Bus Expansion Chassis
    BusExpansionChassis,
    /// Peripheral Chassis
    PeripheralChassis,
    /// RAID Chassis
    RaidChassis,
    /// Rack Mount Chassis
    RackMountChassis,
    /// Sealed-case PC
    SealedCasePc,
    /// Multi-system chassis
    MultiSystemChassis,
    /// Compact PCI
    CompactPci,
    /// Advanced TCA
    AdvancedTca,
    /// Blade
    Blade,
    /// Blade Enclosure
    BladeEnclosure,
    /// Tablet
    Tablet,
    /// Convertible
    Convertible,
    /// Detachable
    Detachable,
    /// IoT Gateway
    IotGateway,
    /// Embedded PC
    EmbeddedPc,
    /// Mini PC
    MiniPc,
    /// Stick PC
    StickPc,
}

impl ChassisType {
    /// Create from byte
    pub fn from_byte(value: u8) -> Self {
        match value & 0x7F {
            1 => Self::Other,
            2 => Self::Unknown,
            3 => Self::Desktop,
            4 => Self::LowProfileDesktop,
            5 => Self::PizzaBox,
            6 => Self::MiniTower,
            7 => Self::Tower,
            8 => Self::Portable,
            9 => Self::Laptop,
            10 => Self::Notebook,
            11 => Self::HandHeld,
            12 => Self::DockingStation,
            13 => Self::AllInOne,
            14 => Self::SubNotebook,
            15 => Self::SpaceSaving,
            16 => Self::LunchBox,
            17 => Self::MainServerChassis,
            18 => Self::ExpansionChassis,
            19 => Self::SubChassis,
            20 => Self::BusExpansionChassis,
            21 => Self::PeripheralChassis,
            22 => Self::RaidChassis,
            23 => Self::RackMountChassis,
            24 => Self::SealedCasePc,
            25 => Self::MultiSystemChassis,
            26 => Self::CompactPci,
            27 => Self::AdvancedTca,
            28 => Self::Blade,
            29 => Self::BladeEnclosure,
            30 => Self::Tablet,
            31 => Self::Convertible,
            32 => Self::Detachable,
            33 => Self::IotGateway,
            34 => Self::EmbeddedPc,
            35 => Self::MiniPc,
            36 => Self::StickPc,
            _ => Self::Unknown,
        }
    }
}

/// Chassis state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChassisState {
    /// Other
    Other,
    /// Unknown
    Unknown,
    /// Safe
    Safe,
    /// Warning
    Warning,
    /// Critical
    Critical,
    /// Non-recoverable
    NonRecoverable,
}

impl ChassisState {
    /// Create from byte
    pub fn from_byte(value: u8) -> Self {
        match value {
            1 => Self::Other,
            2 => Self::Unknown,
            3 => Self::Safe,
            4 => Self::Warning,
            5 => Self::Critical,
            6 => Self::NonRecoverable,
            _ => Self::Unknown,
        }
    }
}

/// Chassis security status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChassisSecurityStatus {
    /// Other
    Other,
    /// Unknown
    Unknown,
    /// None
    None,
    /// External Interface Locked Out
    ExternalInterfaceLockedOut,
    /// External Interface Enabled
    ExternalInterfaceEnabled,
}

impl ChassisSecurityStatus {
    /// Create from byte
    pub fn from_byte(value: u8) -> Self {
        match value {
            1 => Self::Other,
            2 => Self::Unknown,
            3 => Self::None,
            4 => Self::ExternalInterfaceLockedOut,
            5 => Self::ExternalInterfaceEnabled,
            _ => Self::Unknown,
        }
    }
}

// =============================================================================
// TYPE 4: PROCESSOR INFORMATION
// =============================================================================

/// Processor Information (Type 4)
#[derive(Debug, Clone)]
pub struct ProcessorInfo {
    /// Socket designation
    pub socket: Option<String>,
    /// Processor type
    pub processor_type: ProcessorType,
    /// Processor family
    pub processor_family: u16,
    /// Manufacturer
    pub manufacturer: Option<String>,
    /// Processor ID
    pub processor_id: u64,
    /// Version
    pub version: Option<String>,
    /// Voltage
    pub voltage: u8,
    /// External clock (MHz)
    pub external_clock: u16,
    /// Max speed (MHz)
    pub max_speed: u16,
    /// Current speed (MHz)
    pub current_speed: u16,
    /// Status
    pub status: u8,
    /// Upgrade
    pub upgrade: u8,
    /// Core count
    pub core_count: Option<u16>,
    /// Core enabled
    pub core_enabled: Option<u16>,
    /// Thread count
    pub thread_count: Option<u16>,
}

impl ProcessorInfo {
    /// Parse from structure
    pub unsafe fn from_structure(s: &SmbiosStructure) -> Self {
        let data = (s.address.0 + 4) as *const u8;

        let socket_idx = *data;
        let processor_type = ProcessorType::from_byte(*data.add(1));
        let family = *data.add(2);
        let manufacturer_idx = *data.add(3);
        let processor_id = ptr::read_unaligned(data.add(4) as *const u64);
        let version_idx = *data.add(12);
        let voltage = *data.add(13);
        let external_clock = ptr::read_unaligned(data.add(14) as *const u16);
        let max_speed = ptr::read_unaligned(data.add(16) as *const u16);
        let current_speed = ptr::read_unaligned(data.add(18) as *const u16);
        let status = *data.add(20);
        let upgrade = *data.add(21);

        let processor_family = if family == 0xFE && s.length >= 0x2A {
            ptr::read_unaligned(data.add(38) as *const u16)
        } else {
            family as u16
        };

        let mut core_count = None;
        let mut core_enabled = None;
        let mut thread_count = None;

        if s.length >= 0x28 {
            let cc = *data.add(31);
            let ce = *data.add(32);
            let tc = *data.add(33);

            if s.length >= 0x30 && (cc == 0xFF || ce == 0xFF || tc == 0xFF) {
                core_count = Some(ptr::read_unaligned(data.add(38) as *const u16));
                core_enabled = Some(ptr::read_unaligned(data.add(40) as *const u16));
                thread_count = Some(ptr::read_unaligned(data.add(42) as *const u16));
            } else {
                core_count = Some(cc as u16);
                core_enabled = Some(ce as u16);
                thread_count = Some(tc as u16);
            }
        }

        Self {
            socket: s.get_string(socket_idx).map(String::from),
            processor_type,
            processor_family,
            manufacturer: s.get_string(manufacturer_idx).map(String::from),
            processor_id,
            version: s.get_string(version_idx).map(String::from),
            voltage,
            external_clock,
            max_speed,
            current_speed,
            status,
            upgrade,
            core_count,
            core_enabled,
            thread_count,
        }
    }

    /// Check if CPU is populated
    pub fn is_populated(&self) -> bool {
        (self.status & 0x40) != 0
    }

    /// Get CPU status
    pub fn cpu_status(&self) -> CpuStatus {
        match self.status & 0x07 {
            0 => CpuStatus::Unknown,
            1 => CpuStatus::Enabled,
            2 => CpuStatus::DisabledByUser,
            3 => CpuStatus::DisabledByBios,
            4 => CpuStatus::Idle,
            7 => CpuStatus::Other,
            _ => CpuStatus::Unknown,
        }
    }
}

/// Processor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessorType {
    /// Other
    Other,
    /// Unknown
    Unknown,
    /// Central Processor
    CentralProcessor,
    /// Math Processor
    MathProcessor,
    /// DSP Processor
    DspProcessor,
    /// Video Processor
    VideoProcessor,
}

impl ProcessorType {
    /// Create from byte
    pub fn from_byte(value: u8) -> Self {
        match value {
            1 => Self::Other,
            2 => Self::Unknown,
            3 => Self::CentralProcessor,
            4 => Self::MathProcessor,
            5 => Self::DspProcessor,
            6 => Self::VideoProcessor,
            _ => Self::Unknown,
        }
    }
}

/// CPU status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuStatus {
    /// Unknown
    Unknown,
    /// Enabled
    Enabled,
    /// Disabled by user
    DisabledByUser,
    /// Disabled by BIOS
    DisabledByBios,
    /// Idle
    Idle,
    /// Other
    Other,
}

// =============================================================================
// TYPE 17: MEMORY DEVICE
// =============================================================================

/// Memory Device Information (Type 17)
#[derive(Debug, Clone)]
pub struct MemoryDeviceInfo {
    /// Physical memory array handle
    pub array_handle: u16,
    /// Memory error information handle
    pub error_handle: u16,
    /// Total width (bits)
    pub total_width: Option<u16>,
    /// Data width (bits)
    pub data_width: Option<u16>,
    /// Size (bytes)
    pub size: Option<u64>,
    /// Form factor
    pub form_factor: MemoryFormFactor,
    /// Device set
    pub device_set: u8,
    /// Device locator
    pub device_locator: Option<String>,
    /// Bank locator
    pub bank_locator: Option<String>,
    /// Memory type
    pub memory_type: MemoryType,
    /// Type detail
    pub type_detail: u16,
    /// Speed (MT/s)
    pub speed: Option<u16>,
    /// Manufacturer
    pub manufacturer: Option<String>,
    /// Serial number
    pub serial_number: Option<String>,
    /// Asset tag
    pub asset_tag: Option<String>,
    /// Part number
    pub part_number: Option<String>,
    /// Configured speed (MT/s)
    pub configured_speed: Option<u16>,
}

impl MemoryDeviceInfo {
    /// Parse from structure
    pub unsafe fn from_structure(s: &SmbiosStructure) -> Self {
        let data = (s.address.0 + 4) as *const u8;

        let array_handle = ptr::read_unaligned(data as *const u16);
        let error_handle = ptr::read_unaligned(data.add(2) as *const u16);

        let total_width = ptr::read_unaligned(data.add(4) as *const u16);
        let data_width = ptr::read_unaligned(data.add(6) as *const u16);
        let size_raw = ptr::read_unaligned(data.add(8) as *const u16);

        let form_factor = MemoryFormFactor::from_byte(*data.add(10));
        let device_set = *data.add(11);
        let device_locator_idx = *data.add(12);
        let bank_locator_idx = *data.add(13);
        let memory_type = MemoryType::from_byte(*data.add(14));
        let type_detail = ptr::read_unaligned(data.add(15) as *const u16);

        let mut speed = None;
        let mut manufacturer_idx = 0;
        let mut serial_idx = 0;
        let mut asset_idx = 0;
        let mut part_idx = 0;
        let mut configured_speed = None;

        if s.length >= 0x1C {
            speed = Some(ptr::read_unaligned(data.add(17) as *const u16));
            manufacturer_idx = *data.add(19);
            serial_idx = *data.add(20);
            asset_idx = *data.add(21);
            part_idx = *data.add(22);
        }

        if s.length >= 0x22 {
            configured_speed = Some(ptr::read_unaligned(data.add(29) as *const u16));
        }

        // Calculate size
        let size = if size_raw == 0 || size_raw == 0xFFFF {
            None
        } else if size_raw == 0x7FFF {
            // Extended size field (SMBIOS 2.7+)
            if s.length >= 0x20 {
                let ext_size = ptr::read_unaligned(data.add(24) as *const u32);
                Some((ext_size as u64) * 1024 * 1024) // MB
            } else {
                None
            }
        } else {
            let granularity = if (size_raw & 0x8000) != 0 {
                1024
            } else {
                1024 * 1024
            };
            Some(((size_raw & 0x7FFF) as u64) * granularity)
        };

        Self {
            array_handle,
            error_handle,
            total_width: if total_width == 0xFFFF {
                None
            } else {
                Some(total_width)
            },
            data_width: if data_width == 0xFFFF {
                None
            } else {
                Some(data_width)
            },
            size,
            form_factor,
            device_set,
            device_locator: s.get_string(device_locator_idx).map(String::from),
            bank_locator: s.get_string(bank_locator_idx).map(String::from),
            memory_type,
            type_detail,
            speed: speed.filter(|&s| s != 0),
            manufacturer: s.get_string(manufacturer_idx).map(String::from),
            serial_number: s.get_string(serial_idx).map(String::from),
            asset_tag: s.get_string(asset_idx).map(String::from),
            part_number: s.get_string(part_idx).map(String::from),
            configured_speed: configured_speed.filter(|&s| s != 0),
        }
    }

    /// Get size in human-readable format
    pub fn size_string(&self) -> String {
        match self.size {
            None => String::from("No Module Installed"),
            Some(bytes) => {
                if bytes >= 1024 * 1024 * 1024 {
                    alloc::format!("{} GB", bytes / (1024 * 1024 * 1024))
                } else if bytes >= 1024 * 1024 {
                    alloc::format!("{} MB", bytes / (1024 * 1024))
                } else if bytes >= 1024 {
                    alloc::format!("{} KB", bytes / 1024)
                } else {
                    alloc::format!("{} B", bytes)
                }
            },
        }
    }
}

/// Memory form factor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryFormFactor {
    /// Other
    Other,
    /// Unknown
    Unknown,
    /// SIMM
    Simm,
    /// SIP
    Sip,
    /// Chip
    Chip,
    /// DIP
    Dip,
    /// ZIP
    Zip,
    /// Proprietary Card
    ProprietaryCard,
    /// DIMM
    Dimm,
    /// TSOP
    Tsop,
    /// Row of chips
    RowOfChips,
    /// RIMM
    Rimm,
    /// SODIMM
    Sodimm,
    /// SRIMM
    Srimm,
    /// FB-DIMM
    FbDimm,
    /// Die
    Die,
}

impl MemoryFormFactor {
    /// Create from byte
    pub fn from_byte(value: u8) -> Self {
        match value {
            1 => Self::Other,
            2 => Self::Unknown,
            3 => Self::Simm,
            4 => Self::Sip,
            5 => Self::Chip,
            6 => Self::Dip,
            7 => Self::Zip,
            8 => Self::ProprietaryCard,
            9 => Self::Dimm,
            10 => Self::Tsop,
            11 => Self::RowOfChips,
            12 => Self::Rimm,
            13 => Self::Sodimm,
            14 => Self::Srimm,
            15 => Self::FbDimm,
            16 => Self::Die,
            _ => Self::Unknown,
        }
    }
}

/// Memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    /// Other
    Other,
    /// Unknown
    Unknown,
    /// DRAM
    Dram,
    /// EDRAM
    Edram,
    /// VRAM
    Vram,
    /// SRAM
    Sram,
    /// RAM
    Ram,
    /// ROM
    Rom,
    /// FLASH
    Flash,
    /// EEPROM
    Eeprom,
    /// FEPROM
    Feprom,
    /// EPROM
    Eprom,
    /// CDRAM
    Cdram,
    /// 3DRAM
    ThreeDram,
    /// SDRAM
    Sdram,
    /// SGRAM
    Sgram,
    /// RDRAM
    Rdram,
    /// DDR
    Ddr,
    /// DDR2
    Ddr2,
    /// DDR2 FB-DIMM
    Ddr2FbDimm,
    /// DDR3
    Ddr3,
    /// FBD2
    Fbd2,
    /// DDR4
    Ddr4,
    /// LPDDR
    Lpddr,
    /// LPDDR2
    Lpddr2,
    /// LPDDR3
    Lpddr3,
    /// LPDDR4
    Lpddr4,
    /// Logical non-volatile device
    LogicalNonVolatile,
    /// HBM
    Hbm,
    /// HBM2
    Hbm2,
    /// DDR5
    Ddr5,
    /// LPDDR5
    Lpddr5,
    /// HBM3
    Hbm3,
}

impl MemoryType {
    /// Create from byte
    pub fn from_byte(value: u8) -> Self {
        match value {
            1 => Self::Other,
            2 => Self::Unknown,
            3 => Self::Dram,
            4 => Self::Edram,
            5 => Self::Vram,
            6 => Self::Sram,
            7 => Self::Ram,
            8 => Self::Rom,
            9 => Self::Flash,
            10 => Self::Eeprom,
            11 => Self::Feprom,
            12 => Self::Eprom,
            13 => Self::Cdram,
            14 => Self::ThreeDram,
            15 => Self::Sdram,
            16 => Self::Sgram,
            17 => Self::Rdram,
            18 => Self::Ddr,
            19 => Self::Ddr2,
            20 => Self::Ddr2FbDimm,
            24 => Self::Ddr3,
            25 => Self::Fbd2,
            26 => Self::Ddr4,
            27 => Self::Lpddr,
            28 => Self::Lpddr2,
            29 => Self::Lpddr3,
            30 => Self::Lpddr4,
            31 => Self::LogicalNonVolatile,
            32 => Self::Hbm,
            33 => Self::Hbm2,
            34 => Self::Ddr5,
            35 => Self::Lpddr5,
            36 => Self::Hbm3,
            _ => Self::Unknown,
        }
    }

    /// Get name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Other => "Other",
            Self::Unknown => "Unknown",
            Self::Dram => "DRAM",
            Self::Edram => "EDRAM",
            Self::Vram => "VRAM",
            Self::Sram => "SRAM",
            Self::Ram => "RAM",
            Self::Rom => "ROM",
            Self::Flash => "Flash",
            Self::Eeprom => "EEPROM",
            Self::Feprom => "FEPROM",
            Self::Eprom => "EPROM",
            Self::Cdram => "CDRAM",
            Self::ThreeDram => "3DRAM",
            Self::Sdram => "SDRAM",
            Self::Sgram => "SGRAM",
            Self::Rdram => "RDRAM",
            Self::Ddr => "DDR",
            Self::Ddr2 => "DDR2",
            Self::Ddr2FbDimm => "DDR2 FB-DIMM",
            Self::Ddr3 => "DDR3",
            Self::Fbd2 => "FBD2",
            Self::Ddr4 => "DDR4",
            Self::Lpddr => "LPDDR",
            Self::Lpddr2 => "LPDDR2",
            Self::Lpddr3 => "LPDDR3",
            Self::Lpddr4 => "LPDDR4",
            Self::LogicalNonVolatile => "Logical Non-Volatile",
            Self::Hbm => "HBM",
            Self::Hbm2 => "HBM2",
            Self::Ddr5 => "DDR5",
            Self::Lpddr5 => "LPDDR5",
            Self::Hbm3 => "HBM3",
        }
    }
}

// =============================================================================
// SMBIOS GUIDS
// =============================================================================

/// SMBIOS GUIDs
pub mod smbios_guids {
    use super::*;

    /// SMBIOS table GUID
    pub const SMBIOS: Guid = Guid::new(0xEB9D2D31, 0x2D88, 0x11D3, [
        0x9A, 0x16, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    /// SMBIOS 3.0 table GUID
    pub const SMBIOS3: Guid = Guid::new(0xF2FD1544, 0x9794, 0x4A2C, [
        0x99, 0x2E, 0xE5, 0xBB, 0xCF, 0x20, 0xE3, 0x94,
    ]);
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_smbios_version() {
        let v = SmbiosVersion::new(3, 4);
        assert_eq!(v.major, 3);
        assert_eq!(v.minor, 4);
        assert!(v > SmbiosVersion::V3_0);
    }

    #[test]
    fn test_chassis_type() {
        assert_eq!(ChassisType::from_byte(3), ChassisType::Desktop);
        assert_eq!(ChassisType::from_byte(9), ChassisType::Laptop);
        assert_eq!(ChassisType::from_byte(23), ChassisType::RackMountChassis);
    }

    #[test]
    fn test_memory_type_name() {
        assert_eq!(MemoryType::Ddr4.name(), "DDR4");
        assert_eq!(MemoryType::Ddr5.name(), "DDR5");
        assert_eq!(MemoryType::Lpddr4.name(), "LPDDR4");
    }

    #[test]
    fn test_processor_type() {
        assert_eq!(ProcessorType::from_byte(3), ProcessorType::CentralProcessor);
        assert_eq!(ProcessorType::from_byte(6), ProcessorType::VideoProcessor);
    }
}
