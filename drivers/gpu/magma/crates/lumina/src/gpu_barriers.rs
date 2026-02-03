//! GPU Barriers Types for Lumina
//!
//! This module provides GPU synchronization barrier infrastructure
//! for memory and execution dependencies.

extern crate alloc;

// ============================================================================
// Memory Barrier Types
// ============================================================================

/// Memory barrier
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MemoryBarrier {
    /// Source access mask
    pub src_access: AccessFlags,
    /// Destination access mask
    pub dst_access: AccessFlags,
}

impl MemoryBarrier {
    /// Creates barrier
    pub const fn new(src: AccessFlags, dst: AccessFlags) -> Self {
        Self {
            src_access: src,
            dst_access: dst,
        }
    }

    /// Full barrier
    pub const fn full() -> Self {
        Self::new(AccessFlags::all(), AccessFlags::all())
    }

    /// Shader read after write
    pub const fn shader_read_after_write() -> Self {
        Self::new(
            AccessFlags::SHADER_WRITE,
            AccessFlags::SHADER_READ,
        )
    }

    /// Transfer to shader read
    pub const fn transfer_to_shader_read() -> Self {
        Self::new(
            AccessFlags::TRANSFER_WRITE,
            AccessFlags::SHADER_READ,
        )
    }

    /// Shader write to transfer
    pub const fn shader_write_to_transfer() -> Self {
        Self::new(
            AccessFlags::SHADER_WRITE,
            AccessFlags::TRANSFER_READ,
        )
    }

    /// Color attachment to shader read
    pub const fn color_to_shader_read() -> Self {
        Self::new(
            AccessFlags::COLOR_ATTACHMENT_WRITE,
            AccessFlags::SHADER_READ,
        )
    }

    /// Depth attachment to shader read
    pub const fn depth_to_shader_read() -> Self {
        Self::new(
            AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            AccessFlags::SHADER_READ,
        )
    }
}

bitflags::bitflags! {
    /// Access flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct AccessFlags: u32 {
        /// None
        const NONE = 0;
        /// Indirect command read
        const INDIRECT_COMMAND_READ = 1 << 0;
        /// Index read
        const INDEX_READ = 1 << 1;
        /// Vertex attribute read
        const VERTEX_ATTRIBUTE_READ = 1 << 2;
        /// Uniform read
        const UNIFORM_READ = 1 << 3;
        /// Input attachment read
        const INPUT_ATTACHMENT_READ = 1 << 4;
        /// Shader read
        const SHADER_READ = 1 << 5;
        /// Shader write
        const SHADER_WRITE = 1 << 6;
        /// Color attachment read
        const COLOR_ATTACHMENT_READ = 1 << 7;
        /// Color attachment write
        const COLOR_ATTACHMENT_WRITE = 1 << 8;
        /// Depth stencil attachment read
        const DEPTH_STENCIL_ATTACHMENT_READ = 1 << 9;
        /// Depth stencil attachment write
        const DEPTH_STENCIL_ATTACHMENT_WRITE = 1 << 10;
        /// Transfer read
        const TRANSFER_READ = 1 << 11;
        /// Transfer write
        const TRANSFER_WRITE = 1 << 12;
        /// Host read
        const HOST_READ = 1 << 13;
        /// Host write
        const HOST_WRITE = 1 << 14;
        /// Memory read
        const MEMORY_READ = 1 << 15;
        /// Memory write
        const MEMORY_WRITE = 1 << 16;
        /// Acceleration structure read
        const ACCELERATION_STRUCTURE_READ = 1 << 17;
        /// Acceleration structure write
        const ACCELERATION_STRUCTURE_WRITE = 1 << 18;
        /// Fragment shading rate attachment read
        const FRAGMENT_SHADING_RATE_ATTACHMENT_READ = 1 << 19;
    }
}

impl AccessFlags {
    /// All read flags
    pub const fn all_reads() -> Self {
        Self::from_bits_truncate(
            Self::INDIRECT_COMMAND_READ.bits() |
            Self::INDEX_READ.bits() |
            Self::VERTEX_ATTRIBUTE_READ.bits() |
            Self::UNIFORM_READ.bits() |
            Self::INPUT_ATTACHMENT_READ.bits() |
            Self::SHADER_READ.bits() |
            Self::COLOR_ATTACHMENT_READ.bits() |
            Self::DEPTH_STENCIL_ATTACHMENT_READ.bits() |
            Self::TRANSFER_READ.bits() |
            Self::HOST_READ.bits() |
            Self::MEMORY_READ.bits() |
            Self::ACCELERATION_STRUCTURE_READ.bits()
        )
    }

    /// All write flags
    pub const fn all_writes() -> Self {
        Self::from_bits_truncate(
            Self::SHADER_WRITE.bits() |
            Self::COLOR_ATTACHMENT_WRITE.bits() |
            Self::DEPTH_STENCIL_ATTACHMENT_WRITE.bits() |
            Self::TRANSFER_WRITE.bits() |
            Self::HOST_WRITE.bits() |
            Self::MEMORY_WRITE.bits() |
            Self::ACCELERATION_STRUCTURE_WRITE.bits()
        )
    }
}

// ============================================================================
// Buffer Barrier
// ============================================================================

/// Buffer memory barrier
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferMemoryBarrier {
    /// Buffer handle
    pub buffer: u64,
    /// Source access mask
    pub src_access: AccessFlags,
    /// Destination access mask
    pub dst_access: AccessFlags,
    /// Offset in bytes
    pub offset: u64,
    /// Size in bytes (0 = whole buffer)
    pub size: u64,
    /// Source queue family
    pub src_queue_family: u32,
    /// Destination queue family
    pub dst_queue_family: u32,
}

impl BufferMemoryBarrier {
    /// Queue family ignored
    pub const QUEUE_FAMILY_IGNORED: u32 = u32::MAX;

    /// Creates barrier
    pub const fn new(buffer: u64, src: AccessFlags, dst: AccessFlags) -> Self {
        Self {
            buffer,
            src_access: src,
            dst_access: dst,
            offset: 0,
            size: 0,  // Whole buffer
            src_queue_family: Self::QUEUE_FAMILY_IGNORED,
            dst_queue_family: Self::QUEUE_FAMILY_IGNORED,
        }
    }

    /// With range
    pub const fn with_range(mut self, offset: u64, size: u64) -> Self {
        self.offset = offset;
        self.size = size;
        self
    }

    /// With queue family transfer
    pub const fn with_queue_transfer(mut self, src: u32, dst: u32) -> Self {
        self.src_queue_family = src;
        self.dst_queue_family = dst;
        self
    }
}

impl Default for BufferMemoryBarrier {
    fn default() -> Self {
        Self::new(0, AccessFlags::NONE, AccessFlags::NONE)
    }
}

// ============================================================================
// Image Barrier
// ============================================================================

/// Image memory barrier
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageMemoryBarrier {
    /// Image handle
    pub image: u64,
    /// Source access mask
    pub src_access: AccessFlags,
    /// Destination access mask
    pub dst_access: AccessFlags,
    /// Old layout
    pub old_layout: ImageLayout,
    /// New layout
    pub new_layout: ImageLayout,
    /// Source queue family
    pub src_queue_family: u32,
    /// Destination queue family
    pub dst_queue_family: u32,
    /// Subresource range
    pub subresource: ImageSubresourceRange,
}

impl ImageMemoryBarrier {
    /// Queue family ignored
    pub const QUEUE_FAMILY_IGNORED: u32 = u32::MAX;

    /// Creates barrier
    pub const fn new(image: u64, old_layout: ImageLayout, new_layout: ImageLayout) -> Self {
        Self {
            image,
            src_access: AccessFlags::NONE,
            dst_access: AccessFlags::NONE,
            old_layout,
            new_layout,
            src_queue_family: Self::QUEUE_FAMILY_IGNORED,
            dst_queue_family: Self::QUEUE_FAMILY_IGNORED,
            subresource: ImageSubresourceRange::all(),
        }
    }

    /// With access masks
    pub const fn with_access(mut self, src: AccessFlags, dst: AccessFlags) -> Self {
        self.src_access = src;
        self.dst_access = dst;
        self
    }

    /// With subresource
    pub const fn with_subresource(mut self, subresource: ImageSubresourceRange) -> Self {
        self.subresource = subresource;
        self
    }

    /// Undefined to transfer dst
    pub const fn undefined_to_transfer_dst(image: u64) -> Self {
        Self::new(image, ImageLayout::Undefined, ImageLayout::TransferDstOptimal)
            .with_access(AccessFlags::NONE, AccessFlags::TRANSFER_WRITE)
    }

    /// Transfer dst to shader read
    pub const fn transfer_to_shader_read(image: u64) -> Self {
        Self::new(image, ImageLayout::TransferDstOptimal, ImageLayout::ShaderReadOnlyOptimal)
            .with_access(AccessFlags::TRANSFER_WRITE, AccessFlags::SHADER_READ)
    }

    /// Color attachment to shader read
    pub const fn color_to_shader_read(image: u64) -> Self {
        Self::new(image, ImageLayout::ColorAttachmentOptimal, ImageLayout::ShaderReadOnlyOptimal)
            .with_access(AccessFlags::COLOR_ATTACHMENT_WRITE, AccessFlags::SHADER_READ)
    }

    /// Shader read to color attachment
    pub const fn shader_read_to_color(image: u64) -> Self {
        Self::new(image, ImageLayout::ShaderReadOnlyOptimal, ImageLayout::ColorAttachmentOptimal)
            .with_access(AccessFlags::SHADER_READ, AccessFlags::COLOR_ATTACHMENT_WRITE)
    }

    /// Depth to shader read
    pub const fn depth_to_shader_read(image: u64) -> Self {
        Self::new(image, ImageLayout::DepthStencilAttachmentOptimal, ImageLayout::ShaderReadOnlyOptimal)
            .with_access(AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE, AccessFlags::SHADER_READ)
    }

    /// Undefined to color attachment
    pub const fn undefined_to_color_attachment(image: u64) -> Self {
        Self::new(image, ImageLayout::Undefined, ImageLayout::ColorAttachmentOptimal)
            .with_access(AccessFlags::NONE, AccessFlags::COLOR_ATTACHMENT_WRITE)
    }

    /// Undefined to depth attachment
    pub const fn undefined_to_depth_attachment(image: u64) -> Self {
        Self::new(image, ImageLayout::Undefined, ImageLayout::DepthStencilAttachmentOptimal)
            .with_access(AccessFlags::NONE, AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
    }

    /// Color to present
    pub const fn color_to_present(image: u64) -> Self {
        Self::new(image, ImageLayout::ColorAttachmentOptimal, ImageLayout::PresentSrc)
            .with_access(AccessFlags::COLOR_ATTACHMENT_WRITE, AccessFlags::NONE)
    }

    /// Present to color
    pub const fn present_to_color(image: u64) -> Self {
        Self::new(image, ImageLayout::PresentSrc, ImageLayout::ColorAttachmentOptimal)
            .with_access(AccessFlags::NONE, AccessFlags::COLOR_ATTACHMENT_WRITE)
    }

    /// General to storage (compute)
    pub const fn general_to_storage(image: u64) -> Self {
        Self::new(image, ImageLayout::General, ImageLayout::General)
            .with_access(AccessFlags::SHADER_READ, AccessFlags::SHADER_WRITE)
    }

    /// Storage to general (compute)
    pub const fn storage_to_general(image: u64) -> Self {
        Self::new(image, ImageLayout::General, ImageLayout::General)
            .with_access(AccessFlags::SHADER_WRITE, AccessFlags::SHADER_READ)
    }
}

impl Default for ImageMemoryBarrier {
    fn default() -> Self {
        Self::new(0, ImageLayout::Undefined, ImageLayout::General)
    }
}

/// Image layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ImageLayout {
    /// Undefined
    #[default]
    Undefined = 0,
    /// General (all operations)
    General = 1,
    /// Color attachment optimal
    ColorAttachmentOptimal = 2,
    /// Depth stencil attachment optimal
    DepthStencilAttachmentOptimal = 3,
    /// Depth stencil read only optimal
    DepthStencilReadOnlyOptimal = 4,
    /// Shader read only optimal
    ShaderReadOnlyOptimal = 5,
    /// Transfer source optimal
    TransferSrcOptimal = 6,
    /// Transfer destination optimal
    TransferDstOptimal = 7,
    /// Preinitialized
    Preinitialized = 8,
    /// Depth read only stencil attachment optimal
    DepthReadOnlyStencilAttachmentOptimal = 9,
    /// Depth attachment stencil read only optimal
    DepthAttachmentStencilReadOnlyOptimal = 10,
    /// Depth attachment optimal
    DepthAttachmentOptimal = 11,
    /// Depth read only optimal
    DepthReadOnlyOptimal = 12,
    /// Stencil attachment optimal
    StencilAttachmentOptimal = 13,
    /// Stencil read only optimal
    StencilReadOnlyOptimal = 14,
    /// Present source
    PresentSrc = 15,
    /// Read only optimal
    ReadOnlyOptimal = 16,
    /// Attachment optimal
    AttachmentOptimal = 17,
    /// Fragment shading rate attachment optimal
    FragmentShadingRateAttachmentOptimal = 18,
}

impl ImageLayout {
    /// Is depth format compatible
    pub const fn is_depth_compatible(&self) -> bool {
        matches!(
            self,
            Self::DepthStencilAttachmentOptimal |
            Self::DepthStencilReadOnlyOptimal |
            Self::DepthReadOnlyStencilAttachmentOptimal |
            Self::DepthAttachmentStencilReadOnlyOptimal |
            Self::DepthAttachmentOptimal |
            Self::DepthReadOnlyOptimal
        )
    }

    /// Is color compatible
    pub const fn is_color_compatible(&self) -> bool {
        matches!(
            self,
            Self::ColorAttachmentOptimal |
            Self::ShaderReadOnlyOptimal |
            Self::TransferSrcOptimal |
            Self::TransferDstOptimal |
            Self::PresentSrc |
            Self::General
        )
    }

    /// Is read only
    pub const fn is_read_only(&self) -> bool {
        matches!(
            self,
            Self::ShaderReadOnlyOptimal |
            Self::DepthStencilReadOnlyOptimal |
            Self::DepthReadOnlyOptimal |
            Self::StencilReadOnlyOptimal |
            Self::ReadOnlyOptimal |
            Self::TransferSrcOptimal
        )
    }
}

/// Image subresource range
#[derive(Clone, Copy, Debug)]
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
    /// Remaining mip levels
    pub const REMAINING_MIP_LEVELS: u32 = u32::MAX;
    /// Remaining array layers
    pub const REMAINING_ARRAY_LAYERS: u32 = u32::MAX;

    /// All subresources (color)
    pub const fn all() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: Self::REMAINING_MIP_LEVELS,
            base_array_layer: 0,
            layer_count: Self::REMAINING_ARRAY_LAYERS,
        }
    }

    /// All depth
    pub const fn all_depth() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::DEPTH,
            ..Self::all()
        }
    }

    /// All depth stencil
    pub const fn all_depth_stencil() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::from_bits_truncate(
                ImageAspectFlags::DEPTH.bits() | ImageAspectFlags::STENCIL.bits()
            ),
            ..Self::all()
        }
    }

    /// Single mip
    pub const fn single_mip(mip: u32) -> Self {
        Self {
            base_mip_level: mip,
            level_count: 1,
            ..Self::all()
        }
    }

    /// Single layer
    pub const fn single_layer(layer: u32) -> Self {
        Self {
            base_array_layer: layer,
            layer_count: 1,
            ..Self::all()
        }
    }

    /// Mip range
    pub const fn mip_range(base: u32, count: u32) -> Self {
        Self {
            base_mip_level: base,
            level_count: count,
            ..Self::all()
        }
    }

    /// With aspect
    pub const fn with_aspect(mut self, aspect: ImageAspectFlags) -> Self {
        self.aspect_mask = aspect;
        self
    }
}

impl Default for ImageSubresourceRange {
    fn default() -> Self {
        Self::all()
    }
}

bitflags::bitflags! {
    /// Image aspect flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct ImageAspectFlags: u32 {
        /// None
        const NONE = 0;
        /// Color
        const COLOR = 1 << 0;
        /// Depth
        const DEPTH = 1 << 1;
        /// Stencil
        const STENCIL = 1 << 2;
        /// Metadata
        const METADATA = 1 << 3;
        /// Plane 0
        const PLANE_0 = 1 << 4;
        /// Plane 1
        const PLANE_1 = 1 << 5;
        /// Plane 2
        const PLANE_2 = 1 << 6;
    }
}

// ============================================================================
// Pipeline Barrier
// ============================================================================

/// Pipeline barrier
#[derive(Clone, Debug, Default)]
pub struct PipelineBarrier {
    /// Source stage mask
    pub src_stage: PipelineStageFlags,
    /// Destination stage mask
    pub dst_stage: PipelineStageFlags,
    /// Dependency flags
    pub dependency_flags: DependencyFlags,
    /// Memory barriers
    pub memory_barriers: alloc::vec::Vec<MemoryBarrier>,
    /// Buffer barriers
    pub buffer_barriers: alloc::vec::Vec<BufferMemoryBarrier>,
    /// Image barriers
    pub image_barriers: alloc::vec::Vec<ImageMemoryBarrier>,
}

impl PipelineBarrier {
    /// Creates barrier
    pub fn new(src: PipelineStageFlags, dst: PipelineStageFlags) -> Self {
        Self {
            src_stage: src,
            dst_stage: dst,
            dependency_flags: DependencyFlags::empty(),
            memory_barriers: alloc::vec::Vec::new(),
            buffer_barriers: alloc::vec::Vec::new(),
            image_barriers: alloc::vec::Vec::new(),
        }
    }

    /// With memory barrier
    pub fn with_memory_barrier(mut self, barrier: MemoryBarrier) -> Self {
        self.memory_barriers.push(barrier);
        self
    }

    /// With buffer barrier
    pub fn with_buffer_barrier(mut self, barrier: BufferMemoryBarrier) -> Self {
        self.buffer_barriers.push(barrier);
        self
    }

    /// With image barrier
    pub fn with_image_barrier(mut self, barrier: ImageMemoryBarrier) -> Self {
        self.image_barriers.push(barrier);
        self
    }

    /// Full barrier
    pub fn full() -> Self {
        Self::new(PipelineStageFlags::ALL_COMMANDS, PipelineStageFlags::ALL_COMMANDS)
            .with_memory_barrier(MemoryBarrier::full())
    }

    /// Compute to compute
    pub fn compute_to_compute() -> Self {
        Self::new(PipelineStageFlags::COMPUTE_SHADER, PipelineStageFlags::COMPUTE_SHADER)
            .with_memory_barrier(MemoryBarrier::shader_read_after_write())
    }

    /// Compute to graphics
    pub fn compute_to_graphics() -> Self {
        Self::new(
            PipelineStageFlags::COMPUTE_SHADER,
            PipelineStageFlags::VERTEX_SHADER.union(PipelineStageFlags::FRAGMENT_SHADER)
        ).with_memory_barrier(MemoryBarrier::shader_read_after_write())
    }

    /// Graphics to compute
    pub fn graphics_to_compute() -> Self {
        Self::new(
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT.union(PipelineStageFlags::LATE_FRAGMENT_TESTS),
            PipelineStageFlags::COMPUTE_SHADER
        ).with_memory_barrier(MemoryBarrier::new(
            AccessFlags::COLOR_ATTACHMENT_WRITE.union(AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE),
            AccessFlags::SHADER_READ
        ))
    }
}

bitflags::bitflags! {
    /// Pipeline stage flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct PipelineStageFlags: u32 {
        /// None
        const NONE = 0;
        /// Top of pipe
        const TOP_OF_PIPE = 1 << 0;
        /// Draw indirect
        const DRAW_INDIRECT = 1 << 1;
        /// Vertex input
        const VERTEX_INPUT = 1 << 2;
        /// Vertex shader
        const VERTEX_SHADER = 1 << 3;
        /// Tessellation control shader
        const TESSELLATION_CONTROL_SHADER = 1 << 4;
        /// Tessellation evaluation shader
        const TESSELLATION_EVALUATION_SHADER = 1 << 5;
        /// Geometry shader
        const GEOMETRY_SHADER = 1 << 6;
        /// Fragment shader
        const FRAGMENT_SHADER = 1 << 7;
        /// Early fragment tests
        const EARLY_FRAGMENT_TESTS = 1 << 8;
        /// Late fragment tests
        const LATE_FRAGMENT_TESTS = 1 << 9;
        /// Color attachment output
        const COLOR_ATTACHMENT_OUTPUT = 1 << 10;
        /// Compute shader
        const COMPUTE_SHADER = 1 << 11;
        /// Transfer
        const TRANSFER = 1 << 12;
        /// Bottom of pipe
        const BOTTOM_OF_PIPE = 1 << 13;
        /// Host
        const HOST = 1 << 14;
        /// All graphics
        const ALL_GRAPHICS = 1 << 15;
        /// All commands
        const ALL_COMMANDS = 1 << 16;
        /// Ray tracing shader
        const RAY_TRACING_SHADER = 1 << 17;
        /// Acceleration structure build
        const ACCELERATION_STRUCTURE_BUILD = 1 << 18;
        /// Task shader
        const TASK_SHADER = 1 << 19;
        /// Mesh shader
        const MESH_SHADER = 1 << 20;
        /// Fragment shading rate attachment
        const FRAGMENT_SHADING_RATE_ATTACHMENT = 1 << 21;
    }
}

bitflags::bitflags! {
    /// Dependency flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct DependencyFlags: u32 {
        /// None
        const NONE = 0;
        /// By region
        const BY_REGION = 1 << 0;
        /// Device group
        const DEVICE_GROUP = 1 << 1;
        /// View local
        const VIEW_LOCAL = 1 << 2;
    }
}

// ============================================================================
// Execution Barrier
// ============================================================================

/// Execution barrier (simpler form)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ExecutionBarrier {
    /// Source stage
    pub src_stage: PipelineStageFlags,
    /// Destination stage
    pub dst_stage: PipelineStageFlags,
}

impl ExecutionBarrier {
    /// Creates barrier
    pub const fn new(src: PipelineStageFlags, dst: PipelineStageFlags) -> Self {
        Self {
            src_stage: src,
            dst_stage: dst,
        }
    }

    /// Wait for all
    pub const fn wait_all() -> Self {
        Self::new(PipelineStageFlags::ALL_COMMANDS, PipelineStageFlags::ALL_COMMANDS)
    }

    /// Compute finish
    pub const fn compute_finish() -> Self {
        Self::new(PipelineStageFlags::COMPUTE_SHADER, PipelineStageFlags::BOTTOM_OF_PIPE)
    }

    /// Graphics finish
    pub const fn graphics_finish() -> Self {
        Self::new(PipelineStageFlags::ALL_GRAPHICS, PipelineStageFlags::BOTTOM_OF_PIPE)
    }
}
