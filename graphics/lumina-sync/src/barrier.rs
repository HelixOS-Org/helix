//! Pipeline Barriers
//!
//! In-command buffer synchronization barriers.

use alloc::vec::Vec;

use bitflags::bitflags;

// ============================================================================
// Pipeline Stage Flags
// ============================================================================

bitflags! {
    /// Pipeline stage flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PipelineStageFlags: u32 {
        /// Top of pipe.
        const TOP_OF_PIPE = 1 << 0;
        /// Draw indirect.
        const DRAW_INDIRECT = 1 << 1;
        /// Vertex input.
        const VERTEX_INPUT = 1 << 2;
        /// Vertex shader.
        const VERTEX_SHADER = 1 << 3;
        /// Tessellation control shader.
        const TESSELLATION_CONTROL_SHADER = 1 << 4;
        /// Tessellation evaluation shader.
        const TESSELLATION_EVALUATION_SHADER = 1 << 5;
        /// Geometry shader.
        const GEOMETRY_SHADER = 1 << 6;
        /// Fragment shader.
        const FRAGMENT_SHADER = 1 << 7;
        /// Early fragment tests.
        const EARLY_FRAGMENT_TESTS = 1 << 8;
        /// Late fragment tests.
        const LATE_FRAGMENT_TESTS = 1 << 9;
        /// Color attachment output.
        const COLOR_ATTACHMENT_OUTPUT = 1 << 10;
        /// Compute shader.
        const COMPUTE_SHADER = 1 << 11;
        /// Transfer.
        const TRANSFER = 1 << 12;
        /// Bottom of pipe.
        const BOTTOM_OF_PIPE = 1 << 13;
        /// Host.
        const HOST = 1 << 14;
        /// All graphics.
        const ALL_GRAPHICS = 1 << 15;
        /// All commands.
        const ALL_COMMANDS = 1 << 16;
        /// Conditional rendering.
        const CONDITIONAL_RENDERING = 1 << 17;
        /// Acceleration structure build.
        const ACCELERATION_STRUCTURE_BUILD = 1 << 18;
        /// Ray tracing shader.
        const RAY_TRACING_SHADER = 1 << 19;
        /// Task shader.
        const TASK_SHADER = 1 << 20;
        /// Mesh shader.
        const MESH_SHADER = 1 << 21;
        /// Fragment shading rate attachment.
        const FRAGMENT_SHADING_RATE_ATTACHMENT = 1 << 22;
    }
}

impl Default for PipelineStageFlags {
    fn default() -> Self {
        PipelineStageFlags::ALL_COMMANDS
    }
}

// ============================================================================
// Access Flags
// ============================================================================

bitflags! {
    /// Memory access flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AccessFlags: u32 {
        /// Indirect command read.
        const INDIRECT_COMMAND_READ = 1 << 0;
        /// Index read.
        const INDEX_READ = 1 << 1;
        /// Vertex attribute read.
        const VERTEX_ATTRIBUTE_READ = 1 << 2;
        /// Uniform read.
        const UNIFORM_READ = 1 << 3;
        /// Input attachment read.
        const INPUT_ATTACHMENT_READ = 1 << 4;
        /// Shader read.
        const SHADER_READ = 1 << 5;
        /// Shader write.
        const SHADER_WRITE = 1 << 6;
        /// Color attachment read.
        const COLOR_ATTACHMENT_READ = 1 << 7;
        /// Color attachment write.
        const COLOR_ATTACHMENT_WRITE = 1 << 8;
        /// Depth stencil attachment read.
        const DEPTH_STENCIL_ATTACHMENT_READ = 1 << 9;
        /// Depth stencil attachment write.
        const DEPTH_STENCIL_ATTACHMENT_WRITE = 1 << 10;
        /// Transfer read.
        const TRANSFER_READ = 1 << 11;
        /// Transfer write.
        const TRANSFER_WRITE = 1 << 12;
        /// Host read.
        const HOST_READ = 1 << 13;
        /// Host write.
        const HOST_WRITE = 1 << 14;
        /// Memory read.
        const MEMORY_READ = 1 << 15;
        /// Memory write.
        const MEMORY_WRITE = 1 << 16;
        /// Conditional rendering read.
        const CONDITIONAL_RENDERING_READ = 1 << 17;
        /// Acceleration structure read.
        const ACCELERATION_STRUCTURE_READ = 1 << 18;
        /// Acceleration structure write.
        const ACCELERATION_STRUCTURE_WRITE = 1 << 19;
        /// Shading rate image read.
        const SHADING_RATE_IMAGE_READ = 1 << 20;
    }
}

impl Default for AccessFlags {
    fn default() -> Self {
        AccessFlags::empty()
    }
}

// ============================================================================
// Image Layout
// ============================================================================

/// Image layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageLayout {
    /// Undefined.
    Undefined,
    /// General.
    General,
    /// Color attachment optimal.
    ColorAttachmentOptimal,
    /// Depth stencil attachment optimal.
    DepthStencilAttachmentOptimal,
    /// Depth stencil read-only optimal.
    DepthStencilReadOnlyOptimal,
    /// Shader read-only optimal.
    ShaderReadOnlyOptimal,
    /// Transfer source optimal.
    TransferSrcOptimal,
    /// Transfer destination optimal.
    TransferDstOptimal,
    /// Preinitialized.
    Preinitialized,
    /// Depth read-only stencil attachment optimal.
    DepthReadOnlyStencilAttachmentOptimal,
    /// Depth attachment stencil read-only optimal.
    DepthAttachmentStencilReadOnlyOptimal,
    /// Depth attachment optimal.
    DepthAttachmentOptimal,
    /// Depth read-only optimal.
    DepthReadOnlyOptimal,
    /// Stencil attachment optimal.
    StencilAttachmentOptimal,
    /// Stencil read-only optimal.
    StencilReadOnlyOptimal,
    /// Present source.
    PresentSrc,
    /// Shading rate optimal.
    ShadingRateOptimal,
    /// Fragment density map optimal.
    FragmentDensityMapOptimal,
}

impl Default for ImageLayout {
    fn default() -> Self {
        ImageLayout::Undefined
    }
}

// ============================================================================
// Memory Barrier
// ============================================================================

/// Global memory barrier.
#[derive(Debug, Clone, Copy)]
pub struct MemoryBarrier {
    /// Source access flags.
    pub src_access: AccessFlags,
    /// Destination access flags.
    pub dst_access: AccessFlags,
}

impl MemoryBarrier {
    /// Create a new memory barrier.
    pub fn new(src_access: AccessFlags, dst_access: AccessFlags) -> Self {
        Self { src_access, dst_access }
    }

    /// Create a read-after-write barrier.
    pub fn read_after_write() -> Self {
        Self {
            src_access: AccessFlags::MEMORY_WRITE,
            dst_access: AccessFlags::MEMORY_READ,
        }
    }

    /// Create a write-after-read barrier.
    pub fn write_after_read() -> Self {
        Self {
            src_access: AccessFlags::MEMORY_READ,
            dst_access: AccessFlags::MEMORY_WRITE,
        }
    }

    /// Create a write-after-write barrier.
    pub fn write_after_write() -> Self {
        Self {
            src_access: AccessFlags::MEMORY_WRITE,
            dst_access: AccessFlags::MEMORY_WRITE,
        }
    }
}

// ============================================================================
// Buffer Barrier
// ============================================================================

/// Buffer memory barrier.
#[derive(Debug, Clone, Copy)]
pub struct BufferBarrier {
    /// Buffer index/handle.
    pub buffer: u32,
    /// Source access flags.
    pub src_access: AccessFlags,
    /// Destination access flags.
    pub dst_access: AccessFlags,
    /// Offset.
    pub offset: u64,
    /// Size (0 = whole buffer).
    pub size: u64,
    /// Source queue family.
    pub src_queue_family: u32,
    /// Destination queue family.
    pub dst_queue_family: u32,
}

impl BufferBarrier {
    /// Create a new buffer barrier.
    pub fn new(buffer: u32, src_access: AccessFlags, dst_access: AccessFlags) -> Self {
        Self {
            buffer,
            src_access,
            dst_access,
            offset: 0,
            size: 0, // Whole buffer
            src_queue_family: !0,
            dst_queue_family: !0,
        }
    }

    /// Set range.
    pub fn with_range(mut self, offset: u64, size: u64) -> Self {
        self.offset = offset;
        self.size = size;
        self
    }

    /// Set queue family transfer.
    pub fn with_queue_transfer(mut self, src_family: u32, dst_family: u32) -> Self {
        self.src_queue_family = src_family;
        self.dst_queue_family = dst_family;
        self
    }

    /// Create for uniform buffer access.
    pub fn uniform_buffer(buffer: u32) -> Self {
        Self::new(
            buffer,
            AccessFlags::SHADER_WRITE | AccessFlags::TRANSFER_WRITE,
            AccessFlags::UNIFORM_READ,
        )
    }

    /// Create for storage buffer access.
    pub fn storage_buffer(buffer: u32) -> Self {
        Self::new(
            buffer,
            AccessFlags::SHADER_WRITE,
            AccessFlags::SHADER_READ | AccessFlags::SHADER_WRITE,
        )
    }

    /// Create for vertex buffer access.
    pub fn vertex_buffer(buffer: u32) -> Self {
        Self::new(
            buffer,
            AccessFlags::TRANSFER_WRITE,
            AccessFlags::VERTEX_ATTRIBUTE_READ,
        )
    }

    /// Create for index buffer access.
    pub fn index_buffer(buffer: u32) -> Self {
        Self::new(
            buffer,
            AccessFlags::TRANSFER_WRITE,
            AccessFlags::INDEX_READ,
        )
    }
}

// ============================================================================
// Image Barrier
// ============================================================================

/// Image subresource range.
#[derive(Debug, Clone, Copy)]
pub struct ImageSubresourceRange {
    /// Aspect mask (color, depth, stencil).
    pub aspect_mask: ImageAspect,
    /// Base mip level.
    pub base_mip_level: u32,
    /// Mip level count (0 = remaining).
    pub level_count: u32,
    /// Base array layer.
    pub base_array_layer: u32,
    /// Array layer count (0 = remaining).
    pub layer_count: u32,
}

impl Default for ImageSubresourceRange {
    fn default() -> Self {
        Self {
            aspect_mask: ImageAspect::COLOR,
            base_mip_level: 0,
            level_count: 0, // All remaining
            base_array_layer: 0,
            layer_count: 0, // All remaining
        }
    }
}

impl ImageSubresourceRange {
    /// Create for color.
    pub fn color() -> Self {
        Self {
            aspect_mask: ImageAspect::COLOR,
            ..Default::default()
        }
    }

    /// Create for depth.
    pub fn depth() -> Self {
        Self {
            aspect_mask: ImageAspect::DEPTH,
            ..Default::default()
        }
    }

    /// Create for stencil.
    pub fn stencil() -> Self {
        Self {
            aspect_mask: ImageAspect::STENCIL,
            ..Default::default()
        }
    }

    /// Create for depth-stencil.
    pub fn depth_stencil() -> Self {
        Self {
            aspect_mask: ImageAspect::DEPTH | ImageAspect::STENCIL,
            ..Default::default()
        }
    }
}

bitflags! {
    /// Image aspect flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ImageAspect: u32 {
        /// Color aspect.
        const COLOR = 1 << 0;
        /// Depth aspect.
        const DEPTH = 1 << 1;
        /// Stencil aspect.
        const STENCIL = 1 << 2;
        /// Metadata aspect.
        const METADATA = 1 << 3;
    }
}

/// Image memory barrier.
#[derive(Debug, Clone)]
pub struct ImageBarrier {
    /// Image index/handle.
    pub image: u32,
    /// Source access flags.
    pub src_access: AccessFlags,
    /// Destination access flags.
    pub dst_access: AccessFlags,
    /// Old layout.
    pub old_layout: ImageLayout,
    /// New layout.
    pub new_layout: ImageLayout,
    /// Subresource range.
    pub subresource_range: ImageSubresourceRange,
    /// Source queue family.
    pub src_queue_family: u32,
    /// Destination queue family.
    pub dst_queue_family: u32,
}

impl ImageBarrier {
    /// Create a new image barrier.
    pub fn new(image: u32, old_layout: ImageLayout, new_layout: ImageLayout) -> Self {
        Self {
            image,
            src_access: AccessFlags::empty(),
            dst_access: AccessFlags::empty(),
            old_layout,
            new_layout,
            subresource_range: ImageSubresourceRange::default(),
            src_queue_family: !0,
            dst_queue_family: !0,
        }
    }

    /// Set access flags.
    pub fn with_access(mut self, src_access: AccessFlags, dst_access: AccessFlags) -> Self {
        self.src_access = src_access;
        self.dst_access = dst_access;
        self
    }

    /// Set subresource range.
    pub fn with_subresource(mut self, range: ImageSubresourceRange) -> Self {
        self.subresource_range = range;
        self
    }

    /// Set queue family transfer.
    pub fn with_queue_transfer(mut self, src_family: u32, dst_family: u32) -> Self {
        self.src_queue_family = src_family;
        self.dst_queue_family = dst_family;
        self
    }

    /// Transition to shader read.
    pub fn to_shader_read(image: u32, old_layout: ImageLayout) -> Self {
        Self::new(image, old_layout, ImageLayout::ShaderReadOnlyOptimal)
            .with_access(
                AccessFlags::TRANSFER_WRITE | AccessFlags::COLOR_ATTACHMENT_WRITE,
                AccessFlags::SHADER_READ,
            )
    }

    /// Transition to color attachment.
    pub fn to_color_attachment(image: u32, old_layout: ImageLayout) -> Self {
        Self::new(image, old_layout, ImageLayout::ColorAttachmentOptimal)
            .with_access(
                AccessFlags::SHADER_READ,
                AccessFlags::COLOR_ATTACHMENT_READ | AccessFlags::COLOR_ATTACHMENT_WRITE,
            )
    }

    /// Transition to depth attachment.
    pub fn to_depth_attachment(image: u32, old_layout: ImageLayout) -> Self {
        Self::new(image, old_layout, ImageLayout::DepthStencilAttachmentOptimal)
            .with_access(
                AccessFlags::SHADER_READ,
                AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ | AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            )
            .with_subresource(ImageSubresourceRange::depth())
    }

    /// Transition to transfer source.
    pub fn to_transfer_src(image: u32, old_layout: ImageLayout) -> Self {
        Self::new(image, old_layout, ImageLayout::TransferSrcOptimal)
            .with_access(AccessFlags::MEMORY_READ, AccessFlags::TRANSFER_READ)
    }

    /// Transition to transfer destination.
    pub fn to_transfer_dst(image: u32, old_layout: ImageLayout) -> Self {
        Self::new(image, old_layout, ImageLayout::TransferDstOptimal)
            .with_access(AccessFlags::empty(), AccessFlags::TRANSFER_WRITE)
    }

    /// Transition to present.
    pub fn to_present(image: u32, old_layout: ImageLayout) -> Self {
        Self::new(image, old_layout, ImageLayout::PresentSrc)
            .with_access(AccessFlags::COLOR_ATTACHMENT_WRITE, AccessFlags::empty())
    }
}

// ============================================================================
// Barrier
// ============================================================================

/// Combined barrier type.
#[derive(Debug, Clone)]
pub enum Barrier {
    /// Memory barrier.
    Memory(MemoryBarrier),
    /// Buffer barrier.
    Buffer(BufferBarrier),
    /// Image barrier.
    Image(ImageBarrier),
}

impl From<MemoryBarrier> for Barrier {
    fn from(b: MemoryBarrier) -> Self {
        Barrier::Memory(b)
    }
}

impl From<BufferBarrier> for Barrier {
    fn from(b: BufferBarrier) -> Self {
        Barrier::Buffer(b)
    }
}

impl From<ImageBarrier> for Barrier {
    fn from(b: ImageBarrier) -> Self {
        Barrier::Image(b)
    }
}

// ============================================================================
// Barrier Batch
// ============================================================================

/// Batch of barriers for efficient submission.
#[derive(Debug, Clone, Default)]
pub struct BarrierBatch {
    /// Source stage.
    pub src_stage: PipelineStageFlags,
    /// Destination stage.
    pub dst_stage: PipelineStageFlags,
    /// Memory barriers.
    pub memory_barriers: Vec<MemoryBarrier>,
    /// Buffer barriers.
    pub buffer_barriers: Vec<BufferBarrier>,
    /// Image barriers.
    pub image_barriers: Vec<ImageBarrier>,
}

impl BarrierBatch {
    /// Create a new batch.
    pub fn new(src_stage: PipelineStageFlags, dst_stage: PipelineStageFlags) -> Self {
        Self {
            src_stage,
            dst_stage,
            memory_barriers: Vec::new(),
            buffer_barriers: Vec::new(),
            image_barriers: Vec::new(),
        }
    }

    /// Add a memory barrier.
    pub fn memory(mut self, barrier: MemoryBarrier) -> Self {
        self.memory_barriers.push(barrier);
        self
    }

    /// Add a buffer barrier.
    pub fn buffer(mut self, barrier: BufferBarrier) -> Self {
        self.buffer_barriers.push(barrier);
        self
    }

    /// Add an image barrier.
    pub fn image(mut self, barrier: ImageBarrier) -> Self {
        self.image_barriers.push(barrier);
        self
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.memory_barriers.is_empty()
            && self.buffer_barriers.is_empty()
            && self.image_barriers.is_empty()
    }

    /// Get total barrier count.
    pub fn barrier_count(&self) -> usize {
        self.memory_barriers.len() + self.buffer_barriers.len() + self.image_barriers.len()
    }

    /// Clear all barriers.
    pub fn clear(&mut self) {
        self.memory_barriers.clear();
        self.buffer_barriers.clear();
        self.image_barriers.clear();
    }
}

// ============================================================================
// Common Barrier Patterns
// ============================================================================

/// Common barrier patterns.
pub mod patterns {
    use super::*;

    /// Compute to graphics barrier.
    pub fn compute_to_graphics() -> BarrierBatch {
        BarrierBatch::new(
            PipelineStageFlags::COMPUTE_SHADER,
            PipelineStageFlags::VERTEX_INPUT | PipelineStageFlags::FRAGMENT_SHADER,
        )
        .memory(MemoryBarrier::read_after_write())
    }

    /// Graphics to compute barrier.
    pub fn graphics_to_compute() -> BarrierBatch {
        BarrierBatch::new(
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            PipelineStageFlags::COMPUTE_SHADER,
        )
        .memory(MemoryBarrier::read_after_write())
    }

    /// Transfer to graphics barrier.
    pub fn transfer_to_graphics() -> BarrierBatch {
        BarrierBatch::new(
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::VERTEX_INPUT | PipelineStageFlags::FRAGMENT_SHADER,
        )
        .memory(MemoryBarrier::new(
            AccessFlags::TRANSFER_WRITE,
            AccessFlags::VERTEX_ATTRIBUTE_READ | AccessFlags::INDEX_READ | AccessFlags::SHADER_READ,
        ))
    }

    /// Graphics to present barrier.
    pub fn graphics_to_present() -> BarrierBatch {
        BarrierBatch::new(
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            PipelineStageFlags::BOTTOM_OF_PIPE,
        )
    }
}
