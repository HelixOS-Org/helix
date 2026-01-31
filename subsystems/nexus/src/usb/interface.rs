//! USB Interface and Configuration
//!
//! USB interface and configuration structures.

use alloc::string::String;
use alloc::vec::Vec;

use super::{UsbClass, UsbEndpoint};

/// USB interface
#[derive(Debug, Clone)]
pub struct UsbInterface {
    /// Interface number
    pub number: u8,
    /// Alternate setting
    pub alt_setting: u8,
    /// Class
    pub class: UsbClass,
    /// Subclass
    pub subclass: u8,
    /// Protocol
    pub protocol: u8,
    /// Endpoints
    pub endpoints: Vec<UsbEndpoint>,
    /// Driver name
    pub driver: Option<String>,
}

impl UsbInterface {
    /// Create new interface
    pub fn new(number: u8, class: UsbClass) -> Self {
        Self {
            number,
            alt_setting: 0,
            class,
            subclass: 0,
            protocol: 0,
            endpoints: Vec::new(),
            driver: None,
        }
    }

    /// Add endpoint
    pub fn add_endpoint(&mut self, endpoint: UsbEndpoint) {
        self.endpoints.push(endpoint);
    }

    /// Has driver
    pub fn has_driver(&self) -> bool {
        self.driver.is_some()
    }
}

/// USB configuration
#[derive(Debug, Clone)]
pub struct UsbConfiguration {
    /// Configuration value
    pub value: u8,
    /// Max power (mA)
    pub max_power: u16,
    /// Self powered
    pub self_powered: bool,
    /// Remote wakeup
    pub remote_wakeup: bool,
    /// Interfaces
    pub interfaces: Vec<UsbInterface>,
}

impl UsbConfiguration {
    /// Create new configuration
    pub fn new(value: u8) -> Self {
        Self {
            value,
            max_power: 100,
            self_powered: false,
            remote_wakeup: false,
            interfaces: Vec::new(),
        }
    }

    /// Add interface
    pub fn add_interface(&mut self, interface: UsbInterface) {
        self.interfaces.push(interface);
    }

    /// Interface count
    pub fn interface_count(&self) -> usize {
        self.interfaces.len()
    }
}
