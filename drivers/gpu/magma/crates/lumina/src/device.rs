//! Device and physical device types
//!
//! This module provides types for GPU device management.

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;

/// Physical device handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PhysicalDeviceHandle(pub u64);

impl PhysicalDeviceHandle {
    /// Null/invalid handle
    pub const NULL: Self = Self(0);

    /// Creates from raw value
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns raw value
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Checks if valid
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

/// Device handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DeviceHandle(pub u64);

impl DeviceHandle {
    /// Null/invalid handle
    pub const NULL: Self = Self(0);

    /// Creates from raw value
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns raw value
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Checks if valid
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

/// Queue handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct QueueHandle(pub u64);

impl QueueHandle {
    /// Null/invalid handle
    pub const NULL: Self = Self(0);

    /// Creates from raw value
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns raw value
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Checks if valid
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

/// Instance handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct InstanceHandle(pub u64);

impl InstanceHandle {
    /// Null/invalid handle
    pub const NULL: Self = Self(0);

    /// Creates from raw value
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns raw value
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Checks if valid
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

/// Physical device type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicalDeviceType {
    /// Other/unknown type
    Other,
    /// Integrated GPU
    IntegratedGpu,
    /// Discrete GPU
    DiscreteGpu,
    /// Virtual GPU
    VirtualGpu,
    /// CPU renderer
    Cpu,
}

impl Default for PhysicalDeviceType {
    fn default() -> Self {
        Self::Other
    }
}

/// Physical device properties
#[derive(Clone, Debug)]
pub struct PhysicalDeviceProperties {
    /// API version supported
    pub api_version: u32,
    /// Driver version
    pub driver_version: u32,
    /// Vendor ID
    pub vendor_id: u32,
    /// Device ID
    pub device_id: u32,
    /// Device type
    pub device_type: PhysicalDeviceType,
    /// Device name
    pub device_name: String,
    /// Pipeline cache UUID
    pub pipeline_cache_uuid: [u8; 16],
    /// Limits
    pub limits: PhysicalDeviceLimits,
    /// Sparse properties
    pub sparse_properties: SparseProperties,
}

impl PhysicalDeviceProperties {
    /// Returns API version as major.minor.patch
    pub const fn api_version_parts(&self) -> (u32, u32, u32) {
        (
            (self.api_version >> 22) & 0x7F,
            (self.api_version >> 12) & 0x3FF,
            self.api_version & 0xFFF,
        )
    }

    /// Returns vendor name from ID
    pub fn vendor_name(&self) -> &'static str {
        match self.vendor_id {
            0x1002 => "AMD",
            0x1010 => "ImgTec",
            0x10DE => "NVIDIA",
            0x13B5 => "ARM",
            0x5143 => "Qualcomm",
            0x8086 => "Intel",
            _ => "Unknown",
        }
    }
}

/// Physical device limits
#[derive(Clone, Debug, Default)]
pub struct PhysicalDeviceLimits {
    /// Max 1D texture dimension
    pub max_image_dimension_1d: u32,
    /// Max 2D texture dimension
    pub max_image_dimension_2d: u32,
    /// Max 3D texture dimension
    pub max_image_dimension_3d: u32,
    /// Max cube texture dimension
    pub max_image_dimension_cube: u32,
    /// Max array layers
    pub max_image_array_layers: u32,
    /// Max texel buffer elements
    pub max_texel_buffer_elements: u32,
    /// Max uniform buffer range
    pub max_uniform_buffer_range: u32,
    /// Max storage buffer range
    pub max_storage_buffer_range: u32,
    /// Max push constants size
    pub max_push_constants_size: u32,
    /// Max memory allocation count
    pub max_memory_allocation_count: u32,
    /// Max sampler allocation count
    pub max_sampler_allocation_count: u32,
    /// Buffer-image granularity
    pub buffer_image_granularity: u64,
    /// Sparse address space size
    pub sparse_address_space_size: u64,
    /// Max bound descriptor sets
    pub max_bound_descriptor_sets: u32,
    /// Max per-stage descriptor samplers
    pub max_per_stage_descriptor_samplers: u32,
    /// Max per-stage descriptor uniform buffers
    pub max_per_stage_descriptor_uniform_buffers: u32,
    /// Max per-stage descriptor storage buffers
    pub max_per_stage_descriptor_storage_buffers: u32,
    /// Max per-stage descriptor sampled images
    pub max_per_stage_descriptor_sampled_images: u32,
    /// Max per-stage descriptor storage images
    pub max_per_stage_descriptor_storage_images: u32,
    /// Max per-stage descriptor input attachments
    pub max_per_stage_descriptor_input_attachments: u32,
    /// Max per-stage resources
    pub max_per_stage_resources: u32,
    /// Max descriptor set samplers
    pub max_descriptor_set_samplers: u32,
    /// Max descriptor set uniform buffers
    pub max_descriptor_set_uniform_buffers: u32,
    /// Max descriptor set uniform buffers dynamic
    pub max_descriptor_set_uniform_buffers_dynamic: u32,
    /// Max descriptor set storage buffers
    pub max_descriptor_set_storage_buffers: u32,
    /// Max descriptor set storage buffers dynamic
    pub max_descriptor_set_storage_buffers_dynamic: u32,
    /// Max descriptor set sampled images
    pub max_descriptor_set_sampled_images: u32,
    /// Max descriptor set storage images
    pub max_descriptor_set_storage_images: u32,
    /// Max descriptor set input attachments
    pub max_descriptor_set_input_attachments: u32,
    /// Max vertex input attributes
    pub max_vertex_input_attributes: u32,
    /// Max vertex input bindings
    pub max_vertex_input_bindings: u32,
    /// Max vertex input attribute offset
    pub max_vertex_input_attribute_offset: u32,
    /// Max vertex input binding stride
    pub max_vertex_input_binding_stride: u32,
    /// Max vertex output components
    pub max_vertex_output_components: u32,
    /// Max tessellation generation level
    pub max_tessellation_generation_level: u32,
    /// Max tessellation patch size
    pub max_tessellation_patch_size: u32,
    /// Max tessellation control per-vertex input components
    pub max_tessellation_control_per_vertex_input_components: u32,
    /// Max tessellation control per-vertex output components
    pub max_tessellation_control_per_vertex_output_components: u32,
    /// Max tessellation control per-patch output components
    pub max_tessellation_control_per_patch_output_components: u32,
    /// Max tessellation control total output components
    pub max_tessellation_control_total_output_components: u32,
    /// Max tessellation evaluation input components
    pub max_tessellation_evaluation_input_components: u32,
    /// Max tessellation evaluation output components
    pub max_tessellation_evaluation_output_components: u32,
    /// Max geometry shader invocations
    pub max_geometry_shader_invocations: u32,
    /// Max geometry input components
    pub max_geometry_input_components: u32,
    /// Max geometry output components
    pub max_geometry_output_components: u32,
    /// Max geometry output vertices
    pub max_geometry_output_vertices: u32,
    /// Max geometry total output components
    pub max_geometry_total_output_components: u32,
    /// Max fragment input components
    pub max_fragment_input_components: u32,
    /// Max fragment output attachments
    pub max_fragment_output_attachments: u32,
    /// Max fragment dual-source attachments
    pub max_fragment_dual_src_attachments: u32,
    /// Max fragment combined output resources
    pub max_fragment_combined_output_resources: u32,
    /// Max compute shared memory size
    pub max_compute_shared_memory_size: u32,
    /// Max compute workgroup count
    pub max_compute_work_group_count: [u32; 3],
    /// Max compute workgroup invocations
    pub max_compute_work_group_invocations: u32,
    /// Max compute workgroup size
    pub max_compute_work_group_size: [u32; 3],
    /// Subpixel precision bits
    pub sub_pixel_precision_bits: u32,
    /// Subtexel precision bits
    pub sub_texel_precision_bits: u32,
    /// Mip map precision bits
    pub mip_map_precision_bits: u32,
    /// Max draw indexed index value
    pub max_draw_indexed_index_value: u32,
    /// Max draw indirect count
    pub max_draw_indirect_count: u32,
    /// Max sampler LOD bias
    pub max_sampler_lod_bias: f32,
    /// Max sampler anisotropy
    pub max_sampler_anisotropy: f32,
    /// Max viewports
    pub max_viewports: u32,
    /// Max viewport dimensions
    pub max_viewport_dimensions: [u32; 2],
    /// Viewport bounds range
    pub viewport_bounds_range: [f32; 2],
    /// Viewport subpixel bits
    pub viewport_sub_pixel_bits: u32,
    /// Min memory map alignment
    pub min_memory_map_alignment: usize,
    /// Min texel buffer offset alignment
    pub min_texel_buffer_offset_alignment: u64,
    /// Min uniform buffer offset alignment
    pub min_uniform_buffer_offset_alignment: u64,
    /// Min storage buffer offset alignment
    pub min_storage_buffer_offset_alignment: u64,
    /// Min texel offset
    pub min_texel_offset: i32,
    /// Max texel offset
    pub max_texel_offset: u32,
    /// Min texel gather offset
    pub min_texel_gather_offset: i32,
    /// Max texel gather offset
    pub max_texel_gather_offset: u32,
    /// Min interpolation offset
    pub min_interpolation_offset: f32,
    /// Max interpolation offset
    pub max_interpolation_offset: f32,
    /// Subpixel interpolation offset bits
    pub sub_pixel_interpolation_offset_bits: u32,
    /// Max framebuffer width
    pub max_framebuffer_width: u32,
    /// Max framebuffer height
    pub max_framebuffer_height: u32,
    /// Max framebuffer layers
    pub max_framebuffer_layers: u32,
    /// Framebuffer color sample counts
    pub framebuffer_color_sample_counts: u32,
    /// Framebuffer depth sample counts
    pub framebuffer_depth_sample_counts: u32,
    /// Framebuffer stencil sample counts
    pub framebuffer_stencil_sample_counts: u32,
    /// Framebuffer no-attachment sample counts
    pub framebuffer_no_attachments_sample_counts: u32,
    /// Max color attachments
    pub max_color_attachments: u32,
    /// Sampled image color sample counts
    pub sampled_image_color_sample_counts: u32,
    /// Sampled image integer sample counts
    pub sampled_image_integer_sample_counts: u32,
    /// Sampled image depth sample counts
    pub sampled_image_depth_sample_counts: u32,
    /// Sampled image stencil sample counts
    pub sampled_image_stencil_sample_counts: u32,
    /// Storage image sample counts
    pub storage_image_sample_counts: u32,
    /// Max sample mask words
    pub max_sample_mask_words: u32,
    /// Timestamp compute and graphics
    pub timestamp_compute_and_graphics: bool,
    /// Timestamp period (nanoseconds per tick)
    pub timestamp_period: f32,
    /// Max clip distances
    pub max_clip_distances: u32,
    /// Max cull distances
    pub max_cull_distances: u32,
    /// Max combined clip and cull distances
    pub max_combined_clip_and_cull_distances: u32,
    /// Discrete queue priorities
    pub discrete_queue_priorities: u32,
    /// Point size range
    pub point_size_range: [f32; 2],
    /// Line width range
    pub line_width_range: [f32; 2],
    /// Point size granularity
    pub point_size_granularity: f32,
    /// Line width granularity
    pub line_width_granularity: f32,
    /// Strict lines
    pub strict_lines: bool,
    /// Standard sample locations
    pub standard_sample_locations: bool,
    /// Optimal buffer copy offset alignment
    pub optimal_buffer_copy_offset_alignment: u64,
    /// Optimal buffer copy row pitch alignment
    pub optimal_buffer_copy_row_pitch_alignment: u64,
    /// Non-coherent atom size
    pub non_coherent_atom_size: u64,
}

/// Sparse properties
#[derive(Clone, Debug, Default)]
pub struct SparseProperties {
    /// Residency standard 2D block shape
    pub residency_standard_2d_block_shape: bool,
    /// Residency standard 2D multisample block shape
    pub residency_standard_2d_multisample_block_shape: bool,
    /// Residency standard 3D block shape
    pub residency_standard_3d_block_shape: bool,
    /// Residency aligned mip size
    pub residency_aligned_mip_size: bool,
    /// Residency non-resident strict
    pub residency_non_resident_strict: bool,
}

/// Physical device features
#[derive(Clone, Debug, Default)]
pub struct PhysicalDeviceFeatures {
    /// Robust buffer access
    pub robust_buffer_access: bool,
    /// Full draw index uint32
    pub full_draw_index_uint32: bool,
    /// Image cube array
    pub image_cube_array: bool,
    /// Independent blend
    pub independent_blend: bool,
    /// Geometry shader
    pub geometry_shader: bool,
    /// Tessellation shader
    pub tessellation_shader: bool,
    /// Sample rate shading
    pub sample_rate_shading: bool,
    /// Dual-source blend
    pub dual_src_blend: bool,
    /// Logic operations
    pub logic_op: bool,
    /// Multi-draw indirect
    pub multi_draw_indirect: bool,
    /// Draw indirect first instance
    pub draw_indirect_first_instance: bool,
    /// Depth clamp
    pub depth_clamp: bool,
    /// Depth bias clamp
    pub depth_bias_clamp: bool,
    /// Fill mode non-solid
    pub fill_mode_non_solid: bool,
    /// Depth bounds
    pub depth_bounds: bool,
    /// Wide lines
    pub wide_lines: bool,
    /// Large points
    pub large_points: bool,
    /// Alpha to one
    pub alpha_to_one: bool,
    /// Multi-viewport
    pub multi_viewport: bool,
    /// Sampler anisotropy
    pub sampler_anisotropy: bool,
    /// Texture compression ETC2
    pub texture_compression_etc2: bool,
    /// Texture compression ASTC LDR
    pub texture_compression_astc_ldr: bool,
    /// Texture compression BC
    pub texture_compression_bc: bool,
    /// Occlusion query precise
    pub occlusion_query_precise: bool,
    /// Pipeline statistics query
    pub pipeline_statistics_query: bool,
    /// Vertex pipeline stores and atomics
    pub vertex_pipeline_stores_and_atomics: bool,
    /// Fragment stores and atomics
    pub fragment_stores_and_atomics: bool,
    /// Shader tessellation and geometry point size
    pub shader_tessellation_and_geometry_point_size: bool,
    /// Shader image gather extended
    pub shader_image_gather_extended: bool,
    /// Shader storage image extended formats
    pub shader_storage_image_extended_formats: bool,
    /// Shader storage image multisample
    pub shader_storage_image_multisample: bool,
    /// Shader storage image read without format
    pub shader_storage_image_read_without_format: bool,
    /// Shader storage image write without format
    pub shader_storage_image_write_without_format: bool,
    /// Shader uniform buffer array dynamic indexing
    pub shader_uniform_buffer_array_dynamic_indexing: bool,
    /// Shader sampled image array dynamic indexing
    pub shader_sampled_image_array_dynamic_indexing: bool,
    /// Shader storage buffer array dynamic indexing
    pub shader_storage_buffer_array_dynamic_indexing: bool,
    /// Shader storage image array dynamic indexing
    pub shader_storage_image_array_dynamic_indexing: bool,
    /// Shader clip distance
    pub shader_clip_distance: bool,
    /// Shader cull distance
    pub shader_cull_distance: bool,
    /// Shader float64
    pub shader_float64: bool,
    /// Shader int64
    pub shader_int64: bool,
    /// Shader int16
    pub shader_int16: bool,
    /// Shader resource residency
    pub shader_resource_residency: bool,
    /// Shader resource min LOD
    pub shader_resource_min_lod: bool,
    /// Sparse binding
    pub sparse_binding: bool,
    /// Sparse residency buffer
    pub sparse_residency_buffer: bool,
    /// Sparse residency image 2D
    pub sparse_residency_image_2d: bool,
    /// Sparse residency image 3D
    pub sparse_residency_image_3d: bool,
    /// Sparse residency 2 samples
    pub sparse_residency_2_samples: bool,
    /// Sparse residency 4 samples
    pub sparse_residency_4_samples: bool,
    /// Sparse residency 8 samples
    pub sparse_residency_8_samples: bool,
    /// Sparse residency 16 samples
    pub sparse_residency_16_samples: bool,
    /// Sparse residency aliased
    pub sparse_residency_aliased: bool,
    /// Variable multisample rate
    pub variable_multisample_rate: bool,
    /// Inherited queries
    pub inherited_queries: bool,
}

/// Queue family properties
#[derive(Clone, Debug)]
pub struct QueueFamilyProperties {
    /// Queue flags
    pub queue_flags: QueueFlags,
    /// Number of queues in family
    pub queue_count: u32,
    /// Timestamp valid bits
    pub timestamp_valid_bits: u32,
    /// Min image transfer granularity
    pub min_image_transfer_granularity: [u32; 3],
}

/// Queue capability flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct QueueFlags(pub u32);

impl QueueFlags {
    /// No capabilities
    pub const NONE: Self = Self(0);
    /// Graphics capable
    pub const GRAPHICS: Self = Self(1 << 0);
    /// Compute capable
    pub const COMPUTE: Self = Self(1 << 1);
    /// Transfer capable
    pub const TRANSFER: Self = Self(1 << 2);
    /// Sparse binding capable
    pub const SPARSE_BINDING: Self = Self(1 << 3);
    /// Protected capable
    pub const PROTECTED: Self = Self(1 << 4);
    /// Video decode capable
    pub const VIDEO_DECODE: Self = Self(1 << 5);
    /// Video encode capable
    pub const VIDEO_ENCODE: Self = Self(1 << 6);

    /// Checks if flag is set
    pub const fn contains(self, flag: Self) -> bool {
        (self.0 & flag.0) == flag.0
    }

    /// Checks if graphics capable
    pub const fn is_graphics(self) -> bool {
        self.contains(Self::GRAPHICS)
    }

    /// Checks if compute capable
    pub const fn is_compute(self) -> bool {
        self.contains(Self::COMPUTE)
    }

    /// Checks if transfer capable
    pub const fn is_transfer(self) -> bool {
        self.contains(Self::TRANSFER)
    }
}

impl core::ops::BitOr for QueueFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Device create info
#[derive(Clone, Debug)]
pub struct DeviceCreateInfo<'a> {
    /// Queue create infos
    pub queue_create_infos: Vec<DeviceQueueCreateInfo<'a>>,
    /// Enabled extension names
    pub enabled_extensions: Vec<&'a str>,
    /// Enabled features
    pub enabled_features: PhysicalDeviceFeatures,
}

impl<'a> DeviceCreateInfo<'a> {
    /// Creates device create info with graphics queue
    pub fn graphics() -> Self {
        Self {
            queue_create_infos: vec![DeviceQueueCreateInfo {
                queue_family_index: 0,
                queue_priorities: &[1.0],
            }],
            enabled_extensions: Vec::new(),
            enabled_features: PhysicalDeviceFeatures::default(),
        }
    }

    /// Adds an extension
    pub fn with_extension(mut self, extension: &'a str) -> Self {
        self.enabled_extensions.push(extension);
        self
    }

    /// Adds swapchain extension
    pub fn with_swapchain(self) -> Self {
        self.with_extension("VK_KHR_swapchain")
    }
}

/// Device queue create info
#[derive(Clone, Debug)]
pub struct DeviceQueueCreateInfo<'a> {
    /// Queue family index
    pub queue_family_index: u32,
    /// Queue priorities (one per queue)
    pub queue_priorities: &'a [f32],
}

/// Instance create info
#[derive(Clone, Debug)]
pub struct InstanceCreateInfo<'a> {
    /// Application info
    pub app_info: ApplicationInfo<'a>,
    /// Enabled layer names
    pub enabled_layers: Vec<&'a str>,
    /// Enabled extension names
    pub enabled_extensions: Vec<&'a str>,
}

impl<'a> InstanceCreateInfo<'a> {
    /// Creates instance create info
    pub fn new(app_name: &'a str, app_version: u32) -> Self {
        Self {
            app_info: ApplicationInfo {
                app_name,
                app_version,
                engine_name: "LUMINA",
                engine_version: 1,
                api_version: (1 << 22) | (3 << 12), // 1.3
            },
            enabled_layers: Vec::new(),
            enabled_extensions: Vec::new(),
        }
    }

    /// Adds validation layer
    pub fn with_validation(mut self) -> Self {
        self.enabled_layers.push("VK_LAYER_KHRONOS_validation");
        self
    }

    /// Adds surface extensions
    pub fn with_surface(mut self) -> Self {
        self.enabled_extensions.push("VK_KHR_surface");
        self
    }
}

/// Application info
#[derive(Clone, Debug)]
pub struct ApplicationInfo<'a> {
    /// Application name
    pub app_name: &'a str,
    /// Application version
    pub app_version: u32,
    /// Engine name
    pub engine_name: &'a str,
    /// Engine version
    pub engine_version: u32,
    /// API version
    pub api_version: u32,
}
