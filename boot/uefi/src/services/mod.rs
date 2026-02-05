//! Safe UEFI Services
//!
//! This module provides safe, high-level wrappers around UEFI Boot Services
//! and Runtime Services.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                         Safe Services Layer                         │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐   │
//! │  │   Memory    │ │   Events    │ │  Protocols  │ │   Images    │   │
//! │  │  Services   │ │  & Timers   │ │   Locate    │ │  Loading    │   │
//! │  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘   │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐   │
//! │  │   Runtime   │ │  Variables  │ │    Time     │ │   Capsule   │   │
//! │  │  Services   │ │   Access    │ │  Services   │ │   Update    │   │
//! │  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘   │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

pub mod boot;
pub mod capsule;
pub mod events;
pub mod memory;
pub mod protocols;
pub mod runtime;
pub mod time;
pub mod tpl;
pub mod variables;
pub mod watchdog;

use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

// Explicit re-exports to avoid ambiguity
// boot.rs exports TPL_* constants that conflict with tpl.rs Tpl struct
// We prefer tpl.rs's Tpl struct over the raw constants
pub use boot::{BootServices, EventNotify, EventType, HandleBuffer, MemoryMapInfo, ProtocolBuffer};
pub use capsule::*;
pub use events::*;
pub use memory::*;
pub use protocols::*;
// runtime.rs exports VariableStorageInfo that conflicts with variables.rs
// We prefer variables.rs's version which has additional methods
pub use runtime::{CapsuleCapabilities, RuntimeServices};
pub use time::*;
pub use tpl::*;
pub use variables::*;
pub use watchdog::*;

use crate::raw::boot_services::EfiBootServices;
use crate::raw::runtime_services::EfiRuntimeServices;
use crate::raw::system_table::EfiSystemTable;
use crate::raw::types::*;

// =============================================================================
// GLOBAL STATE
// =============================================================================

/// Global system table pointer
static SYSTEM_TABLE: AtomicPtr<EfiSystemTable> = AtomicPtr::new(core::ptr::null_mut());

/// Global image handle
static IMAGE_HANDLE: AtomicPtr<core::ffi::c_void> = AtomicPtr::new(core::ptr::null_mut());

/// Whether boot services have exited
static BOOT_SERVICES_EXITED: AtomicBool = AtomicBool::new(false);

// =============================================================================
// SERVICES INITIALIZATION
// =============================================================================

/// Initialize the UEFI services layer
///
/// # Safety
/// Must be called exactly once at UEFI entry point with valid pointers.
pub unsafe fn initialize(
    image_handle: Handle,
    system_table: *mut EfiSystemTable,
) -> Result<(), Status> {
    if system_table.is_null() {
        return Err(Status::INVALID_PARAMETER);
    }

    // Validate system table signature
    let st = &*system_table;
    if st.hdr.signature != EfiSystemTable::SIGNATURE {
        return Err(Status::UNSUPPORTED);
    }

    // Store global pointers
    SYSTEM_TABLE.store(system_table, Ordering::Release);
    IMAGE_HANDLE.store(image_handle.as_ptr(), Ordering::Release);

    Ok(())
}

/// Check if services are initialized
pub fn is_initialized() -> bool {
    !SYSTEM_TABLE.load(Ordering::Acquire).is_null()
}

/// Check if boot services are still available
pub fn boot_services_available() -> bool {
    is_initialized() && !BOOT_SERVICES_EXITED.load(Ordering::Acquire)
}

/// Get the image handle
pub fn image_handle() -> Option<Handle> {
    let ptr = IMAGE_HANDLE.load(Ordering::Acquire);
    if ptr.is_null() {
        None
    } else {
        Some(Handle::new(ptr))
    }
}

// =============================================================================
// SYSTEM TABLE ACCESS
// =============================================================================

/// Get a reference to the system table
///
/// # Safety
/// Must only be called after initialize() succeeds.
pub unsafe fn system_table() -> &'static EfiSystemTable {
    let ptr = SYSTEM_TABLE.load(Ordering::Acquire);
    assert!(!ptr.is_null(), "UEFI services not initialized");
    &*ptr
}

/// Get a mutable reference to the system table
///
/// # Safety
/// Must only be called after initialize() succeeds.
/// Caller must ensure exclusive access.
pub unsafe fn system_table_mut() -> &'static mut EfiSystemTable {
    let ptr = SYSTEM_TABLE.load(Ordering::Acquire);
    assert!(!ptr.is_null(), "UEFI services not initialized");
    &mut *ptr
}

/// Get boot services
///
/// # Safety
/// Must only be called after initialize() and before ExitBootServices.
pub unsafe fn boot_services() -> &'static EfiBootServices {
    assert!(boot_services_available(), "Boot services not available");
    let st = system_table();
    assert!(!st.boot_services.is_null(), "Boot services pointer is null");
    &*st.boot_services
}

/// Get runtime services
///
/// # Safety
/// Must only be called after initialize().
pub unsafe fn runtime_services() -> &'static EfiRuntimeServices {
    let st = system_table();
    assert!(
        !st.runtime_services.is_null(),
        "Runtime services pointer is null"
    );
    &*st.runtime_services
}

// =============================================================================
// CONSOLE ACCESS
// =============================================================================

/// Get console output
///
/// # Safety
/// Must only be called after initialize().
pub unsafe fn console_out(
) -> Option<&'static mut crate::raw::system_table::EfiSimpleTextOutputProtocol> {
    let st = system_table_mut();
    if st.con_out.is_null() {
        None
    } else {
        Some(&mut *st.con_out)
    }
}

/// Get console input
///
/// # Safety
/// Must only be called after initialize().
pub unsafe fn console_in(
) -> Option<&'static mut crate::raw::system_table::EfiSimpleTextInputProtocol> {
    let st = system_table_mut();
    if st.con_in.is_null() {
        None
    } else {
        Some(&mut *st.con_in)
    }
}

/// Get standard error output
///
/// # Safety
/// Must only be called after initialize().
pub unsafe fn std_err() -> Option<&'static mut crate::raw::system_table::EfiSimpleTextOutputProtocol>
{
    let st = system_table_mut();
    if st.std_err.is_null() {
        None
    } else {
        Some(&mut *st.std_err)
    }
}

// =============================================================================
// FIRMWARE INFO
// =============================================================================

/// Firmware information
#[derive(Debug, Clone)]
pub struct FirmwareInfo {
    /// Vendor name
    pub vendor: Option<alloc::string::String>,
    /// Firmware revision
    pub revision: u32,
    /// UEFI specification revision
    pub uefi_revision: UefiRevision,
}

/// UEFI specification revision
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UefiRevision {
    /// Major version
    pub major: u16,
    /// Minor version
    pub minor: u16,
}

impl UefiRevision {
    /// UEFI 2.0
    pub const V2_0: Self = Self { major: 2, minor: 0 };
    /// UEFI 2.1
    pub const V2_1: Self = Self {
        major: 2,
        minor: 10,
    };
    /// UEFI 2.2
    pub const V2_2: Self = Self {
        major: 2,
        minor: 20,
    };
    /// UEFI 2.3
    pub const V2_3: Self = Self {
        major: 2,
        minor: 30,
    };
    /// UEFI 2.3.1
    pub const V2_3_1: Self = Self {
        major: 2,
        minor: 31,
    };
    /// UEFI 2.4
    pub const V2_4: Self = Self {
        major: 2,
        minor: 40,
    };
    /// UEFI 2.5
    pub const V2_5: Self = Self {
        major: 2,
        minor: 50,
    };
    /// UEFI 2.6
    pub const V2_6: Self = Self {
        major: 2,
        minor: 60,
    };
    /// UEFI 2.7
    pub const V2_7: Self = Self {
        major: 2,
        minor: 70,
    };
    /// UEFI 2.8
    pub const V2_8: Self = Self {
        major: 2,
        minor: 80,
    };
    /// UEFI 2.9
    pub const V2_9: Self = Self {
        major: 2,
        minor: 90,
    };
    /// UEFI 2.10
    pub const V2_10: Self = Self {
        major: 2,
        minor: 100,
    };

    /// Create from packed revision
    pub fn from_packed(revision: u32) -> Self {
        Self {
            major: (revision >> 16) as u16,
            minor: (revision & 0xFFFF) as u16,
        }
    }

    /// Pack into u32
    pub fn to_packed(self) -> u32 {
        ((self.major as u32) << 16) | (self.minor as u32)
    }
}

impl core::fmt::Display for UefiRevision {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.minor % 10 == 0 {
            write!(f, "{}.{}", self.major, self.minor / 10)
        } else {
            write!(f, "{}.{}.{}", self.major, self.minor / 10, self.minor % 10)
        }
    }
}

/// Get firmware information
///
/// # Safety
/// Must only be called after initialize().
pub unsafe fn firmware_info() -> FirmwareInfo {
    let st = system_table();

    FirmwareInfo {
        vendor: None, // TODO: implement UTF-16 conversion
        revision: st.firmware_revision,
        uefi_revision: UefiRevision::from_packed(st.hdr.revision),
    }
}

// =============================================================================
// CONFIGURATION TABLE ACCESS
// =============================================================================

use crate::raw::types::ConfigurationTable;

/// Get configuration tables
///
/// # Safety
/// Must only be called after initialize().
pub unsafe fn configuration_tables() -> &'static [ConfigurationTable] {
    let st = system_table();
    if st.configuration_table.is_null() || st.number_of_table_entries == 0 {
        &[]
    } else {
        core::slice::from_raw_parts(st.configuration_table, st.number_of_table_entries)
    }
}

/// Find a configuration table by GUID
///
/// # Safety
/// Must only be called after initialize().
pub unsafe fn find_configuration_table(guid: &Guid) -> Option<*const core::ffi::c_void> {
    for table in configuration_tables() {
        if table.vendor_guid == *guid {
            return Some(table.vendor_table);
        }
    }
    None
}

// =============================================================================
// EXIT BOOT SERVICES
// =============================================================================

/// Exit boot services
///
/// This is the point of no return. After calling this:
/// - Boot services are no longer available
/// - Only runtime services can be used
/// - You own all memory and hardware
///
/// # Safety
/// This is extremely dangerous. Caller must:
/// - Have obtained memory map with matching key
/// - Be prepared to take over the system
pub unsafe fn exit_boot_services(
    memory_map_key: crate::raw::memory::MemoryMapKey,
) -> Result<(), Status> {
    if !boot_services_available() {
        return Err(Status::UNSUPPORTED);
    }

    let handle = image_handle().ok_or(Status::INVALID_PARAMETER)?;
    let bs = boot_services();

    let status = (bs.exit_boot_services)(handle, memory_map_key.0);

    if status.is_success() {
        BOOT_SERVICES_EXITED.store(true, Ordering::Release);
        Ok(())
    } else {
        Err(status)
    }
}

// =============================================================================
// SHUTDOWN
// =============================================================================

/// Reset the system
///
/// # Safety
/// This will reset the machine!
pub unsafe fn reset_system(reset_type: ResetType, status: Status, data: Option<&[u8]>) -> ! {
    let rs = runtime_services();

    let (data_size, data_ptr) = match data {
        Some(d) => (d.len(), d.as_ptr()),
        None => (0, core::ptr::null()),
    };

    (rs.reset_system)(reset_type, status, data_size, data_ptr);

    // Should never return, but just in case
    #[allow(unreachable_code)]
    loop {
        core::hint::spin_loop();
    }
}

/// Shutdown the system
///
/// # Safety
/// This will shut down the machine!
pub unsafe fn shutdown() -> ! {
    reset_system(ResetType::Shutdown, Status::SUCCESS, None)
}

/// Reboot the system
///
/// # Safety
/// This will reboot the machine!
pub unsafe fn reboot() -> ! {
    reset_system(ResetType::Cold, Status::SUCCESS, None)
}

// =============================================================================
// EXTERN ALLOC
// =============================================================================

extern crate alloc;
