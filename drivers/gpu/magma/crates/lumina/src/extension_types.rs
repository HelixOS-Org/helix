//! Extension Types for Lumina
//!
//! This module provides Vulkan extension type definitions, extension
//! enumeration, and extension property types.

use core::fmt;

// ============================================================================
// Extension Names
// ============================================================================

/// Vulkan instance extension
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum InstanceExtension {
    /// VK_KHR_surface
    Surface             = 0,
    /// VK_KHR_display
    Display             = 1,
    /// VK_KHR_xlib_surface
    XlibSurface         = 2,
    /// VK_KHR_xcb_surface
    XcbSurface          = 3,
    /// VK_KHR_wayland_surface
    WaylandSurface      = 4,
    /// VK_KHR_win32_surface
    Win32Surface        = 5,
    /// VK_EXT_debug_utils
    DebugUtils          = 6,
    /// VK_EXT_debug_report
    DebugReport         = 7,
    /// VK_KHR_get_physical_device_properties2
    GetPhysicalDeviceProperties2 = 8,
    /// VK_KHR_get_surface_capabilities2
    GetSurfaceCapabilities2 = 9,
    /// VK_KHR_external_memory_capabilities
    ExternalMemoryCapabilities = 10,
    /// VK_KHR_external_semaphore_capabilities
    ExternalSemaphoreCapabilities = 11,
    /// VK_KHR_external_fence_capabilities
    ExternalFenceCapabilities = 12,
    /// VK_EXT_swapchain_colorspace
    SwapchainColorspace = 13,
    /// VK_EXT_validation_features
    ValidationFeatures  = 14,
    /// VK_EXT_validation_flags
    ValidationFlags     = 15,
    /// VK_KHR_device_group_creation
    DeviceGroupCreation = 16,
    /// VK_EXT_headless_surface
    HeadlessSurface     = 17,
    /// VK_EXT_metal_surface
    MetalSurface        = 18,
    /// VK_KHR_portability_enumeration
    PortabilityEnumeration = 19,
    /// VK_EXT_surface_maintenance1
    SurfaceMaintenance1 = 20,
    /// VK_EXT_direct_mode_display
    DirectModeDisplay   = 21,
    /// VK_EXT_acquire_xlib_display
    AcquireXlibDisplay  = 22,
    /// VK_EXT_display_surface_counter
    DisplaySurfaceCounter = 23,
}

impl InstanceExtension {
    /// Extension name string
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Surface => "VK_KHR_surface",
            Self::Display => "VK_KHR_display",
            Self::XlibSurface => "VK_KHR_xlib_surface",
            Self::XcbSurface => "VK_KHR_xcb_surface",
            Self::WaylandSurface => "VK_KHR_wayland_surface",
            Self::Win32Surface => "VK_KHR_win32_surface",
            Self::DebugUtils => "VK_EXT_debug_utils",
            Self::DebugReport => "VK_EXT_debug_report",
            Self::GetPhysicalDeviceProperties2 => "VK_KHR_get_physical_device_properties2",
            Self::GetSurfaceCapabilities2 => "VK_KHR_get_surface_capabilities2",
            Self::ExternalMemoryCapabilities => "VK_KHR_external_memory_capabilities",
            Self::ExternalSemaphoreCapabilities => "VK_KHR_external_semaphore_capabilities",
            Self::ExternalFenceCapabilities => "VK_KHR_external_fence_capabilities",
            Self::SwapchainColorspace => "VK_EXT_swapchain_colorspace",
            Self::ValidationFeatures => "VK_EXT_validation_features",
            Self::ValidationFlags => "VK_EXT_validation_flags",
            Self::DeviceGroupCreation => "VK_KHR_device_group_creation",
            Self::HeadlessSurface => "VK_EXT_headless_surface",
            Self::MetalSurface => "VK_EXT_metal_surface",
            Self::PortabilityEnumeration => "VK_KHR_portability_enumeration",
            Self::SurfaceMaintenance1 => "VK_EXT_surface_maintenance1",
            Self::DirectModeDisplay => "VK_EXT_direct_mode_display",
            Self::AcquireXlibDisplay => "VK_EXT_acquire_xlib_display",
            Self::DisplaySurfaceCounter => "VK_EXT_display_surface_counter",
        }
    }

    /// Is debug extension
    #[inline]
    pub const fn is_debug(&self) -> bool {
        matches!(
            self,
            Self::DebugUtils | Self::DebugReport | Self::ValidationFeatures | Self::ValidationFlags
        )
    }

    /// Is platform-specific
    #[inline]
    pub const fn is_platform_specific(&self) -> bool {
        matches!(
            self,
            Self::XlibSurface
                | Self::XcbSurface
                | Self::WaylandSurface
                | Self::Win32Surface
                | Self::MetalSurface
        )
    }
}

impl fmt::Display for InstanceExtension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Vulkan device extension
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum DeviceExtension {
    // ========================================================================
    // Swapchain & Display
    // ========================================================================
    /// VK_KHR_swapchain
    Swapchain            = 0,
    /// VK_KHR_display_swapchain
    DisplaySwapchain     = 1,
    /// VK_EXT_swapchain_maintenance1
    SwapchainMaintenance1 = 2,
    /// VK_EXT_hdr_metadata
    HdrMetadata          = 3,
    /// VK_EXT_full_screen_exclusive
    FullScreenExclusive  = 4,
    /// VK_KHR_present_id
    PresentId            = 5,
    /// VK_KHR_present_wait
    PresentWait          = 6,

    // ========================================================================
    // Ray Tracing
    // ========================================================================
    /// VK_KHR_acceleration_structure
    AccelerationStructure = 10,
    /// VK_KHR_ray_tracing_pipeline
    RayTracingPipeline   = 11,
    /// VK_KHR_ray_query
    RayQuery             = 12,
    /// VK_KHR_ray_tracing_maintenance1
    RayTracingMaintenance1 = 13,
    /// VK_KHR_ray_tracing_position_fetch
    RayTracingPositionFetch = 14,
    /// VK_NV_ray_tracing
    RayTracingNV         = 15,
    /// VK_NV_ray_tracing_motion_blur
    RayTracingMotionBlur = 16,
    /// VK_NV_ray_tracing_invocation_reorder
    RayTracingInvocationReorder = 17,

    // ========================================================================
    // Mesh Shading
    // ========================================================================
    /// VK_EXT_mesh_shader
    MeshShader           = 20,
    /// VK_NV_mesh_shader
    MeshShaderNV         = 21,

    // ========================================================================
    // Descriptors
    // ========================================================================
    /// VK_KHR_push_descriptor
    PushDescriptor       = 30,
    /// VK_EXT_descriptor_indexing
    DescriptorIndexing   = 31,
    /// VK_EXT_descriptor_buffer
    DescriptorBuffer     = 32,
    /// VK_KHR_descriptor_update_template
    DescriptorUpdateTemplate = 33,
    /// VK_EXT_mutable_descriptor_type
    MutableDescriptorType = 34,

    // ========================================================================
    // Memory
    // ========================================================================
    /// VK_KHR_dedicated_allocation
    DedicatedAllocation  = 40,
    /// VK_KHR_buffer_device_address
    BufferDeviceAddress  = 41,
    /// VK_EXT_memory_budget
    MemoryBudget         = 42,
    /// VK_EXT_memory_priority
    MemoryPriority       = 43,
    /// VK_EXT_pageable_device_local_memory
    PageableDeviceLocalMemory = 44,
    /// VK_KHR_external_memory
    ExternalMemory       = 45,
    /// VK_KHR_external_memory_fd
    ExternalMemoryFd     = 46,
    /// VK_KHR_external_memory_win32
    ExternalMemoryWin32  = 47,
    /// VK_EXT_external_memory_host
    ExternalMemoryHost   = 48,
    /// VK_EXT_external_memory_dma_buf
    ExternalMemoryDmaBuf = 49,
    /// VK_KHR_bind_memory2
    BindMemory2          = 50,
    /// VK_KHR_get_memory_requirements2
    GetMemoryRequirements2 = 51,

    // ========================================================================
    // Synchronization
    // ========================================================================
    /// VK_KHR_timeline_semaphore
    TimelineSemaphore    = 60,
    /// VK_KHR_synchronization2
    Synchronization2     = 61,
    /// VK_KHR_external_semaphore
    ExternalSemaphore    = 62,
    /// VK_KHR_external_semaphore_fd
    ExternalSemaphoreFd  = 63,
    /// VK_KHR_external_semaphore_win32
    ExternalSemaphoreWin32 = 64,
    /// VK_KHR_external_fence
    ExternalFence        = 65,
    /// VK_KHR_external_fence_fd
    ExternalFenceFd      = 66,
    /// VK_KHR_external_fence_win32
    ExternalFenceWin32   = 67,

    // ========================================================================
    // Pipelines
    // ========================================================================
    /// VK_EXT_graphics_pipeline_library
    GraphicsPipelineLibrary = 70,
    /// VK_KHR_pipeline_library
    PipelineLibrary      = 71,
    /// VK_KHR_pipeline_executable_properties
    PipelineExecutableProperties = 72,
    /// VK_EXT_pipeline_creation_cache_control
    PipelineCreationCacheControl = 73,
    /// VK_EXT_pipeline_creation_feedback
    PipelineCreationFeedback = 74,
    /// VK_EXT_shader_object
    ShaderObject         = 75,

    // ========================================================================
    // Rendering
    // ========================================================================
    /// VK_KHR_dynamic_rendering
    DynamicRendering     = 80,
    /// VK_KHR_multiview
    Multiview            = 81,
    /// VK_EXT_multisampled_render_to_single_sampled
    MultisampledRenderToSingleSampled = 82,
    /// VK_KHR_fragment_shading_rate
    FragmentShadingRate  = 83,
    /// VK_EXT_fragment_density_map
    FragmentDensityMap   = 84,
    /// VK_EXT_fragment_density_map2
    FragmentDensityMap2  = 85,
    /// VK_EXT_attachment_feedback_loop_layout
    AttachmentFeedbackLoopLayout = 86,
    /// VK_EXT_attachment_feedback_loop_dynamic_state
    AttachmentFeedbackLoopDynamicState = 87,
    /// VK_KHR_imageless_framebuffer
    ImagelessFramebuffer = 88,
    /// VK_KHR_depth_stencil_resolve
    DepthStencilResolve  = 89,
    /// VK_KHR_separate_depth_stencil_layouts
    SeparateDepthStencilLayouts = 90,
    /// VK_EXT_conservative_rasterization
    ConservativeRasterization = 91,
    /// VK_EXT_depth_clip_enable
    DepthClipEnable      = 92,
    /// VK_EXT_depth_clamp_zero_one
    DepthClampZeroOne    = 93,

    // ========================================================================
    // Dynamic State
    // ========================================================================
    /// VK_EXT_extended_dynamic_state
    ExtendedDynamicState = 100,
    /// VK_EXT_extended_dynamic_state2
    ExtendedDynamicState2 = 101,
    /// VK_EXT_extended_dynamic_state3
    ExtendedDynamicState3 = 102,
    /// VK_EXT_vertex_input_dynamic_state
    VertexInputDynamicState = 103,
    /// VK_EXT_color_write_enable
    ColorWriteEnable     = 104,

    // ========================================================================
    // Images & Formats
    // ========================================================================
    /// VK_KHR_sampler_ycbcr_conversion
    SamplerYcbcrConversion = 110,
    /// VK_EXT_ycbcr_2plane_444_formats
    Ycbcr2Plane444Formats = 111,
    /// VK_EXT_4444_formats
    Formats4444          = 112,
    /// VK_EXT_rgba10x6_formats
    Rgba10x6Formats      = 113,
    /// VK_KHR_image_format_list
    ImageFormatList      = 114,
    /// VK_EXT_image_view_min_lod
    ImageViewMinLod      = 115,
    /// VK_EXT_image_sliced_view_of_3d
    ImageSlicedViewOf3d  = 116,
    /// VK_EXT_filter_cubic
    FilterCubic          = 117,
    /// VK_EXT_sampler_filter_minmax
    SamplerFilterMinmax  = 118,
    /// VK_EXT_astc_decode_mode
    AstcDecodeMode       = 119,
    /// VK_EXT_texture_compression_astc_hdr
    TextureCompressionAstcHdr = 120,
    /// VK_KHR_format_feature_flags2
    FormatFeatureFlags2  = 121,

    // ========================================================================
    // Shaders
    // ========================================================================
    /// VK_KHR_shader_float16_int8
    ShaderFloat16Int8    = 130,
    /// VK_KHR_shader_float_controls
    ShaderFloatControls  = 131,
    /// VK_KHR_shader_atomic_int64
    ShaderAtomicInt64    = 132,
    /// VK_EXT_shader_image_atomic_int64
    ShaderImageAtomicInt64 = 133,
    /// VK_KHR_shader_clock
    ShaderClock          = 134,
    /// VK_EXT_shader_subgroup_ballot
    ShaderSubgroupBallot = 135,
    /// VK_EXT_shader_subgroup_vote
    ShaderSubgroupVote   = 136,
    /// VK_KHR_shader_subgroup_extended_types
    ShaderSubgroupExtendedTypes = 137,
    /// VK_EXT_subgroup_size_control
    SubgroupSizeControl  = 138,
    /// VK_EXT_shader_demote_to_helper_invocation
    ShaderDemoteToHelperInvocation = 139,
    /// VK_EXT_shader_stencil_export
    ShaderStencilExport  = 140,
    /// VK_EXT_shader_viewport_index_layer
    ShaderViewportIndexLayer = 141,
    /// VK_KHR_shader_draw_parameters
    ShaderDrawParameters = 142,
    /// VK_KHR_shader_non_semantic_info
    ShaderNonSemanticInfo = 143,
    /// VK_KHR_shader_terminate_invocation
    ShaderTerminateInvocation = 144,
    /// VK_EXT_shader_module_identifier
    ShaderModuleIdentifier = 145,
    /// VK_KHR_spirv_1_4
    Spirv14              = 146,
    /// VK_KHR_vulkan_memory_model
    VulkanMemoryModel    = 147,

    // ========================================================================
    // Debug & Validation
    // ========================================================================
    /// VK_EXT_debug_marker
    DebugMarker          = 150,
    /// VK_EXT_tooling_info
    ToolingInfo          = 151,
    /// VK_EXT_device_fault
    DeviceFault          = 152,
    /// VK_EXT_device_address_binding_report
    DeviceAddressBindingReport = 153,
    /// VK_EXT_device_memory_report
    DeviceMemoryReport   = 154,

    // ========================================================================
    // Video
    // ========================================================================
    /// VK_KHR_video_queue
    VideoQueue           = 160,
    /// VK_KHR_video_decode_queue
    VideoDecodeQueue     = 161,
    /// VK_KHR_video_encode_queue
    VideoEncodeQueue     = 162,
    /// VK_KHR_video_decode_h264
    VideoDecodeH264      = 163,
    /// VK_KHR_video_decode_h265
    VideoDecodeH265      = 164,
    /// VK_KHR_video_encode_h264
    VideoEncodeH264      = 165,
    /// VK_KHR_video_encode_h265
    VideoEncodeH265      = 166,
    /// VK_KHR_video_decode_av1
    VideoDecodeAv1       = 167,

    // ========================================================================
    // Transform Feedback
    // ========================================================================
    /// VK_EXT_transform_feedback
    TransformFeedback    = 170,

    // ========================================================================
    // Conditional Rendering
    // ========================================================================
    /// VK_EXT_conditional_rendering
    ConditionalRendering = 171,

    // ========================================================================
    // Multi-GPU
    // ========================================================================
    /// VK_KHR_device_group
    DeviceGroup          = 180,
    /// VK_EXT_physical_device_drm
    PhysicalDeviceDrm    = 181,
    /// VK_EXT_pci_bus_info
    PciBusInfo           = 182,

    // ========================================================================
    // Maintenance
    // ========================================================================
    /// VK_KHR_maintenance1
    Maintenance1         = 190,
    /// VK_KHR_maintenance2
    Maintenance2         = 191,
    /// VK_KHR_maintenance3
    Maintenance3         = 192,
    /// VK_KHR_maintenance4
    Maintenance4         = 193,
    /// VK_KHR_maintenance5
    Maintenance5         = 194,
    /// VK_KHR_maintenance6
    Maintenance6         = 195,

    // ========================================================================
    // Other
    // ========================================================================
    /// VK_EXT_host_query_reset
    HostQueryReset       = 200,
    /// VK_KHR_create_renderpass2
    CreateRenderpass2    = 201,
    /// VK_EXT_custom_border_color
    CustomBorderColor    = 202,
    /// VK_EXT_border_color_swizzle
    BorderColorSwizzle   = 203,
    /// VK_EXT_robustness2
    Robustness2          = 204,
    /// VK_EXT_image_robustness
    ImageRobustness      = 205,
    /// VK_EXT_inline_uniform_block
    InlineUniformBlock   = 206,
    /// VK_EXT_private_data
    PrivateData          = 207,
    /// VK_KHR_draw_indirect_count
    DrawIndirectCount    = 208,
    /// VK_EXT_multi_draw
    MultiDraw            = 209,
    /// VK_EXT_index_type_uint8
    IndexTypeUint8       = 210,
    /// VK_EXT_primitive_topology_list_restart
    PrimitiveTopologyListRestart = 211,
    /// VK_EXT_primitives_generated_query
    PrimitivesGeneratedQuery = 212,
    /// VK_EXT_load_store_op_none
    LoadStoreOpNone      = 213,
    /// VK_EXT_line_rasterization
    LineRasterization    = 214,
    /// VK_EXT_provoking_vertex
    ProvokingVertex      = 215,
    /// VK_EXT_non_seamless_cube_map
    NonSeamlessCubeMap   = 216,
    /// VK_EXT_calibrated_timestamps
    CalibratedTimestamps = 217,
    /// VK_EXT_host_image_copy
    HostImageCopy        = 218,
    /// VK_KHR_copy_commands2
    CopyCommands2        = 219,
    /// VK_EXT_sample_locations
    SampleLocations      = 220,
    /// VK_KHR_portability_subset
    PortabilitySubset    = 221,
    /// VK_EXT_scalar_block_layout
    ScalarBlockLayout    = 222,
    /// VK_KHR_uniform_buffer_standard_layout
    UniformBufferStandardLayout = 223,
    /// VK_EXT_global_priority
    GlobalPriority       = 224,
    /// VK_KHR_global_priority
    GlobalPriorityKhr    = 225,
    /// VK_NV_optical_flow
    OpticalFlow          = 226,
    /// VK_EXT_opacity_micromap
    OpacityMicromap      = 227,
    /// VK_NV_displacement_micromap
    DisplacementMicromap = 228,
    /// VK_EXT_nested_command_buffer
    NestedCommandBuffer  = 229,
    /// VK_EXT_dynamic_rendering_unused_attachments
    DynamicRenderingUnusedAttachments = 230,
    /// VK_EXT_legacy_dithering
    LegacyDithering      = 231,
    /// VK_EXT_depth_bias_control
    DepthBiasControl     = 232,
    /// VK_EXT_frame_boundary
    FrameBoundary        = 233,
    /// VK_EXT_map_memory_placed
    MapMemoryPlaced      = 234,
}

impl DeviceExtension {
    /// Extension name string
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Swapchain => "VK_KHR_swapchain",
            Self::DisplaySwapchain => "VK_KHR_display_swapchain",
            Self::SwapchainMaintenance1 => "VK_EXT_swapchain_maintenance1",
            Self::HdrMetadata => "VK_EXT_hdr_metadata",
            Self::FullScreenExclusive => "VK_EXT_full_screen_exclusive",
            Self::PresentId => "VK_KHR_present_id",
            Self::PresentWait => "VK_KHR_present_wait",

            Self::AccelerationStructure => "VK_KHR_acceleration_structure",
            Self::RayTracingPipeline => "VK_KHR_ray_tracing_pipeline",
            Self::RayQuery => "VK_KHR_ray_query",
            Self::RayTracingMaintenance1 => "VK_KHR_ray_tracing_maintenance1",
            Self::RayTracingPositionFetch => "VK_KHR_ray_tracing_position_fetch",
            Self::RayTracingNV => "VK_NV_ray_tracing",
            Self::RayTracingMotionBlur => "VK_NV_ray_tracing_motion_blur",
            Self::RayTracingInvocationReorder => "VK_NV_ray_tracing_invocation_reorder",

            Self::MeshShader => "VK_EXT_mesh_shader",
            Self::MeshShaderNV => "VK_NV_mesh_shader",

            Self::PushDescriptor => "VK_KHR_push_descriptor",
            Self::DescriptorIndexing => "VK_EXT_descriptor_indexing",
            Self::DescriptorBuffer => "VK_EXT_descriptor_buffer",
            Self::DescriptorUpdateTemplate => "VK_KHR_descriptor_update_template",
            Self::MutableDescriptorType => "VK_EXT_mutable_descriptor_type",

            Self::DedicatedAllocation => "VK_KHR_dedicated_allocation",
            Self::BufferDeviceAddress => "VK_KHR_buffer_device_address",
            Self::MemoryBudget => "VK_EXT_memory_budget",
            Self::MemoryPriority => "VK_EXT_memory_priority",
            Self::PageableDeviceLocalMemory => "VK_EXT_pageable_device_local_memory",
            Self::ExternalMemory => "VK_KHR_external_memory",
            Self::ExternalMemoryFd => "VK_KHR_external_memory_fd",
            Self::ExternalMemoryWin32 => "VK_KHR_external_memory_win32",
            Self::ExternalMemoryHost => "VK_EXT_external_memory_host",
            Self::ExternalMemoryDmaBuf => "VK_EXT_external_memory_dma_buf",
            Self::BindMemory2 => "VK_KHR_bind_memory2",
            Self::GetMemoryRequirements2 => "VK_KHR_get_memory_requirements2",

            Self::TimelineSemaphore => "VK_KHR_timeline_semaphore",
            Self::Synchronization2 => "VK_KHR_synchronization2",
            Self::ExternalSemaphore => "VK_KHR_external_semaphore",
            Self::ExternalSemaphoreFd => "VK_KHR_external_semaphore_fd",
            Self::ExternalSemaphoreWin32 => "VK_KHR_external_semaphore_win32",
            Self::ExternalFence => "VK_KHR_external_fence",
            Self::ExternalFenceFd => "VK_KHR_external_fence_fd",
            Self::ExternalFenceWin32 => "VK_KHR_external_fence_win32",

            Self::GraphicsPipelineLibrary => "VK_EXT_graphics_pipeline_library",
            Self::PipelineLibrary => "VK_KHR_pipeline_library",
            Self::PipelineExecutableProperties => "VK_KHR_pipeline_executable_properties",
            Self::PipelineCreationCacheControl => "VK_EXT_pipeline_creation_cache_control",
            Self::PipelineCreationFeedback => "VK_EXT_pipeline_creation_feedback",
            Self::ShaderObject => "VK_EXT_shader_object",

            Self::DynamicRendering => "VK_KHR_dynamic_rendering",
            Self::Multiview => "VK_KHR_multiview",
            Self::MultisampledRenderToSingleSampled => {
                "VK_EXT_multisampled_render_to_single_sampled"
            },
            Self::FragmentShadingRate => "VK_KHR_fragment_shading_rate",
            Self::FragmentDensityMap => "VK_EXT_fragment_density_map",
            Self::FragmentDensityMap2 => "VK_EXT_fragment_density_map2",
            Self::AttachmentFeedbackLoopLayout => "VK_EXT_attachment_feedback_loop_layout",
            Self::AttachmentFeedbackLoopDynamicState => {
                "VK_EXT_attachment_feedback_loop_dynamic_state"
            },
            Self::ImagelessFramebuffer => "VK_KHR_imageless_framebuffer",
            Self::DepthStencilResolve => "VK_KHR_depth_stencil_resolve",
            Self::SeparateDepthStencilLayouts => "VK_KHR_separate_depth_stencil_layouts",
            Self::ConservativeRasterization => "VK_EXT_conservative_rasterization",
            Self::DepthClipEnable => "VK_EXT_depth_clip_enable",
            Self::DepthClampZeroOne => "VK_EXT_depth_clamp_zero_one",

            Self::ExtendedDynamicState => "VK_EXT_extended_dynamic_state",
            Self::ExtendedDynamicState2 => "VK_EXT_extended_dynamic_state2",
            Self::ExtendedDynamicState3 => "VK_EXT_extended_dynamic_state3",
            Self::VertexInputDynamicState => "VK_EXT_vertex_input_dynamic_state",
            Self::ColorWriteEnable => "VK_EXT_color_write_enable",

            Self::SamplerYcbcrConversion => "VK_KHR_sampler_ycbcr_conversion",
            Self::Ycbcr2Plane444Formats => "VK_EXT_ycbcr_2plane_444_formats",
            Self::Formats4444 => "VK_EXT_4444_formats",
            Self::Rgba10x6Formats => "VK_EXT_rgba10x6_formats",
            Self::ImageFormatList => "VK_KHR_image_format_list",
            Self::ImageViewMinLod => "VK_EXT_image_view_min_lod",
            Self::ImageSlicedViewOf3d => "VK_EXT_image_sliced_view_of_3d",
            Self::FilterCubic => "VK_EXT_filter_cubic",
            Self::SamplerFilterMinmax => "VK_EXT_sampler_filter_minmax",
            Self::AstcDecodeMode => "VK_EXT_astc_decode_mode",
            Self::TextureCompressionAstcHdr => "VK_EXT_texture_compression_astc_hdr",
            Self::FormatFeatureFlags2 => "VK_KHR_format_feature_flags2",

            Self::ShaderFloat16Int8 => "VK_KHR_shader_float16_int8",
            Self::ShaderFloatControls => "VK_KHR_shader_float_controls",
            Self::ShaderAtomicInt64 => "VK_KHR_shader_atomic_int64",
            Self::ShaderImageAtomicInt64 => "VK_EXT_shader_image_atomic_int64",
            Self::ShaderClock => "VK_KHR_shader_clock",
            Self::ShaderSubgroupBallot => "VK_EXT_shader_subgroup_ballot",
            Self::ShaderSubgroupVote => "VK_EXT_shader_subgroup_vote",
            Self::ShaderSubgroupExtendedTypes => "VK_KHR_shader_subgroup_extended_types",
            Self::SubgroupSizeControl => "VK_EXT_subgroup_size_control",
            Self::ShaderDemoteToHelperInvocation => "VK_EXT_shader_demote_to_helper_invocation",
            Self::ShaderStencilExport => "VK_EXT_shader_stencil_export",
            Self::ShaderViewportIndexLayer => "VK_EXT_shader_viewport_index_layer",
            Self::ShaderDrawParameters => "VK_KHR_shader_draw_parameters",
            Self::ShaderNonSemanticInfo => "VK_KHR_shader_non_semantic_info",
            Self::ShaderTerminateInvocation => "VK_KHR_shader_terminate_invocation",
            Self::ShaderModuleIdentifier => "VK_EXT_shader_module_identifier",
            Self::Spirv14 => "VK_KHR_spirv_1_4",
            Self::VulkanMemoryModel => "VK_KHR_vulkan_memory_model",

            Self::DebugMarker => "VK_EXT_debug_marker",
            Self::ToolingInfo => "VK_EXT_tooling_info",
            Self::DeviceFault => "VK_EXT_device_fault",
            Self::DeviceAddressBindingReport => "VK_EXT_device_address_binding_report",
            Self::DeviceMemoryReport => "VK_EXT_device_memory_report",

            Self::VideoQueue => "VK_KHR_video_queue",
            Self::VideoDecodeQueue => "VK_KHR_video_decode_queue",
            Self::VideoEncodeQueue => "VK_KHR_video_encode_queue",
            Self::VideoDecodeH264 => "VK_KHR_video_decode_h264",
            Self::VideoDecodeH265 => "VK_KHR_video_decode_h265",
            Self::VideoEncodeH264 => "VK_KHR_video_encode_h264",
            Self::VideoEncodeH265 => "VK_KHR_video_encode_h265",
            Self::VideoDecodeAv1 => "VK_KHR_video_decode_av1",

            Self::TransformFeedback => "VK_EXT_transform_feedback",
            Self::ConditionalRendering => "VK_EXT_conditional_rendering",

            Self::DeviceGroup => "VK_KHR_device_group",
            Self::PhysicalDeviceDrm => "VK_EXT_physical_device_drm",
            Self::PciBusInfo => "VK_EXT_pci_bus_info",

            Self::Maintenance1 => "VK_KHR_maintenance1",
            Self::Maintenance2 => "VK_KHR_maintenance2",
            Self::Maintenance3 => "VK_KHR_maintenance3",
            Self::Maintenance4 => "VK_KHR_maintenance4",
            Self::Maintenance5 => "VK_KHR_maintenance5",
            Self::Maintenance6 => "VK_KHR_maintenance6",

            Self::HostQueryReset => "VK_EXT_host_query_reset",
            Self::CreateRenderpass2 => "VK_KHR_create_renderpass2",
            Self::CustomBorderColor => "VK_EXT_custom_border_color",
            Self::BorderColorSwizzle => "VK_EXT_border_color_swizzle",
            Self::Robustness2 => "VK_EXT_robustness2",
            Self::ImageRobustness => "VK_EXT_image_robustness",
            Self::InlineUniformBlock => "VK_EXT_inline_uniform_block",
            Self::PrivateData => "VK_EXT_private_data",
            Self::DrawIndirectCount => "VK_KHR_draw_indirect_count",
            Self::MultiDraw => "VK_EXT_multi_draw",
            Self::IndexTypeUint8 => "VK_EXT_index_type_uint8",
            Self::PrimitiveTopologyListRestart => "VK_EXT_primitive_topology_list_restart",
            Self::PrimitivesGeneratedQuery => "VK_EXT_primitives_generated_query",
            Self::LoadStoreOpNone => "VK_EXT_load_store_op_none",
            Self::LineRasterization => "VK_EXT_line_rasterization",
            Self::ProvokingVertex => "VK_EXT_provoking_vertex",
            Self::NonSeamlessCubeMap => "VK_EXT_non_seamless_cube_map",
            Self::CalibratedTimestamps => "VK_EXT_calibrated_timestamps",
            Self::HostImageCopy => "VK_EXT_host_image_copy",
            Self::CopyCommands2 => "VK_KHR_copy_commands2",
            Self::SampleLocations => "VK_EXT_sample_locations",
            Self::PortabilitySubset => "VK_KHR_portability_subset",
            Self::ScalarBlockLayout => "VK_EXT_scalar_block_layout",
            Self::UniformBufferStandardLayout => "VK_KHR_uniform_buffer_standard_layout",
            Self::GlobalPriority => "VK_EXT_global_priority",
            Self::GlobalPriorityKhr => "VK_KHR_global_priority",
            Self::OpticalFlow => "VK_NV_optical_flow",
            Self::OpacityMicromap => "VK_EXT_opacity_micromap",
            Self::DisplacementMicromap => "VK_NV_displacement_micromap",
            Self::NestedCommandBuffer => "VK_EXT_nested_command_buffer",
            Self::DynamicRenderingUnusedAttachments => {
                "VK_EXT_dynamic_rendering_unused_attachments"
            },
            Self::LegacyDithering => "VK_EXT_legacy_dithering",
            Self::DepthBiasControl => "VK_EXT_depth_bias_control",
            Self::FrameBoundary => "VK_EXT_frame_boundary",
            Self::MapMemoryPlaced => "VK_EXT_map_memory_placed",
        }
    }

    /// Is ray tracing extension
    #[inline]
    pub const fn is_ray_tracing(&self) -> bool {
        matches!(
            self,
            Self::AccelerationStructure
                | Self::RayTracingPipeline
                | Self::RayQuery
                | Self::RayTracingMaintenance1
                | Self::RayTracingPositionFetch
                | Self::RayTracingNV
                | Self::RayTracingMotionBlur
                | Self::RayTracingInvocationReorder
        )
    }

    /// Is video extension
    #[inline]
    pub const fn is_video(&self) -> bool {
        matches!(
            self,
            Self::VideoQueue
                | Self::VideoDecodeQueue
                | Self::VideoEncodeQueue
                | Self::VideoDecodeH264
                | Self::VideoDecodeH265
                | Self::VideoEncodeH264
                | Self::VideoEncodeH265
                | Self::VideoDecodeAv1
        )
    }

    /// Is shader extension
    #[inline]
    pub const fn is_shader(&self) -> bool {
        matches!(
            self,
            Self::ShaderFloat16Int8
                | Self::ShaderFloatControls
                | Self::ShaderAtomicInt64
                | Self::ShaderImageAtomicInt64
                | Self::ShaderClock
                | Self::ShaderSubgroupBallot
                | Self::ShaderSubgroupVote
                | Self::ShaderSubgroupExtendedTypes
                | Self::SubgroupSizeControl
                | Self::ShaderDemoteToHelperInvocation
                | Self::ShaderStencilExport
                | Self::ShaderViewportIndexLayer
                | Self::ShaderDrawParameters
                | Self::ShaderNonSemanticInfo
                | Self::ShaderTerminateInvocation
                | Self::ShaderModuleIdentifier
                | Self::ShaderObject
        )
    }

    /// Is debug extension
    #[inline]
    pub const fn is_debug(&self) -> bool {
        matches!(
            self,
            Self::DebugMarker
                | Self::ToolingInfo
                | Self::DeviceFault
                | Self::DeviceAddressBindingReport
                | Self::DeviceMemoryReport
        )
    }

    /// Is KHR extension
    #[inline]
    pub const fn is_khr(&self) -> bool {
        // Check if name starts with VK_KHR_
        matches!(
            self,
            Self::Swapchain
                | Self::DisplaySwapchain
                | Self::AccelerationStructure
                | Self::RayTracingPipeline
                | Self::RayQuery
                | Self::RayTracingMaintenance1
                | Self::RayTracingPositionFetch
                | Self::PushDescriptor
                | Self::DescriptorUpdateTemplate
                | Self::DedicatedAllocation
                | Self::BufferDeviceAddress
                | Self::ExternalMemory
                | Self::ExternalMemoryFd
                | Self::ExternalMemoryWin32
                | Self::BindMemory2
                | Self::GetMemoryRequirements2
                | Self::TimelineSemaphore
                | Self::Synchronization2
                | Self::ExternalSemaphore
                | Self::ExternalSemaphoreFd
                | Self::ExternalSemaphoreWin32
                | Self::ExternalFence
                | Self::ExternalFenceFd
                | Self::ExternalFenceWin32
                | Self::PipelineLibrary
                | Self::PipelineExecutableProperties
                | Self::DynamicRendering
                | Self::Multiview
                | Self::FragmentShadingRate
                | Self::ImagelessFramebuffer
                | Self::DepthStencilResolve
                | Self::SeparateDepthStencilLayouts
                | Self::SamplerYcbcrConversion
                | Self::ImageFormatList
                | Self::FormatFeatureFlags2
                | Self::ShaderFloat16Int8
                | Self::ShaderFloatControls
                | Self::ShaderAtomicInt64
                | Self::ShaderClock
                | Self::ShaderSubgroupExtendedTypes
                | Self::ShaderDrawParameters
                | Self::ShaderNonSemanticInfo
                | Self::ShaderTerminateInvocation
                | Self::Spirv14
                | Self::VulkanMemoryModel
                | Self::VideoQueue
                | Self::VideoDecodeQueue
                | Self::VideoEncodeQueue
                | Self::VideoDecodeH264
                | Self::VideoDecodeH265
                | Self::VideoEncodeH264
                | Self::VideoEncodeH265
                | Self::VideoDecodeAv1
                | Self::DeviceGroup
                | Self::Maintenance1
                | Self::Maintenance2
                | Self::Maintenance3
                | Self::Maintenance4
                | Self::Maintenance5
                | Self::Maintenance6
                | Self::CreateRenderpass2
                | Self::DrawIndirectCount
                | Self::CopyCommands2
                | Self::PortabilitySubset
                | Self::UniformBufferStandardLayout
                | Self::GlobalPriorityKhr
                | Self::PresentId
                | Self::PresentWait
        )
    }
}

impl fmt::Display for DeviceExtension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ============================================================================
// Extension Properties
// ============================================================================

/// Extension properties
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ExtensionProperties {
    /// Extension name (up to 256 bytes)
    pub extension_name: [u8; 256],
    /// Spec version
    pub spec_version: u32,
}

impl ExtensionProperties {
    /// Creates new
    #[inline]
    pub const fn new(name: &[u8], spec_version: u32) -> Self {
        let mut extension_name = [0u8; 256];
        let len = if name.len() < 256 { name.len() } else { 255 };
        let mut i = 0;
        while i < len {
            extension_name[i] = name[i];
            i += 1;
        }
        Self {
            extension_name,
            spec_version,
        }
    }

    /// Get name as str
    #[inline]
    pub fn name_str(&self) -> &str {
        let end = self
            .extension_name
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(256);
        core::str::from_utf8(&self.extension_name[..end]).unwrap_or("")
    }
}

impl Default for ExtensionProperties {
    fn default() -> Self {
        Self {
            extension_name: [0; 256],
            spec_version: 0,
        }
    }
}

// ============================================================================
// Extension Set
// ============================================================================

/// Extension set for tracking enabled extensions
#[derive(Clone, Debug, Default)]
pub struct ExtensionSet {
    /// Enabled instance extensions (bitfield)
    pub instance_extensions: u64,
    /// Enabled device extensions (bitfield for first 64)
    pub device_extensions_0: u64,
    /// Enabled device extensions (bitfield for 64-127)
    pub device_extensions_1: u64,
    /// Enabled device extensions (bitfield for 128-191)
    pub device_extensions_2: u64,
    /// Enabled device extensions (bitfield for 192-255)
    pub device_extensions_3: u64,
}

impl ExtensionSet {
    /// Creates empty set
    #[inline]
    pub const fn new() -> Self {
        Self {
            instance_extensions: 0,
            device_extensions_0: 0,
            device_extensions_1: 0,
            device_extensions_2: 0,
            device_extensions_3: 0,
        }
    }

    /// Enable instance extension
    #[inline]
    pub fn enable_instance(&mut self, ext: InstanceExtension) {
        self.instance_extensions |= 1 << (ext as u32);
    }

    /// Check if instance extension is enabled
    #[inline]
    pub const fn has_instance(&self, ext: InstanceExtension) -> bool {
        (self.instance_extensions & (1 << (ext as u32))) != 0
    }

    /// Enable device extension
    #[inline]
    pub fn enable_device(&mut self, ext: DeviceExtension) {
        let idx = ext as u32;
        if idx < 64 {
            self.device_extensions_0 |= 1 << idx;
        } else if idx < 128 {
            self.device_extensions_1 |= 1 << (idx - 64);
        } else if idx < 192 {
            self.device_extensions_2 |= 1 << (idx - 128);
        } else {
            self.device_extensions_3 |= 1 << (idx - 192);
        }
    }

    /// Check if device extension is enabled
    #[inline]
    pub const fn has_device(&self, ext: DeviceExtension) -> bool {
        let idx = ext as u32;
        if idx < 64 {
            (self.device_extensions_0 & (1 << idx)) != 0
        } else if idx < 128 {
            (self.device_extensions_1 & (1 << (idx - 64))) != 0
        } else if idx < 192 {
            (self.device_extensions_2 & (1 << (idx - 128))) != 0
        } else {
            (self.device_extensions_3 & (1 << (idx - 192))) != 0
        }
    }

    /// Common set for modern rendering
    pub fn modern_rendering() -> Self {
        let mut set = Self::new();
        set.enable_instance(InstanceExtension::Surface);
        set.enable_instance(InstanceExtension::DebugUtils);
        set.enable_device(DeviceExtension::Swapchain);
        set.enable_device(DeviceExtension::DynamicRendering);
        set.enable_device(DeviceExtension::Synchronization2);
        set.enable_device(DeviceExtension::TimelineSemaphore);
        set.enable_device(DeviceExtension::BufferDeviceAddress);
        set.enable_device(DeviceExtension::DescriptorIndexing);
        set
    }

    /// Ray tracing extensions
    pub fn ray_tracing() -> Self {
        let mut set = Self::modern_rendering();
        set.enable_device(DeviceExtension::AccelerationStructure);
        set.enable_device(DeviceExtension::RayTracingPipeline);
        set.enable_device(DeviceExtension::RayQuery);
        set
    }

    /// Mesh shading extensions
    pub fn mesh_shading() -> Self {
        let mut set = Self::modern_rendering();
        set.enable_device(DeviceExtension::MeshShader);
        set
    }
}

// ============================================================================
// Layer Properties
// ============================================================================

/// Layer properties
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct LayerProperties {
    /// Layer name
    pub layer_name: [u8; 256],
    /// Spec version
    pub spec_version: u32,
    /// Implementation version
    pub implementation_version: u32,
    /// Description
    pub description: [u8; 256],
}

impl LayerProperties {
    /// Get layer name as str
    #[inline]
    pub fn layer_name_str(&self) -> &str {
        let end = self.layer_name.iter().position(|&b| b == 0).unwrap_or(256);
        core::str::from_utf8(&self.layer_name[..end]).unwrap_or("")
    }

    /// Get description as str
    #[inline]
    pub fn description_str(&self) -> &str {
        let end = self.description.iter().position(|&b| b == 0).unwrap_or(256);
        core::str::from_utf8(&self.description[..end]).unwrap_or("")
    }
}

impl Default for LayerProperties {
    fn default() -> Self {
        Self {
            layer_name: [0; 256],
            spec_version: 0,
            implementation_version: 0,
            description: [0; 256],
        }
    }
}

/// Common validation layer names
pub mod layers {
    /// Khronos validation layer
    pub const KHRONOS_VALIDATION: &str = "VK_LAYER_KHRONOS_validation";
    /// Khronos synchronization layer
    pub const KHRONOS_SYNCHRONIZATION2: &str = "VK_LAYER_KHRONOS_synchronization2";
    /// Khronos profiles layer
    pub const KHRONOS_PROFILES: &str = "VK_LAYER_KHRONOS_profiles";
    /// LunarG API dump layer
    pub const LUNARG_API_DUMP: &str = "VK_LAYER_LUNARG_api_dump";
    /// LunarG device simulation layer
    pub const LUNARG_DEVICE_SIMULATION: &str = "VK_LAYER_LUNARG_device_simulation";
    /// LunarG monitor layer
    pub const LUNARG_MONITOR: &str = "VK_LAYER_LUNARG_monitor";
    /// LunarG screenshot layer
    pub const LUNARG_SCREENSHOT: &str = "VK_LAYER_LUNARG_screenshot";
}
