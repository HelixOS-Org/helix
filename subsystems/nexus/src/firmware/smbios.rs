//! SMBIOS Parser
//!
//! SMBIOS table parsing.

use alloc::string::String;
use alloc::vec::Vec;

/// SMBIOS structure types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmbiosType {
    /// BIOS Information
    BiosInfo = 0,
    /// System Information
    SystemInfo = 1,
    /// Baseboard Information
    BaseboardInfo = 2,
    /// System Enclosure
    SystemEnclosure = 3,
    /// Processor Information
    ProcessorInfo = 4,
    /// Memory Controller Information (obsolete)
    MemoryController = 5,
    /// Memory Module Information (obsolete)
    MemoryModule = 6,
    /// Cache Information
    CacheInfo = 7,
    /// Port Connector Information
    PortConnector = 8,
    /// System Slots
    SystemSlots = 9,
    /// Physical Memory Array
    PhysicalMemoryArray = 16,
    /// Memory Device
    MemoryDevice = 17,
    /// Memory Array Mapped Address
    MemoryArrayMappedAddress = 19,
    /// Memory Device Mapped Address
    MemoryDeviceMappedAddress = 20,
    /// System Boot Information
    SystemBoot = 32,
    /// End of Table
    EndOfTable = 127,
}

/// SMBIOS structure
#[derive(Debug, Clone)]
pub struct SmbiosStructure {
    /// Structure type
    pub stype: u8,
    /// Length
    pub length: u8,
    /// Handle
    pub handle: u16,
    /// Raw data
    pub data: Vec<u8>,
    /// Strings
    pub strings: Vec<String>,
}

impl SmbiosStructure {
    /// Create new structure
    pub fn new(stype: u8, handle: u16) -> Self {
        Self {
            stype,
            length: 0,
            handle,
            data: Vec::new(),
            strings: Vec::new(),
        }
    }

    /// Get string by index (1-based)
    #[inline]
    pub fn get_string(&self, index: u8) -> Option<&str> {
        if index == 0 {
            return None;
        }
        self.strings.get((index - 1) as usize).map(|s| s.as_str())
    }

    /// Get byte at offset
    #[inline(always)]
    pub fn get_byte(&self, offset: usize) -> Option<u8> {
        self.data.get(offset).copied()
    }

    /// Get word at offset
    #[inline]
    pub fn get_word(&self, offset: usize) -> Option<u16> {
        if offset + 1 >= self.data.len() {
            return None;
        }
        Some(u16::from_le_bytes([self.data[offset], self.data[offset + 1]]))
    }

    /// Get dword at offset
    #[inline]
    pub fn get_dword(&self, offset: usize) -> Option<u32> {
        if offset + 3 >= self.data.len() {
            return None;
        }
        Some(u32::from_le_bytes([
            self.data[offset],
            self.data[offset + 1],
            self.data[offset + 2],
            self.data[offset + 3],
        ]))
    }
}

/// SMBIOS entry point info
#[derive(Debug, Clone)]
pub struct SmbiosInfo {
    /// SMBIOS version major
    pub version_major: u8,
    /// SMBIOS version minor
    pub version_minor: u8,
    /// Maximum structure size
    pub max_structure_size: u16,
    /// Number of structures
    pub structure_count: u16,
    /// BIOS vendor
    pub bios_vendor: String,
    /// BIOS version
    pub bios_version: String,
    /// System manufacturer
    pub system_manufacturer: String,
    /// System product name
    pub system_product: String,
    /// System serial number
    pub system_serial: String,
    /// System UUID
    pub system_uuid: [u8; 16],
    /// Total memory (bytes)
    pub total_memory: u64,
    /// CPU count
    pub cpu_count: u32,
}

impl Default for SmbiosInfo {
    fn default() -> Self {
        Self {
            version_major: 0,
            version_minor: 0,
            max_structure_size: 0,
            structure_count: 0,
            bios_vendor: String::new(),
            bios_version: String::new(),
            system_manufacturer: String::new(),
            system_product: String::new(),
            system_serial: String::new(),
            system_uuid: [0; 16],
            total_memory: 0,
            cpu_count: 0,
        }
    }
}

/// SMBIOS parser
pub struct SmbiosParser {
    /// Entry point address
    entry_point: Option<u64>,
    /// Structures
    structures: Vec<SmbiosStructure>,
    /// Parsed info
    info: SmbiosInfo,
    /// Parse complete
    parsed: bool,
}

impl SmbiosParser {
    /// Create new parser
    pub fn new() -> Self {
        Self {
            entry_point: None,
            structures: Vec::new(),
            info: SmbiosInfo::default(),
            parsed: false,
        }
    }

    /// Set entry point
    #[inline(always)]
    pub fn set_entry_point(&mut self, address: u64) {
        self.entry_point = Some(address);
    }

    /// Add structure
    #[inline(always)]
    pub fn add_structure(&mut self, structure: SmbiosStructure) {
        self.structures.push(structure);
    }

    /// Parse and extract info
    pub fn parse(&mut self) -> &SmbiosInfo {
        if self.parsed {
            return &self.info;
        }

        self.info.structure_count = self.structures.len() as u16;

        // Extract BIOS info
        if let Some(bios) = self.get_structure(SmbiosType::BiosInfo as u8) {
            if let Some(vendor) = bios.get_string(bios.get_byte(0x04).unwrap_or(0)) {
                self.info.bios_vendor = String::from(vendor);
            }
            if let Some(version) = bios.get_string(bios.get_byte(0x05).unwrap_or(0)) {
                self.info.bios_version = String::from(version);
            }
        }

        // Extract system info
        if let Some(sys) = self.get_structure(SmbiosType::SystemInfo as u8) {
            if let Some(mfr) = sys.get_string(sys.get_byte(0x04).unwrap_or(0)) {
                self.info.system_manufacturer = String::from(mfr);
            }
            if let Some(product) = sys.get_string(sys.get_byte(0x05).unwrap_or(0)) {
                self.info.system_product = String::from(product);
            }
            if let Some(serial) = sys.get_string(sys.get_byte(0x07).unwrap_or(0)) {
                self.info.system_serial = String::from(serial);
            }
            // UUID at offset 0x08, 16 bytes
            if sys.data.len() >= 0x18 {
                self.info.system_uuid.copy_from_slice(&sys.data[0x08..0x18]);
            }
        }

        // Count CPUs
        self.info.cpu_count = self.structures.iter()
            .filter(|s| s.stype == SmbiosType::ProcessorInfo as u8)
            .count() as u32;

        // Calculate total memory
        self.info.total_memory = self.structures.iter()
            .filter(|s| s.stype == SmbiosType::MemoryDevice as u8)
            .filter_map(|s| {
                let size = s.get_word(0x0C)?;
                if size == 0 || size == 0xFFFF {
                    return None;
                }
                let bytes = if size & 0x8000 != 0 {
                    ((size & 0x7FFF) as u64) * 1024
                } else {
                    (size as u64) * 1024 * 1024
                };
                Some(bytes)
            })
            .sum();

        self.parsed = true;
        &self.info
    }

    /// Get structure by type
    #[inline(always)]
    pub fn get_structure(&self, stype: u8) -> Option<&SmbiosStructure> {
        self.structures.iter().find(|s| s.stype == stype)
    }

    /// Get all structures of type
    #[inline(always)]
    pub fn get_structures(&self, stype: u8) -> Vec<&SmbiosStructure> {
        self.structures.iter().filter(|s| s.stype == stype).collect()
    }

    /// Get parsed info
    #[inline(always)]
    pub fn info(&self) -> &SmbiosInfo {
        &self.info
    }
}

impl Default for SmbiosParser {
    fn default() -> Self {
        Self::new()
    }
}
