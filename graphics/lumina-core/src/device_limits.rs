//! Device Limits and Properties for Lumina
//!
//! This module provides comprehensive GPU device limits, properties,
//! and feature capability queries.

use alloc::string::String;

// ============================================================================
// Device Properties
// ============================================================================

/// GPU device properties
#[derive(Clone, Debug)]
#[repr(C)]
pub struct DeviceProperties {
    /// API version
    pub api_version: ApiVersion,
    /// Driver version
    pub driver_version: u32,
    /// Vendor ID
    pub vendor_id: u32,
    /// Device ID
    pub device_id: u32,
    /// Device type
    pub device_type: DeviceType,
    /// Device name
    pub device_name: DeviceName,
    /// Pipeline cache UUID
    pub pipeline_cache_uuid: [u8; 16],
    /// Device limits
    pub limits: DeviceLimits,
    /// Sparse properties
    pub sparse_properties: SparseProperties,
}

impl DeviceProperties {
    /// Creates new device properties
    #[inline]
    pub fn new(device_name: &str, device_type: DeviceType) -> Self {
        Self {
            api_version: ApiVersion::LUMINA_1_0,
            driver_version: 1,
            vendor_id: 0,
            device_id: 0,
            device_type,
            device_name: DeviceName::from_str(device_name),
            pipeline_cache_uuid: [0; 16],
            limits: DeviceLimits::default(),
            sparse_properties: SparseProperties::default(),
        }
    }

    /// Is discrete GPU
    #[inline]
    pub const fn is_discrete(&self) -> bool {
        matches!(self.device_type, DeviceType::DiscreteGpu)
    }

    /// Is integrated GPU
    #[inline]
    pub const fn is_integrated(&self) -> bool {
        matches!(self.device_type, DeviceType::IntegratedGpu)
    }

    /// Vendor name
    #[inline]
    pub const fn vendor_name(&self) -> &'static str {
        match self.vendor_id {
            0x1002 => "AMD",
            0x10DE => "NVIDIA",
            0x8086 => "Intel",
            0x13B5 => "ARM",
            0x5143 => "Qualcomm",
            0x1010 => "ImgTec",
            0x106B => "Apple",
            0x1022 => "AMD",
            0x1414 => "Microsoft",
            0x15AD => "VMware",
            _ => "Unknown",
        }
    }
}

/// API version
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ApiVersion {
    /// Major version
    pub major: u32,
    /// Minor version
    pub minor: u32,
    /// Patch version
    pub patch: u32,
}

impl ApiVersion {
    /// Creates a new API version
    #[inline]
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Lumina 1.0
    pub const LUMINA_1_0: Self = Self::new(1, 0, 0);
    /// Lumina 1.1
    pub const LUMINA_1_1: Self = Self::new(1, 1, 0);
    /// Lumina 1.2
    pub const LUMINA_1_2: Self = Self::new(1, 2, 0);
    /// Lumina 1.3
    pub const LUMINA_1_3: Self = Self::new(1, 3, 0);

    /// Packed version number
    #[inline]
    pub const fn packed(&self) -> u32 {
        ((self.major & 0x3FF) << 22) | ((self.minor & 0x3FF) << 12) | (self.patch & 0xFFF)
    }

    /// From packed version
    #[inline]
    pub const fn from_packed(packed: u32) -> Self {
        Self {
            major: (packed >> 22) & 0x3FF,
            minor: (packed >> 12) & 0x3FF,
            patch: packed & 0xFFF,
        }
    }

    /// Is at least version
    #[inline]
    pub const fn is_at_least(&self, major: u32, minor: u32) -> bool {
        self.major > major || (self.major == major && self.minor >= minor)
    }
}

impl Default for ApiVersion {
    fn default() -> Self {
        Self::LUMINA_1_0
    }
}

/// Device type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DeviceType {
    /// Other/unknown device type
    Other         = 0,
    /// Integrated GPU
    IntegratedGpu = 1,
    /// Discrete GPU
    #[default]
    DiscreteGpu   = 2,
    /// Virtual GPU
    VirtualGpu    = 3,
    /// CPU (software rendering)
    Cpu           = 4,
}

impl DeviceType {
    /// Type name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Other => "Other",
            Self::IntegratedGpu => "Integrated GPU",
            Self::DiscreteGpu => "Discrete GPU",
            Self::VirtualGpu => "Virtual GPU",
            Self::Cpu => "CPU",
        }
    }

    /// Is hardware GPU
    #[inline]
    pub const fn is_hardware(&self) -> bool {
        matches!(self, Self::DiscreteGpu | Self::IntegratedGpu)
    }
}

/// Device name (fixed-size for no_std)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DeviceName {
    /// Name buffer
    pub data: [u8; 256],
    /// Name length
    pub len: usize,
}

impl DeviceName {
    /// Creates from string
    #[inline]
    pub fn from_str(s: &str) -> Self {
        let mut data = [0u8; 256];
        let len = s.len().min(255);
        data[..len].copy_from_slice(&s.as_bytes()[..len]);
        Self { data, len }
    }

    /// As string slice
    #[inline]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.data[..self.len]).unwrap_or("Unknown")
    }

    /// To String
    #[inline]
    pub fn to_string(&self) -> String {
        String::from(self.as_str())
    }
}

impl Default for DeviceName {
    fn default() -> Self {
        Self::from_str("Unknown GPU")
    }
}

// ============================================================================
// Device Limits
// ============================================================================

/// GPU device limits
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DeviceLimits {
    // Image limits
    /// Max 1D image dimension
    pub max_image_dimension_1d: u32,
    /// Max 2D image dimension
    pub max_image_dimension_2d: u32,
    /// Max 3D image dimension
    pub max_image_dimension_3d: u32,
    /// Max cube image dimension
    pub max_image_dimension_cube: u32,
    /// Max image array layers
    pub max_image_array_layers: u32,

    // Buffer limits
    /// Max texel buffer elements
    pub max_texel_buffer_elements: u32,
    /// Max uniform buffer range
    pub max_uniform_buffer_range: u32,
    /// Max storage buffer range
    pub max_storage_buffer_range: u32,
    /// Max push constants size
    pub max_push_constants_size: u32,

    // Memory limits
    /// Max memory allocation count
    pub max_memory_allocation_count: u32,
    /// Max sampler allocation count
    pub max_sampler_allocation_count: u32,
    /// Buffer image granularity
    pub buffer_image_granularity: u64,
    /// Non-coherent atom size
    pub non_coherent_atom_size: u64,

    // Descriptor limits
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

    // Vertex input limits
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

    // Tessellation limits
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

    // Geometry limits
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

    // Fragment limits
    /// Max fragment input components
    pub max_fragment_input_components: u32,
    /// Max fragment output attachments
    pub max_fragment_output_attachments: u32,
    /// Max fragment dual src attachments
    pub max_fragment_dual_src_attachments: u32,
    /// Max fragment combined output resources
    pub max_fragment_combined_output_resources: u32,

    // Compute limits
    /// Max compute shared memory size
    pub max_compute_shared_memory_size: u32,
    /// Max compute work group count
    pub max_compute_work_group_count: [u32; 3],
    /// Max compute work group invocations
    pub max_compute_work_group_invocations: u32,
    /// Max compute work group size
    pub max_compute_work_group_size: [u32; 3],

    // Subpixel/subgroup limits
    /// Sub-pixel precision bits
    pub sub_pixel_precision_bits: u32,
    /// Sub-texel precision bits
    pub sub_texel_precision_bits: u32,
    /// Mipmap precision bits
    pub mipmap_precision_bits: u32,

    // Draw limits
    /// Max draw indexed index value
    pub max_draw_indexed_index_value: u32,
    /// Max draw indirect count
    pub max_draw_indirect_count: u32,

    // Sampler limits
    /// Max sampler LOD bias
    pub max_sampler_lod_bias: f32,
    /// Max sampler anisotropy
    pub max_sampler_anisotropy: f32,

    // Viewport limits
    /// Max viewports
    pub max_viewports: u32,
    /// Max viewport dimensions
    pub max_viewport_dimensions: [u32; 2],
    /// Viewport bounds range
    pub viewport_bounds_range: [f32; 2],
    /// Viewport sub-pixel bits
    pub viewport_sub_pixel_bits: u32,

    // Alignment requirements
    /// Min memory map alignment
    pub min_memory_map_alignment: u64,
    /// Min texel buffer offset alignment
    pub min_texel_buffer_offset_alignment: u64,
    /// Min uniform buffer offset alignment
    pub min_uniform_buffer_offset_alignment: u64,
    /// Min storage buffer offset alignment
    pub min_storage_buffer_offset_alignment: u64,

    // Texel offsets
    /// Min texel offset
    pub min_texel_offset: i32,
    /// Max texel offset
    pub max_texel_offset: u32,
    /// Min texel gather offset
    pub min_texel_gather_offset: i32,
    /// Max texel gather offset
    pub max_texel_gather_offset: u32,

    // Interpolation offsets
    /// Min interpolation offset
    pub min_interpolation_offset: f32,
    /// Max interpolation offset
    pub max_interpolation_offset: f32,
    /// Sub-pixel interpolation offset bits
    pub sub_pixel_interpolation_offset_bits: u32,

    // Framebuffer limits
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

    // Color attachment limits
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
    /// Timestamp period
    pub timestamp_period: f32,

    // Clip/cull limits
    /// Max clip distances
    pub max_clip_distances: u32,
    /// Max cull distances
    pub max_cull_distances: u32,
    /// Max combined clip and cull distances
    pub max_combined_clip_and_cull_distances: u32,

    // Point size limits
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

    // Optimal buffer copy limits
    /// Optimal buffer copy offset alignment
    pub optimal_buffer_copy_offset_alignment: u64,
    /// Optimal buffer copy row pitch alignment
    pub optimal_buffer_copy_row_pitch_alignment: u64,
}

impl Default for DeviceLimits {
    fn default() -> Self {
        Self {
            // Image limits - typical modern GPU
            max_image_dimension_1d: 16384,
            max_image_dimension_2d: 16384,
            max_image_dimension_3d: 2048,
            max_image_dimension_cube: 16384,
            max_image_array_layers: 2048,

            // Buffer limits
            max_texel_buffer_elements: 128 * 1024 * 1024,
            max_uniform_buffer_range: 64 * 1024,
            max_storage_buffer_range: 2 * 1024 * 1024 * 1024,
            max_push_constants_size: 256,

            // Memory limits
            max_memory_allocation_count: 4096,
            max_sampler_allocation_count: 4000,
            buffer_image_granularity: 1024,
            non_coherent_atom_size: 64,

            // Descriptor limits
            max_bound_descriptor_sets: 8,
            max_per_stage_descriptor_samplers: 16,
            max_per_stage_descriptor_uniform_buffers: 15,
            max_per_stage_descriptor_storage_buffers: 16,
            max_per_stage_descriptor_sampled_images: 128,
            max_per_stage_descriptor_storage_images: 16,
            max_per_stage_descriptor_input_attachments: 8,
            max_per_stage_resources: 256,

            max_descriptor_set_samplers: 96,
            max_descriptor_set_uniform_buffers: 90,
            max_descriptor_set_uniform_buffers_dynamic: 8,
            max_descriptor_set_storage_buffers: 96,
            max_descriptor_set_storage_buffers_dynamic: 8,
            max_descriptor_set_sampled_images: 768,
            max_descriptor_set_storage_images: 96,
            max_descriptor_set_input_attachments: 8,

            // Vertex input limits
            max_vertex_input_attributes: 32,
            max_vertex_input_bindings: 32,
            max_vertex_input_attribute_offset: 2047,
            max_vertex_input_binding_stride: 2048,
            max_vertex_output_components: 128,

            // Tessellation limits
            max_tessellation_generation_level: 64,
            max_tessellation_patch_size: 32,
            max_tessellation_control_per_vertex_input_components: 128,
            max_tessellation_control_per_vertex_output_components: 128,
            max_tessellation_control_per_patch_output_components: 120,
            max_tessellation_control_total_output_components: 4096,
            max_tessellation_evaluation_input_components: 128,
            max_tessellation_evaluation_output_components: 128,

            // Geometry limits
            max_geometry_shader_invocations: 32,
            max_geometry_input_components: 128,
            max_geometry_output_components: 128,
            max_geometry_output_vertices: 256,
            max_geometry_total_output_components: 1024,

            // Fragment limits
            max_fragment_input_components: 128,
            max_fragment_output_attachments: 8,
            max_fragment_dual_src_attachments: 1,
            max_fragment_combined_output_resources: 16,

            // Compute limits
            max_compute_shared_memory_size: 32768,
            max_compute_work_group_count: [65535, 65535, 65535],
            max_compute_work_group_invocations: 1024,
            max_compute_work_group_size: [1024, 1024, 64],

            // Subpixel limits
            sub_pixel_precision_bits: 8,
            sub_texel_precision_bits: 8,
            mipmap_precision_bits: 8,

            // Draw limits
            max_draw_indexed_index_value: u32::MAX - 1,
            max_draw_indirect_count: u32::MAX,

            // Sampler limits
            max_sampler_lod_bias: 16.0,
            max_sampler_anisotropy: 16.0,

            // Viewport limits
            max_viewports: 16,
            max_viewport_dimensions: [16384, 16384],
            viewport_bounds_range: [-32768.0, 32767.0],
            viewport_sub_pixel_bits: 8,

            // Alignment requirements
            min_memory_map_alignment: 64,
            min_texel_buffer_offset_alignment: 16,
            min_uniform_buffer_offset_alignment: 256,
            min_storage_buffer_offset_alignment: 16,

            // Texel offsets
            min_texel_offset: -8,
            max_texel_offset: 7,
            min_texel_gather_offset: -32,
            max_texel_gather_offset: 31,

            // Interpolation offsets
            min_interpolation_offset: -0.5,
            max_interpolation_offset: 0.4375,
            sub_pixel_interpolation_offset_bits: 4,

            // Framebuffer limits
            max_framebuffer_width: 16384,
            max_framebuffer_height: 16384,
            max_framebuffer_layers: 2048,
            framebuffer_color_sample_counts: SampleCountFlags::ALL,
            framebuffer_depth_sample_counts: SampleCountFlags::ALL,
            framebuffer_stencil_sample_counts: SampleCountFlags::ALL,
            framebuffer_no_attachments_sample_counts: SampleCountFlags::ALL,

            // Color attachment limits
            max_color_attachments: 8,
            sampled_image_color_sample_counts: SampleCountFlags::ALL,
            sampled_image_integer_sample_counts: SampleCountFlags::ALL,
            sampled_image_depth_sample_counts: SampleCountFlags::ALL,
            sampled_image_stencil_sample_counts: SampleCountFlags::ALL,
            storage_image_sample_counts: SampleCountFlags::SAMPLE_1,

            max_sample_mask_words: 1,

            timestamp_compute_and_graphics: true,
            timestamp_period: 1.0,

            // Clip/cull limits
            max_clip_distances: 8,
            max_cull_distances: 8,
            max_combined_clip_and_cull_distances: 8,

            // Point/line limits
            point_size_range: [1.0, 256.0],
            line_width_range: [1.0, 8.0],
            point_size_granularity: 1.0,
            line_width_granularity: 1.0,

            strict_lines: true,
            standard_sample_locations: true,

            // Buffer copy limits
            optimal_buffer_copy_offset_alignment: 64,
            optimal_buffer_copy_row_pitch_alignment: 64,
        }
    }
}

/// Sample count flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SampleCountFlags(pub u32);

impl SampleCountFlags {
    /// 1 sample
    pub const SAMPLE_1: Self = Self(1 << 0);
    /// 2 samples
    pub const SAMPLE_2: Self = Self(1 << 1);
    /// 4 samples
    pub const SAMPLE_4: Self = Self(1 << 2);
    /// 8 samples
    pub const SAMPLE_8: Self = Self(1 << 3);
    /// 16 samples
    pub const SAMPLE_16: Self = Self(1 << 4);
    /// 32 samples
    pub const SAMPLE_32: Self = Self(1 << 5);
    /// 64 samples
    pub const SAMPLE_64: Self = Self(1 << 6);

    /// All sample counts
    pub const ALL: Self = Self(0x7F);

    /// Checks if flag is set
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Maximum sample count
    #[inline]
    pub const fn max_samples(&self) -> u32 {
        if self.0 & Self::SAMPLE_64.0 != 0 {
            64
        } else if self.0 & Self::SAMPLE_32.0 != 0 {
            32
        } else if self.0 & Self::SAMPLE_16.0 != 0 {
            16
        } else if self.0 & Self::SAMPLE_8.0 != 0 {
            8
        } else if self.0 & Self::SAMPLE_4.0 != 0 {
            4
        } else if self.0 & Self::SAMPLE_2.0 != 0 {
            2
        } else {
            1
        }
    }
}

// ============================================================================
// Sparse Properties
// ============================================================================

/// Sparse resource properties
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SparseProperties {
    /// Residency for standard 2D block shape
    pub residency_standard_2d_block_shape: bool,
    /// Residency for standard 2D multisample block shape
    pub residency_standard_2d_multisample_block_shape: bool,
    /// Residency for standard 3D block shape
    pub residency_standard_3d_block_shape: bool,
    /// Residency aligned mip size
    pub residency_aligned_mip_size: bool,
    /// Residency non-resident strict
    pub residency_non_resident_strict: bool,
}

// ============================================================================
// Device Features
// ============================================================================

/// Device feature set
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DeviceFeatures {
    // Core features
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

    // Extended features
    /// Timeline semaphores
    pub timeline_semaphore: bool,
    /// Buffer device address
    pub buffer_device_address: bool,
    /// Descriptor indexing
    pub descriptor_indexing: bool,
    /// Dynamic rendering
    pub dynamic_rendering: bool,
    /// Synchronization2
    pub synchronization2: bool,
    /// Maintenance4
    pub maintenance4: bool,
    /// Ray tracing pipeline
    pub ray_tracing_pipeline: bool,
    /// Ray query
    pub ray_query: bool,
    /// Acceleration structure
    pub acceleration_structure: bool,
    /// Mesh shading
    pub mesh_shader: bool,
    /// Task shading
    pub task_shader: bool,
    /// Fragment shading rate
    pub fragment_shading_rate: bool,
    /// Shader int8
    pub shader_int8: bool,
    /// Shader float16
    pub shader_float16: bool,
    /// 16-bit storage
    pub storage_16bit: bool,
    /// 8-bit storage
    pub storage_8bit: bool,
    /// Multiview
    pub multiview: bool,
    /// Variable pointers
    pub variable_pointers: bool,
    /// Subgroup operations
    pub subgroup_operations: bool,
}

impl DeviceFeatures {
    /// All features enabled
    #[inline]
    pub fn all() -> Self {
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
            texture_compression_etc2: true,
            texture_compression_astc_ldr: true,
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
            timeline_semaphore: true,
            buffer_device_address: true,
            descriptor_indexing: true,
            dynamic_rendering: true,
            synchronization2: true,
            maintenance4: true,
            ray_tracing_pipeline: true,
            ray_query: true,
            acceleration_structure: true,
            mesh_shader: true,
            task_shader: true,
            fragment_shading_rate: true,
            shader_int8: true,
            shader_float16: true,
            storage_16bit: true,
            storage_8bit: true,
            multiview: true,
            variable_pointers: true,
            subgroup_operations: true,
        }
    }

    /// Core features only
    #[inline]
    pub fn core() -> Self {
        let mut features = Self::default();
        features.robust_buffer_access = true;
        features.full_draw_index_uint32 = true;
        features.image_cube_array = true;
        features.independent_blend = true;
        features.multi_draw_indirect = true;
        features.sampler_anisotropy = true;
        features.texture_compression_bc = true;
        features.occlusion_query_precise = true;
        features.pipeline_statistics_query = true;
        features.shader_clip_distance = true;
        features
    }
}
