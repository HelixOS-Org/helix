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

        if bytes[0..4] != SMBIOS2_ANCHOR {
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
    #[must_use]
    pub const fn version(&self) -> (u8, u8) {
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

        if bytes[0..5] != SMBIOS3_ANCHOR {
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
    #[must_use]
    pub const fn version(&self) -> (u8, u8, u8) {
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
    /// BIOS information (Type 0)
    pub const BIOS_INFORMATION: u8 = 0;
    /// System information (Type 1)
    pub const SYSTEM_INFORMATION: u8 = 1;
    /// Baseboard information (Type 2)
    pub const BASEBOARD_INFORMATION: u8 = 2;
    /// System enclosure (Type 3)
    pub const SYSTEM_ENCLOSURE: u8 = 3;
    /// Processor information (Type 4)
    pub const PROCESSOR_INFORMATION: u8 = 4;
    /// Memory controller (Type 5)
    pub const MEMORY_CONTROLLER: u8 = 5;
    /// Memory module (Type 6)
    pub const MEMORY_MODULE: u8 = 6;
    /// Cache information (Type 7)
    pub const CACHE_INFORMATION: u8 = 7;
    /// Port connector (Type 8)
    pub const PORT_CONNECTOR: u8 = 8;
    /// System slots (Type 9)
    pub const SYSTEM_SLOTS: u8 = 9;
    /// On-board devices (Type 10)
    pub const ON_BOARD_DEVICES: u8 = 10;
    /// OEM strings (Type 11)
    pub const OEM_STRINGS: u8 = 11;
    /// System configuration options (Type 12)
    pub const SYSTEM_CONFIG_OPTIONS: u8 = 12;
    /// BIOS language (Type 13)
    pub const BIOS_LANGUAGE: u8 = 13;
    /// Group associations (Type 14)
    pub const GROUP_ASSOCIATIONS: u8 = 14;
    /// System event log (Type 15)
    pub const SYSTEM_EVENT_LOG: u8 = 15;
    /// Physical memory array (Type 16)
    pub const PHYSICAL_MEMORY_ARRAY: u8 = 16;
    /// Memory device (Type 17)
    pub const MEMORY_DEVICE: u8 = 17;
    /// 32-bit memory error (Type 18)
    pub const MEMORY_ERROR_32BIT: u8 = 18;
    /// Memory array mapped address (Type 19)
    pub const MEMORY_ARRAY_MAPPED_ADDRESS: u8 = 19;
    /// Memory device mapped address (Type 20)
    pub const MEMORY_DEVICE_MAPPED_ADDRESS: u8 = 20;
    /// Built-in pointing device (Type 21)
    pub const BUILT_IN_POINTING_DEVICE: u8 = 21;
    /// Portable battery (Type 22)
    pub const PORTABLE_BATTERY: u8 = 22;
    /// System reset (Type 23)
    pub const SYSTEM_RESET: u8 = 23;
    /// Hardware security (Type 24)
    pub const HARDWARE_SECURITY: u8 = 24;
    /// System power controls (Type 25)
    pub const SYSTEM_POWER_CONTROLS: u8 = 25;
    /// Voltage probe (Type 26)
    pub const VOLTAGE_PROBE: u8 = 26;
    /// Cooling device (Type 27)
    pub const COOLING_DEVICE: u8 = 27;
    /// Temperature probe (Type 28)
    pub const TEMPERATURE_PROBE: u8 = 28;
    /// Electrical current probe (Type 29)
    pub const ELECTRICAL_CURRENT_PROBE: u8 = 29;
    /// Out-of-band remote access (Type 30)
    pub const OUT_OF_BAND_REMOTE_ACCESS: u8 = 30;
    /// Boot integrity services (Type 31)
    pub const BOOT_INTEGRITY_SERVICES: u8 = 31;
    /// System boot information (Type 32)
    pub const SYSTEM_BOOT_INFORMATION: u8 = 32;
    /// 64-bit memory error (Type 33)
    pub const MEMORY_ERROR_64BIT: u8 = 33;
    /// Management device (Type 34)
    pub const MANAGEMENT_DEVICE: u8 = 34;
    /// Management device component (Type 35)
    pub const MANAGEMENT_DEVICE_COMPONENT: u8 = 35;
    /// Management device threshold (Type 36)
    pub const MANAGEMENT_DEVICE_THRESHOLD: u8 = 36;
    /// Memory channel (Type 37)
    pub const MEMORY_CHANNEL: u8 = 37;
    /// IPMI device (Type 38)
    pub const IPMI_DEVICE: u8 = 38;
    /// System power supply (Type 39)
    pub const SYSTEM_POWER_SUPPLY: u8 = 39;
    /// Additional information (Type 40)
    pub const ADDITIONAL_INFORMATION: u8 = 40;
    /// Onboard devices extended (Type 41)
    pub const ONBOARD_DEVICES_EXTENDED: u8 = 41;
    /// Management controller host (Type 42)
    pub const MANAGEMENT_CONTROLLER_HOST: u8 = 42;
    /// TPM device (Type 43)
    pub const TPM_DEVICE: u8 = 43;
    /// Processor additional (Type 44)
    pub const PROCESSOR_ADDITIONAL: u8 = 44;
    /// Inactive structure (Type 126)
    pub const INACTIVE: u8 = 126;
    /// End of table (Type 127)
    pub const END_OF_TABLE: u8 = 127;
}

// =============================================================================
// BIOS INFORMATION (TYPE 0)
// =============================================================================

/// BIOS Information (Type 0)
#[derive(Clone)]
pub struct BiosInformation<'a> {
    _header: StructureHeader,
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
            _header: header,
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
            let size_64k = u32::from(self.data[9]);
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
    /// Reserved bit 0
    pub const RESERVED: u64 = 1 << 0;
    /// Reserved bit 1
    pub const RESERVED2: u64 = 1 << 1;
    /// Unknown characteristics
    pub const UNKNOWN: u64 = 1 << 2;
    /// BIOS characteristics not supported
    pub const NOT_SUPPORTED: u64 = 1 << 3;
    /// ISA is supported
    pub const ISA_SUPPORTED: u64 = 1 << 4;
    /// MCA is supported
    pub const MCA_SUPPORTED: u64 = 1 << 5;
    /// EISA is supported
    pub const EISA_SUPPORTED: u64 = 1 << 6;
    /// PCI is supported
    pub const PCI_SUPPORTED: u64 = 1 << 7;
    /// PC Card (PCMCIA) is supported
    pub const PCMCIA_SUPPORTED: u64 = 1 << 8;
    /// Plug and Play is supported
    pub const PNP_SUPPORTED: u64 = 1 << 9;
    /// APM is supported
    pub const APM_SUPPORTED: u64 = 1 << 10;
    /// BIOS is upgradeable
    pub const UPGRADEABLE: u64 = 1 << 11;
    /// BIOS shadowing is supported
    pub const SHADOWING_SUPPORTED: u64 = 1 << 12;
    /// VL-VESA is supported
    pub const VL_VESA_SUPPORTED: u64 = 1 << 13;
    /// ESCD support is available
    pub const ESCD_SUPPORTED: u64 = 1 << 14;
    /// Boot from CD is supported
    pub const CD_BOOT_SUPPORTED: u64 = 1 << 15;
    /// Selectable boot is supported
    pub const SELECTABLE_BOOT: u64 = 1 << 16;
    /// BIOS ROM is socketed
    pub const ROM_SOCKETED: u64 = 1 << 17;
    /// Boot from PC Card is supported
    pub const PCMCIA_BOOT: u64 = 1 << 18;
    /// EDD specification is supported
    pub const EDD_SUPPORTED: u64 = 1 << 19;
    /// NEC 9800 Japanese floppy is supported
    pub const JAPANESE_FLOPPY_NEC: u64 = 1 << 20;
    /// Toshiba Japanese floppy is supported
    pub const JAPANESE_FLOPPY_TOSHIBA: u64 = 1 << 21;
    /// 360 KB floppy is supported
    pub const FLOPPY_360K: u64 = 1 << 22;
    /// 1.2 MB floppy is supported
    pub const FLOPPY_1_2M: u64 = 1 << 23;
    /// 720 KB floppy is supported
    pub const FLOPPY_720K: u64 = 1 << 24;
    /// 2.88 MB floppy is supported
    pub const FLOPPY_2_88M: u64 = 1 << 25;
    /// Print screen service is supported
    pub const PRINT_SCREEN: u64 = 1 << 26;
    /// 8042 keyboard services are supported
    pub const KEYBOARD_8042: u64 = 1 << 27;
    /// Serial services are supported
    pub const SERIAL_SERVICES: u64 = 1 << 28;
    /// Printer services are supported
    pub const PRINTER_SERVICES: u64 = 1 << 29;
    /// CGA/Mono video services are supported
    pub const CGA_MONO_VIDEO: u64 = 1 << 30;
    /// NEC PC-98 is supported
    pub const NEC_PC98: u64 = 1 << 31;
    /// ACPI is supported
    pub const ACPI: u64 = 1 << 32;
    /// USB Legacy is supported
    pub const USB_LEGACY: u64 = 1 << 33;
    /// AGP is supported
    pub const AGP: u64 = 1 << 34;
    /// I2O boot is supported
    pub const I20_BOOT: u64 = 1 << 35;
    /// LS-120 `SuperDisk` boot is supported
    pub const LS120_BOOT: u64 = 1 << 36;
    /// ATAPI ZIP drive boot is supported
    pub const ATAPI_ZIP_BOOT: u64 = 1 << 37;
    /// IEEE 1394 boot is supported
    pub const IEEE_1394_BOOT: u64 = 1 << 38;
    /// Smart battery is supported
    pub const SMART_BATTERY: u64 = 1 << 39;
    /// BIOS Boot Specification is supported
    pub const BIOS_BOOT_SPEC: u64 = 1 << 40;
    /// Function key-initiated network boot is supported
    pub const FUNCTION_KEY_NETWORK_BOOT: u64 = 1 << 41;
    /// Targeted content distribution is supported
    pub const TARGETED_CONTENT_DIST: u64 = 1 << 42;
    /// UEFI is supported
    pub const UEFI_SUPPORTED: u64 = 1 << 43;
    /// System is a virtual machine
    pub const VIRTUAL_MACHINE: u64 = 1 << 44;
}

// =============================================================================
// SYSTEM INFORMATION (TYPE 1)
// =============================================================================

/// System Information (Type 1)
#[derive(Clone)]
pub struct SystemInformation<'a> {
    _header: StructureHeader,
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
            _header: header,
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
    /// Reserved value
    Reserved,
    /// Other wakeup type
    Other,
    /// Unknown wakeup type
    Unknown,
    /// APM timer wakeup
    ApmTimer,
    /// Modem ring wakeup
    ModemRing,
    /// LAN remote wakeup
    LanRemote,
    /// Power switch wakeup
    PowerSwitch,
    /// PCI PME wakeup
    PciPme,
    /// AC power restored wakeup
    AcPowerRestored,
}

impl WakeupType {
    const fn from_u8(value: u8) -> Option<Self> {
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
    _header: StructureHeader,
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
            _header: header,
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
                Some(f32::from(v & 0x7F) / 10.0)
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

    /// Get external clock in `MHz`
    pub fn external_clock(&self) -> Option<u16> {
        if self.data.len() >= 20 {
            Some(u16::from_le_bytes([self.data[18], self.data[19]]))
        } else {
            None
        }
    }

    /// Get max speed in `MHz`
    pub fn max_speed(&self) -> Option<u16> {
        if self.data.len() >= 22 {
            Some(u16::from_le_bytes([self.data[20], self.data[21]]))
        } else {
            None
        }
    }

    /// Get current speed in `MHz`
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
    /// Other processor type
    Other,
    /// Unknown processor type
    Unknown,
    /// Central processor (CPU)
    CentralProcessor,
    /// Math processor (FPU)
    MathProcessor,
    /// DSP processor
    DspProcessor,
    /// Video processor (GPU)
    VideoProcessor,
}

impl ProcessorType {
    const fn from_u8(value: u8) -> Option<Self> {
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
    const fn from_u8(value: u8) -> Self {
        Self { value }
    }

    /// Is socket populated
    #[must_use]
    pub const fn is_populated(&self) -> bool {
        self.value & 0x40 != 0
    }

    /// Get CPU status
    #[must_use]
    pub const fn cpu_status(&self) -> CpuStatus {
        match self.value & 0x07 {
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
    /// Unknown CPU status
    Unknown,
    /// CPU is enabled
    Enabled,
    /// CPU is disabled by user
    DisabledByUser,
    /// CPU is disabled by BIOS
    DisabledByBios,
    /// CPU is idle
    Idle,
    /// Other CPU status
    Other,
}

// =============================================================================
// MEMORY DEVICE (TYPE 17)
// =============================================================================

/// Memory Device (Type 17)
#[derive(Clone)]
pub struct MemoryDevice<'a> {
    _header: StructureHeader,
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
            _header: header,
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
            (width != 0xFFFF).then_some(width)
        } else {
            None
        }
    }

    /// Get data width in bits
    pub fn data_width(&self) -> Option<u16> {
        if self.data.len() >= 12 {
            let width = u16::from_le_bytes([self.data[10], self.data[11]]);
            (width != 0xFFFF).then_some(width)
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
                Some(u32::from(size & 0x7FFF) / 1024)
            } else {
                // Size in MB
                Some(u32::from(size))
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
    /// Other form factor
    Other,
    /// Unknown form factor
    Unknown,
    /// SIMM module
    Simm,
    /// SIP module
    Sip,
    /// Chip on board
    Chip,
    /// DIP package
    Dip,
    /// ZIP package
    Zip,
    /// Proprietary card
    ProprietaryCard,
    /// DIMM module
    Dimm,
    /// TSOP package
    Tsop,
    /// Row of chips
    RowOfChips,
    /// RIMM module
    Rimm,
    /// SO-DIMM module
    Sodimm,
    /// SRIMM module
    Srimm,
    /// FB-DIMM module
    FbDimm,
    /// Die form factor
    Die,
}

impl MemoryFormFactor {
    const fn from_u8(value: u8) -> Option<Self> {
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
    /// Other memory type
    Other,
    /// Unknown memory type
    Unknown,
    /// Dynamic RAM
    Dram,
    /// Enhanced DRAM
    Edram,
    /// Video RAM
    Vram,
    /// Static RAM
    Sram,
    /// Random Access Memory
    Ram,
    /// Read-Only Memory
    Rom,
    /// Flash memory
    Flash,
    /// Electrically Erasable PROM
    Eeprom,
    /// Flash EPROM
    Feprom,
    /// Erasable PROM
    Eprom,
    /// Cache DRAM
    Cdram,
    /// 3D RAM
    Ram3D,
    /// Synchronous DRAM
    Sdram,
    /// Synchronous Graphics RAM
    Sgram,
    /// Rambus DRAM
    Rdram,
    /// DDR SDRAM
    Ddr,
    /// DDR2 SDRAM
    Ddr2,
    /// DDR2 FB-DIMM
    Ddr2FbDimm,
    /// DDR3 SDRAM
    Ddr3,
    /// FBD2 memory
    Fbd2,
    /// DDR4 SDRAM
    Ddr4,
    /// Low-Power DDR
    LpDdr,
    /// Low-Power DDR2
    LpDdr2,
    /// Low-Power DDR3
    LpDdr3,
    /// Low-Power DDR4
    LpDdr4,
    /// Logical non-volatile device
    LogicalNonVolatile,
    /// High Bandwidth Memory
    Hbm,
    /// High Bandwidth Memory 2
    Hbm2,
    /// DDR5 SDRAM
    Ddr5,
    /// Low-Power DDR5
    LpDdr5,
}

impl MemoryType {
    const fn from_u8(value: u8) -> Option<Self> {
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
    #[must_use]
    pub const fn new(data: &'a [u8]) -> Self {
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
    #[must_use]
    pub const fn iter(&self) -> StringTableIter<'a> {
        StringTableIter {
            data: self.data,
            pos: 0,
        }
    }
}

impl<'a> IntoIterator for &StringTable<'a> {
    type Item = &'a str;
    type IntoIter = StringTableIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
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
    #[must_use]
    pub const fn new(data: &'a [u8], version: (u8, u8)) -> Self {
        Self { data, version }
    }

    /// Get version
    #[must_use]
    pub const fn version(&self) -> (u8, u8) {
        self.version
    }

    /// Iterate all structures
    #[must_use]
    pub const fn structures(&self) -> StructureIter<'a> {
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
            .map(u64::from)
            .sum()
    }
}

/// Raw structure with string table
pub struct Structure<'a> {
    /// Structure header
    pub header: StructureHeader,
    /// Raw structure data
    pub data: &'a [u8],
    /// String table for this structure
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
