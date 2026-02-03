//! Feature Flags for Lumina
//!
//! This module provides GPU feature flags and capability detection for
//! determining supported hardware features.

// ============================================================================
// Physical Device Features
// ============================================================================

/// Physical device features
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PhysicalDeviceFeatures {
    // ========================================================================
    // Core Features
    // ========================================================================
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
    /// No features
    pub const NONE: Self = Self::new();

    /// Creates new with no features
    #[inline]
    pub const fn new() -> Self {
        Self {
            robust_buffer_access: false,
            full_draw_index_uint32: false,
            image_cube_array: false,
            independent_blend: false,
            geometry_shader: false,
            tessellation_shader: false,
            sample_rate_shading: false,
            dual_src_blend: false,
            logic_op: false,
            multi_draw_indirect: false,
            draw_indirect_first_instance: false,
            depth_clamp: false,
            depth_bias_clamp: false,
            fill_mode_non_solid: false,
            depth_bounds: false,
            wide_lines: false,
            large_points: false,
            alpha_to_one: false,
            multi_viewport: false,
            sampler_anisotropy: false,
            texture_compression_etc2: false,
            texture_compression_astc_ldr: false,
            texture_compression_bc: false,
            occlusion_query_precise: false,
            pipeline_statistics_query: false,
            vertex_pipeline_stores_and_atomics: false,
            fragment_stores_and_atomics: false,
            shader_tessellation_and_geometry_point_size: false,
            shader_image_gather_extended: false,
            shader_storage_image_extended_formats: false,
            shader_storage_image_multisample: false,
            shader_storage_image_read_without_format: false,
            shader_storage_image_write_without_format: false,
            shader_uniform_buffer_array_dynamic_indexing: false,
            shader_sampled_image_array_dynamic_indexing: false,
            shader_storage_buffer_array_dynamic_indexing: false,
            shader_storage_image_array_dynamic_indexing: false,
            shader_clip_distance: false,
            shader_cull_distance: false,
            shader_float64: false,
            shader_int64: false,
            shader_int16: false,
            shader_resource_residency: false,
            shader_resource_min_lod: false,
            sparse_binding: false,
            sparse_residency_buffer: false,
            sparse_residency_image_2d: false,
            sparse_residency_image_3d: false,
            sparse_residency_2_samples: false,
            sparse_residency_4_samples: false,
            sparse_residency_8_samples: false,
            sparse_residency_16_samples: false,
            sparse_residency_aliased: false,
            variable_multisample_rate: false,
            inherited_queries: false,
        }
    }

    /// Baseline features (minimum for modern GPU)
    pub const BASELINE: Self = Self {
        robust_buffer_access: true,
        full_draw_index_uint32: true,
        image_cube_array: true,
        independent_blend: true,
        geometry_shader: false,
        tessellation_shader: false,
        sample_rate_shading: true,
        dual_src_blend: true,
        logic_op: true,
        multi_draw_indirect: true,
        draw_indirect_first_instance: true,
        depth_clamp: true,
        depth_bias_clamp: true,
        fill_mode_non_solid: true,
        depth_bounds: false,
        wide_lines: false,
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
        shader_tessellation_and_geometry_point_size: false,
        shader_image_gather_extended: true,
        shader_storage_image_extended_formats: true,
        shader_storage_image_multisample: false,
        shader_storage_image_read_without_format: true,
        shader_storage_image_write_without_format: true,
        shader_uniform_buffer_array_dynamic_indexing: true,
        shader_sampled_image_array_dynamic_indexing: true,
        shader_storage_buffer_array_dynamic_indexing: true,
        shader_storage_image_array_dynamic_indexing: true,
        shader_clip_distance: true,
        shader_cull_distance: true,
        shader_float64: false,
        shader_int64: true,
        shader_int16: true,
        shader_resource_residency: false,
        shader_resource_min_lod: true,
        sparse_binding: false,
        sparse_residency_buffer: false,
        sparse_residency_image_2d: false,
        sparse_residency_image_3d: false,
        sparse_residency_2_samples: false,
        sparse_residency_4_samples: false,
        sparse_residency_8_samples: false,
        sparse_residency_16_samples: false,
        sparse_residency_aliased: false,
        variable_multisample_rate: true,
        inherited_queries: true,
    };

    /// All features enabled
    pub const ALL: Self = Self {
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
    };

    /// Check if features are subset
    #[inline]
    pub const fn is_subset_of(&self, other: &Self) -> bool {
        (!self.robust_buffer_access || other.robust_buffer_access)
            && (!self.full_draw_index_uint32 || other.full_draw_index_uint32)
            && (!self.image_cube_array || other.image_cube_array)
            && (!self.independent_blend || other.independent_blend)
            && (!self.geometry_shader || other.geometry_shader)
            && (!self.tessellation_shader || other.tessellation_shader)
            && (!self.sample_rate_shading || other.sample_rate_shading)
            && (!self.dual_src_blend || other.dual_src_blend)
            && (!self.logic_op || other.logic_op)
            && (!self.multi_draw_indirect || other.multi_draw_indirect)
    }
}

// ============================================================================
// Vulkan 1.1 Features
// ============================================================================

/// Vulkan 1.1 features
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Vulkan11Features {
    /// Storage buffer 16-bit access
    pub storage_buffer_16_bit_access: bool,
    /// Uniform and storage buffer 16-bit access
    pub uniform_and_storage_buffer_16_bit_access: bool,
    /// Storage push constant 16
    pub storage_push_constant_16: bool,
    /// Storage input output 16
    pub storage_input_output_16: bool,
    /// Multiview
    pub multiview: bool,
    /// Multiview geometry shader
    pub multiview_geometry_shader: bool,
    /// Multiview tessellation shader
    pub multiview_tessellation_shader: bool,
    /// Variable pointers storage buffer
    pub variable_pointers_storage_buffer: bool,
    /// Variable pointers
    pub variable_pointers: bool,
    /// Protected memory
    pub protected_memory: bool,
    /// Sampler Ycbcr conversion
    pub sampler_ycbcr_conversion: bool,
    /// Shader draw parameters
    pub shader_draw_parameters: bool,
}

impl Vulkan11Features {
    /// No features
    pub const NONE: Self = Self::new();

    /// Creates new
    #[inline]
    pub const fn new() -> Self {
        Self {
            storage_buffer_16_bit_access: false,
            uniform_and_storage_buffer_16_bit_access: false,
            storage_push_constant_16: false,
            storage_input_output_16: false,
            multiview: false,
            multiview_geometry_shader: false,
            multiview_tessellation_shader: false,
            variable_pointers_storage_buffer: false,
            variable_pointers: false,
            protected_memory: false,
            sampler_ycbcr_conversion: false,
            shader_draw_parameters: false,
        }
    }

    /// Baseline features
    pub const BASELINE: Self = Self {
        storage_buffer_16_bit_access: true,
        uniform_and_storage_buffer_16_bit_access: true,
        storage_push_constant_16: false,
        storage_input_output_16: false,
        multiview: true,
        multiview_geometry_shader: false,
        multiview_tessellation_shader: false,
        variable_pointers_storage_buffer: true,
        variable_pointers: true,
        protected_memory: false,
        sampler_ycbcr_conversion: false,
        shader_draw_parameters: true,
    };
}

// ============================================================================
// Vulkan 1.2 Features
// ============================================================================

/// Vulkan 1.2 features
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Vulkan12Features {
    /// Sampler mirror clamp to edge
    pub sampler_mirror_clamp_to_edge: bool,
    /// Draw indirect count
    pub draw_indirect_count: bool,
    /// Storage buffer 8-bit access
    pub storage_buffer_8_bit_access: bool,
    /// Uniform and storage buffer 8-bit access
    pub uniform_and_storage_buffer_8_bit_access: bool,
    /// Storage push constant 8
    pub storage_push_constant_8: bool,
    /// Shader buffer int64 atomics
    pub shader_buffer_int64_atomics: bool,
    /// Shader shared int64 atomics
    pub shader_shared_int64_atomics: bool,
    /// Shader float16
    pub shader_float16: bool,
    /// Shader int8
    pub shader_int8: bool,
    /// Descriptor indexing
    pub descriptor_indexing: bool,
    /// Shader input attachment array dynamic indexing
    pub shader_input_attachment_array_dynamic_indexing: bool,
    /// Shader uniform texel buffer array dynamic indexing
    pub shader_uniform_texel_buffer_array_dynamic_indexing: bool,
    /// Shader storage texel buffer array dynamic indexing
    pub shader_storage_texel_buffer_array_dynamic_indexing: bool,
    /// Shader uniform buffer array non-uniform indexing
    pub shader_uniform_buffer_array_non_uniform_indexing: bool,
    /// Shader sampled image array non-uniform indexing
    pub shader_sampled_image_array_non_uniform_indexing: bool,
    /// Shader storage buffer array non-uniform indexing
    pub shader_storage_buffer_array_non_uniform_indexing: bool,
    /// Shader storage image array non-uniform indexing
    pub shader_storage_image_array_non_uniform_indexing: bool,
    /// Shader input attachment array non-uniform indexing
    pub shader_input_attachment_array_non_uniform_indexing: bool,
    /// Shader uniform texel buffer array non-uniform indexing
    pub shader_uniform_texel_buffer_array_non_uniform_indexing: bool,
    /// Shader storage texel buffer array non-uniform indexing
    pub shader_storage_texel_buffer_array_non_uniform_indexing: bool,
    /// Descriptor binding uniform buffer update after bind
    pub descriptor_binding_uniform_buffer_update_after_bind: bool,
    /// Descriptor binding sampled image update after bind
    pub descriptor_binding_sampled_image_update_after_bind: bool,
    /// Descriptor binding storage image update after bind
    pub descriptor_binding_storage_image_update_after_bind: bool,
    /// Descriptor binding storage buffer update after bind
    pub descriptor_binding_storage_buffer_update_after_bind: bool,
    /// Descriptor binding uniform texel buffer update after bind
    pub descriptor_binding_uniform_texel_buffer_update_after_bind: bool,
    /// Descriptor binding storage texel buffer update after bind
    pub descriptor_binding_storage_texel_buffer_update_after_bind: bool,
    /// Descriptor binding update unused while pending
    pub descriptor_binding_update_unused_while_pending: bool,
    /// Descriptor binding partially bound
    pub descriptor_binding_partially_bound: bool,
    /// Descriptor binding variable descriptor count
    pub descriptor_binding_variable_descriptor_count: bool,
    /// Runtime descriptor array
    pub runtime_descriptor_array: bool,
    /// Sampler filter minmax
    pub sampler_filter_minmax: bool,
    /// Scalar block layout
    pub scalar_block_layout: bool,
    /// Imageless framebuffer
    pub imageless_framebuffer: bool,
    /// Uniform buffer standard layout
    pub uniform_buffer_standard_layout: bool,
    /// Shader subgroup extended types
    pub shader_subgroup_extended_types: bool,
    /// Separate depth stencil layouts
    pub separate_depth_stencil_layouts: bool,
    /// Host query reset
    pub host_query_reset: bool,
    /// Timeline semaphore
    pub timeline_semaphore: bool,
    /// Buffer device address
    pub buffer_device_address: bool,
    /// Buffer device address capture replay
    pub buffer_device_address_capture_replay: bool,
    /// Buffer device address multi device
    pub buffer_device_address_multi_device: bool,
    /// Vulkan memory model
    pub vulkan_memory_model: bool,
    /// Vulkan memory model device scope
    pub vulkan_memory_model_device_scope: bool,
    /// Vulkan memory model availability visibility chains
    pub vulkan_memory_model_availability_visibility_chains: bool,
    /// Shader output viewport index
    pub shader_output_viewport_index: bool,
    /// Shader output layer
    pub shader_output_layer: bool,
    /// Subgroup broadcast dynamic ID
    pub subgroup_broadcast_dynamic_id: bool,
}

impl Vulkan12Features {
    /// No features
    pub const NONE: Self = Self::new();

    /// Creates new
    #[inline]
    pub const fn new() -> Self {
        Self {
            sampler_mirror_clamp_to_edge: false,
            draw_indirect_count: false,
            storage_buffer_8_bit_access: false,
            uniform_and_storage_buffer_8_bit_access: false,
            storage_push_constant_8: false,
            shader_buffer_int64_atomics: false,
            shader_shared_int64_atomics: false,
            shader_float16: false,
            shader_int8: false,
            descriptor_indexing: false,
            shader_input_attachment_array_dynamic_indexing: false,
            shader_uniform_texel_buffer_array_dynamic_indexing: false,
            shader_storage_texel_buffer_array_dynamic_indexing: false,
            shader_uniform_buffer_array_non_uniform_indexing: false,
            shader_sampled_image_array_non_uniform_indexing: false,
            shader_storage_buffer_array_non_uniform_indexing: false,
            shader_storage_image_array_non_uniform_indexing: false,
            shader_input_attachment_array_non_uniform_indexing: false,
            shader_uniform_texel_buffer_array_non_uniform_indexing: false,
            shader_storage_texel_buffer_array_non_uniform_indexing: false,
            descriptor_binding_uniform_buffer_update_after_bind: false,
            descriptor_binding_sampled_image_update_after_bind: false,
            descriptor_binding_storage_image_update_after_bind: false,
            descriptor_binding_storage_buffer_update_after_bind: false,
            descriptor_binding_uniform_texel_buffer_update_after_bind: false,
            descriptor_binding_storage_texel_buffer_update_after_bind: false,
            descriptor_binding_update_unused_while_pending: false,
            descriptor_binding_partially_bound: false,
            descriptor_binding_variable_descriptor_count: false,
            runtime_descriptor_array: false,
            sampler_filter_minmax: false,
            scalar_block_layout: false,
            imageless_framebuffer: false,
            uniform_buffer_standard_layout: false,
            shader_subgroup_extended_types: false,
            separate_depth_stencil_layouts: false,
            host_query_reset: false,
            timeline_semaphore: false,
            buffer_device_address: false,
            buffer_device_address_capture_replay: false,
            buffer_device_address_multi_device: false,
            vulkan_memory_model: false,
            vulkan_memory_model_device_scope: false,
            vulkan_memory_model_availability_visibility_chains: false,
            shader_output_viewport_index: false,
            shader_output_layer: false,
            subgroup_broadcast_dynamic_id: false,
        }
    }

    /// Bindless features
    pub const BINDLESS: Self = Self {
        descriptor_indexing: true,
        shader_sampled_image_array_non_uniform_indexing: true,
        shader_storage_buffer_array_non_uniform_indexing: true,
        shader_storage_image_array_non_uniform_indexing: true,
        descriptor_binding_sampled_image_update_after_bind: true,
        descriptor_binding_storage_image_update_after_bind: true,
        descriptor_binding_storage_buffer_update_after_bind: true,
        descriptor_binding_update_unused_while_pending: true,
        descriptor_binding_partially_bound: true,
        descriptor_binding_variable_descriptor_count: true,
        runtime_descriptor_array: true,
        ..Self::NONE
    };

    /// Modern GPU features
    pub const MODERN: Self = Self {
        draw_indirect_count: true,
        descriptor_indexing: true,
        scalar_block_layout: true,
        imageless_framebuffer: true,
        uniform_buffer_standard_layout: true,
        separate_depth_stencil_layouts: true,
        host_query_reset: true,
        timeline_semaphore: true,
        buffer_device_address: true,
        shader_output_viewport_index: true,
        shader_output_layer: true,
        ..Self::BINDLESS
    };
}

// ============================================================================
// Vulkan 1.3 Features
// ============================================================================

/// Vulkan 1.3 features
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Vulkan13Features {
    /// Robust image access
    pub robust_image_access: bool,
    /// Inline uniform block
    pub inline_uniform_block: bool,
    /// Descriptor binding inline uniform block update after bind
    pub descriptor_binding_inline_uniform_block_update_after_bind: bool,
    /// Pipeline creation cache control
    pub pipeline_creation_cache_control: bool,
    /// Private data
    pub private_data: bool,
    /// Shader demote to helper invocation
    pub shader_demote_to_helper_invocation: bool,
    /// Shader terminate invocation
    pub shader_terminate_invocation: bool,
    /// Subgroup size control
    pub subgroup_size_control: bool,
    /// Compute full subgroups
    pub compute_full_subgroups: bool,
    /// Synchronization 2
    pub synchronization2: bool,
    /// Texture compression ASTC HDR
    pub texture_compression_astc_hdr: bool,
    /// Shader zero initialize workgroup memory
    pub shader_zero_initialize_workgroup_memory: bool,
    /// Dynamic rendering
    pub dynamic_rendering: bool,
    /// Shader integer dot product
    pub shader_integer_dot_product: bool,
    /// Maintenance 4
    pub maintenance4: bool,
}

impl Vulkan13Features {
    /// No features
    pub const NONE: Self = Self::new();

    /// Creates new
    #[inline]
    pub const fn new() -> Self {
        Self {
            robust_image_access: false,
            inline_uniform_block: false,
            descriptor_binding_inline_uniform_block_update_after_bind: false,
            pipeline_creation_cache_control: false,
            private_data: false,
            shader_demote_to_helper_invocation: false,
            shader_terminate_invocation: false,
            subgroup_size_control: false,
            compute_full_subgroups: false,
            synchronization2: false,
            texture_compression_astc_hdr: false,
            shader_zero_initialize_workgroup_memory: false,
            dynamic_rendering: false,
            shader_integer_dot_product: false,
            maintenance4: false,
        }
    }

    /// Baseline features
    pub const BASELINE: Self = Self {
        robust_image_access: true,
        inline_uniform_block: true,
        descriptor_binding_inline_uniform_block_update_after_bind: true,
        pipeline_creation_cache_control: true,
        private_data: true,
        shader_demote_to_helper_invocation: true,
        shader_terminate_invocation: true,
        subgroup_size_control: true,
        compute_full_subgroups: true,
        synchronization2: true,
        texture_compression_astc_hdr: false,
        shader_zero_initialize_workgroup_memory: true,
        dynamic_rendering: true,
        shader_integer_dot_product: true,
        maintenance4: true,
    };
}

// ============================================================================
// Extension Features
// ============================================================================

/// Ray tracing features
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct RayTracingFeatures {
    /// Ray tracing pipeline
    pub ray_tracing_pipeline: bool,
    /// Ray tracing pipeline shader group handle capture replay
    pub ray_tracing_pipeline_shader_group_handle_capture_replay: bool,
    /// Ray tracing pipeline shader group handle capture replay mixed
    pub ray_tracing_pipeline_shader_group_handle_capture_replay_mixed: bool,
    /// Ray tracing pipeline trace rays indirect
    pub ray_tracing_pipeline_trace_rays_indirect: bool,
    /// Ray traversal primitive culling
    pub ray_traversal_primitive_culling: bool,
    /// Acceleration structure
    pub acceleration_structure: bool,
    /// Acceleration structure capture replay
    pub acceleration_structure_capture_replay: bool,
    /// Acceleration structure indirect build
    pub acceleration_structure_indirect_build: bool,
    /// Acceleration structure host commands
    pub acceleration_structure_host_commands: bool,
    /// Descriptor binding acceleration structure update after bind
    pub descriptor_binding_acceleration_structure_update_after_bind: bool,
    /// Ray query
    pub ray_query: bool,
}

impl RayTracingFeatures {
    /// No features
    pub const NONE: Self = Self::new();

    /// Creates new
    #[inline]
    pub const fn new() -> Self {
        Self {
            ray_tracing_pipeline: false,
            ray_tracing_pipeline_shader_group_handle_capture_replay: false,
            ray_tracing_pipeline_shader_group_handle_capture_replay_mixed: false,
            ray_tracing_pipeline_trace_rays_indirect: false,
            ray_traversal_primitive_culling: false,
            acceleration_structure: false,
            acceleration_structure_capture_replay: false,
            acceleration_structure_indirect_build: false,
            acceleration_structure_host_commands: false,
            descriptor_binding_acceleration_structure_update_after_bind: false,
            ray_query: false,
        }
    }

    /// Full ray tracing
    pub const FULL: Self = Self {
        ray_tracing_pipeline: true,
        ray_tracing_pipeline_shader_group_handle_capture_replay: true,
        ray_tracing_pipeline_shader_group_handle_capture_replay_mixed: false,
        ray_tracing_pipeline_trace_rays_indirect: true,
        ray_traversal_primitive_culling: true,
        acceleration_structure: true,
        acceleration_structure_capture_replay: true,
        acceleration_structure_indirect_build: true,
        acceleration_structure_host_commands: false,
        descriptor_binding_acceleration_structure_update_after_bind: true,
        ray_query: true,
    };

    /// Ray query only
    pub const RAY_QUERY_ONLY: Self = Self {
        acceleration_structure: true,
        ray_query: true,
        ..Self::NONE
    };
}

/// Mesh shader features
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MeshShaderFeatures {
    /// Task shader
    pub task_shader: bool,
    /// Mesh shader
    pub mesh_shader: bool,
    /// Multiview mesh shader
    pub multiview_mesh_shader: bool,
    /// Primitive fragment shading rate mesh shader
    pub primitive_fragment_shading_rate_mesh_shader: bool,
    /// Mesh shader queries
    pub mesh_shader_queries: bool,
}

impl MeshShaderFeatures {
    /// No features
    pub const NONE: Self = Self::new();

    /// Creates new
    #[inline]
    pub const fn new() -> Self {
        Self {
            task_shader: false,
            mesh_shader: false,
            multiview_mesh_shader: false,
            primitive_fragment_shading_rate_mesh_shader: false,
            mesh_shader_queries: false,
        }
    }

    /// Full mesh shading
    pub const FULL: Self = Self {
        task_shader: true,
        mesh_shader: true,
        multiview_mesh_shader: true,
        primitive_fragment_shading_rate_mesh_shader: true,
        mesh_shader_queries: true,
    };

    /// Mesh shader only (no task)
    pub const MESH_ONLY: Self = Self {
        mesh_shader: true,
        ..Self::NONE
    };
}

/// Fragment shading rate features
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FragmentShadingRateFeatures {
    /// Pipeline fragment shading rate
    pub pipeline_fragment_shading_rate: bool,
    /// Primitive fragment shading rate
    pub primitive_fragment_shading_rate: bool,
    /// Attachment fragment shading rate
    pub attachment_fragment_shading_rate: bool,
}

impl FragmentShadingRateFeatures {
    /// No features
    pub const NONE: Self = Self::new();

    /// Creates new
    #[inline]
    pub const fn new() -> Self {
        Self {
            pipeline_fragment_shading_rate: false,
            primitive_fragment_shading_rate: false,
            attachment_fragment_shading_rate: false,
        }
    }

    /// All features
    pub const ALL: Self = Self {
        pipeline_fragment_shading_rate: true,
        primitive_fragment_shading_rate: true,
        attachment_fragment_shading_rate: true,
    };
}

// ============================================================================
// Feature Set
// ============================================================================

/// Complete feature set
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FeatureSet {
    /// Core features
    pub core: PhysicalDeviceFeatures,
    /// Vulkan 1.1 features
    pub vulkan_1_1: Vulkan11Features,
    /// Vulkan 1.2 features
    pub vulkan_1_2: Vulkan12Features,
    /// Vulkan 1.3 features
    pub vulkan_1_3: Vulkan13Features,
    /// Ray tracing features
    pub ray_tracing: RayTracingFeatures,
    /// Mesh shader features
    pub mesh_shader: MeshShaderFeatures,
    /// Fragment shading rate features
    pub fragment_shading_rate: FragmentShadingRateFeatures,
}

impl FeatureSet {
    /// No features
    pub const NONE: Self = Self {
        core: PhysicalDeviceFeatures::NONE,
        vulkan_1_1: Vulkan11Features::NONE,
        vulkan_1_2: Vulkan12Features::NONE,
        vulkan_1_3: Vulkan13Features::NONE,
        ray_tracing: RayTracingFeatures::NONE,
        mesh_shader: MeshShaderFeatures::NONE,
        fragment_shading_rate: FragmentShadingRateFeatures::NONE,
    };

    /// Baseline modern GPU features
    pub const BASELINE: Self = Self {
        core: PhysicalDeviceFeatures::BASELINE,
        vulkan_1_1: Vulkan11Features::BASELINE,
        vulkan_1_2: Vulkan12Features::MODERN,
        vulkan_1_3: Vulkan13Features::BASELINE,
        ray_tracing: RayTracingFeatures::NONE,
        mesh_shader: MeshShaderFeatures::NONE,
        fragment_shading_rate: FragmentShadingRateFeatures::NONE,
    };

    /// High-end GPU features
    pub const HIGH_END: Self = Self {
        core: PhysicalDeviceFeatures::ALL,
        vulkan_1_1: Vulkan11Features::BASELINE,
        vulkan_1_2: Vulkan12Features::MODERN,
        vulkan_1_3: Vulkan13Features::BASELINE,
        ray_tracing: RayTracingFeatures::FULL,
        mesh_shader: MeshShaderFeatures::FULL,
        fragment_shading_rate: FragmentShadingRateFeatures::ALL,
    };

    /// Has ray tracing
    #[inline]
    pub const fn has_ray_tracing(&self) -> bool {
        self.ray_tracing.ray_tracing_pipeline || self.ray_tracing.ray_query
    }

    /// Has mesh shading
    #[inline]
    pub const fn has_mesh_shading(&self) -> bool {
        self.mesh_shader.mesh_shader
    }

    /// Has bindless
    #[inline]
    pub const fn has_bindless(&self) -> bool {
        self.vulkan_1_2.descriptor_indexing && self.vulkan_1_2.runtime_descriptor_array
    }

    /// Has dynamic rendering
    #[inline]
    pub const fn has_dynamic_rendering(&self) -> bool {
        self.vulkan_1_3.dynamic_rendering
    }
}

// ============================================================================
// Feature Requirements
// ============================================================================

/// Feature requirement
#[derive(Clone, Copy, Debug)]
pub struct FeatureRequirement {
    /// Feature name
    pub name: &'static str,
    /// Required
    pub required: bool,
    /// Fallback available
    pub fallback_available: bool,
}

impl FeatureRequirement {
    /// Creates required feature
    #[inline]
    pub const fn required(name: &'static str) -> Self {
        Self {
            name,
            required: true,
            fallback_available: false,
        }
    }

    /// Creates optional feature
    #[inline]
    pub const fn optional(name: &'static str) -> Self {
        Self {
            name,
            required: false,
            fallback_available: false,
        }
    }

    /// With fallback
    #[inline]
    pub const fn with_fallback(mut self) -> Self {
        self.fallback_available = true;
        self
    }
}

/// Feature compatibility result
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureCompatibility {
    /// Fully compatible
    Compatible,
    /// Compatible with fallbacks
    CompatibleWithFallbacks,
    /// Missing required features
    MissingRequired,
}

impl FeatureCompatibility {
    /// Is compatible
    #[inline]
    pub const fn is_compatible(&self) -> bool {
        matches!(self, Self::Compatible | Self::CompatibleWithFallbacks)
    }

    /// Is fully compatible
    #[inline]
    pub const fn is_fully_compatible(&self) -> bool {
        matches!(self, Self::Compatible)
    }
}
