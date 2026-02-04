//! SMBIOS Parser
//!
//! System Management BIOS table parsing for hardware information.

use core::fmt;

// =============================================================================
// SMBIOS ENTRY POINT
// =============================================================================

/// SMBIOS 2.x anchor string
pub const SMBIOS2_ANCHOR: [u8; 4] = *b"_SM_";

/// SMBIOS 3.x anchor string
pub const SMBIOS3_ANCHOR: [u8; 5] = *b"_SM3_";

/// DMI anchor string
pub const DMI_ANCHOR: [u8; 5] = *b"_DMI_";

/// SMBIOS 2.x entry point (32-bit)
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Smbios2EntryPoint {
    /// Anchor string "_SM_"
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
    pub entry_point_revision: u8,
    /// Formatted area
    pub formatted_area: [u8; 5],
    /// Intermediate anchor "_DMI_"
    pub intermediate_anchor: [u8; 5],
    /// Intermediate checksum
    pub intermediate_checksum: u8,
    /// Structure table length
    pub structure_table_length: u16,
    /// Structure table address
    pub structure_table_address: u32,
    /// Number of structures
    pub number_of_structures: u16,
    /// BCD revision
    pub bcd_revision: u8,
}

impl Smbios2EntryPoint {
    /// Size
    pub const SIZE: usize = 31;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        if &bytes[0..4] != &SMBIOS2_ANCHOR {
            return None;
        }

        Some(Self {
            anchor: bytes[0..4].try_into().ok()?,
            checksum: bytes[4],
            length: bytes[5],
            major_version: bytes[6],
            minor_version: bytes[7],
            max_structure_size: u16::from_le_bytes([bytes[8], bytes[9]]),
            entry_point_revision: bytes[10],
            formatted_area: bytes[11..16].try_into().ok()?,
            intermediate_anchor: bytes[16..21].try_into().ok()?,
            intermediate_checksum: bytes[21],
            structure_table_length: u16::from_le_bytes([bytes[22], bytes[23]]),
            structure_table_address: u32::from_le_bytes(bytes[24..28].try_into().ok()?),
            number_of_structures: u16::from_le_bytes([bytes[28], bytes[29]]),
            bcd_revision: bytes[30],
        })
    }

    /// Validate checksum
    pub fn validate_checksum(&self, bytes: &[u8]) -> bool {
        let len = self.length as usize;
        if bytes.len() < len {
            return false;
        }

        let sum: u8 = bytes[..len].iter().fold(0u8, |a, &b| a.wrapping_add(b));
        sum == 0
    }

    /// Get version string
    pub fn version(&self) -> (u8, u8) {
        (self.major_version, self.minor_version)
    }
}

/// SMBIOS 3.x entry point (64-bit)
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Smbios3EntryPoint {
    /// Anchor string "_SM3_"
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
    pub entry_point_revision: u8,
    /// Reserved
    pub reserved: u8,
    /// Maximum structure size
    pub structure_table_max_size: u32,
    /// Structure table address
    pub structure_table_address: u64,
}

impl Smbios3EntryPoint {
    /// Size
    pub const SIZE: usize = 24;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        if &bytes[0..5] != &SMBIOS3_ANCHOR {
            return None;
        }

        Some(Self {
            anchor: bytes[0..5].try_into().ok()?,
            checksum: bytes[5],
            length: bytes[6],
            major_version: bytes[7],
            minor_version: bytes[8],
            docrev: bytes[9],
            entry_point_revision: bytes[10],
            reserved: bytes[11],
            structure_table_max_size: u32::from_le_bytes(bytes[12..16].try_into().ok()?),
            structure_table_address: u64::from_le_bytes(bytes[16..24].try_into().ok()?),
        })
    }

    /// Validate checksum
    pub fn validate_checksum(&self, bytes: &[u8]) -> bool {
        let len = self.length as usize;
        if bytes.len() < len {
            return false;
        }

        let sum: u8 = bytes[..len].iter().fold(0u8, |a, &b| a.wrapping_add(b));
        sum == 0
    }

    /// Get version string
    pub fn version(&self) -> (u8, u8, u8) {
        (self.major_version, self.minor_version, self.docrev)
    }
}

// =============================================================================
// STRUCTURE HEADER
// =============================================================================

/// SMBIOS structure header
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct StructureHeader {
    /// Structure type
    pub structure_type: u8,
    /// Structure length
    pub length: u8,
    /// Handle
    pub handle: u16,
}

impl StructureHeader {
    /// Size
    pub const SIZE: usize = 4;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            structure_type: bytes[0],
            length: bytes[1],
            handle: u16::from_le_bytes([bytes[2], bytes[3]]),
        })
    }
}

// =============================================================================
// STRUCTURE TYPES
// =============================================================================

/// SMBIOS structure types
pub mod structure_type {
    pub const BIOS_INFORMATION: u8 = 0;
    pub const SYSTEM_INFORMATION: u8 = 1;
    pub const BASEBOARD_INFORMATION: u8 = 2;
    pub const SYSTEM_ENCLOSURE: u8 = 3;
    pub const PROCESSOR_INFORMATION: u8 = 4;
    pub const MEMORY_CONTROLLER: u8 = 5;
    pub const MEMORY_MODULE: u8 = 6;
    pub const CACHE_INFORMATION: u8 = 7;
    pub const PORT_CONNECTOR: u8 = 8;
    pub const SYSTEM_SLOTS: u8 = 9;
    pub const ON_BOARD_DEVICES: u8 = 10;
    pub const OEM_STRINGS: u8 = 11;
    pub const SYSTEM_CONFIG_OPTIONS: u8 = 12;
    pub const BIOS_LANGUAGE: u8 = 13;
    pub const GROUP_ASSOCIATIONS: u8 = 14;
    pub const SYSTEM_EVENT_LOG: u8 = 15;
    pub const PHYSICAL_MEMORY_ARRAY: u8 = 16;
    pub const MEMORY_DEVICE: u8 = 17;
    pub const MEMORY_ERROR_32BIT: u8 = 18;
    pub const MEMORY_ARRAY_MAPPED_ADDRESS: u8 = 19;
    pub const MEMORY_DEVICE_MAPPED_ADDRESS: u8 = 20;
    pub const BUILT_IN_POINTING_DEVICE: u8 = 21;
    pub const PORTABLE_BATTERY: u8 = 22;
    pub const SYSTEM_RESET: u8 = 23;
    pub const HARDWARE_SECURITY: u8 = 24;
    pub const SYSTEM_POWER_CONTROLS: u8 = 25;
    pub const VOLTAGE_PROBE: u8 = 26;
    pub const COOLING_DEVICE: u8 = 27;
    pub const TEMPERATURE_PROBE: u8 = 28;
    pub const ELECTRICAL_CURRENT_PROBE: u8 = 29;
    pub const OUT_OF_BAND_REMOTE_ACCESS: u8 = 30;
    pub const BOOT_INTEGRITY_SERVICES: u8 = 31;
    pub const SYSTEM_BOOT_INFORMATION: u8 = 32;
    pub const MEMORY_ERROR_64BIT: u8 = 33;
    pub const MANAGEMENT_DEVICE: u8 = 34;
    pub const MANAGEMENT_DEVICE_COMPONENT: u8 = 35;
    pub const MANAGEMENT_DEVICE_THRESHOLD: u8 = 36;
    pub const MEMORY_CHANNEL: u8 = 37;
    pub const IPMI_DEVICE: u8 = 38;
    pub const SYSTEM_POWER_SUPPLY: u8 = 39;
    pub const ADDITIONAL_INFORMATION: u8 = 40;
    pub const ONBOARD_DEVICES_EXTENDED: u8 = 41;
    pub const MANAGEMENT_CONTROLLER_HOST: u8 = 42;
    pub const TPM_DEVICE: u8 = 43;
    pub const PROCESSOR_ADDITIONAL: u8 = 44;
    pub const INACTIVE: u8 = 126;
    pub const END_OF_TABLE: u8 = 127;
}

// =============================================================================
// BIOS INFORMATION (TYPE 0)
// =============================================================================

/// BIOS Information (Type 0)
#[derive(Clone)]
pub struct BiosInformation<'a> {
    header: StructureHeader,
    data: &'a [u8],
    strings: StringTable<'a>,
}

impl<'a> BiosInformation<'a> {
    /// Parse from structure
    pub fn parse(data: &'a [u8], strings: StringTable<'a>) -> Option<Self> {
        let header = StructureHeader::from_bytes(data)?;
        if header.structure_type != structure_type::BIOS_INFORMATION {
            return None;
        }

        Some(Self {
            header,
            data,
            strings,
        })
    }

    /// Get vendor
    pub fn vendor(&self) -> Option<&str> {
        if self.data.len() > 4 {
            self.strings.get(self.data[4])
        } else {
            None
        }
    }

    /// Get version
    pub fn version(&self) -> Option<&str> {
        if self.data.len() > 5 {
            self.strings.get(self.data[5])
        } else {
            None
        }
    }

    /// Get release date
    pub fn release_date(&self) -> Option<&str> {
        if self.data.len() > 8 {
            self.strings.get(self.data[8])
        } else {
            None
        }
    }

    /// Get ROM size in KB
    pub fn rom_size_kb(&self) -> Option<u32> {
        if self.data.len() > 9 {
            let size_64k = self.data[9] as u32;
            Some((size_64k + 1) * 64)
        } else {
            None
        }
    }

    /// Get characteristics
    pub fn characteristics(&self) -> Option<u64> {
        if self.data.len() >= 18 {
            Some(u64::from_le_bytes(self.data[10..18].try_into().ok()?))
        } else {
            None
        }
    }
}

/// BIOS characteristics
pub mod bios_characteristics {
    pub const RESERVED: u64 = 1 << 0;
    pub const RESERVED2: u64 = 1 << 1;
    pub const UNKNOWN: u64 = 1 << 2;
    pub const NOT_SUPPORTED: u64 = 1 << 3;
    pub const ISA_SUPPORTED: u64 = 1 << 4;
    pub const MCA_SUPPORTED: u64 = 1 << 5;
    pub const EISA_SUPPORTED: u64 = 1 << 6;
    pub const PCI_SUPPORTED: u64 = 1 << 7;
    pub const PCMCIA_SUPPORTED: u64 = 1 << 8;
    pub const PNP_SUPPORTED: u64 = 1 << 9;
    pub const APM_SUPPORTED: u64 = 1 << 10;
    pub const UPGRADEABLE: u64 = 1 << 11;
    pub const SHADOWING_SUPPORTED: u64 = 1 << 12;
    pub const VL_VESA_SUPPORTED: u64 = 1 << 13;
    pub const ESCD_SUPPORTED: u64 = 1 << 14;
    pub const CD_BOOT_SUPPORTED: u64 = 1 << 15;
    pub const SELECTABLE_BOOT: u64 = 1 << 16;
    pub const ROM_SOCKETED: u64 = 1 << 17;
    pub const PCMCIA_BOOT: u64 = 1 << 18;
    pub const EDD_SUPPORTED: u64 = 1 << 19;
    pub const JAPANESE_FLOPPY_NEC: u64 = 1 << 20;
    pub const JAPANESE_FLOPPY_TOSHIBA: u64 = 1 << 21;
    pub const FLOPPY_360K: u64 = 1 << 22;
    pub const FLOPPY_1_2M: u64 = 1 << 23;
    pub const FLOPPY_720K: u64 = 1 << 24;
    pub const FLOPPY_2_88M: u64 = 1 << 25;
    pub const PRINT_SCREEN: u64 = 1 << 26;
    pub const KEYBOARD_8042: u64 = 1 << 27;
    pub const SERIAL_SERVICES: u64 = 1 << 28;
    pub const PRINTER_SERVICES: u64 = 1 << 29;
    pub const CGA_MONO_VIDEO: u64 = 1 << 30;
    pub const NEC_PC98: u64 = 1 << 31;
    pub const ACPI: u64 = 1 << 32;
    pub const USB_LEGACY: u64 = 1 << 33;
    pub const AGP: u64 = 1 << 34;
    pub const I20_BOOT: u64 = 1 << 35;
    pub const LS120_BOOT: u64 = 1 << 36;
    pub const ATAPI_ZIP_BOOT: u64 = 1 << 37;
    pub const IEEE_1394_BOOT: u64 = 1 << 38;
    pub const SMART_BATTERY: u64 = 1 << 39;
    pub const BIOS_BOOT_SPEC: u64 = 1 << 40;
    pub const FUNCTION_KEY_NETWORK_BOOT: u64 = 1 << 41;
    pub const TARGETED_CONTENT_DIST: u64 = 1 << 42;
    pub const UEFI_SUPPORTED: u64 = 1 << 43;
    pub const VIRTUAL_MACHINE: u64 = 1 << 44;
}

// =============================================================================
// SYSTEM INFORMATION (TYPE 1)
// =============================================================================

/// System Information (Type 1)
#[derive(Clone)]
pub struct SystemInformation<'a> {
    header: StructureHeader,
    data: &'a [u8],
    strings: StringTable<'a>,
}

impl<'a> SystemInformation<'a> {
    /// Parse from structure
    pub fn parse(data: &'a [u8], strings: StringTable<'a>) -> Option<Self> {
        let header = StructureHeader::from_bytes(data)?;
        if header.structure_type != structure_type::SYSTEM_INFORMATION {
            return None;
        }

        Some(Self {
            header,
            data,
            strings,
        })
    }

    /// Get manufacturer
    pub fn manufacturer(&self) -> Option<&str> {
        if self.data.len() > 4 {
            self.strings.get(self.data[4])
        } else {
            None
        }
    }

    /// Get product name
    pub fn product_name(&self) -> Option<&str> {
        if self.data.len() > 5 {
            self.strings.get(self.data[5])
        } else {
            None
        }
    }

    /// Get version
    pub fn version(&self) -> Option<&str> {
        if self.data.len() > 6 {
            self.strings.get(self.data[6])
        } else {
            None
        }
    }

    /// Get serial number
    pub fn serial_number(&self) -> Option<&str> {
        if self.data.len() > 7 {
            self.strings.get(self.data[7])
        } else {
            None
        }
    }

    /// Get UUID (16 bytes)
    pub fn uuid(&self) -> Option<[u8; 16]> {
        if self.data.len() >= 24 {
            Some(self.data[8..24].try_into().ok()?)
        } else {
            None
        }
    }

    /// Get wakeup type
    pub fn wakeup_type(&self) -> Option<WakeupType> {
        if self.data.len() > 24 {
            WakeupType::from_u8(self.data[24])
        } else {
            None
        }
    }

    /// Get SKU number
    pub fn sku_number(&self) -> Option<&str> {
        if self.data.len() > 25 {
            self.strings.get(self.data[25])
        } else {
            None
        }
    }

    /// Get family
    pub fn family(&self) -> Option<&str> {
        if self.data.len() > 26 {
            self.strings.get(self.data[26])
        } else {
            None
        }
    }
}

/// Wakeup type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeupType {
    Reserved,
    Other,
    Unknown,
    ApmTimer,
    ModemRing,
    LanRemote,
    PowerSwitch,
    PciPme,
    AcPowerRestored,
}

impl WakeupType {
    fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Reserved,
            1 => Self::Other,
            2 => Self::Unknown,
            3 => Self::ApmTimer,
            4 => Self::ModemRing,
            5 => Self::LanRemote,
            6 => Self::PowerSwitch,
            7 => Self::PciPme,
            8 => Self::AcPowerRestored,
            _ => return None,
        })
    }
}

// =============================================================================
// PROCESSOR INFORMATION (TYPE 4)
// =============================================================================

/// Processor Information (Type 4)
#[derive(Clone)]
pub struct ProcessorInformation<'a> {
    header: StructureHeader,
    data: &'a [u8],
    strings: StringTable<'a>,
}

impl<'a> ProcessorInformation<'a> {
    /// Parse from structure
    pub fn parse(data: &'a [u8], strings: StringTable<'a>) -> Option<Self> {
        let header = StructureHeader::from_bytes(data)?;
        if header.structure_type != structure_type::PROCESSOR_INFORMATION {
            return None;
        }

        Some(Self {
            header,
            data,
            strings,
        })
    }

    /// Get socket designation
    pub fn socket_designation(&self) -> Option<&str> {
        if self.data.len() > 4 {
            self.strings.get(self.data[4])
        } else {
            None
        }
    }

    /// Get processor type
    pub fn processor_type(&self) -> Option<ProcessorType> {
        if self.data.len() > 5 {
            ProcessorType::from_u8(self.data[5])
        } else {
            None
        }
    }

    /// Get processor family
    pub fn processor_family(&self) -> Option<u8> {
        if self.data.len() > 6 {
            Some(self.data[6])
        } else {
            None
        }
    }

    /// Get manufacturer
    pub fn manufacturer(&self) -> Option<&str> {
        if self.data.len() > 7 {
            self.strings.get(self.data[7])
        } else {
            None
        }
    }

    /// Get processor ID (8 bytes)
    pub fn processor_id(&self) -> Option<u64> {
        if self.data.len() >= 16 {
            Some(u64::from_le_bytes(self.data[8..16].try_into().ok()?))
        } else {
            None
        }
    }

    /// Get version
    pub fn version(&self) -> Option<&str> {
        if self.data.len() > 16 {
            self.strings.get(self.data[16])
        } else {
            None
        }
    }

    /// Get voltage
    pub fn voltage(&self) -> Option<f32> {
        if self.data.len() > 17 {
            let v = self.data[17];
            if v & 0x80 != 0 {
                Some((v & 0x7F) as f32 / 10.0)
            } else {
                // Legacy voltage values
                match v {
                    0x01 => Some(5.0),
                    0x02 => Some(3.3),
                    0x04 => Some(2.9),
                    _ => None,
                }
            }
        } else {
            None
        }
    }

    /// Get external clock in MHz
    pub fn external_clock(&self) -> Option<u16> {
        if self.data.len() >= 20 {
            Some(u16::from_le_bytes([self.data[18], self.data[19]]))
        } else {
            None
        }
    }

    /// Get max speed in MHz
    pub fn max_speed(&self) -> Option<u16> {
        if self.data.len() >= 22 {
            Some(u16::from_le_bytes([self.data[20], self.data[21]]))
        } else {
            None
        }
    }

    /// Get current speed in MHz
    pub fn current_speed(&self) -> Option<u16> {
        if self.data.len() >= 24 {
            Some(u16::from_le_bytes([self.data[22], self.data[23]]))
        } else {
            None
        }
    }

    /// Get status
    pub fn status(&self) -> Option<ProcessorStatus> {
        if self.data.len() > 24 {
            Some(ProcessorStatus::from_u8(self.data[24]))
        } else {
            None
        }
    }

    /// Get core count
    pub fn core_count(&self) -> Option<u8> {
        if self.data.len() > 35 {
            Some(self.data[35])
        } else {
            None
        }
    }

    /// Get enabled core count
    pub fn core_enabled(&self) -> Option<u8> {
        if self.data.len() > 36 {
            Some(self.data[36])
        } else {
            None
        }
    }

    /// Get thread count
    pub fn thread_count(&self) -> Option<u8> {
        if self.data.len() > 37 {
            Some(self.data[37])
        } else {
            None
        }
    }
}

/// Processor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessorType {
    Other,
    Unknown,
    CentralProcessor,
    MathProcessor,
    DspProcessor,
    VideoProcessor,
}

impl ProcessorType {
    fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            1 => Self::Other,
            2 => Self::Unknown,
            3 => Self::CentralProcessor,
            4 => Self::MathProcessor,
            5 => Self::DspProcessor,
            6 => Self::VideoProcessor,
            _ => return None,
        })
    }
}

/// Processor status
#[derive(Debug, Clone, Copy)]
pub struct ProcessorStatus {
    value: u8,
}

impl ProcessorStatus {
    fn from_u8(value: u8) -> Self {
        Self { value }
    }

    /// Is socket populated
    pub fn is_populated(&self) -> bool {
        self.value & 0x40 != 0
    }

    /// Get CPU status
    pub fn cpu_status(&self) -> CpuStatus {
        match self.value & 0x07 {
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

/// CPU status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuStatus {
    Unknown,
    Enabled,
    DisabledByUser,
    DisabledByBios,
    Idle,
    Other,
}

// =============================================================================
// MEMORY DEVICE (TYPE 17)
// =============================================================================

/// Memory Device (Type 17)
#[derive(Clone)]
pub struct MemoryDevice<'a> {
    header: StructureHeader,
    data: &'a [u8],
    strings: StringTable<'a>,
}

impl<'a> MemoryDevice<'a> {
    /// Parse from structure
    pub fn parse(data: &'a [u8], strings: StringTable<'a>) -> Option<Self> {
        let header = StructureHeader::from_bytes(data)?;
        if header.structure_type != structure_type::MEMORY_DEVICE {
            return None;
        }

        Some(Self {
            header,
            data,
            strings,
        })
    }

    /// Get physical memory array handle
    pub fn physical_memory_array_handle(&self) -> Option<u16> {
        if self.data.len() >= 6 {
            Some(u16::from_le_bytes([self.data[4], self.data[5]]))
        } else {
            None
        }
    }

    /// Get total width in bits
    pub fn total_width(&self) -> Option<u16> {
        if self.data.len() >= 10 {
            let width = u16::from_le_bytes([self.data[8], self.data[9]]);
            if width != 0xFFFF {
                Some(width)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get data width in bits
    pub fn data_width(&self) -> Option<u16> {
        if self.data.len() >= 12 {
            let width = u16::from_le_bytes([self.data[10], self.data[11]]);
            if width != 0xFFFF {
                Some(width)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get size in MB
    pub fn size_mb(&self) -> Option<u32> {
        if self.data.len() >= 14 {
            let size = u16::from_le_bytes([self.data[12], self.data[13]]);
            if size == 0 {
                return None; // No memory installed
            }
            if size == 0xFFFF {
                // Use extended size
                if self.data.len() >= 32 {
                    return Some(u32::from_le_bytes(self.data[28..32].try_into().ok()?));
                }
                return None;
            }
            if size & 0x8000 != 0 {
                // Size in KB
                Some(((size & 0x7FFF) as u32) / 1024)
            } else {
                // Size in MB
                Some(size as u32)
            }
        } else {
            None
        }
    }

    /// Get form factor
    pub fn form_factor(&self) -> Option<MemoryFormFactor> {
        if self.data.len() > 14 {
            MemoryFormFactor::from_u8(self.data[14])
        } else {
            None
        }
    }

    /// Get device locator
    pub fn device_locator(&self) -> Option<&str> {
        if self.data.len() > 16 {
            self.strings.get(self.data[16])
        } else {
            None
        }
    }

    /// Get bank locator
    pub fn bank_locator(&self) -> Option<&str> {
        if self.data.len() > 17 {
            self.strings.get(self.data[17])
        } else {
            None
        }
    }

    /// Get memory type
    pub fn memory_type(&self) -> Option<MemoryType> {
        if self.data.len() > 18 {
            MemoryType::from_u8(self.data[18])
        } else {
            None
        }
    }

    /// Get speed in MT/s
    pub fn speed(&self) -> Option<u16> {
        if self.data.len() >= 23 {
            let speed = u16::from_le_bytes([self.data[21], self.data[22]]);
            if speed != 0 {
                Some(speed)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get manufacturer
    pub fn manufacturer(&self) -> Option<&str> {
        if self.data.len() > 23 {
            self.strings.get(self.data[23])
        } else {
            None
        }
    }

    /// Get serial number
    pub fn serial_number(&self) -> Option<&str> {
        if self.data.len() > 24 {
            self.strings.get(self.data[24])
        } else {
            None
        }
    }

    /// Get part number
    pub fn part_number(&self) -> Option<&str> {
        if self.data.len() > 26 {
            self.strings.get(self.data[26])
        } else {
            None
        }
    }
}

/// Memory form factor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryFormFactor {
    Other,
    Unknown,
    Simm,
    Sip,
    Chip,
    Dip,
    Zip,
    ProprietaryCard,
    Dimm,
    Tsop,
    RowOfChips,
    Rimm,
    Sodimm,
    Srimm,
    FbDimm,
    Die,
}

impl MemoryFormFactor {
    fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::Simm,
            0x04 => Self::Sip,
            0x05 => Self::Chip,
            0x06 => Self::Dip,
            0x07 => Self::Zip,
            0x08 => Self::ProprietaryCard,
            0x09 => Self::Dimm,
            0x0A => Self::Tsop,
            0x0B => Self::RowOfChips,
            0x0C => Self::Rimm,
            0x0D => Self::Sodimm,
            0x0E => Self::Srimm,
            0x0F => Self::FbDimm,
            0x10 => Self::Die,
            _ => return None,
        })
    }
}

/// Memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    Other,
    Unknown,
    Dram,
    Edram,
    Vram,
    Sram,
    Ram,
    Rom,
    Flash,
    Eeprom,
    Feprom,
    Eprom,
    Cdram,
    Ram3D,
    Sdram,
    Sgram,
    Rdram,
    Ddr,
    Ddr2,
    Ddr2FbDimm,
    Ddr3,
    Fbd2,
    Ddr4,
    LpDdr,
    LpDdr2,
    LpDdr3,
    LpDdr4,
    LogicalNonVolatile,
    Hbm,
    Hbm2,
    Ddr5,
    LpDdr5,
}

impl MemoryType {
    fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::Dram,
            0x04 => Self::Edram,
            0x05 => Self::Vram,
            0x06 => Self::Sram,
            0x07 => Self::Ram,
            0x08 => Self::Rom,
            0x09 => Self::Flash,
            0x0A => Self::Eeprom,
            0x0B => Self::Feprom,
            0x0C => Self::Eprom,
            0x0D => Self::Cdram,
            0x0E => Self::Ram3D,
            0x0F => Self::Sdram,
            0x10 => Self::Sgram,
            0x11 => Self::Rdram,
            0x12 => Self::Ddr,
            0x13 => Self::Ddr2,
            0x14 => Self::Ddr2FbDimm,
            0x18 => Self::Ddr3,
            0x19 => Self::Fbd2,
            0x1A => Self::Ddr4,
            0x1B => Self::LpDdr,
            0x1C => Self::LpDdr2,
            0x1D => Self::LpDdr3,
            0x1E => Self::LpDdr4,
            0x1F => Self::LogicalNonVolatile,
            0x20 => Self::Hbm,
            0x21 => Self::Hbm2,
            0x22 => Self::Ddr5,
            0x23 => Self::LpDdr5,
            _ => return None,
        })
    }
}

// =============================================================================
// STRING TABLE
// =============================================================================

/// SMBIOS string table
#[derive(Clone)]
pub struct StringTable<'a> {
    data: &'a [u8],
}

impl<'a> StringTable<'a> {
    /// Create from data following structure
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Get string by 1-based index
    pub fn get(&self, index: u8) -> Option<&'a str> {
        if index == 0 {
            return None;
        }

        let mut current_index = 1u8;
        let mut pos = 0;

        while pos < self.data.len() {
            // Find end of current string
            let start = pos;
            while pos < self.data.len() && self.data[pos] != 0 {
                pos += 1;
            }

            if current_index == index {
                return core::str::from_utf8(&self.data[start..pos]).ok();
            }

            // Skip null terminator
            pos += 1;
            current_index += 1;

            // Check for double null (end of strings)
            if pos < self.data.len() && self.data[pos] == 0 {
                break;
            }
        }

        None
    }

    /// Iterate all strings
    pub fn iter(&self) -> StringTableIter<'a> {
        StringTableIter {
            data: self.data,
            pos: 0,
        }
    }
}

/// String table iterator
pub struct StringTableIter<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Iterator for StringTableIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.data.len() {
            return None;
        }

        // Check for end (double null)
        if self.data[self.pos] == 0 {
            return None;
        }

        let start = self.pos;
        while self.pos < self.data.len() && self.data[self.pos] != 0 {
            self.pos += 1;
        }

        let string = core::str::from_utf8(&self.data[start..self.pos]).ok()?;

        // Skip null terminator
        self.pos += 1;

        Some(string)
    }
}

// =============================================================================
// SMBIOS TABLE
// =============================================================================

/// SMBIOS table parser
pub struct SmbiosTable<'a> {
    data: &'a [u8],
    version: (u8, u8),
}

impl<'a> SmbiosTable<'a> {
    /// Create from structure table data
    pub fn new(data: &'a [u8], version: (u8, u8)) -> Self {
        Self { data, version }
    }

    /// Get version
    pub fn version(&self) -> (u8, u8) {
        self.version
    }

    /// Iterate all structures
    pub fn structures(&self) -> StructureIter<'a> {
        StructureIter {
            data: self.data,
            offset: 0,
        }
    }

    /// Find structure by type
    pub fn find_by_type(&self, structure_type: u8) -> Option<Structure<'a>> {
        self.structures()
            .find(|s| s.header.structure_type == structure_type)
    }

    /// Find all structures of type
    pub fn find_all_by_type(&self, structure_type: u8) -> impl Iterator<Item = Structure<'a>> {
        self.structures()
            .filter(move |s| s.header.structure_type == structure_type)
    }

    /// Get BIOS information
    pub fn bios_information(&self) -> Option<BiosInformation<'a>> {
        let structure = self.find_by_type(structure_type::BIOS_INFORMATION)?;
        BiosInformation::parse(structure.data, structure.strings)
    }

    /// Get system information
    pub fn system_information(&self) -> Option<SystemInformation<'a>> {
        let structure = self.find_by_type(structure_type::SYSTEM_INFORMATION)?;
        SystemInformation::parse(structure.data, structure.strings)
    }

    /// Get processor information
    pub fn processor_information(&self) -> impl Iterator<Item = ProcessorInformation<'a>> {
        self.find_all_by_type(structure_type::PROCESSOR_INFORMATION)
            .filter_map(|s| ProcessorInformation::parse(s.data, s.strings))
    }

    /// Get memory devices
    pub fn memory_devices(&self) -> impl Iterator<Item = MemoryDevice<'a>> {
        self.find_all_by_type(structure_type::MEMORY_DEVICE)
            .filter_map(|s| MemoryDevice::parse(s.data, s.strings))
    }

    /// Get total system memory in MB
    pub fn total_memory_mb(&self) -> u64 {
        self.memory_devices()
            .filter_map(|m| m.size_mb())
            .map(|s| s as u64)
            .sum()
    }
}

/// Raw structure with string table
pub struct Structure<'a> {
    pub header: StructureHeader,
    pub data: &'a [u8],
    pub strings: StringTable<'a>,
}

/// Structure iterator
pub struct StructureIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for StructureIter<'a> {
    type Item = Structure<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + StructureHeader::SIZE > self.data.len() {
            return None;
        }

        let header = StructureHeader::from_bytes(&self.data[self.offset..])?;

        // Check for end of table
        if header.structure_type == structure_type::END_OF_TABLE {
            return None;
        }

        let structure_end = self.offset + header.length as usize;
        if structure_end > self.data.len() {
            return None;
        }

        let structure_data = &self.data[self.offset..structure_end];

        // Find end of string table (double null)
        let mut string_end = structure_end;
        while string_end + 1 < self.data.len() {
            if self.data[string_end] == 0 && self.data[string_end + 1] == 0 {
                string_end += 2;
                break;
            }
            string_end += 1;
        }

        let string_data = &self.data[structure_end..string_end];
        let strings = StringTable::new(string_data);

        let structure = Structure {
            header,
            data: structure_data,
            strings,
        };

        self.offset = string_end;
        Some(structure)
    }
}

// =============================================================================
// SMBIOS ERROR
// =============================================================================

/// SMBIOS error
#[derive(Debug, Clone)]
pub enum SmbiosError {
    /// Invalid entry point
    InvalidEntryPoint,
    /// Invalid structure
    InvalidStructure,
    /// Checksum error
    ChecksumError,
    /// Not found
    NotFound,
}

impl fmt::Display for SmbiosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEntryPoint => write!(f, "invalid SMBIOS entry point"),
            Self::InvalidStructure => write!(f, "invalid SMBIOS structure"),
            Self::ChecksumError => write!(f, "SMBIOS checksum error"),
            Self::NotFound => write!(f, "SMBIOS table not found"),
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_smbios_anchors() {
        assert_eq!(&SMBIOS2_ANCHOR, b"_SM_");
        assert_eq!(&SMBIOS3_ANCHOR, b"_SM3_");
        assert_eq!(&DMI_ANCHOR, b"_DMI_");
    }

    #[test]
    fn test_memory_type() {
        assert!(matches!(MemoryType::from_u8(0x1A), Some(MemoryType::Ddr4)));
        assert!(matches!(MemoryType::from_u8(0x22), Some(MemoryType::Ddr5)));
    }

    #[test]
    fn test_string_table() {
        let data = b"First\0Second\0Third\0\0";
        let table = StringTable::new(data);

        assert_eq!(table.get(1), Some("First"));
        assert_eq!(table.get(2), Some("Second"));
        assert_eq!(table.get(3), Some("Third"));
        assert_eq!(table.get(4), None);
        assert_eq!(table.get(0), None);
    }
}
