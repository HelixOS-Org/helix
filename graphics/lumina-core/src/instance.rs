//! Instance and physical device types
//!
//! This module provides types for instance creation and physical device enumeration.

extern crate alloc;
use alloc::vec::Vec;

/// Instance handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct InstanceHandle(pub u64);

impl InstanceHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// Physical device handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PhysicalDeviceHandle(pub u64);

impl PhysicalDeviceHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// Instance create info
#[derive(Clone, Debug, Default)]
pub struct InstanceCreateInfo {
    /// Application name
    pub application_name: Vec<u8>,
    /// Application version
    pub application_version: Version,
    /// Engine name
    pub engine_name: Vec<u8>,
    /// Engine version
    pub engine_version: Version,
    /// API version
    pub api_version: Version,
    /// Enabled layer names
    pub enabled_layers: Vec<Vec<u8>>,
    /// Enabled extension names
    pub enabled_extensions: Vec<Vec<u8>>,
}

impl InstanceCreateInfo {
    /// Creates new instance info
    pub fn new(app_name: &[u8], app_version: Version) -> Self {
        Self {
            application_name: app_name.to_vec(),
            application_version: app_version,
            engine_name: b"Lumina".to_vec(),
            engine_version: Version::new(1, 0, 0),
            api_version: Version::new(1, 3, 0),
            enabled_layers: Vec::new(),
            enabled_extensions: Vec::new(),
        }
    }

    /// Adds a layer
    pub fn with_layer(mut self, layer: &[u8]) -> Self {
        self.enabled_layers.push(layer.to_vec());
        self
    }

    /// Adds an extension
    pub fn with_extension(mut self, ext: &[u8]) -> Self {
        self.enabled_extensions.push(ext.to_vec());
        self
    }

    /// Enables validation layers
    pub fn with_validation(self) -> Self {
        self.with_layer(b"VK_LAYER_KHRONOS_validation")
    }

    /// Enables surface extensions
    pub fn with_surface(self) -> Self {
        self.with_extension(b"VK_KHR_surface")
    }
}

/// Version
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Version {
    /// Major version
    pub major: u32,
    /// Minor version
    pub minor: u32,
    /// Patch version
    pub patch: u32,
}

impl Version {
    /// Creates a new version
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Vulkan 1.0
    pub const VK_1_0: Self = Self::new(1, 0, 0);
    /// Vulkan 1.1
    pub const VK_1_1: Self = Self::new(1, 1, 0);
    /// Vulkan 1.2
    pub const VK_1_2: Self = Self::new(1, 2, 0);
    /// Vulkan 1.3
    pub const VK_1_3: Self = Self::new(1, 3, 0);

    /// Encodes to u32 (Vulkan format)
    pub const fn encode(&self) -> u32 {
        (self.major << 22) | (self.minor << 12) | self.patch
    }

    /// Decodes from u32
    pub const fn decode(encoded: u32) -> Self {
        Self {
            major: (encoded >> 22) & 0x7F,
            minor: (encoded >> 12) & 0x3FF,
            patch: encoded & 0xFFF,
        }
    }

    /// Compares versions
    pub const fn compare(&self, other: &Self) -> i32 {
        if self.major != other.major {
            return self.major as i32 - other.major as i32;
        }
        if self.minor != other.minor {
            return self.minor as i32 - other.minor as i32;
        }
        self.patch as i32 - other.patch as i32
    }

    /// Is at least version
    pub const fn is_at_least(&self, other: &Self) -> bool {
        self.compare(other) >= 0
    }
}

/// Physical device type
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PhysicalDeviceType {
    /// Unknown
    #[default]
    Other,
    /// Integrated GPU
    IntegratedGpu,
    /// Discrete GPU
    DiscreteGpu,
    /// Virtual GPU
    VirtualGpu,
    /// CPU
    Cpu,
}

impl PhysicalDeviceType {
    /// Priority for selection (higher = better)
    pub const fn priority(&self) -> u32 {
        match self {
            Self::DiscreteGpu => 5,
            Self::IntegratedGpu => 4,
            Self::VirtualGpu => 3,
            Self::Cpu => 2,
            Self::Other => 1,
        }
    }
}

/// Physical device properties
#[derive(Clone, Debug, Default)]
pub struct PhysicalDeviceProperties {
    /// API version
    pub api_version: Version,
    /// Driver version
    pub driver_version: u32,
    /// Vendor ID
    pub vendor_id: u32,
    /// Device ID
    pub device_id: u32,
    /// Device type
    pub device_type: PhysicalDeviceType,
    /// Device name
    pub device_name: Vec<u8>,
    /// Pipeline cache UUID
    pub pipeline_cache_uuid: [u8; 16],
    /// Limits
    pub limits: PhysicalDeviceLimits,
    /// Sparse properties
    pub sparse_properties: SparseProperties,
}

impl PhysicalDeviceProperties {
    /// Is NVIDIA
    pub const fn is_nvidia(&self) -> bool {
        self.vendor_id == 0x10DE
    }

    /// Is AMD
    pub const fn is_amd(&self) -> bool {
        self.vendor_id == 0x1002
    }

    /// Is Intel
    pub const fn is_intel(&self) -> bool {
        self.vendor_id == 0x8086
    }

    /// Is discrete GPU
    pub const fn is_discrete(&self) -> bool {
        matches!(self.device_type, PhysicalDeviceType::DiscreteGpu)
    }
}

/// Physical device limits
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PhysicalDeviceLimits {
    /// Max image dimension 1D
    pub max_image_dimension_1d: u32,
    /// Max image dimension 2D
    pub max_image_dimension_2d: u32,
    /// Max image dimension 3D
    pub max_image_dimension_3d: u32,
    /// Max image dimension cube
    pub max_image_dimension_cube: u32,
    /// Max image array layers
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
    /// Buffer image granularity
    pub buffer_image_granularity: u64,
    /// Sparse address space size
    pub sparse_address_space_size: u64,
    /// Max bound descriptor sets
    pub max_bound_descriptor_sets: u32,
    /// Max per stage descriptor samplers
    pub max_per_stage_descriptor_samplers: u32,
    /// Max per stage descriptor uniform buffers
    pub max_per_stage_descriptor_uniform_buffers: u32,
    /// Max per stage descriptor storage buffers
    pub max_per_stage_descriptor_storage_buffers: u32,
    /// Max per stage descriptor sampled images
    pub max_per_stage_descriptor_sampled_images: u32,
    /// Max per stage descriptor storage images
    pub max_per_stage_descriptor_storage_images: u32,
    /// Max per stage descriptor input attachments
    pub max_per_stage_descriptor_input_attachments: u32,
    /// Max per stage resources
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
    /// Max tessellation control per vertex input components
    pub max_tessellation_control_per_vertex_input_components: u32,
    /// Max tessellation control per vertex output components
    pub max_tessellation_control_per_vertex_output_components: u32,
    /// Max tessellation control per patch output components
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
    /// Max fragment dual src attachments
    pub max_fragment_dual_src_attachments: u32,
    /// Max fragment combined output resources
    pub max_fragment_combined_output_resources: u32,
    /// Max compute shared memory size
    pub max_compute_shared_memory_size: u32,
    /// Max compute work group count
    pub max_compute_work_group_count: [u32; 3],
    /// Max compute work group invocations
    pub max_compute_work_group_invocations: u32,
    /// Max compute work group size
    pub max_compute_work_group_size: [u32; 3],
    /// Subpixel precision bits
    pub sub_pixel_precision_bits: u32,
    /// Subpixel interpolation offset bits
    pub sub_texel_precision_bits: u32,
    /// Mipmap precision bits
    pub mipmap_precision_bits: u32,
    /// Max draw indexed index value
    pub max_draw_indexed_index_value: u32,
    /// Max draw indirect count
    pub max_draw_indirect_count: u32,
    /// Max sampler lod bias
    pub max_sampler_lod_bias: f32,
    /// Max sampler anisotropy
    pub max_sampler_anisotropy: f32,
    /// Max viewports
    pub max_viewports: u32,
    /// Max viewport dimensions
    pub max_viewport_dimensions: [u32; 2],
    /// Viewport bounds range
    pub viewport_bounds_range: [f32; 2],
    /// Viewport sub pixel bits
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
    /// Sub pixel interpolation offset bits
    pub sub_pixel_interpolation_offset_bits: u32,
    /// Max framebuffer width
    pub max_framebuffer_width: u32,
    /// Max framebuffer height
    pub max_framebuffer_height: u32,
    /// Max framebuffer layers
    pub max_framebuffer_layers: u32,
    /// Framebuffer color sample counts
    pub framebuffer_color_sample_counts: SampleCountFlags,
    /// Framebuffer depth sample counts
    pub framebuffer_depth_sample_counts: SampleCountFlags,
    /// Framebuffer stencil sample counts
    pub framebuffer_stencil_sample_counts: SampleCountFlags,
    /// Framebuffer no attachments sample counts
    pub framebuffer_no_attachments_sample_counts: SampleCountFlags,
    /// Max color attachments
    pub max_color_attachments: u32,
    /// Sampled image color sample counts
    pub sampled_image_color_sample_counts: SampleCountFlags,
    /// Sampled image integer sample counts
    pub sampled_image_integer_sample_counts: SampleCountFlags,
    /// Sampled image depth sample counts
    pub sampled_image_depth_sample_counts: SampleCountFlags,
    /// Sampled image stencil sample counts
    pub sampled_image_stencil_sample_counts: SampleCountFlags,
    /// Storage image sample counts
    pub storage_image_sample_counts: SampleCountFlags,
    /// Max sample mask words
    pub max_sample_mask_words: u32,
    /// Timestamp compute and graphics
    pub timestamp_compute_and_graphics: bool,
    /// Timestamp period (ns per tick)
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
    /// Non coherent atom size
    pub non_coherent_atom_size: u64,
}

/// Sample count flags
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SampleCountFlags(pub u8);

impl SampleCountFlags {
    /// 1 sample
    pub const S1: Self = Self(1 << 0);
    /// 2 samples
    pub const S2: Self = Self(1 << 1);
    /// 4 samples
    pub const S4: Self = Self(1 << 2);
    /// 8 samples
    pub const S8: Self = Self(1 << 3);
    /// 16 samples
    pub const S16: Self = Self(1 << 4);
    /// 32 samples
    pub const S32: Self = Self(1 << 5);
    /// 64 samples
    pub const S64: Self = Self(1 << 6);

    /// Checks if contains
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    /// Maximum supported sample count
    pub const fn max_supported(&self) -> u32 {
        if self.contains(Self::S64) {
            64
        } else if self.contains(Self::S32) {
            32
        } else if self.contains(Self::S16) {
            16
        } else if self.contains(Self::S8) {
            8
        } else if self.contains(Self::S4) {
            4
        } else if self.contains(Self::S2) {
            2
        } else {
            1
        }
    }
}

/// Sparse properties
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SparseProperties {
    /// Residency standard 2D block shape
    pub residency_standard_2d_block_shape: bool,
    /// Residency standard 2D multisample block shape
    pub residency_standard_2d_multisample_block_shape: bool,
    /// Residency standard 3D block shape
    pub residency_standard_3d_block_shape: bool,
    /// Residency aligned mip size
    pub residency_aligned_mip_size: bool,
    /// Residency non resident strict
    pub residency_non_resident_strict: bool,
}

/// Physical device features
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
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
    /// Dual src blend
    pub dual_src_blend: bool,
    /// Logic op
    pub logic_op: bool,
    /// Multi draw indirect
    pub multi_draw_indirect: bool,
    /// Draw indirect first instance
    pub draw_indirect_first_instance: bool,
    /// Depth clamp
    pub depth_clamp: bool,
    /// Depth bias clamp
    pub depth_bias_clamp: bool,
    /// Fill mode non solid
    pub fill_mode_non_solid: bool,
    /// Depth bounds
    pub depth_bounds: bool,
    /// Wide lines
    pub wide_lines: bool,
    /// Large points
    pub large_points: bool,
    /// Alpha to one
    pub alpha_to_one: bool,
    /// Multi viewport
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
    /// Shader resource min lod
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

/// Memory heap
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MemoryHeap {
    /// Size in bytes
    pub size: u64,
    /// Flags
    pub flags: MemoryHeapFlags,
}

/// Memory heap flags
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MemoryHeapFlags(pub u32);

impl MemoryHeapFlags {
    /// Device local
    pub const DEVICE_LOCAL: Self = Self(1 << 0);
    /// Multi instance
    pub const MULTI_INSTANCE: Self = Self(1 << 1);

    /// Is device local
    pub const fn is_device_local(&self) -> bool {
        (self.0 & Self::DEVICE_LOCAL.0) != 0
    }
}

/// Memory type
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MemoryType {
    /// Property flags
    pub property_flags: MemoryPropertyFlags,
    /// Heap index
    pub heap_index: u32,
}

/// Memory property flags
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MemoryPropertyFlags(pub u32);

impl MemoryPropertyFlags {
    /// Device local
    pub const DEVICE_LOCAL: Self = Self(1 << 0);
    /// Host visible
    pub const HOST_VISIBLE: Self = Self(1 << 1);
    /// Host coherent
    pub const HOST_COHERENT: Self = Self(1 << 2);
    /// Host cached
    pub const HOST_CACHED: Self = Self(1 << 3);
    /// Lazily allocated
    pub const LAZILY_ALLOCATED: Self = Self(1 << 4);
    /// Protected
    pub const PROTECTED: Self = Self(1 << 5);

    /// Checks if contains
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    /// Is device local
    pub const fn is_device_local(&self) -> bool {
        self.contains(Self::DEVICE_LOCAL)
    }

    /// Is host visible
    pub const fn is_host_visible(&self) -> bool {
        self.contains(Self::HOST_VISIBLE)
    }
}

/// Physical device memory properties
#[derive(Clone, Debug, Default)]
pub struct PhysicalDeviceMemoryProperties {
    /// Memory types
    pub memory_types: Vec<MemoryType>,
    /// Memory heaps
    pub memory_heaps: Vec<MemoryHeap>,
}

impl PhysicalDeviceMemoryProperties {
    /// Finds memory type
    pub fn find_memory_type(
        &self,
        type_filter: u32,
        properties: MemoryPropertyFlags,
    ) -> Option<u32> {
        for (i, mem_type) in self.memory_types.iter().enumerate() {
            if (type_filter & (1 << i)) != 0
                && (mem_type.property_flags.0 & properties.0) == properties.0
            {
                return Some(i as u32);
            }
        }
        None
    }

    /// Total device local memory
    pub fn total_device_local(&self) -> u64 {
        self.memory_heaps
            .iter()
            .filter(|h| h.flags.is_device_local())
            .map(|h| h.size)
            .sum()
    }
}
