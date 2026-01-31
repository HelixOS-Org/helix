//! PCI bus representation.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::device::PciDevice;
use super::types::PciDeviceId;

// ============================================================================
// PCI BUS
// ============================================================================

/// PCI bus
pub struct PciBus {
    /// Bus number
    pub number: u8,
    /// Segment
    pub segment: u16,
    /// Devices on this bus
    pub devices: BTreeMap<PciDeviceId, PciDevice>,
    /// Parent bridge
    pub parent_bridge: Option<PciDeviceId>,
    /// Child buses
    pub child_buses: Vec<u8>,
    /// Secondary bus (for bridges)
    pub secondary_bus: Option<u8>,
    /// Subordinate bus (for bridges)
    pub subordinate_bus: Option<u8>,
}

impl PciBus {
    /// Create new bus
    pub fn new(number: u8, segment: u16) -> Self {
        Self {
            number,
            segment,
            devices: BTreeMap::new(),
            parent_bridge: None,
            child_buses: Vec::new(),
            secondary_bus: None,
            subordinate_bus: None,
        }
    }

    /// Add device
    pub fn add_device(&mut self, device: PciDevice) {
        self.devices.insert(device.id, device);
    }

    /// Get device
    pub fn get_device(&self, id: PciDeviceId) -> Option<&PciDevice> {
        self.devices.get(&id)
    }

    /// Device count
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }
}
