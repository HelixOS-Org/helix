//! # Vulkan Types
//!
//! Vulkan type definitions and handles.

use core::ffi::c_void;
use core::ptr::NonNull;

// =============================================================================
// BASIC TYPES
// =============================================================================

/// Vulkan boolean
pub type VkBool32 = u32;
/// Vulkan flags (32-bit)
pub type VkFlags = u32;
/// Vulkan flags (64-bit)
pub type VkFlags64 = u64;
/// Vulkan device size
pub type VkDeviceSize = u64;
/// Vulkan device address
pub type VkDeviceAddress = u64;
/// Sample mask
pub type VkSampleMask = u32;

/// True value
pub const VK_TRUE: VkBool32 = 1;
/// False value
pub const VK_FALSE: VkBool32 = 0;

// =============================================================================
// API VERSION
// =============================================================================

/// Create Vulkan API version
pub const fn vk_make_api_version(variant: u32, major: u32, minor: u32, patch: u32) -> u32 {
    (variant << 29) | (major << 22) | (minor << 12) | patch
}

/// Vulkan 1.0
pub const VK_API_VERSION_1_0: u32 = vk_make_api_version(0, 1, 0, 0);
/// Vulkan 1.1
pub const VK_API_VERSION_1_1: u32 = vk_make_api_version(0, 1, 1, 0);
/// Vulkan 1.2
pub const VK_API_VERSION_1_2: u32 = vk_make_api_version(0, 1, 2, 0);
/// Vulkan 1.3
pub const VK_API_VERSION_1_3: u32 = vk_make_api_version(0, 1, 3, 0);

/// MAGMA driver version
pub const MAGMA_DRIVER_VERSION: u32 = vk_make_api_version(0, 0, 1, 0);

// =============================================================================
// HANDLES
// =============================================================================

/// Vulkan handle trait
pub trait VkHandle: Copy + Clone {
    /// Null handle value
    const NULL: Self;

    /// Check if handle is null
    fn is_null(&self) -> bool;
}

/// Dispatchable handle (pointer-sized)
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct VkDispatchableHandle(pub *mut c_void);

impl VkHandle for VkDispatchableHandle {
    const NULL: Self = Self(core::ptr::null_mut());

    fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

impl Default for VkDispatchableHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// SAFETY: Handles are just pointers that can be sent across threads
unsafe impl Send for VkDispatchableHandle {}
unsafe impl Sync for VkDispatchableHandle {}

/// Non-dispatchable handle (64-bit)
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct VkNonDispatchableHandle(pub u64);

impl VkHandle for VkNonDispatchableHandle {
    const NULL: Self = Self(0);

    fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for VkNonDispatchableHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// =============================================================================
// CONCRETE HANDLES
// =============================================================================

macro_rules! define_handle {
    (dispatchable $name:ident) => {
        #[doc = concat!("Vulkan ", stringify!($name), " handle")]
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
        pub struct $name(pub VkDispatchableHandle);

        impl $name {
            /// Null handle
            pub const NULL: Self = Self(VkDispatchableHandle::NULL);

            /// Check if null
            pub fn is_null(&self) -> bool {
                self.0.is_null()
            }

            /// Create from raw pointer
            pub fn from_raw(ptr: *mut c_void) -> Self {
                Self(VkDispatchableHandle(ptr))
            }

            /// Get raw pointer
            pub fn as_raw(&self) -> *mut c_void {
                self.0.0
            }
        }

        unsafe impl Send for $name {}
        unsafe impl Sync for $name {}
    };
    (non_dispatchable $name:ident) => {
        #[doc = concat!("Vulkan ", stringify!($name), " handle")]
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
        pub struct $name(pub VkNonDispatchableHandle);

        impl $name {
            /// Null handle
            pub const NULL: Self = Self(VkNonDispatchableHandle::NULL);

            /// Check if null
            pub fn is_null(&self) -> bool {
                self.0.is_null()
            }

            /// Create from raw value
            pub fn from_raw(value: u64) -> Self {
                Self(VkNonDispatchableHandle(value))
            }

            /// Get raw value
            pub fn as_raw(&self) -> u64 {
                self.0.0
            }
        }

        unsafe impl Send for $name {}
        unsafe impl Sync for $name {}
    };
}

// Dispatchable handles
define_handle!(dispatchable VkInstanceHandle);
define_handle!(dispatchable VkPhysicalDeviceHandle);
define_handle!(dispatchable VkDeviceHandle);
define_handle!(dispatchable VkQueueHandle);
define_handle!(dispatchable VkCommandBufferHandle);

// Non-dispatchable handles
define_handle!(non_dispatchable VkSemaphoreHandle);
define_handle!(non_dispatchable VkFenceHandle);
define_handle!(non_dispatchable VkDeviceMemoryHandle);
define_handle!(non_dispatchable VkBufferHandle);
define_handle!(non_dispatchable VkImageHandle);
define_handle!(non_dispatchable VkEventHandle);
define_handle!(non_dispatchable VkQueryPoolHandle);
define_handle!(non_dispatchable VkBufferViewHandle);
define_handle!(non_dispatchable VkImageViewHandle);
define_handle!(non_dispatchable VkShaderModuleHandle);
define_handle!(non_dispatchable VkPipelineCacheHandle);
define_handle!(non_dispatchable VkPipelineLayoutHandle);
define_handle!(non_dispatchable VkRenderPassHandle);
define_handle!(non_dispatchable VkPipelineHandle);
define_handle!(non_dispatchable VkDescriptorSetLayoutHandle);
define_handle!(non_dispatchable VkSamplerHandle);
define_handle!(non_dispatchable VkDescriptorPoolHandle);
define_handle!(non_dispatchable VkDescriptorSetHandle);
define_handle!(non_dispatchable VkFramebufferHandle);
define_handle!(non_dispatchable VkCommandPoolHandle);

// =============================================================================
// STRUCTURES
// =============================================================================

/// Maximum physical device name length
pub const VK_MAX_PHYSICAL_DEVICE_NAME_SIZE: usize = 256;
/// UUID size
pub const VK_UUID_SIZE: usize = 16;
/// Maximum memory types
pub const VK_MAX_MEMORY_TYPES: usize = 32;
/// Maximum memory heaps
pub const VK_MAX_MEMORY_HEAPS: usize = 16;

/// Vulkan extent 2D
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VkExtent2D {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

/// Vulkan extent 3D
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VkExtent3D {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth
    pub depth: u32,
}

/// Vulkan offset 2D
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VkOffset2D {
    /// X coordinate
    pub x: i32,
    /// Y coordinate
    pub y: i32,
}

/// Vulkan offset 3D
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VkOffset3D {
    /// X coordinate
    pub x: i32,
    /// Y coordinate
    pub y: i32,
    /// Z coordinate
    pub z: i32,
}

/// Vulkan rect 2D
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VkRect2D {
    /// Offset
    pub offset: VkOffset2D,
    /// Extent
    pub extent: VkExtent2D,
}

/// Vulkan viewport
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VkViewport {
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
    /// Minimum depth
    pub min_depth: f32,
    /// Maximum depth
    pub max_depth: f32,
}

// =============================================================================
// STRUCTURE TYPE
// =============================================================================

/// Vulkan structure type
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VkStructureType {
    /// Application info
    ApplicationInfo = 0,
    /// Instance create info
    InstanceCreateInfo = 1,
    /// Device queue create info
    DeviceQueueCreateInfo = 2,
    /// Device create info
    DeviceCreateInfo = 3,
    /// Submit info
    SubmitInfo = 4,
    /// Memory allocate info
    MemoryAllocateInfo = 5,
    /// Mapped memory range
    MappedMemoryRange = 6,
    /// Fence create info
    FenceCreateInfo = 8,
    /// Semaphore create info
    SemaphoreCreateInfo = 9,
    /// Buffer create info
    BufferCreateInfo = 12,
    /// Image create info
    ImageCreateInfo = 14,
    /// Image view create info
    ImageViewCreateInfo = 15,
    /// Shader module create info
    ShaderModuleCreateInfo = 16,
    /// Pipeline cache create info
    PipelineCacheCreateInfo = 17,
    /// Graphics pipeline create info
    GraphicsPipelineCreateInfo = 28,
    /// Compute pipeline create info
    ComputePipelineCreateInfo = 29,
    /// Command pool create info
    CommandPoolCreateInfo = 39,
    /// Command buffer allocate info
    CommandBufferAllocateInfo = 40,
    /// Command buffer begin info
    CommandBufferBeginInfo = 42,
    /// Render pass begin info
    RenderPassBeginInfo = 43,
    /// Physical device features 2
    PhysicalDeviceFeatures2 = 1000059000,
    /// Physical device Vulkan 1.1 features
    PhysicalDeviceVulkan11Features = 49,
    /// Physical device Vulkan 1.2 features
    PhysicalDeviceVulkan12Features = 51,
    /// Physical device Vulkan 1.3 features
    PhysicalDeviceVulkan13Features = 53,
    /// Rendering info (VK 1.3 dynamic rendering)
    RenderingInfo = 1000044000,
}
