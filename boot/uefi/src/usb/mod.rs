//! USB Protocol Support for Helix UEFI Bootloader
//!
//! This module provides comprehensive USB support for the UEFI boot environment,
//! including USB device enumeration, class drivers, and transfer management.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         USB Protocol Stack                              │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Class Drivers   │  HID  │  Mass Storage  │  Hub  │  Audio  │  Video  │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  USB Core        │  Device Enumeration  │  Endpoint Management         │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Host Controller │  XHCI  │  EHCI  │  UHCI/OHCI                        │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Hardware        │  Root Hub Ports  │  External Hubs                   │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

use core::fmt;

// =============================================================================
// USB CONSTANTS
// =============================================================================

/// Maximum USB devices supported
pub const MAX_USB_DEVICES: usize = 127;

/// Maximum endpoints per device
pub const MAX_ENDPOINTS: usize = 32;

/// Maximum configuration size
pub const MAX_CONFIG_SIZE: usize = 4096;

/// Control transfer timeout (milliseconds)
pub const CONTROL_TIMEOUT_MS: u32 = 5000;

/// Bulk transfer timeout (milliseconds)
pub const BULK_TIMEOUT_MS: u32 = 30000;

/// Maximum packet sizes for different speeds
pub const MAX_PACKET_LOW_SPEED: u16 = 8;
pub const MAX_PACKET_FULL_SPEED: u16 = 64;
pub const MAX_PACKET_HIGH_SPEED: u16 = 512;
pub const MAX_PACKET_SUPER_SPEED: u16 = 1024;

// =============================================================================
// USB SPEED
// =============================================================================

/// USB device speed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UsbSpeed {
    /// Low speed (1.5 Mbps)
    Low         = 0,
    /// Full speed (12 Mbps)
    Full        = 1,
    /// High speed (480 Mbps)
    High        = 2,
    /// Super speed (5 Gbps)
    Super       = 3,
    /// Super speed+ (10 Gbps)
    SuperPlus   = 4,
    /// Super speed+ 20 Gbps
    SuperPlus20 = 5,
}

impl UsbSpeed {
    /// Get the maximum packet size for this speed
    pub const fn max_packet_size(&self) -> u16 {
        match self {
            UsbSpeed::Low => MAX_PACKET_LOW_SPEED,
            UsbSpeed::Full => MAX_PACKET_FULL_SPEED,
            UsbSpeed::High => MAX_PACKET_HIGH_SPEED,
            UsbSpeed::Super | UsbSpeed::SuperPlus | UsbSpeed::SuperPlus20 => MAX_PACKET_SUPER_SPEED,
        }
    }

    /// Get speed name
    pub const fn name(&self) -> &'static str {
        match self {
            UsbSpeed::Low => "Low Speed (1.5 Mbps)",
            UsbSpeed::Full => "Full Speed (12 Mbps)",
            UsbSpeed::High => "High Speed (480 Mbps)",
            UsbSpeed::Super => "SuperSpeed (5 Gbps)",
            UsbSpeed::SuperPlus => "SuperSpeed+ (10 Gbps)",
            UsbSpeed::SuperPlus20 => "SuperSpeed+ (20 Gbps)",
        }
    }

    /// Get theoretical bandwidth in bytes per second
    pub const fn bandwidth_bps(&self) -> u64 {
        match self {
            UsbSpeed::Low => 1_500_000 / 8,
            UsbSpeed::Full => 12_000_000 / 8,
            UsbSpeed::High => 480_000_000 / 8,
            UsbSpeed::Super => 5_000_000_000 / 8,
            UsbSpeed::SuperPlus => 10_000_000_000 / 8,
            UsbSpeed::SuperPlus20 => 20_000_000_000 / 8,
        }
    }
}

impl fmt::Display for UsbSpeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// =============================================================================
// USB DEVICE ADDRESS
// =============================================================================

/// USB device address (0-127)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UsbAddress(pub u8);

impl UsbAddress {
    /// Default address (0) used during enumeration
    pub const DEFAULT: Self = Self(0);

    /// Create a new USB address
    pub const fn new(addr: u8) -> Option<Self> {
        if addr <= 127 {
            Some(Self(addr))
        } else {
            None
        }
    }

    /// Get the raw address value
    pub const fn value(&self) -> u8 {
        self.0
    }

    /// Check if this is the default address
    pub const fn is_default(&self) -> bool {
        self.0 == 0
    }
}

// =============================================================================
// USB ENDPOINT
// =============================================================================

/// USB endpoint address
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndpointAddress(pub u8);

impl EndpointAddress {
    /// Control endpoint 0 IN
    pub const CONTROL_IN: Self = Self(0x80);
    /// Control endpoint 0 OUT
    pub const CONTROL_OUT: Self = Self(0x00);

    /// Create endpoint from number and direction
    pub const fn new(number: u8, is_in: bool) -> Self {
        let addr = (number & 0x0F) | if is_in { 0x80 } else { 0x00 };
        Self(addr)
    }

    /// Get endpoint number (0-15)
    pub const fn number(&self) -> u8 {
        self.0 & 0x0F
    }

    /// Check if endpoint is IN (device to host)
    pub const fn is_in(&self) -> bool {
        (self.0 & 0x80) != 0
    }

    /// Check if endpoint is OUT (host to device)
    pub const fn is_out(&self) -> bool {
        (self.0 & 0x80) == 0
    }

    /// Get raw address value
    pub const fn value(&self) -> u8 {
        self.0
    }
}

/// USB endpoint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EndpointType {
    /// Control endpoint
    Control     = 0,
    /// Isochronous endpoint (streaming)
    Isochronous = 1,
    /// Bulk endpoint (large data)
    Bulk        = 2,
    /// Interrupt endpoint (small, guaranteed latency)
    Interrupt   = 3,
}

impl EndpointType {
    /// Create from bmAttributes value
    pub const fn from_attributes(attrs: u8) -> Self {
        match attrs & 0x03 {
            0 => EndpointType::Control,
            1 => EndpointType::Isochronous,
            2 => EndpointType::Bulk,
            3 => EndpointType::Interrupt,
            _ => EndpointType::Control, // Unreachable
        }
    }
}

/// USB endpoint descriptor
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct EndpointDescriptor {
    /// Descriptor length (7)
    pub length: u8,
    /// Descriptor type (5 = Endpoint)
    pub descriptor_type: u8,
    /// Endpoint address
    pub endpoint_address: u8,
    /// Attributes (type, sync, usage)
    pub attributes: u8,
    /// Maximum packet size
    pub max_packet_size: u16,
    /// Polling interval
    pub interval: u8,
}

impl EndpointDescriptor {
    /// Get endpoint address
    pub const fn address(&self) -> EndpointAddress {
        EndpointAddress(self.endpoint_address)
    }

    /// Get endpoint type
    pub const fn endpoint_type(&self) -> EndpointType {
        EndpointType::from_attributes(self.attributes)
    }

    /// Get synchronization type for isochronous endpoints
    pub const fn sync_type(&self) -> u8 {
        (self.attributes >> 2) & 0x03
    }

    /// Get usage type for isochronous endpoints
    pub const fn usage_type(&self) -> u8 {
        (self.attributes >> 4) & 0x03
    }

    /// Get maximum packet size
    pub const fn max_packet(&self) -> u16 {
        self.max_packet_size & 0x07FF
    }

    /// Get additional transactions per microframe (high speed)
    pub const fn additional_transactions(&self) -> u8 {
        ((self.max_packet_size >> 11) & 0x03) as u8
    }
}

// =============================================================================
// USB DEVICE DESCRIPTOR
// =============================================================================

/// USB device descriptor
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DeviceDescriptor {
    /// Descriptor length (18)
    pub length: u8,
    /// Descriptor type (1 = Device)
    pub descriptor_type: u8,
    /// USB specification version (BCD)
    pub usb_version: u16,
    /// Device class
    pub device_class: u8,
    /// Device subclass
    pub device_subclass: u8,
    /// Device protocol
    pub device_protocol: u8,
    /// Maximum packet size for endpoint 0
    pub max_packet_size0: u8,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
    /// Device version (BCD)
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

impl DeviceDescriptor {
    /// Get USB version as (major, minor, sub)
    pub const fn usb_version_tuple(&self) -> (u8, u8, u8) {
        let major = ((self.usb_version >> 8) & 0xFF) as u8;
        let minor = ((self.usb_version >> 4) & 0x0F) as u8;
        let sub = (self.usb_version & 0x0F) as u8;
        (major, minor, sub)
    }

    /// Get device version as (major, minor)
    pub const fn device_version_tuple(&self) -> (u8, u8) {
        let major = ((self.device_version >> 8) & 0xFF) as u8;
        let minor = (self.device_version & 0xFF) as u8;
        (major, minor)
    }

    /// Check if device is USB 3.x
    pub const fn is_usb3(&self) -> bool {
        (self.usb_version >> 8) >= 3
    }

    /// Check if device is a hub
    pub const fn is_hub(&self) -> bool {
        self.device_class == 0x09
    }

    /// Check if class is defined at interface level
    pub const fn class_at_interface(&self) -> bool {
        self.device_class == 0x00
    }
}

// =============================================================================
// USB CONFIGURATION DESCRIPTOR
// =============================================================================

/// USB configuration descriptor
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct ConfigurationDescriptor {
    /// Descriptor length (9)
    pub length: u8,
    /// Descriptor type (2 = Configuration)
    pub descriptor_type: u8,
    /// Total length of configuration data
    pub total_length: u16,
    /// Number of interfaces
    pub num_interfaces: u8,
    /// Configuration value
    pub configuration_value: u8,
    /// Configuration string index
    pub configuration_index: u8,
    /// Attributes (self-powered, remote wakeup)
    pub attributes: u8,
    /// Maximum power (in 2mA units)
    pub max_power: u8,
}

impl ConfigurationDescriptor {
    /// Check if device is self-powered
    pub const fn is_self_powered(&self) -> bool {
        (self.attributes & 0x40) != 0
    }

    /// Check if remote wakeup is supported
    pub const fn supports_remote_wakeup(&self) -> bool {
        (self.attributes & 0x20) != 0
    }

    /// Get maximum power in milliamps
    pub const fn max_power_ma(&self) -> u16 {
        (self.max_power as u16) * 2
    }
}

// =============================================================================
// USB INTERFACE DESCRIPTOR
// =============================================================================

/// USB interface descriptor
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct InterfaceDescriptor {
    /// Descriptor length (9)
    pub length: u8,
    /// Descriptor type (4 = Interface)
    pub descriptor_type: u8,
    /// Interface number
    pub interface_number: u8,
    /// Alternate setting
    pub alternate_setting: u8,
    /// Number of endpoints
    pub num_endpoints: u8,
    /// Interface class
    pub interface_class: u8,
    /// Interface subclass
    pub interface_subclass: u8,
    /// Interface protocol
    pub interface_protocol: u8,
    /// Interface string index
    pub interface_index: u8,
}

impl InterfaceDescriptor {
    /// Check if this is a HID interface
    pub const fn is_hid(&self) -> bool {
        self.interface_class == 0x03
    }

    /// Check if this is a mass storage interface
    pub const fn is_mass_storage(&self) -> bool {
        self.interface_class == 0x08
    }

    /// Check if this is a hub interface
    pub const fn is_hub(&self) -> bool {
        self.interface_class == 0x09
    }

    /// Check if this is a CDC interface
    pub const fn is_cdc(&self) -> bool {
        self.interface_class == 0x02
    }

    /// Check if this is a vendor-specific interface
    pub const fn is_vendor_specific(&self) -> bool {
        self.interface_class == 0xFF
    }
}

// =============================================================================
// USB STRING DESCRIPTOR
// =============================================================================

/// USB string descriptor header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct StringDescriptorHeader {
    /// Descriptor length
    pub length: u8,
    /// Descriptor type (3 = String)
    pub descriptor_type: u8,
}

/// USB string descriptor with fixed buffer
#[derive(Clone, Copy)]
pub struct StringDescriptor {
    /// Length of descriptor
    pub length: u8,
    /// String data (UTF-16LE)
    pub data: [u16; 126],
}

impl StringDescriptor {
    /// Create empty string descriptor
    pub const fn new() -> Self {
        Self {
            length: 0,
            data: [0; 126],
        }
    }

    /// Get string length in UTF-16 characters
    pub const fn len(&self) -> usize {
        if self.length < 2 {
            0
        } else {
            ((self.length - 2) / 2) as usize
        }
    }

    /// Check if string is empty
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get string data slice
    pub fn as_slice(&self) -> &[u16] {
        &self.data[..self.len()]
    }
}

impl Default for StringDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// USB DEVICE CLASS CODES
// =============================================================================

/// USB class codes
pub mod class {
    /// Class defined at interface level
    pub const INTERFACE: u8 = 0x00;
    /// Audio class
    pub const AUDIO: u8 = 0x01;
    /// Communications and CDC Control
    pub const CDC: u8 = 0x02;
    /// Human Interface Device (HID)
    pub const HID: u8 = 0x03;
    /// Physical
    pub const PHYSICAL: u8 = 0x05;
    /// Image
    pub const IMAGE: u8 = 0x06;
    /// Printer
    pub const PRINTER: u8 = 0x07;
    /// Mass Storage
    pub const MASS_STORAGE: u8 = 0x08;
    /// Hub
    pub const HUB: u8 = 0x09;
    /// CDC-Data
    pub const CDC_DATA: u8 = 0x0A;
    /// Smart Card
    pub const SMART_CARD: u8 = 0x0B;
    /// Content Security
    pub const CONTENT_SECURITY: u8 = 0x0D;
    /// Video
    pub const VIDEO: u8 = 0x0E;
    /// Personal Healthcare
    pub const PERSONAL_HEALTHCARE: u8 = 0x0F;
    /// Audio/Video Devices
    pub const AUDIO_VIDEO: u8 = 0x10;
    /// Billboard Device
    pub const BILLBOARD: u8 = 0x11;
    /// USB Type-C Bridge
    pub const TYPE_C_BRIDGE: u8 = 0x12;
    /// Diagnostic Device
    pub const DIAGNOSTIC: u8 = 0xDC;
    /// Wireless Controller
    pub const WIRELESS_CONTROLLER: u8 = 0xE0;
    /// Miscellaneous
    pub const MISCELLANEOUS: u8 = 0xEF;
    /// Application Specific
    pub const APPLICATION_SPECIFIC: u8 = 0xFE;
    /// Vendor Specific
    pub const VENDOR_SPECIFIC: u8 = 0xFF;

    /// Get class name
    pub const fn class_name(class: u8) -> &'static str {
        match class {
            INTERFACE => "Interface-defined",
            AUDIO => "Audio",
            CDC => "Communications",
            HID => "HID",
            PHYSICAL => "Physical",
            IMAGE => "Image",
            PRINTER => "Printer",
            MASS_STORAGE => "Mass Storage",
            HUB => "Hub",
            CDC_DATA => "CDC-Data",
            SMART_CARD => "Smart Card",
            CONTENT_SECURITY => "Content Security",
            VIDEO => "Video",
            PERSONAL_HEALTHCARE => "Personal Healthcare",
            AUDIO_VIDEO => "Audio/Video",
            BILLBOARD => "Billboard",
            TYPE_C_BRIDGE => "USB Type-C Bridge",
            DIAGNOSTIC => "Diagnostic",
            WIRELESS_CONTROLLER => "Wireless Controller",
            MISCELLANEOUS => "Miscellaneous",
            APPLICATION_SPECIFIC => "Application Specific",
            VENDOR_SPECIFIC => "Vendor Specific",
            _ => "Unknown",
        }
    }
}

// =============================================================================
// USB STANDARD REQUESTS
// =============================================================================

/// USB standard request types
pub mod request {
    /// Get status
    pub const GET_STATUS: u8 = 0;
    /// Clear feature
    pub const CLEAR_FEATURE: u8 = 1;
    /// Set feature
    pub const SET_FEATURE: u8 = 3;
    /// Set address
    pub const SET_ADDRESS: u8 = 5;
    /// Get descriptor
    pub const GET_DESCRIPTOR: u8 = 6;
    /// Set descriptor
    pub const SET_DESCRIPTOR: u8 = 7;
    /// Get configuration
    pub const GET_CONFIGURATION: u8 = 8;
    /// Set configuration
    pub const SET_CONFIGURATION: u8 = 9;
    /// Get interface
    pub const GET_INTERFACE: u8 = 10;
    /// Set interface
    pub const SET_INTERFACE: u8 = 11;
    /// Synch frame
    pub const SYNCH_FRAME: u8 = 12;
}

/// USB descriptor types
pub mod descriptor_type {
    /// Device descriptor
    pub const DEVICE: u8 = 1;
    /// Configuration descriptor
    pub const CONFIGURATION: u8 = 2;
    /// String descriptor
    pub const STRING: u8 = 3;
    /// Interface descriptor
    pub const INTERFACE: u8 = 4;
    /// Endpoint descriptor
    pub const ENDPOINT: u8 = 5;
    /// Device qualifier
    pub const DEVICE_QUALIFIER: u8 = 6;
    /// Other speed configuration
    pub const OTHER_SPEED_CONFIGURATION: u8 = 7;
    /// Interface power
    pub const INTERFACE_POWER: u8 = 8;
    /// OTG descriptor
    pub const OTG: u8 = 9;
    /// Debug descriptor
    pub const DEBUG: u8 = 10;
    /// Interface association
    pub const INTERFACE_ASSOCIATION: u8 = 11;
    /// BOS descriptor
    pub const BOS: u8 = 15;
    /// Device capability
    pub const DEVICE_CAPABILITY: u8 = 16;
    /// SuperSpeed endpoint companion
    pub const SS_ENDPOINT_COMPANION: u8 = 48;
    /// SuperSpeedPlus isochronous endpoint companion
    pub const SSP_ISOCHRONOUS_ENDPOINT_COMPANION: u8 = 49;
}

/// USB feature selectors
pub mod feature {
    /// Endpoint halt
    pub const ENDPOINT_HALT: u16 = 0;
    /// Device remote wakeup
    pub const DEVICE_REMOTE_WAKEUP: u16 = 1;
    /// Test mode
    pub const TEST_MODE: u16 = 2;
    /// Function suspend (USB 3.0)
    pub const FUNCTION_SUSPEND: u16 = 0;
    /// U1 enable (USB 3.0)
    pub const U1_ENABLE: u16 = 48;
    /// U2 enable (USB 3.0)
    pub const U2_ENABLE: u16 = 49;
    /// LTM enable (USB 3.0)
    pub const LTM_ENABLE: u16 = 50;
}

// =============================================================================
// USB SETUP PACKET
// =============================================================================

/// USB setup packet for control transfers
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct SetupPacket {
    /// Request type (direction, type, recipient)
    pub request_type: u8,
    /// Request code
    pub request: u8,
    /// Value (request-specific)
    pub value: u16,
    /// Index (request-specific)
    pub index: u16,
    /// Length of data stage
    pub length: u16,
}

impl SetupPacket {
    /// Create a GET_DESCRIPTOR request for device descriptor
    pub const fn get_device_descriptor() -> Self {
        Self {
            request_type: 0x80, // IN, Standard, Device
            request: request::GET_DESCRIPTOR,
            value: (descriptor_type::DEVICE as u16) << 8,
            index: 0,
            length: 18,
        }
    }

    /// Create a GET_DESCRIPTOR request for configuration descriptor
    pub const fn get_configuration_descriptor(index: u8, length: u16) -> Self {
        Self {
            request_type: 0x80,
            request: request::GET_DESCRIPTOR,
            value: ((descriptor_type::CONFIGURATION as u16) << 8) | (index as u16),
            index: 0,
            length,
        }
    }

    /// Create a GET_DESCRIPTOR request for string descriptor
    pub const fn get_string_descriptor(index: u8, language_id: u16) -> Self {
        Self {
            request_type: 0x80,
            request: request::GET_DESCRIPTOR,
            value: ((descriptor_type::STRING as u16) << 8) | (index as u16),
            index: language_id,
            length: 255,
        }
    }

    /// Create a SET_ADDRESS request
    pub const fn set_address(address: u8) -> Self {
        Self {
            request_type: 0x00,
            request: request::SET_ADDRESS,
            value: address as u16,
            index: 0,
            length: 0,
        }
    }

    /// Create a SET_CONFIGURATION request
    pub const fn set_configuration(config: u8) -> Self {
        Self {
            request_type: 0x00,
            request: request::SET_CONFIGURATION,
            value: config as u16,
            index: 0,
            length: 0,
        }
    }

    /// Create a GET_STATUS request for device
    pub const fn get_device_status() -> Self {
        Self {
            request_type: 0x80,
            request: request::GET_STATUS,
            value: 0,
            index: 0,
            length: 2,
        }
    }

    /// Create a CLEAR_FEATURE request
    pub const fn clear_feature(recipient: u8, feature: u16, index: u16) -> Self {
        Self {
            request_type: recipient,
            request: request::CLEAR_FEATURE,
            value: feature,
            index,
            length: 0,
        }
    }

    /// Create a SET_FEATURE request
    pub const fn set_feature(recipient: u8, feature: u16, index: u16) -> Self {
        Self {
            request_type: recipient,
            request: request::SET_FEATURE,
            value: feature,
            index,
            length: 0,
        }
    }

    /// Create a SET_INTERFACE request
    pub const fn set_interface(interface: u8, alternate: u8) -> Self {
        Self {
            request_type: 0x01, // OUT, Standard, Interface
            request: request::SET_INTERFACE,
            value: alternate as u16,
            index: interface as u16,
            length: 0,
        }
    }

    /// Check if direction is IN (device to host)
    pub const fn is_in(&self) -> bool {
        (self.request_type & 0x80) != 0
    }

    /// Check if direction is OUT (host to device)
    pub const fn is_out(&self) -> bool {
        (self.request_type & 0x80) == 0
    }

    /// Get request type (Standard, Class, Vendor)
    pub const fn request_type_type(&self) -> u8 {
        (self.request_type >> 5) & 0x03
    }

    /// Get recipient (Device, Interface, Endpoint, Other)
    pub const fn recipient(&self) -> u8 {
        self.request_type & 0x1F
    }
}

// =============================================================================
// USB DEVICE STATE
// =============================================================================

/// USB device state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    /// Device not yet attached
    Detached,
    /// Device attached, waiting for reset
    Attached,
    /// Device powered
    Powered,
    /// Device reset, using default address
    Default,
    /// Device has been assigned an address
    Addressed,
    /// Device is configured and ready
    Configured,
    /// Device is suspended
    Suspended,
}

// =============================================================================
// USB TRANSFER TYPES
// =============================================================================

/// USB transfer status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferStatus {
    /// Transfer completed successfully
    Success,
    /// Transfer in progress
    Pending,
    /// Transfer stalled
    Stalled,
    /// Data buffer error
    BufferError,
    /// Babble detected
    Babble,
    /// Transaction error
    TransactionError,
    /// CRC/Timeout/Bit-stuff error
    ProtocolError,
    /// Device not responding
    NoDevice,
    /// Transfer cancelled
    Cancelled,
    /// Unknown error
    Unknown,
}

/// USB transfer direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    /// Host to device
    Out,
    /// Device to host
    In,
}

/// USB transfer request
#[derive(Debug, Clone, Copy)]
pub struct TransferRequest {
    /// Device address
    pub device: UsbAddress,
    /// Endpoint address
    pub endpoint: EndpointAddress,
    /// Transfer direction
    pub direction: TransferDirection,
    /// Data buffer pointer
    pub buffer: *mut u8,
    /// Buffer length
    pub length: usize,
    /// Actual bytes transferred
    pub actual: usize,
    /// Transfer status
    pub status: TransferStatus,
    /// Setup packet (for control transfers)
    pub setup: Option<SetupPacket>,
}

impl TransferRequest {
    /// Create a control transfer request
    pub const fn control(
        device: UsbAddress,
        setup: SetupPacket,
        buffer: *mut u8,
        length: usize,
    ) -> Self {
        Self {
            device,
            endpoint: if setup.is_in() {
                EndpointAddress::CONTROL_IN
            } else {
                EndpointAddress::CONTROL_OUT
            },
            direction: if setup.is_in() {
                TransferDirection::In
            } else {
                TransferDirection::Out
            },
            buffer,
            length,
            actual: 0,
            status: TransferStatus::Pending,
            setup: Some(setup),
        }
    }

    /// Create a bulk IN transfer request
    pub const fn bulk_in(device: UsbAddress, endpoint: u8, buffer: *mut u8, length: usize) -> Self {
        Self {
            device,
            endpoint: EndpointAddress::new(endpoint, true),
            direction: TransferDirection::In,
            buffer,
            length,
            actual: 0,
            status: TransferStatus::Pending,
            setup: None,
        }
    }

    /// Create a bulk OUT transfer request
    pub const fn bulk_out(
        device: UsbAddress,
        endpoint: u8,
        buffer: *mut u8,
        length: usize,
    ) -> Self {
        Self {
            device,
            endpoint: EndpointAddress::new(endpoint, false),
            direction: TransferDirection::Out,
            buffer,
            length,
            actual: 0,
            status: TransferStatus::Pending,
            setup: None,
        }
    }

    /// Check if transfer completed successfully
    pub const fn is_success(&self) -> bool {
        matches!(self.status, TransferStatus::Success)
    }
}

// =============================================================================
// USB HUB
// =============================================================================

/// USB hub descriptor
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct HubDescriptor {
    /// Descriptor length
    pub length: u8,
    /// Descriptor type (0x29 = Hub)
    pub descriptor_type: u8,
    /// Number of downstream ports
    pub num_ports: u8,
    /// Hub characteristics
    pub characteristics: u16,
    /// Time to power on (in 2ms units)
    pub power_on_time: u8,
    /// Maximum current (in mA)
    pub max_current: u8,
    /// Device removable bitmap
    pub device_removable: [u8; 32],
}

impl HubDescriptor {
    /// Get power on time in milliseconds
    pub const fn power_on_time_ms(&self) -> u16 {
        (self.power_on_time as u16) * 2
    }

    /// Check if compound device
    pub const fn is_compound(&self) -> bool {
        (self.characteristics & 0x0004) != 0
    }

    /// Get power switching mode
    pub const fn power_switching_mode(&self) -> u8 {
        (self.characteristics & 0x0003) as u8
    }

    /// Get overcurrent protection mode
    pub const fn overcurrent_mode(&self) -> u8 {
        ((self.characteristics >> 3) & 0x03) as u8
    }

    /// Get TT think time (for high-speed hubs)
    pub const fn tt_think_time(&self) -> u8 {
        ((self.characteristics >> 5) & 0x03) as u8
    }
}

/// Hub port status
#[derive(Debug, Clone, Copy)]
pub struct HubPortStatus {
    /// Port status bits
    pub status: u16,
    /// Port change bits
    pub change: u16,
}

impl HubPortStatus {
    /// Port connection status
    pub const fn is_connected(&self) -> bool {
        (self.status & 0x0001) != 0
    }

    /// Port enabled
    pub const fn is_enabled(&self) -> bool {
        (self.status & 0x0002) != 0
    }

    /// Port suspended
    pub const fn is_suspended(&self) -> bool {
        (self.status & 0x0004) != 0
    }

    /// Port overcurrent
    pub const fn is_overcurrent(&self) -> bool {
        (self.status & 0x0008) != 0
    }

    /// Port reset
    pub const fn is_reset(&self) -> bool {
        (self.status & 0x0010) != 0
    }

    /// Port power enabled
    pub const fn is_powered(&self) -> bool {
        (self.status & 0x0100) != 0
    }

    /// Port speed
    pub const fn speed(&self) -> UsbSpeed {
        if (self.status & 0x0200) != 0 {
            UsbSpeed::Low
        } else if (self.status & 0x0400) != 0 {
            UsbSpeed::High
        } else {
            UsbSpeed::Full
        }
    }

    /// Connection changed
    pub const fn connection_changed(&self) -> bool {
        (self.change & 0x0001) != 0
    }

    /// Enable changed
    pub const fn enable_changed(&self) -> bool {
        (self.change & 0x0002) != 0
    }

    /// Suspend changed
    pub const fn suspend_changed(&self) -> bool {
        (self.change & 0x0004) != 0
    }

    /// Overcurrent changed
    pub const fn overcurrent_changed(&self) -> bool {
        (self.change & 0x0008) != 0
    }

    /// Reset changed
    pub const fn reset_changed(&self) -> bool {
        (self.change & 0x0010) != 0
    }
}

// =============================================================================
// USB MASS STORAGE
// =============================================================================

/// USB mass storage subclass codes
pub mod mass_storage_subclass {
    /// SCSI transparent command set
    pub const SCSI: u8 = 0x06;
    /// Reduced Block Commands (RBC)
    pub const RBC: u8 = 0x01;
    /// MMC-5 (ATAPI)
    pub const MMC5: u8 = 0x02;
    /// UFI (USB Floppy Interface)
    pub const UFI: u8 = 0x04;
    /// SFF-8070i
    pub const SFF8070I: u8 = 0x05;
    /// Vendor specific
    pub const VENDOR: u8 = 0xFF;
}

/// USB mass storage protocol codes
pub mod mass_storage_protocol {
    /// Control/Bulk/Interrupt with command completion interrupt
    pub const CBI_INTERRUPT: u8 = 0x00;
    /// Control/Bulk/Interrupt without command completion interrupt
    pub const CBI_NO_INTERRUPT: u8 = 0x01;
    /// Bulk-Only Transport
    pub const BBB: u8 = 0x50;
    /// USB Attached SCSI (UAS)
    pub const UAS: u8 = 0x62;
    /// Vendor specific
    pub const VENDOR: u8 = 0xFF;
}

/// Command Block Wrapper (CBW) for Bulk-Only Transport
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct CommandBlockWrapper {
    /// Signature (0x43425355)
    pub signature: u32,
    /// Tag (unique identifier)
    pub tag: u32,
    /// Data transfer length
    pub data_transfer_length: u32,
    /// Flags (direction)
    pub flags: u8,
    /// LUN
    pub lun: u8,
    /// Command block length
    pub cb_length: u8,
    /// Command block (SCSI command)
    pub command_block: [u8; 16],
}

impl CommandBlockWrapper {
    /// CBW signature
    pub const SIGNATURE: u32 = 0x43425355;

    /// Create a new CBW
    pub const fn new(tag: u32, data_length: u32, direction_in: bool, lun: u8) -> Self {
        Self {
            signature: Self::SIGNATURE,
            tag,
            data_transfer_length: data_length,
            flags: if direction_in { 0x80 } else { 0x00 },
            lun,
            cb_length: 0,
            command_block: [0; 16],
        }
    }

    /// Set SCSI command
    pub fn set_command(&mut self, command: &[u8]) {
        let len = command.len().min(16);
        self.command_block[..len].copy_from_slice(&command[..len]);
        self.cb_length = len as u8;
    }

    /// Check if direction is IN
    ///
    /// # Safety
    /// Uses unaligned reads for packed struct field access.
    pub fn is_in(&self) -> bool {
        // SAFETY: Reading from packed struct field using unaligned access
        unsafe {
            let flags = core::ptr::addr_of!(self.flags).read_unaligned();
            (flags & 0x80) != 0
        }
    }
}

/// Command Status Wrapper (CSW) for Bulk-Only Transport
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct CommandStatusWrapper {
    /// Signature (0x53425355)
    pub signature: u32,
    /// Tag (must match CBW)
    pub tag: u32,
    /// Data residue
    pub data_residue: u32,
    /// Status
    pub status: u8,
}

impl CommandStatusWrapper {
    /// CSW signature
    pub const SIGNATURE: u32 = 0x53425355;

    /// Command passed
    pub const STATUS_PASSED: u8 = 0x00;
    /// Command failed
    pub const STATUS_FAILED: u8 = 0x01;
    /// Phase error
    pub const STATUS_PHASE_ERROR: u8 = 0x02;

    /// Check if command succeeded
    ///
    /// # Safety
    /// Uses unaligned reads for packed struct field access.
    pub fn is_success(&self) -> bool {
        // SAFETY: Reading from packed struct fields using unaligned access
        unsafe {
            let status = core::ptr::addr_of!(self.status).read_unaligned();
            let signature = core::ptr::addr_of!(self.signature).read_unaligned();
            status == Self::STATUS_PASSED && signature == Self::SIGNATURE
        }
    }

    /// Check if signature is valid
    ///
    /// # Safety
    /// Uses unaligned reads for packed struct field access.
    pub fn is_valid(&self) -> bool {
        // SAFETY: Reading from packed struct field using unaligned access
        unsafe {
            let signature = core::ptr::addr_of!(self.signature).read_unaligned();
            signature == Self::SIGNATURE
        }
    }
}

// =============================================================================
// USB HID
// =============================================================================

/// USB HID descriptor
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct HidDescriptor {
    /// Descriptor length
    pub length: u8,
    /// Descriptor type (0x21 = HID)
    pub descriptor_type: u8,
    /// HID specification version (BCD)
    pub hid_version: u16,
    /// Country code
    pub country_code: u8,
    /// Number of descriptors
    pub num_descriptors: u8,
    /// Class descriptor type (usually Report = 0x22)
    pub class_descriptor_type: u8,
    /// Class descriptor length
    pub class_descriptor_length: u16,
}

impl HidDescriptor {
    /// Get HID version as (major, minor)
    pub const fn version_tuple(&self) -> (u8, u8) {
        let major = ((self.hid_version >> 8) & 0xFF) as u8;
        let minor = (self.hid_version & 0xFF) as u8;
        (major, minor)
    }
}

/// HID protocol codes
pub mod hid_protocol {
    /// None
    pub const NONE: u8 = 0;
    /// Keyboard
    pub const KEYBOARD: u8 = 1;
    /// Mouse
    pub const MOUSE: u8 = 2;
}

/// HID subclass codes
pub mod hid_subclass {
    /// No subclass
    pub const NONE: u8 = 0;
    /// Boot interface
    pub const BOOT: u8 = 1;
}

/// HID class requests
pub mod hid_request {
    /// Get report
    pub const GET_REPORT: u8 = 0x01;
    /// Get idle
    pub const GET_IDLE: u8 = 0x02;
    /// Get protocol
    pub const GET_PROTOCOL: u8 = 0x03;
    /// Set report
    pub const SET_REPORT: u8 = 0x09;
    /// Set idle
    pub const SET_IDLE: u8 = 0x0A;
    /// Set protocol
    pub const SET_PROTOCOL: u8 = 0x0B;
}

/// USB keyboard boot report
#[derive(Debug, Clone, Copy, Default)]
#[repr(C, packed)]
pub struct KeyboardBootReport {
    /// Modifier keys
    pub modifiers: u8,
    /// Reserved
    pub reserved: u8,
    /// Key codes (up to 6 simultaneous keys)
    pub keys: [u8; 6],
}

impl KeyboardBootReport {
    /// Check if left control is pressed
    pub const fn left_ctrl(&self) -> bool {
        (self.modifiers & 0x01) != 0
    }

    /// Check if left shift is pressed
    pub const fn left_shift(&self) -> bool {
        (self.modifiers & 0x02) != 0
    }

    /// Check if left alt is pressed
    pub const fn left_alt(&self) -> bool {
        (self.modifiers & 0x04) != 0
    }

    /// Check if left GUI is pressed
    pub const fn left_gui(&self) -> bool {
        (self.modifiers & 0x08) != 0
    }

    /// Check if right control is pressed
    pub const fn right_ctrl(&self) -> bool {
        (self.modifiers & 0x10) != 0
    }

    /// Check if right shift is pressed
    pub const fn right_shift(&self) -> bool {
        (self.modifiers & 0x20) != 0
    }

    /// Check if right alt is pressed
    pub const fn right_alt(&self) -> bool {
        (self.modifiers & 0x40) != 0
    }

    /// Check if right GUI is pressed
    pub const fn right_gui(&self) -> bool {
        (self.modifiers & 0x80) != 0
    }

    /// Check if any shift is pressed
    pub const fn shift(&self) -> bool {
        (self.modifiers & 0x22) != 0
    }

    /// Check if any ctrl is pressed
    pub const fn ctrl(&self) -> bool {
        (self.modifiers & 0x11) != 0
    }

    /// Check if any alt is pressed
    pub const fn alt(&self) -> bool {
        (self.modifiers & 0x44) != 0
    }
}

/// USB mouse boot report
#[derive(Debug, Clone, Copy, Default)]
#[repr(C, packed)]
pub struct MouseBootReport {
    /// Button state
    pub buttons: u8,
    /// X movement
    pub x: i8,
    /// Y movement
    pub y: i8,
}

impl MouseBootReport {
    /// Check if left button is pressed
    pub const fn left_button(&self) -> bool {
        (self.buttons & 0x01) != 0
    }

    /// Check if right button is pressed
    pub const fn right_button(&self) -> bool {
        (self.buttons & 0x02) != 0
    }

    /// Check if middle button is pressed
    pub const fn middle_button(&self) -> bool {
        (self.buttons & 0x04) != 0
    }
}

// =============================================================================
// USB DEVICE INFO
// =============================================================================

/// Complete USB device information
#[derive(Clone)]
pub struct UsbDeviceInfo {
    /// Device address
    pub address: UsbAddress,
    /// Device speed
    pub speed: UsbSpeed,
    /// Device state
    pub state: DeviceState,
    /// Parent hub address (0 for root hub devices)
    pub parent_hub: UsbAddress,
    /// Parent port number
    pub parent_port: u8,
    /// Device descriptor
    pub device_descriptor: DeviceDescriptor,
    /// Active configuration
    pub configuration: u8,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
    /// Device class
    pub device_class: u8,
    /// Is hub
    pub is_hub: bool,
}

impl UsbDeviceInfo {
    /// Create new device info
    pub const fn new(address: UsbAddress, speed: UsbSpeed) -> Self {
        Self {
            address,
            speed,
            state: DeviceState::Default,
            parent_hub: UsbAddress::DEFAULT,
            parent_port: 0,
            device_descriptor: unsafe { core::mem::zeroed() },
            configuration: 0,
            vendor_id: 0,
            product_id: 0,
            device_class: 0,
            is_hub: false,
        }
    }
}

// =============================================================================
// USB ERROR TYPES
// =============================================================================

/// USB error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbError {
    /// Device not found
    DeviceNotFound,
    /// No free address available
    NoFreeAddress,
    /// Transfer timeout
    Timeout,
    /// Transfer stalled
    Stalled,
    /// CRC error
    CrcError,
    /// Bit stuff error
    BitStuffError,
    /// Data toggle mismatch
    DataToggleError,
    /// Device not responding
    NoResponse,
    /// Buffer too small
    BufferTooSmall,
    /// Invalid parameter
    InvalidParameter,
    /// Unsupported operation
    Unsupported,
    /// Out of memory
    OutOfMemory,
    /// Protocol error
    ProtocolError,
    /// Device disconnected
    Disconnected,
    /// Invalid descriptor
    InvalidDescriptor,
    /// Transfer in progress
    Busy,
    /// Unknown error
    Unknown,
}

impl fmt::Display for UsbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UsbError::DeviceNotFound => write!(f, "Device not found"),
            UsbError::NoFreeAddress => write!(f, "No free USB address"),
            UsbError::Timeout => write!(f, "Transfer timeout"),
            UsbError::Stalled => write!(f, "Endpoint stalled"),
            UsbError::CrcError => write!(f, "CRC error"),
            UsbError::BitStuffError => write!(f, "Bit stuff error"),
            UsbError::DataToggleError => write!(f, "Data toggle mismatch"),
            UsbError::NoResponse => write!(f, "Device not responding"),
            UsbError::BufferTooSmall => write!(f, "Buffer too small"),
            UsbError::InvalidParameter => write!(f, "Invalid parameter"),
            UsbError::Unsupported => write!(f, "Unsupported operation"),
            UsbError::OutOfMemory => write!(f, "Out of memory"),
            UsbError::ProtocolError => write!(f, "Protocol error"),
            UsbError::Disconnected => write!(f, "Device disconnected"),
            UsbError::InvalidDescriptor => write!(f, "Invalid descriptor"),
            UsbError::Busy => write!(f, "Transfer in progress"),
            UsbError::Unknown => write!(f, "Unknown USB error"),
        }
    }
}

// =============================================================================
// WELL-KNOWN VENDOR/PRODUCT IDS
// =============================================================================

/// Well-known USB vendor IDs
pub mod vendor {
    /// Unknown vendor
    pub const UNKNOWN: u16 = 0x0000;
    /// Apple
    pub const APPLE: u16 = 0x05AC;
    /// Logitech
    pub const LOGITECH: u16 = 0x046D;
    /// Microsoft
    pub const MICROSOFT: u16 = 0x045E;
    /// SanDisk
    pub const SANDISK: u16 = 0x0781;
    /// Samsung
    pub const SAMSUNG: u16 = 0x04E8;
    /// Kingston
    pub const KINGSTON: u16 = 0x0951;
    /// Western Digital
    pub const WESTERN_DIGITAL: u16 = 0x1058;
    /// Seagate
    pub const SEAGATE: u16 = 0x0BC2;
    /// Corsair
    pub const CORSAIR: u16 = 0x1B1C;
    /// Razer
    pub const RAZER: u16 = 0x1532;
    /// ASUS
    pub const ASUS: u16 = 0x0B05;
    /// Intel
    pub const INTEL: u16 = 0x8087;
    /// Realtek
    pub const REALTEK: u16 = 0x0BDA;
    /// Generic Hub
    pub const GENERIC_HUB: u16 = 0x0409;

    /// Get vendor name
    pub const fn name(vendor_id: u16) -> &'static str {
        match vendor_id {
            APPLE => "Apple",
            LOGITECH => "Logitech",
            MICROSOFT => "Microsoft",
            SANDISK => "SanDisk",
            SAMSUNG => "Samsung",
            KINGSTON => "Kingston",
            WESTERN_DIGITAL => "Western Digital",
            SEAGATE => "Seagate",
            CORSAIR => "Corsair",
            RAZER => "Razer",
            ASUS => "ASUS",
            INTEL => "Intel",
            REALTEK => "Realtek",
            _ => "Unknown",
        }
    }
}

// =============================================================================
// SCSI COMMANDS (for Mass Storage)
// =============================================================================

/// SCSI command codes
pub mod scsi {
    /// Test unit ready
    pub const TEST_UNIT_READY: u8 = 0x00;
    /// Request sense
    pub const REQUEST_SENSE: u8 = 0x03;
    /// Inquiry
    pub const INQUIRY: u8 = 0x12;
    /// Mode sense (6)
    pub const MODE_SENSE_6: u8 = 0x1A;
    /// Start/Stop unit
    pub const START_STOP_UNIT: u8 = 0x1B;
    /// Prevent/Allow medium removal
    pub const PREVENT_ALLOW_MEDIUM_REMOVAL: u8 = 0x1E;
    /// Read capacity (10)
    pub const READ_CAPACITY_10: u8 = 0x25;
    /// Read (10)
    pub const READ_10: u8 = 0x28;
    /// Write (10)
    pub const WRITE_10: u8 = 0x2A;
    /// Synchronize cache (10)
    pub const SYNCHRONIZE_CACHE_10: u8 = 0x35;
    /// Mode sense (10)
    pub const MODE_SENSE_10: u8 = 0x5A;
    /// Read capacity (16)
    pub const READ_CAPACITY_16: u8 = 0x9E;
    /// Read (16)
    pub const READ_16: u8 = 0x88;
    /// Write (16)
    pub const WRITE_16: u8 = 0x8A;
}

/// SCSI Inquiry response
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct ScsiInquiryResponse {
    /// Peripheral device type
    pub peripheral: u8,
    /// Removable media
    pub removable: u8,
    /// Version
    pub version: u8,
    /// Response data format
    pub response_format: u8,
    /// Additional length
    pub additional_length: u8,
    /// Flags
    pub flags: [u8; 3],
    /// Vendor identification
    pub vendor: [u8; 8],
    /// Product identification
    pub product: [u8; 16],
    /// Product revision
    pub revision: [u8; 4],
}

impl ScsiInquiryResponse {
    /// Get peripheral device type
    pub const fn device_type(&self) -> u8 {
        self.peripheral & 0x1F
    }

    /// Check if media is removable
    pub const fn is_removable(&self) -> bool {
        (self.removable & 0x80) != 0
    }
}

/// SCSI Read Capacity (10) response
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct ScsiReadCapacity10Response {
    /// Last logical block address (big-endian)
    pub last_lba: [u8; 4],
    /// Block length in bytes (big-endian)
    pub block_length: [u8; 4],
}

impl ScsiReadCapacity10Response {
    /// Get last LBA
    pub fn last_lba(&self) -> u32 {
        u32::from_be_bytes(self.last_lba)
    }

    /// Get block size
    pub fn block_size(&self) -> u32 {
        u32::from_be_bytes(self.block_length)
    }

    /// Get total capacity in bytes
    pub fn capacity_bytes(&self) -> u64 {
        (self.last_lba() as u64 + 1) * self.block_size() as u64
    }
}

// =============================================================================
// USB BOS DESCRIPTOR (USB 3.0+)
// =============================================================================

/// Binary Device Object Store (BOS) descriptor
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct BosDescriptor {
    /// Descriptor length (5)
    pub length: u8,
    /// Descriptor type (15 = BOS)
    pub descriptor_type: u8,
    /// Total length
    pub total_length: u16,
    /// Number of device capabilities
    pub num_device_caps: u8,
}

/// Device capability types
pub mod capability_type {
    /// Wireless USB
    pub const WIRELESS_USB: u8 = 0x01;
    /// USB 2.0 Extension
    pub const USB_20_EXTENSION: u8 = 0x02;
    /// SuperSpeed USB
    pub const SUPERSPEED_USB: u8 = 0x03;
    /// Container ID
    pub const CONTAINER_ID: u8 = 0x04;
    /// Platform
    pub const PLATFORM: u8 = 0x05;
    /// Power Delivery
    pub const POWER_DELIVERY: u8 = 0x06;
    /// Battery Info
    pub const BATTERY_INFO: u8 = 0x07;
    /// PD Consumer Port
    pub const PD_CONSUMER_PORT: u8 = 0x08;
    /// PD Provider Port
    pub const PD_PROVIDER_PORT: u8 = 0x09;
    /// SuperSpeed Plus
    pub const SUPERSPEED_PLUS: u8 = 0x0A;
    /// Precision Time Measurement
    pub const PRECISION_TIME_MEASUREMENT: u8 = 0x0B;
    /// Wireless USB Extension
    pub const WIRELESS_USB_EXT: u8 = 0x0C;
    /// Billboard
    pub const BILLBOARD: u8 = 0x0D;
    /// Authentication
    pub const AUTHENTICATION: u8 = 0x0E;
    /// Billboard Extension
    pub const BILLBOARD_EX: u8 = 0x0F;
    /// Configuration Summary
    pub const CONFIGURATION_SUMMARY: u8 = 0x10;
}

/// USB 2.0 Extension capability
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Usb20ExtensionCapability {
    /// Descriptor length
    pub length: u8,
    /// Descriptor type (16 = Device Capability)
    pub descriptor_type: u8,
    /// Capability type (2 = USB 2.0 Extension)
    pub capability_type: u8,
    /// Attributes
    pub attributes: u32,
}

impl Usb20ExtensionCapability {
    /// Check if LPM is supported
    pub const fn supports_lpm(&self) -> bool {
        (self.attributes & 0x02) != 0
    }

    /// Check if BESL/alternate HIRD is supported
    pub const fn supports_besl(&self) -> bool {
        (self.attributes & 0x04) != 0
    }
}

/// SuperSpeed USB capability
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct SuperSpeedCapability {
    /// Descriptor length
    pub length: u8,
    /// Descriptor type (16 = Device Capability)
    pub descriptor_type: u8,
    /// Capability type (3 = SuperSpeed USB)
    pub capability_type: u8,
    /// Attributes
    pub attributes: u8,
    /// Speeds supported bitmap
    pub speeds_supported: u16,
    /// Functionality support
    pub functionality_support: u8,
    /// U1 device exit latency
    pub u1_dev_exit_lat: u8,
    /// U2 device exit latency
    pub u2_dev_exit_lat: u16,
}

impl SuperSpeedCapability {
    /// Check if Low-Power Mode (LPM) is supported
    pub const fn supports_lpm(&self) -> bool {
        (self.attributes & 0x02) != 0
    }

    /// Check if Low-Speed is supported
    pub const fn supports_low_speed(&self) -> bool {
        (self.speeds_supported & 0x01) != 0
    }

    /// Check if Full-Speed is supported
    pub const fn supports_full_speed(&self) -> bool {
        (self.speeds_supported & 0x02) != 0
    }

    /// Check if High-Speed is supported
    pub const fn supports_high_speed(&self) -> bool {
        (self.speeds_supported & 0x04) != 0
    }

    /// Check if SuperSpeed is supported
    pub const fn supports_super_speed(&self) -> bool {
        (self.speeds_supported & 0x08) != 0
    }
}

// =============================================================================
// XHCI HOST CONTROLLER TYPES
// =============================================================================

/// xHCI capability register offset
pub const XHCI_CAP_LENGTH: usize = 0x00;
/// xHCI interface version offset
pub const XHCI_HCI_VERSION: usize = 0x02;
/// xHCI structural parameters 1 offset
pub const XHCI_HCSPARAMS1: usize = 0x04;
/// xHCI structural parameters 2 offset
pub const XHCI_HCSPARAMS2: usize = 0x08;
/// xHCI structural parameters 3 offset
pub const XHCI_HCSPARAMS3: usize = 0x0C;
/// xHCI capability parameters 1 offset
pub const XHCI_HCCPARAMS1: usize = 0x10;
/// xHCI doorbell offset offset
pub const XHCI_DBOFF: usize = 0x14;
/// xHCI runtime register offset offset
pub const XHCI_RTSOFF: usize = 0x18;
/// xHCI capability parameters 2 offset
pub const XHCI_HCCPARAMS2: usize = 0x1C;

/// xHCI operational register offsets (relative to operational base)
pub mod xhci_op {
    /// USB command register
    pub const USBCMD: usize = 0x00;
    /// USB status register
    pub const USBSTS: usize = 0x04;
    /// Page size register
    pub const PAGESIZE: usize = 0x08;
    /// Device notification control
    pub const DNCTRL: usize = 0x14;
    /// Command ring control register
    pub const CRCR: usize = 0x18;
    /// Device context base address array pointer
    pub const DCBAAP: usize = 0x30;
    /// Configure register
    pub const CONFIG: usize = 0x38;
}

/// xHCI USB command register bits
pub mod xhci_usbcmd {
    /// Run/Stop
    pub const RUN_STOP: u32 = 1 << 0;
    /// Host Controller Reset
    pub const HCRST: u32 = 1 << 1;
    /// Interrupter Enable
    pub const INTE: u32 = 1 << 2;
    /// Host System Error Enable
    pub const HSEE: u32 = 1 << 3;
    /// Light Host Controller Reset
    pub const LHCRST: u32 = 1 << 7;
    /// Controller Save State
    pub const CSS: u32 = 1 << 8;
    /// Controller Restore State
    pub const CRS: u32 = 1 << 9;
    /// Enable Wrap Event
    pub const EWE: u32 = 1 << 10;
    /// Enable U3 MFINDEX Stop
    pub const EU3S: u32 = 1 << 11;
    /// CEM Enable
    pub const CME: u32 = 1 << 13;
}

/// xHCI USB status register bits
pub mod xhci_usbsts {
    /// Host Controller Halted
    pub const HCH: u32 = 1 << 0;
    /// Host System Error
    pub const HSE: u32 = 1 << 2;
    /// Event Interrupt
    pub const EINT: u32 = 1 << 3;
    /// Port Change Detect
    pub const PCD: u32 = 1 << 4;
    /// Save State Status
    pub const SSS: u32 = 1 << 8;
    /// Restore State Status
    pub const RSS: u32 = 1 << 9;
    /// Save/Restore Error
    pub const SRE: u32 = 1 << 10;
    /// Controller Not Ready
    pub const CNR: u32 = 1 << 11;
    /// Host Controller Error
    pub const HCE: u32 = 1 << 12;
}

/// xHCI TRB (Transfer Request Block) types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TrbType {
    /// Reserved
    Reserved            = 0,
    /// Normal
    Normal              = 1,
    /// Setup Stage
    SetupStage          = 2,
    /// Data Stage
    DataStage           = 3,
    /// Status Stage
    StatusStage         = 4,
    /// Isoch
    Isoch               = 5,
    /// Link
    Link                = 6,
    /// Event Data
    EventData           = 7,
    /// No Op
    NoOpTransfer        = 8,
    /// Enable Slot
    EnableSlot          = 9,
    /// Disable Slot
    DisableSlot         = 10,
    /// Address Device
    AddressDevice       = 11,
    /// Configure Endpoint
    ConfigureEndpoint   = 12,
    /// Evaluate Context
    EvaluateContext     = 13,
    /// Reset Endpoint
    ResetEndpoint       = 14,
    /// Stop Endpoint
    StopEndpoint        = 15,
    /// Set TR Dequeue Pointer
    SetTRDequeuePointer = 16,
    /// Reset Device
    ResetDevice         = 17,
    /// Force Event
    ForceEvent          = 18,
    /// Negotiate Bandwidth
    NegotiateBandwidth  = 19,
    /// Set Latency Tolerance Value
    SetLatencyToleranceValue = 20,
    /// Get Port Bandwidth
    GetPortBandwidth    = 21,
    /// Force Header
    ForceHeader         = 22,
    /// No Op Command
    NoOpCommand         = 23,
    /// Transfer Event
    TransferEvent       = 32,
    /// Command Completion Event
    CommandCompletionEvent = 33,
    /// Port Status Change Event
    PortStatusChangeEvent = 34,
    /// Bandwidth Request Event
    BandwidthRequestEvent = 35,
    /// Doorbell Event
    DoorbellEvent       = 36,
    /// Host Controller Event
    HostControllerEvent = 37,
    /// Device Notification Event
    DeviceNotificationEvent = 38,
    /// MFINDEX Wrap Event
    MfindexWrapEvent    = 39,
}

/// xHCI Transfer Request Block
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Trb {
    /// Parameter (or data buffer pointer)
    pub parameter: u64,
    /// Status
    pub status: u32,
    /// Control
    pub control: u32,
}

impl Trb {
    /// Create a new empty TRB
    pub const fn new() -> Self {
        Self {
            parameter: 0,
            status: 0,
            control: 0,
        }
    }

    /// Get TRB type
    pub const fn trb_type(&self) -> u8 {
        ((self.control >> 10) & 0x3F) as u8
    }

    /// Get cycle bit
    pub const fn cycle(&self) -> bool {
        (self.control & 1) != 0
    }

    /// Set cycle bit
    pub fn set_cycle(&mut self, cycle: bool) {
        if cycle {
            self.control |= 1;
        } else {
            self.control &= !1;
        }
    }

    /// Create a Link TRB
    pub const fn link(ring_addr: u64, toggle_cycle: bool) -> Self {
        Self {
            parameter: ring_addr,
            status: 0,
            control: ((TrbType::Link as u32) << 10) | if toggle_cycle { 1 << 1 } else { 0 },
        }
    }

    /// Create a No-Op Command TRB
    pub const fn noop_command() -> Self {
        Self {
            parameter: 0,
            status: 0,
            control: (TrbType::NoOpCommand as u32) << 10,
        }
    }

    /// Create an Enable Slot TRB
    pub const fn enable_slot() -> Self {
        Self {
            parameter: 0,
            status: 0,
            control: (TrbType::EnableSlot as u32) << 10,
        }
    }

    /// Create a Disable Slot TRB
    pub const fn disable_slot(slot_id: u8) -> Self {
        Self {
            parameter: 0,
            status: 0,
            control: ((TrbType::DisableSlot as u32) << 10) | ((slot_id as u32) << 24),
        }
    }

    /// Create an Address Device TRB
    pub const fn address_device(input_context_ptr: u64, slot_id: u8, bsr: bool) -> Self {
        Self {
            parameter: input_context_ptr,
            status: 0,
            control: ((TrbType::AddressDevice as u32) << 10)
                | ((slot_id as u32) << 24)
                | if bsr { 1 << 9 } else { 0 },
        }
    }

    /// Create a Configure Endpoint TRB
    pub const fn configure_endpoint(input_context_ptr: u64, slot_id: u8, dc: bool) -> Self {
        Self {
            parameter: input_context_ptr,
            status: 0,
            control: ((TrbType::ConfigureEndpoint as u32) << 10)
                | ((slot_id as u32) << 24)
                | if dc { 1 << 9 } else { 0 },
        }
    }

    /// Create a Setup Stage TRB
    pub fn setup_stage(setup: &SetupPacket, trt: u8) -> Self {
        let param = unsafe {
            let ptr = setup as *const SetupPacket as *const u64;
            *ptr
        };

        Self {
            parameter: param,
            status: 8, // TRB transfer length = 8
            control: ((TrbType::SetupStage as u32) << 10)
                | ((trt as u32) << 16) // Transfer Type
                | (1 << 6), // IDT (Immediate Data)
        }
    }

    /// Create a Data Stage TRB
    pub const fn data_stage(buffer: u64, length: u32, direction_in: bool) -> Self {
        Self {
            parameter: buffer,
            status: length & 0x1FFFF, // TD Size = 0, TRB transfer length
            control: ((TrbType::DataStage as u32) << 10) | if direction_in { 1 << 16 } else { 0 },
        }
    }

    /// Create a Status Stage TRB
    pub const fn status_stage(direction_in: bool) -> Self {
        Self {
            parameter: 0,
            status: 0,
            control: ((TrbType::StatusStage as u32) << 10)
                | if direction_in { 1 << 16 } else { 0 }
                | (1 << 5), // IOC (Interrupt On Completion)
        }
    }

    /// Create a Normal TRB
    pub const fn normal(buffer: u64, length: u32) -> Self {
        Self {
            parameter: buffer,
            status: length & 0x1FFFF,
            control: ((TrbType::Normal as u32) << 10) | (1 << 5), // IOC
        }
    }
}

impl Default for Trb {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// USB 3.0+ LINK POWER MANAGEMENT
// =============================================================================

/// USB 3.0 link states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LinkState {
    /// U0 - Fully operational
    U0             = 0,
    /// U1 - Standby (fast exit)
    U1             = 1,
    /// U2 - Standby (slower exit)
    U2             = 2,
    /// U3 - Suspend
    U3             = 3,
    /// Disabled
    Disabled       = 4,
    /// RxDetect
    RxDetect       = 5,
    /// Inactive
    Inactive       = 6,
    /// Polling
    Polling        = 7,
    /// Recovery
    Recovery       = 8,
    /// Hot Reset
    HotReset       = 9,
    /// Compliance Mode
    ComplianceMode = 10,
    /// Test Mode
    TestMode       = 11,
    /// Resume
    Resume         = 15,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_usb_speed() {
        assert_eq!(UsbSpeed::Low.max_packet_size(), 8);
        assert_eq!(UsbSpeed::Full.max_packet_size(), 64);
        assert_eq!(UsbSpeed::High.max_packet_size(), 512);
        assert_eq!(UsbSpeed::Super.max_packet_size(), 1024);
    }

    #[test]
    fn test_usb_address() {
        assert!(UsbAddress::new(0).is_some());
        assert!(UsbAddress::new(127).is_some());
        assert!(UsbAddress::new(128).is_none());
        assert!(UsbAddress::DEFAULT.is_default());
    }

    #[test]
    fn test_endpoint_address() {
        let ep_in = EndpointAddress::new(1, true);
        assert!(ep_in.is_in());
        assert_eq!(ep_in.number(), 1);

        let ep_out = EndpointAddress::new(2, false);
        assert!(ep_out.is_out());
        assert_eq!(ep_out.number(), 2);
    }

    #[test]
    fn test_setup_packet() {
        let get_dev = SetupPacket::get_device_descriptor();
        assert!(get_dev.is_in());
        assert_eq!(get_dev.request, request::GET_DESCRIPTOR);
        assert_eq!(get_dev.length, 18);

        let set_addr = SetupPacket::set_address(5);
        assert!(set_addr.is_out());
        assert_eq!(set_addr.value, 5);
    }

    #[test]
    fn test_cbw_csw() {
        let cbw = CommandBlockWrapper::new(1, 512, true, 0);
        // SAFETY: Reading from packed struct field using unaligned access
        unsafe {
            assert_eq!(
                core::ptr::addr_of!(cbw.signature).read_unaligned(),
                CommandBlockWrapper::SIGNATURE
            );
        }
        assert!(cbw.is_in());

        let csw = CommandStatusWrapper {
            signature: CommandStatusWrapper::SIGNATURE,
            tag: 1,
            data_residue: 0,
            status: CommandStatusWrapper::STATUS_PASSED,
        };
        assert!(csw.is_success());
    }

    #[test]
    fn test_trb() {
        let trb = Trb::enable_slot();
        assert_eq!(trb.trb_type(), TrbType::EnableSlot as u8);

        let link = Trb::link(0x1000, true);
        assert_eq!(link.trb_type(), TrbType::Link as u8);
    }

    #[test]
    fn test_class_names() {
        assert_eq!(class::class_name(class::HID), "HID");
        assert_eq!(class::class_name(class::MASS_STORAGE), "Mass Storage");
        assert_eq!(class::class_name(class::HUB), "Hub");
    }

    #[test]
    fn test_vendor_names() {
        assert_eq!(vendor::name(vendor::APPLE), "Apple");
        assert_eq!(vendor::name(vendor::MICROSOFT), "Microsoft");
        assert_eq!(vendor::name(0x1234), "Unknown");
    }
}
