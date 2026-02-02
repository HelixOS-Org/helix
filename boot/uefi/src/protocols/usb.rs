//! USB Protocol
//!
//! High-level USB abstraction for USB device access.

use super::Protocol;
use crate::error::Result;
use crate::raw::types::*;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// USB DEVICE
// =============================================================================

/// High-level USB device abstraction
pub struct UsbDevice {
    /// Handle
    handle: Handle,
    /// Device descriptor
    descriptor: DeviceDescriptor,
    /// Device address
    address: u8,
    /// Speed
    speed: UsbSpeed,
    /// Configurations
    configurations: Vec<ConfigurationDescriptor>,
    /// Interfaces
    interfaces: Vec<InterfaceDescriptor>,
    /// Endpoints
    endpoints: Vec<EndpointDescriptor>,
}

impl UsbDevice {
    /// Create new USB device
    pub fn new(handle: Handle) -> Self {
        Self {
            handle,
            descriptor: DeviceDescriptor::default(),
            address: 0,
            speed: UsbSpeed::Full,
            configurations: Vec::new(),
            interfaces: Vec::new(),
            endpoints: Vec::new(),
        }
    }

    /// Get device handle
    pub fn handle(&self) -> Handle {
        self.handle
    }

    /// Get device descriptor
    pub fn descriptor(&self) -> &DeviceDescriptor {
        &self.descriptor
    }

    /// Get device address
    pub fn address(&self) -> u8 {
        self.address
    }

    /// Get speed
    pub fn speed(&self) -> UsbSpeed {
        self.speed
    }

    /// Get vendor ID
    pub fn vendor_id(&self) -> u16 {
        self.descriptor.vendor_id
    }

    /// Get product ID
    pub fn product_id(&self) -> u16 {
        self.descriptor.product_id
    }

    /// Get device class
    pub fn device_class(&self) -> UsbClass {
        UsbClass::from_code(self.descriptor.device_class)
    }

    /// Get device subclass
    pub fn device_subclass(&self) -> u8 {
        self.descriptor.device_subclass
    }

    /// Get device protocol
    pub fn device_protocol(&self) -> u8 {
        self.descriptor.device_protocol
    }

    /// Get USB version
    pub fn usb_version(&self) -> (u8, u8) {
        (
            (self.descriptor.bcd_usb >> 8) as u8,
            (self.descriptor.bcd_usb & 0xFF) as u8,
        )
    }

    /// Get device version
    pub fn device_version(&self) -> (u8, u8) {
        (
            (self.descriptor.bcd_device >> 8) as u8,
            (self.descriptor.bcd_device & 0xFF) as u8,
        )
    }

    /// Get configurations
    pub fn configurations(&self) -> &[ConfigurationDescriptor] {
        &self.configurations
    }

    /// Get interfaces
    pub fn interfaces(&self) -> &[InterfaceDescriptor] {
        &self.interfaces
    }

    /// Get endpoints
    pub fn endpoints(&self) -> &[EndpointDescriptor] {
        &self.endpoints
    }

    /// Get string descriptor
    pub fn get_string(&self, _index: u8, _language: u16) -> Result<String> {
        // TODO: Implement actual string descriptor read
        Ok(String::new())
    }

    /// Get manufacturer string
    pub fn manufacturer(&self) -> Result<String> {
        if self.descriptor.manufacturer_index != 0 {
            self.get_string(self.descriptor.manufacturer_index, 0x0409)
        } else {
            Ok(String::new())
        }
    }

    /// Get product string
    pub fn product(&self) -> Result<String> {
        if self.descriptor.product_index != 0 {
            self.get_string(self.descriptor.product_index, 0x0409)
        } else {
            Ok(String::new())
        }
    }

    /// Get serial number string
    pub fn serial_number(&self) -> Result<String> {
        if self.descriptor.serial_number_index != 0 {
            self.get_string(self.descriptor.serial_number_index, 0x0409)
        } else {
            Ok(String::new())
        }
    }

    /// Set configuration
    pub fn set_configuration(&mut self, _config_value: u8) -> Result<()> {
        // TODO: Implement actual configuration set
        Ok(())
    }

    /// Set interface alternate setting
    pub fn set_interface(&mut self, _interface: u8, _alternate: u8) -> Result<()> {
        // TODO: Implement actual interface set
        Ok(())
    }

    /// Clear feature
    pub fn clear_feature(&self, _recipient: Recipient, _feature: u16, _index: u16) -> Result<()> {
        // TODO: Implement actual clear feature
        Ok(())
    }

    /// Set feature
    pub fn set_feature(&self, _recipient: Recipient, _feature: u16, _index: u16) -> Result<()> {
        // TODO: Implement actual set feature
        Ok(())
    }

    /// Get status
    pub fn get_status(&self, _recipient: Recipient, _index: u16) -> Result<u16> {
        // TODO: Implement actual get status
        Ok(0)
    }

    /// Control transfer
    pub fn control_transfer(
        &self,
        _request_type: RequestType,
        _request: u8,
        _value: u16,
        _index: u16,
        _data: Option<&mut [u8]>,
        _timeout: u32,
    ) -> Result<usize> {
        // TODO: Implement actual control transfer
        Ok(0)
    }

    /// Bulk transfer (out)
    pub fn bulk_out(&self, _endpoint: u8, _data: &[u8], _timeout: u32) -> Result<usize> {
        // TODO: Implement actual bulk out
        Ok(0)
    }

    /// Bulk transfer (in)
    pub fn bulk_in(&self, _endpoint: u8, _data: &mut [u8], _timeout: u32) -> Result<usize> {
        // TODO: Implement actual bulk in
        Ok(0)
    }

    /// Interrupt transfer (out)
    pub fn interrupt_out(&self, _endpoint: u8, _data: &[u8], _timeout: u32) -> Result<usize> {
        // TODO: Implement actual interrupt out
        Ok(0)
    }

    /// Interrupt transfer (in)
    pub fn interrupt_in(&self, _endpoint: u8, _data: &mut [u8], _timeout: u32) -> Result<usize> {
        // TODO: Implement actual interrupt in
        Ok(0)
    }

    /// Reset device
    pub fn reset(&mut self) -> Result<()> {
        // TODO: Implement actual reset
        Ok(())
    }
}

impl Protocol for UsbDevice {
    const GUID: Guid = Guid::new(0x2B2F68D6, 0x0CD2, 0x44CF, [
        0x8E, 0x8B, 0xBB, 0xA2, 0x0B, 0x1B, 0x5B, 0x75,
    ]);

    fn open(handle: Handle) -> Result<Self> {
        Ok(Self::new(handle))
    }
}

// =============================================================================
// USB SPEED
// =============================================================================

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
}

impl UsbSpeed {
    /// Get speed in Mbps
    pub fn mbps(&self) -> u32 {
        match self {
            Self::Low => 1,
            Self::Full => 12,
            Self::High => 480,
            Self::Super => 5000,
            Self::SuperPlus => 10000,
        }
    }

    /// Get name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Low => "Low Speed (1.5 Mbps)",
            Self::Full => "Full Speed (12 Mbps)",
            Self::High => "High Speed (480 Mbps)",
            Self::Super => "Super Speed (5 Gbps)",
            Self::SuperPlus => "Super Speed+ (10 Gbps)",
        }
    }
}

// =============================================================================
// USB CLASS
// =============================================================================

/// USB device class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbClass {
    /// Per-interface class
    PerInterface,
    /// Audio
    Audio,
    /// Communications and CDC Control
    Cdc,
    /// Human Interface Device
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
    /// Audio/Video Devices
    AudioVideo,
    /// Billboard Device
    Billboard,
    /// Type-C Bridge
    TypeCBridge,
    /// Diagnostic Device
    Diagnostic,
    /// Wireless Controller
    Wireless,
    /// Miscellaneous
    Miscellaneous,
    /// Application Specific
    ApplicationSpecific,
    /// Vendor Specific
    VendorSpecific,
    /// Unknown class
    Unknown(u8),
}

impl UsbClass {
    /// Create from class code
    pub fn from_code(code: u8) -> Self {
        match code {
            0x00 => Self::PerInterface,
            0x01 => Self::Audio,
            0x02 => Self::Cdc,
            0x03 => Self::Hid,
            0x05 => Self::Physical,
            0x06 => Self::Image,
            0x07 => Self::Printer,
            0x08 => Self::MassStorage,
            0x09 => Self::Hub,
            0x0A => Self::CdcData,
            0x0B => Self::SmartCard,
            0x0D => Self::ContentSecurity,
            0x0E => Self::Video,
            0x0F => Self::PersonalHealthcare,
            0x10 => Self::AudioVideo,
            0x11 => Self::Billboard,
            0x12 => Self::TypeCBridge,
            0xDC => Self::Diagnostic,
            0xE0 => Self::Wireless,
            0xEF => Self::Miscellaneous,
            0xFE => Self::ApplicationSpecific,
            0xFF => Self::VendorSpecific,
            other => Self::Unknown(other),
        }
    }

    /// Get class code
    pub fn code(&self) -> u8 {
        match self {
            Self::PerInterface => 0x00,
            Self::Audio => 0x01,
            Self::Cdc => 0x02,
            Self::Hid => 0x03,
            Self::Physical => 0x05,
            Self::Image => 0x06,
            Self::Printer => 0x07,
            Self::MassStorage => 0x08,
            Self::Hub => 0x09,
            Self::CdcData => 0x0A,
            Self::SmartCard => 0x0B,
            Self::ContentSecurity => 0x0D,
            Self::Video => 0x0E,
            Self::PersonalHealthcare => 0x0F,
            Self::AudioVideo => 0x10,
            Self::Billboard => 0x11,
            Self::TypeCBridge => 0x12,
            Self::Diagnostic => 0xDC,
            Self::Wireless => 0xE0,
            Self::Miscellaneous => 0xEF,
            Self::ApplicationSpecific => 0xFE,
            Self::VendorSpecific => 0xFF,
            Self::Unknown(code) => *code,
        }
    }

    /// Get name
    pub fn name(&self) -> &'static str {
        match self {
            Self::PerInterface => "Per-interface",
            Self::Audio => "Audio",
            Self::Cdc => "Communications and CDC Control",
            Self::Hid => "Human Interface Device",
            Self::Physical => "Physical",
            Self::Image => "Image",
            Self::Printer => "Printer",
            Self::MassStorage => "Mass Storage",
            Self::Hub => "Hub",
            Self::CdcData => "CDC Data",
            Self::SmartCard => "Smart Card",
            Self::ContentSecurity => "Content Security",
            Self::Video => "Video",
            Self::PersonalHealthcare => "Personal Healthcare",
            Self::AudioVideo => "Audio/Video Devices",
            Self::Billboard => "Billboard Device",
            Self::TypeCBridge => "Type-C Bridge",
            Self::Diagnostic => "Diagnostic Device",
            Self::Wireless => "Wireless Controller",
            Self::Miscellaneous => "Miscellaneous",
            Self::ApplicationSpecific => "Application Specific",
            Self::VendorSpecific => "Vendor Specific",
            Self::Unknown(_) => "Unknown",
        }
    }
}

// =============================================================================
// DEVICE DESCRIPTOR
// =============================================================================

/// USB device descriptor
#[derive(Debug, Clone, Default)]
pub struct DeviceDescriptor {
    /// USB specification version
    pub bcd_usb: u16,
    /// Device class code
    pub device_class: u8,
    /// Device subclass code
    pub device_subclass: u8,
    /// Device protocol code
    pub device_protocol: u8,
    /// Maximum packet size for endpoint 0
    pub max_packet_size0: u8,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
    /// Device release number
    pub bcd_device: u16,
    /// Manufacturer string index
    pub manufacturer_index: u8,
    /// Product string index
    pub product_index: u8,
    /// Serial number string index
    pub serial_number_index: u8,
    /// Number of configurations
    pub num_configurations: u8,
}

impl DeviceDescriptor {
    /// Parse from bytes
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 18 {
            return None;
        }

        if data[0] != 18 || data[1] != 1 {
            return None;
        }

        Some(Self {
            bcd_usb: u16::from_le_bytes([data[2], data[3]]),
            device_class: data[4],
            device_subclass: data[5],
            device_protocol: data[6],
            max_packet_size0: data[7],
            vendor_id: u16::from_le_bytes([data[8], data[9]]),
            product_id: u16::from_le_bytes([data[10], data[11]]),
            bcd_device: u16::from_le_bytes([data[12], data[13]]),
            manufacturer_index: data[14],
            product_index: data[15],
            serial_number_index: data[16],
            num_configurations: data[17],
        })
    }
}

// =============================================================================
// CONFIGURATION DESCRIPTOR
// =============================================================================

/// USB configuration descriptor
#[derive(Debug, Clone, Default)]
pub struct ConfigurationDescriptor {
    /// Total length of data returned
    pub total_length: u16,
    /// Number of interfaces
    pub num_interfaces: u8,
    /// Configuration value
    pub configuration_value: u8,
    /// Configuration string index
    pub configuration_index: u8,
    /// Attributes
    pub attributes: ConfigurationAttributes,
    /// Maximum power (in 2mA units)
    pub max_power: u8,
}

impl ConfigurationDescriptor {
    /// Parse from bytes
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 9 {
            return None;
        }

        if data[0] != 9 || data[1] != 2 {
            return None;
        }

        Some(Self {
            total_length: u16::from_le_bytes([data[2], data[3]]),
            num_interfaces: data[4],
            configuration_value: data[5],
            configuration_index: data[6],
            attributes: ConfigurationAttributes(data[7]),
            max_power: data[8],
        })
    }

    /// Get maximum power in mA
    pub fn max_power_ma(&self) -> u16 {
        self.max_power as u16 * 2
    }
}

/// Configuration attributes
#[derive(Debug, Clone, Copy, Default)]
pub struct ConfigurationAttributes(pub u8);

impl ConfigurationAttributes {
    /// Check if self-powered
    pub fn self_powered(&self) -> bool {
        (self.0 & 0x40) != 0
    }

    /// Check if remote wakeup supported
    pub fn remote_wakeup(&self) -> bool {
        (self.0 & 0x20) != 0
    }
}

// =============================================================================
// INTERFACE DESCRIPTOR
// =============================================================================

/// USB interface descriptor
#[derive(Debug, Clone, Default)]
pub struct InterfaceDescriptor {
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
    /// Parse from bytes
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 9 {
            return None;
        }

        if data[0] != 9 || data[1] != 4 {
            return None;
        }

        Some(Self {
            interface_number: data[2],
            alternate_setting: data[3],
            num_endpoints: data[4],
            interface_class: data[5],
            interface_subclass: data[6],
            interface_protocol: data[7],
            interface_index: data[8],
        })
    }

    /// Get interface class
    pub fn class(&self) -> UsbClass {
        UsbClass::from_code(self.interface_class)
    }
}

// =============================================================================
// ENDPOINT DESCRIPTOR
// =============================================================================

/// USB endpoint descriptor
#[derive(Debug, Clone, Default)]
pub struct EndpointDescriptor {
    /// Endpoint address
    pub endpoint_address: u8,
    /// Attributes
    pub attributes: EndpointAttributes,
    /// Maximum packet size
    pub max_packet_size: u16,
    /// Interval
    pub interval: u8,
}

impl EndpointDescriptor {
    /// Parse from bytes
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 7 {
            return None;
        }

        if data[0] != 7 || data[1] != 5 {
            return None;
        }

        Some(Self {
            endpoint_address: data[2],
            attributes: EndpointAttributes(data[3]),
            max_packet_size: u16::from_le_bytes([data[4], data[5]]),
            interval: data[6],
        })
    }

    /// Get endpoint number (0-15)
    pub fn endpoint_number(&self) -> u8 {
        self.endpoint_address & 0x0F
    }

    /// Check if IN endpoint
    pub fn is_in(&self) -> bool {
        (self.endpoint_address & 0x80) != 0
    }

    /// Check if OUT endpoint
    pub fn is_out(&self) -> bool {
        (self.endpoint_address & 0x80) == 0
    }

    /// Get direction
    pub fn direction(&self) -> EndpointDirection {
        if self.is_in() {
            EndpointDirection::In
        } else {
            EndpointDirection::Out
        }
    }

    /// Get transfer type
    pub fn transfer_type(&self) -> TransferType {
        self.attributes.transfer_type()
    }
}

/// Endpoint attributes
#[derive(Debug, Clone, Copy, Default)]
pub struct EndpointAttributes(pub u8);

impl EndpointAttributes {
    /// Get transfer type
    pub fn transfer_type(&self) -> TransferType {
        match self.0 & 0x03 {
            0 => TransferType::Control,
            1 => TransferType::Isochronous,
            2 => TransferType::Bulk,
            3 => TransferType::Interrupt,
            _ => unreachable!(),
        }
    }

    /// Get synchronization type (for isochronous)
    pub fn sync_type(&self) -> SyncType {
        match (self.0 >> 2) & 0x03 {
            0 => SyncType::None,
            1 => SyncType::Asynchronous,
            2 => SyncType::Adaptive,
            3 => SyncType::Synchronous,
            _ => unreachable!(),
        }
    }

    /// Get usage type (for isochronous)
    pub fn usage_type(&self) -> UsageType {
        match (self.0 >> 4) & 0x03 {
            0 => UsageType::Data,
            1 => UsageType::Feedback,
            2 => UsageType::ImplicitFeedback,
            _ => UsageType::Reserved,
        }
    }
}

/// Endpoint direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointDirection {
    /// OUT (host to device)
    Out,
    /// IN (device to host)
    In,
}

/// Transfer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferType {
    /// Control transfer
    Control,
    /// Isochronous transfer
    Isochronous,
    /// Bulk transfer
    Bulk,
    /// Interrupt transfer
    Interrupt,
}

/// Synchronization type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncType {
    /// No synchronization
    None,
    /// Asynchronous
    Asynchronous,
    /// Adaptive
    Adaptive,
    /// Synchronous
    Synchronous,
}

/// Usage type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageType {
    /// Data endpoint
    Data,
    /// Feedback endpoint
    Feedback,
    /// Implicit feedback data endpoint
    ImplicitFeedback,
    /// Reserved
    Reserved,
}

// =============================================================================
// REQUEST TYPES
// =============================================================================

/// Request type
#[derive(Debug, Clone, Copy)]
pub struct RequestType(pub u8);

impl RequestType {
    /// Create request type
    pub const fn new(
        direction: Direction,
        request_type: RequestKind,
        recipient: Recipient,
    ) -> Self {
        Self((direction as u8) << 7 | (request_type as u8) << 5 | (recipient as u8))
    }

    /// Get direction
    pub fn direction(&self) -> Direction {
        if (self.0 & 0x80) != 0 {
            Direction::In
        } else {
            Direction::Out
        }
    }

    /// Get request type
    pub fn request_type(&self) -> RequestKind {
        match (self.0 >> 5) & 0x03 {
            0 => RequestKind::Standard,
            1 => RequestKind::Class,
            2 => RequestKind::Vendor,
            _ => RequestKind::Reserved,
        }
    }

    /// Get recipient
    pub fn recipient(&self) -> Recipient {
        match self.0 & 0x1F {
            0 => Recipient::Device,
            1 => Recipient::Interface,
            2 => Recipient::Endpoint,
            3 => Recipient::Other,
            _ => Recipient::Reserved,
        }
    }
}

/// Direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Direction {
    /// Host to device
    Out = 0,
    /// Device to host
    In  = 1,
}

/// Request kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RequestKind {
    /// Standard request
    Standard = 0,
    /// Class-specific request
    Class    = 1,
    /// Vendor-specific request
    Vendor   = 2,
    /// Reserved
    Reserved = 3,
}

/// Recipient
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Recipient {
    /// Device
    Device    = 0,
    /// Interface
    Interface = 1,
    /// Endpoint
    Endpoint  = 2,
    /// Other
    Other     = 3,
    /// Reserved
    Reserved  = 4,
}

// =============================================================================
// STANDARD REQUESTS
// =============================================================================

/// Standard USB requests
pub mod requests {
    /// Get status
    pub const GET_STATUS: u8 = 0x00;
    /// Clear feature
    pub const CLEAR_FEATURE: u8 = 0x01;
    /// Set feature
    pub const SET_FEATURE: u8 = 0x03;
    /// Set address
    pub const SET_ADDRESS: u8 = 0x05;
    /// Get descriptor
    pub const GET_DESCRIPTOR: u8 = 0x06;
    /// Set descriptor
    pub const SET_DESCRIPTOR: u8 = 0x07;
    /// Get configuration
    pub const GET_CONFIGURATION: u8 = 0x08;
    /// Set configuration
    pub const SET_CONFIGURATION: u8 = 0x09;
    /// Get interface
    pub const GET_INTERFACE: u8 = 0x0A;
    /// Set interface
    pub const SET_INTERFACE: u8 = 0x0B;
    /// Synch frame
    pub const SYNCH_FRAME: u8 = 0x0C;
}

/// Descriptor types
pub mod descriptor_types {
    /// Device descriptor
    pub const DEVICE: u8 = 0x01;
    /// Configuration descriptor
    pub const CONFIGURATION: u8 = 0x02;
    /// String descriptor
    pub const STRING: u8 = 0x03;
    /// Interface descriptor
    pub const INTERFACE: u8 = 0x04;
    /// Endpoint descriptor
    pub const ENDPOINT: u8 = 0x05;
    /// Device qualifier
    pub const DEVICE_QUALIFIER: u8 = 0x06;
    /// Other speed configuration
    pub const OTHER_SPEED_CONFIGURATION: u8 = 0x07;
    /// Interface power
    pub const INTERFACE_POWER: u8 = 0x08;
    /// OTG
    pub const OTG: u8 = 0x09;
    /// Debug
    pub const DEBUG: u8 = 0x0A;
    /// Interface association
    pub const INTERFACE_ASSOCIATION: u8 = 0x0B;
    /// BOS
    pub const BOS: u8 = 0x0F;
    /// Device capability
    pub const DEVICE_CAPABILITY: u8 = 0x10;
    /// HID
    pub const HID: u8 = 0x21;
    /// Report
    pub const REPORT: u8 = 0x22;
    /// Physical
    pub const PHYSICAL: u8 = 0x23;
    /// Hub
    pub const HUB: u8 = 0x29;
    /// SuperSpeed Hub
    pub const SS_HUB: u8 = 0x2A;
    /// SuperSpeed Endpoint Companion
    pub const SS_ENDPOINT_COMPANION: u8 = 0x30;
}

// =============================================================================
// USB HUB
// =============================================================================

/// USB hub
pub struct UsbHub {
    /// Underlying device
    device: UsbDevice,
    /// Hub descriptor
    descriptor: HubDescriptor,
}

impl UsbHub {
    /// Create from device
    pub fn new(device: UsbDevice) -> Result<Self> {
        Ok(Self {
            device,
            descriptor: HubDescriptor::default(),
        })
    }

    /// Get number of ports
    pub fn num_ports(&self) -> u8 {
        self.descriptor.num_ports
    }

    /// Get hub characteristics
    pub fn characteristics(&self) -> u16 {
        self.descriptor.hub_characteristics
    }

    /// Check if port is powered
    pub fn is_port_powered(&self, _port: u8) -> Result<bool> {
        // TODO: Implement actual port status
        Ok(true)
    }

    /// Check if port is connected
    pub fn is_port_connected(&self, _port: u8) -> Result<bool> {
        // TODO: Implement actual port status
        Ok(false)
    }

    /// Check if port is enabled
    pub fn is_port_enabled(&self, _port: u8) -> Result<bool> {
        // TODO: Implement actual port status
        Ok(false)
    }

    /// Power on port
    pub fn power_on_port(&self, _port: u8) -> Result<()> {
        // TODO: Implement actual port control
        Ok(())
    }

    /// Power off port
    pub fn power_off_port(&self, _port: u8) -> Result<()> {
        // TODO: Implement actual port control
        Ok(())
    }

    /// Reset port
    pub fn reset_port(&self, _port: u8) -> Result<()> {
        // TODO: Implement actual port control
        Ok(())
    }
}

/// Hub descriptor
#[derive(Debug, Clone, Default)]
pub struct HubDescriptor {
    /// Number of ports
    pub num_ports: u8,
    /// Hub characteristics
    pub hub_characteristics: u16,
    /// Power on to power good time (in 2ms units)
    pub power_on_delay: u8,
    /// Maximum current (in mA)
    pub max_current: u8,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usb_class() {
        assert_eq!(UsbClass::from_code(0x08), UsbClass::MassStorage);
        assert_eq!(UsbClass::MassStorage.code(), 0x08);
        assert_eq!(UsbClass::MassStorage.name(), "Mass Storage");
    }

    #[test]
    fn test_usb_speed() {
        assert_eq!(UsbSpeed::High.mbps(), 480);
        assert_eq!(UsbSpeed::Super.mbps(), 5000);
    }

    #[test]
    fn test_device_descriptor_parse() {
        let data = [
            18, 1, // bLength, bDescriptorType
            0x10, 0x02, // bcdUSB (2.10)
            0x00, // bDeviceClass
            0x00, // bDeviceSubClass
            0x00, // bDeviceProtocol
            64,   // bMaxPacketSize0
            0xD8, 0x04, // idVendor
            0x01, 0x00, // idProduct
            0x00, 0x01, // bcdDevice
            1, 2, 3, // iManufacturer, iProduct, iSerialNumber
            1, // bNumConfigurations
        ];

        let desc = DeviceDescriptor::parse(&data).unwrap();
        assert_eq!(desc.vendor_id, 0x04D8);
        assert_eq!(desc.product_id, 0x0001);
        assert_eq!(desc.max_packet_size0, 64);
    }

    #[test]
    fn test_endpoint_direction() {
        let ep_in = EndpointDescriptor {
            endpoint_address: 0x81,
            ..Default::default()
        };
        assert!(ep_in.is_in());
        assert!(!ep_in.is_out());

        let ep_out = EndpointDescriptor {
            endpoint_address: 0x02,
            ..Default::default()
        };
        assert!(ep_out.is_out());
        assert!(!ep_out.is_in());
    }

    #[test]
    fn test_request_type() {
        let rt = RequestType::new(Direction::In, RequestKind::Standard, Recipient::Device);
        assert_eq!(rt.direction(), Direction::In);
        assert_eq!(rt.request_type(), RequestKind::Standard);
        assert_eq!(rt.recipient(), Recipient::Device);
    }
}
