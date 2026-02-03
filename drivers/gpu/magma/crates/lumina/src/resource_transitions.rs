//! Resource Transitions and Barrier Helpers for Lumina
//!
//! This module provides comprehensive resource transition helpers,
//! automatic barrier insertion, and memory dependency management.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Resource State Tracking
// ============================================================================

/// Resource state for automatic barrier insertion
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ResourceState {
    /// Access flags
    pub access: AccessFlags2,
    /// Pipeline stage
    pub stage: PipelineStageFlags2,
    /// Image layout (for images)
    pub layout: ImageLayout,
    /// Queue family index
    pub queue_family: u32,
}

impl ResourceState {
    /// Undefined initial state
    pub const UNDEFINED: Self = Self {
        access: AccessFlags2::NONE,
        stage: PipelineStageFlags2::TOP_OF_PIPE,
        layout: ImageLayout::Undefined,
        queue_family: QUEUE_FAMILY_IGNORED,
    };

    /// Common state for vertex buffer
    pub const VERTEX_BUFFER: Self = Self {
        access: AccessFlags2::VERTEX_ATTRIBUTE_READ,
        stage: PipelineStageFlags2::VERTEX_ATTRIBUTE_INPUT,
        layout: ImageLayout::Undefined,
        queue_family: QUEUE_FAMILY_IGNORED,
    };

    /// Common state for index buffer
    pub const INDEX_BUFFER: Self = Self {
        access: AccessFlags2::INDEX_READ,
        stage: PipelineStageFlags2::INDEX_INPUT,
        layout: ImageLayout::Undefined,
        queue_family: QUEUE_FAMILY_IGNORED,
    };

    /// Common state for uniform buffer read
    pub const UNIFORM_READ: Self = Self {
        access: AccessFlags2::UNIFORM_READ,
        stage: PipelineStageFlags2::ALL_GRAPHICS,
        layout: ImageLayout::Undefined,
        queue_family: QUEUE_FAMILY_IGNORED,
    };

    /// Common state for storage buffer read
    pub const STORAGE_READ: Self = Self {
        access: AccessFlags2::SHADER_STORAGE_READ,
        stage: PipelineStageFlags2::ALL_GRAPHICS,
        layout: ImageLayout::Undefined,
        queue_family: QUEUE_FAMILY_IGNORED,
    };

    /// Common state for storage buffer write
    pub const STORAGE_WRITE: Self = Self {
        access: AccessFlags2::SHADER_STORAGE_WRITE,
        stage: PipelineStageFlags2::ALL_GRAPHICS,
        layout: ImageLayout::Undefined,
        queue_family: QUEUE_FAMILY_IGNORED,
    };

    /// Common state for transfer source
    pub const TRANSFER_SRC: Self = Self {
        access: AccessFlags2::TRANSFER_READ,
        stage: PipelineStageFlags2::TRANSFER,
        layout: ImageLayout::TransferSrcOptimal,
        queue_family: QUEUE_FAMILY_IGNORED,
    };

    /// Common state for transfer destination
    pub const TRANSFER_DST: Self = Self {
        access: AccessFlags2::TRANSFER_WRITE,
        stage: PipelineStageFlags2::TRANSFER,
        layout: ImageLayout::TransferDstOptimal,
        queue_family: QUEUE_FAMILY_IGNORED,
    };

    /// Color attachment write
    pub const COLOR_ATTACHMENT_WRITE: Self = Self {
        access: AccessFlags2::COLOR_ATTACHMENT_WRITE,
        stage: PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        layout: ImageLayout::ColorAttachmentOptimal,
        queue_family: QUEUE_FAMILY_IGNORED,
    };

    /// Color attachment read-write
    pub const COLOR_ATTACHMENT_RW: Self = Self {
        access: AccessFlags2(
            AccessFlags2::COLOR_ATTACHMENT_READ.0 | AccessFlags2::COLOR_ATTACHMENT_WRITE.0,
        ),
        stage: PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        layout: ImageLayout::ColorAttachmentOptimal,
        queue_family: QUEUE_FAMILY_IGNORED,
    };

    /// Depth attachment write
    pub const DEPTH_ATTACHMENT_WRITE: Self = Self {
        access: AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
        stage: PipelineStageFlags2(
            PipelineStageFlags2::EARLY_FRAGMENT_TESTS.0
                | PipelineStageFlags2::LATE_FRAGMENT_TESTS.0,
        ),
        layout: ImageLayout::DepthStencilAttachmentOptimal,
        queue_family: QUEUE_FAMILY_IGNORED,
    };

    /// Depth attachment read
    pub const DEPTH_ATTACHMENT_READ: Self = Self {
        access: AccessFlags2::DEPTH_STENCIL_ATTACHMENT_READ,
        stage: PipelineStageFlags2(
            PipelineStageFlags2::EARLY_FRAGMENT_TESTS.0
                | PipelineStageFlags2::LATE_FRAGMENT_TESTS.0,
        ),
        layout: ImageLayout::DepthStencilReadOnlyOptimal,
        queue_family: QUEUE_FAMILY_IGNORED,
    };

    /// Shader read (sampled image)
    pub const SHADER_READ: Self = Self {
        access: AccessFlags2::SHADER_SAMPLED_READ,
        stage: PipelineStageFlags2::FRAGMENT_SHADER,
        layout: ImageLayout::ShaderReadOnlyOptimal,
        queue_family: QUEUE_FAMILY_IGNORED,
    };

    /// Present
    pub const PRESENT: Self = Self {
        access: AccessFlags2::NONE,
        stage: PipelineStageFlags2::BOTTOM_OF_PIPE,
        layout: ImageLayout::PresentSrc,
        queue_family: QUEUE_FAMILY_IGNORED,
    };

    /// Creates new state
    #[inline]
    pub const fn new(access: AccessFlags2, stage: PipelineStageFlags2, layout: ImageLayout) -> Self {
        Self {
            access,
            stage,
            layout,
            queue_family: QUEUE_FAMILY_IGNORED,
        }
    }

    /// For buffer
    #[inline]
    pub const fn buffer(access: AccessFlags2, stage: PipelineStageFlags2) -> Self {
        Self {
            access,
            stage,
            layout: ImageLayout::Undefined,
            queue_family: QUEUE_FAMILY_IGNORED,
        }
    }

    /// For image
    #[inline]
    pub const fn image(
        access: AccessFlags2,
        stage: PipelineStageFlags2,
        layout: ImageLayout,
    ) -> Self {
        Self {
            access,
            stage,
            layout,
            queue_family: QUEUE_FAMILY_IGNORED,
        }
    }

    /// With queue family
    #[inline]
    pub const fn with_queue_family(mut self, queue_family: u32) -> Self {
        self.queue_family = queue_family;
        self
    }
}

impl Default for ResourceState {
    fn default() -> Self {
        Self::UNDEFINED
    }
}

/// Queue family ignored constant
pub const QUEUE_FAMILY_IGNORED: u32 = !0;

// ============================================================================
// Access Flags 2
// ============================================================================

/// Access flags (VK_KHR_synchronization2)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct AccessFlags2(pub u64);

impl AccessFlags2 {
    /// None
    pub const NONE: Self = Self(0);
    /// Indirect command read
    pub const INDIRECT_COMMAND_READ: Self = Self(1 << 0);
    /// Index read
    pub const INDEX_READ: Self = Self(1 << 1);
    /// Vertex attribute read
    pub const VERTEX_ATTRIBUTE_READ: Self = Self(1 << 2);
    /// Uniform read
    pub const UNIFORM_READ: Self = Self(1 << 3);
    /// Input attachment read
    pub const INPUT_ATTACHMENT_READ: Self = Self(1 << 4);
    /// Shader read
    pub const SHADER_READ: Self = Self(1 << 5);
    /// Shader write
    pub const SHADER_WRITE: Self = Self(1 << 6);
    /// Color attachment read
    pub const COLOR_ATTACHMENT_READ: Self = Self(1 << 7);
    /// Color attachment write
    pub const COLOR_ATTACHMENT_WRITE: Self = Self(1 << 8);
    /// Depth stencil attachment read
    pub const DEPTH_STENCIL_ATTACHMENT_READ: Self = Self(1 << 9);
    /// Depth stencil attachment write
    pub const DEPTH_STENCIL_ATTACHMENT_WRITE: Self = Self(1 << 10);
    /// Transfer read
    pub const TRANSFER_READ: Self = Self(1 << 11);
    /// Transfer write
    pub const TRANSFER_WRITE: Self = Self(1 << 12);
    /// Host read
    pub const HOST_READ: Self = Self(1 << 13);
    /// Host write
    pub const HOST_WRITE: Self = Self(1 << 14);
    /// Memory read
    pub const MEMORY_READ: Self = Self(1 << 15);
    /// Memory write
    pub const MEMORY_WRITE: Self = Self(1 << 16);
    /// Shader sampled read
    pub const SHADER_SAMPLED_READ: Self = Self(1 << 32);
    /// Shader storage read
    pub const SHADER_STORAGE_READ: Self = Self(1 << 33);
    /// Shader storage write
    pub const SHADER_STORAGE_WRITE: Self = Self(1 << 34);
    /// Video decode read
    pub const VIDEO_DECODE_READ: Self = Self(1 << 35);
    /// Video decode write
    pub const VIDEO_DECODE_WRITE: Self = Self(1 << 36);
    /// Video encode read
    pub const VIDEO_ENCODE_READ: Self = Self(1 << 37);
    /// Video encode write
    pub const VIDEO_ENCODE_WRITE: Self = Self(1 << 38);
    /// Acceleration structure read
    pub const ACCELERATION_STRUCTURE_READ: Self = Self(1 << 21);
    /// Acceleration structure write
    pub const ACCELERATION_STRUCTURE_WRITE: Self = Self(1 << 22);
    /// Fragment shading rate attachment read
    pub const FRAGMENT_SHADING_RATE_ATTACHMENT_READ: Self = Self(1 << 23);
    /// Fragment density map read
    pub const FRAGMENT_DENSITY_MAP_READ: Self = Self(1 << 24);

    /// Is read access
    #[inline]
    pub const fn is_read(&self) -> bool {
        const READ_MASK: u64 = AccessFlags2::INDIRECT_COMMAND_READ.0
            | AccessFlags2::INDEX_READ.0
            | AccessFlags2::VERTEX_ATTRIBUTE_READ.0
            | AccessFlags2::UNIFORM_READ.0
            | AccessFlags2::INPUT_ATTACHMENT_READ.0
            | AccessFlags2::SHADER_READ.0
            | AccessFlags2::COLOR_ATTACHMENT_READ.0
            | AccessFlags2::DEPTH_STENCIL_ATTACHMENT_READ.0
            | AccessFlags2::TRANSFER_READ.0
            | AccessFlags2::HOST_READ.0
            | AccessFlags2::MEMORY_READ.0
            | AccessFlags2::SHADER_SAMPLED_READ.0
            | AccessFlags2::SHADER_STORAGE_READ.0
            | AccessFlags2::ACCELERATION_STRUCTURE_READ.0
            | AccessFlags2::FRAGMENT_SHADING_RATE_ATTACHMENT_READ.0
            | AccessFlags2::FRAGMENT_DENSITY_MAP_READ.0;
        (self.0 & READ_MASK) != 0
    }

    /// Is write access
    #[inline]
    pub const fn is_write(&self) -> bool {
        const WRITE_MASK: u64 = AccessFlags2::SHADER_WRITE.0
            | AccessFlags2::COLOR_ATTACHMENT_WRITE.0
            | AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE.0
            | AccessFlags2::TRANSFER_WRITE.0
            | AccessFlags2::HOST_WRITE.0
            | AccessFlags2::MEMORY_WRITE.0
            | AccessFlags2::SHADER_STORAGE_WRITE.0
            | AccessFlags2::ACCELERATION_STRUCTURE_WRITE.0;
        (self.0 & WRITE_MASK) != 0
    }

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
// Pipeline Stage Flags 2
// ============================================================================

/// Pipeline stage flags (VK_KHR_synchronization2)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PipelineStageFlags2(pub u64);

impl PipelineStageFlags2 {
    /// None
    pub const NONE: Self = Self(0);
    /// Top of pipe
    pub const TOP_OF_PIPE: Self = Self(1 << 0);
    /// Draw indirect
    pub const DRAW_INDIRECT: Self = Self(1 << 1);
    /// Vertex input
    pub const VERTEX_INPUT: Self = Self(1 << 2);
    /// Vertex shader
    pub const VERTEX_SHADER: Self = Self(1 << 3);
    /// Tessellation control shader
    pub const TESSELLATION_CONTROL_SHADER: Self = Self(1 << 4);
    /// Tessellation evaluation shader
    pub const TESSELLATION_EVALUATION_SHADER: Self = Self(1 << 5);
    /// Geometry shader
    pub const GEOMETRY_SHADER: Self = Self(1 << 6);
    /// Fragment shader
    pub const FRAGMENT_SHADER: Self = Self(1 << 7);
    /// Early fragment tests
    pub const EARLY_FRAGMENT_TESTS: Self = Self(1 << 8);
    /// Late fragment tests
    pub const LATE_FRAGMENT_TESTS: Self = Self(1 << 9);
    /// Color attachment output
    pub const COLOR_ATTACHMENT_OUTPUT: Self = Self(1 << 10);
    /// Compute shader
    pub const COMPUTE_SHADER: Self = Self(1 << 11);
    /// Transfer
    pub const TRANSFER: Self = Self(1 << 12);
    /// Bottom of pipe
    pub const BOTTOM_OF_PIPE: Self = Self(1 << 13);
    /// Host
    pub const HOST: Self = Self(1 << 14);
    /// All graphics
    pub const ALL_GRAPHICS: Self = Self(1 << 15);
    /// All commands
    pub const ALL_COMMANDS: Self = Self(1 << 16);
    /// Copy
    pub const COPY: Self = Self(1 << 32);
    /// Resolve
    pub const RESOLVE: Self = Self(1 << 33);
    /// Blit
    pub const BLIT: Self = Self(1 << 34);
    /// Clear
    pub const CLEAR: Self = Self(1 << 35);
    /// Index input
    pub const INDEX_INPUT: Self = Self(1 << 36);
    /// Vertex attribute input
    pub const VERTEX_ATTRIBUTE_INPUT: Self = Self(1 << 37);
    /// Pre-rasterization shaders
    pub const PRE_RASTERIZATION_SHADERS: Self = Self(1 << 38);
    /// Ray tracing shader
    pub const RAY_TRACING_SHADER: Self = Self(1 << 21);
    /// Acceleration structure build
    pub const ACCELERATION_STRUCTURE_BUILD: Self = Self(1 << 25);
    /// Fragment shading rate attachment
    pub const FRAGMENT_SHADING_RATE_ATTACHMENT: Self = Self(1 << 22);
    /// Fragment density process
    pub const FRAGMENT_DENSITY_PROCESS: Self = Self(1 << 23);
    /// Task shader
    pub const TASK_SHADER: Self = Self(1 << 19);
    /// Mesh shader
    pub const MESH_SHADER: Self = Self(1 << 20);

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
// Image Layout
// ============================================================================

/// Image layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ImageLayout {
    /// Undefined
    #[default]
    Undefined = 0,
    /// General
    General = 1,
    /// Color attachment optimal
    ColorAttachmentOptimal = 2,
    /// Depth stencil attachment optimal
    DepthStencilAttachmentOptimal = 3,
    /// Depth stencil read-only optimal
    DepthStencilReadOnlyOptimal = 4,
    /// Shader read-only optimal
    ShaderReadOnlyOptimal = 5,
    /// Transfer source optimal
    TransferSrcOptimal = 6,
    /// Transfer destination optimal
    TransferDstOptimal = 7,
    /// Preinitialized
    Preinitialized = 8,
    /// Depth read-only, stencil attachment optimal
    DepthReadOnlyStencilAttachmentOptimal = 1000117000,
    /// Depth attachment, stencil read-only optimal
    DepthAttachmentStencilReadOnlyOptimal = 1000117001,
    /// Depth attachment optimal
    DepthAttachmentOptimal = 1000241000,
    /// Depth read-only optimal
    DepthReadOnlyOptimal = 1000241001,
    /// Stencil attachment optimal
    StencilAttachmentOptimal = 1000241002,
    /// Stencil read-only optimal
    StencilReadOnlyOptimal = 1000241003,
    /// Read-only optimal
    ReadOnlyOptimal = 1000314000,
    /// Attachment optimal
    AttachmentOptimal = 1000314001,
    /// Present source
    PresentSrc = 1000001002,
    /// Fragment density map optimal
    FragmentDensityMapOptimal = 1000218000,
    /// Fragment shading rate attachment optimal
    FragmentShadingRateAttachmentOptimal = 1000164003,
}

impl ImageLayout {
    /// Is read-only layout
    #[inline]
    pub const fn is_read_only(&self) -> bool {
        matches!(
            self,
            Self::ShaderReadOnlyOptimal
                | Self::TransferSrcOptimal
                | Self::DepthStencilReadOnlyOptimal
                | Self::DepthReadOnlyStencilAttachmentOptimal
                | Self::DepthReadOnlyOptimal
                | Self::StencilReadOnlyOptimal
                | Self::ReadOnlyOptimal
        )
    }

    /// Is depth layout
    #[inline]
    pub const fn is_depth(&self) -> bool {
        matches!(
            self,
            Self::DepthStencilAttachmentOptimal
                | Self::DepthStencilReadOnlyOptimal
                | Self::DepthReadOnlyStencilAttachmentOptimal
                | Self::DepthAttachmentStencilReadOnlyOptimal
                | Self::DepthAttachmentOptimal
                | Self::DepthReadOnlyOptimal
        )
    }
}

// ============================================================================
// Image Barrier
// ============================================================================

/// Image memory barrier (VK_KHR_synchronization2)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ImageMemoryBarrier2 {
    /// Source stage mask
    pub src_stage_mask: PipelineStageFlags2,
    /// Source access mask
    pub src_access_mask: AccessFlags2,
    /// Destination stage mask
    pub dst_stage_mask: PipelineStageFlags2,
    /// Destination access mask
    pub dst_access_mask: AccessFlags2,
    /// Old layout
    pub old_layout: ImageLayout,
    /// New layout
    pub new_layout: ImageLayout,
    /// Source queue family index
    pub src_queue_family_index: u32,
    /// Destination queue family index
    pub dst_queue_family_index: u32,
    /// Image handle
    pub image: u64,
    /// Subresource range
    pub subresource_range: ImageSubresourceRange,
}

impl ImageMemoryBarrier2 {
    /// Creates new barrier
    #[inline]
    pub const fn new(image: u64) -> Self {
        Self {
            src_stage_mask: PipelineStageFlags2::NONE,
            src_access_mask: AccessFlags2::NONE,
            dst_stage_mask: PipelineStageFlags2::NONE,
            dst_access_mask: AccessFlags2::NONE,
            old_layout: ImageLayout::Undefined,
            new_layout: ImageLayout::Undefined,
            src_queue_family_index: QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: QUEUE_FAMILY_IGNORED,
            image,
            subresource_range: ImageSubresourceRange::COLOR_ALL,
        }
    }

    /// Layout transition
    #[inline]
    pub const fn layout_transition(
        image: u64,
        old_layout: ImageLayout,
        new_layout: ImageLayout,
        src_state: ResourceState,
        dst_state: ResourceState,
    ) -> Self {
        Self {
            src_stage_mask: src_state.stage,
            src_access_mask: src_state.access,
            dst_stage_mask: dst_state.stage,
            dst_access_mask: dst_state.access,
            old_layout,
            new_layout,
            src_queue_family_index: QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: QUEUE_FAMILY_IGNORED,
            image,
            subresource_range: ImageSubresourceRange::COLOR_ALL,
        }
    }

    /// From resource states
    #[inline]
    pub const fn from_states(image: u64, src: ResourceState, dst: ResourceState) -> Self {
        Self {
            src_stage_mask: src.stage,
            src_access_mask: src.access,
            dst_stage_mask: dst.stage,
            dst_access_mask: dst.access,
            old_layout: src.layout,
            new_layout: dst.layout,
            src_queue_family_index: src.queue_family,
            dst_queue_family_index: dst.queue_family,
            image,
            subresource_range: ImageSubresourceRange::COLOR_ALL,
        }
    }

    /// With subresource range
    #[inline]
    pub const fn with_subresource(mut self, range: ImageSubresourceRange) -> Self {
        self.subresource_range = range;
        self
    }

    /// Undefined to color attachment
    #[inline]
    pub const fn undefined_to_color_attachment(image: u64) -> Self {
        Self::from_states(image, ResourceState::UNDEFINED, ResourceState::COLOR_ATTACHMENT_WRITE)
    }

    /// Undefined to depth attachment
    #[inline]
    pub const fn undefined_to_depth_attachment(image: u64) -> Self {
        Self::from_states(image, ResourceState::UNDEFINED, ResourceState::DEPTH_ATTACHMENT_WRITE)
            .with_subresource(ImageSubresourceRange::DEPTH_ALL)
    }

    /// Undefined to transfer destination
    #[inline]
    pub const fn undefined_to_transfer_dst(image: u64) -> Self {
        Self::from_states(image, ResourceState::UNDEFINED, ResourceState::TRANSFER_DST)
    }

    /// Transfer destination to shader read
    #[inline]
    pub const fn transfer_dst_to_shader_read(image: u64) -> Self {
        Self::from_states(image, ResourceState::TRANSFER_DST, ResourceState::SHADER_READ)
    }

    /// Color attachment to shader read
    #[inline]
    pub const fn color_attachment_to_shader_read(image: u64) -> Self {
        Self::from_states(image, ResourceState::COLOR_ATTACHMENT_WRITE, ResourceState::SHADER_READ)
    }

    /// Color attachment to present
    #[inline]
    pub const fn color_attachment_to_present(image: u64) -> Self {
        Self::from_states(image, ResourceState::COLOR_ATTACHMENT_WRITE, ResourceState::PRESENT)
    }

    /// Present to color attachment
    #[inline]
    pub const fn present_to_color_attachment(image: u64) -> Self {
        Self::from_states(image, ResourceState::PRESENT, ResourceState::COLOR_ATTACHMENT_WRITE)
    }
}

impl Default for ImageMemoryBarrier2 {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Image subresource range
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ImageSubresourceRange {
    /// Aspect mask
    pub aspect_mask: ImageAspectFlags,
    /// Base mip level
    pub base_mip_level: u32,
    /// Level count
    pub level_count: u32,
    /// Base array layer
    pub base_array_layer: u32,
    /// Layer count
    pub layer_count: u32,
}

impl ImageSubresourceRange {
    /// All remaining
    pub const REMAINING_MIP_LEVELS: u32 = !0;
    pub const REMAINING_ARRAY_LAYERS: u32 = !0;

    /// All color mips and layers
    pub const COLOR_ALL: Self = Self {
        aspect_mask: ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: Self::REMAINING_MIP_LEVELS,
        base_array_layer: 0,
        layer_count: Self::REMAINING_ARRAY_LAYERS,
    };

    /// All depth mips and layers
    pub const DEPTH_ALL: Self = Self {
        aspect_mask: ImageAspectFlags::DEPTH,
        base_mip_level: 0,
        level_count: Self::REMAINING_MIP_LEVELS,
        base_array_layer: 0,
        layer_count: Self::REMAINING_ARRAY_LAYERS,
    };

    /// All depth-stencil mips and layers
    pub const DEPTH_STENCIL_ALL: Self = Self {
        aspect_mask: ImageAspectFlags(
            ImageAspectFlags::DEPTH.0 | ImageAspectFlags::STENCIL.0,
        ),
        base_mip_level: 0,
        level_count: Self::REMAINING_MIP_LEVELS,
        base_array_layer: 0,
        layer_count: Self::REMAINING_ARRAY_LAYERS,
    };

    /// Creates new range
    #[inline]
    pub const fn new(aspect: ImageAspectFlags, mip: u32, mip_count: u32, layer: u32, layer_count: u32) -> Self {
        Self {
            aspect_mask: aspect,
            base_mip_level: mip,
            level_count: mip_count,
            base_array_layer: layer,
            layer_count,
        }
    }

    /// Single mip level
    #[inline]
    pub const fn single_mip(aspect: ImageAspectFlags, mip: u32) -> Self {
        Self {
            aspect_mask: aspect,
            base_mip_level: mip,
            level_count: 1,
            base_array_layer: 0,
            layer_count: Self::REMAINING_ARRAY_LAYERS,
        }
    }

    /// Single layer
    #[inline]
    pub const fn single_layer(aspect: ImageAspectFlags, layer: u32) -> Self {
        Self {
            aspect_mask: aspect,
            base_mip_level: 0,
            level_count: Self::REMAINING_MIP_LEVELS,
            base_array_layer: layer,
            layer_count: 1,
        }
    }
}

impl Default for ImageSubresourceRange {
    fn default() -> Self {
        Self::COLOR_ALL
    }
}

/// Image aspect flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ImageAspectFlags(pub u32);

impl ImageAspectFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Color
    pub const COLOR: Self = Self(1 << 0);
    /// Depth
    pub const DEPTH: Self = Self(1 << 1);
    /// Stencil
    pub const STENCIL: Self = Self(1 << 2);
    /// Metadata
    pub const METADATA: Self = Self(1 << 3);

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
// Buffer Barrier
// ============================================================================

/// Buffer memory barrier (VK_KHR_synchronization2)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct BufferMemoryBarrier2 {
    /// Source stage mask
    pub src_stage_mask: PipelineStageFlags2,
    /// Source access mask
    pub src_access_mask: AccessFlags2,
    /// Destination stage mask
    pub dst_stage_mask: PipelineStageFlags2,
    /// Destination access mask
    pub dst_access_mask: AccessFlags2,
    /// Source queue family index
    pub src_queue_family_index: u32,
    /// Destination queue family index
    pub dst_queue_family_index: u32,
    /// Buffer handle
    pub buffer: u64,
    /// Offset
    pub offset: u64,
    /// Size
    pub size: u64,
}

impl BufferMemoryBarrier2 {
    /// Whole buffer
    pub const WHOLE_SIZE: u64 = !0;

    /// Creates new barrier
    #[inline]
    pub const fn new(buffer: u64) -> Self {
        Self {
            src_stage_mask: PipelineStageFlags2::NONE,
            src_access_mask: AccessFlags2::NONE,
            dst_stage_mask: PipelineStageFlags2::NONE,
            dst_access_mask: AccessFlags2::NONE,
            src_queue_family_index: QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: QUEUE_FAMILY_IGNORED,
            buffer,
            offset: 0,
            size: Self::WHOLE_SIZE,
        }
    }

    /// From resource states
    #[inline]
    pub const fn from_states(buffer: u64, src: ResourceState, dst: ResourceState) -> Self {
        Self {
            src_stage_mask: src.stage,
            src_access_mask: src.access,
            dst_stage_mask: dst.stage,
            dst_access_mask: dst.access,
            src_queue_family_index: src.queue_family,
            dst_queue_family_index: dst.queue_family,
            buffer,
            offset: 0,
            size: Self::WHOLE_SIZE,
        }
    }

    /// With range
    #[inline]
    pub const fn with_range(mut self, offset: u64, size: u64) -> Self {
        self.offset = offset;
        self.size = size;
        self
    }

    /// Transfer to vertex buffer
    #[inline]
    pub const fn transfer_to_vertex(buffer: u64) -> Self {
        Self::from_states(buffer, ResourceState::TRANSFER_DST, ResourceState::VERTEX_BUFFER)
    }

    /// Transfer to index buffer
    #[inline]
    pub const fn transfer_to_index(buffer: u64) -> Self {
        Self::from_states(buffer, ResourceState::TRANSFER_DST, ResourceState::INDEX_BUFFER)
    }

    /// Transfer to uniform buffer
    #[inline]
    pub const fn transfer_to_uniform(buffer: u64) -> Self {
        Self::from_states(buffer, ResourceState::TRANSFER_DST, ResourceState::UNIFORM_READ)
    }

    /// Storage write to storage read
    #[inline]
    pub const fn storage_write_to_read(buffer: u64) -> Self {
        Self::from_states(buffer, ResourceState::STORAGE_WRITE, ResourceState::STORAGE_READ)
    }
}

impl Default for BufferMemoryBarrier2 {
    fn default() -> Self {
        Self::new(0)
    }
}

// ============================================================================
// Memory Barrier
// ============================================================================

/// Global memory barrier (VK_KHR_synchronization2)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct MemoryBarrier2 {
    /// Source stage mask
    pub src_stage_mask: PipelineStageFlags2,
    /// Source access mask
    pub src_access_mask: AccessFlags2,
    /// Destination stage mask
    pub dst_stage_mask: PipelineStageFlags2,
    /// Destination access mask
    pub dst_access_mask: AccessFlags2,
}

impl MemoryBarrier2 {
    /// Creates new barrier
    #[inline]
    pub const fn new(
        src_stage: PipelineStageFlags2,
        src_access: AccessFlags2,
        dst_stage: PipelineStageFlags2,
        dst_access: AccessFlags2,
    ) -> Self {
        Self {
            src_stage_mask: src_stage,
            src_access_mask: src_access,
            dst_stage_mask: dst_stage,
            dst_access_mask: dst_access,
        }
    }

    /// Full pipeline barrier
    pub const FULL: Self = Self {
        src_stage_mask: PipelineStageFlags2::ALL_COMMANDS,
        src_access_mask: AccessFlags2::MEMORY_WRITE,
        dst_stage_mask: PipelineStageFlags2::ALL_COMMANDS,
        dst_access_mask: AccessFlags2(AccessFlags2::MEMORY_READ.0 | AccessFlags2::MEMORY_WRITE.0),
    };

    /// Compute write to graphics read
    pub const COMPUTE_TO_GRAPHICS: Self = Self {
        src_stage_mask: PipelineStageFlags2::COMPUTE_SHADER,
        src_access_mask: AccessFlags2::SHADER_STORAGE_WRITE,
        dst_stage_mask: PipelineStageFlags2::ALL_GRAPHICS,
        dst_access_mask: AccessFlags2::SHADER_STORAGE_READ,
    };

    /// Graphics to compute
    pub const GRAPHICS_TO_COMPUTE: Self = Self {
        src_stage_mask: PipelineStageFlags2::ALL_GRAPHICS,
        src_access_mask: AccessFlags2::SHADER_STORAGE_WRITE,
        dst_stage_mask: PipelineStageFlags2::COMPUTE_SHADER,
        dst_access_mask: AccessFlags2::SHADER_STORAGE_READ,
    };
}

// ============================================================================
// Dependency Info
// ============================================================================

/// Dependency info for pipeline barrier (VK_KHR_synchronization2)
#[derive(Clone, Debug, Default)]
pub struct DependencyInfo {
    /// Dependency flags
    pub dependency_flags: DependencyFlags,
    /// Memory barriers
    pub memory_barriers: Vec<MemoryBarrier2>,
    /// Buffer memory barriers
    pub buffer_memory_barriers: Vec<BufferMemoryBarrier2>,
    /// Image memory barriers
    pub image_memory_barriers: Vec<ImageMemoryBarrier2>,
}

impl DependencyInfo {
    /// Creates new dependency info
    pub fn new() -> Self {
        Self {
            dependency_flags: DependencyFlags::NONE,
            memory_barriers: Vec::new(),
            buffer_memory_barriers: Vec::new(),
            image_memory_barriers: Vec::new(),
        }
    }

    /// Add memory barrier
    #[inline]
    pub fn memory_barrier(mut self, barrier: MemoryBarrier2) -> Self {
        self.memory_barriers.push(barrier);
        self
    }

    /// Add buffer barrier
    #[inline]
    pub fn buffer_barrier(mut self, barrier: BufferMemoryBarrier2) -> Self {
        self.buffer_memory_barriers.push(barrier);
        self
    }

    /// Add image barrier
    #[inline]
    pub fn image_barrier(mut self, barrier: ImageMemoryBarrier2) -> Self {
        self.image_memory_barriers.push(barrier);
        self
    }

    /// With flags
    #[inline]
    pub fn with_flags(mut self, flags: DependencyFlags) -> Self {
        self.dependency_flags = flags;
        self
    }

    /// By region
    #[inline]
    pub fn by_region(mut self) -> Self {
        self.dependency_flags = DependencyFlags::BY_REGION;
        self
    }

    /// Is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.memory_barriers.is_empty()
            && self.buffer_memory_barriers.is_empty()
            && self.image_memory_barriers.is_empty()
    }
}

/// Dependency flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DependencyFlags(pub u32);

impl DependencyFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// By region
    pub const BY_REGION: Self = Self(1 << 0);
    /// Device group
    pub const DEVICE_GROUP: Self = Self(1 << 2);
    /// View local
    pub const VIEW_LOCAL: Self = Self(1 << 1);

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
// Common Transitions
// ============================================================================

/// Common transition helper
pub struct Transitions;

impl Transitions {
    /// Undefined to shader read (for texture upload)
    pub fn texture_upload(image: u64) -> DependencyInfo {
        DependencyInfo::new()
            .image_barrier(ImageMemoryBarrier2::undefined_to_transfer_dst(image))
    }

    /// After texture upload, transition to shader read
    pub fn texture_ready(image: u64) -> DependencyInfo {
        DependencyInfo::new()
            .image_barrier(ImageMemoryBarrier2::transfer_dst_to_shader_read(image))
    }

    /// Prepare color attachment
    pub fn color_attachment_init(image: u64) -> DependencyInfo {
        DependencyInfo::new()
            .image_barrier(ImageMemoryBarrier2::undefined_to_color_attachment(image))
    }

    /// Prepare depth attachment
    pub fn depth_attachment_init(image: u64) -> DependencyInfo {
        DependencyInfo::new()
            .image_barrier(ImageMemoryBarrier2::undefined_to_depth_attachment(image))
    }

    /// Color attachment to present
    pub fn present(image: u64) -> DependencyInfo {
        DependencyInfo::new()
            .image_barrier(ImageMemoryBarrier2::color_attachment_to_present(image))
    }

    /// After present, prepare for rendering
    pub fn after_present(image: u64) -> DependencyInfo {
        DependencyInfo::new()
            .image_barrier(ImageMemoryBarrier2::present_to_color_attachment(image))
    }

    /// Buffer ready for vertex usage after upload
    pub fn vertex_buffer_ready(buffer: u64) -> DependencyInfo {
        DependencyInfo::new()
            .buffer_barrier(BufferMemoryBarrier2::transfer_to_vertex(buffer))
    }

    /// Buffer ready for index usage after upload
    pub fn index_buffer_ready(buffer: u64) -> DependencyInfo {
        DependencyInfo::new()
            .buffer_barrier(BufferMemoryBarrier2::transfer_to_index(buffer))
    }

    /// Compute to graphics synchronization
    pub fn compute_to_graphics() -> DependencyInfo {
        DependencyInfo::new()
            .memory_barrier(MemoryBarrier2::COMPUTE_TO_GRAPHICS)
    }

    /// Full pipeline barrier
    pub fn full_barrier() -> DependencyInfo {
        DependencyInfo::new()
            .memory_barrier(MemoryBarrier2::FULL)
    }
}
