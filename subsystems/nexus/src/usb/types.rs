//! USB Core Types
//!
//! Fundamental types for USB subsystem management.

/// USB bus ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BusId(pub u8);

impl BusId {
    /// Create new bus ID
    #[inline(always)]
    pub const fn new(id: u8) -> Self {
        Self(id)
    }
}

/// USB device address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceAddress(pub u8);

impl DeviceAddress {
    /// Create new address
    #[inline(always)]
    pub const fn new(addr: u8) -> Self {
        Self(addr)
    }
}

/// USB device ID (bus + address)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UsbDeviceId {
    /// Bus
    pub bus: BusId,
    /// Address
    pub address: DeviceAddress,
}

impl UsbDeviceId {
    /// Create new device ID
    #[inline]
    pub const fn new(bus: u8, address: u8) -> Self {
        Self {
            bus: BusId::new(bus),
            address: DeviceAddress::new(address),
        }
    }
}

impl core::fmt::Display for UsbDeviceId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.bus.0, self.address.0)
    }
}

/// USB vendor ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UsbVendorId(pub u16);

/// USB product ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UsbProductId(pub u16);

/// Well-known vendor IDs
pub mod vendors {
    use super::UsbVendorId;

    pub const LINUX_FOUNDATION: UsbVendorId = UsbVendorId(0x1d6b);
    pub const APPLE: UsbVendorId = UsbVendorId(0x05ac);
    pub const MICROSOFT: UsbVendorId = UsbVendorId(0x045e);
    pub const LOGITECH: UsbVendorId = UsbVendorId(0x046d);
    pub const SANDISK: UsbVendorId = UsbVendorId(0x0781);
    pub const KINGSTON: UsbVendorId = UsbVendorId(0x0951);
    pub const SAMSUNG: UsbVendorId = UsbVendorId(0x04e8);
    pub const SEAGATE: UsbVendorId = UsbVendorId(0x0bc2);
    pub const WESTERN_DIGITAL: UsbVendorId = UsbVendorId(0x1058);
    pub const INTEL: UsbVendorId = UsbVendorId(0x8087);
    pub const REALTEK: UsbVendorId = UsbVendorId(0x0bda);
    pub const RALINK: UsbVendorId = UsbVendorId(0x148f);
}

/// USB speed
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UsbSpeed {
    /// Low speed (1.5 Mbit/s)
    Low,
    /// Full speed (12 Mbit/s)
    Full,
    /// High speed (480 Mbit/s, USB 2.0)
    High,
    /// Super speed (5 Gbit/s, USB 3.0)
    Super,
    /// Super speed plus (10 Gbit/s, USB 3.1 Gen 2)
    SuperPlus,
    /// Super speed plus x2 (20 Gbit/s, USB 3.2 Gen 2x2)
    SuperPlusX2,
    /// USB4 (40 Gbit/s)
    Usb4,
    /// Unknown
    Unknown,
}

impl UsbSpeed {
    /// Get speed name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Low => "1.5 Mbit/s",
            Self::Full => "12 Mbit/s",
            Self::High => "480 Mbit/s",
            Self::Super => "5 Gbit/s",
            Self::SuperPlus => "10 Gbit/s",
            Self::SuperPlusX2 => "20 Gbit/s",
            Self::Usb4 => "40 Gbit/s",
            Self::Unknown => "unknown",
        }
    }

    /// Get USB version string
    pub fn usb_version(&self) -> &'static str {
        match self {
            Self::Low => "USB 1.0",
            Self::Full => "USB 1.1",
            Self::High => "USB 2.0",
            Self::Super => "USB 3.0",
            Self::SuperPlus => "USB 3.1",
            Self::SuperPlusX2 => "USB 3.2",
            Self::Usb4 => "USB4",
            Self::Unknown => "unknown",
        }
    }

    /// Get bandwidth in bytes/sec
    pub fn bandwidth(&self) -> u64 {
        match self {
            Self::Low => 187_500,
            Self::Full => 1_500_000,
            Self::High => 60_000_000,
            Self::Super => 625_000_000,
            Self::SuperPlus => 1_250_000_000,
            Self::SuperPlusX2 => 2_500_000_000,
            Self::Usb4 => 5_000_000_000,
            Self::Unknown => 0,
        }
    }
}

/// USB device class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbClass {
    /// Interface-specific
    InterfaceSpecific,
    /// Audio
    Audio,
    /// CDC (Communications)
    Cdc,
    /// HID (Human Interface Device)
    Hid,
    /// Physical
    Physical,
    /// Image
    Image,
    /// Printer
    Printer,
    /// Mass Storage
    MassStorage,
    /// Hub
    Hub,
    /// CDC Data
    CdcData,
    /// Smart Card
    SmartCard,
    /// Content Security
    ContentSecurity,
    /// Video
    Video,
    /// Personal Healthcare
    PersonalHealthcare,
    /// Audio/Video
    AudioVideo,
    /// Billboard
    Billboard,
    /// USB-C Bridge
    UsbCBridge,
    /// Diagnostic
    Diagnostic,
    /// Wireless
    Wireless,
    /// Miscellaneous
    Miscellaneous,
    /// Application Specific
    ApplicationSpecific,
    /// Vendor Specific
    VendorSpecific,
    /// Unknown
    Unknown,
}

impl UsbClass {
    /// From class code
    pub fn from_code(code: u8) -> Self {
        match code {
            0x00 => Self::InterfaceSpecific,
            0x01 => Self::Audio,
            0x02 => Self::Cdc,
            0x03 => Self::Hid,
            0x05 => Self::Physical,
            0x06 => Self::Image,
            0x07 => Self::Printer,
            0x08 => Self::MassStorage,
            0x09 => Self::Hub,
            0x0a => Self::CdcData,
            0x0b => Self::SmartCard,
            0x0d => Self::ContentSecurity,
            0x0e => Self::Video,
            0x0f => Self::PersonalHealthcare,
            0x10 => Self::AudioVideo,
            0x11 => Self::Billboard,
            0x12 => Self::UsbCBridge,
            0xdc => Self::Diagnostic,
            0xe0 => Self::Wireless,
            0xef => Self::Miscellaneous,
            0xfe => Self::ApplicationSpecific,
            0xff => Self::VendorSpecific,
            _ => Self::Unknown,
        }
    }

    /// Get class name
    pub fn name(&self) -> &'static str {
        match self {
            Self::InterfaceSpecific => "interface-specific",
            Self::Audio => "audio",
            Self::Cdc => "cdc",
            Self::Hid => "hid",
            Self::Physical => "physical",
            Self::Image => "image",
            Self::Printer => "printer",
            Self::MassStorage => "mass-storage",
            Self::Hub => "hub",
            Self::CdcData => "cdc-data",
            Self::SmartCard => "smart-card",
            Self::ContentSecurity => "content-security",
            Self::Video => "video",
            Self::PersonalHealthcare => "personal-healthcare",
            Self::AudioVideo => "audio-video",
            Self::Billboard => "billboard",
            Self::UsbCBridge => "usb-c-bridge",
            Self::Diagnostic => "diagnostic",
            Self::Wireless => "wireless",
            Self::Miscellaneous => "misc",
            Self::ApplicationSpecific => "application-specific",
            Self::VendorSpecific => "vendor-specific",
            Self::Unknown => "unknown",
        }
    }

    /// Is storage class
    #[inline(always)]
    pub fn is_storage(&self) -> bool {
        matches!(self, Self::MassStorage)
    }

    /// Is input device
    #[inline(always)]
    pub fn is_input(&self) -> bool {
        matches!(self, Self::Hid)
    }

    /// Is network device
    #[inline(always)]
    pub fn is_network(&self) -> bool {
        matches!(self, Self::Cdc | Self::CdcData | Self::Wireless)
    }
}

/// USB device state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbDeviceState {
    /// Attached but not addressed
    Attached,
    /// Default state
    Default,
    /// Addressed
    Addressed,
    /// Configured
    Configured,
    /// Suspended
    Suspended,
    /// Disconnected
    Disconnected,
}

impl UsbDeviceState {
    /// Get state name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Attached => "attached",
            Self::Default => "default",
            Self::Addressed => "addressed",
            Self::Configured => "configured",
            Self::Suspended => "suspended",
            Self::Disconnected => "disconnected",
        }
    }

    /// Is active
    #[inline(always)]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Configured)
    }
}
