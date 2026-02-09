//! Device and Driver Information
//!
//! Metadata structures for devices and drivers.

use alloc::string::String;
use alloc::vec::Vec;

use super::{BusId, BusType, ClassId, DeviceId, DeviceState, DriverId, PowerState};

/// Device information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Device ID
    pub id: DeviceId,
    /// Device name
    pub name: String,
    /// Bus type
    pub bus_type: BusType,
    /// Bus ID
    pub bus_id: BusId,
    /// Device class
    pub class_id: Option<ClassId>,
    /// Parent device
    pub parent: Option<DeviceId>,
    /// Current state
    pub state: DeviceState,
    /// Power state
    pub power_state: PowerState,
    /// Bound driver
    pub driver_id: Option<DriverId>,
    /// Vendor ID
    pub vendor_id: u32,
    /// Device specific ID
    pub device_id: u32,
    /// Subsystem vendor ID
    pub subsystem_vendor: u32,
    /// Subsystem device ID
    pub subsystem_device: u32,
    /// Device class code
    pub class_code: u32,
    /// Creation timestamp
    pub created_at: u64,
    /// Last state change
    pub state_changed_at: u64,
}

impl DeviceInfo {
    /// Create new device info
    pub fn new(id: DeviceId, name: String, bus_type: BusType, timestamp: u64) -> Self {
        Self {
            id,
            name,
            bus_type,
            bus_id: BusId::new(0),
            class_id: None,
            parent: None,
            state: DeviceState::NotInitialized,
            power_state: PowerState::D0,
            driver_id: None,
            vendor_id: 0,
            device_id: 0,
            subsystem_vendor: 0,
            subsystem_device: 0,
            class_code: 0,
            created_at: timestamp,
            state_changed_at: timestamp,
        }
    }

    /// Check if device is operational
    #[inline(always)]
    pub fn is_operational(&self) -> bool {
        matches!(self.state, DeviceState::Bound)
    }

    /// Check if device needs driver
    #[inline(always)]
    pub fn needs_driver(&self) -> bool {
        self.driver_id.is_none()
            && !matches!(self.state, DeviceState::Removed | DeviceState::Error)
    }
}

/// Driver information
#[derive(Debug, Clone)]
pub struct DriverInfo {
    /// Driver ID
    pub id: DriverId,
    /// Driver name
    pub name: String,
    /// Bus type
    pub bus_type: BusType,
    /// Supported vendor IDs
    pub vendor_ids: Vec<u32>,
    /// Supported device IDs
    pub device_ids: Vec<u32>,
    /// Supported class codes
    pub class_codes: Vec<u32>,
    /// Driver priority (higher = preferred)
    pub priority: u8,
    /// Probe time (microseconds)
    pub avg_probe_time_us: u64,
    /// Devices bound
    pub bound_count: u32,
    /// Probe failures
    pub failure_count: u32,
}

impl DriverInfo {
    /// Create new driver info
    pub fn new(id: DriverId, name: String, bus_type: BusType) -> Self {
        Self {
            id,
            name,
            bus_type,
            vendor_ids: Vec::new(),
            device_ids: Vec::new(),
            class_codes: Vec::new(),
            priority: 50,
            avg_probe_time_us: 0,
            bound_count: 0,
            failure_count: 0,
        }
    }

    /// Check if driver matches device
    pub fn matches(&self, device: &DeviceInfo) -> bool {
        if self.bus_type != device.bus_type {
            return false;
        }

        // Check vendor/device ID match
        if !self.vendor_ids.is_empty() {
            if !self.vendor_ids.contains(&device.vendor_id) {
                return false;
            }
            if !self.device_ids.is_empty() && !self.device_ids.contains(&device.device_id) {
                return false;
            }
        }

        // Check class code match
        if !self.class_codes.is_empty() && !self.class_codes.contains(&device.class_code) {
            return false;
        }

        true
    }

    /// Get probe success rate
    #[inline]
    pub fn success_rate(&self) -> f32 {
        let total = self.bound_count + self.failure_count;
        if total == 0 {
            return 1.0;
        }
        self.bound_count as f32 / total as f32
    }
}
