//! PCI/PCIe Configuration and Access
//!
//! PCI configuration space access, device enumeration, and BAR handling.

use core::fmt;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Maximum PCI buses
pub const MAX_PCI_BUS: u8 = 255;

/// Maximum PCI devices per bus
pub const MAX_PCI_DEVICE: u8 = 31;

/// Maximum PCI functions per device
pub const MAX_PCI_FUNCTION: u8 = 7;

/// PCI configuration space size (legacy)
pub const PCI_CONFIG_SPACE_SIZE: usize = 256;

/// PCIe extended configuration space size
pub const PCIE_CONFIG_SPACE_SIZE: usize = 4096;

/// Invalid vendor ID
pub const INVALID_VENDOR_ID: u16 = 0xFFFF;

// =============================================================================
// PCI ADDRESS
// =============================================================================

/// PCI address (bus:device.function)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PciAddress {
    /// Segment (domain)
    pub segment: u16,
    /// Bus number
    pub bus: u8,
    /// Device number (0-31)
    pub device: u8,
    /// Function number (0-7)
    pub function: u8,
}

impl PciAddress {
    /// Create new PCI address
    pub const fn new(bus: u8, device: u8, function: u8) -> Self {
        Self {
            segment: 0,
            bus,
            device: device & 0x1F,
            function: function & 0x07,
        }
    }

    /// Create with segment
    pub const fn with_segment(segment: u16, bus: u8, device: u8, function: u8) -> Self {
        Self {
            segment,
            bus,
            device: device & 0x1F,
            function: function & 0x07,
        }
    }

    /// Create from BDF (legacy format)
    pub const fn from_bdf(bdf: u16) -> Self {
        Self {
            segment: 0,
            bus: (bdf >> 8) as u8,
            device: ((bdf >> 3) & 0x1F) as u8,
            function: (bdf & 0x07) as u8,
        }
    }

    /// Convert to BDF
    pub const fn to_bdf(self) -> u16 {
        ((self.bus as u16) << 8) | ((self.device as u16) << 3) | (self.function as u16)
    }

    /// Calculate legacy config address for I/O access
    pub const fn config_address(self, offset: u8) -> u32 {
        0x80000000 |
        ((self.bus as u32) << 16) |
        ((self.device as u32) << 11) |
        ((self.function as u32) << 8) |
        ((offset as u32) & 0xFC)
    }

    /// Calculate ECAM (PCIe) address offset
    pub const fn ecam_offset(self, offset: u16) -> u64 {
        ((self.bus as u64) << 20) |
        ((self.device as u64) << 15) |
        ((self.function as u64) << 12) |
        ((offset as u64) & 0xFFF)
    }

    /// Is root (bus 0, device 0, function 0)
    pub const fn is_root(self) -> bool {
        self.bus == 0 && self.device == 0 && self.function == 0
    }

    /// Is multi-function base (function 0)
    pub const fn is_function_zero(self) -> bool {
        self.function == 0
    }
}

impl fmt::Display for PciAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.segment != 0 {
            write!(f, "{:04x}:{:02x}:{:02x}.{}",
                self.segment, self.bus, self.device, self.function)
        } else {
            write!(f, "{:02x}:{:02x}.{}", self.bus, self.device, self.function)
        }
    }
}

// =============================================================================
// PCI CONFIGURATION HEADER
// =============================================================================

/// PCI configuration header (common fields)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PciConfigHeader {
    /// Vendor ID
    pub vendor_id: u16,
    /// Device ID
    pub device_id: u16,
    /// Command register
    pub command: u16,
    /// Status register
    pub status: u16,
    /// Revision ID
    pub revision_id: u8,
    /// Programming interface
    pub prog_if: u8,
    /// Subclass
    pub subclass: u8,
    /// Class code
    pub class_code: u8,
    /// Cache line size
    pub cache_line_size: u8,
    /// Latency timer
    pub latency_timer: u8,
    /// Header type
    pub header_type: u8,
    /// BIST
    pub bist: u8,
}

impl PciConfigHeader {
    /// Size of common header
    pub const SIZE: usize = 16;

    /// Read from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            vendor_id: u16::from_le_bytes([bytes[0], bytes[1]]),
            device_id: u16::from_le_bytes([bytes[2], bytes[3]]),
            command: u16::from_le_bytes([bytes[4], bytes[5]]),
            status: u16::from_le_bytes([bytes[6], bytes[7]]),
            revision_id: bytes[8],
            prog_if: bytes[9],
            subclass: bytes[10],
            class_code: bytes[11],
            cache_line_size: bytes[12],
            latency_timer: bytes[13],
            header_type: bytes[14],
            bist: bytes[15],
        })
    }

    /// Is valid (vendor ID not 0xFFFF)
    pub fn is_valid(&self) -> bool {
        self.vendor_id != INVALID_VENDOR_ID
    }

    /// Get header type (0, 1, or 2)
    pub fn header_type_num(&self) -> u8 {
        self.header_type & 0x7F
    }

    /// Is multi-function device
    pub fn is_multi_function(&self) -> bool {
        self.header_type & 0x80 != 0
    }

    /// Is type 0 (endpoint)
    pub fn is_type0(&self) -> bool {
        self.header_type_num() == 0
    }

    /// Is type 1 (bridge)
    pub fn is_type1(&self) -> bool {
        self.header_type_num() == 1
    }

    /// Get class as tuple
    pub fn class(&self) -> (u8, u8, u8) {
        (self.class_code, self.subclass, self.prog_if)
    }

    /// Get full class ID
    pub fn class_id(&self) -> u32 {
        ((self.class_code as u32) << 16) |
        ((self.subclass as u32) << 8) |
        (self.prog_if as u32)
    }
}

// =============================================================================
// PCI TYPE 0 HEADER (ENDPOINT)
// =============================================================================

/// PCI Type 0 header (endpoint device)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PciType0Header {
    /// Common header
    pub common: PciConfigHeader,
    /// Base Address Registers
    pub bar: [u32; 6],
    /// Cardbus CIS pointer
    pub cardbus_cis: u32,
    /// Subsystem vendor ID
    pub subsystem_vendor_id: u16,
    /// Subsystem ID
    pub subsystem_id: u16,
    /// Expansion ROM base address
    pub expansion_rom: u32,
    /// Capabilities pointer
    pub capabilities_ptr: u8,
    /// Reserved
    _reserved: [u8; 7],
    /// Interrupt line
    pub interrupt_line: u8,
    /// Interrupt pin
    pub interrupt_pin: u8,
    /// Min grant
    pub min_grant: u8,
    /// Max latency
    pub max_latency: u8,
}

impl PciType0Header {
    /// Size of type 0 header
    pub const SIZE: usize = 64;

    /// Read from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        let common = PciConfigHeader::from_bytes(bytes)?;

        Some(Self {
            common,
            bar: [
                u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
                u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
                u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
                u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]),
                u32::from_le_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]),
                u32::from_le_bytes([bytes[36], bytes[37], bytes[38], bytes[39]]),
            ],
            cardbus_cis: u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]),
            subsystem_vendor_id: u16::from_le_bytes([bytes[44], bytes[45]]),
            subsystem_id: u16::from_le_bytes([bytes[46], bytes[47]]),
            expansion_rom: u32::from_le_bytes([bytes[48], bytes[49], bytes[50], bytes[51]]),
            capabilities_ptr: bytes[52],
            _reserved: [bytes[53], bytes[54], bytes[55], bytes[56], bytes[57], bytes[58], bytes[59]],
            interrupt_line: bytes[60],
            interrupt_pin: bytes[61],
            min_grant: bytes[62],
            max_latency: bytes[63],
        })
    }

    /// Has capabilities
    pub fn has_capabilities(&self) -> bool {
        self.common.status & 0x10 != 0
    }
}

// =============================================================================
// PCI TYPE 1 HEADER (BRIDGE)
// =============================================================================

/// PCI Type 1 header (PCI-to-PCI bridge)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PciType1Header {
    /// Common header
    pub common: PciConfigHeader,
    /// Base Address Registers
    pub bar: [u32; 2],
    /// Primary bus number
    pub primary_bus: u8,
    /// Secondary bus number
    pub secondary_bus: u8,
    /// Subordinate bus number
    pub subordinate_bus: u8,
    /// Secondary latency timer
    pub secondary_latency_timer: u8,
    /// I/O base
    pub io_base: u8,
    /// I/O limit
    pub io_limit: u8,
    /// Secondary status
    pub secondary_status: u16,
    /// Memory base
    pub memory_base: u16,
    /// Memory limit
    pub memory_limit: u16,
    /// Prefetchable memory base
    pub prefetch_base: u16,
    /// Prefetchable memory limit
    pub prefetch_limit: u16,
    /// Prefetchable base upper 32 bits
    pub prefetch_base_upper: u32,
    /// Prefetchable limit upper 32 bits
    pub prefetch_limit_upper: u32,
    /// I/O base upper 16 bits
    pub io_base_upper: u16,
    /// I/O limit upper 16 bits
    pub io_limit_upper: u16,
    /// Capabilities pointer
    pub capabilities_ptr: u8,
    /// Reserved
    _reserved: [u8; 3],
    /// Expansion ROM base
    pub expansion_rom: u32,
    /// Interrupt line
    pub interrupt_line: u8,
    /// Interrupt pin
    pub interrupt_pin: u8,
    /// Bridge control
    pub bridge_control: u16,
}

impl PciType1Header {
    /// Size of type 1 header
    pub const SIZE: usize = 64;

    /// Read from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        let common = PciConfigHeader::from_bytes(bytes)?;

        Some(Self {
            common,
            bar: [
                u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
                u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
            ],
            primary_bus: bytes[24],
            secondary_bus: bytes[25],
            subordinate_bus: bytes[26],
            secondary_latency_timer: bytes[27],
            io_base: bytes[28],
            io_limit: bytes[29],
            secondary_status: u16::from_le_bytes([bytes[30], bytes[31]]),
            memory_base: u16::from_le_bytes([bytes[32], bytes[33]]),
            memory_limit: u16::from_le_bytes([bytes[34], bytes[35]]),
            prefetch_base: u16::from_le_bytes([bytes[36], bytes[37]]),
            prefetch_limit: u16::from_le_bytes([bytes[38], bytes[39]]),
            prefetch_base_upper: u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]),
            prefetch_limit_upper: u32::from_le_bytes([bytes[44], bytes[45], bytes[46], bytes[47]]),
            io_base_upper: u16::from_le_bytes([bytes[48], bytes[49]]),
            io_limit_upper: u16::from_le_bytes([bytes[50], bytes[51]]),
            capabilities_ptr: bytes[52],
            _reserved: [bytes[53], bytes[54], bytes[55]],
            expansion_rom: u32::from_le_bytes([bytes[56], bytes[57], bytes[58], bytes[59]]),
            interrupt_line: bytes[60],
            interrupt_pin: bytes[61],
            bridge_control: u16::from_le_bytes([bytes[62], bytes[63]]),
        })
    }

    /// Get memory window
    pub fn memory_window(&self) -> (u32, u32) {
        let base = (self.memory_base as u32 & 0xFFF0) << 16;
        let limit = ((self.memory_limit as u32 & 0xFFF0) << 16) | 0xFFFFF;
        (base, limit)
    }

    /// Get I/O window
    pub fn io_window(&self) -> (u32, u32) {
        let base = ((self.io_base as u32 & 0xF0) << 8) | ((self.io_base_upper as u32) << 16);
        let limit = ((self.io_limit as u32 & 0xF0) << 8) | ((self.io_limit_upper as u32) << 16) | 0xFFF;
        (base, limit)
    }

    /// Get prefetchable memory window
    pub fn prefetch_window(&self) -> (u64, u64) {
        let base = ((self.prefetch_base as u64 & 0xFFF0) << 16) |
                   ((self.prefetch_base_upper as u64) << 32);
        let limit = ((self.prefetch_limit as u64 & 0xFFF0) << 16) |
                    ((self.prefetch_limit_upper as u64) << 32) | 0xFFFFF;
        (base, limit)
    }
}

// =============================================================================
// BASE ADDRESS REGISTER (BAR)
// =============================================================================

/// BAR type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarType {
    /// Memory BAR (32-bit)
    Memory32,
    /// Memory BAR (64-bit, uses two consecutive BARs)
    Memory64,
    /// I/O BAR
    Io,
    /// Invalid or disabled
    Invalid,
}

/// Base Address Register information
#[derive(Debug, Clone, Copy)]
pub struct Bar {
    /// BAR index (0-5)
    pub index: u8,
    /// BAR type
    pub bar_type: BarType,
    /// Base address
    pub address: u64,
    /// Size in bytes
    pub size: u64,
    /// Is prefetchable (memory only)
    pub prefetchable: bool,
}

impl Bar {
    /// Decode BAR value
    pub fn decode(index: u8, value: u32, next_value: Option<u32>, size_value: u32) -> Option<Self> {
        if value == 0 && size_value == 0 {
            return None;
        }

        let is_io = value & 0x01 != 0;

        if is_io {
            let address = (value & 0xFFFFFFFC) as u64;
            let size = (!((size_value & 0xFFFFFFFC) | 0x03) + 1) as u64;

            if size == 0 {
                return None;
            }

            Some(Self {
                index,
                bar_type: BarType::Io,
                address,
                size,
                prefetchable: false,
            })
        } else {
            let memory_type = (value >> 1) & 0x03;
            let prefetchable = value & 0x08 != 0;

            match memory_type {
                0 => {
                    // 32-bit memory
                    let address = (value & 0xFFFFFFF0) as u64;
                    let size = (!((size_value & 0xFFFFFFF0) | 0x0F) + 1) as u64;

                    if size == 0 {
                        return None;
                    }

                    Some(Self {
                        index,
                        bar_type: BarType::Memory32,
                        address,
                        size,
                        prefetchable,
                    })
                }
                2 => {
                    // 64-bit memory
                    let next = next_value?;
                    let address = ((value & 0xFFFFFFF0) as u64) | ((next as u64) << 32);

                    // Size calculation for 64-bit BAR
                    let size_low = size_value & 0xFFFFFFF0;
                    let size = if size_low == 0 {
                        0 // 64-bit size not supported in this simple implementation
                    } else {
                        !((size_low | 0x0F) as u64) + 1
                    };

                    Some(Self {
                        index,
                        bar_type: BarType::Memory64,
                        address,
                        size,
                        prefetchable,
                    })
                }
                _ => None,
            }
        }
    }

    /// Is memory BAR
    pub fn is_memory(&self) -> bool {
        matches!(self.bar_type, BarType::Memory32 | BarType::Memory64)
    }

    /// Is I/O BAR
    pub fn is_io(&self) -> bool {
        matches!(self.bar_type, BarType::Io)
    }

    /// Is 64-bit
    pub fn is_64bit(&self) -> bool {
        matches!(self.bar_type, BarType::Memory64)
    }
}

// =============================================================================
// PCI CAPABILITY
// =============================================================================

/// PCI capability ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CapabilityId {
    /// Power Management
    PowerManagement = 0x01,
    /// AGP
    Agp = 0x02,
    /// VPD
    Vpd = 0x03,
    /// Slot Identification
    SlotId = 0x04,
    /// MSI
    Msi = 0x05,
    /// CompactPCI Hot Swap
    CpciHotSwap = 0x06,
    /// PCI-X
    PciX = 0x07,
    /// HyperTransport
    HyperTransport = 0x08,
    /// Vendor Specific
    VendorSpecific = 0x09,
    /// Debug port
    DebugPort = 0x0A,
    /// CompactPCI Resource Control
    CpciResourceCtrl = 0x0B,
    /// PCI Hot-Plug
    PciHotPlug = 0x0C,
    /// PCI Bridge Subsystem Vendor ID
    BridgeSubsystem = 0x0D,
    /// AGP 8x
    Agp8x = 0x0E,
    /// Secure Device
    SecureDevice = 0x0F,
    /// PCI Express
    PciExpress = 0x10,
    /// MSI-X
    MsiX = 0x11,
    /// SATA Configuration
    Sata = 0x12,
    /// Advanced Features
    AdvancedFeatures = 0x13,
    /// Enhanced Allocation
    EnhancedAllocation = 0x14,
    /// Flattening Portal Bridge
    FpBridge = 0x15,
}

impl CapabilityId {
    /// Try from u8
    pub fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::PowerManagement),
            0x02 => Some(Self::Agp),
            0x03 => Some(Self::Vpd),
            0x04 => Some(Self::SlotId),
            0x05 => Some(Self::Msi),
            0x06 => Some(Self::CpciHotSwap),
            0x07 => Some(Self::PciX),
            0x08 => Some(Self::HyperTransport),
            0x09 => Some(Self::VendorSpecific),
            0x0A => Some(Self::DebugPort),
            0x0B => Some(Self::CpciResourceCtrl),
            0x0C => Some(Self::PciHotPlug),
            0x0D => Some(Self::BridgeSubsystem),
            0x0E => Some(Self::Agp8x),
            0x0F => Some(Self::SecureDevice),
            0x10 => Some(Self::PciExpress),
            0x11 => Some(Self::MsiX),
            0x12 => Some(Self::Sata),
            0x13 => Some(Self::AdvancedFeatures),
            0x14 => Some(Self::EnhancedAllocation),
            0x15 => Some(Self::FpBridge),
            _ => None,
        }
    }
}

/// PCI capability
#[derive(Debug, Clone, Copy)]
pub struct Capability {
    /// Capability ID
    pub id: u8,
    /// Offset in configuration space
    pub offset: u8,
    /// Next capability pointer
    pub next: u8,
}

impl Capability {
    /// Parse from bytes at offset
    pub fn from_bytes(bytes: &[u8], offset: u8) -> Option<Self> {
        let off = offset as usize;
        if off + 2 > bytes.len() {
            return None;
        }

        Some(Self {
            id: bytes[off],
            offset,
            next: bytes[off + 1],
        })
    }

    /// Get capability ID enum
    pub fn capability_id(&self) -> Option<CapabilityId> {
        CapabilityId::try_from_u8(self.id)
    }

    /// Is MSI
    pub fn is_msi(&self) -> bool {
        self.id == CapabilityId::Msi as u8
    }

    /// Is MSI-X
    pub fn is_msix(&self) -> bool {
        self.id == CapabilityId::MsiX as u8
    }

    /// Is PCI Express
    pub fn is_pcie(&self) -> bool {
        self.id == CapabilityId::PciExpress as u8
    }
}

// =============================================================================
// PCI CLASS CODES
// =============================================================================

/// PCI class codes
pub mod class_code {
    pub const UNCLASSIFIED: u8 = 0x00;
    pub const MASS_STORAGE: u8 = 0x01;
    pub const NETWORK: u8 = 0x02;
    pub const DISPLAY: u8 = 0x03;
    pub const MULTIMEDIA: u8 = 0x04;
    pub const MEMORY: u8 = 0x05;
    pub const BRIDGE: u8 = 0x06;
    pub const SIMPLE_COMM: u8 = 0x07;
    pub const BASE_PERIPHERAL: u8 = 0x08;
    pub const INPUT_DEVICE: u8 = 0x09;
    pub const DOCKING_STATION: u8 = 0x0A;
    pub const PROCESSOR: u8 = 0x0B;
    pub const SERIAL_BUS: u8 = 0x0C;
    pub const WIRELESS: u8 = 0x0D;
    pub const INTELLIGENT_IO: u8 = 0x0E;
    pub const SATELLITE_COMM: u8 = 0x0F;
    pub const ENCRYPTION: u8 = 0x10;
    pub const SIGNAL_PROCESSING: u8 = 0x11;
    pub const PROCESSING_ACCEL: u8 = 0x12;
    pub const NON_ESSENTIAL: u8 = 0x13;
    pub const COPROCESSOR: u8 = 0x40;
    pub const UNASSIGNED: u8 = 0xFF;
}

/// Get class name
pub fn class_name(class: u8) -> &'static str {
    match class {
        class_code::UNCLASSIFIED => "Unclassified",
        class_code::MASS_STORAGE => "Mass Storage",
        class_code::NETWORK => "Network",
        class_code::DISPLAY => "Display",
        class_code::MULTIMEDIA => "Multimedia",
        class_code::MEMORY => "Memory",
        class_code::BRIDGE => "Bridge",
        class_code::SIMPLE_COMM => "Communication",
        class_code::BASE_PERIPHERAL => "Base Peripheral",
        class_code::INPUT_DEVICE => "Input Device",
        class_code::DOCKING_STATION => "Docking Station",
        class_code::PROCESSOR => "Processor",
        class_code::SERIAL_BUS => "Serial Bus",
        class_code::WIRELESS => "Wireless",
        class_code::INTELLIGENT_IO => "Intelligent I/O",
        class_code::SATELLITE_COMM => "Satellite",
        class_code::ENCRYPTION => "Encryption",
        class_code::SIGNAL_PROCESSING => "Signal Processing",
        class_code::PROCESSING_ACCEL => "Processing Accelerator",
        class_code::NON_ESSENTIAL => "Non-Essential",
        class_code::COPROCESSOR => "Co-Processor",
        _ => "Unknown",
    }
}

// =============================================================================
// PCI COMMAND REGISTER
// =============================================================================

/// PCI command register bits
pub mod command {
    pub const IO_SPACE: u16 = 0x0001;
    pub const MEMORY_SPACE: u16 = 0x0002;
    pub const BUS_MASTER: u16 = 0x0004;
    pub const SPECIAL_CYCLES: u16 = 0x0008;
    pub const MWI_ENABLE: u16 = 0x0010;
    pub const VGA_PALETTE_SNOOP: u16 = 0x0020;
    pub const PARITY_ERROR_RESPONSE: u16 = 0x0040;
    pub const SERR_ENABLE: u16 = 0x0100;
    pub const FAST_B2B_ENABLE: u16 = 0x0200;
    pub const INTERRUPT_DISABLE: u16 = 0x0400;
}

// =============================================================================
// PCI DEVICE INFO
// =============================================================================

/// PCI device information
#[derive(Debug, Clone)]
pub struct PciDeviceInfo {
    /// Address
    pub address: PciAddress,
    /// Vendor ID
    pub vendor_id: u16,
    /// Device ID
    pub device_id: u16,
    /// Class code
    pub class_code: u8,
    /// Subclass
    pub subclass: u8,
    /// Programming interface
    pub prog_if: u8,
    /// Revision
    pub revision: u8,
    /// Header type
    pub header_type: u8,
    /// Is multi-function
    pub multi_function: bool,
    /// Interrupt line
    pub interrupt_line: u8,
    /// Interrupt pin
    pub interrupt_pin: u8,
}

impl PciDeviceInfo {
    /// From header
    pub fn from_header(address: PciAddress, header: &PciConfigHeader) -> Self {
        Self {
            address,
            vendor_id: header.vendor_id,
            device_id: header.device_id,
            class_code: header.class_code,
            subclass: header.subclass,
            prog_if: header.prog_if,
            revision: header.revision_id,
            header_type: header.header_type_num(),
            multi_function: header.is_multi_function(),
            interrupt_line: 0,
            interrupt_pin: 0,
        }
    }

    /// Get class name
    pub fn class_name(&self) -> &'static str {
        class_name(self.class_code)
    }

    /// Is bridge
    pub fn is_bridge(&self) -> bool {
        self.class_code == class_code::BRIDGE
    }

    /// Is host bridge
    pub fn is_host_bridge(&self) -> bool {
        self.class_code == class_code::BRIDGE && self.subclass == 0x00
    }

    /// Is PCI-to-PCI bridge
    pub fn is_pci_bridge(&self) -> bool {
        self.class_code == class_code::BRIDGE && self.subclass == 0x04
    }
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// PCI error
#[derive(Debug, Clone)]
pub enum PciError {
    /// Device not found
    DeviceNotFound,
    /// Invalid address
    InvalidAddress,
    /// Access error
    AccessError,
    /// Capability not found
    CapabilityNotFound,
    /// Configuration error
    ConfigError,
}

impl fmt::Display for PciError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceNotFound => write!(f, "device not found"),
            Self::InvalidAddress => write!(f, "invalid address"),
            Self::AccessError => write!(f, "access error"),
            Self::CapabilityNotFound => write!(f, "capability not found"),
            Self::ConfigError => write!(f, "configuration error"),
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pci_address() {
        let addr = PciAddress::new(1, 2, 3);
        assert_eq!(addr.bus, 1);
        assert_eq!(addr.device, 2);
        assert_eq!(addr.function, 3);

        let bdf = addr.to_bdf();
        let addr2 = PciAddress::from_bdf(bdf);
        assert_eq!(addr, addr2);
    }

    #[test]
    fn test_pci_address_display() {
        let addr = PciAddress::new(0, 31, 0);
        assert_eq!(format!("{}", addr), "00:1f.0");

        let addr2 = PciAddress::with_segment(1, 0, 31, 0);
        assert_eq!(format!("{}", addr2), "0001:00:1f.0");
    }

    #[test]
    fn test_config_address() {
        let addr = PciAddress::new(0, 0, 0);
        let cfg = addr.config_address(0);
        assert_eq!(cfg, 0x80000000);
    }
}
