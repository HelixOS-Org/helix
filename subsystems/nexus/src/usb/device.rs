//! USB Device
//!
//! USB device structure.

use alloc::string::String;
use alloc::vec::Vec;

use super::{
    UsbClass, UsbConfiguration, UsbDeviceId, UsbDeviceState, UsbProductId, UsbSpeed, UsbVendorId,
};

/// USB device
#[derive(Debug)]
pub struct UsbDevice {
    /// Device ID
    pub id: UsbDeviceId,
    /// Vendor ID
    pub vendor: UsbVendorId,
    /// Product ID
    pub product: UsbProductId,
    /// Device class
    pub class: UsbClass,
    /// Device subclass
    pub subclass: u8,
    /// Device protocol
    pub protocol: u8,
    /// Speed
    pub speed: UsbSpeed,
    /// State
    pub state: UsbDeviceState,
    /// USB version (BCD)
    pub usb_version: u16,
    /// Device version (BCD)
    pub device_version: u16,
    /// Manufacturer string
    pub manufacturer: Option<String>,
    /// Product string
    pub product_name: Option<String>,
    /// Serial number
    pub serial: Option<String>,
    /// Configurations
    pub configurations: Vec<UsbConfiguration>,
    /// Active configuration
    pub active_config: Option<u8>,
    /// Parent hub
    pub parent_hub: Option<UsbDeviceId>,
    /// Port on parent hub
    pub port: Option<u8>,
    /// Is hub
    pub is_hub: bool,
    /// Hub ports (if hub)
    pub hub_ports: Option<u8>,
    /// Connected timestamp
    pub connected_at: u64,
    /// Driver bound
    pub has_driver: bool,
    /// Wakeup capable
    pub wakeup_capable: bool,
    /// Wakeup enabled
    pub wakeup_enabled: bool,
}

impl UsbDevice {
    /// Create new device
    pub fn new(
        id: UsbDeviceId,
        vendor: UsbVendorId,
        product: UsbProductId,
        speed: UsbSpeed,
    ) -> Self {
        Self {
            id,
            vendor,
            product,
            class: UsbClass::Unknown,
            subclass: 0,
            protocol: 0,
            speed,
            state: UsbDeviceState::Attached,
            usb_version: 0x0200,
            device_version: 0,
            manufacturer: None,
            product_name: None,
            serial: None,
            configurations: Vec::new(),
            active_config: None,
            parent_hub: None,
            port: None,
            is_hub: false,
            hub_ports: None,
            connected_at: 0,
            has_driver: false,
            wakeup_capable: false,
            wakeup_enabled: false,
        }
    }

    /// Get total endpoints
    #[inline]
    pub fn total_endpoints(&self) -> usize {
        self.configurations
            .iter()
            .flat_map(|c| c.interfaces.iter())
            .map(|i| i.endpoints.len())
            .sum()
    }

    /// Get max power (mA)
    #[inline]
    pub fn max_power(&self) -> u16 {
        self.active_config
            .and_then(|c| self.configurations.iter().find(|cfg| cfg.value == c))
            .map(|cfg| cfg.max_power)
            .unwrap_or(100)
    }

    /// Is high speed or above
    #[inline]
    pub fn is_high_speed_plus(&self) -> bool {
        matches!(
            self.speed,
            UsbSpeed::High
                | UsbSpeed::Super
                | UsbSpeed::SuperPlus
                | UsbSpeed::SuperPlusX2
                | UsbSpeed::Usb4
        )
    }

    /// Is super speed
    #[inline]
    pub fn is_super_speed(&self) -> bool {
        matches!(
            self.speed,
            UsbSpeed::Super | UsbSpeed::SuperPlus | UsbSpeed::SuperPlusX2 | UsbSpeed::Usb4
        )
    }
}
