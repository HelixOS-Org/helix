//! Resource Barriers for Lumina
//!
//! This module provides memory barrier and resource transition types
//! for synchronizing GPU operations and managing resource state.

// ============================================================================
// Memory Barrier
// ============================================================================

/// Global memory barrier
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryBarrier {
    /// Source access mask
    pub src_access: AccessFlags,
    /// Destination access mask
    pub dst_access: AccessFlags,
}

impl MemoryBarrier {
    /// Creates new memory barrier
    #[inline]
    pub const fn new(src_access: AccessFlags, dst_access: AccessFlags) -> Self {
        Self {
            src_access,
            dst_access,
        }
    }

    /// Full barrier (all accesses)
    pub const FULL: Self = Self {
        src_access: AccessFlags::ALL,
        dst_access: AccessFlags::ALL,
    };

    /// Shader read after write
    pub const SHADER_READ_AFTER_WRITE: Self = Self {
        src_access: AccessFlags::SHADER_WRITE,
        dst_access: AccessFlags::SHADER_READ,
    };

    /// Transfer read after write
    pub const TRANSFER_READ_AFTER_WRITE: Self = Self {
        src_access: AccessFlags::TRANSFER_WRITE,
        dst_access: AccessFlags::TRANSFER_READ,
    };

    /// Host read after write
    pub const HOST_READ_AFTER_WRITE: Self = Self {
        src_access: AccessFlags::HOST_WRITE,
        dst_access: AccessFlags::HOST_READ,
    };

    /// Color attachment after shader write
    pub const COLOR_AFTER_SHADER_WRITE: Self = Self {
        src_access: AccessFlags::SHADER_WRITE,
        dst_access: AccessFlags::COLOR_ATTACHMENT_WRITE,
    };
}

impl Default for MemoryBarrier {
    fn default() -> Self {
        Self::FULL
    }
}

// ============================================================================
// Buffer Memory Barrier
// ============================================================================

/// Buffer memory barrier
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferMemoryBarrier {
    /// Source access mask
    pub src_access: AccessFlags,
    /// Destination access mask
    pub dst_access: AccessFlags,
    /// Source queue family index
    pub src_queue_family: u32,
    /// Destination queue family index
    pub dst_queue_family: u32,
    /// Buffer handle
    pub buffer: u64,
    /// Offset
    pub offset: u64,
    /// Size (WHOLE_SIZE for entire buffer)
    pub size: u64,
}

impl BufferMemoryBarrier {
    /// Whole size constant
    pub const WHOLE_SIZE: u64 = u64::MAX;
    /// Ignored queue family
    pub const QUEUE_FAMILY_IGNORED: u32 = u32::MAX;

    /// Creates new barrier
    #[inline]
    pub const fn new(buffer: u64, src_access: AccessFlags, dst_access: AccessFlags) -> Self {
        Self {
            src_access,
            dst_access,
            src_queue_family: Self::QUEUE_FAMILY_IGNORED,
            dst_queue_family: Self::QUEUE_FAMILY_IGNORED,
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

    /// With queue family transfer
    #[inline]
    pub const fn with_queue_transfer(mut self, src: u32, dst: u32) -> Self {
        self.src_queue_family = src;
        self.dst_queue_family = dst;
        self
    }

    /// Shader read after write
    #[inline]
    pub const fn shader_read_after_write(buffer: u64) -> Self {
        Self::new(buffer, AccessFlags::SHADER_WRITE, AccessFlags::SHADER_READ)
    }

    /// Transfer to shader
    #[inline]
    pub const fn transfer_to_shader(buffer: u64) -> Self {
        Self::new(
            buffer,
            AccessFlags::TRANSFER_WRITE,
            AccessFlags::SHADER_READ,
        )
    }

    /// Shader to transfer
    #[inline]
    pub const fn shader_to_transfer(buffer: u64) -> Self {
        Self::new(
            buffer,
            AccessFlags::SHADER_WRITE,
            AccessFlags::TRANSFER_READ,
        )
    }

    /// Shader to host
    #[inline]
    pub const fn shader_to_host(buffer: u64) -> Self {
        Self::new(buffer, AccessFlags::SHADER_WRITE, AccessFlags::HOST_READ)
    }

    /// Host to shader
    #[inline]
    pub const fn host_to_shader(buffer: u64) -> Self {
        Self::new(buffer, AccessFlags::HOST_WRITE, AccessFlags::SHADER_READ)
    }

    /// Index buffer after write
    #[inline]
    pub const fn index_after_write(buffer: u64) -> Self {
        Self::new(buffer, AccessFlags::SHADER_WRITE, AccessFlags::INDEX_READ)
    }

    /// Vertex buffer after write
    #[inline]
    pub const fn vertex_after_write(buffer: u64) -> Self {
        Self::new(
            buffer,
            AccessFlags::SHADER_WRITE,
            AccessFlags::VERTEX_ATTRIBUTE_READ,
        )
    }

    /// Uniform buffer after write
    #[inline]
    pub const fn uniform_after_write(buffer: u64) -> Self {
        Self::new(
            buffer,
            AccessFlags::TRANSFER_WRITE,
            AccessFlags::UNIFORM_READ,
        )
    }

    /// Indirect buffer after write
    #[inline]
    pub const fn indirect_after_write(buffer: u64) -> Self {
        Self::new(
            buffer,
            AccessFlags::SHADER_WRITE,
            AccessFlags::INDIRECT_COMMAND_READ,
        )
    }
}

impl Default for BufferMemoryBarrier {
    fn default() -> Self {
        Self::new(0, AccessFlags::NONE, AccessFlags::NONE)
    }
}

// ============================================================================
// Image Memory Barrier
// ============================================================================

/// Image memory barrier
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageMemoryBarrier {
    /// Source access mask
    pub src_access: AccessFlags,
    /// Destination access mask
    pub dst_access: AccessFlags,
    /// Old layout
    pub old_layout: ImageLayout,
    /// New layout
    pub new_layout: ImageLayout,
    /// Source queue family index
    pub src_queue_family: u32,
    /// Destination queue family index
    pub dst_queue_family: u32,
    /// Image handle
    pub image: u64,
    /// Subresource range
    pub subresource_range: ImageSubresourceRange,
}

impl ImageMemoryBarrier {
    /// Ignored queue family
    pub const QUEUE_FAMILY_IGNORED: u32 = u32::MAX;

    /// Creates new barrier
    #[inline]
    pub const fn new(
        image: u64,
        old_layout: ImageLayout,
        new_layout: ImageLayout,
        src_access: AccessFlags,
        dst_access: AccessFlags,
    ) -> Self {
        Self {
            src_access,
            dst_access,
            old_layout,
            new_layout,
            src_queue_family: Self::QUEUE_FAMILY_IGNORED,
            dst_queue_family: Self::QUEUE_FAMILY_IGNORED,
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

    /// With queue family transfer
    #[inline]
    pub const fn with_queue_transfer(mut self, src: u32, dst: u32) -> Self {
        self.src_queue_family = src;
        self.dst_queue_family = dst;
        self
    }

    /// Undefined to transfer destination
    #[inline]
    pub const fn undefined_to_transfer_dst(image: u64) -> Self {
        Self::new(
            image,
            ImageLayout::Undefined,
            ImageLayout::TransferDstOptimal,
            AccessFlags::NONE,
            AccessFlags::TRANSFER_WRITE,
        )
    }

    /// Transfer destination to shader read
    #[inline]
    pub const fn transfer_dst_to_shader_read(image: u64) -> Self {
        Self::new(
            image,
            ImageLayout::TransferDstOptimal,
            ImageLayout::ShaderReadOnlyOptimal,
            AccessFlags::TRANSFER_WRITE,
            AccessFlags::SHADER_READ,
        )
    }

    /// Undefined to color attachment
    #[inline]
    pub const fn undefined_to_color_attachment(image: u64) -> Self {
        Self::new(
            image,
            ImageLayout::Undefined,
            ImageLayout::ColorAttachmentOptimal,
            AccessFlags::NONE,
            AccessFlags::COLOR_ATTACHMENT_WRITE,
        )
    }

    /// Color attachment to shader read
    #[inline]
    pub const fn color_attachment_to_shader_read(image: u64) -> Self {
        Self::new(
            image,
            ImageLayout::ColorAttachmentOptimal,
            ImageLayout::ShaderReadOnlyOptimal,
            AccessFlags::COLOR_ATTACHMENT_WRITE,
            AccessFlags::SHADER_READ,
        )
    }

    /// Color attachment to present
    #[inline]
    pub const fn color_attachment_to_present(image: u64) -> Self {
        Self::new(
            image,
            ImageLayout::ColorAttachmentOptimal,
            ImageLayout::PresentSrc,
            AccessFlags::COLOR_ATTACHMENT_WRITE,
            AccessFlags::NONE,
        )
    }

    /// Present to color attachment
    #[inline]
    pub const fn present_to_color_attachment(image: u64) -> Self {
        Self::new(
            image,
            ImageLayout::PresentSrc,
            ImageLayout::ColorAttachmentOptimal,
            AccessFlags::NONE,
            AccessFlags::COLOR_ATTACHMENT_WRITE,
        )
    }

    /// Undefined to depth-stencil attachment
    #[inline]
    pub const fn undefined_to_depth_stencil(image: u64) -> Self {
        Self::new(
            image,
            ImageLayout::Undefined,
            ImageLayout::DepthStencilAttachmentOptimal,
            AccessFlags::NONE,
            AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
        )
        .with_subresource(ImageSubresourceRange::DEPTH_ALL)
    }

    /// Depth-stencil to shader read
    #[inline]
    pub const fn depth_stencil_to_shader_read(image: u64) -> Self {
        Self::new(
            image,
            ImageLayout::DepthStencilAttachmentOptimal,
            ImageLayout::ShaderReadOnlyOptimal,
            AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            AccessFlags::SHADER_READ,
        )
        .with_subresource(ImageSubresourceRange::DEPTH_ALL)
    }

    /// Shader read to transfer source
    #[inline]
    pub const fn shader_read_to_transfer_src(image: u64) -> Self {
        Self::new(
            image,
            ImageLayout::ShaderReadOnlyOptimal,
            ImageLayout::TransferSrcOptimal,
            AccessFlags::SHADER_READ,
            AccessFlags::TRANSFER_READ,
        )
    }

    /// Transfer source to shader read
    #[inline]
    pub const fn transfer_src_to_shader_read(image: u64) -> Self {
        Self::new(
            image,
            ImageLayout::TransferSrcOptimal,
            ImageLayout::ShaderReadOnlyOptimal,
            AccessFlags::TRANSFER_READ,
            AccessFlags::SHADER_READ,
        )
    }

    /// General to storage
    #[inline]
    pub const fn general_to_storage(image: u64) -> Self {
        Self::new(
            image,
            ImageLayout::General,
            ImageLayout::General,
            AccessFlags::SHADER_READ,
            AccessFlags::SHADER_WRITE,
        )
    }

    /// Storage to general read
    #[inline]
    pub const fn storage_to_general_read(image: u64) -> Self {
        Self::new(
            image,
            ImageLayout::General,
            ImageLayout::General,
            AccessFlags::SHADER_WRITE,
            AccessFlags::SHADER_READ,
        )
    }
}

impl Default for ImageMemoryBarrier {
    fn default() -> Self {
        Self::new(
            0,
            ImageLayout::Undefined,
            ImageLayout::General,
            AccessFlags::NONE,
            AccessFlags::NONE,
        )
    }
}

// ============================================================================
// Access Flags
// ============================================================================

/// Access flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct AccessFlags(pub u64);

impl AccessFlags {
    /// No access
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
    /// Depth-stencil attachment read
    pub const DEPTH_STENCIL_ATTACHMENT_READ: Self = Self(1 << 9);
    /// Depth-stencil attachment write
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
    /// Transform feedback write
    pub const TRANSFORM_FEEDBACK_WRITE: Self = Self(1 << 17);
    /// Transform feedback counter read
    pub const TRANSFORM_FEEDBACK_COUNTER_READ: Self = Self(1 << 18);
    /// Transform feedback counter write
    pub const TRANSFORM_FEEDBACK_COUNTER_WRITE: Self = Self(1 << 19);
    /// Conditional rendering read
    pub const CONDITIONAL_RENDERING_READ: Self = Self(1 << 20);
    /// Color attachment read non-coherent
    pub const COLOR_ATTACHMENT_READ_NONCOHERENT: Self = Self(1 << 21);
    /// Acceleration structure read
    pub const ACCELERATION_STRUCTURE_READ: Self = Self(1 << 22);
    /// Acceleration structure write
    pub const ACCELERATION_STRUCTURE_WRITE: Self = Self(1 << 23);
    /// Fragment shading rate attachment read
    pub const FRAGMENT_SHADING_RATE_ATTACHMENT_READ: Self = Self(1 << 24);
    /// Fragment density map read
    pub const FRAGMENT_DENSITY_MAP_READ: Self = Self(1 << 25);
    /// Command preprocess read (NV)
    pub const COMMAND_PREPROCESS_READ_NV: Self = Self(1 << 26);
    /// Command preprocess write (NV)
    pub const COMMAND_PREPROCESS_WRITE_NV: Self = Self(1 << 27);
    /// Video decode read
    pub const VIDEO_DECODE_READ: Self = Self(1 << 28);
    /// Video decode write
    pub const VIDEO_DECODE_WRITE: Self = Self(1 << 29);
    /// Video encode read
    pub const VIDEO_ENCODE_READ: Self = Self(1 << 30);
    /// Video encode write
    pub const VIDEO_ENCODE_WRITE: Self = Self(1 << 31);
    /// Optical flow read
    pub const OPTICAL_FLOW_READ: Self = Self(1 << 32);
    /// Optical flow write
    pub const OPTICAL_FLOW_WRITE: Self = Self(1 << 33);
    /// Micromap read
    pub const MICROMAP_READ: Self = Self(1 << 34);
    /// Micromap write
    pub const MICROMAP_WRITE: Self = Self(1 << 35);
    /// Descriptor buffer read
    pub const DESCRIPTOR_BUFFER_READ: Self = Self(1 << 36);
    /// Shader binding table read
    pub const SHADER_BINDING_TABLE_READ: Self = Self(1 << 37);

    /// All reads
    pub const ALL_READS: Self = Self(
        Self::INDIRECT_COMMAND_READ.0
            | Self::INDEX_READ.0
            | Self::VERTEX_ATTRIBUTE_READ.0
            | Self::UNIFORM_READ.0
            | Self::INPUT_ATTACHMENT_READ.0
            | Self::SHADER_READ.0
            | Self::COLOR_ATTACHMENT_READ.0
            | Self::DEPTH_STENCIL_ATTACHMENT_READ.0
            | Self::TRANSFER_READ.0
            | Self::HOST_READ.0
            | Self::MEMORY_READ.0,
    );

    /// All writes
    pub const ALL_WRITES: Self = Self(
        Self::SHADER_WRITE.0
            | Self::COLOR_ATTACHMENT_WRITE.0
            | Self::DEPTH_STENCIL_ATTACHMENT_WRITE.0
            | Self::TRANSFER_WRITE.0
            | Self::HOST_WRITE.0
            | Self::MEMORY_WRITE.0,
    );

    /// All access
    pub const ALL: Self = Self(Self::ALL_READS.0 | Self::ALL_WRITES.0);

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

    /// Intersection
    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Is read-only
    #[inline]
    pub const fn is_read_only(&self) -> bool {
        (self.0 & Self::ALL_WRITES.0) == 0
    }

    /// Is write
    #[inline]
    pub const fn is_write(&self) -> bool {
        (self.0 & Self::ALL_WRITES.0) != 0
    }
}

// ============================================================================
// Pipeline Stage Flags
// ============================================================================

/// Pipeline stage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PipelineStageFlags(pub u64);

impl PipelineStageFlags {
    /// No stage
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
    pub const COPY: Self = Self(1 << 17);
    /// Resolve
    pub const RESOLVE: Self = Self(1 << 18);
    /// Blit
    pub const BLIT: Self = Self(1 << 19);
    /// Clear
    pub const CLEAR: Self = Self(1 << 20);
    /// Index input
    pub const INDEX_INPUT: Self = Self(1 << 21);
    /// Vertex attribute input
    pub const VERTEX_ATTRIBUTE_INPUT: Self = Self(1 << 22);
    /// Pre-rasterization shaders
    pub const PRE_RASTERIZATION_SHADERS: Self = Self(1 << 23);
    /// Video decode
    pub const VIDEO_DECODE: Self = Self(1 << 24);
    /// Video encode
    pub const VIDEO_ENCODE: Self = Self(1 << 25);
    /// Transform feedback
    pub const TRANSFORM_FEEDBACK: Self = Self(1 << 26);
    /// Conditional rendering
    pub const CONDITIONAL_RENDERING: Self = Self(1 << 27);
    /// Acceleration structure build
    pub const ACCELERATION_STRUCTURE_BUILD: Self = Self(1 << 28);
    /// Ray tracing shader
    pub const RAY_TRACING_SHADER: Self = Self(1 << 29);
    /// Fragment shading rate attachment
    pub const FRAGMENT_SHADING_RATE_ATTACHMENT: Self = Self(1 << 30);
    /// Fragment density process
    pub const FRAGMENT_DENSITY_PROCESS: Self = Self(1 << 31);
    /// Task shader
    pub const TASK_SHADER: Self = Self(1 << 32);
    /// Mesh shader
    pub const MESH_SHADER: Self = Self(1 << 33);
    /// Subpass shading
    pub const SUBPASS_SHADING: Self = Self(1 << 34);
    /// Invocation mask
    pub const INVOCATION_MASK: Self = Self(1 << 35);
    /// Acceleration structure copy
    pub const ACCELERATION_STRUCTURE_COPY: Self = Self(1 << 36);
    /// Micromap build
    pub const MICROMAP_BUILD: Self = Self(1 << 37);
    /// Optical flow
    pub const OPTICAL_FLOW: Self = Self(1 << 38);

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

    /// Intersection
    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
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
    Undefined            = 0,
    /// General
    General              = 1,
    /// Color attachment optimal
    ColorAttachmentOptimal = 2,
    /// Depth-stencil attachment optimal
    DepthStencilAttachmentOptimal = 3,
    /// Depth-stencil read-only optimal
    DepthStencilReadOnlyOptimal = 4,
    /// Shader read-only optimal
    ShaderReadOnlyOptimal = 5,
    /// Transfer source optimal
    TransferSrcOptimal   = 6,
    /// Transfer destination optimal
    TransferDstOptimal   = 7,
    /// Preinitialized
    Preinitialized       = 8,
    /// Present source
    PresentSrc           = 1000001002,
    /// Shared present
    SharedPresent        = 1000111000,
    /// Depth read-only stencil attachment optimal
    DepthReadOnlyStencilAttachmentOptimal = 1000117000,
    /// Depth attachment stencil read-only optimal
    DepthAttachmentStencilReadOnlyOptimal = 1000117001,
    /// Fragment shading rate attachment optimal
    FragmentShadingRateAttachmentOptimal = 1000164003,
    /// Fragment density map optimal
    FragmentDensityMapOptimal = 1000218000,
    /// Depth attachment optimal
    DepthAttachmentOptimal = 1000241000,
    /// Depth read-only optimal
    DepthReadOnlyOptimal = 1000241001,
    /// Stencil attachment optimal
    StencilAttachmentOptimal = 1000241002,
    /// Stencil read-only optimal
    StencilReadOnlyOptimal = 1000241003,
    /// Video decode destination
    VideoDecodeDestination = 1000024000,
    /// Video decode source
    VideoDecodeSource    = 1000024001,
    /// Video decode DPB
    VideoDecodeDPB       = 1000024002,
    /// Video encode destination
    VideoEncodeDestination = 1000299000,
    /// Video encode source
    VideoEncodeSource    = 1000299001,
    /// Video encode DPB
    VideoEncodeDPB       = 1000299002,
    /// Read-only optimal
    ReadOnlyOptimal      = 1000314000,
    /// Attachment optimal
    AttachmentOptimal    = 1000314001,
    /// Attachment feedback loop optimal
    AttachmentFeedbackLoopOptimal = 1000339000,
}

impl ImageLayout {
    /// Is read-only
    #[inline]
    pub const fn is_read_only(&self) -> bool {
        matches!(
            self,
            Self::DepthStencilReadOnlyOptimal
                | Self::ShaderReadOnlyOptimal
                | Self::TransferSrcOptimal
                | Self::DepthReadOnlyOptimal
                | Self::StencilReadOnlyOptimal
                | Self::ReadOnlyOptimal
        )
    }

    /// Is attachment
    #[inline]
    pub const fn is_attachment(&self) -> bool {
        matches!(
            self,
            Self::ColorAttachmentOptimal
                | Self::DepthStencilAttachmentOptimal
                | Self::DepthAttachmentOptimal
                | Self::StencilAttachmentOptimal
                | Self::AttachmentOptimal
        )
    }

    /// Requires layout transition
    #[inline]
    pub const fn requires_transition(&self, other: Self) -> bool {
        (*self as u32) != (other as u32) && !matches!(other, Self::Undefined)
    }
}

// ============================================================================
// Image Subresource Range
// ============================================================================

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
    pub const REMAINING: u32 = u32::MAX;

    /// Color, all mips and layers
    pub const COLOR_ALL: Self = Self {
        aspect_mask: ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: Self::REMAINING,
        base_array_layer: 0,
        layer_count: Self::REMAINING,
    };

    /// Depth, all mips and layers
    pub const DEPTH_ALL: Self = Self {
        aspect_mask: ImageAspectFlags::DEPTH,
        base_mip_level: 0,
        level_count: Self::REMAINING,
        base_array_layer: 0,
        layer_count: Self::REMAINING,
    };

    /// Creates new range
    #[inline]
    pub const fn new(
        aspect_mask: ImageAspectFlags,
        base_mip_level: u32,
        level_count: u32,
        base_array_layer: u32,
        layer_count: u32,
    ) -> Self {
        Self {
            aspect_mask,
            base_mip_level,
            level_count,
            base_array_layer,
            layer_count,
        }
    }

    /// Single mip and layer
    #[inline]
    pub const fn single(aspect: ImageAspectFlags, mip: u32, layer: u32) -> Self {
        Self {
            aspect_mask: aspect,
            base_mip_level: mip,
            level_count: 1,
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
    /// Plane 0
    pub const PLANE_0: Self = Self(1 << 4);
    /// Plane 1
    pub const PLANE_1: Self = Self(1 << 5);
    /// Plane 2
    pub const PLANE_2: Self = Self(1 << 6);
    /// Depth and stencil
    pub const DEPTH_STENCIL: Self = Self(Self::DEPTH.0 | Self::STENCIL.0);

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
// Dependency Info
// ============================================================================

/// Dependency flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DependencyFlags(pub u32);

impl DependencyFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// By region
    pub const BY_REGION: Self = Self(1 << 0);
    /// Device group
    pub const DEVICE_GROUP: Self = Self(1 << 1);
    /// View local
    pub const VIEW_LOCAL: Self = Self(1 << 2);
    /// Feedback loop
    pub const FEEDBACK_LOOP: Self = Self(1 << 3);

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

/// Dependency info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DependencyInfo {
    /// Flags
    pub flags: DependencyFlags,
    /// Memory barriers
    pub memory_barriers: &'static [MemoryBarrier],
    /// Buffer memory barriers
    pub buffer_barriers: &'static [BufferMemoryBarrier],
    /// Image memory barriers
    pub image_barriers: &'static [ImageMemoryBarrier],
}

impl DependencyInfo {
    /// Creates new dependency info
    #[inline]
    pub const fn new() -> Self {
        Self {
            flags: DependencyFlags::NONE,
            memory_barriers: &[],
            buffer_barriers: &[],
            image_barriers: &[],
        }
    }

    /// With memory barriers
    #[inline]
    pub const fn with_memory_barriers(mut self, barriers: &'static [MemoryBarrier]) -> Self {
        self.memory_barriers = barriers;
        self
    }

    /// With buffer barriers
    #[inline]
    pub const fn with_buffer_barriers(mut self, barriers: &'static [BufferMemoryBarrier]) -> Self {
        self.buffer_barriers = barriers;
        self
    }

    /// With image barriers
    #[inline]
    pub const fn with_image_barriers(mut self, barriers: &'static [ImageMemoryBarrier]) -> Self {
        self.image_barriers = barriers;
        self
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: DependencyFlags) -> Self {
        self.flags = flags;
        self
    }
}
