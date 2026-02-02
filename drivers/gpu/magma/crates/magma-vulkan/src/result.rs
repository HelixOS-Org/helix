//! # Vulkan Result Codes
//!
//! VkResult definition and error handling.

use core::fmt;

// =============================================================================
// VK RESULT
// =============================================================================

/// Vulkan result code
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VkResult {
    // Success codes
    /// Command successfully completed
    Success = 0,
    /// A fence or query has not yet completed
    NotReady = 1,
    /// A wait operation has not completed in the specified time
    Timeout = 2,
    /// An event is signaled
    EventSet = 3,
    /// An event is unsignaled
    EventReset = 4,
    /// A return array was too small
    Incomplete = 5,
    /// A swapchain no longer matches the surface properties
    SuboptimalKhr = 1000001003,

    // Error codes
    /// Out of host memory
    ErrorOutOfHostMemory = -1,
    /// Out of device memory
    ErrorOutOfDeviceMemory = -2,
    /// Initialization failed
    ErrorInitializationFailed = -3,
    /// Device lost
    ErrorDeviceLost = -4,
    /// Memory mapping failed
    ErrorMemoryMapFailed = -5,
    /// Layer not present
    ErrorLayerNotPresent = -6,
    /// Extension not present
    ErrorExtensionNotPresent = -7,
    /// Feature not present
    ErrorFeatureNotPresent = -8,
    /// Incompatible driver
    ErrorIncompatibleDriver = -9,
    /// Too many objects
    ErrorTooManyObjects = -10,
    /// Format not supported
    ErrorFormatNotSupported = -11,
    /// Fragmented pool
    ErrorFragmentedPool = -12,
    /// Unknown error
    ErrorUnknown = -13,
    /// Out of pool memory
    ErrorOutOfPoolMemory = -1000069000,
    /// Invalid external handle
    ErrorInvalidExternalHandle = -1000072003,
    /// Fragmentation
    ErrorFragmentation = -1000161000,
    /// Invalid opaque capture address
    ErrorInvalidOpaqueCaptureAddress = -1000257000,
    /// Surface lost
    ErrorSurfaceLostKhr = -1000000000,
    /// Native window in use
    ErrorNativeWindowInUseKhr = -1000000001,
    /// Out of date
    ErrorOutOfDateKhr = -1000001004,
    /// Incompatible display
    ErrorIncompatibleDisplayKhr = -1000003001,
    /// Validation failed
    ErrorValidationFailedExt = -1000011001,
    /// Invalid shader
    ErrorInvalidShaderNv = -1000012000,
    /// Full screen exclusive mode lost
    ErrorFullScreenExclusiveModeLostExt = -1000255000,
}

impl VkResult {
    /// Check if result is a success code
    pub fn is_success(&self) -> bool {
        (*self as i32) >= 0
    }

    /// Check if result is an error code
    pub fn is_error(&self) -> bool {
        (*self as i32) < 0
    }

    /// Convert to raw i32
    pub fn as_raw(&self) -> i32 {
        *self as i32
    }

    /// Create from raw i32
    pub fn from_raw(value: i32) -> Self {
        match value {
            0 => Self::Success,
            1 => Self::NotReady,
            2 => Self::Timeout,
            3 => Self::EventSet,
            4 => Self::EventReset,
            5 => Self::Incomplete,
            -1 => Self::ErrorOutOfHostMemory,
            -2 => Self::ErrorOutOfDeviceMemory,
            -3 => Self::ErrorInitializationFailed,
            -4 => Self::ErrorDeviceLost,
            -5 => Self::ErrorMemoryMapFailed,
            -6 => Self::ErrorLayerNotPresent,
            -7 => Self::ErrorExtensionNotPresent,
            -8 => Self::ErrorFeatureNotPresent,
            -9 => Self::ErrorIncompatibleDriver,
            -10 => Self::ErrorTooManyObjects,
            -11 => Self::ErrorFormatNotSupported,
            -12 => Self::ErrorFragmentedPool,
            _ => Self::ErrorUnknown,
        }
    }
}

impl fmt::Display for VkResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "VK_SUCCESS"),
            Self::NotReady => write!(f, "VK_NOT_READY"),
            Self::Timeout => write!(f, "VK_TIMEOUT"),
            Self::EventSet => write!(f, "VK_EVENT_SET"),
            Self::EventReset => write!(f, "VK_EVENT_RESET"),
            Self::Incomplete => write!(f, "VK_INCOMPLETE"),
            Self::SuboptimalKhr => write!(f, "VK_SUBOPTIMAL_KHR"),
            Self::ErrorOutOfHostMemory => write!(f, "VK_ERROR_OUT_OF_HOST_MEMORY"),
            Self::ErrorOutOfDeviceMemory => write!(f, "VK_ERROR_OUT_OF_DEVICE_MEMORY"),
            Self::ErrorInitializationFailed => write!(f, "VK_ERROR_INITIALIZATION_FAILED"),
            Self::ErrorDeviceLost => write!(f, "VK_ERROR_DEVICE_LOST"),
            Self::ErrorMemoryMapFailed => write!(f, "VK_ERROR_MEMORY_MAP_FAILED"),
            Self::ErrorLayerNotPresent => write!(f, "VK_ERROR_LAYER_NOT_PRESENT"),
            Self::ErrorExtensionNotPresent => write!(f, "VK_ERROR_EXTENSION_NOT_PRESENT"),
            Self::ErrorFeatureNotPresent => write!(f, "VK_ERROR_FEATURE_NOT_PRESENT"),
            Self::ErrorIncompatibleDriver => write!(f, "VK_ERROR_INCOMPATIBLE_DRIVER"),
            Self::ErrorTooManyObjects => write!(f, "VK_ERROR_TOO_MANY_OBJECTS"),
            Self::ErrorFormatNotSupported => write!(f, "VK_ERROR_FORMAT_NOT_SUPPORTED"),
            Self::ErrorFragmentedPool => write!(f, "VK_ERROR_FRAGMENTED_POOL"),
            Self::ErrorUnknown => write!(f, "VK_ERROR_UNKNOWN"),
            _ => write!(f, "VK_ERROR_UNKNOWN ({})", *self as i32),
        }
    }
}

// =============================================================================
// RESULT HELPERS
// =============================================================================

/// Vulkan success variant
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VkSuccess {
    /// Complete success
    Complete,
    /// Not ready
    NotReady,
    /// Timeout
    Timeout,
    /// Event set
    EventSet,
    /// Event reset
    EventReset,
    /// Incomplete (array too small)
    Incomplete,
}

impl From<VkSuccess> for VkResult {
    fn from(s: VkSuccess) -> Self {
        match s {
            VkSuccess::Complete => VkResult::Success,
            VkSuccess::NotReady => VkResult::NotReady,
            VkSuccess::Timeout => VkResult::Timeout,
            VkSuccess::EventSet => VkResult::EventSet,
            VkSuccess::EventReset => VkResult::EventReset,
            VkSuccess::Incomplete => VkResult::Incomplete,
        }
    }
}

/// Vulkan error variant
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VkError {
    /// Out of host memory
    OutOfHostMemory,
    /// Out of device memory
    OutOfDeviceMemory,
    /// Initialization failed
    InitializationFailed,
    /// Device lost
    DeviceLost,
    /// Memory map failed
    MemoryMapFailed,
    /// Layer not present
    LayerNotPresent,
    /// Extension not present
    ExtensionNotPresent,
    /// Feature not present
    FeatureNotPresent,
    /// Incompatible driver
    IncompatibleDriver,
    /// Too many objects
    TooManyObjects,
    /// Format not supported
    FormatNotSupported,
    /// Fragmented pool
    FragmentedPool,
    /// Unknown error
    Unknown,
}

impl From<VkError> for VkResult {
    fn from(e: VkError) -> Self {
        match e {
            VkError::OutOfHostMemory => VkResult::ErrorOutOfHostMemory,
            VkError::OutOfDeviceMemory => VkResult::ErrorOutOfDeviceMemory,
            VkError::InitializationFailed => VkResult::ErrorInitializationFailed,
            VkError::DeviceLost => VkResult::ErrorDeviceLost,
            VkError::MemoryMapFailed => VkResult::ErrorMemoryMapFailed,
            VkError::LayerNotPresent => VkResult::ErrorLayerNotPresent,
            VkError::ExtensionNotPresent => VkResult::ErrorExtensionNotPresent,
            VkError::FeatureNotPresent => VkResult::ErrorFeatureNotPresent,
            VkError::IncompatibleDriver => VkResult::ErrorIncompatibleDriver,
            VkError::TooManyObjects => VkResult::ErrorTooManyObjects,
            VkError::FormatNotSupported => VkResult::ErrorFormatNotSupported,
            VkError::FragmentedPool => VkResult::ErrorFragmentedPool,
            VkError::Unknown => VkResult::ErrorUnknown,
        }
    }
}

impl From<magma_core::Error> for VkError {
    fn from(e: magma_core::Error) -> Self {
        match e {
            magma_core::Error::OutOfMemory => VkError::OutOfDeviceMemory,
            magma_core::Error::NotFound => VkError::Unknown,
            magma_core::Error::InvalidParameter => VkError::Unknown,
            magma_core::Error::NotSupported => VkError::FeatureNotPresent,
            magma_core::Error::NotInitialized => VkError::InitializationFailed,
            magma_core::Error::Timeout => VkError::Unknown,
            _ => VkError::Unknown,
        }
    }
}
