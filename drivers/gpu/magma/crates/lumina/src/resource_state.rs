//! Resource state tracking for automatic barriers
//!
//! This module provides automatic resource state tracking and barrier generation.

use crate::barrier::{
    AccessFlags, BufferBarrier, BufferHandle, DependencyFlags, ImageAspectFlags, ImageBarrier,
    ImageHandle, ImageLayout, ImageSubresourceRange, MemoryBarrier, PipelineBarrier,
    PipelineStageFlags, QUEUE_FAMILY_IGNORED, WHOLE_SIZE,
};

/// Resource state for a buffer
#[derive(Clone, Copy, Debug)]
pub struct BufferState {
    /// Current access flags
    pub access: AccessFlags,
    /// Current pipeline stage
    pub stage: PipelineStageFlags,
    /// Current queue family
    pub queue_family: u32,
}

impl Default for BufferState {
    fn default() -> Self {
        Self {
            access: AccessFlags::NONE,
            stage: PipelineStageFlags::TOP_OF_PIPE,
            queue_family: QUEUE_FAMILY_IGNORED,
        }
    }
}

impl BufferState {
    /// Creates a new buffer state
    pub const fn new(access: AccessFlags, stage: PipelineStageFlags) -> Self {
        Self {
            access,
            stage,
            queue_family: QUEUE_FAMILY_IGNORED,
        }
    }

    /// Vertex buffer read state
    pub const fn vertex_buffer() -> Self {
        Self::new(
            AccessFlags::VERTEX_ATTRIBUTE_READ,
            PipelineStageFlags::VERTEX_INPUT,
        )
    }

    /// Index buffer read state
    pub const fn index_buffer() -> Self {
        Self::new(AccessFlags::INDEX_READ, PipelineStageFlags::VERTEX_INPUT)
    }

    /// Uniform buffer read state
    pub const fn uniform_buffer() -> Self {
        Self::new(AccessFlags::UNIFORM_READ, PipelineStageFlags::ALL_SHADERS)
    }

    /// Storage buffer read state
    pub const fn storage_read() -> Self {
        Self::new(
            AccessFlags::SHADER_STORAGE_READ,
            PipelineStageFlags::ALL_SHADERS,
        )
    }

    /// Storage buffer write state
    pub const fn storage_write() -> Self {
        Self::new(
            AccessFlags::SHADER_STORAGE_WRITE,
            PipelineStageFlags::ALL_SHADERS,
        )
    }

    /// Storage buffer read/write state
    pub const fn storage_read_write() -> Self {
        Self::new(
            AccessFlags(AccessFlags::SHADER_STORAGE_READ.0 | AccessFlags::SHADER_STORAGE_WRITE.0),
            PipelineStageFlags::ALL_SHADERS,
        )
    }

    /// Transfer source state
    pub const fn transfer_src() -> Self {
        Self::new(AccessFlags::TRANSFER_READ, PipelineStageFlags::TRANSFER)
    }

    /// Transfer destination state
    pub const fn transfer_dst() -> Self {
        Self::new(AccessFlags::TRANSFER_WRITE, PipelineStageFlags::TRANSFER)
    }

    /// Indirect buffer state
    pub const fn indirect() -> Self {
        Self::new(
            AccessFlags::INDIRECT_COMMAND_READ,
            PipelineStageFlags::DRAW_INDIRECT,
        )
    }
}

/// Resource state for an image
#[derive(Clone, Copy, Debug)]
pub struct ImageState {
    /// Current access flags
    pub access: AccessFlags,
    /// Current pipeline stage
    pub stage: PipelineStageFlags,
    /// Current layout
    pub layout: ImageLayout,
    /// Current queue family
    pub queue_family: u32,
}

impl Default for ImageState {
    fn default() -> Self {
        Self {
            access: AccessFlags::NONE,
            stage: PipelineStageFlags::TOP_OF_PIPE,
            layout: ImageLayout::Undefined,
            queue_family: QUEUE_FAMILY_IGNORED,
        }
    }
}

impl ImageState {
    /// Creates a new image state
    pub const fn new(access: AccessFlags, stage: PipelineStageFlags, layout: ImageLayout) -> Self {
        Self {
            access,
            stage,
            layout,
            queue_family: QUEUE_FAMILY_IGNORED,
        }
    }

    /// Shader read state
    pub const fn shader_read() -> Self {
        Self::new(
            AccessFlags::SHADER_READ,
            PipelineStageFlags::ALL_SHADERS,
            ImageLayout::ShaderReadOnlyOptimal,
        )
    }

    /// Shader storage read state
    pub const fn storage_read() -> Self {
        Self::new(
            AccessFlags::SHADER_STORAGE_READ,
            PipelineStageFlags::ALL_SHADERS,
            ImageLayout::General,
        )
    }

    /// Shader storage write state
    pub const fn storage_write() -> Self {
        Self::new(
            AccessFlags::SHADER_STORAGE_WRITE,
            PipelineStageFlags::ALL_SHADERS,
            ImageLayout::General,
        )
    }

    /// Color attachment state
    pub const fn color_attachment() -> Self {
        Self::new(
            AccessFlags(AccessFlags::COLOR_ATTACHMENT_READ.0 | AccessFlags::COLOR_ATTACHMENT_WRITE.0),
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ImageLayout::ColorAttachmentOptimal,
        )
    }

    /// Depth attachment state
    pub const fn depth_attachment() -> Self {
        Self::new(
            AccessFlags(AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ.0 | AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE.0),
            PipelineStageFlags(PipelineStageFlags::EARLY_FRAGMENT_TESTS.0 | PipelineStageFlags::LATE_FRAGMENT_TESTS.0),
            ImageLayout::DepthStencilAttachmentOptimal,
        )
    }

    /// Depth read-only state
    pub const fn depth_read() -> Self {
        Self::new(
            AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,
            PipelineStageFlags(PipelineStageFlags::EARLY_FRAGMENT_TESTS.0 | PipelineStageFlags::LATE_FRAGMENT_TESTS.0),
            ImageLayout::DepthStencilReadOnlyOptimal,
        )
    }

    /// Transfer source state
    pub const fn transfer_src() -> Self {
        Self::new(
            AccessFlags::TRANSFER_READ,
            PipelineStageFlags::TRANSFER,
            ImageLayout::TransferSrcOptimal,
        )
    }

    /// Transfer destination state
    pub const fn transfer_dst() -> Self {
        Self::new(
            AccessFlags::TRANSFER_WRITE,
            PipelineStageFlags::TRANSFER,
            ImageLayout::TransferDstOptimal,
        )
    }

    /// Present state
    pub const fn present() -> Self {
        Self::new(
            AccessFlags::NONE,
            PipelineStageFlags::BOTTOM_OF_PIPE,
            ImageLayout::PresentSrc,
        )
    }
}

/// Transition for a buffer resource
#[derive(Clone, Copy, Debug)]
pub struct BufferTransition {
    /// Buffer handle
    pub buffer: BufferHandle,
    /// Old state
    pub old_state: BufferState,
    /// New state
    pub new_state: BufferState,
    /// Buffer offset
    pub offset: u64,
    /// Buffer size
    pub size: u64,
}

impl BufferTransition {
    /// Creates a new buffer transition
    pub const fn new(buffer: BufferHandle, old_state: BufferState, new_state: BufferState) -> Self {
        Self {
            buffer,
            old_state,
            new_state,
            offset: 0,
            size: WHOLE_SIZE,
        }
    }

    /// With specific region
    pub const fn with_region(mut self, offset: u64, size: u64) -> Self {
        self.offset = offset;
        self.size = size;
        self
    }

    /// Converts to buffer barrier
    pub const fn to_barrier(&self) -> BufferBarrier {
        BufferBarrier {
            src_access: self.old_state.access,
            dst_access: self.new_state.access,
            src_queue_family: self.old_state.queue_family,
            dst_queue_family: self.new_state.queue_family,
            buffer: self.buffer,
            offset: self.offset,
            size: self.size,
        }
    }

    /// Checks if barrier is needed
    pub const fn needs_barrier(&self) -> bool {
        // Need barrier if:
        // 1. Write to read hazard
        // 2. Write to write hazard
        // 3. Queue family transfer
        self.old_state.access.is_write()
            || self.new_state.access.is_write()
            || (self.old_state.queue_family != self.new_state.queue_family
                && self.old_state.queue_family != QUEUE_FAMILY_IGNORED
                && self.new_state.queue_family != QUEUE_FAMILY_IGNORED)
    }
}

/// Transition for an image resource
#[derive(Clone, Copy, Debug)]
pub struct ImageTransition {
    /// Image handle
    pub image: ImageHandle,
    /// Old state
    pub old_state: ImageState,
    /// New state
    pub new_state: ImageState,
    /// Subresource range
    pub subresource_range: ImageSubresourceRange,
}

impl ImageTransition {
    /// Creates a new image transition
    pub const fn new(image: ImageHandle, old_state: ImageState, new_state: ImageState) -> Self {
        Self {
            image,
            old_state,
            new_state,
            subresource_range: ImageSubresourceRange::color(),
        }
    }

    /// With subresource range
    pub const fn with_subresource(mut self, range: ImageSubresourceRange) -> Self {
        self.subresource_range = range;
        self
    }

    /// For depth aspect
    pub fn for_depth(mut self) -> Self {
        self.subresource_range.aspect_mask = ImageAspectFlags::DEPTH;
        self
    }

    /// Converts to image barrier
    pub const fn to_barrier(&self) -> ImageBarrier {
        ImageBarrier {
            src_access: self.old_state.access,
            dst_access: self.new_state.access,
            old_layout: self.old_state.layout,
            new_layout: self.new_state.layout,
            src_queue_family: self.old_state.queue_family,
            dst_queue_family: self.new_state.queue_family,
            image: self.image,
            subresource_range: self.subresource_range,
        }
    }

    /// Checks if barrier is needed
    pub const fn needs_barrier(&self) -> bool {
        // Need barrier if:
        // 1. Layout change
        // 2. Write to read hazard
        // 3. Write to write hazard
        // 4. Queue family transfer
        !matches!(
            (&self.old_state.layout, &self.new_state.layout),
            (a, b) if *a as u32 == *b as u32
        ) || self.old_state.access.is_write()
            || self.new_state.access.is_write()
            || (self.old_state.queue_family != self.new_state.queue_family
                && self.old_state.queue_family != QUEUE_FAMILY_IGNORED
                && self.new_state.queue_family != QUEUE_FAMILY_IGNORED)
    }
}

/// Common resource usage patterns
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceUsage {
    /// Not used
    None,
    /// Vertex buffer
    VertexBuffer,
    /// Index buffer
    IndexBuffer,
    /// Uniform buffer
    UniformBuffer,
    /// Storage buffer read
    StorageBufferRead,
    /// Storage buffer write
    StorageBufferWrite,
    /// Storage buffer read/write
    StorageBufferReadWrite,
    /// Indirect buffer
    IndirectBuffer,
    /// Transfer source
    TransferSrc,
    /// Transfer destination
    TransferDst,
    /// Sampled texture
    SampledTexture,
    /// Storage image read
    StorageImageRead,
    /// Storage image write
    StorageImageWrite,
    /// Color attachment
    ColorAttachment,
    /// Depth attachment
    DepthAttachment,
    /// Depth read
    DepthRead,
    /// Input attachment
    InputAttachment,
    /// Present
    Present,
}

impl ResourceUsage {
    /// Converts to buffer state
    pub const fn to_buffer_state(&self) -> BufferState {
        match self {
            Self::None => BufferState::new(AccessFlags::NONE, PipelineStageFlags::TOP_OF_PIPE),
            Self::VertexBuffer => BufferState::vertex_buffer(),
            Self::IndexBuffer => BufferState::index_buffer(),
            Self::UniformBuffer => BufferState::uniform_buffer(),
            Self::StorageBufferRead => BufferState::storage_read(),
            Self::StorageBufferWrite => BufferState::storage_write(),
            Self::StorageBufferReadWrite => BufferState::storage_read_write(),
            Self::IndirectBuffer => BufferState::indirect(),
            Self::TransferSrc => BufferState::transfer_src(),
            Self::TransferDst => BufferState::transfer_dst(),
            _ => BufferState::new(AccessFlags::NONE, PipelineStageFlags::TOP_OF_PIPE),
        }
    }

    /// Converts to image state
    pub const fn to_image_state(&self) -> ImageState {
        match self {
            Self::None => ImageState::new(AccessFlags::NONE, PipelineStageFlags::TOP_OF_PIPE, ImageLayout::Undefined),
            Self::SampledTexture => ImageState::shader_read(),
            Self::StorageImageRead => ImageState::storage_read(),
            Self::StorageImageWrite => ImageState::storage_write(),
            Self::ColorAttachment => ImageState::color_attachment(),
            Self::DepthAttachment => ImageState::depth_attachment(),
            Self::DepthRead => ImageState::depth_read(),
            Self::TransferSrc => ImageState::transfer_src(),
            Self::TransferDst => ImageState::transfer_dst(),
            Self::Present => ImageState::present(),
            Self::InputAttachment => ImageState::new(
                AccessFlags::INPUT_ATTACHMENT_READ,
                PipelineStageFlags::FRAGMENT_SHADER,
                ImageLayout::ShaderReadOnlyOptimal,
            ),
            _ => ImageState::new(AccessFlags::NONE, PipelineStageFlags::TOP_OF_PIPE, ImageLayout::Undefined),
        }
    }
}

/// Barrier batch for efficient barrier recording
#[derive(Clone, Debug, Default)]
pub struct BarrierBatch {
    /// Pipeline barrier info
    pub pipeline_barrier: PipelineBarrier,
    /// Memory barriers
    pub memory_barriers: [Option<MemoryBarrier>; 4],
    /// Memory barrier count
    pub memory_barrier_count: usize,
    /// Buffer barriers
    pub buffer_barriers: [Option<BufferBarrier>; 16],
    /// Buffer barrier count
    pub buffer_barrier_count: usize,
    /// Image barriers
    pub image_barriers: [Option<ImageBarrier>; 16],
    /// Image barrier count
    pub image_barrier_count: usize,
}

impl BarrierBatch {
    /// Creates a new empty batch
    pub const fn new() -> Self {
        Self {
            pipeline_barrier: PipelineBarrier {
                src_stage_mask: PipelineStageFlags::NONE,
                dst_stage_mask: PipelineStageFlags::NONE,
                dependency_flags: DependencyFlags::NONE,
            },
            memory_barriers: [None; 4],
            memory_barrier_count: 0,
            buffer_barriers: [None; 16],
            buffer_barrier_count: 0,
            image_barriers: [None; 16],
            image_barrier_count: 0,
        }
    }

    /// Adds a memory barrier
    pub fn add_memory_barrier(&mut self, barrier: MemoryBarrier) {
        if self.memory_barrier_count < 4 {
            self.memory_barriers[self.memory_barrier_count] = Some(barrier);
            self.memory_barrier_count += 1;
        }
    }

    /// Adds a buffer barrier
    pub fn add_buffer_barrier(&mut self, barrier: BufferBarrier, src_stage: PipelineStageFlags, dst_stage: PipelineStageFlags) {
        if self.buffer_barrier_count < 16 {
            self.buffer_barriers[self.buffer_barrier_count] = Some(barrier);
            self.buffer_barrier_count += 1;
            self.pipeline_barrier.src_stage_mask =
                PipelineStageFlags(self.pipeline_barrier.src_stage_mask.0 | src_stage.0);
            self.pipeline_barrier.dst_stage_mask =
                PipelineStageFlags(self.pipeline_barrier.dst_stage_mask.0 | dst_stage.0);
        }
    }

    /// Adds an image barrier
    pub fn add_image_barrier(&mut self, barrier: ImageBarrier, src_stage: PipelineStageFlags, dst_stage: PipelineStageFlags) {
        if self.image_barrier_count < 16 {
            self.image_barriers[self.image_barrier_count] = Some(barrier);
            self.image_barrier_count += 1;
            self.pipeline_barrier.src_stage_mask =
                PipelineStageFlags(self.pipeline_barrier.src_stage_mask.0 | src_stage.0);
            self.pipeline_barrier.dst_stage_mask =
                PipelineStageFlags(self.pipeline_barrier.dst_stage_mask.0 | dst_stage.0);
        }
    }

    /// Adds a buffer transition
    pub fn add_buffer_transition(&mut self, transition: BufferTransition) {
        if transition.needs_barrier() {
            self.add_buffer_barrier(
                transition.to_barrier(),
                transition.old_state.stage,
                transition.new_state.stage,
            );
        }
    }

    /// Adds an image transition
    pub fn add_image_transition(&mut self, transition: ImageTransition) {
        if transition.needs_barrier() {
            self.add_image_barrier(
                transition.to_barrier(),
                transition.old_state.stage,
                transition.new_state.stage,
            );
        }
    }

    /// Checks if batch is empty
    pub const fn is_empty(&self) -> bool {
        self.memory_barrier_count == 0
            && self.buffer_barrier_count == 0
            && self.image_barrier_count == 0
    }

    /// Clears the batch
    pub fn clear(&mut self) {
        self.pipeline_barrier = PipelineBarrier::new(
            PipelineStageFlags::NONE,
            PipelineStageFlags::NONE,
        );
        self.memory_barrier_count = 0;
        self.buffer_barrier_count = 0;
        self.image_barrier_count = 0;
    }
}

/// Global barrier (simple full barrier)
pub fn global_barrier() -> BarrierBatch {
    let mut batch = BarrierBatch::new();
    batch.pipeline_barrier = PipelineBarrier::full();
    batch.add_memory_barrier(MemoryBarrier::full());
    batch
}

/// Compute dispatch barrier
pub fn compute_dispatch_barrier() -> BarrierBatch {
    let mut batch = BarrierBatch::new();
    batch.pipeline_barrier = PipelineBarrier::compute_to_compute();
    batch.add_memory_barrier(MemoryBarrier::shader_read_after_write());
    batch
}
