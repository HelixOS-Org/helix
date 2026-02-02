//! Driver Discovery and Management for Helix UEFI Bootloader
//!
//! This module provides comprehensive driver management including
//! device enumeration, driver binding, and device tree construction.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                     Driver Framework                                    │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Driver Registry                               │   │
//! │  │  Storage │ Network │ Graphics │ USB │ Input │ Serial            │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Device Enumeration                            │   │
//! │  │  PCI │ ACPI │ USB │ Platform │ Firmware                         │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Binding Protocol                              │   │
//! │  │  Supported │ Start │ Stop │ GetControllerName                   │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Device Tree                                   │   │
//! │  │  Root │ PCI Bus │ USB Hub │ Controller │ Device                 │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

#![no_std]

use core::fmt;

// =============================================================================
// DEVICE TYPES
// =============================================================================

/// Device classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    /// Unknown device
    Unknown,
    /// System device (CPU, memory controller)
    System,
    /// Storage controller
    Storage,
    /// Network controller
    Network,
    /// Display controller
    Display,
    /// Multimedia controller
    Multimedia,
    /// Memory controller
    Memory,
    /// Bridge device
    Bridge,
    /// Communication controller
    Communication,
    /// Input device
    Input,
    /// Docking station
    Docking,
    /// Processor
    Processor,
    /// Serial bus controller
    SerialBus,
    /// Wireless controller
    Wireless,
    /// Intelligent I/O
    IntelligentIO,
    /// Satellite communication
    Satellite,
    /// Encryption controller
    Encryption,
    /// Signal processing controller
    SignalProcessing,
}

/// Device subclass for storage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageSubclass {
    /// SCSI
    Scsi,
    /// IDE
    Ide,
    /// Floppy
    Floppy,
    /// IPI
    Ipi,
    /// RAID
    Raid,
    /// ATA
    Ata,
    /// SATA (AHCI)
    Sata,
    /// SAS
    Sas,
    /// NVMe
    Nvme,
    /// UFS
    Ufs,
    /// Other
    Other,
}

/// Device subclass for network
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkSubclass {
    /// Ethernet
    Ethernet,
    /// Token Ring
    TokenRing,
    /// FDDI
    Fddi,
    /// ATM
    Atm,
    /// ISDN
    Isdn,
    /// WorldFIP
    WorldFip,
    /// PICMG
    Picmg,
    /// InfiniBand
    InfiniBand,
    /// Fabric
    Fabric,
    /// Other
    Other,
}

/// Device subclass for display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplaySubclass {
    /// VGA compatible
    Vga,
    /// XGA
    Xga,
    /// 3D controller
    Controller3d,
    /// Other
    Other,
}

// =============================================================================
// PCI IDENTIFICATION
// =============================================================================

/// PCI device identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PciDeviceId {
    /// Vendor ID
    pub vendor_id: u16,
    /// Device ID
    pub device_id: u16,
    /// Subsystem vendor ID
    pub subsystem_vendor_id: u16,
    /// Subsystem device ID
    pub subsystem_device_id: u16,
    /// Revision ID
    pub revision_id: u8,
    /// Class code
    pub class_code: u8,
    /// Subclass code
    pub subclass_code: u8,
    /// Programming interface
    pub prog_if: u8,
}

/// Well-known vendor IDs
pub mod vendor_id {
    pub const INTEL: u16 = 0x8086;
    pub const AMD: u16 = 0x1022;
    pub const NVIDIA: u16 = 0x10DE;
    pub const QUALCOMM: u16 = 0x17CB;
    pub const BROADCOM: u16 = 0x14E4;
    pub const REALTEK: u16 = 0x10EC;
    pub const MARVELL: u16 = 0x1B4B;
    pub const SAMSUNG: u16 = 0x144D;
    pub const SANDISK: u16 = 0x15B7;
    pub const WESTERN_DIGITAL: u16 = 0x1B96;
    pub const SEAGATE: u16 = 0x1BB1;
    pub const MICRON: u16 = 0x1344;
    pub const TOSHIBA: u16 = 0x1179;
    pub const SK_HYNIX: u16 = 0x1C5C;
    pub const DELL: u16 = 0x1028;
    pub const HP: u16 = 0x103C;
    pub const LENOVO: u16 = 0x17AA;
    pub const ASUS: u16 = 0x1043;
    pub const MSI: u16 = 0x1462;
    pub const GIGABYTE: u16 = 0x1458;
    pub const VMWARE: u16 = 0x15AD;
    pub const QEMU: u16 = 0x1B36;
    pub const VIRTIO: u16 = 0x1AF4;
    pub const RED_HAT: u16 = 0x1B36;
    pub const AMAZON: u16 = 0x1D0F;
    pub const MICROSOFT: u16 = 0x1414;
    pub const APPLE: u16 = 0x106B;
}

/// PCI class codes
pub mod pci_class {
    pub const UNCLASSIFIED: u8 = 0x00;
    pub const MASS_STORAGE: u8 = 0x01;
    pub const NETWORK: u8 = 0x02;
    pub const DISPLAY: u8 = 0x03;
    pub const MULTIMEDIA: u8 = 0x04;
    pub const MEMORY: u8 = 0x05;
    pub const BRIDGE: u8 = 0x06;
    pub const SIMPLE_COMM: u8 = 0x07;
    pub const BASE_PERIPHERAL: u8 = 0x08;
    pub const INPUT: u8 = 0x09;
    pub const DOCKING: u8 = 0x0A;
    pub const PROCESSOR: u8 = 0x0B;
    pub const SERIAL_BUS: u8 = 0x0C;
    pub const WIRELESS: u8 = 0x0D;
    pub const INTELLIGENT_IO: u8 = 0x0E;
    pub const SATELLITE: u8 = 0x0F;
    pub const ENCRYPTION: u8 = 0x10;
    pub const SIGNAL_PROCESSING: u8 = 0x11;
    pub const PROCESSING_ACCEL: u8 = 0x12;
    pub const NON_ESSENTIAL: u8 = 0x13;
    pub const COPROCESSOR: u8 = 0x40;
    pub const UNASSIGNED: u8 = 0xFF;
}

/// PCI subclass codes for mass storage
pub mod pci_storage_subclass {
    pub const SCSI: u8 = 0x00;
    pub const IDE: u8 = 0x01;
    pub const FLOPPY: u8 = 0x02;
    pub const IPI: u8 = 0x03;
    pub const RAID: u8 = 0x04;
    pub const ATA: u8 = 0x05;
    pub const SATA: u8 = 0x06;
    pub const SAS: u8 = 0x07;
    pub const NVME: u8 = 0x08;
    pub const UFS: u8 = 0x09;
    pub const OTHER: u8 = 0x80;
}

/// PCI subclass codes for network
pub mod pci_network_subclass {
    pub const ETHERNET: u8 = 0x00;
    pub const TOKEN_RING: u8 = 0x01;
    pub const FDDI: u8 = 0x02;
    pub const ATM: u8 = 0x03;
    pub const ISDN: u8 = 0x04;
    pub const WORLDFIP: u8 = 0x05;
    pub const PICMG: u8 = 0x06;
    pub const INFINIBAND: u8 = 0x07;
    pub const FABRIC: u8 = 0x08;
    pub const OTHER: u8 = 0x80;
}

/// PCI subclass codes for display
pub mod pci_display_subclass {
    pub const VGA: u8 = 0x00;
    pub const XGA: u8 = 0x01;
    pub const CONTROLLER_3D: u8 = 0x02;
    pub const OTHER: u8 = 0x80;
}

/// PCI subclass codes for serial bus
pub mod pci_serial_subclass {
    pub const FIREWIRE: u8 = 0x00;
    pub const ACCESS_BUS: u8 = 0x01;
    pub const SSA: u8 = 0x02;
    pub const USB: u8 = 0x03;
    pub const FIBRE_CHANNEL: u8 = 0x04;
    pub const SMBUS: u8 = 0x05;
    pub const INFINIBAND: u8 = 0x06;
    pub const IPMI: u8 = 0x07;
    pub const SERCOS: u8 = 0x08;
    pub const CANBUS: u8 = 0x09;
    pub const OTHER: u8 = 0x80;
}

/// USB programming interface
pub mod usb_prog_if {
    pub const UHCI: u8 = 0x00;
    pub const OHCI: u8 = 0x10;
    pub const EHCI: u8 = 0x20;
    pub const XHCI: u8 = 0x30;
    pub const USB4_HOST: u8 = 0x40;
}

// =============================================================================
// PCI LOCATION
// =============================================================================

/// PCI device location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PciLocation {
    /// Segment (for PCIe)
    pub segment: u16,
    /// Bus number
    pub bus: u8,
    /// Device number
    pub device: u8,
    /// Function number
    pub function: u8,
}

impl PciLocation {
    /// Create new location
    pub const fn new(segment: u16, bus: u8, device: u8, function: u8) -> Self {
        Self {
            segment,
            bus,
            device,
            function,
        }
    }

    /// Create from BDF
    pub const fn from_bdf(bus: u8, device: u8, function: u8) -> Self {
        Self {
            segment: 0,
            bus,
            device,
            function,
        }
    }

    /// Convert to config address
    pub const fn config_address(&self) -> u32 {
        (1 << 31)
            | ((self.bus as u32) << 16)
            | ((self.device as u32) << 11)
            | ((self.function as u32) << 8)
    }
}

impl fmt::Display for PciLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04x}:{:02x}:{:02x}.{}",
            self.segment, self.bus, self.device, self.function
        )
    }
}

// =============================================================================
// DRIVER STATUS
// =============================================================================

/// Driver binding status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverStatus {
    /// Driver not loaded
    NotLoaded,
    /// Driver loaded but not started
    Loaded,
    /// Driver started successfully
    Started,
    /// Driver stopping
    Stopping,
    /// Driver failed to start
    Failed,
    /// Driver disabled
    Disabled,
}

/// Device status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatus {
    /// Device present but not configured
    Present,
    /// Device configured and working
    Working,
    /// Device has problem
    Problem,
    /// Device disabled
    Disabled,
    /// Device removed
    Removed,
    /// Device moved
    Moved,
}

// =============================================================================
// DRIVER INFORMATION
// =============================================================================

/// Driver version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverVersion {
    /// Major version
    pub major: u16,
    /// Minor version
    pub minor: u16,
    /// Patch version
    pub patch: u16,
    /// Build number
    pub build: u16,
}

impl DriverVersion {
    /// Create new version
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
            build: 0,
        }
    }
}

impl fmt::Display for DriverVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Driver type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverType {
    /// Bus driver
    Bus,
    /// Device driver
    Device,
    /// Filter driver
    Filter,
    /// Service driver
    Service,
    /// Platform driver
    Platform,
}

/// Driver flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverFlags(u32);

impl DriverFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Driver supports hot-plug
    pub const HOT_PLUG: Self = Self(1 << 0);
    /// Driver supports power management
    pub const POWER_MGMT: Self = Self(1 << 1);
    /// Driver is critical (cannot be unloaded)
    pub const CRITICAL: Self = Self(1 << 2);
    /// Driver loaded from firmware
    pub const FIRMWARE: Self = Self(1 << 3);
    /// Driver is built-in
    pub const BUILTIN: Self = Self(1 << 4);
    /// Driver supports DMA
    pub const DMA: Self = Self(1 << 5);
    /// Driver supports IOMMU
    pub const IOMMU: Self = Self(1 << 6);
    /// Driver supports MSI
    pub const MSI: Self = Self(1 << 7);
    /// Driver supports MSI-X
    pub const MSIX: Self = Self(1 << 8);
    /// Driver supports SR-IOV
    pub const SRIOV: Self = Self(1 << 9);

    /// Check if flag is set
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Combine flags
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// =============================================================================
// DRIVER DESCRIPTOR
// =============================================================================

/// Driver descriptor
#[derive(Debug, Clone)]
pub struct DriverDescriptor {
    /// Driver name
    pub name: [u8; 64],
    /// Driver name length
    pub name_len: usize,
    /// Driver version
    pub version: DriverVersion,
    /// Driver type
    pub driver_type: DriverType,
    /// Driver flags
    pub flags: DriverFlags,
    /// Device class handled
    pub device_class: DeviceClass,
    /// Supported vendor IDs (0 = any)
    pub vendor_ids: [u16; 8],
    /// Number of vendor IDs
    pub vendor_count: usize,
}

impl DriverDescriptor {
    /// Create new descriptor
    pub const fn new() -> Self {
        Self {
            name: [0u8; 64],
            name_len: 0,
            version: DriverVersion::new(1, 0, 0),
            driver_type: DriverType::Device,
            flags: DriverFlags::NONE,
            device_class: DeviceClass::Unknown,
            vendor_ids: [0u16; 8],
            vendor_count: 0,
        }
    }
}

impl Default for DriverDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// DEVICE NODE
// =============================================================================

/// Device node type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceNodeType {
    /// Hardware device
    Hardware,
    /// ACPI device
    Acpi,
    /// End of device path
    End,
    /// Firmware device
    Firmware,
    /// Virtual device
    Virtual,
}

/// Device node
#[derive(Debug, Clone)]
pub struct DeviceNode {
    /// Node type
    pub node_type: DeviceNodeType,
    /// Node subtype
    pub subtype: u8,
    /// Node data
    pub data: [u8; 128],
    /// Data length
    pub data_len: usize,
}

impl DeviceNode {
    /// Create new node
    pub const fn new(node_type: DeviceNodeType) -> Self {
        Self {
            node_type,
            subtype: 0,
            data: [0u8; 128],
            data_len: 0,
        }
    }
}

// =============================================================================
// DEVICE TREE
// =============================================================================

/// Device relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceRelation {
    /// Parent of
    Parent,
    /// Child of
    Child,
    /// Sibling
    Sibling,
    /// Dependency
    Dependency,
}

/// Device capabilities
#[derive(Debug, Clone, Copy, Default)]
pub struct DeviceCapabilities {
    /// Device can be removed
    pub removable: bool,
    /// Device is ejectable
    pub ejectable: bool,
    /// Device has unique ID
    pub unique_id: bool,
    /// Device is docking
    pub docking: bool,
    /// Device is silent install
    pub silent_install: bool,
    /// Device supports power management
    pub power_managed: bool,
    /// Device supports wake
    pub wake_capable: bool,
    /// Device is virtual
    pub virtual_device: bool,
}

/// Resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    /// Memory region
    Memory,
    /// I/O port
    IoPort,
    /// IRQ
    Irq,
    /// DMA channel
    Dma,
    /// Bus number
    BusNumber,
}

/// Resource descriptor
#[derive(Debug, Clone, Copy)]
pub struct ResourceDescriptor {
    /// Resource type
    pub res_type: ResourceType,
    /// Start address/number
    pub start: u64,
    /// Length/count
    pub length: u64,
    /// Flags
    pub flags: u32,
}

impl ResourceDescriptor {
    /// Create memory resource
    pub const fn memory(start: u64, length: u64) -> Self {
        Self {
            res_type: ResourceType::Memory,
            start,
            length,
            flags: 0,
        }
    }

    /// Create I/O resource
    pub const fn io_port(start: u16, length: u16) -> Self {
        Self {
            res_type: ResourceType::IoPort,
            start: start as u64,
            length: length as u64,
            flags: 0,
        }
    }

    /// Create IRQ resource
    pub const fn irq(irq: u32) -> Self {
        Self {
            res_type: ResourceType::Irq,
            start: irq as u64,
            length: 1,
            flags: 0,
        }
    }
}

// =============================================================================
// ACPI DEVICE
// =============================================================================

/// ACPI device information
#[derive(Debug, Clone)]
pub struct AcpiDeviceInfo {
    /// Hardware ID
    pub hid: [u8; 16],
    /// HID length
    pub hid_len: usize,
    /// Unique ID
    pub uid: u32,
    /// Compatible IDs
    pub cids: [[u8; 16]; 4],
    /// CID count
    pub cid_count: usize,
    /// Address (if applicable)
    pub address: u64,
    /// Status bits
    pub status: u32,
}

impl AcpiDeviceInfo {
    /// Status bit: Present
    pub const STA_PRESENT: u32 = 1 << 0;
    /// Status bit: Enabled
    pub const STA_ENABLED: u32 = 1 << 1;
    /// Status bit: Show in UI
    pub const STA_SHOW_UI: u32 = 1 << 2;
    /// Status bit: Functioning
    pub const STA_FUNCTIONING: u32 = 1 << 3;
    /// Status bit: Battery present
    pub const STA_BATTERY: u32 = 1 << 4;

    /// Check if present
    pub const fn is_present(&self) -> bool {
        (self.status & Self::STA_PRESENT) != 0
    }

    /// Check if enabled
    pub const fn is_enabled(&self) -> bool {
        (self.status & Self::STA_ENABLED) != 0
    }
}

/// Well-known ACPI device IDs
pub mod acpi_hid {
    /// PCI Root Bridge
    pub const PNP0A03: &[u8] = b"PNP0A03";
    /// PCIe Root Bridge
    pub const PNP0A08: &[u8] = b"PNP0A08";
    /// System Timer
    pub const PNP0100: &[u8] = b"PNP0100";
    /// DMA Controller
    pub const PNP0200: &[u8] = b"PNP0200";
    /// System Speaker
    pub const PNP0800: &[u8] = b"PNP0800";
    /// Keyboard Controller
    pub const PNP0303: &[u8] = b"PNP0303";
    /// PS/2 Mouse
    pub const PNP0F13: &[u8] = b"PNP0F13";
    /// Serial Port
    pub const PNP0501: &[u8] = b"PNP0501";
    /// RTC
    pub const PNP0B00: &[u8] = b"PNP0B00";
    /// ACPI Button
    pub const PNP0C0C: &[u8] = b"PNP0C0C";
    /// ACPI Lid
    pub const PNP0C0D: &[u8] = b"PNP0C0D";
    /// ACPI Sleep Button
    pub const PNP0C0E: &[u8] = b"PNP0C0E";
    /// Processor
    pub const ACPI0007: &[u8] = b"ACPI0007";
    /// Thermal Zone
    pub const ACPI0005: &[u8] = b"ACPI0005";
    /// Fan
    pub const PNP0C0B: &[u8] = b"PNP0C0B";
    /// Generic Container
    pub const PNP0A05: &[u8] = b"PNP0A05";
    /// EC (Embedded Controller)
    pub const PNP0C09: &[u8] = b"PNP0C09";
    /// Battery
    pub const PNP0C0A: &[u8] = b"PNP0C0A";
    /// AC Adapter
    pub const ACPI0003: &[u8] = b"ACPI0003";
    /// TPM 2.0
    pub const MSFT0101: &[u8] = b"MSFT0101";
}

// =============================================================================
// USB DEVICE
// =============================================================================

/// USB device descriptor
#[derive(Debug, Clone, Copy, Default)]
pub struct UsbDeviceDescriptor {
    /// USB specification version
    pub usb_version: u16,
    /// Device class
    pub device_class: u8,
    /// Device subclass
    pub device_subclass: u8,
    /// Device protocol
    pub device_protocol: u8,
    /// Max packet size for endpoint 0
    pub max_packet_size: u8,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
    /// Device version
    pub device_version: u16,
    /// Manufacturer string index
    pub manufacturer_index: u8,
    /// Product string index
    pub product_index: u8,
    /// Serial number string index
    pub serial_index: u8,
    /// Number of configurations
    pub num_configurations: u8,
}

/// USB device class
pub mod usb_class {
    pub const INTERFACE: u8 = 0x00;
    pub const AUDIO: u8 = 0x01;
    pub const CDC: u8 = 0x02;
    pub const HID: u8 = 0x03;
    pub const PHYSICAL: u8 = 0x05;
    pub const IMAGE: u8 = 0x06;
    pub const PRINTER: u8 = 0x07;
    pub const MASS_STORAGE: u8 = 0x08;
    pub const HUB: u8 = 0x09;
    pub const CDC_DATA: u8 = 0x0A;
    pub const SMART_CARD: u8 = 0x0B;
    pub const VIDEO: u8 = 0x0E;
    pub const PERSONAL_HEALTHCARE: u8 = 0x0F;
    pub const AUDIO_VIDEO: u8 = 0x10;
    pub const BILLBOARD: u8 = 0x11;
    pub const USB_C_BRIDGE: u8 = 0x12;
    pub const DIAGNOSTIC: u8 = 0xDC;
    pub const WIRELESS: u8 = 0xE0;
    pub const MISC: u8 = 0xEF;
    pub const APPLICATION: u8 = 0xFE;
    pub const VENDOR: u8 = 0xFF;
}

/// USB speed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbSpeed {
    /// Low speed (1.5 Mbps)
    Low,
    /// Full speed (12 Mbps)
    Full,
    /// High speed (480 Mbps)
    High,
    /// Super speed (5 Gbps)
    Super,
    /// Super speed+ (10 Gbps)
    SuperPlus,
    /// Super speed+ (20 Gbps)
    SuperPlus20,
}

impl UsbSpeed {
    /// Get speed in Mbps
    pub const fn mbps(&self) -> u32 {
        match self {
            UsbSpeed::Low => 1,
            UsbSpeed::Full => 12,
            UsbSpeed::High => 480,
            UsbSpeed::Super => 5000,
            UsbSpeed::SuperPlus => 10000,
            UsbSpeed::SuperPlus20 => 20000,
        }
    }
}

// =============================================================================
// ENUMERATION
// =============================================================================

/// Enumeration phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumerationPhase {
    /// Initial discovery
    Discovery,
    /// Reading device descriptors
    ReadDescriptors,
    /// Finding driver
    DriverMatch,
    /// Starting driver
    DriverStart,
    /// Device configured
    Configured,
}

/// Enumeration result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumerationResult {
    /// Device enumerated successfully
    Success,
    /// Device not supported
    NotSupported,
    /// No driver found
    NoDriver,
    /// Driver failed to start
    DriverFailed,
    /// Communication error
    CommError,
    /// Device removed during enumeration
    Removed,
}

// =============================================================================
// INTERRUPT ROUTING
// =============================================================================

/// Interrupt type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptType {
    /// Legacy IRQ
    Legacy,
    /// MSI
    Msi,
    /// MSI-X
    MsiX,
    /// IPI
    Ipi,
}

/// Interrupt routing entry
#[derive(Debug, Clone, Copy)]
pub struct InterruptRoute {
    /// Device address
    pub device_address: u32,
    /// Pin (INTA=0, INTB=1, etc.)
    pub pin: u8,
    /// Source (GSI or link)
    pub source: u32,
    /// Source index
    pub source_index: u32,
}

/// MSI capability
#[derive(Debug, Clone, Copy)]
pub struct MsiCapability {
    /// Message control
    pub control: u16,
    /// Message address
    pub address: u64,
    /// Message data
    pub data: u16,
    /// Mask bits (if supported)
    pub mask: u32,
    /// Pending bits (if supported)
    pub pending: u32,
}

impl MsiCapability {
    /// Maximum vectors supported (2^N)
    pub const fn max_vectors(&self) -> u8 {
        1 << ((self.control >> 1) & 0x07)
    }

    /// Check if 64-bit capable
    pub const fn is_64bit(&self) -> bool {
        (self.control & (1 << 7)) != 0
    }

    /// Check if per-vector masking supported
    pub const fn has_mask(&self) -> bool {
        (self.control & (1 << 8)) != 0
    }
}

// =============================================================================
// POWER MANAGEMENT
// =============================================================================

/// Device power state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevicePowerState {
    /// Full power (D0)
    D0,
    /// Low power, context preserved (D1)
    D1,
    /// Low power, partial context (D2)
    D2,
    /// Off (D3Hot - power present)
    D3Hot,
    /// Off (D3Cold - no power)
    D3Cold,
    /// Unknown
    Unknown,
}

impl DevicePowerState {
    /// Get power consumption level (0=lowest, 4=highest)
    pub const fn power_level(&self) -> u8 {
        match self {
            DevicePowerState::D3Cold => 0,
            DevicePowerState::D3Hot => 1,
            DevicePowerState::D2 => 2,
            DevicePowerState::D1 => 3,
            DevicePowerState::D0 => 4,
            DevicePowerState::Unknown => 4,
        }
    }
}

/// Power capabilities
#[derive(Debug, Clone, Copy, Default)]
pub struct PowerCapabilities {
    /// Device supports D1
    pub d1_supported: bool,
    /// Device supports D2
    pub d2_supported: bool,
    /// Device supports PME from D0
    pub pme_d0: bool,
    /// Device supports PME from D1
    pub pme_d1: bool,
    /// Device supports PME from D2
    pub pme_d2: bool,
    /// Device supports PME from D3Hot
    pub pme_d3hot: bool,
    /// Device supports PME from D3Cold
    pub pme_d3cold: bool,
    /// Device supports wake
    pub wake_capable: bool,
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// Driver error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverError {
    /// Driver not found
    NotFound,
    /// Device already has driver
    AlreadyBound,
    /// Driver doesn't support device
    NotSupported,
    /// Driver failed to start
    StartFailed,
    /// Driver failed to stop
    StopFailed,
    /// Resource conflict
    ResourceConflict,
    /// Out of resources
    OutOfResources,
    /// Invalid parameter
    InvalidParameter,
    /// Communication error
    CommError,
    /// Timeout
    Timeout,
    /// Device removed
    DeviceRemoved,
    /// Access denied
    AccessDenied,
}

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriverError::NotFound => write!(f, "Driver not found"),
            DriverError::AlreadyBound => write!(f, "Device already bound"),
            DriverError::NotSupported => write!(f, "Not supported"),
            DriverError::StartFailed => write!(f, "Start failed"),
            DriverError::StopFailed => write!(f, "Stop failed"),
            DriverError::ResourceConflict => write!(f, "Resource conflict"),
            DriverError::OutOfResources => write!(f, "Out of resources"),
            DriverError::InvalidParameter => write!(f, "Invalid parameter"),
            DriverError::CommError => write!(f, "Communication error"),
            DriverError::Timeout => write!(f, "Timeout"),
            DriverError::DeviceRemoved => write!(f, "Device removed"),
            DriverError::AccessDenied => write!(f, "Access denied"),
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
    fn test_pci_location() {
        let loc = PciLocation::from_bdf(0, 2, 0);
        assert_eq!(loc.bus, 0);
        assert_eq!(loc.device, 2);
        assert_eq!(loc.function, 0);
    }

    #[test]
    fn test_driver_version() {
        let ver = DriverVersion::new(1, 2, 3);
        assert_eq!(ver.major, 1);
        assert_eq!(ver.minor, 2);
        assert_eq!(ver.patch, 3);
    }

    #[test]
    fn test_driver_flags() {
        let flags = DriverFlags::HOT_PLUG.union(DriverFlags::POWER_MGMT);
        assert!(flags.contains(DriverFlags::HOT_PLUG));
        assert!(flags.contains(DriverFlags::POWER_MGMT));
        assert!(!flags.contains(DriverFlags::CRITICAL));
    }

    #[test]
    fn test_usb_speed() {
        assert_eq!(UsbSpeed::High.mbps(), 480);
        assert_eq!(UsbSpeed::Super.mbps(), 5000);
    }

    #[test]
    fn test_device_power_state() {
        assert_eq!(DevicePowerState::D0.power_level(), 4);
        assert_eq!(DevicePowerState::D3Cold.power_level(), 0);
    }
}
