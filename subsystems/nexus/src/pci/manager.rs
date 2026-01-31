//! PCI device and bus management.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use super::bus::PciBus;
use super::device::PciDevice;
use super::types::{PciDeviceId, PciDeviceType, VendorId};

// ============================================================================
// PCI MANAGER
// ============================================================================

/// PCI manager
pub struct PciManager {
    /// Buses
    buses: BTreeMap<u8, PciBus>,
    /// All devices (flat)
    pub(crate) all_devices: BTreeMap<PciDeviceId, PciDevice>,
    /// Device count
    device_count: AtomicU32,
    /// Enabled
    enabled: AtomicBool,
}

impl PciManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            buses: BTreeMap::new(),
            all_devices: BTreeMap::new(),
            device_count: AtomicU32::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Register bus
    pub fn register_bus(&mut self, bus: PciBus) {
        self.buses.insert(bus.number, bus);
    }

    /// Register device
    pub fn register_device(&mut self, device: PciDevice) {
        self.device_count.fetch_add(1, Ordering::Relaxed);

        let bus_num = device.id.bus;
        self.all_devices.insert(device.id, device);

        // Also add to bus if exists
        if let Some(bus) = self.buses.get_mut(&bus_num) {
            if let Some(dev) = self.all_devices.get(&PciDeviceId::from_bdf(
                bus_num,
                self.all_devices
                    .keys()
                    .filter(|k| k.bus == bus_num)
                    .last()
                    .map(|k| k.device)
                    .unwrap_or(0),
                0,
            )) {
                // Device already tracked in all_devices
                let _ = dev;
            }
        }
    }

    /// Get device
    pub fn get_device(&self, id: PciDeviceId) -> Option<&PciDevice> {
        self.all_devices.get(&id)
    }

    /// Get device mutably
    pub fn get_device_mut(&mut self, id: PciDeviceId) -> Option<&mut PciDevice> {
        self.all_devices.get_mut(&id)
    }

    /// Get devices by vendor
    pub fn devices_by_vendor(&self, vendor: VendorId) -> Vec<&PciDevice> {
        self.all_devices
            .values()
            .filter(|d| d.vendor == vendor)
            .collect()
    }

    /// Get devices by class
    pub fn devices_by_class(&self, class: u8) -> Vec<&PciDevice> {
        self.all_devices
            .values()
            .filter(|d| d.class.class == class)
            .collect()
    }

    /// Get bridges
    pub fn bridges(&self) -> Vec<&PciDevice> {
        self.all_devices
            .values()
            .filter(|d| d.device_type.is_bridge())
            .collect()
    }

    /// Get endpoints
    pub fn endpoints(&self) -> Vec<&PciDevice> {
        self.all_devices
            .values()
            .filter(|d| matches!(d.device_type, PciDeviceType::Endpoint))
            .collect()
    }

    /// Get device count
    pub fn device_count(&self) -> u32 {
        self.device_count.load(Ordering::Relaxed)
    }

    /// Get bus count
    pub fn bus_count(&self) -> usize {
        self.buses.len()
    }
}

impl Default for PciManager {
    fn default() -> Self {
        Self::new()
    }
}
