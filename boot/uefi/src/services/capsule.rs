//! Capsule Update Services
//!
//! Safe wrappers for UEFI capsule update functionality.

use super::runtime::runtime_services;
use crate::raw::types::*;

// =============================================================================
// CAPSULE
// =============================================================================

/// Capsule wrapper
pub struct Capsule {
    /// Capsule GUID
    pub guid: Guid,
    /// Capsule flags
    pub flags: CapsuleFlags,
    /// Capsule data
    pub data: alloc::vec::Vec<u8>,
}

impl Capsule {
    /// Create a new capsule
    pub fn new(guid: Guid, flags: CapsuleFlags, data: alloc::vec::Vec<u8>) -> Self {
        Self { guid, flags, data }
    }

    /// Create a firmware update capsule
    pub fn firmware_update(data: alloc::vec::Vec<u8>) -> Self {
        Self::new(
            CAPSULE_GUID_FIRMWARE_UPDATE,
            CapsuleFlags::PERSIST_ACROSS_RESET | CapsuleFlags::INITIATE_RESET,
            data,
        )
    }

    /// Get capsule size (header + data)
    pub fn size(&self) -> usize {
        core::mem::size_of::<CapsuleHeader>() + self.data.len()
    }

    /// Build capsule header
    pub fn build_header(&self) -> CapsuleHeader {
        CapsuleHeader {
            capsule_guid: self.guid,
            header_size: core::mem::size_of::<CapsuleHeader>() as u32,
            flags: self.flags.bits(),
            capsule_image_size: self.size() as u32,
        }
    }

    /// Query if capsule can be processed
    pub fn query_capabilities(&self) -> Result<CapsuleCapabilities, Status> {
        let header = self.build_header();
        let header_ptr = &header as *const CapsuleHeader as *mut CapsuleHeader;
        let headers = [header_ptr];

        let rs = unsafe { runtime_services() };
        unsafe { rs.query_capsule_capabilities(&headers) }
    }

    /// Check if capsule will require reset
    pub fn requires_reset(&self) -> bool {
        self.flags.contains(CapsuleFlags::INITIATE_RESET)
    }
}

// =============================================================================
// CAPSULE FLAGS
// =============================================================================

/// Capsule flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapsuleFlags(u32);

impl CapsuleFlags {
    /// Persist capsule across reset
    pub const PERSIST_ACROSS_RESET: Self = Self(0x00010000);

    /// Populate system table with capsule on reset
    pub const POPULATE_SYSTEM_TABLE: Self = Self(0x00020000);

    /// Initiate reset after processing
    pub const INITIATE_RESET: Self = Self(0x00040000);

    /// Empty flags
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create from raw bits
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// Get raw bits
    pub const fn bits(&self) -> u32 {
        self.0
    }

    /// Check if contains flag
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl core::ops::BitOr for CapsuleFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for CapsuleFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

// =============================================================================
// CAPSULE CAPABILITIES
// =============================================================================

pub use super::runtime::CapsuleCapabilities;

// =============================================================================
// CAPSULE GUIDS
// =============================================================================

/// Firmware management capsule GUID
pub const CAPSULE_GUID_FIRMWARE_UPDATE: Guid = Guid::new(0x6DCBD5ED, 0xE82D, 0x4C44, [
    0xBD, 0xA1, 0x71, 0x94, 0x19, 0x9A, 0xD9, 0x2A,
]);

/// Windows UX capsule GUID
pub const CAPSULE_GUID_WINDOWS_UX: Guid = Guid::new(0x3B8C8162, 0x188C, 0x46A4, [
    0xAE, 0xC9, 0xBE, 0x43, 0xF1, 0xD6, 0x56, 0x97,
]);

/// ESRT capsule GUID
pub const CAPSULE_GUID_ESRT: Guid = Guid::new(0xB122A263, 0x3661, 0x4F68, [
    0x99, 0x29, 0x78, 0xF8, 0xB0, 0xD6, 0x21, 0x80,
]);

/// JSON capsule GUID
pub const CAPSULE_GUID_JSON: Guid = Guid::new(0x67D6F4CD, 0xD6B1, 0x4D16, [
    0x92, 0x01, 0x50, 0x3F, 0x5C, 0xF8, 0x42, 0x70,
]);

// =============================================================================
// CAPSULE UPDATE
// =============================================================================

/// Submit capsule for processing
///
/// # Safety
/// This may trigger a system reset!
pub unsafe fn submit_capsule(capsule: &Capsule) -> Result<(), Status> {
    let header = capsule.build_header();

    // Allocate contiguous buffer for header + data
    let total_size = capsule.size();
    let buffer = super::memory::allocate_pool(
        crate::raw::memory::MemoryType::RuntimeServicesData,
        total_size,
    )?;

    // Copy header
    let header_bytes = core::slice::from_raw_parts(
        &header as *const CapsuleHeader as *const u8,
        core::mem::size_of::<CapsuleHeader>(),
    );
    core::ptr::copy_nonoverlapping(header_bytes.as_ptr(), buffer, header_bytes.len());

    // Copy data
    core::ptr::copy_nonoverlapping(
        capsule.data.as_ptr(),
        buffer.add(core::mem::size_of::<CapsuleHeader>()),
        capsule.data.len(),
    );

    // Create header pointer array
    let headers = [buffer as *mut CapsuleHeader];

    let rs = runtime_services();
    rs.update_capsule(&headers, PhysicalAddress(0))
}

/// Submit multiple capsules
///
/// # Safety
/// This may trigger a system reset!
pub unsafe fn submit_capsules(capsules: &[Capsule]) -> Result<(), Status> {
    if capsules.is_empty() {
        return Ok(());
    }

    let mut buffers = alloc::vec::Vec::with_capacity(capsules.len());
    let mut headers = alloc::vec::Vec::with_capacity(capsules.len());

    for capsule in capsules {
        let header = capsule.build_header();
        let total_size = capsule.size();

        let buffer = super::memory::allocate_pool(
            crate::raw::memory::MemoryType::RuntimeServicesData,
            total_size,
        )?;

        // Copy header
        let header_bytes = core::slice::from_raw_parts(
            &header as *const CapsuleHeader as *const u8,
            core::mem::size_of::<CapsuleHeader>(),
        );
        core::ptr::copy_nonoverlapping(header_bytes.as_ptr(), buffer, header_bytes.len());

        // Copy data
        core::ptr::copy_nonoverlapping(
            capsule.data.as_ptr(),
            buffer.add(core::mem::size_of::<CapsuleHeader>()),
            capsule.data.len(),
        );

        buffers.push(buffer);
        headers.push(buffer as *mut CapsuleHeader);
    }

    let rs = runtime_services();
    rs.update_capsule(&headers, PhysicalAddress(0))
}

// =============================================================================
// ESRT (EFI System Resource Table)
// =============================================================================

/// System Resource Table entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EsrtEntry {
    /// Firmware class GUID
    pub firmware_class: Guid,
    /// Firmware type
    pub firmware_type: u32,
    /// Firmware version
    pub firmware_version: u32,
    /// Lowest supported version
    pub lowest_supported_version: u32,
    /// Capsule flags
    pub capsule_flags: u32,
    /// Last attempt version
    pub last_attempt_version: u32,
    /// Last attempt status
    pub last_attempt_status: u32,
}

impl EsrtEntry {
    /// Firmware type: Unknown
    pub const TYPE_UNKNOWN: u32 = 0;
    /// Firmware type: System
    pub const TYPE_SYSTEM: u32 = 1;
    /// Firmware type: Device
    pub const TYPE_DEVICE: u32 = 2;
    /// Firmware type: UEFI Driver
    pub const TYPE_UEFI_DRIVER: u32 = 3;

    /// Status: Success
    pub const STATUS_SUCCESS: u32 = 0;
    /// Status: Error: Unsuccessful
    pub const STATUS_ERROR: u32 = 1;
    /// Status: Error: Insufficient resources
    pub const STATUS_INSUFFICIENT_RESOURCES: u32 = 2;
    /// Status: Error: Incorrect version
    pub const STATUS_INCORRECT_VERSION: u32 = 3;
    /// Status: Error: Invalid format
    pub const STATUS_INVALID_FORMAT: u32 = 4;
    /// Status: Error: Authentication error
    pub const STATUS_AUTH_ERROR: u32 = 5;
    /// Status: Error: Power event (AC not connected)
    pub const STATUS_POWER_EVENT_AC: u32 = 6;
    /// Status: Error: Power event (insufficient battery)
    pub const STATUS_POWER_EVENT_BATT: u32 = 7;
    /// Status: Error: Unsatisfied dependencies
    pub const STATUS_UNSATISFIED_DEPENDENCIES: u32 = 8;

    /// Get firmware type name
    pub fn firmware_type_name(&self) -> &'static str {
        match self.firmware_type {
            Self::TYPE_UNKNOWN => "Unknown",
            Self::TYPE_SYSTEM => "System",
            Self::TYPE_DEVICE => "Device",
            Self::TYPE_UEFI_DRIVER => "UEFI Driver",
            _ => "Unknown",
        }
    }

    /// Get last attempt status name
    pub fn last_attempt_status_name(&self) -> &'static str {
        match self.last_attempt_status {
            Self::STATUS_SUCCESS => "Success",
            Self::STATUS_ERROR => "Error",
            Self::STATUS_INSUFFICIENT_RESOURCES => "Insufficient Resources",
            Self::STATUS_INCORRECT_VERSION => "Incorrect Version",
            Self::STATUS_INVALID_FORMAT => "Invalid Format",
            Self::STATUS_AUTH_ERROR => "Authentication Error",
            Self::STATUS_POWER_EVENT_AC => "Power Event (AC)",
            Self::STATUS_POWER_EVENT_BATT => "Power Event (Battery)",
            Self::STATUS_UNSATISFIED_DEPENDENCIES => "Unsatisfied Dependencies",
            _ => "Unknown",
        }
    }
}

/// EFI System Resource Table
/// Note: Entries follow this struct
#[derive(Debug)]
#[repr(C)]
pub struct EsrtTable {
    /// Resource count
    pub resource_count: u32,
    /// Resource count max
    pub resource_count_max: u32,
    /// Resource version
    pub resource_version: u64,
}

impl EsrtTable {
    /// ESRT GUID
    pub const GUID: Guid = Guid::new(0xB122A263, 0x3661, 0x4F68, [
        0x99, 0x29, 0x78, 0xF8, 0xB0, 0xD6, 0x21, 0x80,
    ]);

    /// Get entries
    ///
    /// # Safety
    /// Table pointer must be valid
    pub unsafe fn entries(&self) -> &[EsrtEntry] {
        let ptr = (self as *const Self).add(1) as *const EsrtEntry;
        core::slice::from_raw_parts(ptr, self.resource_count as usize)
    }
}

/// Get ESRT table
pub fn get_esrt_table() -> Option<&'static EsrtTable> {
    unsafe {
        super::find_configuration_table(&EsrtTable::GUID).map(|ptr| &*(ptr as *const EsrtTable))
    }
}

// =============================================================================
// FIRMWARE UPDATE HELPER
// =============================================================================

/// Firmware update information
#[derive(Debug)]
pub struct FirmwareUpdateInfo {
    /// Firmware class GUID
    pub class: Guid,
    /// Current version
    pub current_version: u32,
    /// Lowest supported version
    pub lowest_supported: u32,
    /// Can update
    pub can_update: bool,
    /// Last update status
    pub last_status: Option<u32>,
}

/// Get firmware update information for all components
pub fn get_firmware_update_info() -> Option<alloc::vec::Vec<FirmwareUpdateInfo>> {
    let esrt = get_esrt_table()?;
    let entries = unsafe { esrt.entries() };

    let mut info = alloc::vec::Vec::with_capacity(entries.len());

    for entry in entries {
        info.push(FirmwareUpdateInfo {
            class: entry.firmware_class,
            current_version: entry.firmware_version,
            lowest_supported: entry.lowest_supported_version,
            can_update: true, // Would need to check capabilities
            last_status: if entry.last_attempt_status != 0 {
                Some(entry.last_attempt_status)
            } else {
                None
            },
        });
    }

    Some(info)
}

// =============================================================================
// EXTERN ALLOC
// =============================================================================

extern crate alloc;

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capsule_flags() {
        let flags = CapsuleFlags::PERSIST_ACROSS_RESET | CapsuleFlags::INITIATE_RESET;
        assert!(flags.contains(CapsuleFlags::PERSIST_ACROSS_RESET));
        assert!(flags.contains(CapsuleFlags::INITIATE_RESET));
        assert!(!flags.contains(CapsuleFlags::POPULATE_SYSTEM_TABLE));
    }

    #[test]
    fn test_esrt_entry_status() {
        let entry = EsrtEntry {
            firmware_class: Guid::NULL,
            firmware_type: EsrtEntry::TYPE_SYSTEM,
            firmware_version: 1,
            lowest_supported_version: 1,
            capsule_flags: 0,
            last_attempt_version: 0,
            last_attempt_status: EsrtEntry::STATUS_SUCCESS,
        };

        assert_eq!(entry.firmware_type_name(), "System");
        assert_eq!(entry.last_attempt_status_name(), "Success");
    }
}
