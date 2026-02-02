//! Boot Services Wrapper
//!
//! Safe wrappers for UEFI Boot Services.

use core::ptr::NonNull;

use crate::raw::boot_services::EfiBootServices;
use crate::raw::memory::{MemoryDescriptor, MemoryMapKey, MemoryType};
use crate::raw::types::*;

// =============================================================================
// BOOT SERVICES
// =============================================================================

/// Boot services wrapper
pub struct BootServices {
    inner: NonNull<EfiBootServices>,
}

impl BootServices {
    /// Create from raw pointer
    ///
    /// # Safety
    /// Pointer must be valid and boot services must be available.
    pub unsafe fn from_ptr(ptr: *mut EfiBootServices) -> Option<Self> {
        NonNull::new(ptr).map(|inner| Self { inner })
    }

    /// Get the raw boot services pointer
    pub fn as_ptr(&self) -> *mut EfiBootServices {
        self.inner.as_ptr()
    }

    /// Get boot services reference
    fn bs(&self) -> &EfiBootServices {
        unsafe { self.inner.as_ref() }
    }
}

// =============================================================================
// TPL SERVICES
// =============================================================================

impl BootServices {
    /// Raise task priority level
    ///
    /// # Safety
    /// Caller must restore TPL properly.
    pub unsafe fn raise_tpl(&self, new_tpl: Tpl) -> Tpl {
        (self.bs().raise_tpl)(new_tpl)
    }

    /// Restore task priority level
    ///
    /// # Safety
    /// Must only be called with a TPL returned by raise_tpl.
    pub unsafe fn restore_tpl(&self, old_tpl: Tpl) {
        (self.bs().restore_tpl)(old_tpl)
    }

    /// Execute code at raised TPL
    pub fn with_tpl<F, R>(&self, tpl: Tpl, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        unsafe {
            let old = self.raise_tpl(tpl);
            let result = f();
            self.restore_tpl(old);
            result
        }
    }
}

// =============================================================================
// MEMORY SERVICES
// =============================================================================

impl BootServices {
    /// Allocate pool memory
    pub fn allocate_pool(&self, memory_type: MemoryType, size: usize) -> Result<*mut u8, Status> {
        let mut buffer: *mut u8 = core::ptr::null_mut();

        let status = unsafe { (self.bs().allocate_pool)(memory_type, size, &mut buffer) };

        status.to_status_result_with(buffer)
    }

    /// Free pool memory
    pub fn free_pool(&self, buffer: *mut u8) -> Result<(), Status> {
        if buffer.is_null() {
            return Ok(());
        }

        let status = unsafe { (self.bs().free_pool)(buffer) };

        status.to_status_result()
    }

    /// Allocate pages
    pub fn allocate_pages(
        &self,
        alloc_type: AllocateType,
        memory_type: MemoryType,
        pages: usize,
        address: &mut PhysicalAddress,
    ) -> Result<(), Status> {
        let status = unsafe { (self.bs().allocate_pages)(alloc_type, memory_type, pages, address) };
        status.to_status_result()
    }

    /// Free pages
    pub fn free_pages(&self, address: PhysicalAddress, pages: usize) -> Result<(), Status> {
        let status = unsafe { (self.bs().free_pages)(address, pages) };
        status.to_status_result()
    }

    /// Get memory map
    pub fn get_memory_map(&self, buffer: &mut [u8]) -> Result<MemoryMapInfo, Status> {
        let mut map_size = buffer.len();
        let mut key_val: usize = 0;
        let mut descriptor_size = 0;
        let mut descriptor_version = 0;

        let status = unsafe {
            (self.bs().get_memory_map)(
                &mut map_size,
                buffer.as_mut_ptr() as *mut MemoryDescriptor,
                &mut key_val,
                &mut descriptor_size,
                &mut descriptor_version,
            )
        };

        status.to_status_result_with(MemoryMapInfo {
            map_size,
            key: MemoryMapKey(key_val),
            descriptor_size,
            descriptor_version,
        })
    }
}

/// Memory map information
#[derive(Debug, Clone, Copy)]
pub struct MemoryMapInfo {
    /// Size of the memory map
    pub map_size: usize,
    /// Memory map key
    pub key: MemoryMapKey,
    /// Size of each descriptor
    pub descriptor_size: usize,
    /// Descriptor version
    pub descriptor_version: u32,
}

impl MemoryMapInfo {
    /// Get number of entries
    pub fn entry_count(&self) -> usize {
        self.map_size / self.descriptor_size
    }
}

// =============================================================================
// EVENT SERVICES
// =============================================================================

impl BootServices {
    /// Create an event
    pub fn create_event(
        &self,
        event_type: u32,
        notify_tpl: Tpl,
        notify_function: Option<EventNotify>,
        notify_context: *mut core::ffi::c_void,
    ) -> Result<Event, Status> {
        let mut event = Event::null();

        let status = unsafe {
            (self.bs().create_event)(
                event_type,
                notify_tpl,
                notify_function,
                notify_context,
                &mut event,
            )
        };

        status.to_status_result_with(event)
    }

    /// Create a timer event
    pub fn create_timer_event(&self) -> Result<Event, Status> {
        self.create_event(EventType::TIMER, TPL_CALLBACK, None, core::ptr::null_mut())
    }

    /// Close an event
    pub fn close_event(&self, event: Event) -> Result<(), Status> {
        let status = unsafe { (self.bs().close_event)(event) };
        status.to_status_result()
    }

    /// Signal an event
    pub fn signal_event(&self, event: Event) -> Result<(), Status> {
        let status = unsafe { (self.bs().signal_event)(event) };
        status.to_status_result()
    }

    /// Wait for events
    pub fn wait_for_event(&self, events: &[Event]) -> Result<usize, Status> {
        let mut index = 0;

        let status =
            unsafe { (self.bs().wait_for_event)(events.len(), events.as_ptr(), &mut index) };

        status.to_status_result_with(index)
    }

    /// Check if event is signaled
    pub fn check_event(&self, event: Event) -> Result<bool, Status> {
        let status = unsafe { (self.bs().check_event)(event) };

        match status {
            Status::SUCCESS => Ok(true),
            Status::NOT_READY => Ok(false),
            _ => Err(status),
        }
    }

    /// Set timer
    pub fn set_timer(
        &self,
        event: Event,
        timer_type: TimerDelay,
        trigger_time: u64,
    ) -> Result<(), Status> {
        let status = unsafe { (self.bs().set_timer)(event, timer_type, trigger_time) };
        status.to_status_result()
    }
}

/// Event notify function type
pub type EventNotify = unsafe extern "efiapi" fn(Event, *mut core::ffi::c_void);

/// Event type flags
pub struct EventType;

impl EventType {
    /// Timer event
    pub const TIMER: u32 = 0x80000000;
    /// Runtime event
    pub const RUNTIME: u32 = 0x40000000;
    /// Notify wait
    pub const NOTIFY_WAIT: u32 = 0x00000100;
    /// Notify signal
    pub const NOTIFY_SIGNAL: u32 = 0x00000200;
    /// Signal exit boot services
    pub const SIGNAL_EXIT_BOOT_SERVICES: u32 = 0x00000201;
    /// Signal virtual address change
    pub const SIGNAL_VIRTUAL_ADDRESS_CHANGE: u32 = 0x60000202;
}

/// Task priority levels
pub const TPL_APPLICATION: Tpl = 4;
pub const TPL_CALLBACK: Tpl = 8;
pub const TPL_NOTIFY: Tpl = 16;
pub const TPL_HIGH_LEVEL: Tpl = 31;

// =============================================================================
// PROTOCOL SERVICES
// =============================================================================

impl BootServices {
    /// Locate handle by protocol
    pub fn locate_handle(
        &self,
        search_type: LocateSearchType,
        protocol: Option<&Guid>,
        search_key: *mut core::ffi::c_void,
        buffer: &mut [Handle],
    ) -> Result<usize, Status> {
        let mut buffer_size = buffer.len() * core::mem::size_of::<Handle>();

        let status = unsafe {
            (self.bs().locate_handle)(
                search_type,
                protocol
                    .map(|g| g as *const Guid)
                    .unwrap_or(core::ptr::null()),
                search_key,
                &mut buffer_size,
                buffer.as_mut_ptr(),
            )
        };

        status.to_status_result_with(buffer_size / core::mem::size_of::<Handle>())
    }

    /// Locate protocol
    pub fn locate_protocol<T>(&self, protocol: &Guid) -> Result<*mut T, Status> {
        let mut interface: *mut core::ffi::c_void = core::ptr::null_mut();

        let status =
            unsafe { (self.bs().locate_protocol)(protocol, core::ptr::null_mut(), &mut interface) };

        status.to_status_result_with(interface as *mut T)
    }

    /// Handle protocol
    pub fn handle_protocol<T>(&self, handle: Handle, protocol: &Guid) -> Result<*mut T, Status> {
        let mut interface: *mut core::ffi::c_void = core::ptr::null_mut();

        let status = unsafe { (self.bs().handle_protocol)(handle, protocol, &mut interface) };

        status.to_status_result_with(interface as *mut T)
    }

    /// Open protocol
    pub fn open_protocol<T>(
        &self,
        handle: Handle,
        protocol: &Guid,
        agent_handle: Handle,
        controller_handle: Handle,
        attributes: u32,
    ) -> Result<*mut T, Status> {
        let mut interface: *mut core::ffi::c_void = core::ptr::null_mut();

        let status = unsafe {
            (self.bs().open_protocol)(
                handle,
                protocol,
                &mut interface,
                agent_handle,
                controller_handle,
                attributes,
            )
        };

        status.to_status_result_with(interface as *mut T)
    }

    /// Close protocol
    pub fn close_protocol(
        &self,
        handle: Handle,
        protocol: &Guid,
        agent_handle: Handle,
        controller_handle: Handle,
    ) -> Result<(), Status> {
        let status = unsafe {
            (self.bs().close_protocol)(handle, protocol, agent_handle, controller_handle)
        };
        status.to_status_result()
    }

    /// Locate handle buffer
    pub fn locate_handle_buffer(
        &self,
        search_type: LocateSearchType,
        protocol: Option<&Guid>,
    ) -> Result<HandleBuffer, Status> {
        let mut count = 0;
        let mut buffer: *mut Handle = core::ptr::null_mut();

        let status = unsafe {
            (self.bs().locate_handle_buffer)(
                search_type,
                protocol
                    .map(|g| g as *const Guid)
                    .unwrap_or(core::ptr::null()),
                core::ptr::null_mut(),
                &mut count,
                &mut buffer,
            )
        };

        status.to_status_result_with(HandleBuffer { buffer, count })
    }

    /// Get protocols per handle
    pub fn protocols_per_handle(&self, handle: Handle) -> Result<ProtocolBuffer, Status> {
        let mut protocols: *mut *const Guid = core::ptr::null_mut();
        let mut count = 0;

        let status =
            unsafe { (self.bs().protocols_per_handle)(handle, &mut protocols, &mut count) };

        status.to_status_result_with(ProtocolBuffer { protocols, count })
    }
}

// Use LocateSearchType from raw::types

/// Handle buffer returned by locate_handle_buffer
pub struct HandleBuffer {
    buffer: *mut Handle,
    count: usize,
}

impl HandleBuffer {
    /// Get handles as slice
    pub fn handles(&self) -> &[Handle] {
        if self.buffer.is_null() || self.count == 0 {
            &[]
        } else {
            unsafe { core::slice::from_raw_parts(self.buffer, self.count) }
        }
    }

    /// Get number of handles
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Protocol buffer returned by protocols_per_handle
pub struct ProtocolBuffer {
    protocols: *mut *const Guid,
    count: usize,
}

impl ProtocolBuffer {
    /// Get protocols as slice
    pub fn protocols(&self) -> &[*const Guid] {
        if self.protocols.is_null() || self.count == 0 {
            &[]
        } else {
            unsafe { core::slice::from_raw_parts(self.protocols, self.count) }
        }
    }

    /// Get number of protocols
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

// =============================================================================
// IMAGE SERVICES
// =============================================================================

impl BootServices {
    /// Load an image
    pub fn load_image(
        &self,
        boot_policy: bool,
        parent_image_handle: Handle,
        device_path: *mut crate::raw::protocols::loaded_image::EfiDevicePathProtocol,
        source_buffer: Option<&[u8]>,
    ) -> Result<Handle, Status> {
        let mut image_handle = Handle::null();

        let (buffer, size) = match source_buffer {
            Some(buf) => (buf.as_ptr(), buf.len()),
            None => (core::ptr::null(), 0),
        };

        let status = unsafe {
            (self.bs().load_image)(
                boot_policy as u8,
                parent_image_handle,
                device_path as *const core::ffi::c_void,
                buffer,
                size,
                &mut image_handle,
            )
        };

        status.to_status_result_with(image_handle)
    }

    /// Start an image
    pub fn start_image(&self, image_handle: Handle) -> Result<(), Status> {
        let mut exit_data_size = 0;
        let mut exit_data: *mut u16 = core::ptr::null_mut();

        let status =
            unsafe { (self.bs().start_image)(image_handle, &mut exit_data_size, &mut exit_data) };

        status.to_status_result()
    }

    /// Unload an image
    pub fn unload_image(&self, image_handle: Handle) -> Result<(), Status> {
        let status = unsafe { (self.bs().unload_image)(image_handle) };
        status.to_status_result()
    }

    /// Exit from an image
    pub fn exit(&self, image_handle: Handle, exit_status: Status, exit_data: Option<&[u16]>) -> ! {
        let (data, size) = match exit_data {
            Some(d) => (d.as_ptr() as *mut u16, d.len()),
            None => (core::ptr::null_mut(), 0),
        };

        unsafe {
            (self.bs().exit)(image_handle, exit_status, size, data);
        }

        // Should never return
        loop {
            core::hint::spin_loop();
        }
    }

    /// Exit boot services
    ///
    /// # Safety
    /// This is the point of no return!
    pub unsafe fn exit_boot_services(
        &self,
        image_handle: Handle,
        map_key: MemoryMapKey,
    ) -> Result<(), Status> {
        let status = (self.bs().exit_boot_services)(image_handle, map_key.0);
        status.to_status_result()
    }
}

// =============================================================================
// MISC SERVICES
// =============================================================================

impl BootServices {
    /// Stall for microseconds
    pub fn stall(&self, microseconds: usize) -> Result<(), Status> {
        let status = unsafe { (self.bs().stall)(microseconds) };
        status.to_status_result()
    }

    /// Set watchdog timer
    pub fn set_watchdog_timer(
        &self,
        timeout: usize,
        watchdog_code: u64,
        data: Option<&[u16]>,
    ) -> Result<(), Status> {
        let (data_ptr, data_size) = match data {
            Some(d) => (d.as_ptr(), d.len()),
            None => (core::ptr::null(), 0),
        };

        let status =
            unsafe { (self.bs().set_watchdog_timer)(timeout, watchdog_code, data_size, data_ptr) };

        status.to_status_result()
    }

    /// Disable watchdog timer
    pub fn disable_watchdog(&self) -> Result<(), Status> {
        self.set_watchdog_timer(0, 0, None)
    }

    /// Copy memory
    pub fn copy_mem(&self, dest: *mut u8, src: *const u8, length: usize) {
        unsafe {
            (self.bs().copy_mem)(dest, src, length);
        }
    }

    /// Set memory
    pub fn set_mem(&self, buffer: *mut u8, size: usize, value: u8) {
        unsafe {
            (self.bs().set_mem)(buffer, size, value);
        }
    }

    /// Get next monotonic count
    pub fn get_next_monotonic_count(&self) -> Result<u64, Status> {
        let mut count = 0u64;
        let status = unsafe { (self.bs().get_next_monotonic_count)(&mut count) };
        status.to_status_result_with(count)
    }

    /// Install configuration table
    pub fn install_configuration_table(
        &self,
        guid: &Guid,
        table: *mut core::ffi::c_void,
    ) -> Result<(), Status> {
        let status = unsafe { (self.bs().install_configuration_table)(guid, table) };
        status.to_status_result()
    }

    /// Calculate CRC32
    pub fn calculate_crc32(&self, data: &[u8]) -> Result<u32, Status> {
        let mut crc = 0u32;
        let status = unsafe { (self.bs().calculate_crc32)(data.as_ptr(), data.len(), &mut crc) };
        status.to_status_result_with(crc)
    }
}

// =============================================================================
// GLOBAL ACCESS
// =============================================================================

/// Get boot services from global state
///
/// # Safety
/// Must only be called after initialization and before ExitBootServices.
pub unsafe fn boot_services() -> BootServices {
    BootServices::from_ptr(super::boot_services() as *const _ as *mut _)
        .expect("Boot services not available")
}
