//! Resource Limits for Lumina
//!
//! This module provides GPU resource limits, validation, and constraint types
//! for ensuring resources stay within hardware capabilities.

// ============================================================================
// Physical Device Limits
// ============================================================================

/// Physical device limits
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PhysicalDeviceLimits {
    // ========================================================================
    // Image Limits
    // ========================================================================
    /// Maximum 1D image dimension
    pub max_image_dimension_1d: u32,
    /// Maximum 2D image dimension
    pub max_image_dimension_2d: u32,
    /// Maximum 3D image dimension
    pub max_image_dimension_3d: u32,
    /// Maximum cube image dimension
    pub max_image_dimension_cube: u32,
    /// Maximum image array layers
    pub max_image_array_layers: u32,

    // ========================================================================
    // Buffer Limits
    // ========================================================================
    /// Maximum texel buffer elements
    pub max_texel_buffer_elements: u32,
    /// Maximum uniform buffer range
    pub max_uniform_buffer_range: u32,
    /// Maximum storage buffer range
    pub max_storage_buffer_range: u32,

    // ========================================================================
    // Push Constant Limits
    // ========================================================================
    /// Maximum push constants size
    pub max_push_constants_size: u32,

    // ========================================================================
    // Memory Limits
    // ========================================================================
    /// Maximum memory allocation count
    pub max_memory_allocation_count: u32,
    /// Maximum sampler allocation count
    pub max_sampler_allocation_count: u32,
    /// Buffer image granularity
    pub buffer_image_granularity: u64,
    /// Sparse address space size
    pub sparse_address_space_size: u64,

    // ========================================================================
    // Descriptor Limits
    // ========================================================================
    /// Maximum bound descriptor sets
    pub max_bound_descriptor_sets: u32,
    /// Maximum per-stage descriptor samplers
    pub max_per_stage_descriptor_samplers: u32,
    /// Maximum per-stage descriptor uniform buffers
    pub max_per_stage_descriptor_uniform_buffers: u32,
    /// Maximum per-stage descriptor storage buffers
    pub max_per_stage_descriptor_storage_buffers: u32,
    /// Maximum per-stage descriptor sampled images
    pub max_per_stage_descriptor_sampled_images: u32,
    /// Maximum per-stage descriptor storage images
    pub max_per_stage_descriptor_storage_images: u32,
    /// Maximum per-stage descriptor input attachments
    pub max_per_stage_descriptor_input_attachments: u32,
    /// Maximum per-stage resources
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

    // ========================================================================
    // Vertex Limits
    // ========================================================================
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

    // ========================================================================
    // Tessellation Limits
    // ========================================================================
    /// Maximum tessellation generation level
    pub max_tessellation_generation_level: u32,
    /// Maximum tessellation patch size
    pub max_tessellation_patch_size: u32,
    /// Maximum tessellation control per-vertex input components
    pub max_tessellation_control_per_vertex_input_components: u32,
    /// Maximum tessellation control per-vertex output components
    pub max_tessellation_control_per_vertex_output_components: u32,
    /// Maximum tessellation control per-patch output components
    pub max_tessellation_control_per_patch_output_components: u32,
    /// Maximum tessellation control total output components
    pub max_tessellation_control_total_output_components: u32,
    /// Maximum tessellation evaluation input components
    pub max_tessellation_evaluation_input_components: u32,
    /// Maximum tessellation evaluation output components
    pub max_tessellation_evaluation_output_components: u32,

    // ========================================================================
    // Geometry Limits
    // ========================================================================
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

    // ========================================================================
    // Fragment Limits
    // ========================================================================
    /// Maximum fragment input components
    pub max_fragment_input_components: u32,
    /// Maximum fragment output attachments
    pub max_fragment_output_attachments: u32,
    /// Maximum fragment dual src attachments
    pub max_fragment_dual_src_attachments: u32,
    /// Maximum fragment combined output resources
    pub max_fragment_combined_output_resources: u32,

    // ========================================================================
    // Compute Limits
    // ========================================================================
    /// Maximum compute shared memory size
    pub max_compute_shared_memory_size: u32,
    /// Maximum compute work group count
    pub max_compute_work_group_count: [u32; 3],
    /// Maximum compute work group invocations
    pub max_compute_work_group_invocations: u32,
    /// Maximum compute work group size
    pub max_compute_work_group_size: [u32; 3],

    // ========================================================================
    // Subpixel/Subgroup Limits
    // ========================================================================
    /// Sub-pixel precision bits
    pub sub_pixel_precision_bits: u32,
    /// Sub-texel precision bits
    pub sub_texel_precision_bits: u32,
    /// Mipmap precision bits
    pub mipmap_precision_bits: u32,

    // ========================================================================
    // Draw Limits
    // ========================================================================
    /// Maximum draw indexed index value
    pub max_draw_indexed_index_value: u32,
    /// Maximum draw indirect count
    pub max_draw_indirect_count: u32,

    // ========================================================================
    // Sampler Limits
    // ========================================================================
    /// Maximum sampler LOD bias
    pub max_sampler_lod_bias: f32,
    /// Maximum sampler anisotropy
    pub max_sampler_anisotropy: f32,

    // ========================================================================
    // Viewport Limits
    // ========================================================================
    /// Maximum viewports
    pub max_viewports: u32,
    /// Maximum viewport dimensions
    pub max_viewport_dimensions: [u32; 2],
    /// Viewport bounds range
    pub viewport_bounds_range: [f32; 2],
    /// Viewport sub-pixel bits
    pub viewport_sub_pixel_bits: u32,

    // ========================================================================
    // Memory Alignment
    // ========================================================================
    /// Minimum memory map alignment
    pub min_memory_map_alignment: u64,
    /// Minimum texel buffer offset alignment
    pub min_texel_buffer_offset_alignment: u64,
    /// Minimum uniform buffer offset alignment
    pub min_uniform_buffer_offset_alignment: u64,
    /// Minimum storage buffer offset alignment
    pub min_storage_buffer_offset_alignment: u64,

    // ========================================================================
    // Texel Offset Limits
    // ========================================================================
    /// Minimum texel offset
    pub min_texel_offset: i32,
    /// Maximum texel offset
    pub max_texel_offset: u32,
    /// Minimum texel gather offset
    pub min_texel_gather_offset: i32,
    /// Maximum texel gather offset
    pub max_texel_gather_offset: u32,

    // ========================================================================
    // Interpolation Limits
    // ========================================================================
    /// Minimum interpolation offset
    pub min_interpolation_offset: f32,
    /// Maximum interpolation offset
    pub max_interpolation_offset: f32,
    /// Sub-pixel interpolation offset bits
    pub sub_pixel_interpolation_offset_bits: u32,

    // ========================================================================
    // Framebuffer Limits
    // ========================================================================
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
    /// Framebuffer no-attachments sample counts
    pub framebuffer_no_attachments_sample_counts: SampleCountFlags,

    // ========================================================================
    // Color Attachment Limits
    // ========================================================================
    /// Maximum color attachments
    pub max_color_attachments: u32,

    // ========================================================================
    // Sample Limits
    // ========================================================================
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

    // ========================================================================
    // Timestamp Limits
    // ========================================================================
    /// Timestamp compute and graphics
    pub timestamp_compute_and_graphics: bool,
    /// Timestamp period
    pub timestamp_period: f32,

    // ========================================================================
    // Clip/Cull Limits
    // ========================================================================
    /// Maximum clip distances
    pub max_clip_distances: u32,
    /// Maximum cull distances
    pub max_cull_distances: u32,
    /// Maximum combined clip and cull distances
    pub max_combined_clip_and_cull_distances: u32,

    // ========================================================================
    // Point Size Limits
    // ========================================================================
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

    // ========================================================================
    // Rasterization Limits
    // ========================================================================
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

impl PhysicalDeviceLimits {
    /// Default limits (conservative baseline)
    pub const DEFAULT: Self = Self {
        max_image_dimension_1d: 4096,
        max_image_dimension_2d: 4096,
        max_image_dimension_3d: 256,
        max_image_dimension_cube: 4096,
        max_image_array_layers: 256,
        max_texel_buffer_elements: 65536,
        max_uniform_buffer_range: 16384,
        max_storage_buffer_range: 1 << 27,
        max_push_constants_size: 128,
        max_memory_allocation_count: 4096,
        max_sampler_allocation_count: 4000,
        buffer_image_granularity: 131072,
        sparse_address_space_size: 1 << 40,
        max_bound_descriptor_sets: 4,
        max_per_stage_descriptor_samplers: 16,
        max_per_stage_descriptor_uniform_buffers: 12,
        max_per_stage_descriptor_storage_buffers: 4,
        max_per_stage_descriptor_sampled_images: 16,
        max_per_stage_descriptor_storage_images: 4,
        max_per_stage_descriptor_input_attachments: 4,
        max_per_stage_resources: 128,
        max_descriptor_set_samplers: 96,
        max_descriptor_set_uniform_buffers: 72,
        max_descriptor_set_uniform_buffers_dynamic: 8,
        max_descriptor_set_storage_buffers: 24,
        max_descriptor_set_storage_buffers_dynamic: 4,
        max_descriptor_set_sampled_images: 96,
        max_descriptor_set_storage_images: 24,
        max_descriptor_set_input_attachments: 4,
        max_vertex_input_attributes: 16,
        max_vertex_input_bindings: 16,
        max_vertex_input_attribute_offset: 2047,
        max_vertex_input_binding_stride: 2048,
        max_vertex_output_components: 64,
        max_tessellation_generation_level: 64,
        max_tessellation_patch_size: 32,
        max_tessellation_control_per_vertex_input_components: 64,
        max_tessellation_control_per_vertex_output_components: 64,
        max_tessellation_control_per_patch_output_components: 120,
        max_tessellation_control_total_output_components: 2048,
        max_tessellation_evaluation_input_components: 64,
        max_tessellation_evaluation_output_components: 64,
        max_geometry_shader_invocations: 32,
        max_geometry_input_components: 64,
        max_geometry_output_components: 64,
        max_geometry_output_vertices: 256,
        max_geometry_total_output_components: 1024,
        max_fragment_input_components: 64,
        max_fragment_output_attachments: 4,
        max_fragment_dual_src_attachments: 1,
        max_fragment_combined_output_resources: 4,
        max_compute_shared_memory_size: 16384,
        max_compute_work_group_count: [65535, 65535, 65535],
        max_compute_work_group_invocations: 128,
        max_compute_work_group_size: [128, 128, 64],
        sub_pixel_precision_bits: 4,
        sub_texel_precision_bits: 4,
        mipmap_precision_bits: 4,
        max_draw_indexed_index_value: u32::MAX - 1,
        max_draw_indirect_count: u32::MAX - 1,
        max_sampler_lod_bias: 2.0,
        max_sampler_anisotropy: 16.0,
        max_viewports: 16,
        max_viewport_dimensions: [4096, 4096],
        viewport_bounds_range: [-8192.0, 8191.0],
        viewport_sub_pixel_bits: 0,
        min_memory_map_alignment: 64,
        min_texel_buffer_offset_alignment: 256,
        min_uniform_buffer_offset_alignment: 256,
        min_storage_buffer_offset_alignment: 256,
        min_texel_offset: -8,
        max_texel_offset: 7,
        min_texel_gather_offset: -8,
        max_texel_gather_offset: 7,
        min_interpolation_offset: -0.5,
        max_interpolation_offset: 0.5,
        sub_pixel_interpolation_offset_bits: 4,
        max_framebuffer_width: 4096,
        max_framebuffer_height: 4096,
        max_framebuffer_layers: 256,
        framebuffer_color_sample_counts: SampleCountFlags::S1_S4,
        framebuffer_depth_sample_counts: SampleCountFlags::S1_S4,
        framebuffer_stencil_sample_counts: SampleCountFlags::S1_S4,
        framebuffer_no_attachments_sample_counts: SampleCountFlags::S1_S4,
        max_color_attachments: 4,
        sampled_image_color_sample_counts: SampleCountFlags::S1_S4,
        sampled_image_integer_sample_counts: SampleCountFlags::S1,
        sampled_image_depth_sample_counts: SampleCountFlags::S1_S4,
        sampled_image_stencil_sample_counts: SampleCountFlags::S1_S4,
        storage_image_sample_counts: SampleCountFlags::S1,
        max_sample_mask_words: 1,
        timestamp_compute_and_graphics: true,
        timestamp_period: 1.0,
        max_clip_distances: 8,
        max_cull_distances: 8,
        max_combined_clip_and_cull_distances: 8,
        discrete_queue_priorities: 2,
        point_size_range: [1.0, 64.0],
        line_width_range: [1.0, 1.0],
        point_size_granularity: 1.0,
        line_width_granularity: 1.0,
        strict_lines: false,
        standard_sample_locations: true,
        optimal_buffer_copy_offset_alignment: 256,
        optimal_buffer_copy_row_pitch_alignment: 256,
        non_coherent_atom_size: 256,
    };

    /// NVIDIA-like limits
    pub const NVIDIA_LIKE: Self = Self {
        max_image_dimension_1d: 32768,
        max_image_dimension_2d: 32768,
        max_image_dimension_3d: 16384,
        max_image_dimension_cube: 32768,
        max_image_array_layers: 2048,
        max_texel_buffer_elements: 1 << 27,
        max_uniform_buffer_range: 65536,
        max_storage_buffer_range: 1 << 30,
        max_push_constants_size: 256,
        max_memory_allocation_count: 1 << 20,
        max_sampler_allocation_count: 1 << 20,
        buffer_image_granularity: 1,
        sparse_address_space_size: 1 << 48,
        max_bound_descriptor_sets: 32,
        max_per_stage_descriptor_samplers: 1 << 20,
        max_per_stage_descriptor_uniform_buffers: 1 << 20,
        max_per_stage_descriptor_storage_buffers: 1 << 20,
        max_per_stage_descriptor_sampled_images: 1 << 20,
        max_per_stage_descriptor_storage_images: 1 << 20,
        max_per_stage_descriptor_input_attachments: 1 << 20,
        max_per_stage_resources: 1 << 20,
        max_descriptor_set_samplers: 1 << 20,
        max_descriptor_set_uniform_buffers: 1 << 20,
        max_descriptor_set_uniform_buffers_dynamic: 15,
        max_descriptor_set_storage_buffers: 1 << 20,
        max_descriptor_set_storage_buffers_dynamic: 16,
        max_descriptor_set_sampled_images: 1 << 20,
        max_descriptor_set_storage_images: 1 << 20,
        max_descriptor_set_input_attachments: 1 << 20,
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
        max_tessellation_control_total_output_components: 4216,
        max_tessellation_evaluation_input_components: 128,
        max_tessellation_evaluation_output_components: 128,
        max_geometry_shader_invocations: 32,
        max_geometry_input_components: 128,
        max_geometry_output_components: 128,
        max_geometry_output_vertices: 1024,
        max_geometry_total_output_components: 1024,
        max_fragment_input_components: 128,
        max_fragment_output_attachments: 8,
        max_fragment_dual_src_attachments: 1,
        max_fragment_combined_output_resources: 16,
        max_compute_shared_memory_size: 49152,
        max_compute_work_group_count: [2147483647, 65535, 65535],
        max_compute_work_group_invocations: 1024,
        max_compute_work_group_size: [1024, 1024, 64],
        sub_pixel_precision_bits: 8,
        sub_texel_precision_bits: 8,
        mipmap_precision_bits: 8,
        max_draw_indexed_index_value: u32::MAX - 1,
        max_draw_indirect_count: u32::MAX - 1,
        max_sampler_lod_bias: 15.0,
        max_sampler_anisotropy: 16.0,
        max_viewports: 16,
        max_viewport_dimensions: [32768, 32768],
        viewport_bounds_range: [-65536.0, 65536.0],
        viewport_sub_pixel_bits: 8,
        min_memory_map_alignment: 64,
        min_texel_buffer_offset_alignment: 16,
        min_uniform_buffer_offset_alignment: 64,
        min_storage_buffer_offset_alignment: 16,
        min_texel_offset: -8,
        max_texel_offset: 7,
        min_texel_gather_offset: -32,
        max_texel_gather_offset: 31,
        min_interpolation_offset: -0.5,
        max_interpolation_offset: 0.4375,
        sub_pixel_interpolation_offset_bits: 4,
        max_framebuffer_width: 32768,
        max_framebuffer_height: 32768,
        max_framebuffer_layers: 2048,
        framebuffer_color_sample_counts: SampleCountFlags::ALL,
        framebuffer_depth_sample_counts: SampleCountFlags::ALL,
        framebuffer_stencil_sample_counts: SampleCountFlags::S1_S8,
        framebuffer_no_attachments_sample_counts: SampleCountFlags::ALL,
        max_color_attachments: 8,
        sampled_image_color_sample_counts: SampleCountFlags::ALL,
        sampled_image_integer_sample_counts: SampleCountFlags::ALL,
        sampled_image_depth_sample_counts: SampleCountFlags::ALL,
        sampled_image_stencil_sample_counts: SampleCountFlags::S1_S8,
        storage_image_sample_counts: SampleCountFlags::ALL,
        max_sample_mask_words: 1,
        timestamp_compute_and_graphics: true,
        timestamp_period: 1.0,
        max_clip_distances: 8,
        max_cull_distances: 8,
        max_combined_clip_and_cull_distances: 8,
        discrete_queue_priorities: 2,
        point_size_range: [1.0, 2047.9375],
        line_width_range: [1.0, 64.0],
        point_size_granularity: 0.0625,
        line_width_granularity: 0.0625,
        strict_lines: true,
        standard_sample_locations: true,
        optimal_buffer_copy_offset_alignment: 1,
        optimal_buffer_copy_row_pitch_alignment: 1,
        non_coherent_atom_size: 64,
    };

    /// AMD-like limits
    pub const AMD_LIKE: Self = Self {
        max_compute_shared_memory_size: 65536,
        max_compute_work_group_invocations: 1024,
        max_compute_work_group_size: [1024, 1024, 1024],
        ..Self::NVIDIA_LIKE
    };
}

impl Default for PhysicalDeviceLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

// ============================================================================
// Sample Count Flags
// ============================================================================

/// Sample count flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SampleCountFlags(pub u32);

impl SampleCountFlags {
    /// No samples
    pub const NONE: Self = Self(0);
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
    /// 1-4 samples
    pub const S1_S4: Self = Self(0b0111);
    /// 1-8 samples
    pub const S1_S8: Self = Self(0b1111);
    /// All samples
    pub const ALL: Self = Self(0b1111111);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Max sample count
    #[inline]
    pub const fn max_samples(&self) -> u32 {
        if self.0 & Self::S64.0 != 0 {
            64
        } else if self.0 & Self::S32.0 != 0 {
            32
        } else if self.0 & Self::S16.0 != 0 {
            16
        } else if self.0 & Self::S8.0 != 0 {
            8
        } else if self.0 & Self::S4.0 != 0 {
            4
        } else if self.0 & Self::S2.0 != 0 {
            2
        } else {
            1
        }
    }
}

// ============================================================================
// Resource Validation
// ============================================================================

/// Resource validation result
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceValidationResult {
    /// Valid
    Valid,
    /// Exceeds image dimension limit
    ImageDimensionExceeded,
    /// Exceeds image array layers limit
    ImageArrayLayersExceeded,
    /// Exceeds buffer size limit
    BufferSizeExceeded,
    /// Exceeds memory allocation limit
    MemoryAllocationExceeded,
    /// Exceeds descriptor limit
    DescriptorLimitExceeded,
    /// Exceeds vertex input limit
    VertexInputLimitExceeded,
    /// Exceeds attachment limit
    AttachmentLimitExceeded,
    /// Exceeds push constant limit
    PushConstantLimitExceeded,
    /// Exceeds compute workgroup limit
    ComputeWorkgroupLimitExceeded,
    /// Exceeds viewport limit
    ViewportLimitExceeded,
    /// Invalid alignment
    InvalidAlignment,
    /// Sample count not supported
    SampleCountNotSupported,
}

impl ResourceValidationResult {
    /// Is valid
    #[inline]
    pub const fn is_valid(&self) -> bool {
        matches!(self, Self::Valid)
    }

    /// Error message
    #[inline]
    pub const fn message(&self) -> &'static str {
        match self {
            Self::Valid => "Resource is valid",
            Self::ImageDimensionExceeded => "Image dimension exceeds device limits",
            Self::ImageArrayLayersExceeded => "Image array layers exceed device limits",
            Self::BufferSizeExceeded => "Buffer size exceeds device limits",
            Self::MemoryAllocationExceeded => "Memory allocation count exceeded",
            Self::DescriptorLimitExceeded => "Descriptor limit exceeded",
            Self::VertexInputLimitExceeded => "Vertex input limit exceeded",
            Self::AttachmentLimitExceeded => "Attachment limit exceeded",
            Self::PushConstantLimitExceeded => "Push constant size limit exceeded",
            Self::ComputeWorkgroupLimitExceeded => "Compute workgroup limit exceeded",
            Self::ViewportLimitExceeded => "Viewport limit exceeded",
            Self::InvalidAlignment => "Invalid memory alignment",
            Self::SampleCountNotSupported => "Sample count not supported",
        }
    }
}

/// Resource validator
#[derive(Clone, Copy, Debug)]
pub struct ResourceValidator {
    limits: PhysicalDeviceLimits,
}

impl ResourceValidator {
    /// Creates new validator
    #[inline]
    pub const fn new(limits: PhysicalDeviceLimits) -> Self {
        Self { limits }
    }

    /// Validate 2D image dimensions
    #[inline]
    pub const fn validate_image_2d(&self, width: u32, height: u32) -> ResourceValidationResult {
        if width > self.limits.max_image_dimension_2d || height > self.limits.max_image_dimension_2d
        {
            ResourceValidationResult::ImageDimensionExceeded
        } else {
            ResourceValidationResult::Valid
        }
    }

    /// Validate 3D image dimensions
    #[inline]
    pub const fn validate_image_3d(
        &self,
        width: u32,
        height: u32,
        depth: u32,
    ) -> ResourceValidationResult {
        if width > self.limits.max_image_dimension_3d
            || height > self.limits.max_image_dimension_3d
            || depth > self.limits.max_image_dimension_3d
        {
            ResourceValidationResult::ImageDimensionExceeded
        } else {
            ResourceValidationResult::Valid
        }
    }

    /// Validate image array layers
    #[inline]
    pub const fn validate_array_layers(&self, layers: u32) -> ResourceValidationResult {
        if layers > self.limits.max_image_array_layers {
            ResourceValidationResult::ImageArrayLayersExceeded
        } else {
            ResourceValidationResult::Valid
        }
    }

    /// Validate uniform buffer size
    #[inline]
    pub const fn validate_uniform_buffer(&self, size: u32) -> ResourceValidationResult {
        if size > self.limits.max_uniform_buffer_range {
            ResourceValidationResult::BufferSizeExceeded
        } else {
            ResourceValidationResult::Valid
        }
    }

    /// Validate storage buffer size
    #[inline]
    pub const fn validate_storage_buffer(&self, size: u32) -> ResourceValidationResult {
        if size > self.limits.max_storage_buffer_range {
            ResourceValidationResult::BufferSizeExceeded
        } else {
            ResourceValidationResult::Valid
        }
    }

    /// Validate push constants size
    #[inline]
    pub const fn validate_push_constants(&self, size: u32) -> ResourceValidationResult {
        if size > self.limits.max_push_constants_size {
            ResourceValidationResult::PushConstantLimitExceeded
        } else {
            ResourceValidationResult::Valid
        }
    }

    /// Validate vertex input attributes
    #[inline]
    pub const fn validate_vertex_attributes(&self, count: u32) -> ResourceValidationResult {
        if count > self.limits.max_vertex_input_attributes {
            ResourceValidationResult::VertexInputLimitExceeded
        } else {
            ResourceValidationResult::Valid
        }
    }

    /// Validate color attachments
    #[inline]
    pub const fn validate_color_attachments(&self, count: u32) -> ResourceValidationResult {
        if count > self.limits.max_color_attachments {
            ResourceValidationResult::AttachmentLimitExceeded
        } else {
            ResourceValidationResult::Valid
        }
    }

    /// Validate compute workgroup
    #[inline]
    pub const fn validate_compute_workgroup(
        &self,
        x: u32,
        y: u32,
        z: u32,
    ) -> ResourceValidationResult {
        if x > self.limits.max_compute_work_group_size[0]
            || y > self.limits.max_compute_work_group_size[1]
            || z > self.limits.max_compute_work_group_size[2]
        {
            return ResourceValidationResult::ComputeWorkgroupLimitExceeded;
        }

        if x * y * z > self.limits.max_compute_work_group_invocations {
            return ResourceValidationResult::ComputeWorkgroupLimitExceeded;
        }

        ResourceValidationResult::Valid
    }

    /// Validate compute dispatch
    #[inline]
    pub const fn validate_compute_dispatch(
        &self,
        groups_x: u32,
        groups_y: u32,
        groups_z: u32,
    ) -> ResourceValidationResult {
        if groups_x > self.limits.max_compute_work_group_count[0]
            || groups_y > self.limits.max_compute_work_group_count[1]
            || groups_z > self.limits.max_compute_work_group_count[2]
        {
            ResourceValidationResult::ComputeWorkgroupLimitExceeded
        } else {
            ResourceValidationResult::Valid
        }
    }

    /// Validate viewports
    #[inline]
    pub const fn validate_viewports(&self, count: u32) -> ResourceValidationResult {
        if count > self.limits.max_viewports {
            ResourceValidationResult::ViewportLimitExceeded
        } else {
            ResourceValidationResult::Valid
        }
    }

    /// Validate framebuffer dimensions
    #[inline]
    pub const fn validate_framebuffer(
        &self,
        width: u32,
        height: u32,
        layers: u32,
    ) -> ResourceValidationResult {
        if width > self.limits.max_framebuffer_width {
            return ResourceValidationResult::ImageDimensionExceeded;
        }
        if height > self.limits.max_framebuffer_height {
            return ResourceValidationResult::ImageDimensionExceeded;
        }
        if layers > self.limits.max_framebuffer_layers {
            return ResourceValidationResult::ImageArrayLayersExceeded;
        }
        ResourceValidationResult::Valid
    }

    /// Validate uniform buffer alignment
    #[inline]
    pub const fn validate_uniform_buffer_alignment(&self, offset: u64) -> ResourceValidationResult {
        if offset % self.limits.min_uniform_buffer_offset_alignment != 0 {
            ResourceValidationResult::InvalidAlignment
        } else {
            ResourceValidationResult::Valid
        }
    }

    /// Validate storage buffer alignment
    #[inline]
    pub const fn validate_storage_buffer_alignment(&self, offset: u64) -> ResourceValidationResult {
        if offset % self.limits.min_storage_buffer_offset_alignment != 0 {
            ResourceValidationResult::InvalidAlignment
        } else {
            ResourceValidationResult::Valid
        }
    }

    /// Get limits
    #[inline]
    pub const fn limits(&self) -> &PhysicalDeviceLimits {
        &self.limits
    }
}

impl Default for ResourceValidator {
    fn default() -> Self {
        Self::new(PhysicalDeviceLimits::DEFAULT)
    }
}

// ============================================================================
// Allocation Limits Tracker
// ============================================================================

/// Allocation limits tracker
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AllocationTracker {
    /// Current memory allocations
    pub memory_allocations: u32,
    /// Maximum memory allocations
    pub max_memory_allocations: u32,
    /// Current sampler allocations
    pub sampler_allocations: u32,
    /// Maximum sampler allocations
    pub max_sampler_allocations: u32,
    /// Current descriptor set allocations
    pub descriptor_set_allocations: u32,
    /// Maximum descriptor set allocations
    pub max_descriptor_set_allocations: u32,
}

impl AllocationTracker {
    /// Creates new tracker
    #[inline]
    pub const fn new(limits: &PhysicalDeviceLimits) -> Self {
        Self {
            memory_allocations: 0,
            max_memory_allocations: limits.max_memory_allocation_count,
            sampler_allocations: 0,
            max_sampler_allocations: limits.max_sampler_allocation_count,
            descriptor_set_allocations: 0,
            max_descriptor_set_allocations: 4096, // Conservative default
        }
    }

    /// Can allocate memory
    #[inline]
    pub const fn can_allocate_memory(&self) -> bool {
        self.memory_allocations < self.max_memory_allocations
    }

    /// Can allocate sampler
    #[inline]
    pub const fn can_allocate_sampler(&self) -> bool {
        self.sampler_allocations < self.max_sampler_allocations
    }

    /// Can allocate descriptor set
    #[inline]
    pub const fn can_allocate_descriptor_set(&self) -> bool {
        self.descriptor_set_allocations < self.max_descriptor_set_allocations
    }

    /// Memory usage percentage
    #[inline]
    pub fn memory_usage_percent(&self) -> f32 {
        if self.max_memory_allocations == 0 {
            0.0
        } else {
            self.memory_allocations as f32 / self.max_memory_allocations as f32 * 100.0
        }
    }
}

// ============================================================================
// Subgroup Limits
// ============================================================================

/// Subgroup limits
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SubgroupLimits {
    /// Subgroup size
    pub subgroup_size: u32,
    /// Minimum subgroup size (variable subgroups)
    pub min_subgroup_size: u32,
    /// Maximum subgroup size (variable subgroups)
    pub max_subgroup_size: u32,
    /// Supported stages
    pub supported_stages: u32,
    /// Supported operations
    pub supported_operations: SubgroupFeatureFlags,
    /// Quad operations in all stages
    pub quad_operations_in_all_stages: bool,
}

impl SubgroupLimits {
    /// Default NVIDIA-like
    pub const NVIDIA_LIKE: Self = Self {
        subgroup_size: 32,
        min_subgroup_size: 32,
        max_subgroup_size: 32,
        supported_stages: 0x7F, // All stages
        supported_operations: SubgroupFeatureFlags::ALL,
        quad_operations_in_all_stages: true,
    };

    /// AMD-like
    pub const AMD_LIKE: Self = Self {
        subgroup_size: 64,
        min_subgroup_size: 32,
        max_subgroup_size: 64,
        supported_stages: 0x7F,
        supported_operations: SubgroupFeatureFlags::ALL,
        quad_operations_in_all_stages: true,
    };

    /// Intel-like
    pub const INTEL_LIKE: Self = Self {
        subgroup_size: 32,
        min_subgroup_size: 8,
        max_subgroup_size: 32,
        supported_stages: 0x7F,
        supported_operations: SubgroupFeatureFlags::BASIC,
        quad_operations_in_all_stages: false,
    };
}

impl Default for SubgroupLimits {
    fn default() -> Self {
        Self::NVIDIA_LIKE
    }
}

/// Subgroup feature flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SubgroupFeatureFlags(pub u32);

impl SubgroupFeatureFlags {
    /// No features
    pub const NONE: Self = Self(0);
    /// Basic subgroup operations
    pub const BASIC: Self = Self(1 << 0);
    /// Vote operations
    pub const VOTE: Self = Self(1 << 1);
    /// Arithmetic operations
    pub const ARITHMETIC: Self = Self(1 << 2);
    /// Ballot operations
    pub const BALLOT: Self = Self(1 << 3);
    /// Shuffle operations
    pub const SHUFFLE: Self = Self(1 << 4);
    /// Shuffle relative operations
    pub const SHUFFLE_RELATIVE: Self = Self(1 << 5);
    /// Clustered operations
    pub const CLUSTERED: Self = Self(1 << 6);
    /// Quad operations
    pub const QUAD: Self = Self(1 << 7);
    /// Partitioned operations (NV)
    pub const PARTITIONED: Self = Self(1 << 8);
    /// All features
    pub const ALL: Self = Self(0x1FF);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Mesh Shader Limits
// ============================================================================

/// Mesh shader limits
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MeshShaderLimits {
    /// Maximum task work group total count
    pub max_task_work_group_total_count: u32,
    /// Maximum task work group count
    pub max_task_work_group_count: [u32; 3],
    /// Maximum task work group invocations
    pub max_task_work_group_invocations: u32,
    /// Maximum task work group size
    pub max_task_work_group_size: [u32; 3],
    /// Maximum task payload size
    pub max_task_payload_size: u32,
    /// Maximum task shared memory size
    pub max_task_shared_memory_size: u32,
    /// Maximum task payload and shared memory size
    pub max_task_payload_and_shared_memory_size: u32,
    /// Maximum mesh work group total count
    pub max_mesh_work_group_total_count: u32,
    /// Maximum mesh work group count
    pub max_mesh_work_group_count: [u32; 3],
    /// Maximum mesh work group invocations
    pub max_mesh_work_group_invocations: u32,
    /// Maximum mesh work group size
    pub max_mesh_work_group_size: [u32; 3],
    /// Maximum mesh shared memory size
    pub max_mesh_shared_memory_size: u32,
    /// Maximum mesh output memory size
    pub max_mesh_output_memory_size: u32,
    /// Maximum mesh payload and output memory size
    pub max_mesh_payload_and_output_memory_size: u32,
    /// Maximum mesh output components
    pub max_mesh_output_components: u32,
    /// Maximum mesh output vertices
    pub max_mesh_output_vertices: u32,
    /// Maximum mesh output primitives
    pub max_mesh_output_primitives: u32,
    /// Maximum mesh output layers
    pub max_mesh_output_layers: u32,
    /// Maximum mesh multiview view count
    pub max_mesh_multiview_view_count: u32,
    /// Mesh output per vertex granularity
    pub mesh_output_per_vertex_granularity: u32,
    /// Mesh output per primitive granularity
    pub mesh_output_per_primitive_granularity: u32,
    /// Maximum preferred task work group invocations
    pub max_preferred_task_work_group_invocations: u32,
    /// Maximum preferred mesh work group invocations
    pub max_preferred_mesh_work_group_invocations: u32,
    /// Prefers local invocation vertex output
    pub prefers_local_invocation_vertex_output: bool,
    /// Prefers local invocation primitive output
    pub prefers_local_invocation_primitive_output: bool,
    /// Prefers compact vertex output
    pub prefers_compact_vertex_output: bool,
    /// Prefers compact primitive output
    pub prefers_compact_primitive_output: bool,
}

impl MeshShaderLimits {
    /// NVIDIA-like limits
    pub const NVIDIA_LIKE: Self = Self {
        max_task_work_group_total_count: 4194304,
        max_task_work_group_count: [65535, 65535, 65535],
        max_task_work_group_invocations: 1024,
        max_task_work_group_size: [1024, 1024, 1024],
        max_task_payload_size: 16384,
        max_task_shared_memory_size: 32768,
        max_task_payload_and_shared_memory_size: 32768,
        max_mesh_work_group_total_count: 4194304,
        max_mesh_work_group_count: [65535, 65535, 65535],
        max_mesh_work_group_invocations: 1024,
        max_mesh_work_group_size: [1024, 1024, 1024],
        max_mesh_shared_memory_size: 28672,
        max_mesh_output_memory_size: 32768,
        max_mesh_payload_and_output_memory_size: 47104,
        max_mesh_output_components: 128,
        max_mesh_output_vertices: 256,
        max_mesh_output_primitives: 256,
        max_mesh_output_layers: 8,
        max_mesh_multiview_view_count: 4,
        mesh_output_per_vertex_granularity: 32,
        mesh_output_per_primitive_granularity: 32,
        max_preferred_task_work_group_invocations: 32,
        max_preferred_mesh_work_group_invocations: 32,
        prefers_local_invocation_vertex_output: true,
        prefers_local_invocation_primitive_output: true,
        prefers_compact_vertex_output: false,
        prefers_compact_primitive_output: false,
    };
}

impl Default for MeshShaderLimits {
    fn default() -> Self {
        Self::NVIDIA_LIKE
    }
}

// ============================================================================
// Ray Tracing Limits
// ============================================================================

/// Ray tracing limits
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RayTracingLimits {
    /// Shader group handle size
    pub shader_group_handle_size: u32,
    /// Maximum ray recursion depth
    pub max_ray_recursion_depth: u32,
    /// Maximum shader group stride
    pub max_shader_group_stride: u32,
    /// Shader group base alignment
    pub shader_group_base_alignment: u32,
    /// Shader group handle capture replay size
    pub shader_group_handle_capture_replay_size: u32,
    /// Maximum ray dispatch invocation count
    pub max_ray_dispatch_invocation_count: u32,
    /// Shader group handle alignment
    pub shader_group_handle_alignment: u32,
    /// Maximum ray hit attribute size
    pub max_ray_hit_attribute_size: u32,
}

impl RayTracingLimits {
    /// NVIDIA-like limits
    pub const NVIDIA_LIKE: Self = Self {
        shader_group_handle_size: 32,
        max_ray_recursion_depth: 31,
        max_shader_group_stride: 4096,
        shader_group_base_alignment: 64,
        shader_group_handle_capture_replay_size: 32,
        max_ray_dispatch_invocation_count: 1 << 30,
        shader_group_handle_alignment: 32,
        max_ray_hit_attribute_size: 32,
    };

    /// AMD-like limits
    pub const AMD_LIKE: Self = Self {
        shader_group_handle_size: 32,
        max_ray_recursion_depth: 31,
        max_shader_group_stride: 4096,
        shader_group_base_alignment: 64,
        shader_group_handle_capture_replay_size: 64,
        max_ray_dispatch_invocation_count: 1 << 30,
        shader_group_handle_alignment: 32,
        max_ray_hit_attribute_size: 32,
    };
}

impl Default for RayTracingLimits {
    fn default() -> Self {
        Self::NVIDIA_LIKE
    }
}

/// Acceleration structure limits
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AccelerationStructureLimits {
    /// Maximum geometry count
    pub max_geometry_count: u64,
    /// Maximum instance count
    pub max_instance_count: u64,
    /// Maximum primitive count
    pub max_primitive_count: u64,
    /// Maximum per-stage descriptor acceleration structures
    pub max_per_stage_descriptor_acceleration_structures: u32,
    /// Maximum per-stage descriptor update after bind acceleration structures
    pub max_per_stage_descriptor_update_after_bind_acceleration_structures: u32,
    /// Maximum descriptor set acceleration structures
    pub max_descriptor_set_acceleration_structures: u32,
    /// Maximum descriptor set update after bind acceleration structures
    pub max_descriptor_set_update_after_bind_acceleration_structures: u32,
    /// Minimum acceleration structure scratch offset alignment
    pub min_acceleration_structure_scratch_offset_alignment: u32,
}

impl AccelerationStructureLimits {
    /// NVIDIA-like limits
    pub const NVIDIA_LIKE: Self = Self {
        max_geometry_count: 1 << 24,
        max_instance_count: 1 << 24,
        max_primitive_count: 1 << 29,
        max_per_stage_descriptor_acceleration_structures: 1 << 20,
        max_per_stage_descriptor_update_after_bind_acceleration_structures: 1 << 20,
        max_descriptor_set_acceleration_structures: 1 << 20,
        max_descriptor_set_update_after_bind_acceleration_structures: 1 << 20,
        min_acceleration_structure_scratch_offset_alignment: 128,
    };
}

impl Default for AccelerationStructureLimits {
    fn default() -> Self {
        Self::NVIDIA_LIKE
    }
}
