//! # Vulkan Entry Points
//!
//! Global entry points and function pointer loading.

use core::ffi::{c_char, c_void, CStr};

use crate::extensions::{VkExtensionProperties, VkLayerProperties};
use crate::result::VkResult;
use crate::types::VkInstanceHandle;

// =============================================================================
// LOADER INTERFACE
// =============================================================================

/// Function pointer type for Vulkan functions
pub type PfnVkVoidFunction = Option<unsafe extern "C" fn()>;

/// vkGetInstanceProcAddr signature
pub type PfnVkGetInstanceProcAddr =
    unsafe extern "C" fn(instance: VkInstanceHandle, p_name: *const c_char) -> PfnVkVoidFunction;

/// Global function table
#[repr(C)]
pub struct VkGlobalDispatch {
    /// vkEnumerateInstanceVersion
    pub enumerate_instance_version: Option<unsafe extern "C" fn(*mut u32) -> VkResult>,
    /// vkEnumerateInstanceExtensionProperties
    pub enumerate_instance_extension_properties: Option<
        unsafe extern "C" fn(
            *const c_char,
            *mut u32,
            *mut VkExtensionProperties,
        ) -> VkResult,
    >,
    /// vkEnumerateInstanceLayerProperties
    pub enumerate_instance_layer_properties:
        Option<unsafe extern "C" fn(*mut u32, *mut VkLayerProperties) -> VkResult>,
    /// vkCreateInstance
    pub create_instance: Option<
        unsafe extern "C" fn(
            *const VkInstanceCreateInfo,
            *const c_void,
            *mut VkInstanceHandle,
        ) -> VkResult,
    >,
}

impl Default for VkGlobalDispatch {
    fn default() -> Self {
        Self::new()
    }
}

impl VkGlobalDispatch {
    /// Create new global dispatch table with MAGMA implementations
    pub const fn new() -> Self {
        Self {
            enumerate_instance_version: Some(magma_enumerate_instance_version),
            enumerate_instance_extension_properties: Some(
                magma_enumerate_instance_extension_properties,
            ),
            enumerate_instance_layer_properties: Some(magma_enumerate_instance_layer_properties),
            create_instance: Some(magma_create_instance),
        }
    }
}

// =============================================================================
// INSTANCE CREATE INFO
// =============================================================================

/// Application info structure
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkApplicationInfo {
    /// Structure type (VK_STRUCTURE_TYPE_APPLICATION_INFO = 0)
    pub s_type: u32,
    /// Next structure in chain
    pub p_next: *const c_void,
    /// Application name
    pub p_application_name: *const c_char,
    /// Application version
    pub application_version: u32,
    /// Engine name
    pub p_engine_name: *const c_char,
    /// Engine version
    pub engine_version: u32,
    /// API version
    pub api_version: u32,
}

/// Instance create info structure
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkInstanceCreateInfo {
    /// Structure type (VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO = 1)
    pub s_type: u32,
    /// Next structure in chain
    pub p_next: *const c_void,
    /// Flags (reserved)
    pub flags: u32,
    /// Application info
    pub p_application_info: *const VkApplicationInfo,
    /// Enabled layer count
    pub enabled_layer_count: u32,
    /// Enabled layer names
    pub pp_enabled_layer_names: *const *const c_char,
    /// Enabled extension count
    pub enabled_extension_count: u32,
    /// Enabled extension names
    pub pp_enabled_extension_names: *const *const c_char,
}

// =============================================================================
// GLOBAL FUNCTIONS IMPLEMENTATION
// =============================================================================

/// vkEnumerateInstanceVersion implementation
///
/// # Safety
/// - `p_api_version` must be a valid pointer to u32
unsafe extern "C" fn magma_enumerate_instance_version(p_api_version: *mut u32) -> VkResult {
    if p_api_version.is_null() {
        return VkResult::ErrorInitializationFailed;
    }

    // MAGMA supports Vulkan 1.3
    unsafe {
        *p_api_version = crate::types::VK_API_VERSION_1_3;
    }
    VkResult::Success
}

/// vkEnumerateInstanceExtensionProperties implementation
///
/// # Safety
/// - If `p_properties` is not null, it must point to valid memory
/// - `p_property_count` must be a valid pointer
unsafe extern "C" fn magma_enumerate_instance_extension_properties(
    p_layer_name: *const c_char,
    p_property_count: *mut u32,
    p_properties: *mut VkExtensionProperties,
) -> VkResult {
    if p_property_count.is_null() {
        return VkResult::ErrorInitializationFailed;
    }

    // If layer is specified, return no extensions (we don't support layers)
    if !p_layer_name.is_null() {
        let layer = unsafe { CStr::from_ptr(p_layer_name) };
        if !layer.to_bytes().is_empty() {
            unsafe {
                *p_property_count = 0;
            }
            return VkResult::Success;
        }
    }

    let extensions = crate::extensions::get_instance_extensions();
    let count = extensions.len() as u32;

    if p_properties.is_null() {
        unsafe {
            *p_property_count = count;
        }
        return VkResult::Success;
    }

    let available = unsafe { *p_property_count };
    let to_copy = available.min(count) as usize;

    unsafe {
        core::ptr::copy_nonoverlapping(extensions.as_ptr(), p_properties, to_copy);
        *p_property_count = to_copy as u32;
    }

    if available < count {
        VkResult::Incomplete
    } else {
        VkResult::Success
    }
}

/// vkEnumerateInstanceLayerProperties implementation
///
/// # Safety
/// - `p_property_count` must be a valid pointer
unsafe extern "C" fn magma_enumerate_instance_layer_properties(
    p_property_count: *mut u32,
    _p_properties: *mut VkLayerProperties,
) -> VkResult {
    if p_property_count.is_null() {
        return VkResult::ErrorInitializationFailed;
    }

    // MAGMA doesn't support any layers
    unsafe {
        *p_property_count = 0;
    }
    VkResult::Success
}

/// vkCreateInstance implementation
///
/// # Safety
/// - All pointers must be valid
unsafe extern "C" fn magma_create_instance(
    _p_create_info: *const VkInstanceCreateInfo,
    _p_allocator: *const c_void,
    _p_instance: *mut VkInstanceHandle,
) -> VkResult {
    // TODO: Implement instance creation
    VkResult::ErrorInitializationFailed
}

// =============================================================================
// PROC ADDRESS
// =============================================================================

/// Get instance proc address
///
/// # Safety
/// - `p_name` must be a valid null-terminated string
pub unsafe fn get_instance_proc_addr(
    instance: VkInstanceHandle,
    p_name: *const c_char,
) -> PfnVkVoidFunction {
    if p_name.is_null() {
        return None;
    }

    let name = unsafe { CStr::from_ptr(p_name) };
    let name_bytes = name.to_bytes();

    // Global functions (instance = NULL)
    if instance.is_null() {
        match name_bytes {
            b"vkEnumerateInstanceVersion" => Some(unsafe {
                core::mem::transmute::<unsafe extern "C" fn(*mut u32) -> VkResult, unsafe extern "C" fn()>(
                    magma_enumerate_instance_version,
                )
            }),
            b"vkEnumerateInstanceExtensionProperties" => Some(unsafe {
                core::mem::transmute::<
                    unsafe extern "C" fn(
                        *const c_char,
                        *mut u32,
                        *mut VkExtensionProperties,
                    ) -> VkResult,
                    unsafe extern "C" fn(),
                >(magma_enumerate_instance_extension_properties)
            }),
            b"vkEnumerateInstanceLayerProperties" => Some(unsafe {
                core::mem::transmute::<
                    unsafe extern "C" fn(*mut u32, *mut VkLayerProperties) -> VkResult,
                    unsafe extern "C" fn(),
                >(magma_enumerate_instance_layer_properties)
            }),
            b"vkCreateInstance" => Some(unsafe {
                core::mem::transmute::<
                    unsafe extern "C" fn(
                        *const VkInstanceCreateInfo,
                        *const c_void,
                        *mut VkInstanceHandle,
                    ) -> VkResult,
                    unsafe extern "C" fn(),
                >(magma_create_instance)
            }),
            b"vkGetInstanceProcAddr" => {
                // Return ourselves
                Some(unsafe {
                    core::mem::transmute::<
                        unsafe fn(VkInstanceHandle, *const c_char) -> PfnVkVoidFunction,
                        unsafe extern "C" fn(),
                    >(get_instance_proc_addr)
                })
            }
            _ => None,
        }
    } else {
        // Instance-level functions
        // TODO: Return instance dispatch functions
        None
    }
}

// =============================================================================
// ICD MANIFEST
// =============================================================================

/// ICD interface version
pub const VK_ICD_VERSION: u32 = 6;

/// ICD manifest for loader
#[repr(C)]
pub struct VkIcdSurfaceBase {
    /// Platform type
    pub platform: u32,
}

/// Export the ICD entry point
/// This is what the Vulkan loader calls to initialize the driver
#[no_mangle]
pub unsafe extern "C" fn vk_icdGetInstanceProcAddr(
    instance: VkInstanceHandle,
    p_name: *const c_char,
) -> PfnVkVoidFunction {
    unsafe { get_instance_proc_addr(instance, p_name) }
}

/// Negotiate ICD interface version
#[no_mangle]
pub extern "C" fn vk_icdNegotiateLoaderICDInterfaceVersion(
    p_supported_version: *mut u32,
) -> VkResult {
    if p_supported_version.is_null() {
        return VkResult::ErrorInitializationFailed;
    }

    unsafe {
        if *p_supported_version > VK_ICD_VERSION {
            *p_supported_version = VK_ICD_VERSION;
        }
    }
    VkResult::Success
}
