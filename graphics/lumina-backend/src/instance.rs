//! Backend Instance
//!
//! Backend initialization and global state.

use alloc::string::String;
use alloc::vec::Vec;

use bitflags::bitflags;

use crate::device::{Adapter, AdapterInfo, BackendType, Device, DeviceCapabilities, DeviceDesc};

// ============================================================================
// Instance Flags
// ============================================================================

bitflags! {
    /// Instance creation flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct InstanceFlags: u32 {
        /// Enable validation layers.
        const VALIDATION = 1 << 0;
        /// Enable debug utilities.
        const DEBUG_UTILS = 1 << 1;
        /// Enable GPU-assisted validation.
        const GPU_ASSISTED_VALIDATION = 1 << 2;
        /// Enable best practices validation.
        const BEST_PRACTICES_VALIDATION = 1 << 3;
        /// Enable synchronization validation.
        const SYNCHRONIZATION_VALIDATION = 1 << 4;
        /// Enable shader printf.
        const SHADER_PRINTF = 1 << 5;
    }
}

impl Default for InstanceFlags {
    fn default() -> Self {
        InstanceFlags::empty()
    }
}

// ============================================================================
// Instance Description
// ============================================================================

/// Description for instance creation.
#[derive(Debug, Clone)]
pub struct InstanceDesc {
    /// Application name.
    pub app_name: String,
    /// Application version.
    pub app_version: u32,
    /// Engine name.
    pub engine_name: String,
    /// Engine version.
    pub engine_version: u32,
    /// Backend type.
    pub backend: BackendType,
    /// Instance flags.
    pub flags: InstanceFlags,
    /// Debug label.
    pub label: Option<String>,
}

impl Default for InstanceDesc {
    fn default() -> Self {
        Self {
            app_name: String::from("LUMINA Application"),
            app_version: 1,
            engine_name: String::from("LUMINA"),
            engine_version: 1,
            backend: BackendType::Vulkan,
            flags: InstanceFlags::empty(),
            label: None,
        }
    }
}

impl InstanceDesc {
    /// Create a new instance description.
    pub fn new(app_name: &str, backend: BackendType) -> Self {
        Self {
            app_name: String::from(app_name),
            backend,
            ..Default::default()
        }
    }

    /// Enable validation.
    pub fn with_validation(mut self) -> Self {
        self.flags |= InstanceFlags::VALIDATION;
        self
    }

    /// Enable debug utilities.
    pub fn with_debug(mut self) -> Self {
        self.flags |= InstanceFlags::DEBUG_UTILS | InstanceFlags::VALIDATION;
        self
    }

    /// Enable all validation.
    pub fn with_full_validation(mut self) -> Self {
        self.flags |= InstanceFlags::VALIDATION
            | InstanceFlags::DEBUG_UTILS
            | InstanceFlags::GPU_ASSISTED_VALIDATION
            | InstanceFlags::BEST_PRACTICES_VALIDATION
            | InstanceFlags::SYNCHRONIZATION_VALIDATION;
        self
    }
}

// ============================================================================
// Version
// ============================================================================

/// Version number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    /// Major version.
    pub major: u32,
    /// Minor version.
    pub minor: u32,
    /// Patch version.
    pub patch: u32,
}

impl Version {
    /// Create a new version.
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Create from packed value.
    pub const fn from_packed(packed: u32) -> Self {
        Self {
            major: (packed >> 22) & 0x7F,
            minor: (packed >> 12) & 0x3FF,
            patch: packed & 0xFFF,
        }
    }

    /// Pack into u32.
    pub const fn to_packed(&self) -> u32 {
        ((self.major & 0x7F) << 22) | ((self.minor & 0x3FF) << 12) | (self.patch & 0xFFF)
    }
}

impl core::fmt::Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// ============================================================================
// Instance Info
// ============================================================================

/// Instance information.
#[derive(Debug, Clone)]
pub struct InstanceInfo {
    /// Backend type.
    pub backend: BackendType,
    /// API version.
    pub api_version: Version,
    /// Driver version.
    pub driver_version: Version,
    /// Instance extensions.
    pub extensions: Vec<String>,
    /// Instance layers.
    pub layers: Vec<String>,
}

impl Default for InstanceInfo {
    fn default() -> Self {
        Self {
            backend: BackendType::Null,
            api_version: Version::new(1, 0, 0),
            driver_version: Version::new(1, 0, 0),
            extensions: Vec::new(),
            layers: Vec::new(),
        }
    }
}

// ============================================================================
// Instance
// ============================================================================

/// Backend instance.
pub struct Instance {
    /// Backend type.
    pub backend: BackendType,
    /// Instance flags.
    pub flags: InstanceFlags,
    /// Instance info.
    pub info: InstanceInfo,
    /// Available adapters.
    pub adapters: Vec<Adapter>,
    /// Debug label.
    pub label: Option<String>,
}

impl Instance {
    /// Create a new instance.
    pub fn new(desc: &InstanceDesc) -> Self {
        Self {
            backend: desc.backend,
            flags: desc.flags,
            info: InstanceInfo {
                backend: desc.backend,
                ..Default::default()
            },
            adapters: Vec::new(),
            label: desc.label.clone(),
        }
    }

    /// Check if validation is enabled.
    pub fn validation_enabled(&self) -> bool {
        self.flags.contains(InstanceFlags::VALIDATION)
    }

    /// Check if debug utils are enabled.
    pub fn debug_enabled(&self) -> bool {
        self.flags.contains(InstanceFlags::DEBUG_UTILS)
    }

    /// Enumerate adapters.
    pub fn enumerate_adapters(&self) -> &[Adapter] {
        &self.adapters
    }

    /// Get adapter count.
    pub fn adapter_count(&self) -> usize {
        self.adapters.len()
    }

    /// Get adapter by index.
    pub fn get_adapter(&self, index: usize) -> Option<&Adapter> {
        self.adapters.get(index)
    }

    /// Find best adapter.
    pub fn find_best_adapter(&self) -> Option<&Adapter> {
        self.adapters
            .iter()
            .filter(|a| a.is_discrete_gpu())
            .max_by_key(|a| a.info.dedicated_video_memory)
            .or_else(|| {
                self.adapters
                    .iter()
                    .filter(|a| a.is_integrated_gpu())
                    .max_by_key(|a| a.info.dedicated_video_memory)
            })
            .or_else(|| self.adapters.first())
    }

    /// Find adapter by vendor.
    pub fn find_adapter_by_vendor(&self, vendor: crate::device::VendorId) -> Option<&Adapter> {
        self.adapters.iter().find(|a| a.info.vendor_id == vendor)
    }

    /// Request device from adapter.
    pub fn request_device(&self, adapter: &Adapter, desc: &DeviceDesc) -> Option<Device> {
        let _ = (adapter, desc);
        // Backend-specific implementation
        None
    }
}

// ============================================================================
// Backend Factory
// ============================================================================

/// Backend factory for creating instances.
pub struct BackendFactory;

impl BackendFactory {
    /// Get available backends.
    pub fn available_backends() -> Vec<BackendInfo> {
        vec![
            BackendInfo {
                backend_type: BackendType::Vulkan,
                name: String::from("Vulkan"),
                available: cfg!(feature = "vulkan"),
                preferred: true,
            },
            BackendInfo {
                backend_type: BackendType::Metal,
                name: String::from("Metal"),
                available: cfg!(target_os = "macos") || cfg!(target_os = "ios"),
                preferred: cfg!(target_os = "macos"),
            },
            BackendInfo {
                backend_type: BackendType::Dx12,
                name: String::from("DirectX 12"),
                available: cfg!(target_os = "windows") && cfg!(feature = "dx12"),
                preferred: false,
            },
            BackendInfo {
                backend_type: BackendType::WebGpu,
                name: String::from("WebGPU"),
                available: cfg!(target_arch = "wasm32"),
                preferred: cfg!(target_arch = "wasm32"),
            },
            BackendInfo {
                backend_type: BackendType::Magma,
                name: String::from("MAGMA"),
                available: true, // Native Helix GPU driver
                preferred: false,
            },
            BackendInfo {
                backend_type: BackendType::Null,
                name: String::from("Null"),
                available: true,
                preferred: false,
            },
        ]
    }

    /// Get preferred backend.
    pub fn preferred_backend() -> BackendType {
        Self::available_backends()
            .iter()
            .filter(|b| b.available && b.preferred)
            .map(|b| b.backend_type)
            .next()
            .unwrap_or(BackendType::Null)
    }

    /// Check if backend is available.
    pub fn is_available(backend: BackendType) -> bool {
        Self::available_backends()
            .iter()
            .find(|b| b.backend_type == backend)
            .map(|b| b.available)
            .unwrap_or(false)
    }

    /// Create instance for backend.
    pub fn create_instance(desc: &InstanceDesc) -> Option<Instance> {
        if !Self::is_available(desc.backend) {
            return None;
        }
        Some(Instance::new(desc))
    }
}

// ============================================================================
// Backend Info
// ============================================================================

/// Information about a backend.
#[derive(Debug, Clone)]
pub struct BackendInfo {
    /// Backend type.
    pub backend_type: BackendType,
    /// Backend name.
    pub name: String,
    /// Whether backend is available.
    pub available: bool,
    /// Whether backend is preferred.
    pub preferred: bool,
}

// ============================================================================
// Instance Extensions
// ============================================================================

/// Common instance extensions.
pub mod extensions {
    /// Surface extension.
    pub const SURFACE: &str = "VK_KHR_surface";
    /// Debug utils extension.
    pub const DEBUG_UTILS: &str = "VK_EXT_debug_utils";
    /// Validation features.
    pub const VALIDATION_FEATURES: &str = "VK_EXT_validation_features";
    /// Get physical device properties 2.
    pub const GET_PHYSICAL_DEVICE_PROPERTIES_2: &str = "VK_KHR_get_physical_device_properties2";
    /// External memory capabilities.
    pub const EXTERNAL_MEMORY_CAPABILITIES: &str = "VK_KHR_external_memory_capabilities";
    /// External semaphore capabilities.
    pub const EXTERNAL_SEMAPHORE_CAPABILITIES: &str = "VK_KHR_external_semaphore_capabilities";
    /// External fence capabilities.
    pub const EXTERNAL_FENCE_CAPABILITIES: &str = "VK_KHR_external_fence_capabilities";
    /// Portability enumeration (for MoltenVK).
    pub const PORTABILITY_ENUMERATION: &str = "VK_KHR_portability_enumeration";

    // Platform-specific surface extensions
    #[cfg(target_os = "windows")]
    pub const WIN32_SURFACE: &str = "VK_KHR_win32_surface";
    #[cfg(target_os = "linux")]
    pub const XCB_SURFACE: &str = "VK_KHR_xcb_surface";
    #[cfg(target_os = "linux")]
    pub const XLIB_SURFACE: &str = "VK_KHR_xlib_surface";
    #[cfg(target_os = "linux")]
    pub const WAYLAND_SURFACE: &str = "VK_KHR_wayland_surface";
    #[cfg(target_os = "macos")]
    pub const METAL_SURFACE: &str = "VK_EXT_metal_surface";
    #[cfg(target_os = "android")]
    pub const ANDROID_SURFACE: &str = "VK_KHR_android_surface";
}

// ============================================================================
// Validation Layers
// ============================================================================

/// Common validation layers.
pub mod layers {
    /// Khronos validation layer.
    pub const KHRONOS_VALIDATION: &str = "VK_LAYER_KHRONOS_validation";
    /// LunarG standard validation.
    pub const LUNARG_STANDARD_VALIDATION: &str = "VK_LAYER_LUNARG_standard_validation";
    /// API dump layer.
    pub const API_DUMP: &str = "VK_LAYER_LUNARG_api_dump";
    /// Screenshot layer.
    pub const SCREENSHOT: &str = "VK_LAYER_LUNARG_screenshot";
    /// Monitor layer.
    pub const MONITOR: &str = "VK_LAYER_LUNARG_monitor";
}

// ============================================================================
// Debug Callback
// ============================================================================

/// Debug message severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugSeverity {
    /// Verbose.
    Verbose,
    /// Information.
    Info,
    /// Warning.
    Warning,
    /// Error.
    Error,
}

/// Debug message type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugMessageType {
    /// General message.
    General,
    /// Validation message.
    Validation,
    /// Performance message.
    Performance,
    /// Device address binding.
    DeviceAddressBinding,
}

/// Debug message.
#[derive(Debug, Clone)]
pub struct DebugMessage {
    /// Severity.
    pub severity: DebugSeverity,
    /// Message type.
    pub message_type: DebugMessageType,
    /// Message ID.
    pub message_id: i32,
    /// Message ID name.
    pub message_id_name: Option<String>,
    /// Message text.
    pub message: String,
    /// Queue labels.
    pub queue_labels: Vec<String>,
    /// Command buffer labels.
    pub cmd_buf_labels: Vec<String>,
    /// Object names.
    pub objects: Vec<String>,
}

/// Debug callback type.
pub type DebugCallback = fn(&DebugMessage);

/// Default debug callback.
pub fn default_debug_callback(message: &DebugMessage) {
    let severity = match message.severity {
        DebugSeverity::Verbose => "VERBOSE",
        DebugSeverity::Info => "INFO",
        DebugSeverity::Warning => "WARNING",
        DebugSeverity::Error => "ERROR",
    };
    let msg_type = match message.message_type {
        DebugMessageType::General => "GENERAL",
        DebugMessageType::Validation => "VALIDATION",
        DebugMessageType::Performance => "PERFORMANCE",
        DebugMessageType::DeviceAddressBinding => "DEVICE_ADDRESS",
    };

    // In a real implementation, this would log to the kernel's logging system
    let _ = (severity, msg_type, message);
}
