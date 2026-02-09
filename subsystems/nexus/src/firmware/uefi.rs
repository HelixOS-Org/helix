//! UEFI Runtime Services
//!
//! UEFI runtime services interface.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

/// UEFI variable attributes
#[derive(Debug, Clone, Copy)]
pub struct UefiVariableAttributes(pub u32);

impl UefiVariableAttributes {
    /// Non-volatile
    pub const NON_VOLATILE: u32 = 0x00000001;
    /// Boot service access
    pub const BOOT_SERVICE_ACCESS: u32 = 0x00000002;
    /// Runtime access
    pub const RUNTIME_ACCESS: u32 = 0x00000004;
    /// Hardware error record
    pub const HARDWARE_ERROR_RECORD: u32 = 0x00000008;
    /// Authenticated write access
    pub const AUTHENTICATED_WRITE_ACCESS: u32 = 0x00000010;
    /// Time-based authenticated write access
    pub const TIME_BASED_AUTHENTICATED_WRITE_ACCESS: u32 = 0x00000020;
    /// Append write
    pub const APPEND_WRITE: u32 = 0x00000040;

    /// Check if non-volatile
    #[inline(always)]
    pub fn is_non_volatile(&self) -> bool {
        self.0 & Self::NON_VOLATILE != 0
    }

    /// Check if runtime accessible
    #[inline(always)]
    pub fn is_runtime_accessible(&self) -> bool {
        self.0 & Self::RUNTIME_ACCESS != 0
    }
}

/// UEFI variable
#[derive(Debug, Clone)]
pub struct UefiVariable {
    /// Variable name
    pub name: String,
    /// Vendor GUID
    pub vendor_guid: [u8; 16],
    /// Attributes
    pub attributes: UefiVariableAttributes,
    /// Data
    pub data: Vec<u8>,
}

impl UefiVariable {
    /// Create new variable
    pub fn new(name: String, vendor_guid: [u8; 16], attributes: u32, data: Vec<u8>) -> Self {
        Self {
            name,
            vendor_guid,
            attributes: UefiVariableAttributes(attributes),
            data,
        }
    }
}

/// UEFI time
#[derive(Debug, Clone, Copy, Default)]
pub struct UefiTime {
    /// Year (1900-9999)
    pub year: u16,
    /// Month (1-12)
    pub month: u8,
    /// Day (1-31)
    pub day: u8,
    /// Hour (0-23)
    pub hour: u8,
    /// Minute (0-59)
    pub minute: u8,
    /// Second (0-59)
    pub second: u8,
    /// Nanosecond
    pub nanosecond: u32,
    /// Timezone (-1440 to 1440, or 2047 for unspecified)
    pub timezone: i16,
    /// Daylight saving time
    pub daylight: u8,
}

/// UEFI runtime services interface
pub struct UefiRuntimeServices {
    /// Available
    available: bool,
    /// Virtual address map set
    virtual_map_set: bool,
    /// Variables
    variables: BTreeMap<String, UefiVariable>,
    /// Next monotonic count
    monotonic_count: AtomicU64,
    /// Reset system available
    reset_available: bool,
    /// Get/Set variable available
    variable_services_available: bool,
    /// Time services available
    time_services_available: bool,
    /// Capsule services available
    capsule_services_available: bool,
}

impl UefiRuntimeServices {
    /// Create new UEFI runtime services
    pub fn new() -> Self {
        Self {
            available: false,
            virtual_map_set: false,
            variables: BTreeMap::new(),
            monotonic_count: AtomicU64::new(0),
            reset_available: false,
            variable_services_available: false,
            time_services_available: false,
            capsule_services_available: false,
        }
    }

    /// Initialize runtime services
    #[inline]
    pub fn initialize(&mut self) {
        self.available = true;
        self.reset_available = true;
        self.variable_services_available = true;
        self.time_services_available = true;
    }

    /// Check if available
    #[inline(always)]
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Set virtual address map
    #[inline(always)]
    pub fn set_virtual_address_map(&mut self) {
        self.virtual_map_set = true;
    }

    /// Get variable
    #[inline(always)]
    pub fn get_variable(&self, name: &str) -> Option<&UefiVariable> {
        self.variables.get(name)
    }

    /// Set variable
    #[inline(always)]
    pub fn set_variable(&mut self, variable: UefiVariable) {
        self.variables.insert(variable.name.clone(), variable);
    }

    /// Delete variable
    #[inline(always)]
    pub fn delete_variable(&mut self, name: &str) -> bool {
        self.variables.remove(name).is_some()
    }

    /// Get next monotonic count
    #[inline(always)]
    pub fn get_next_monotonic_count(&self) -> u64 {
        self.monotonic_count.fetch_add(1, Ordering::SeqCst)
    }

    /// Check reset available
    #[inline(always)]
    pub fn can_reset(&self) -> bool {
        self.available && self.reset_available
    }

    /// Check variable services available
    #[inline(always)]
    pub fn can_access_variables(&self) -> bool {
        self.available && self.variable_services_available
    }

    /// Check time services available
    #[inline(always)]
    pub fn can_access_time(&self) -> bool {
        self.available && self.time_services_available
    }

    /// Check capsule services available
    #[inline(always)]
    pub fn can_update_capsule(&self) -> bool {
        self.available && self.capsule_services_available
    }

    /// List variables
    #[inline(always)]
    pub fn list_variables(&self) -> Vec<&str> {
        self.variables.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for UefiRuntimeServices {
    fn default() -> Self {
        Self::new()
    }
}
