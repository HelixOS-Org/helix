//! # Vulkan Instance
//!
//! VkInstance implementation for MAGMA.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void, CStr};
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::entry::{VkApplicationInfo, VkInstanceCreateInfo};
use crate::extensions::InstanceExtensions;
use crate::result::VkResult;
use crate::types::VkInstanceHandle;

// =============================================================================
// INSTANCE
// =============================================================================

/// Next instance ID
static NEXT_INSTANCE_ID: AtomicU32 = AtomicU32::new(1);

/// Vulkan instance
pub struct VkInstance {
    /// Instance ID
    id: u32,
    /// Application name
    app_name: Option<String>,
    /// Application version
    app_version: u32,
    /// Engine name
    engine_name: Option<String>,
    /// Engine version
    engine_version: u32,
    /// API version requested
    api_version: u32,
    /// Enabled extensions
    enabled_extensions: InstanceExtensions,
    /// Physical devices
    physical_devices: Vec<PhysicalDeviceInfo>,
    /// Debug messenger (if enabled)
    debug_messenger: Option<NonNull<c_void>>,
}

impl VkInstance {
    /// Create a new Vulkan instance
    ///
    /// # Safety
    /// - `create_info` must be a valid pointer to VkInstanceCreateInfo
    pub unsafe fn create(create_info: *const VkInstanceCreateInfo) -> Result<Box<Self>, VkResult> {
        if create_info.is_null() {
            return Err(VkResult::ErrorInitializationFailed);
        }

        let info = unsafe { &*create_info };

        // Parse application info
        let (app_name, app_version, engine_name, engine_version, api_version) =
            if !info.p_application_info.is_null() {
                let app_info = unsafe { &*info.p_application_info };
                let app_name = if !app_info.p_application_name.is_null() {
                    unsafe { CStr::from_ptr(app_info.p_application_name) }
                        .to_str()
                        .ok()
                        .map(String::from)
                } else {
                    None
                };
                let engine_name = if !app_info.p_engine_name.is_null() {
                    unsafe { CStr::from_ptr(app_info.p_engine_name) }
                        .to_str()
                        .ok()
                        .map(String::from)
                } else {
                    None
                };
                (
                    app_name,
                    app_info.application_version,
                    engine_name,
                    app_info.engine_version,
                    app_info.api_version,
                )
            } else {
                (None, 0, None, 0, crate::types::VK_API_VERSION_1_0)
            };

        // Validate API version
        if api_version > crate::types::VK_API_VERSION_1_3 {
            return Err(VkResult::ErrorIncompatibleDriver);
        }

        // Parse enabled extensions
        let mut enabled_extensions = InstanceExtensions::empty();
        if info.enabled_extension_count > 0 && !info.pp_enabled_extension_names.is_null() {
            for i in 0..info.enabled_extension_count {
                let ext_name_ptr = unsafe { *info.pp_enabled_extension_names.add(i as usize) };
                if !ext_name_ptr.is_null() {
                    let ext_name = unsafe { CStr::from_ptr(ext_name_ptr) };
                    match ext_name.to_bytes() {
                        b"VK_KHR_surface" => enabled_extensions |= InstanceExtensions::SURFACE,
                        b"VK_KHR_xcb_surface" => {
                            enabled_extensions |= InstanceExtensions::XCB_SURFACE
                        }
                        b"VK_KHR_xlib_surface" => {
                            enabled_extensions |= InstanceExtensions::XLIB_SURFACE
                        }
                        b"VK_KHR_wayland_surface" => {
                            enabled_extensions |= InstanceExtensions::WAYLAND_SURFACE
                        }
                        b"VK_KHR_get_physical_device_properties2" => {
                            enabled_extensions |= InstanceExtensions::GET_PHYSICAL_DEVICE_PROPERTIES_2
                        }
                        b"VK_EXT_debug_utils" => {
                            enabled_extensions |= InstanceExtensions::DEBUG_UTILS
                        }
                        _ => return Err(VkResult::ErrorExtensionNotPresent),
                    }
                }
            }
        }

        let instance = Box::new(Self {
            id: NEXT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed),
            app_name,
            app_version,
            engine_name,
            engine_version,
            api_version,
            enabled_extensions,
            physical_devices: Vec::new(),
            debug_messenger: None,
        });

        Ok(instance)
    }

    /// Get instance ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get application name
    pub fn app_name(&self) -> Option<&str> {
        self.app_name.as_deref()
    }

    /// Get API version
    pub fn api_version(&self) -> u32 {
        self.api_version
    }

    /// Get enabled extensions
    pub fn enabled_extensions(&self) -> InstanceExtensions {
        self.enabled_extensions
    }

    /// Check if extension is enabled
    pub fn is_extension_enabled(&self, ext: InstanceExtensions) -> bool {
        self.enabled_extensions.contains(ext)
    }

    /// Enumerate physical devices
    pub fn enumerate_physical_devices(&mut self) -> Result<&[PhysicalDeviceInfo], VkResult> {
        if self.physical_devices.is_empty() {
            // TODO: Actually probe for NVIDIA GPUs
            // For now, return empty list
        }
        Ok(&self.physical_devices)
    }

    /// Convert to handle
    pub fn to_handle(&self) -> VkInstanceHandle {
        self as *const _ as VkInstanceHandle
    }

    /// Convert from handle
    ///
    /// # Safety
    /// - Handle must be a valid VkInstance pointer
    pub unsafe fn from_handle(handle: VkInstanceHandle) -> Option<&'static Self> {
        if handle.is_null() {
            None
        } else {
            Some(unsafe { &*(handle as *const Self) })
        }
    }

    /// Convert from handle (mutable)
    ///
    /// # Safety
    /// - Handle must be a valid VkInstance pointer
    pub unsafe fn from_handle_mut(handle: VkInstanceHandle) -> Option<&'static mut Self> {
        if handle.is_null() {
            None
        } else {
            Some(unsafe { &mut *(handle as *mut Self) })
        }
    }
}

impl Drop for VkInstance {
    fn drop(&mut self) {
        log::debug!("Destroying VkInstance {}", self.id);
    }
}

// =============================================================================
// PHYSICAL DEVICE INFO
// =============================================================================

/// Physical device type
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VkPhysicalDeviceType {
    /// Other device type
    Other = 0,
    /// Integrated GPU
    IntegratedGpu = 1,
    /// Discrete GPU
    DiscreteGpu = 2,
    /// Virtual GPU
    VirtualGpu = 3,
    /// CPU
    Cpu = 4,
}

/// Maximum physical device name size
pub const VK_MAX_PHYSICAL_DEVICE_NAME_SIZE: usize = 256;
/// UUID size
pub const VK_UUID_SIZE: usize = 16;

/// Physical device info
#[derive(Clone)]
pub struct PhysicalDeviceInfo {
    /// Device name
    pub device_name: [u8; VK_MAX_PHYSICAL_DEVICE_NAME_SIZE],
    /// Device type
    pub device_type: VkPhysicalDeviceType,
    /// Vendor ID
    pub vendor_id: u32,
    /// Device ID
    pub device_id: u32,
    /// Driver version
    pub driver_version: u32,
    /// Pipeline cache UUID
    pub pipeline_cache_uuid: [u8; VK_UUID_SIZE],
    /// API version
    pub api_version: u32,
}

impl PhysicalDeviceInfo {
    /// Create new physical device info for NVIDIA GPU
    pub fn nvidia(device_id: u32, name: &str) -> Self {
        let mut device_name = [0u8; VK_MAX_PHYSICAL_DEVICE_NAME_SIZE];
        let name_bytes = name.as_bytes();
        let len = name_bytes.len().min(VK_MAX_PHYSICAL_DEVICE_NAME_SIZE - 1);
        device_name[..len].copy_from_slice(&name_bytes[..len]);

        Self {
            device_name,
            device_type: VkPhysicalDeviceType::DiscreteGpu,
            vendor_id: 0x10DE, // NVIDIA
            device_id,
            driver_version: make_driver_version(560, 0, 0),
            pipeline_cache_uuid: [0; VK_UUID_SIZE],
            api_version: crate::types::VK_API_VERSION_1_3,
        }
    }
}

/// Make NVIDIA driver version
const fn make_driver_version(major: u32, minor: u32, patch: u32) -> u32 {
    (major << 22) | (minor << 14) | (patch << 6)
}

// =============================================================================
// INSTANCE FUNCTIONS
// =============================================================================

/// vkDestroyInstance implementation
///
/// # Safety
/// - `instance` must be a valid VkInstance handle or null
pub unsafe fn destroy_instance(instance: VkInstanceHandle, _p_allocator: *const c_void) {
    if !instance.is_null() {
        let _ = unsafe { Box::from_raw(instance as *mut VkInstance) };
    }
}

/// vkEnumeratePhysicalDevices implementation
///
/// # Safety
/// - All pointers must be valid
pub unsafe fn enumerate_physical_devices(
    instance: VkInstanceHandle,
    p_physical_device_count: *mut u32,
    p_physical_devices: *mut *mut c_void,
) -> VkResult {
    if p_physical_device_count.is_null() {
        return VkResult::ErrorInitializationFailed;
    }

    let Some(instance) = (unsafe { VkInstance::from_handle_mut(instance) }) else {
        return VkResult::ErrorInitializationFailed;
    };

    let devices = match instance.enumerate_physical_devices() {
        Ok(d) => d,
        Err(e) => return e,
    };

    let count = devices.len() as u32;

    if p_physical_devices.is_null() {
        unsafe {
            *p_physical_device_count = count;
        }
        return VkResult::Success;
    }

    let available = unsafe { *p_physical_device_count };
    let to_copy = available.min(count) as usize;

    for i in 0..to_copy {
        unsafe {
            *p_physical_devices.add(i) = &devices[i] as *const _ as *mut c_void;
        }
    }

    unsafe {
        *p_physical_device_count = to_copy as u32;
    }

    if available < count {
        VkResult::Incomplete
    } else {
        VkResult::Success
    }
}
