//! Device and physical device types
//!
//! This module provides types for GPU device enumeration and capabilities.

use core::num::NonZeroU32;

/// Physical device handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PhysicalDeviceHandle(pub NonZeroU32);

impl PhysicalDeviceHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

/// Logical device handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DeviceHandle(pub NonZeroU32);

impl DeviceHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

/// Physical device type
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum PhysicalDeviceType {
    /// Unknown device type
    #[default]
    Other = 0,
    /// Integrated GPU
    IntegratedGpu = 1,
    /// Discrete GPU
    DiscreteGpu = 2,
    /// Virtual GPU
    VirtualGpu = 3,
    /// CPU (software renderer)
    Cpu = 4,
}

impl PhysicalDeviceType {
    /// Is this a discrete GPU
    pub const fn is_discrete(self) -> bool {
        matches!(self, Self::DiscreteGpu)
    }

    /// Is this an integrated GPU
    pub const fn is_integrated(self) -> bool {
        matches!(self, Self::IntegratedGpu)
    }

    /// Priority for device selection (higher is better)
    pub const fn selection_priority(self) -> u32 {
        match self {
            Self::DiscreteGpu => 4,
            Self::IntegratedGpu => 3,
            Self::VirtualGpu => 2,
            Self::Cpu => 1,
            Self::Other => 0,
        }
    }
}

/// Physical device properties
#[derive(Clone, Debug)]
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
    pub device_name: alloc::string::String,
    /// Pipeline cache UUID
    pub pipeline_cache_uuid: [u8; 16],
    /// Device limits
    pub limits: PhysicalDeviceLimits,
    /// Sparse properties
    pub sparse_properties: SparseProperties,
}

/// Version number
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Vulkan 1.0
    pub const V1_0: Self = Self::new(1, 0, 0);
    /// Vulkan 1.1
    pub const V1_1: Self = Self::new(1, 1, 0);
    /// Vulkan 1.2
    pub const V1_2: Self = Self::new(1, 2, 0);
    /// Vulkan 1.3
    pub const V1_3: Self = Self::new(1, 3, 0);

    /// Packs version into a u32
    pub const fn pack(&self) -> u32 {
        (self.major << 22) | (self.minor << 12) | self.patch
    }

    /// Unpacks from a u32
    pub const fn unpack(packed: u32) -> Self {
        Self {
            major: (packed >> 22) & 0x7F,
            minor: (packed >> 12) & 0x3FF,
            patch: packed & 0xFFF,
        }
    }
}

impl core::fmt::Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Physical device limits
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PhysicalDeviceLimits {
    /// Maximum 1D image dimension
    pub max_image_dimension_1d: u32,
    /// Maximum 2D image dimension
    pub max_image_dimension_2d: u32,
    /// Maximum 3D image dimension
    pub max_image_dimension_3d: u32,
    /// Maximum cube image dimension
    pub max_image_dimension_cube: u32,
    /// Maximum array layers
    pub max_image_array_layers: u32,
    /// Maximum texel buffer elements
    pub max_texel_buffer_elements: u32,
    /// Maximum uniform buffer range
    pub max_uniform_buffer_range: u32,
    /// Maximum storage buffer range
    pub max_storage_buffer_range: u32,
    /// Maximum push constants size
    pub max_push_constants_size: u32,
    /// Maximum memory allocation count
    pub max_memory_allocation_count: u32,
    /// Maximum sampler allocation count
    pub max_sampler_allocation_count: u32,
    /// Buffer image granularity
    pub buffer_image_granularity: u64,
    /// Sparse address space size
    pub sparse_address_space_size: u64,
    /// Maximum bound descriptor sets
    pub max_bound_descriptor_sets: u32,
    /// Maximum per stage descriptor samplers
    pub max_per_stage_descriptor_samplers: u32,
    /// Maximum per stage descriptor uniform buffers
    pub max_per_stage_descriptor_uniform_buffers: u32,
    /// Maximum per stage descriptor storage buffers
    pub max_per_stage_descriptor_storage_buffers: u32,
    /// Maximum per stage descriptor sampled images
    pub max_per_stage_descriptor_sampled_images: u32,
    /// Maximum per stage descriptor storage images
    pub max_per_stage_descriptor_storage_images: u32,
    /// Maximum per stage descriptor input attachments
    pub max_per_stage_descriptor_input_attachments: u32,
    /// Maximum per stage resources
    pub max_per_stage_resources: u32,
    /// Maximum descriptor set samplers
    pub max_descriptor_set_samplers: u32,
    /// Maximum descriptor set uniform buffers
    pub max_descriptor_set_uniform_buffers: u32,
    /// Maximum descriptor set uniform buffers dynamic
    pub max_descriptor_set_uniform_buffers_dynamic: u32,
    /// Maximum descriptor set storage buffers
    pub max_descriptor_set_storage_buffers: u32,
    /// Maximum descriptor set storage buffers dynamic
    pub max_descriptor_set_storage_buffers_dynamic: u32,
    /// Maximum descriptor set sampled images
    pub max_descriptor_set_sampled_images: u32,
    /// Maximum descriptor set storage images
    pub max_descriptor_set_storage_images: u32,
    /// Maximum descriptor set input attachments
    pub max_descriptor_set_input_attachments: u32,
    /// Maximum vertex input attributes
    pub max_vertex_input_attributes: u32,
    /// Maximum vertex input bindings
    pub max_vertex_input_bindings: u32,
    /// Maximum vertex input attribute offset
    pub max_vertex_input_attribute_offset: u32,
    /// Maximum vertex input binding stride
    pub max_vertex_input_binding_stride: u32,
    /// Maximum vertex output components
    pub max_vertex_output_components: u32,
    /// Maximum tessellation generation level
    pub max_tessellation_generation_level: u32,
    /// Maximum tessellation patch size
    pub max_tessellation_patch_size: u32,
    /// Maximum tessellation control per vertex input components
    pub max_tessellation_control_per_vertex_input_components: u32,
    /// Maximum tessellation control per vertex output components
    pub max_tessellation_control_per_vertex_output_components: u32,
    /// Maximum tessellation control per patch output components
    pub max_tessellation_control_per_patch_output_components: u32,
    /// Maximum tessellation control total output components
    pub max_tessellation_control_total_output_components: u32,
    /// Maximum tessellation evaluation input components
    pub max_tessellation_evaluation_input_components: u32,
    /// Maximum tessellation evaluation output components
    pub max_tessellation_evaluation_output_components: u32,
    /// Maximum geometry shader invocations
    pub max_geometry_shader_invocations: u32,
    /// Maximum geometry input components
    pub max_geometry_input_components: u32,
    /// Maximum geometry output components
    pub max_geometry_output_components: u32,
    /// Maximum geometry output vertices
    pub max_geometry_output_vertices: u32,
    /// Maximum geometry total output components
    pub max_geometry_total_output_components: u32,
    /// Maximum fragment input components
    pub max_fragment_input_components: u32,
    /// Maximum fragment output attachments
    pub max_fragment_output_attachments: u32,
    /// Maximum fragment dual source attachments
    pub max_fragment_dual_src_attachments: u32,
    /// Maximum fragment combined output resources
    pub max_fragment_combined_output_resources: u32,
    /// Maximum compute shared memory size
    pub max_compute_shared_memory_size: u32,
    /// Maximum compute work group count
    pub max_compute_work_group_count: [u32; 3],
    /// Maximum compute work group invocations
    pub max_compute_work_group_invocations: u32,
    /// Maximum compute work group size
    pub max_compute_work_group_size: [u32; 3],
    /// Subpixel precision bits
    pub sub_pixel_precision_bits: u32,
    /// Subtexel precision bits
    pub sub_texel_precision_bits: u32,
    /// Mipmap precision bits
    pub mipmap_precision_bits: u32,
    /// Maximum draw indexed index value
    pub max_draw_indexed_index_value: u32,
    /// Maximum draw indirect count
    pub max_draw_indirect_count: u32,
    /// Maximum sampler LOD bias
    pub max_sampler_lod_bias: f32,
    /// Maximum sampler anisotropy
    pub max_sampler_anisotropy: f32,
    /// Maximum viewports
    pub max_viewports: u32,
    /// Maximum viewport dimensions
    pub max_viewport_dimensions: [u32; 2],
    /// Viewport bounds range
    pub viewport_bounds_range: [f32; 2],
    /// Viewport subpixel bits
    pub viewport_sub_pixel_bits: u32,
    /// Minimum memory map alignment
    pub min_memory_map_alignment: usize,
    /// Minimum texel buffer offset alignment
    pub min_texel_buffer_offset_alignment: u64,
    /// Minimum uniform buffer offset alignment
    pub min_uniform_buffer_offset_alignment: u64,
    /// Minimum storage buffer offset alignment
    pub min_storage_buffer_offset_alignment: u64,
    /// Minimum texel offset
    pub min_texel_offset: i32,
    /// Maximum texel offset
    pub max_texel_offset: u32,
    /// Minimum texel gather offset
    pub min_texel_gather_offset: i32,
    /// Maximum texel gather offset
    pub max_texel_gather_offset: u32,
    /// Minimum interpolation offset
    pub min_interpolation_offset: f32,
    /// Maximum interpolation offset
    pub max_interpolation_offset: f32,
    /// Subpixel interpolation offset bits
    pub sub_pixel_interpolation_offset_bits: u32,
    /// Maximum framebuffer width
    pub max_framebuffer_width: u32,
    /// Maximum framebuffer height
    pub max_framebuffer_height: u32,
    /// Maximum framebuffer layers
    pub max_framebuffer_layers: u32,
    /// Framebuffer color sample counts
    pub framebuffer_color_sample_counts: SampleCountFlags,
    /// Framebuffer depth sample counts
    pub framebuffer_depth_sample_counts: SampleCountFlags,
    /// Framebuffer stencil sample counts
    pub framebuffer_stencil_sample_counts: SampleCountFlags,
    /// Framebuffer no attachments sample counts
    pub framebuffer_no_attachments_sample_counts: SampleCountFlags,
    /// Maximum color attachments
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
    /// Maximum sample mask words
    pub max_sample_mask_words: u32,
    /// Timestamp compute and graphics
    pub timestamp_compute_and_graphics: bool,
    /// Timestamp period
    pub timestamp_period: f32,
    /// Maximum clip distances
    pub max_clip_distances: u32,
    /// Maximum cull distances
    pub max_cull_distances: u32,
    /// Maximum combined clip and cull distances
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

impl Default for PhysicalDeviceLimits {
    fn default() -> Self {
        // Sensible defaults for a modern GPU
        Self {
            max_image_dimension_1d: 16384,
            max_image_dimension_2d: 16384,
            max_image_dimension_3d: 2048,
            max_image_dimension_cube: 16384,
            max_image_array_layers: 2048,
            max_texel_buffer_elements: 128 * 1024 * 1024,
            max_uniform_buffer_range: 65536,
            max_storage_buffer_range: u32::MAX,
            max_push_constants_size: 256,
            max_memory_allocation_count: 4096,
            max_sampler_allocation_count: 4000,
            buffer_image_granularity: 1024,
            sparse_address_space_size: 1 << 40,
            max_bound_descriptor_sets: 8,
            max_per_stage_descriptor_samplers: 16,
            max_per_stage_descriptor_uniform_buffers: 15,
            max_per_stage_descriptor_storage_buffers: 16,
            max_per_stage_descriptor_sampled_images: 128,
            max_per_stage_descriptor_storage_images: 8,
            max_per_stage_descriptor_input_attachments: 8,
            max_per_stage_resources: 256,
            max_descriptor_set_samplers: 96,
            max_descriptor_set_uniform_buffers: 90,
            max_descriptor_set_uniform_buffers_dynamic: 8,
            max_descriptor_set_storage_buffers: 96,
            max_descriptor_set_storage_buffers_dynamic: 8,
            max_descriptor_set_sampled_images: 768,
            max_descriptor_set_storage_images: 48,
            max_descriptor_set_input_attachments: 8,
            max_vertex_input_attributes: 32,
            max_vertex_input_bindings: 32,
            max_vertex_input_attribute_offset: 2047,
            max_vertex_input_binding_stride: 2048,
            max_vertex_output_components: 128,
            max_tessellation_generation_level: 64,
            max_tessellation_patch_size: 32,
            max_tessellation_control_per_vertex_input_components: 128,
            max_tessellation_control_per_vertex_output_components: 128,
            max_tessellation_control_per_patch_output_components: 120,
            max_tessellation_control_total_output_components: 4096,
            max_tessellation_evaluation_input_components: 128,
            max_tessellation_evaluation_output_components: 128,
            max_geometry_shader_invocations: 32,
            max_geometry_input_components: 128,
            max_geometry_output_components: 128,
            max_geometry_output_vertices: 256,
            max_geometry_total_output_components: 1024,
            max_fragment_input_components: 128,
            max_fragment_output_attachments: 8,
            max_fragment_dual_src_attachments: 1,
            max_fragment_combined_output_resources: 16,
            max_compute_shared_memory_size: 32768,
            max_compute_work_group_count: [65535, 65535, 65535],
            max_compute_work_group_invocations: 1024,
            max_compute_work_group_size: [1024, 1024, 64],
            sub_pixel_precision_bits: 8,
            sub_texel_precision_bits: 8,
            mipmap_precision_bits: 8,
            max_draw_indexed_index_value: u32::MAX - 1,
            max_draw_indirect_count: u32::MAX,
            max_sampler_lod_bias: 16.0,
            max_sampler_anisotropy: 16.0,
            max_viewports: 16,
            max_viewport_dimensions: [16384, 16384],
            viewport_bounds_range: [-32768.0, 32768.0],
            viewport_sub_pixel_bits: 8,
            min_memory_map_alignment: 64,
            min_texel_buffer_offset_alignment: 16,
            min_uniform_buffer_offset_alignment: 256,
            min_storage_buffer_offset_alignment: 64,
            min_texel_offset: -8,
            max_texel_offset: 7,
            min_texel_gather_offset: -32,
            max_texel_gather_offset: 31,
            min_interpolation_offset: -0.5,
            max_interpolation_offset: 0.5,
            sub_pixel_interpolation_offset_bits: 4,
            max_framebuffer_width: 16384,
            max_framebuffer_height: 16384,
            max_framebuffer_layers: 2048,
            framebuffer_color_sample_counts: SampleCountFlags::all(),
            framebuffer_depth_sample_counts: SampleCountFlags::all(),
            framebuffer_stencil_sample_counts: SampleCountFlags::all(),
            framebuffer_no_attachments_sample_counts: SampleCountFlags::all(),
            max_color_attachments: 8,
            sampled_image_color_sample_counts: SampleCountFlags::all(),
            sampled_image_integer_sample_counts: SampleCountFlags::all(),
            sampled_image_depth_sample_counts: SampleCountFlags::all(),
            sampled_image_stencil_sample_counts: SampleCountFlags::all(),
            storage_image_sample_counts: SampleCountFlags::S1,
            max_sample_mask_words: 1,
            timestamp_compute_and_graphics: true,
            timestamp_period: 1.0,
            max_clip_distances: 8,
            max_cull_distances: 8,
            max_combined_clip_and_cull_distances: 8,
            discrete_queue_priorities: 2,
            point_size_range: [1.0, 256.0],
            line_width_range: [1.0, 64.0],
            point_size_granularity: 1.0,
            line_width_granularity: 1.0,
            strict_lines: true,
            standard_sample_locations: true,
            optimal_buffer_copy_offset_alignment: 1,
            optimal_buffer_copy_row_pitch_alignment: 1,
            non_coherent_atom_size: 256,
        }
    }
}

bitflags::bitflags! {
    /// Sample count flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct SampleCountFlags: u32 {
        /// 1 sample
        const S1 = 1 << 0;
        /// 2 samples
        const S2 = 1 << 1;
        /// 4 samples
        const S4 = 1 << 2;
        /// 8 samples
        const S8 = 1 << 3;
        /// 16 samples
        const S16 = 1 << 4;
        /// 32 samples
        const S32 = 1 << 5;
        /// 64 samples
        const S64 = 1 << 6;
    }
}

impl SampleCountFlags {
    /// All sample counts
    pub fn all() -> Self {
        Self::S1 | Self::S2 | Self::S4 | Self::S8 | Self::S16 | Self::S32 | Self::S64
    }

    /// Maximum supported sample count
    pub fn max_sample_count(self) -> u32 {
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
    /// Residency non-resident strict
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
    /// Dual source blend
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

impl PhysicalDeviceFeatures {
    /// All features enabled
    pub const fn all() -> Self {
        Self {
            robust_buffer_access: true,
            full_draw_index_uint32: true,
            image_cube_array: true,
            independent_blend: true,
            geometry_shader: true,
            tessellation_shader: true,
            sample_rate_shading: true,
            dual_src_blend: true,
            logic_op: true,
            multi_draw_indirect: true,
            draw_indirect_first_instance: true,
            depth_clamp: true,
            depth_bias_clamp: true,
            fill_mode_non_solid: true,
            depth_bounds: true,
            wide_lines: true,
            large_points: true,
            alpha_to_one: true,
            multi_viewport: true,
            sampler_anisotropy: true,
            texture_compression_etc2: false,
            texture_compression_astc_ldr: false,
            texture_compression_bc: true,
            occlusion_query_precise: true,
            pipeline_statistics_query: true,
            vertex_pipeline_stores_and_atomics: true,
            fragment_stores_and_atomics: true,
            shader_tessellation_and_geometry_point_size: true,
            shader_image_gather_extended: true,
            shader_storage_image_extended_formats: true,
            shader_storage_image_multisample: true,
            shader_storage_image_read_without_format: true,
            shader_storage_image_write_without_format: true,
            shader_uniform_buffer_array_dynamic_indexing: true,
            shader_sampled_image_array_dynamic_indexing: true,
            shader_storage_buffer_array_dynamic_indexing: true,
            shader_storage_image_array_dynamic_indexing: true,
            shader_clip_distance: true,
            shader_cull_distance: true,
            shader_float64: true,
            shader_int64: true,
            shader_int16: true,
            shader_resource_residency: true,
            shader_resource_min_lod: true,
            sparse_binding: true,
            sparse_residency_buffer: true,
            sparse_residency_image_2d: true,
            sparse_residency_image_3d: true,
            sparse_residency_2_samples: true,
            sparse_residency_4_samples: true,
            sparse_residency_8_samples: true,
            sparse_residency_16_samples: true,
            sparse_residency_aliased: true,
            variable_multisample_rate: true,
            inherited_queries: true,
        }
    }
}

/// Driver info
#[derive(Clone, Debug)]
pub struct PhysicalDeviceDriverInfo {
    /// Driver ID
    pub driver_id: DriverId,
    /// Driver name
    pub driver_name: alloc::string::String,
    /// Driver info
    pub driver_info: alloc::string::String,
    /// Conformance version
    pub conformance_version: ConformanceVersion,
}

/// Driver ID
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum DriverId {
    /// AMD proprietary
    AmdProprietary = 1,
    /// AMD open source
    AmdOpenSource = 2,
    /// Mesa RADV
    MesaRadv = 3,
    /// NVIDIA proprietary
    NvidiaProprietary = 4,
    /// Intel proprietary Windows
    IntelProprietaryWindows = 5,
    /// Intel open source Mesa
    IntelOpenSourceMesa = 6,
    /// Imagination proprietary
    ImaginationProprietary = 7,
    /// Qualcomm proprietary
    QualcommProprietary = 8,
    /// ARM proprietary
    ArmProprietary = 9,
    /// Google SwiftShader
    GoogleSwiftshader = 10,
    /// GGP proprietary
    GgpProprietary = 11,
    /// Broadcom proprietary
    BroadcomProprietary = 12,
    /// Mesa LLVMpipe
    MesaLlvmpipe = 13,
    /// MoltenVK
    Moltenvk = 14,
    /// Core AVi Mesa
    CoreAviMesa = 15,
    /// Samsung proprietary
    SamsungProprietary = 18,
    /// Mesa Venus
    MesaVenus = 19,
    /// Mesa Dozen
    MesaDozen = 20,
    /// Mesa NVK
    MesaNvk = 21,
    /// Unknown driver
    Unknown = 0,
}

/// Conformance version
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ConformanceVersion {
    /// Major version
    pub major: u8,
    /// Minor version
    pub minor: u8,
    /// Subminor version
    pub subminor: u8,
    /// Patch version
    pub patch: u8,
}
