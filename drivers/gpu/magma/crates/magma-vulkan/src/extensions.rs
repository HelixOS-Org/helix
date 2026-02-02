//! # Vulkan Extensions
//!
//! Extension enumeration and feature detection for Vulkan 1.3.

use core::ffi::CStr;

// =============================================================================
// EXTENSION NAMES
// =============================================================================

/// Instance extensions supported by MAGMA
pub mod instance_extensions {
    /// Surface extension
    pub const VK_KHR_SURFACE: &[u8] = b"VK_KHR_surface\0";
    /// XCB surface extension
    pub const VK_KHR_XCB_SURFACE: &[u8] = b"VK_KHR_xcb_surface\0";
    /// Xlib surface extension
    pub const VK_KHR_XLIB_SURFACE: &[u8] = b"VK_KHR_xlib_surface\0";
    /// Wayland surface extension
    pub const VK_KHR_WAYLAND_SURFACE: &[u8] = b"VK_KHR_wayland_surface\0";
    /// Get physical device properties 2
    pub const VK_KHR_GET_PHYSICAL_DEVICE_PROPERTIES_2: &[u8] =
        b"VK_KHR_get_physical_device_properties2\0";
    /// External memory capabilities
    pub const VK_KHR_EXTERNAL_MEMORY_CAPABILITIES: &[u8] =
        b"VK_KHR_external_memory_capabilities\0";
    /// External semaphore capabilities
    pub const VK_KHR_EXTERNAL_SEMAPHORE_CAPABILITIES: &[u8] =
        b"VK_KHR_external_semaphore_capabilities\0";
    /// External fence capabilities
    pub const VK_KHR_EXTERNAL_FENCE_CAPABILITIES: &[u8] = b"VK_KHR_external_fence_capabilities\0";
    /// Debug utils
    pub const VK_EXT_DEBUG_UTILS: &[u8] = b"VK_EXT_debug_utils\0";
}

/// Device extensions supported by MAGMA
pub mod device_extensions {
    /// Swapchain extension
    pub const VK_KHR_SWAPCHAIN: &[u8] = b"VK_KHR_swapchain\0";
    /// Dynamic rendering
    pub const VK_KHR_DYNAMIC_RENDERING: &[u8] = b"VK_KHR_dynamic_rendering\0";
    /// Synchronization 2
    pub const VK_KHR_SYNCHRONIZATION_2: &[u8] = b"VK_KHR_synchronization2\0";
    /// Timeline semaphore
    pub const VK_KHR_TIMELINE_SEMAPHORE: &[u8] = b"VK_KHR_timeline_semaphore\0";
    /// Maintenance 4
    pub const VK_KHR_MAINTENANCE_4: &[u8] = b"VK_KHR_maintenance4\0";
    /// Buffer device address
    pub const VK_KHR_BUFFER_DEVICE_ADDRESS: &[u8] = b"VK_KHR_buffer_device_address\0";
    /// Descriptor indexing
    pub const VK_EXT_DESCRIPTOR_INDEXING: &[u8] = b"VK_EXT_descriptor_indexing\0";
    /// External memory
    pub const VK_KHR_EXTERNAL_MEMORY: &[u8] = b"VK_KHR_external_memory\0";
    /// External memory FD
    pub const VK_KHR_EXTERNAL_MEMORY_FD: &[u8] = b"VK_KHR_external_memory_fd\0";
    /// External semaphore
    pub const VK_KHR_EXTERNAL_SEMAPHORE: &[u8] = b"VK_KHR_external_semaphore\0";
    /// External semaphore FD
    pub const VK_KHR_EXTERNAL_SEMAPHORE_FD: &[u8] = b"VK_KHR_external_semaphore_fd\0";
    /// External fence
    pub const VK_KHR_EXTERNAL_FENCE: &[u8] = b"VK_KHR_external_fence\0";
    /// External fence FD
    pub const VK_KHR_EXTERNAL_FENCE_FD: &[u8] = b"VK_KHR_external_fence_fd\0";
    /// Push descriptor
    pub const VK_KHR_PUSH_DESCRIPTOR: &[u8] = b"VK_KHR_push_descriptor\0";
    /// Shader draw parameters
    pub const VK_KHR_SHADER_DRAW_PARAMETERS: &[u8] = b"VK_KHR_shader_draw_parameters\0";
    /// 16-bit storage
    pub const VK_KHR_16BIT_STORAGE: &[u8] = b"VK_KHR_16bit_storage\0";
    /// 8-bit storage
    pub const VK_KHR_8BIT_STORAGE: &[u8] = b"VK_KHR_8bit_storage\0";
    /// Shader float16 int8
    pub const VK_KHR_SHADER_FLOAT16_INT8: &[u8] = b"VK_KHR_shader_float16_int8\0";
    /// Ray tracing pipeline
    pub const VK_KHR_RAY_TRACING_PIPELINE: &[u8] = b"VK_KHR_ray_tracing_pipeline\0";
    /// Ray query
    pub const VK_KHR_RAY_QUERY: &[u8] = b"VK_KHR_ray_query\0";
    /// Acceleration structure
    pub const VK_KHR_ACCELERATION_STRUCTURE: &[u8] = b"VK_KHR_acceleration_structure\0";
    /// Mesh shader
    pub const VK_EXT_MESH_SHADER: &[u8] = b"VK_EXT_mesh_shader\0";
}

// =============================================================================
// EXTENSION PROPERTIES
// =============================================================================

/// Maximum extension name length
pub const VK_MAX_EXTENSION_NAME_SIZE: usize = 256;

/// Extension properties
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VkExtensionProperties {
    /// Extension name
    pub extension_name: [u8; VK_MAX_EXTENSION_NAME_SIZE],
    /// Specification version
    pub spec_version: u32,
}

impl VkExtensionProperties {
    /// Create new extension properties
    pub fn new(name: &[u8], spec_version: u32) -> Self {
        let mut extension_name = [0u8; VK_MAX_EXTENSION_NAME_SIZE];
        let len = name.len().min(VK_MAX_EXTENSION_NAME_SIZE);
        extension_name[..len].copy_from_slice(&name[..len]);
        Self {
            extension_name,
            spec_version,
        }
    }

    /// Get extension name as CStr
    pub fn name(&self) -> Option<&CStr> {
        CStr::from_bytes_until_nul(&self.extension_name).ok()
    }
}

impl Default for VkExtensionProperties {
    fn default() -> Self {
        Self {
            extension_name: [0; VK_MAX_EXTENSION_NAME_SIZE],
            spec_version: 0,
        }
    }
}

// =============================================================================
// LAYER PROPERTIES
// =============================================================================

/// Maximum description length
pub const VK_MAX_DESCRIPTION_SIZE: usize = 256;

/// Layer properties
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VkLayerProperties {
    /// Layer name
    pub layer_name: [u8; VK_MAX_EXTENSION_NAME_SIZE],
    /// Spec version
    pub spec_version: u32,
    /// Implementation version
    pub implementation_version: u32,
    /// Description
    pub description: [u8; VK_MAX_DESCRIPTION_SIZE],
}

impl Default for VkLayerProperties {
    fn default() -> Self {
        Self {
            layer_name: [0; VK_MAX_EXTENSION_NAME_SIZE],
            spec_version: 0,
            implementation_version: 0,
            description: [0; VK_MAX_DESCRIPTION_SIZE],
        }
    }
}

// =============================================================================
// FEATURE FLAGS
// =============================================================================

bitflags::bitflags! {
    /// Instance extension flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct InstanceExtensions: u32 {
        /// VK_KHR_surface
        const SURFACE = 1 << 0;
        /// VK_KHR_xcb_surface
        const XCB_SURFACE = 1 << 1;
        /// VK_KHR_xlib_surface
        const XLIB_SURFACE = 1 << 2;
        /// VK_KHR_wayland_surface
        const WAYLAND_SURFACE = 1 << 3;
        /// VK_KHR_get_physical_device_properties2
        const GET_PHYSICAL_DEVICE_PROPERTIES_2 = 1 << 4;
        /// VK_KHR_external_memory_capabilities
        const EXTERNAL_MEMORY_CAPABILITIES = 1 << 5;
        /// VK_KHR_external_semaphore_capabilities
        const EXTERNAL_SEMAPHORE_CAPABILITIES = 1 << 6;
        /// VK_KHR_external_fence_capabilities
        const EXTERNAL_FENCE_CAPABILITIES = 1 << 7;
        /// VK_EXT_debug_utils
        const DEBUG_UTILS = 1 << 8;
    }

    /// Device extension flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct DeviceExtensions: u64 {
        /// VK_KHR_swapchain
        const SWAPCHAIN = 1 << 0;
        /// VK_KHR_dynamic_rendering
        const DYNAMIC_RENDERING = 1 << 1;
        /// VK_KHR_synchronization2
        const SYNCHRONIZATION_2 = 1 << 2;
        /// VK_KHR_timeline_semaphore
        const TIMELINE_SEMAPHORE = 1 << 3;
        /// VK_KHR_maintenance4
        const MAINTENANCE_4 = 1 << 4;
        /// VK_KHR_buffer_device_address
        const BUFFER_DEVICE_ADDRESS = 1 << 5;
        /// VK_EXT_descriptor_indexing
        const DESCRIPTOR_INDEXING = 1 << 6;
        /// VK_KHR_external_memory
        const EXTERNAL_MEMORY = 1 << 7;
        /// VK_KHR_external_memory_fd
        const EXTERNAL_MEMORY_FD = 1 << 8;
        /// VK_KHR_external_semaphore
        const EXTERNAL_SEMAPHORE = 1 << 9;
        /// VK_KHR_external_semaphore_fd
        const EXTERNAL_SEMAPHORE_FD = 1 << 10;
        /// VK_KHR_external_fence
        const EXTERNAL_FENCE = 1 << 11;
        /// VK_KHR_external_fence_fd
        const EXTERNAL_FENCE_FD = 1 << 12;
        /// VK_KHR_push_descriptor
        const PUSH_DESCRIPTOR = 1 << 13;
        /// VK_KHR_shader_draw_parameters
        const SHADER_DRAW_PARAMETERS = 1 << 14;
        /// VK_KHR_16bit_storage
        const STORAGE_16BIT = 1 << 15;
        /// VK_KHR_8bit_storage
        const STORAGE_8BIT = 1 << 16;
        /// VK_KHR_shader_float16_int8
        const SHADER_FLOAT16_INT8 = 1 << 17;
        /// VK_KHR_ray_tracing_pipeline
        const RAY_TRACING_PIPELINE = 1 << 18;
        /// VK_KHR_ray_query
        const RAY_QUERY = 1 << 19;
        /// VK_KHR_acceleration_structure
        const ACCELERATION_STRUCTURE = 1 << 20;
        /// VK_EXT_mesh_shader
        const MESH_SHADER = 1 << 21;

        /// Vulkan 1.3 core features (promoted)
        const VK_1_3_CORE = Self::DYNAMIC_RENDERING.bits()
            | Self::SYNCHRONIZATION_2.bits()
            | Self::MAINTENANCE_4.bits();
    }
}

// =============================================================================
// PHYSICAL DEVICE FEATURES
// =============================================================================

/// Vulkan 1.0 physical device features
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VkPhysicalDeviceFeatures {
    /// Robust buffer access
    pub robust_buffer_access: u32,
    /// Full draw index uint32
    pub full_draw_index_uint32: u32,
    /// Image cube array
    pub image_cube_array: u32,
    /// Independent blend
    pub independent_blend: u32,
    /// Geometry shader
    pub geometry_shader: u32,
    /// Tessellation shader
    pub tessellation_shader: u32,
    /// Sample rate shading
    pub sample_rate_shading: u32,
    /// Dual source blend
    pub dual_src_blend: u32,
    /// Logic op
    pub logic_op: u32,
    /// Multi draw indirect
    pub multi_draw_indirect: u32,
    /// Draw indirect first instance
    pub draw_indirect_first_instance: u32,
    /// Depth clamp
    pub depth_clamp: u32,
    /// Depth bias clamp
    pub depth_bias_clamp: u32,
    /// Fill mode non solid
    pub fill_mode_non_solid: u32,
    /// Depth bounds
    pub depth_bounds: u32,
    /// Wide lines
    pub wide_lines: u32,
    /// Large points
    pub large_points: u32,
    /// Alpha to one
    pub alpha_to_one: u32,
    /// Multi viewport
    pub multi_viewport: u32,
    /// Sampler anisotropy
    pub sampler_anisotropy: u32,
    /// Texture compression ETC2
    pub texture_compression_etc2: u32,
    /// Texture compression ASTC LDR
    pub texture_compression_astc_ldr: u32,
    /// Texture compression BC
    pub texture_compression_bc: u32,
    /// Occlusion query precise
    pub occlusion_query_precise: u32,
    /// Pipeline statistics query
    pub pipeline_statistics_query: u32,
    /// Vertex pipeline stores and atomics
    pub vertex_pipeline_stores_and_atomics: u32,
    /// Fragment stores and atomics
    pub fragment_stores_and_atomics: u32,
    /// Shader tessellation and geometry point size
    pub shader_tessellation_and_geometry_point_size: u32,
    /// Shader image gather extended
    pub shader_image_gather_extended: u32,
    /// Shader storage image extended formats
    pub shader_storage_image_extended_formats: u32,
    /// Shader storage image multisample
    pub shader_storage_image_multisample: u32,
    /// Shader storage image read without format
    pub shader_storage_image_read_without_format: u32,
    /// Shader storage image write without format
    pub shader_storage_image_write_without_format: u32,
    /// Shader uniform buffer array dynamic indexing
    pub shader_uniform_buffer_array_dynamic_indexing: u32,
    /// Shader sampled image array dynamic indexing
    pub shader_sampled_image_array_dynamic_indexing: u32,
    /// Shader storage buffer array dynamic indexing
    pub shader_storage_buffer_array_dynamic_indexing: u32,
    /// Shader storage image array dynamic indexing
    pub shader_storage_image_array_dynamic_indexing: u32,
    /// Shader clip distance
    pub shader_clip_distance: u32,
    /// Shader cull distance
    pub shader_cull_distance: u32,
    /// Shader float64
    pub shader_float64: u32,
    /// Shader int64
    pub shader_int64: u32,
    /// Shader int16
    pub shader_int16: u32,
    /// Shader resource residency
    pub shader_resource_residency: u32,
    /// Shader resource min lod
    pub shader_resource_min_lod: u32,
    /// Sparse binding
    pub sparse_binding: u32,
    /// Sparse residency buffer
    pub sparse_residency_buffer: u32,
    /// Sparse residency image 2D
    pub sparse_residency_image2_d: u32,
    /// Sparse residency image 3D
    pub sparse_residency_image3_d: u32,
    /// Sparse residency 2 samples
    pub sparse_residency2_samples: u32,
    /// Sparse residency 4 samples
    pub sparse_residency4_samples: u32,
    /// Sparse residency 8 samples
    pub sparse_residency8_samples: u32,
    /// Sparse residency 16 samples
    pub sparse_residency16_samples: u32,
    /// Sparse residency aliased
    pub sparse_residency_aliased: u32,
    /// Variable multisample rate
    pub variable_multisample_rate: u32,
    /// Inherited queries
    pub inherited_queries: u32,
}

/// Vulkan 1.3 features
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VkPhysicalDeviceVulkan13Features {
    /// Structure type
    pub s_type: u32,
    /// Next structure
    pub p_next: *mut core::ffi::c_void,
    /// Robust image access
    pub robust_image_access: u32,
    /// Inline uniform block
    pub inline_uniform_block: u32,
    /// Descriptor binding inline uniform block update after bind
    pub descriptor_binding_inline_uniform_block_update_after_bind: u32,
    /// Pipeline creation cache control
    pub pipeline_creation_cache_control: u32,
    /// Private data
    pub private_data: u32,
    /// Shader demote to helper invocation
    pub shader_demote_to_helper_invocation: u32,
    /// Shader terminate invocation
    pub shader_terminate_invocation: u32,
    /// Subgroup size control
    pub subgroup_size_control: u32,
    /// Compute full subgroups
    pub compute_full_subgroups: u32,
    /// Synchronization 2
    pub synchronization2: u32,
    /// Texture compression ASTC HDR
    pub texture_compression_astc_hdr: u32,
    /// Shader zero initialize workgroup memory
    pub shader_zero_initialize_workgroup_memory: u32,
    /// Dynamic rendering
    pub dynamic_rendering: u32,
    /// Shader integer dot product
    pub shader_integer_dot_product: u32,
    /// Maintenance 4
    pub maintenance4: u32,
}

// =============================================================================
// SUPPORTED EXTENSIONS LIST
// =============================================================================

/// Get list of supported instance extensions
pub fn get_instance_extensions() -> &'static [VkExtensionProperties] {
    static EXTENSIONS: &[VkExtensionProperties] = &[
        VkExtensionProperties {
            extension_name: const_extension_name(b"VK_KHR_surface"),
            spec_version: 25,
        },
        VkExtensionProperties {
            extension_name: const_extension_name(b"VK_KHR_get_physical_device_properties2"),
            spec_version: 2,
        },
        VkExtensionProperties {
            extension_name: const_extension_name(b"VK_EXT_debug_utils"),
            spec_version: 2,
        },
    ];
    EXTENSIONS
}

/// Get list of supported device extensions
pub fn get_device_extensions() -> &'static [VkExtensionProperties] {
    static EXTENSIONS: &[VkExtensionProperties] = &[
        VkExtensionProperties {
            extension_name: const_extension_name(b"VK_KHR_swapchain"),
            spec_version: 70,
        },
        VkExtensionProperties {
            extension_name: const_extension_name(b"VK_KHR_dynamic_rendering"),
            spec_version: 1,
        },
        VkExtensionProperties {
            extension_name: const_extension_name(b"VK_KHR_synchronization2"),
            spec_version: 1,
        },
        VkExtensionProperties {
            extension_name: const_extension_name(b"VK_KHR_timeline_semaphore"),
            spec_version: 2,
        },
        VkExtensionProperties {
            extension_name: const_extension_name(b"VK_KHR_buffer_device_address"),
            spec_version: 1,
        },
    ];
    EXTENSIONS
}

/// Helper to create const extension name
const fn const_extension_name(name: &[u8]) -> [u8; VK_MAX_EXTENSION_NAME_SIZE] {
    let mut result = [0u8; VK_MAX_EXTENSION_NAME_SIZE];
    let mut i = 0;
    while i < name.len() && i < VK_MAX_EXTENSION_NAME_SIZE {
        result[i] = name[i];
        i += 1;
    }
    result
}
