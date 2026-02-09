//! USB Bus
//!
//! USB bus and controller management.

use alloc::collections::BTreeMap;
use alloc::string::String;

use super::{BusId, DeviceAddress, UsbDevice, UsbDeviceId, UsbHub, UsbSpeed};

/// USB bus type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbBusType {
    /// UHCI (USB 1.x)
    Uhci,
    /// OHCI (USB 1.x)
    Ohci,
    /// EHCI (USB 2.0)
    Ehci,
    /// xHCI (USB 3.x)
    Xhci,
    /// Virtual
    Virtual,
    /// Unknown
    Unknown,
}

impl UsbBusType {
    /// Get type name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Uhci => "uhci",
            Self::Ohci => "ohci",
            Self::Ehci => "ehci",
            Self::Xhci => "xhci",
            Self::Virtual => "virtual",
            Self::Unknown => "unknown",
        }
    }

    /// Max speed
    #[inline]
    pub fn max_speed(&self) -> UsbSpeed {
        match self {
            Self::Uhci | Self::Ohci => UsbSpeed::Full,
            Self::Ehci => UsbSpeed::High,
            Self::Xhci => UsbSpeed::Usb4,
            Self::Virtual => UsbSpeed::High,
            Self::Unknown => UsbSpeed::Unknown,
        }
    }
}

/// USB bus
pub struct UsbBus {
    /// Bus ID
    pub id: BusId,
    /// Bus type
    pub bus_type: UsbBusType,
    /// Root hub
    pub root_hub: Option<UsbDeviceId>,
    /// Devices on this bus
    pub devices: BTreeMap<DeviceAddress, UsbDevice>,
    /// Hubs
    pub hubs: BTreeMap<UsbDeviceId, UsbHub>,
    /// Controller vendor
    pub controller_vendor: Option<String>,
}

impl UsbBus {
    /// Create new bus
    pub fn new(id: BusId, bus_type: UsbBusType) -> Self {
        Self {
            id,
            bus_type,
            root_hub: None,
            devices: BTreeMap::new(),
            hubs: BTreeMap::new(),
            controller_vendor: None,
        }
    }

    /// Add device
    pub fn add_device(&mut self, device: UsbDevice) {
        let is_hub = device.is_hub;
        let id = device.id;
        let hub_ports = device.hub_ports;

        self.devices.insert(device.id.address, device);

        if is_hub {
            if let Some(ports) = hub_ports {
                self.hubs.insert(id, UsbHub::new(id, ports));
            }
        }
    }

    /// Get device
    #[inline(always)]
    pub fn get_device(&self, address: DeviceAddress) -> Option<&UsbDevice> {
        self.devices.get(&address)
    }

    /// Device count
    #[inline(always)]
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Max speed
    #[inline(always)]
    pub fn max_speed(&self) -> UsbSpeed {
        self.bus_type.max_speed()
    }
}
