//! Raw UEFI Runtime Services
//!
//! Runtime Services remain available after ExitBootServices is called.
//! They provide time, variable storage, and system control services.

use super::memory::MemoryDescriptor;
use super::types::*;

// =============================================================================
// RUNTIME SERVICES TABLE
// =============================================================================

/// EFI Runtime Services Table
///
/// Provides services that remain available after ExitBootServices is called.
#[repr(C)]
pub struct EfiRuntimeServices {
    /// Table header
    pub hdr: TableHeader,

    // =========================================================================
    // Time Services
    // =========================================================================
    /// Get the current time
    pub get_time:
        unsafe extern "efiapi" fn(time: *mut Time, capabilities: *mut TimeCapabilities) -> Status,

    /// Set the current time
    pub set_time: unsafe extern "efiapi" fn(time: *const Time) -> Status,

    /// Get the wakeup alarm time
    pub get_wakeup_time: unsafe extern "efiapi" fn(
        enabled: *mut Boolean,
        pending: *mut Boolean,
        time: *mut Time,
    ) -> Status,

    /// Set the wakeup alarm time
    pub set_wakeup_time: unsafe extern "efiapi" fn(enable: Boolean, time: *const Time) -> Status,

    // =========================================================================
    // Virtual Memory Services
    // =========================================================================
    /// Set virtual address map
    pub set_virtual_address_map: unsafe extern "efiapi" fn(
        memory_map_size: usize,
        descriptor_size: usize,
        descriptor_version: u32,
        virtual_map: *mut MemoryDescriptor,
    ) -> Status,

    /// Convert pointer to virtual address
    pub convert_pointer: unsafe extern "efiapi" fn(
        debug_disposition: usize,
        address: *mut *mut core::ffi::c_void,
    ) -> Status,

    // =========================================================================
    // Variable Services
    // =========================================================================
    /// Get a variable
    pub get_variable: unsafe extern "efiapi" fn(
        variable_name: *const Char16,
        vendor_guid: *const Guid,
        attributes: *mut u32,
        data_size: *mut usize,
        data: *mut u8,
    ) -> Status,

    /// Get the next variable name
    pub get_next_variable_name: unsafe extern "efiapi" fn(
        variable_name_size: *mut usize,
        variable_name: *mut Char16,
        vendor_guid: *mut Guid,
    ) -> Status,

    /// Set a variable
    pub set_variable: unsafe extern "efiapi" fn(
        variable_name: *const Char16,
        vendor_guid: *const Guid,
        attributes: u32,
        data_size: usize,
        data: *const u8,
    ) -> Status,

    // =========================================================================
    // Miscellaneous Services
    // =========================================================================
    /// Get the next high monotonic count
    pub get_next_high_monotonic_count: unsafe extern "efiapi" fn(high_count: *mut u32) -> Status,

    /// Reset the system
    pub reset_system: unsafe extern "efiapi" fn(
        reset_type: ResetType,
        reset_status: Status,
        data_size: usize,
        reset_data: *const u8,
    ) -> !,

    // =========================================================================
    // Capsule Services (UEFI 2.0+)
    // =========================================================================
    /// Update capsule
    pub update_capsule: unsafe extern "efiapi" fn(
        capsule_header_array: *const *const CapsuleHeader,
        capsule_count: usize,
        scatter_gather_list: PhysicalAddress,
    ) -> Status,

    /// Query capsule capabilities
    pub query_capsule_capabilities: unsafe extern "efiapi" fn(
        capsule_header_array: *const *const CapsuleHeader,
        capsule_count: usize,
        maximum_capsule_size: *mut u64,
        reset_type: *mut ResetType,
    ) -> Status,

    // =========================================================================
    // Variable Query Services (UEFI 2.0+)
    // =========================================================================
    /// Query variable info
    pub query_variable_info: unsafe extern "efiapi" fn(
        attributes: u32,
        maximum_variable_storage_size: *mut u64,
        remaining_variable_storage_size: *mut u64,
        maximum_variable_size: *mut u64,
    ) -> Status,
}

impl EfiRuntimeServices {
    /// Runtime Services signature: "RUNTSERV"
    pub const SIGNATURE: u64 = TableHeader::RUNTIME_SERVICES_SIGNATURE;

    /// Validate the runtime services table
    pub fn validate(&self) -> bool {
        self.hdr.validate(Self::SIGNATURE)
    }

    // =========================================================================
    // Time Services (Safe wrappers)
    // =========================================================================

    /// Get the current time
    ///
    /// # Safety
    /// The caller must ensure runtime services are available.
    pub unsafe fn get_time(&self) -> Result<(Time, TimeCapabilities), Status> {
        let mut time = Time::default();
        let mut capabilities = TimeCapabilities::default();
        let status = (self.get_time)(&mut time, &mut capabilities);
        status.to_status_result_with((time, capabilities))
    }

    /// Get the current time (without capabilities)
    ///
    /// # Safety
    /// The caller must ensure runtime services are available.
    pub unsafe fn get_time_simple(&self) -> Result<Time, Status> {
        let mut time = Time::default();
        let status = (self.get_time)(&mut time, core::ptr::null_mut());
        status.to_status_result_with(time)
    }

    /// Set the current time
    ///
    /// # Safety
    /// The caller must ensure runtime services are available.
    pub unsafe fn set_time(&self, time: &Time) -> Result<(), Status> {
        let status = (self.set_time)(time);
        status.to_status_result()
    }

    /// Get wakeup alarm time
    ///
    /// # Safety
    /// The caller must ensure runtime services are available.
    pub unsafe fn get_wakeup_time(&self) -> Result<(bool, bool, Time), Status> {
        let mut enabled = 0;
        let mut pending = 0;
        let mut time = Time::default();
        let status = (self.get_wakeup_time)(&mut enabled, &mut pending, &mut time);
        status.to_status_result_with((enabled != 0, pending != 0, time))
    }

    /// Set wakeup alarm time
    ///
    /// # Safety
    /// The caller must ensure runtime services are available.
    pub unsafe fn set_wakeup_time(&self, enable: bool, time: Option<&Time>) -> Result<(), Status> {
        let status = (self.set_wakeup_time)(
            enable as Boolean,
            time.map(|t| t as *const Time).unwrap_or(core::ptr::null()),
        );
        status.to_status_result()
    }

    // =========================================================================
    // Virtual Memory Services (Safe wrappers)
    // =========================================================================

    /// Set the virtual address map
    ///
    /// # Safety
    /// This must only be called once, immediately after ExitBootServices.
    /// The caller must provide a valid memory map with virtual addresses set.
    pub unsafe fn set_virtual_address_map(
        &self,
        map: &mut [MemoryDescriptor],
        descriptor_size: usize,
        descriptor_version: u32,
    ) -> Result<(), Status> {
        let map_size = map.len() * descriptor_size;
        let status = (self.set_virtual_address_map)(
            map_size,
            descriptor_size,
            descriptor_version,
            map.as_mut_ptr(),
        );
        status.to_status_result()
    }

    /// Convert a pointer from physical to virtual address
    ///
    /// # Safety
    /// This must only be called after SetVirtualAddressMap.
    pub unsafe fn convert_pointer<T>(&self, ptr: *mut *mut T) -> Result<(), Status> {
        let status = (self.convert_pointer)(0, ptr as *mut *mut core::ffi::c_void);
        status.to_status_result()
    }

    // =========================================================================
    // Variable Services (Safe wrappers)
    // =========================================================================

    /// Get a variable's value
    ///
    /// # Safety
    /// The caller must ensure runtime services are available.
    pub unsafe fn get_variable(
        &self,
        name: *const Char16,
        vendor_guid: &Guid,
        buffer: &mut [u8],
    ) -> Result<(usize, u32), Status> {
        let mut attributes = 0;
        let mut size = buffer.len();

        let status = (self.get_variable)(
            name,
            vendor_guid,
            &mut attributes,
            &mut size,
            buffer.as_mut_ptr(),
        );

        status.to_status_result_with((size, attributes))
    }

    /// Get the size of a variable
    ///
    /// # Safety
    /// The caller must ensure runtime services are available.
    pub unsafe fn get_variable_size(
        &self,
        name: *const Char16,
        vendor_guid: &Guid,
    ) -> Result<usize, Status> {
        let mut size = 0;
        let status = (self.get_variable)(
            name,
            vendor_guid,
            core::ptr::null_mut(),
            &mut size,
            core::ptr::null_mut(),
        );

        if status == Status::BUFFER_TOO_SMALL {
            Ok(size)
        } else if status.is_success() {
            Ok(size)
        } else {
            Err(status)
        }
    }

    /// Set a variable
    ///
    /// # Safety
    /// The caller must ensure runtime services are available.
    pub unsafe fn set_variable(
        &self,
        name: *const Char16,
        vendor_guid: &Guid,
        attributes: u32,
        data: &[u8],
    ) -> Result<(), Status> {
        let status = (self.set_variable)(name, vendor_guid, attributes, data.len(), data.as_ptr());
        status.to_status_result()
    }

    /// Delete a variable
    ///
    /// # Safety
    /// The caller must ensure runtime services are available.
    pub unsafe fn delete_variable(
        &self,
        name: *const Char16,
        vendor_guid: &Guid,
    ) -> Result<(), Status> {
        let status = (self.set_variable)(name, vendor_guid, 0, 0, core::ptr::null());
        status.to_status_result()
    }

    /// Query variable storage information
    ///
    /// # Safety
    /// The caller must ensure runtime services are available.
    pub unsafe fn query_variable_info(&self, attributes: u32) -> Result<VariableInfo, Status> {
        let mut max_storage = 0;
        let mut remaining_storage = 0;
        let mut max_variable = 0;

        let status = (self.query_variable_info)(
            attributes,
            &mut max_storage,
            &mut remaining_storage,
            &mut max_variable,
        );

        status.to_status_result_with(VariableInfo {
            maximum_variable_storage_size: max_storage,
            remaining_variable_storage_size: remaining_storage,
            maximum_variable_size: max_variable,
        })
    }

    // =========================================================================
    // System Control Services
    // =========================================================================

    /// Reset the system (cold reboot)
    ///
    /// # Safety
    /// This function never returns.
    pub unsafe fn reset_cold(&self) -> ! {
        (self.reset_system)(ResetType::Cold, Status::SUCCESS, 0, core::ptr::null())
    }

    /// Reset the system (warm reboot)
    ///
    /// # Safety
    /// This function never returns.
    pub unsafe fn reset_warm(&self) -> ! {
        (self.reset_system)(ResetType::Warm, Status::SUCCESS, 0, core::ptr::null())
    }

    /// Shutdown the system
    ///
    /// # Safety
    /// This function never returns.
    pub unsafe fn shutdown(&self) -> ! {
        (self.reset_system)(ResetType::Shutdown, Status::SUCCESS, 0, core::ptr::null())
    }

    /// Reset the system with a specific type and optional data
    ///
    /// # Safety
    /// This function never returns.
    pub unsafe fn reset(&self, reset_type: ResetType, status: Status, data: Option<&[u8]>) -> ! {
        let (size, ptr) = match data {
            Some(d) => (d.len(), d.as_ptr()),
            None => (0, core::ptr::null()),
        };

        (self.reset_system)(reset_type, status, size, ptr)
    }

    // =========================================================================
    // Capsule Services
    // =========================================================================

    /// Query capsule capabilities
    ///
    /// # Safety
    /// The caller must ensure runtime services are available.
    pub unsafe fn query_capsule_capabilities(
        &self,
        capsules: &[*const CapsuleHeader],
    ) -> Result<CapsuleCapabilities, Status> {
        let mut max_size = 0;
        let mut reset_type = ResetType::Cold;

        let status = (self.query_capsule_capabilities)(
            capsules.as_ptr(),
            capsules.len(),
            &mut max_size,
            &mut reset_type,
        );

        status.to_status_result_with(CapsuleCapabilities {
            maximum_capsule_size: max_size,
            reset_type,
        })
    }

    /// Get next high monotonic count
    ///
    /// # Safety
    /// The caller must ensure runtime services are available.
    pub unsafe fn get_next_high_monotonic_count(&self) -> Result<u32, Status> {
        let mut count = 0;
        let status = (self.get_next_high_monotonic_count)(&mut count);
        status.to_status_result_with(count)
    }
}

impl core::fmt::Debug for EfiRuntimeServices {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EfiRuntimeServices")
            .field("hdr", &self.hdr)
            .finish()
    }
}

// Safety: EfiRuntimeServices is designed to be called from various contexts
unsafe impl Send for EfiRuntimeServices {}
unsafe impl Sync for EfiRuntimeServices {}

// =============================================================================
// VARIABLE ATTRIBUTES
// =============================================================================

/// Variable attributes
pub mod variable_attributes {
    /// Non-volatile storage
    pub const NON_VOLATILE: u32 = 0x0000_0001;
    /// Boot service access
    pub const BOOTSERVICE_ACCESS: u32 = 0x0000_0002;
    /// Runtime access
    pub const RUNTIME_ACCESS: u32 = 0x0000_0004;
    /// Hardware error record
    pub const HARDWARE_ERROR_RECORD: u32 = 0x0000_0008;
    /// Authenticated write access (deprecated)
    pub const AUTHENTICATED_WRITE_ACCESS: u32 = 0x0000_0010;
    /// Time-based authenticated write access
    pub const TIME_BASED_AUTHENTICATED_WRITE_ACCESS: u32 = 0x0000_0020;
    /// Append write
    pub const APPEND_WRITE: u32 = 0x0000_0040;
    /// Enhanced authenticated access
    pub const ENHANCED_AUTHENTICATED_ACCESS: u32 = 0x0000_0080;

    /// Common attribute combination for persistent runtime variables
    pub const NV_BS_RT: u32 = NON_VOLATILE | BOOTSERVICE_ACCESS | RUNTIME_ACCESS;

    /// Common attribute combination for boot-time only variables
    pub const BS: u32 = BOOTSERVICE_ACCESS;

    /// Common attribute combination for persistent boot variables
    pub const NV_BS: u32 = NON_VOLATILE | BOOTSERVICE_ACCESS;
}

// =============================================================================
// HELPER TYPES
// =============================================================================

/// Variable storage information
#[derive(Debug, Clone, Copy)]
pub struct VariableInfo {
    /// Maximum variable storage size
    pub maximum_variable_storage_size: u64,
    /// Remaining variable storage size
    pub remaining_variable_storage_size: u64,
    /// Maximum individual variable size
    pub maximum_variable_size: u64,
}

/// Capsule capabilities
#[derive(Debug, Clone, Copy)]
pub struct CapsuleCapabilities {
    /// Maximum capsule size
    pub maximum_capsule_size: u64,
    /// Required reset type
    pub reset_type: ResetType,
}

// =============================================================================
// WELL-KNOWN VARIABLES
// =============================================================================

/// Well-known UEFI variable names
pub mod variable_names {
    /// Boot order variable name
    pub const BOOT_ORDER: &[u16] = &[
        'B' as u16, 'o' as u16, 'o' as u16, 't' as u16, 'O' as u16, 'r' as u16, 'd' as u16,
        'e' as u16, 'r' as u16, 0,
    ];

    /// Boot current variable name
    pub const BOOT_CURRENT: &[u16] = &[
        'B' as u16, 'o' as u16, 'o' as u16, 't' as u16, 'C' as u16, 'u' as u16, 'r' as u16,
        'r' as u16, 'e' as u16, 'n' as u16, 't' as u16, 0,
    ];

    /// Boot next variable name
    pub const BOOT_NEXT: &[u16] = &[
        'B' as u16, 'o' as u16, 'o' as u16, 't' as u16, 'N' as u16, 'e' as u16, 'x' as u16,
        't' as u16, 0,
    ];

    /// Timeout variable name
    pub const TIMEOUT: &[u16] = &[
        'T' as u16, 'i' as u16, 'm' as u16, 'e' as u16, 'o' as u16, 'u' as u16, 't' as u16, 0,
    ];

    /// Secure Boot variable name
    pub const SECURE_BOOT: &[u16] = &[
        'S' as u16, 'e' as u16, 'c' as u16, 'u' as u16, 'r' as u16, 'e' as u16, 'B' as u16,
        'o' as u16, 'o' as u16, 't' as u16, 0,
    ];

    /// Setup Mode variable name
    pub const SETUP_MODE: &[u16] = &[
        'S' as u16, 'e' as u16, 't' as u16, 'u' as u16, 'p' as u16, 'M' as u16, 'o' as u16,
        'd' as u16, 'e' as u16, 0,
    ];

    /// Platform Key variable name
    pub const PK: &[u16] = &['P' as u16, 'K' as u16, 0];

    /// Key Exchange Key variable name
    pub const KEK: &[u16] = &['K' as u16, 'E' as u16, 'K' as u16, 0];

    /// Authorized Signatures Database
    pub const DB: &[u16] = &['d' as u16, 'b' as u16, 0];

    /// Forbidden Signatures Database
    pub const DBX: &[u16] = &['d' as u16, 'b' as u16, 'x' as u16, 0];

    /// Timestamp Signatures Database
    pub const DBT: &[u16] = &['d' as u16, 'b' as u16, 't' as u16, 0];

    /// OS Indications
    pub const OS_INDICATIONS: &[u16] = &[
        'O' as u16, 'S' as u16, 'I' as u16, 'n' as u16, 'd' as u16, 'i' as u16, 'c' as u16,
        'a' as u16, 't' as u16, 'i' as u16, 'o' as u16, 'n' as u16, 's' as u16, 0,
    ];

    /// OS Indications Supported
    pub const OS_INDICATIONS_SUPPORTED: &[u16] = &[
        'O' as u16, 'S' as u16, 'I' as u16, 'n' as u16, 'd' as u16, 'i' as u16, 'c' as u16,
        'a' as u16, 't' as u16, 'i' as u16, 'o' as u16, 'n' as u16, 's' as u16, 'S' as u16,
        'u' as u16, 'p' as u16, 'p' as u16, 'o' as u16, 'r' as u16, 't' as u16, 'e' as u16,
        'd' as u16, 0,
    ];
}

/// OS Indication bits
pub mod os_indications {
    /// Boot to firmware UI
    pub const BOOT_TO_FW_UI: u64 = 0x0000_0000_0000_0001;
    /// Timestamp revocation
    pub const TIMESTAMP_REVOCATION: u64 = 0x0000_0000_0000_0002;
    /// File capsule delivery supported
    pub const FILE_CAPSULE_DELIVERY_SUPPORTED: u64 = 0x0000_0000_0000_0004;
    /// FMP capsule supported
    pub const FMP_CAPSULE_SUPPORTED: u64 = 0x0000_0000_0000_0008;
    /// Capsule result variable supported
    pub const CAPSULE_RESULT_VAR_SUPPORTED: u64 = 0x0000_0000_0000_0010;
    /// Start OS recovery
    pub const START_OS_RECOVERY: u64 = 0x0000_0000_0000_0020;
    /// Start platform recovery
    pub const START_PLATFORM_RECOVERY: u64 = 0x0000_0000_0000_0040;
    /// JSON config data refresh
    pub const JSON_CONFIG_DATA_REFRESH: u64 = 0x0000_0000_0000_0080;
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_services_signature() {
        assert_eq!(EfiRuntimeServices::SIGNATURE, 0x56524553544E5552);
    }

    #[test]
    fn test_variable_attributes() {
        let attrs = variable_attributes::NV_BS_RT;
        assert_eq!(attrs, 0x07);
    }

    #[test]
    fn test_variable_names() {
        // Check that variable names are null-terminated
        assert_eq!(*variable_names::BOOT_ORDER.last().unwrap(), 0);
        assert_eq!(*variable_names::SECURE_BOOT.last().unwrap(), 0);
    }
}
