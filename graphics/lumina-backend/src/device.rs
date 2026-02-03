//! GPU Device Abstraction
//!
//! Provides the core device abstraction for GPU operations including
//! device creation, capability queries, and resource management.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use bitflags::bitflags;
use lumina_core::Handle;

// ============================================================================
// Backend Type
// ============================================================================

/// Supported graphics backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendType {
    /// Vulkan backend (cross-platform).
    Vulkan,
    /// Metal backend (Apple platforms).
    Metal,
    /// DirectX 12 backend (Windows).
    Dx12,
    /// WebGPU backend (web and native).
    WebGpu,
    /// MAGMA backend (Helix native).
    Magma,
    /// Null backend (for testing).
    Null,
}

impl BackendType {
    /// Get backend name.
    pub fn name(&self) -> &'static str {
        match self {
            BackendType::Vulkan => "Vulkan",
            BackendType::Metal => "Metal",
            BackendType::Dx12 => "DirectX 12",
            BackendType::WebGpu => "WebGPU",
            BackendType::Magma => "MAGMA",
            BackendType::Null => "Null",
        }
    }

    /// Check if backend supports ray tracing.
    pub fn supports_ray_tracing(&self) -> bool {
        matches!(
            self,
            BackendType::Vulkan | BackendType::Dx12 | BackendType::Magma
        )
    }

    /// Check if backend supports mesh shaders.
    pub fn supports_mesh_shaders(&self) -> bool {
        matches!(
            self,
            BackendType::Vulkan | BackendType::Dx12 | BackendType::Magma
        )
    }
}

// ============================================================================
// Adapter Type
// ============================================================================

/// Type of GPU adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdapterType {
    /// Discrete GPU (dedicated graphics card).
    DiscreteGpu,
    /// Integrated GPU (part of CPU).
    IntegratedGpu,
    /// Virtual GPU (virtualized environment).
    VirtualGpu,
    /// CPU software renderer.
    Cpu,
    /// Unknown adapter type.
    Unknown,
}

impl AdapterType {
    /// Get preference score (higher = better for gaming/rendering).
    pub fn preference_score(&self) -> u32 {
        match self {
            AdapterType::DiscreteGpu => 100,
            AdapterType::IntegratedGpu => 50,
            AdapterType::VirtualGpu => 25,
            AdapterType::Cpu => 10,
            AdapterType::Unknown => 1,
        }
    }
}

// ============================================================================
// Vendor ID
// ============================================================================

/// Known GPU vendor IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VendorId {
    /// NVIDIA Corporation.
    Nvidia,
    /// AMD/ATI Technologies.
    Amd,
    /// Intel Corporation.
    Intel,
    /// Qualcomm.
    Qualcomm,
    /// ARM (Mali GPUs).
    Arm,
    /// Apple (M-series).
    Apple,
    /// Samsung.
    Samsung,
    /// Unknown vendor.
    Unknown(u32),
}

impl From<u32> for VendorId {
    fn from(id: u32) -> Self {
        match id {
            0x10DE => VendorId::Nvidia,
            0x1002 => VendorId::Amd,
            0x8086 => VendorId::Intel,
            0x5143 => VendorId::Qualcomm,
            0x13B5 => VendorId::Arm,
            0x106B => VendorId::Apple,
            0x144D => VendorId::Samsung,
            _ => VendorId::Unknown(id),
        }
    }
}

impl VendorId {
    /// Get vendor name.
    pub fn name(&self) -> &'static str {
        match self {
            VendorId::Nvidia => "NVIDIA",
            VendorId::Amd => "AMD",
            VendorId::Intel => "Intel",
            VendorId::Qualcomm => "Qualcomm",
            VendorId::Arm => "ARM",
            VendorId::Apple => "Apple",
            VendorId::Samsung => "Samsung",
            VendorId::Unknown(_) => "Unknown",
        }
    }

    /// Get raw vendor ID.
    pub fn raw(&self) -> u32 {
        match self {
            VendorId::Nvidia => 0x10DE,
            VendorId::Amd => 0x1002,
            VendorId::Intel => 0x8086,
            VendorId::Qualcomm => 0x5143,
            VendorId::Arm => 0x13B5,
            VendorId::Apple => 0x106B,
            VendorId::Samsung => 0x144D,
            VendorId::Unknown(id) => *id,
        }
    }
}

// ============================================================================
// Adapter Info
// ============================================================================

/// Information about a GPU adapter.
#[derive(Debug, Clone)]
pub struct AdapterInfo {
    /// Adapter name.
    pub name: String,
    /// Vendor.
    pub vendor: VendorId,
    /// Device ID.
    pub device_id: u32,
    /// Adapter type.
    pub adapter_type: AdapterType,
    /// Backend type.
    pub backend: BackendType,
    /// Driver version.
    pub driver_version: u32,
    /// API version supported.
    pub api_version: u32,
    /// Dedicated video memory (bytes).
    pub dedicated_video_memory: u64,
    /// Dedicated system memory (bytes).
    pub dedicated_system_memory: u64,
    /// Shared system memory (bytes).
    pub shared_system_memory: u64,
    /// LUID (locally unique identifier).
    pub luid: u64,
}

impl AdapterInfo {
    /// Create new adapter info.
    pub fn new(name: impl Into<String>, vendor: VendorId, device_id: u32) -> Self {
        Self {
            name: name.into(),
            vendor,
            device_id,
            adapter_type: AdapterType::Unknown,
            backend: BackendType::Null,
            driver_version: 0,
            api_version: 0,
            dedicated_video_memory: 0,
            dedicated_system_memory: 0,
            shared_system_memory: 0,
            luid: 0,
        }
    }

    /// Total available memory.
    pub fn total_memory(&self) -> u64 {
        self.dedicated_video_memory + self.dedicated_system_memory + self.shared_system_memory
    }

    /// Get driver version string.
    pub fn driver_version_string(&self) -> String {
        let major = (self.driver_version >> 22) & 0x3FF;
        let minor = (self.driver_version >> 12) & 0x3FF;
        let patch = self.driver_version & 0xFFF;
        alloc::format!("{}.{}.{}", major, minor, patch)
    }
}

// ============================================================================
// Device Features
// ============================================================================

bitflags! {
    /// Features that can be enabled on a device.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DeviceFeatures: u64 {
        // Core features
        /// Geometry shaders support.
        const GEOMETRY_SHADERS = 1 << 0;
        /// Tessellation shaders support.
        const TESSELLATION_SHADERS = 1 << 1;
        /// Multi-draw indirect support.
        const MULTI_DRAW_INDIRECT = 1 << 2;
        /// Draw indirect first instance.
        const DRAW_INDIRECT_FIRST_INSTANCE = 1 << 3;
        /// Depth clamp support.
        const DEPTH_CLAMP = 1 << 4;
        /// Depth bias clamp support.
        const DEPTH_BIAS_CLAMP = 1 << 5;
        /// Fill mode non-solid.
        const FILL_MODE_NON_SOLID = 1 << 6;
        /// Wide lines support.
        const WIDE_LINES = 1 << 7;
        /// Large points support.
        const LARGE_POINTS = 1 << 8;
        /// Alpha to one support.
        const ALPHA_TO_ONE = 1 << 9;
        /// Multi-viewport support.
        const MULTI_VIEWPORT = 1 << 10;
        /// Sampler anisotropy.
        const SAMPLER_ANISOTROPY = 1 << 11;
        /// Texture compression BC.
        const TEXTURE_COMPRESSION_BC = 1 << 12;
        /// Texture compression ETC2.
        const TEXTURE_COMPRESSION_ETC2 = 1 << 13;
        /// Texture compression ASTC.
        const TEXTURE_COMPRESSION_ASTC = 1 << 14;
        /// Occlusion query precise.
        const OCCLUSION_QUERY_PRECISE = 1 << 15;
        /// Pipeline statistics query.
        const PIPELINE_STATISTICS_QUERY = 1 << 16;
        /// Vertex pipeline stores and atomics.
        const VERTEX_STORES_AND_ATOMICS = 1 << 17;
        /// Fragment stores and atomics.
        const FRAGMENT_STORES_AND_ATOMICS = 1 << 18;

        // Advanced features
        /// Bindless resources.
        const BINDLESS = 1 << 20;
        /// Buffer device address.
        const BUFFER_DEVICE_ADDRESS = 1 << 21;
        /// Descriptor indexing.
        const DESCRIPTOR_INDEXING = 1 << 22;
        /// Timeline semaphores.
        const TIMELINE_SEMAPHORES = 1 << 23;
        /// Dynamic rendering (renderpass-less).
        const DYNAMIC_RENDERING = 1 << 24;
        /// Synchronization 2.
        const SYNCHRONIZATION_2 = 1 << 25;
        /// Maintenance 4.
        const MAINTENANCE_4 = 1 << 26;

        // Ray tracing features
        /// Ray tracing pipeline.
        const RAY_TRACING_PIPELINE = 1 << 30;
        /// Ray query (inline ray tracing).
        const RAY_QUERY = 1 << 31;
        /// Acceleration structure.
        const ACCELERATION_STRUCTURE = 1 << 32;
        /// Ray tracing motion blur.
        const RAY_TRACING_MOTION_BLUR = 1 << 33;
        /// Ray tracing position fetch.
        const RAY_TRACING_POSITION_FETCH = 1 << 34;

        // Mesh shader features
        /// Mesh shaders.
        const MESH_SHADERS = 1 << 40;
        /// Task shaders.
        const TASK_SHADERS = 1 << 41;
        /// Primitive shading rate.
        const PRIMITIVE_SHADING_RATE = 1 << 42;

        // Other advanced features
        /// Variable rate shading.
        const VARIABLE_RATE_SHADING = 1 << 45;
        /// Fragment shading rate.
        const FRAGMENT_SHADING_RATE = 1 << 46;
        /// Sampler feedback.
        const SAMPLER_FEEDBACK = 1 << 47;
        /// Shader subgroup.
        const SHADER_SUBGROUP = 1 << 48;
        /// Shader float16/int8.
        const SHADER_FLOAT16_INT8 = 1 << 49;
        /// 16-bit storage.
        const STORAGE_16BIT = 1 << 50;
        /// 8-bit storage.
        const STORAGE_8BIT = 1 << 51;
        /// Shader atomic int64.
        const SHADER_ATOMIC_INT64 = 1 << 52;
        /// Shader atomic float.
        const SHADER_ATOMIC_FLOAT = 1 << 53;
    }
}

impl Default for DeviceFeatures {
    fn default() -> Self {
        DeviceFeatures::empty()
    }
}

// ============================================================================
// Device Limits
// ============================================================================

/// Hardware limits of a device.
#[derive(Debug, Clone)]
pub struct DeviceLimits {
    // Image limits
    /// Maximum 1D texture dimension.
    pub max_texture_dimension_1d: u32,
    /// Maximum 2D texture dimension.
    pub max_texture_dimension_2d: u32,
    /// Maximum 3D texture dimension.
    pub max_texture_dimension_3d: u32,
    /// Maximum cube texture dimension.
    pub max_texture_dimension_cube: u32,
    /// Maximum texture array layers.
    pub max_texture_array_layers: u32,

    // Buffer limits
    /// Maximum uniform buffer range.
    pub max_uniform_buffer_range: u32,
    /// Maximum storage buffer range.
    pub max_storage_buffer_range: u32,
    /// Maximum push constants size.
    pub max_push_constants_size: u32,

    // Memory limits
    /// Maximum memory allocation count.
    pub max_memory_allocation_count: u32,
    /// Maximum sampler allocation count.
    pub max_sampler_allocation_count: u32,
    /// Buffer image granularity.
    pub buffer_image_granularity: u64,
    /// Non-coherent atom size.
    pub non_coherent_atom_size: u64,

    // Descriptor limits
    /// Maximum bound descriptor sets.
    pub max_bound_descriptor_sets: u32,
    /// Maximum per-stage descriptor samplers.
    pub max_per_stage_descriptor_samplers: u32,
    /// Maximum per-stage descriptor uniform buffers.
    pub max_per_stage_descriptor_uniform_buffers: u32,
    /// Maximum per-stage descriptor storage buffers.
    pub max_per_stage_descriptor_storage_buffers: u32,
    /// Maximum per-stage descriptor sampled images.
    pub max_per_stage_descriptor_sampled_images: u32,
    /// Maximum per-stage descriptor storage images.
    pub max_per_stage_descriptor_storage_images: u32,
    /// Maximum per-stage resources.
    pub max_per_stage_resources: u32,
    /// Maximum descriptor set samplers.
    pub max_descriptor_set_samplers: u32,
    /// Maximum descriptor set uniform buffers.
    pub max_descriptor_set_uniform_buffers: u32,
    /// Maximum descriptor set storage buffers.
    pub max_descriptor_set_storage_buffers: u32,
    /// Maximum descriptor set sampled images.
    pub max_descriptor_set_sampled_images: u32,
    /// Maximum descriptor set storage images.
    pub max_descriptor_set_storage_images: u32,

    // Vertex input limits
    /// Maximum vertex input attributes.
    pub max_vertex_input_attributes: u32,
    /// Maximum vertex input bindings.
    pub max_vertex_input_bindings: u32,
    /// Maximum vertex input attribute offset.
    pub max_vertex_input_attribute_offset: u32,
    /// Maximum vertex input binding stride.
    pub max_vertex_input_binding_stride: u32,
    /// Maximum vertex output components.
    pub max_vertex_output_components: u32,

    // Fragment limits
    /// Maximum fragment input components.
    pub max_fragment_input_components: u32,
    /// Maximum fragment output attachments.
    pub max_fragment_output_attachments: u32,
    /// Maximum fragment dual source attachments.
    pub max_fragment_dual_source_attachments: u32,
    /// Maximum fragment combined output resources.
    pub max_fragment_combined_output_resources: u32,

    // Compute limits
    /// Maximum compute shared memory size.
    pub max_compute_shared_memory_size: u32,
    /// Maximum compute work group count.
    pub max_compute_work_group_count: [u32; 3],
    /// Maximum compute work group invocations.
    pub max_compute_work_group_invocations: u32,
    /// Maximum compute work group size.
    pub max_compute_work_group_size: [u32; 3],

    // Subgroup
    /// Subgroup size.
    pub subgroup_size: u32,
    /// Supported subgroup stages.
    pub supported_subgroup_stages: u32,
    /// Supported subgroup operations.
    pub supported_subgroup_operations: u32,

    // Viewports
    /// Maximum viewports.
    pub max_viewports: u32,
    /// Maximum viewport dimensions.
    pub max_viewport_dimensions: [u32; 2],
    /// Viewport bounds range.
    pub viewport_bounds_range: [f32; 2],

    // Framebuffer
    /// Maximum framebuffer width.
    pub max_framebuffer_width: u32,
    /// Maximum framebuffer height.
    pub max_framebuffer_height: u32,
    /// Maximum framebuffer layers.
    pub max_framebuffer_layers: u32,
    /// Maximum color attachments.
    pub max_color_attachments: u32,

    // Sample counts
    /// Framebuffer color sample counts.
    pub framebuffer_color_sample_counts: u32,
    /// Framebuffer depth sample counts.
    pub framebuffer_depth_sample_counts: u32,
    /// Framebuffer stencil sample counts.
    pub framebuffer_stencil_sample_counts: u32,

    // Point/line
    /// Point size range.
    pub point_size_range: [f32; 2],
    /// Line width range.
    pub line_width_range: [f32; 2],

    // Precision
    /// Min/max texel offset.
    pub min_texel_offset: i32,
    /// Max texel offset.
    pub max_texel_offset: u32,

    // Ray tracing
    /// Maximum ray recursion depth.
    pub max_ray_recursion_depth: u32,
    /// Maximum ray dispatch invocation count.
    pub max_ray_dispatch_invocation_count: u32,
    /// Shader group handle size.
    pub shader_group_handle_size: u32,
    /// Shader group base alignment.
    pub shader_group_base_alignment: u32,
    /// Maximum geometry count.
    pub max_geometry_count: u64,
    /// Maximum instance count.
    pub max_instance_count: u64,
    /// Maximum primitive count.
    pub max_primitive_count: u64,

    // Mesh shaders
    /// Maximum task work group invocations.
    pub max_task_work_group_invocations: u32,
    /// Maximum task work group size.
    pub max_task_work_group_size: [u32; 3],
    /// Maximum task total memory size.
    pub max_task_total_memory_size: u32,
    /// Maximum task output count.
    pub max_task_output_count: u32,
    /// Maximum mesh work group invocations.
    pub max_mesh_work_group_invocations: u32,
    /// Maximum mesh work group size.
    pub max_mesh_work_group_size: [u32; 3],
    /// Maximum mesh total memory size.
    pub max_mesh_total_memory_size: u32,
    /// Maximum mesh output vertices.
    pub max_mesh_output_vertices: u32,
    /// Maximum mesh output primitives.
    pub max_mesh_output_primitives: u32,
}

impl Default for DeviceLimits {
    fn default() -> Self {
        Self {
            max_texture_dimension_1d: 16384,
            max_texture_dimension_2d: 16384,
            max_texture_dimension_3d: 2048,
            max_texture_dimension_cube: 16384,
            max_texture_array_layers: 2048,
            max_uniform_buffer_range: 65536,
            max_storage_buffer_range: 134217728,
            max_push_constants_size: 256,
            max_memory_allocation_count: 4096,
            max_sampler_allocation_count: 4000,
            buffer_image_granularity: 1,
            non_coherent_atom_size: 256,
            max_bound_descriptor_sets: 8,
            max_per_stage_descriptor_samplers: 16,
            max_per_stage_descriptor_uniform_buffers: 15,
            max_per_stage_descriptor_storage_buffers: 16,
            max_per_stage_descriptor_sampled_images: 128,
            max_per_stage_descriptor_storage_images: 8,
            max_per_stage_resources: 200,
            max_descriptor_set_samplers: 80,
            max_descriptor_set_uniform_buffers: 90,
            max_descriptor_set_storage_buffers: 96,
            max_descriptor_set_sampled_images: 1048576,
            max_descriptor_set_storage_images: 1048576,
            max_vertex_input_attributes: 32,
            max_vertex_input_bindings: 32,
            max_vertex_input_attribute_offset: 2047,
            max_vertex_input_binding_stride: 2048,
            max_vertex_output_components: 128,
            max_fragment_input_components: 128,
            max_fragment_output_attachments: 8,
            max_fragment_dual_source_attachments: 1,
            max_fragment_combined_output_resources: 16,
            max_compute_shared_memory_size: 49152,
            max_compute_work_group_count: [65535, 65535, 65535],
            max_compute_work_group_invocations: 1024,
            max_compute_work_group_size: [1024, 1024, 64],
            subgroup_size: 32,
            supported_subgroup_stages: 0xFF,
            supported_subgroup_operations: 0xFF,
            max_viewports: 16,
            max_viewport_dimensions: [16384, 16384],
            viewport_bounds_range: [-32768.0, 32767.0],
            max_framebuffer_width: 16384,
            max_framebuffer_height: 16384,
            max_framebuffer_layers: 2048,
            max_color_attachments: 8,
            framebuffer_color_sample_counts: 0x1F,
            framebuffer_depth_sample_counts: 0x1F,
            framebuffer_stencil_sample_counts: 0x1F,
            point_size_range: [1.0, 2048.0],
            line_width_range: [1.0, 64.0],
            min_texel_offset: -8,
            max_texel_offset: 7,
            max_ray_recursion_depth: 31,
            max_ray_dispatch_invocation_count: 1073741824,
            shader_group_handle_size: 32,
            shader_group_base_alignment: 64,
            max_geometry_count: 16777215,
            max_instance_count: 16777215,
            max_primitive_count: 536870911,
            max_task_work_group_invocations: 128,
            max_task_work_group_size: [128, 1, 1],
            max_task_total_memory_size: 32768,
            max_task_output_count: 65535,
            max_mesh_work_group_invocations: 128,
            max_mesh_work_group_size: [128, 1, 1],
            max_mesh_total_memory_size: 32768,
            max_mesh_output_vertices: 256,
            max_mesh_output_primitives: 256,
        }
    }
}

// ============================================================================
// Device Capabilities
// ============================================================================

/// Full capabilities of a device.
#[derive(Debug, Clone)]
pub struct DeviceCapabilities {
    /// Supported features.
    pub features: DeviceFeatures,
    /// Hardware limits.
    pub limits: DeviceLimits,
    /// Supported texture formats.
    pub supported_formats: Vec<TextureFormat>,
    /// Supported depth formats.
    pub supported_depth_formats: Vec<TextureFormat>,
    /// Supported sample counts.
    pub supported_sample_counts: Vec<u32>,
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            features: DeviceFeatures::default(),
            limits: DeviceLimits::default(),
            supported_formats: Vec::new(),
            supported_depth_formats: Vec::new(),
            supported_sample_counts: alloc::vec![1, 2, 4, 8],
        }
    }
}

// ============================================================================
// Texture Format
// ============================================================================

/// Texture format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum TextureFormat {
    // 8-bit
    R8Unorm              = 1,
    R8Snorm              = 2,
    R8Uint               = 3,
    R8Sint               = 4,
    // 16-bit
    R16Uint              = 10,
    R16Sint              = 11,
    R16Float             = 12,
    Rg8Unorm             = 13,
    Rg8Snorm             = 14,
    Rg8Uint              = 15,
    Rg8Sint              = 16,
    // 32-bit
    R32Uint              = 20,
    R32Sint              = 21,
    R32Float             = 22,
    Rg16Uint             = 23,
    Rg16Sint             = 24,
    Rg16Float            = 25,
    Rgba8Unorm           = 26,
    Rgba8UnormSrgb       = 27,
    Rgba8Snorm           = 28,
    Rgba8Uint            = 29,
    Rgba8Sint            = 30,
    Bgra8Unorm           = 31,
    Bgra8UnormSrgb       = 32,
    // 64-bit
    Rg32Uint             = 40,
    Rg32Sint             = 41,
    Rg32Float            = 42,
    Rgba16Uint           = 43,
    Rgba16Sint           = 44,
    Rgba16Float          = 45,
    // 128-bit
    Rgba32Uint           = 50,
    Rgba32Sint           = 51,
    Rgba32Float          = 52,
    // Depth/stencil
    Depth16Unorm         = 60,
    Depth24Plus          = 61,
    Depth24PlusStencil8  = 62,
    Depth32Float         = 63,
    Depth32FloatStencil8 = 64,
    // Compressed
    Bc1RgbaUnorm         = 70,
    Bc1RgbaUnormSrgb     = 71,
    Bc2RgbaUnorm         = 72,
    Bc2RgbaUnormSrgb     = 73,
    Bc3RgbaUnorm         = 74,
    Bc3RgbaUnormSrgb     = 75,
    Bc4RUnorm            = 76,
    Bc4RSnorm            = 77,
    Bc5RgUnorm           = 78,
    Bc5RgSnorm           = 79,
    Bc6hRgbUfloat        = 80,
    Bc6hRgbSfloat        = 81,
    Bc7RgbaUnorm         = 82,
    Bc7RgbaUnormSrgb     = 83,
}

impl TextureFormat {
    /// Get bytes per pixel (0 for compressed).
    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::R8Unorm | Self::R8Snorm | Self::R8Uint | Self::R8Sint => 1,
            Self::R16Uint
            | Self::R16Sint
            | Self::R16Float
            | Self::Rg8Unorm
            | Self::Rg8Snorm
            | Self::Rg8Uint
            | Self::Rg8Sint
            | Self::Depth16Unorm => 2,
            Self::R32Uint
            | Self::R32Sint
            | Self::R32Float
            | Self::Rg16Uint
            | Self::Rg16Sint
            | Self::Rg16Float
            | Self::Rgba8Unorm
            | Self::Rgba8UnormSrgb
            | Self::Rgba8Snorm
            | Self::Rgba8Uint
            | Self::Rgba8Sint
            | Self::Bgra8Unorm
            | Self::Bgra8UnormSrgb
            | Self::Depth24Plus
            | Self::Depth24PlusStencil8
            | Self::Depth32Float => 4,
            Self::Rg32Uint
            | Self::Rg32Sint
            | Self::Rg32Float
            | Self::Rgba16Uint
            | Self::Rgba16Sint
            | Self::Rgba16Float
            | Self::Depth32FloatStencil8 => 8,
            Self::Rgba32Uint | Self::Rgba32Sint | Self::Rgba32Float => 16,
            _ => 0, // Compressed formats
        }
    }

    /// Check if format is depth.
    pub fn is_depth(&self) -> bool {
        matches!(
            self,
            Self::Depth16Unorm
                | Self::Depth24Plus
                | Self::Depth24PlusStencil8
                | Self::Depth32Float
                | Self::Depth32FloatStencil8
        )
    }

    /// Check if format is compressed.
    pub fn is_compressed(&self) -> bool {
        matches!(
            self,
            Self::Bc1RgbaUnorm
                | Self::Bc1RgbaUnormSrgb
                | Self::Bc2RgbaUnorm
                | Self::Bc2RgbaUnormSrgb
                | Self::Bc3RgbaUnorm
                | Self::Bc3RgbaUnormSrgb
                | Self::Bc4RUnorm
                | Self::Bc4RSnorm
                | Self::Bc5RgUnorm
                | Self::Bc5RgSnorm
                | Self::Bc6hRgbUfloat
                | Self::Bc6hRgbSfloat
                | Self::Bc7RgbaUnorm
                | Self::Bc7RgbaUnormSrgb
        )
    }

    /// Check if format is sRGB.
    pub fn is_srgb(&self) -> bool {
        matches!(
            self,
            Self::Rgba8UnormSrgb
                | Self::Bgra8UnormSrgb
                | Self::Bc1RgbaUnormSrgb
                | Self::Bc2RgbaUnormSrgb
                | Self::Bc3RgbaUnormSrgb
                | Self::Bc7RgbaUnormSrgb
        )
    }
}

// ============================================================================
// Adapter
// ============================================================================

/// A GPU adapter (physical device).
pub struct Adapter {
    /// Adapter information.
    pub info: AdapterInfo,
    /// Capabilities.
    pub capabilities: DeviceCapabilities,
    /// Queue families.
    pub queue_families: Vec<QueueFamilyInfo>,
    /// Memory properties.
    pub memory_properties: MemoryProperties,
}

/// Queue family information.
#[derive(Debug, Clone)]
pub struct QueueFamilyInfo {
    /// Index.
    pub index: u32,
    /// Queue count.
    pub queue_count: u32,
    /// Supported operations.
    pub capabilities: QueueCapabilities,
    /// Timestamp valid bits.
    pub timestamp_valid_bits: u32,
    /// Minimum image transfer granularity.
    pub min_image_transfer_granularity: [u32; 3],
}

bitflags! {
    /// Queue capabilities.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct QueueCapabilities: u32 {
        /// Graphics operations.
        const GRAPHICS = 1 << 0;
        /// Compute operations.
        const COMPUTE = 1 << 1;
        /// Transfer operations.
        const TRANSFER = 1 << 2;
        /// Sparse binding.
        const SPARSE_BINDING = 1 << 3;
        /// Video decode.
        const VIDEO_DECODE = 1 << 4;
        /// Video encode.
        const VIDEO_ENCODE = 1 << 5;
        /// Presentation.
        const PRESENT = 1 << 6;
    }
}

/// Memory properties.
#[derive(Debug, Clone)]
pub struct MemoryProperties {
    /// Memory types.
    pub memory_types: Vec<MemoryType>,
    /// Memory heaps.
    pub memory_heaps: Vec<MemoryHeap>,
}

/// Memory type.
#[derive(Debug, Clone, Copy)]
pub struct MemoryType {
    /// Heap index.
    pub heap_index: u32,
    /// Memory properties.
    pub properties: MemoryPropertyFlags,
}

bitflags! {
    /// Memory property flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MemoryPropertyFlags: u32 {
        /// Device local memory.
        const DEVICE_LOCAL = 1 << 0;
        /// Host visible memory.
        const HOST_VISIBLE = 1 << 1;
        /// Host coherent memory.
        const HOST_COHERENT = 1 << 2;
        /// Host cached memory.
        const HOST_CACHED = 1 << 3;
        /// Lazily allocated memory.
        const LAZILY_ALLOCATED = 1 << 4;
        /// Protected memory.
        const PROTECTED = 1 << 5;
    }
}

/// Memory heap.
#[derive(Debug, Clone, Copy)]
pub struct MemoryHeap {
    /// Size in bytes.
    pub size: u64,
    /// Heap flags.
    pub flags: MemoryHeapFlags,
}

bitflags! {
    /// Memory heap flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MemoryHeapFlags: u32 {
        /// Device local heap.
        const DEVICE_LOCAL = 1 << 0;
        /// Multi-instance heap.
        const MULTI_INSTANCE = 1 << 1;
    }
}

// ============================================================================
// Device Description
// ============================================================================

/// Description for device creation.
#[derive(Debug, Clone)]
pub struct DeviceDesc {
    /// Required features.
    pub features: DeviceFeatures,
    /// Queue requests.
    pub queues: Vec<QueueRequest>,
    /// Enable validation.
    pub validation: bool,
    /// Enable profiling.
    pub profiling: bool,
}

impl Default for DeviceDesc {
    fn default() -> Self {
        Self {
            features: DeviceFeatures::empty(),
            queues: alloc::vec![QueueRequest {
                family: QueueType::Graphics,
                count: 1,
                priorities: alloc::vec![1.0],
            }],
            validation: false,
            profiling: false,
        }
    }
}

/// Request for queue creation.
#[derive(Debug, Clone)]
pub struct QueueRequest {
    /// Queue type.
    pub family: QueueType,
    /// Number of queues.
    pub count: u32,
    /// Queue priorities.
    pub priorities: Vec<f32>,
}

/// Queue type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueueType {
    /// Graphics + Compute + Transfer.
    Graphics,
    /// Compute + Transfer.
    Compute,
    /// Transfer only.
    Transfer,
    /// Video decode.
    VideoDecode,
    /// Video encode.
    VideoEncode,
}

// ============================================================================
// Device
// ============================================================================

/// Handle to a device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceHandle(Handle<Device>);

/// GPU logical device.
pub struct Device {
    /// Handle.
    pub handle: DeviceHandle,
    /// Adapter info.
    pub adapter_info: AdapterInfo,
    /// Enabled features.
    pub features: DeviceFeatures,
    /// Limits.
    pub limits: DeviceLimits,
    /// Current frame.
    frame_index: AtomicU64,
}

impl Device {
    /// Create a new device.
    pub fn new(adapter: &Adapter, _desc: &DeviceDesc) -> Self {
        Self {
            handle: DeviceHandle(Handle::from_raw_parts(0, 0)),
            adapter_info: adapter.info.clone(),
            features: adapter.capabilities.features,
            limits: adapter.capabilities.limits.clone(),
            frame_index: AtomicU64::new(0),
        }
    }

    /// Get current frame index.
    pub fn frame_index(&self) -> u64 {
        self.frame_index.load(Ordering::Relaxed)
    }

    /// Increment frame.
    pub fn next_frame(&self) -> u64 {
        self.frame_index.fetch_add(1, Ordering::Relaxed)
    }

    /// Check if feature is supported.
    pub fn supports_feature(&self, feature: DeviceFeatures) -> bool {
        self.features.contains(feature)
    }

    /// Wait for device to be idle.
    pub fn wait_idle(&self) {
        // Backend-specific implementation
    }
}
