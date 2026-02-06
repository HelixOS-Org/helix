//! Runtime Services Wrapper
//!
//! Safe wrappers for UEFI Runtime Services.

use core::ptr::NonNull;

use crate::raw::runtime_services::EfiRuntimeServices;
use crate::raw::types::*;

// =============================================================================
// RUNTIME SERVICES
// =============================================================================

/// Runtime services wrapper
pub struct RuntimeServices {
    inner: NonNull<EfiRuntimeServices>,
}

impl RuntimeServices {
    /// Create from raw pointer
    ///
    /// # Safety
    /// Pointer must be valid.
    pub unsafe fn from_ptr(ptr: *mut EfiRuntimeServices) -> Option<Self> {
        NonNull::new(ptr).map(|inner| Self { inner })
    }

    /// Get the raw runtime services pointer
    pub fn as_ptr(&self) -> *mut EfiRuntimeServices {
        self.inner.as_ptr()
    }

    /// Get runtime services reference
    fn rs(&self) -> &EfiRuntimeServices {
        unsafe { self.inner.as_ref() }
    }
}

// =============================================================================
// TIME SERVICES
// =============================================================================

impl RuntimeServices {
    /// Get current time
    pub fn get_time(&self) -> Result<(Time, Option<TimeCapabilities>), Status> {
        let mut time = Time::default();
        let mut capabilities = TimeCapabilities::default();

        let status = unsafe { (self.rs().get_time)(&mut time, &mut capabilities) };

        status.to_status_result_with((time, Some(capabilities)))
    }

    /// Set current time
    pub fn set_time(&self, time: &Time) -> Result<(), Status> {
        let status = unsafe { (self.rs().set_time)(time) };
        status.to_status_result()
    }

    /// Get wakeup time
    pub fn get_wakeup_time(&self) -> Result<(bool, bool, Time), Status> {
        let mut enabled: u8 = 0;
        let mut pending: u8 = 0;
        let mut time = Time::default();

        let status = unsafe { (self.rs().get_wakeup_time)(&mut enabled, &mut pending, &mut time) };

        status.to_status_result_with((enabled != 0, pending != 0, time))
    }

    /// Set wakeup time
    pub fn set_wakeup_time(&self, enable: bool, time: Option<&Time>) -> Result<(), Status> {
        let time_ptr = match time {
            Some(t) => t as *const Time,
            None => core::ptr::null(),
        };

        let status = unsafe { (self.rs().set_wakeup_time)(u8::from(enable), time_ptr) };
        status.to_status_result()
    }
}

// =============================================================================
// VARIABLE SERVICES
// =============================================================================

impl RuntimeServices {
    /// Get variable
    pub fn get_variable(
        &self,
        name: &[u16],
        vendor_guid: &Guid,
        buffer: &mut [u8],
    ) -> Result<(u32, usize), Status> {
        let mut attributes = 0u32;
        let mut data_size = buffer.len();

        let status = unsafe {
            (self.rs().get_variable)(
                name.as_ptr(),
                vendor_guid,
                &mut attributes,
                &mut data_size,
                buffer.as_mut_ptr(),
            )
        };

        status.to_status_result_with((attributes, data_size))
    }

    /// Set variable
    pub fn set_variable(
        &self,
        name: &[u16],
        vendor_guid: &Guid,
        attributes: u32,
        data: &[u8],
    ) -> Result<(), Status> {
        let status = unsafe {
            (self.rs().set_variable)(
                name.as_ptr(),
                vendor_guid,
                attributes,
                data.len(),
                data.as_ptr(),
            )
        };
        status.to_status_result()
    }

    /// Delete variable
    pub fn delete_variable(&self, name: &[u16], vendor_guid: &Guid) -> Result<(), Status> {
        self.set_variable(name, vendor_guid, 0, &[])
    }

    /// Get next variable name
    pub fn get_next_variable_name(
        &self,
        name_buffer: &mut [u16],
        vendor_guid: &mut Guid,
    ) -> Result<usize, Status> {
        let mut name_size = name_buffer.len() * 2;

        let status = unsafe {
            (self.rs().get_next_variable_name)(
                &mut name_size,
                name_buffer.as_mut_ptr(),
                vendor_guid,
            )
        };

        status.to_status_result_with(name_size / 2)
    }

    /// Query variable info
    pub fn query_variable_info(&self, attributes: u32) -> Result<VariableStorageInfo, Status> {
        let mut max_storage = 0u64;
        let mut remaining_storage = 0u64;
        let mut max_variable_size = 0u64;

        let status = unsafe {
            (self.rs().query_variable_info)(
                attributes,
                &mut max_storage,
                &mut remaining_storage,
                &mut max_variable_size,
            )
        };

        status.to_status_result_with(VariableStorageInfo {
            max_storage,
            remaining_storage,
            max_variable_size,
        })
    }
}

/// Variable storage information
#[derive(Debug, Clone, Copy)]
pub struct VariableStorageInfo {
    /// Maximum variable storage size
    pub max_storage: u64,
    /// Remaining variable storage size
    pub remaining_storage: u64,
    /// Maximum size of a single variable
    pub max_variable_size: u64,
}

// =============================================================================
// CAPSULE SERVICES
// =============================================================================

impl RuntimeServices {
    /// Update capsule
    ///
    /// # Safety
    /// Capsule headers must be valid.
    pub unsafe fn update_capsule(
        &self,
        capsule_headers: &[*mut CapsuleHeader],
        scatter_gather_list: PhysicalAddress,
    ) -> Result<(), Status> {
        let status = (self.rs().update_capsule)(
            capsule_headers.as_ptr() as *const *const CapsuleHeader,
            capsule_headers.len(),
            scatter_gather_list,
        );
        status.to_status_result()
    }

    /// Query capsule capabilities
    ///
    /// # Safety
    /// Capsule headers must be valid.
    pub unsafe fn query_capsule_capabilities(
        &self,
        capsule_headers: &[*mut CapsuleHeader],
    ) -> Result<CapsuleCapabilities, Status> {
        let mut max_size = 0u64;
        let mut reset_type = ResetType::Cold;

        let status = (self.rs().query_capsule_capabilities)(
            capsule_headers.as_ptr() as *const *const CapsuleHeader,
            capsule_headers.len(),
            &mut max_size,
            &mut reset_type,
        );

        status.to_status_result_with(CapsuleCapabilities {
            max_capsule_size: max_size,
            reset_type,
        })
    }
}

/// Capsule capabilities
#[derive(Debug, Clone, Copy)]
pub struct CapsuleCapabilities {
    /// Maximum capsule size
    pub max_capsule_size: u64,
    /// Required reset type
    pub reset_type: ResetType,
}

// =============================================================================
// VIRTUAL MEMORY SERVICES
// =============================================================================

impl RuntimeServices {
    /// Set virtual address map
    ///
    /// # Safety
    /// Must be called only once after ExitBootServices.
    pub unsafe fn set_virtual_address_map(
        &self,
        memory_map: &mut [crate::raw::memory::MemoryDescriptor],
        descriptor_size: usize,
    ) -> Result<(), Status> {
        let map_size = memory_map.len() * descriptor_size;

        let status = (self.rs().set_virtual_address_map)(
            map_size,
            descriptor_size,
            1, // descriptor version
            memory_map.as_mut_ptr(),
        );

        status.to_status_result()
    }

    /// Convert pointer
    ///
    /// # Safety
    /// Must be called only after SetVirtualAddressMap.
    pub unsafe fn convert_pointer(
        &self,
        debug_disposition: usize,
        address: *mut *mut core::ffi::c_void,
    ) -> Result<(), Status> {
        let status = (self.rs().convert_pointer)(debug_disposition, address);
        status.to_status_result()
    }
}

// =============================================================================
// RESET SERVICES
// =============================================================================

impl RuntimeServices {
    /// Reset the system
    ///
    /// # Safety
    /// This will reset the machine!
    pub unsafe fn reset_system(
        &self,
        reset_type: ResetType,
        status: Status,
        data: Option<&[u8]>,
    ) -> ! {
        let (data_size, data_ptr) = match data {
            Some(d) => (d.len(), d.as_ptr()),
            None => (0, core::ptr::null()),
        };

        (self.rs().reset_system)(reset_type, status, data_size, data_ptr);

        // Safety fallback - should never be reached as reset_system diverges
        #[allow(unreachable_code)]
        loop {
            core::hint::spin_loop();
        }
    }

    /// Shutdown
    ///
    /// # Safety
    /// This will shutdown the machine!
    pub unsafe fn shutdown(&self) -> ! {
        self.reset_system(ResetType::Shutdown, Status::SUCCESS, None)
    }

    /// Reboot
    ///
    /// # Safety
    /// This will reboot the machine!
    pub unsafe fn reboot(&self) -> ! {
        self.reset_system(ResetType::Cold, Status::SUCCESS, None)
    }

    /// Warm reboot
    ///
    /// # Safety
    /// This will warm reboot the machine!
    pub unsafe fn warm_reboot(&self) -> ! {
        self.reset_system(ResetType::Warm, Status::SUCCESS, None)
    }
}

// =============================================================================
// GLOBAL ACCESS
// =============================================================================

/// Get runtime services from global state
///
/// # Safety
/// Must only be called after initialization.
pub unsafe fn runtime_services() -> RuntimeServices {
    RuntimeServices::from_ptr(super::runtime_services() as *const _ as *mut _)
        .expect("Runtime services not available")
}
