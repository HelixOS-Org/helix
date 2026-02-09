//! Core PCI types and identifiers.

// ============================================================================
// CORE TYPES
// ============================================================================

/// PCI device ID (BDF - Bus:Device.Function)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PciDeviceId {
    /// Segment (domain)
    pub segment: u16,
    /// Bus number
    pub bus: u8,
    /// Device number
    pub device: u8,
    /// Function number
    pub function: u8,
}

impl PciDeviceId {
    /// Create new device ID
    #[inline]
    pub const fn new(segment: u16, bus: u8, device: u8, function: u8) -> Self {
        Self {
            segment,
            bus,
            device,
            function,
        }
    }

    /// Create from BDF
    #[inline(always)]
    pub const fn from_bdf(bus: u8, device: u8, function: u8) -> Self {
        Self::new(0, bus, device, function)
    }

    /// Get BDF as u16
    #[inline(always)]
    pub fn bdf(&self) -> u16 {
        ((self.bus as u16) << 8) | ((self.device as u16) << 3) | (self.function as u16)
    }
}

impl core::fmt::Display for PciDeviceId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:04x}:{:02x}:{:02x}.{}",
            self.segment, self.bus, self.device, self.function
        )
    }
}

/// Vendor ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VendorId(pub u16);

/// Device ID (product ID)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProductId(pub u16);

/// Class code
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClassCode {
    /// Base class
    pub class: u8,
    /// Subclass
    pub subclass: u8,
    /// Programming interface
    pub prog_if: u8,
}

impl ClassCode {
    /// Create new class code
    #[inline]
    pub const fn new(class: u8, subclass: u8, prog_if: u8) -> Self {
        Self {
            class,
            subclass,
            prog_if,
        }
    }

    /// Get full class code as u32
    #[inline(always)]
    pub fn as_u32(&self) -> u32 {
        ((self.class as u32) << 16) | ((self.subclass as u32) << 8) | (self.prog_if as u32)
    }
}

/// Well-known vendor IDs
pub mod vendors {
    use super::VendorId;

    pub const INTEL: VendorId = VendorId(0x8086);
    pub const AMD: VendorId = VendorId(0x1022);
    pub const NVIDIA: VendorId = VendorId(0x10de);
    pub const QUALCOMM: VendorId = VendorId(0x17cb);
    pub const BROADCOM: VendorId = VendorId(0x14e4);
    pub const REALTEK: VendorId = VendorId(0x10ec);
    pub const SAMSUNG: VendorId = VendorId(0x144d);
    pub const MARVELL: VendorId = VendorId(0x1b4b);
    pub const QEMU: VendorId = VendorId(0x1234);
    pub const RED_HAT: VendorId = VendorId(0x1af4);
    pub const VMWARE: VendorId = VendorId(0x15ad);
}

/// Well-known class codes
pub mod classes {
    use super::ClassCode;

    // Class 0x00: Unclassified
    pub const UNCLASSIFIED: ClassCode = ClassCode::new(0x00, 0x00, 0x00);

    // Class 0x01: Mass storage
    pub const SCSI: ClassCode = ClassCode::new(0x01, 0x00, 0x00);
    pub const IDE: ClassCode = ClassCode::new(0x01, 0x01, 0x00);
    pub const FLOPPY: ClassCode = ClassCode::new(0x01, 0x02, 0x00);
    pub const RAID: ClassCode = ClassCode::new(0x01, 0x04, 0x00);
    pub const ATA: ClassCode = ClassCode::new(0x01, 0x05, 0x00);
    pub const SATA: ClassCode = ClassCode::new(0x01, 0x06, 0x00);
    pub const SAS: ClassCode = ClassCode::new(0x01, 0x07, 0x00);
    pub const NVME: ClassCode = ClassCode::new(0x01, 0x08, 0x02);

    // Class 0x02: Network
    pub const ETHERNET: ClassCode = ClassCode::new(0x02, 0x00, 0x00);
    pub const TOKEN_RING: ClassCode = ClassCode::new(0x02, 0x01, 0x00);
    pub const FDDI: ClassCode = ClassCode::new(0x02, 0x02, 0x00);
    pub const ATM: ClassCode = ClassCode::new(0x02, 0x03, 0x00);
    pub const ISDN: ClassCode = ClassCode::new(0x02, 0x04, 0x00);
    pub const WIFI: ClassCode = ClassCode::new(0x02, 0x80, 0x00);

    // Class 0x03: Display
    pub const VGA: ClassCode = ClassCode::new(0x03, 0x00, 0x00);
    pub const XGA: ClassCode = ClassCode::new(0x03, 0x01, 0x00);
    pub const GPU_3D: ClassCode = ClassCode::new(0x03, 0x02, 0x00);

    // Class 0x04: Multimedia
    pub const AUDIO: ClassCode = ClassCode::new(0x04, 0x01, 0x00);
    pub const HD_AUDIO: ClassCode = ClassCode::new(0x04, 0x03, 0x00);

    // Class 0x05: Memory
    pub const RAM: ClassCode = ClassCode::new(0x05, 0x00, 0x00);
    pub const FLASH: ClassCode = ClassCode::new(0x05, 0x01, 0x00);

    // Class 0x06: Bridge
    pub const HOST_BRIDGE: ClassCode = ClassCode::new(0x06, 0x00, 0x00);
    pub const ISA_BRIDGE: ClassCode = ClassCode::new(0x06, 0x01, 0x00);
    pub const PCI_BRIDGE: ClassCode = ClassCode::new(0x06, 0x04, 0x00);
    pub const PCIE_BRIDGE: ClassCode = ClassCode::new(0x06, 0x04, 0x01);
    pub const CARDBUS_BRIDGE: ClassCode = ClassCode::new(0x06, 0x07, 0x00);

    // Class 0x07: Communication
    pub const SERIAL: ClassCode = ClassCode::new(0x07, 0x00, 0x00);
    pub const PARALLEL: ClassCode = ClassCode::new(0x07, 0x01, 0x00);

    // Class 0x08: System peripheral
    pub const PIC: ClassCode = ClassCode::new(0x08, 0x00, 0x00);
    pub const DMA: ClassCode = ClassCode::new(0x08, 0x01, 0x00);
    pub const TIMER: ClassCode = ClassCode::new(0x08, 0x02, 0x00);
    pub const RTC: ClassCode = ClassCode::new(0x08, 0x03, 0x00);

    // Class 0x0C: Serial bus
    pub const FIREWIRE: ClassCode = ClassCode::new(0x0c, 0x00, 0x00);
    pub const USB_UHCI: ClassCode = ClassCode::new(0x0c, 0x03, 0x00);
    pub const USB_OHCI: ClassCode = ClassCode::new(0x0c, 0x03, 0x10);
    pub const USB_EHCI: ClassCode = ClassCode::new(0x0c, 0x03, 0x20);
    pub const USB_XHCI: ClassCode = ClassCode::new(0x0c, 0x03, 0x30);
    pub const SMBUS: ClassCode = ClassCode::new(0x0c, 0x05, 0x00);
}

/// PCI device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciDeviceType {
    /// Endpoint (Type 0)
    Endpoint,
    /// PCI-to-PCI bridge (Type 1)
    PciBridge,
    /// CardBus bridge (Type 2)
    CardBusBridge,
    /// Unknown
    Unknown,
}

impl PciDeviceType {
    /// Get type name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Endpoint => "endpoint",
            Self::PciBridge => "pci-bridge",
            Self::CardBusBridge => "cardbus-bridge",
            Self::Unknown => "unknown",
        }
    }

    /// From header type
    #[inline]
    pub fn from_header_type(header_type: u8) -> Self {
        match header_type & 0x7f {
            0x00 => Self::Endpoint,
            0x01 => Self::PciBridge,
            0x02 => Self::CardBusBridge,
            _ => Self::Unknown,
        }
    }

    /// Is bridge
    #[inline(always)]
    pub fn is_bridge(&self) -> bool {
        matches!(self, Self::PciBridge | Self::CardBusBridge)
    }
}
