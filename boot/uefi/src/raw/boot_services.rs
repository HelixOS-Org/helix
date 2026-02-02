//! Raw UEFI Boot Services
//!
//! Boot Services are only available before ExitBootServices is called.
//! They provide memory management, protocol handling, and device management.

use super::memory::{MemoryDescriptor, MemoryType};
use super::types::*;

// =============================================================================
// BOOT SERVICES TABLE
// =============================================================================

/// EFI Boot Services Table
///
/// Provides services available during the boot phase before
/// ExitBootServices is called.
#[repr(C)]
pub struct EfiBootServices {
    /// Table header
    pub hdr: TableHeader,

    // =========================================================================
    // Task Priority Services
    // =========================================================================
    /// Raise the task priority level
    pub raise_tpl: unsafe extern "efiapi" fn(new_tpl: Tpl) -> Tpl,

    /// Restore the task priority level
    pub restore_tpl: unsafe extern "efiapi" fn(old_tpl: Tpl),

    // =========================================================================
    // Memory Services
    // =========================================================================
    /// Allocate pages of memory
    pub allocate_pages: unsafe extern "efiapi" fn(
        alloc_type: AllocateType,
        memory_type: MemoryType,
        pages: usize,
        memory: *mut PhysicalAddress,
    ) -> Status,

    /// Free pages of memory
    pub free_pages: unsafe extern "efiapi" fn(memory: PhysicalAddress, pages: usize) -> Status,

    /// Get the memory map
    pub get_memory_map: unsafe extern "efiapi" fn(
        memory_map_size: *mut usize,
        memory_map: *mut MemoryDescriptor,
        map_key: *mut usize,
        descriptor_size: *mut usize,
        descriptor_version: *mut u32,
    ) -> Status,

    /// Allocate pool memory
    pub allocate_pool: unsafe extern "efiapi" fn(
        pool_type: MemoryType,
        size: usize,
        buffer: *mut *mut u8,
    ) -> Status,

    /// Free pool memory
    pub free_pool: unsafe extern "efiapi" fn(buffer: *mut u8) -> Status,

    // =========================================================================
    // Event & Timer Services
    // =========================================================================
    /// Create an event
    pub create_event: unsafe extern "efiapi" fn(
        event_type: u32,
        notify_tpl: Tpl,
        notify_function: Option<
            unsafe extern "efiapi" fn(event: Event, context: *mut core::ffi::c_void),
        >,
        notify_context: *mut core::ffi::c_void,
        event: *mut Event,
    ) -> Status,

    /// Set a timer
    pub set_timer: unsafe extern "efiapi" fn(
        event: Event,
        timer_type: TimerDelay,
        trigger_time: u64,
    ) -> Status,

    /// Wait for events
    pub wait_for_event: unsafe extern "efiapi" fn(
        number_of_events: usize,
        events: *const Event,
        index: *mut usize,
    ) -> Status,

    /// Signal an event
    pub signal_event: unsafe extern "efiapi" fn(event: Event) -> Status,

    /// Close an event
    pub close_event: unsafe extern "efiapi" fn(event: Event) -> Status,

    /// Check an event
    pub check_event: unsafe extern "efiapi" fn(event: Event) -> Status,

    // =========================================================================
    // Protocol Handler Services
    // =========================================================================
    /// Install protocol interface
    pub install_protocol_interface: unsafe extern "efiapi" fn(
        handle: *mut Handle,
        protocol: *const Guid,
        interface_type: InterfaceType,
        interface: *mut core::ffi::c_void,
    ) -> Status,

    /// Reinstall protocol interface
    pub reinstall_protocol_interface: unsafe extern "efiapi" fn(
        handle: Handle,
        protocol: *const Guid,
        old_interface: *mut core::ffi::c_void,
        new_interface: *mut core::ffi::c_void,
    ) -> Status,

    /// Uninstall protocol interface
    pub uninstall_protocol_interface: unsafe extern "efiapi" fn(
        handle: Handle,
        protocol: *const Guid,
        interface: *mut core::ffi::c_void,
    ) -> Status,

    /// Handle protocol
    pub handle_protocol: unsafe extern "efiapi" fn(
        handle: Handle,
        protocol: *const Guid,
        interface: *mut *mut core::ffi::c_void,
    ) -> Status,

    /// Reserved (must be NULL)
    pub reserved: *mut core::ffi::c_void,

    /// Register protocol notify
    pub register_protocol_notify: unsafe extern "efiapi" fn(
        protocol: *const Guid,
        event: Event,
        registration: *mut *mut core::ffi::c_void,
    ) -> Status,

    /// Locate handle
    pub locate_handle: unsafe extern "efiapi" fn(
        search_type: LocateSearchType,
        protocol: *const Guid,
        search_key: *mut core::ffi::c_void,
        buffer_size: *mut usize,
        buffer: *mut Handle,
    ) -> Status,

    /// Locate device path
    pub locate_device_path: unsafe extern "efiapi" fn(
        protocol: *const Guid,
        device_path: *mut *mut core::ffi::c_void,
        device: *mut Handle,
    ) -> Status,

    /// Install configuration table
    pub install_configuration_table:
        unsafe extern "efiapi" fn(guid: *const Guid, table: *mut core::ffi::c_void) -> Status,

    // =========================================================================
    // Image Services
    // =========================================================================
    /// Load an image
    pub load_image: unsafe extern "efiapi" fn(
        boot_policy: Boolean,
        parent_image_handle: Handle,
        device_path: *const core::ffi::c_void,
        source_buffer: *const u8,
        source_size: usize,
        image_handle: *mut Handle,
    ) -> Status,

    /// Start an image
    pub start_image: unsafe extern "efiapi" fn(
        image_handle: Handle,
        exit_data_size: *mut usize,
        exit_data: *mut *mut Char16,
    ) -> Status,

    /// Exit from an image
    pub exit: unsafe extern "efiapi" fn(
        image_handle: Handle,
        exit_status: Status,
        exit_data_size: usize,
        exit_data: *const Char16,
    ) -> Status,

    /// Unload an image
    pub unload_image: unsafe extern "efiapi" fn(image_handle: Handle) -> Status,

    /// Exit boot services
    pub exit_boot_services:
        unsafe extern "efiapi" fn(image_handle: Handle, map_key: usize) -> Status,

    // =========================================================================
    // Miscellaneous Services
    // =========================================================================
    /// Get next monotonic count
    pub get_next_monotonic_count: unsafe extern "efiapi" fn(count: *mut u64) -> Status,

    /// Stall execution
    pub stall: unsafe extern "efiapi" fn(microseconds: usize) -> Status,

    /// Set watchdog timer
    pub set_watchdog_timer: unsafe extern "efiapi" fn(
        timeout: usize,
        watchdog_code: u64,
        data_size: usize,
        watchdog_data: *const Char16,
    ) -> Status,

    // =========================================================================
    // DriverSupport Services
    // =========================================================================
    /// Connect controller
    pub connect_controller: unsafe extern "efiapi" fn(
        controller_handle: Handle,
        driver_image_handle: Handle,
        remaining_device_path: *const core::ffi::c_void,
        recursive: Boolean,
    ) -> Status,

    /// Disconnect controller
    pub disconnect_controller: unsafe extern "efiapi" fn(
        controller_handle: Handle,
        driver_image_handle: Handle,
        child_handle: Handle,
    ) -> Status,

    // =========================================================================
    // Open and Close Protocol Services
    // =========================================================================
    /// Open protocol
    pub open_protocol: unsafe extern "efiapi" fn(
        handle: Handle,
        protocol: *const Guid,
        interface: *mut *mut core::ffi::c_void,
        agent_handle: Handle,
        controller_handle: Handle,
        attributes: u32,
    ) -> Status,

    /// Close protocol
    pub close_protocol: unsafe extern "efiapi" fn(
        handle: Handle,
        protocol: *const Guid,
        agent_handle: Handle,
        controller_handle: Handle,
    ) -> Status,

    /// Open protocol information
    pub open_protocol_information: unsafe extern "efiapi" fn(
        handle: Handle,
        protocol: *const Guid,
        entry_buffer: *mut *mut OpenProtocolInformationEntry,
        entry_count: *mut usize,
    ) -> Status,

    // =========================================================================
    // Library Services
    // =========================================================================
    /// Protocols per handle
    pub protocols_per_handle: unsafe extern "efiapi" fn(
        handle: Handle,
        protocol_buffer: *mut *mut *const Guid,
        protocol_buffer_count: *mut usize,
    ) -> Status,

    /// Locate handle buffer
    pub locate_handle_buffer: unsafe extern "efiapi" fn(
        search_type: LocateSearchType,
        protocol: *const Guid,
        search_key: *mut core::ffi::c_void,
        no_handles: *mut usize,
        buffer: *mut *mut Handle,
    ) -> Status,

    /// Locate protocol
    pub locate_protocol: unsafe extern "efiapi" fn(
        protocol: *const Guid,
        registration: *mut core::ffi::c_void,
        interface: *mut *mut core::ffi::c_void,
    ) -> Status,

    /// Install multiple protocol interfaces
    pub install_multiple_protocol_interfaces:
        unsafe extern "efiapi" fn(handle: *mut Handle, ...) -> Status,

    /// Uninstall multiple protocol interfaces
    pub uninstall_multiple_protocol_interfaces:
        unsafe extern "efiapi" fn(handle: Handle, ...) -> Status,

    // =========================================================================
    // 32-bit CRC Services
    // =========================================================================
    /// Calculate CRC32
    pub calculate_crc32:
        unsafe extern "efiapi" fn(data: *const u8, data_size: usize, crc32: *mut u32) -> Status,

    // =========================================================================
    // Miscellaneous Services (continued)
    // =========================================================================
    /// Copy memory
    pub copy_mem: unsafe extern "efiapi" fn(destination: *mut u8, source: *const u8, length: usize),

    /// Set memory
    pub set_mem: unsafe extern "efiapi" fn(buffer: *mut u8, size: usize, value: u8),

    /// Create event (extended)
    pub create_event_ex: unsafe extern "efiapi" fn(
        event_type: u32,
        notify_tpl: Tpl,
        notify_function: Option<
            unsafe extern "efiapi" fn(event: Event, context: *mut core::ffi::c_void),
        >,
        notify_context: *const core::ffi::c_void,
        event_group: *const Guid,
        event: *mut Event,
    ) -> Status,
}

impl EfiBootServices {
    /// Boot Services signature: "BOOTSERV"
    pub const SIGNATURE: u64 = TableHeader::BOOT_SERVICES_SIGNATURE;

    /// Validate the boot services table
    pub fn validate(&self) -> bool {
        self.hdr.validate(Self::SIGNATURE)
    }

    // =========================================================================
    // TPL Services (Safe wrappers)
    // =========================================================================

    /// Raise the task priority level
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn raise_tpl(&self, new_tpl: Tpl) -> Tpl {
        (self.raise_tpl)(new_tpl)
    }

    /// Restore the task priority level
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn restore_tpl(&self, old_tpl: Tpl) {
        (self.restore_tpl)(old_tpl)
    }

    // =========================================================================
    // Memory Services (Safe wrappers)
    // =========================================================================

    /// Allocate pages of memory
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn allocate_pages(
        &self,
        alloc_type: AllocateType,
        memory_type: MemoryType,
        pages: usize,
    ) -> Result<PhysicalAddress, Status> {
        let mut address = PhysicalAddress::NULL;
        let status = (self.allocate_pages)(alloc_type, memory_type, pages, &mut address);
        status.to_status_result_with(address)
    }

    /// Allocate pages at a specific address
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn allocate_pages_at(
        &self,
        address: PhysicalAddress,
        memory_type: MemoryType,
        pages: usize,
    ) -> Result<(), Status> {
        let mut addr = address;
        let status =
            (self.allocate_pages)(AllocateType::AllocateAddress, memory_type, pages, &mut addr);
        status.to_status_result()
    }

    /// Free pages of memory
    ///
    /// # Safety
    /// The caller must ensure the memory was allocated with allocate_pages.
    pub unsafe fn free_pages(&self, memory: PhysicalAddress, pages: usize) -> Result<(), Status> {
        let status = (self.free_pages)(memory, pages);
        status.to_status_result()
    }

    /// Get memory map size (call first to determine buffer size)
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn get_memory_map_size(&self) -> Result<(usize, usize), Status> {
        let mut map_size = 0;
        let mut map_key = 0;
        let mut desc_size = 0;
        let mut desc_version = 0;

        let status = (self.get_memory_map)(
            &mut map_size,
            core::ptr::null_mut(),
            &mut map_key,
            &mut desc_size,
            &mut desc_version,
        );

        // Expected to return BUFFER_TOO_SMALL
        if status == Status::BUFFER_TOO_SMALL {
            // Add extra space for potential changes
            Ok((map_size + desc_size * 4, desc_size))
        } else if status.is_success() {
            Ok((map_size, desc_size))
        } else {
            Err(status)
        }
    }

    /// Get the full memory map
    ///
    /// # Safety
    /// The caller must ensure boot services are available and buffer is valid.
    pub unsafe fn get_memory_map_full(
        &self,
        buffer: *mut u8,
        buffer_size: usize,
    ) -> Result<(usize, usize, usize, u32), Status> {
        let mut map_size = buffer_size;
        let mut map_key = 0;
        let mut desc_size = 0;
        let mut desc_version = 0;

        let status = (self.get_memory_map)(
            &mut map_size,
            buffer as *mut MemoryDescriptor,
            &mut map_key,
            &mut desc_size,
            &mut desc_version,
        );

        status.to_status_result_with((map_size, map_key, desc_size, desc_version))
    }

    /// Allocate pool memory
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn allocate_pool(
        &self,
        pool_type: MemoryType,
        size: usize,
    ) -> Result<*mut u8, Status> {
        let mut buffer = core::ptr::null_mut();
        let status = (self.allocate_pool)(pool_type, size, &mut buffer);
        status.to_status_result_with(buffer)
    }

    /// Free pool memory
    ///
    /// # Safety
    /// The caller must ensure the buffer was allocated with allocate_pool.
    pub unsafe fn free_pool(&self, buffer: *mut u8) -> Result<(), Status> {
        let status = (self.free_pool)(buffer);
        status.to_status_result()
    }

    // =========================================================================
    // Event Services (Safe wrappers)
    // =========================================================================

    /// Create a timer event
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn create_timer_event(&self) -> Result<Event, Status> {
        let mut event = Event::NULL;
        let status = (self.create_event)(
            event_type::TIMER,
            TPL_CALLBACK,
            None,
            core::ptr::null_mut(),
            &mut event,
        );
        status.to_status_result_with(event)
    }

    /// Set a timer
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn set_timer(
        &self,
        event: Event,
        timer_type: TimerDelay,
        trigger_time_100ns: u64,
    ) -> Result<(), Status> {
        let status = (self.set_timer)(event, timer_type, trigger_time_100ns);
        status.to_status_result()
    }

    /// Wait for events
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn wait_for_event(&self, events: &[Event]) -> Result<usize, Status> {
        let mut index = 0;
        let status = (self.wait_for_event)(events.len(), events.as_ptr(), &mut index);
        status.to_status_result_with(index)
    }

    /// Close an event
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn close_event(&self, event: Event) -> Result<(), Status> {
        let status = (self.close_event)(event);
        status.to_status_result()
    }

    // =========================================================================
    // Protocol Services (Safe wrappers)
    // =========================================================================

    /// Locate a protocol by GUID
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn locate_protocol<T>(&self, guid: &Guid) -> Result<*mut T, Status> {
        let mut interface = core::ptr::null_mut();
        let status = (self.locate_protocol)(guid, core::ptr::null_mut(), &mut interface);
        status.to_status_result_with(interface as *mut T)
    }

    /// Handle protocol
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn handle_protocol<T>(&self, handle: Handle, guid: &Guid) -> Result<*mut T, Status> {
        let mut interface = core::ptr::null_mut();
        let status = (self.handle_protocol)(handle, guid, &mut interface);
        status.to_status_result_with(interface as *mut T)
    }

    /// Open protocol
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn open_protocol<T>(
        &self,
        handle: Handle,
        guid: &Guid,
        agent_handle: Handle,
        controller_handle: Handle,
        attributes: u32,
    ) -> Result<*mut T, Status> {
        let mut interface = core::ptr::null_mut();
        let status = (self.open_protocol)(
            handle,
            guid,
            &mut interface,
            agent_handle,
            controller_handle,
            attributes,
        );
        status.to_status_result_with(interface as *mut T)
    }

    /// Close protocol
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn close_protocol(
        &self,
        handle: Handle,
        guid: &Guid,
        agent_handle: Handle,
        controller_handle: Handle,
    ) -> Result<(), Status> {
        let status = (self.close_protocol)(handle, guid, agent_handle, controller_handle);
        status.to_status_result()
    }

    /// Locate handles supporting a protocol
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn locate_handle_buffer(
        &self,
        search_type: LocateSearchType,
        protocol: Option<&Guid>,
    ) -> Result<(*mut Handle, usize), Status> {
        let mut num_handles = 0;
        let mut buffer = core::ptr::null_mut();

        let protocol_ptr = protocol
            .map(|g| g as *const Guid)
            .unwrap_or(core::ptr::null());

        let status = (self.locate_handle_buffer)(
            search_type,
            protocol_ptr,
            core::ptr::null_mut(),
            &mut num_handles,
            &mut buffer,
        );

        status.to_status_result_with((buffer, num_handles))
    }

    // =========================================================================
    // Image Services (Safe wrappers)
    // =========================================================================

    /// Load an image from memory
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn load_image_from_buffer(
        &self,
        parent_handle: Handle,
        source: &[u8],
    ) -> Result<Handle, Status> {
        let mut image_handle = Handle::NULL;
        let status = (self.load_image)(
            0, // Not boot policy
            parent_handle,
            core::ptr::null(),
            source.as_ptr(),
            source.len(),
            &mut image_handle,
        );
        status.to_status_result_with(image_handle)
    }

    /// Start an image
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn start_image(&self, image_handle: Handle) -> Result<(), Status> {
        let mut exit_data_size = 0;
        let mut exit_data = core::ptr::null_mut();
        let status = (self.start_image)(image_handle, &mut exit_data_size, &mut exit_data);
        status.to_status_result()
    }

    /// Unload an image
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn unload_image(&self, image_handle: Handle) -> Result<(), Status> {
        let status = (self.unload_image)(image_handle);
        status.to_status_result()
    }

    /// Exit boot services
    ///
    /// # Safety
    /// This is a one-way operation. After calling this, boot services are no
    /// longer available. The caller must have a valid memory map key.
    pub unsafe fn exit_boot_services(
        &self,
        image_handle: Handle,
        map_key: usize,
    ) -> Result<(), Status> {
        let status = (self.exit_boot_services)(image_handle, map_key);
        status.to_status_result()
    }

    // =========================================================================
    // Miscellaneous Services (Safe wrappers)
    // =========================================================================

    /// Stall execution for a number of microseconds
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn stall(&self, microseconds: usize) -> Result<(), Status> {
        let status = (self.stall)(microseconds);
        status.to_status_result()
    }

    /// Set the watchdog timer
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn set_watchdog_timer(
        &self,
        timeout_seconds: usize,
        watchdog_code: u64,
    ) -> Result<(), Status> {
        let status =
            (self.set_watchdog_timer)(timeout_seconds, watchdog_code, 0, core::ptr::null());
        status.to_status_result()
    }

    /// Disable the watchdog timer
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn disable_watchdog(&self) -> Result<(), Status> {
        self.set_watchdog_timer(0, 0)
    }

    /// Get next monotonic count
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn get_next_monotonic_count(&self) -> Result<u64, Status> {
        let mut count = 0;
        let status = (self.get_next_monotonic_count)(&mut count);
        status.to_status_result_with(count)
    }

    /// Calculate CRC32
    ///
    /// # Safety
    /// The caller must ensure boot services are available.
    pub unsafe fn calculate_crc32(&self, data: &[u8]) -> Result<u32, Status> {
        let mut crc = 0;
        let status = (self.calculate_crc32)(data.as_ptr(), data.len(), &mut crc);
        status.to_status_result_with(crc)
    }

    /// Copy memory
    ///
    /// # Safety
    /// The caller must ensure the memory regions are valid and don't overlap
    /// in a way that would cause undefined behavior.
    pub unsafe fn copy_mem(&self, dest: *mut u8, src: *const u8, len: usize) {
        (self.copy_mem)(dest, src, len)
    }

    /// Set memory to a value
    ///
    /// # Safety
    /// The caller must ensure the memory region is valid.
    pub unsafe fn set_mem(&self, buffer: *mut u8, size: usize, value: u8) {
        (self.set_mem)(buffer, size, value)
    }
}

impl core::fmt::Debug for EfiBootServices {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EfiBootServices")
            .field("hdr", &self.hdr)
            .finish()
    }
}

// Safety: EfiBootServices is only accessed in single-threaded boot context
unsafe impl Send for EfiBootServices {}
unsafe impl Sync for EfiBootServices {}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_services_signature() {
        assert_eq!(EfiBootServices::SIGNATURE, 0x56524553544F4F42);
    }
}
