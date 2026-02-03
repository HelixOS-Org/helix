//! Command Buffer Management
//!
//! GPU command recording and submission.

use alloc::{string::String, vec::Vec, boxed::Box};
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use bitflags::bitflags;
use lumina_core::Handle;
use lumina_math::{Vec4, Mat4};

use crate::device::TextureFormat;
use crate::buffer::BufferHandle;
use crate::texture::TextureHandle;
use crate::pipeline::{RenderPipelineHandle, ComputePipelineHandle};
use crate::descriptor::DescriptorSetHandle;

// ============================================================================
// Command Buffer Level
// ============================================================================

/// Command buffer level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandBufferLevel {
    /// Primary command buffer (can be submitted directly).
    Primary,
    /// Secondary command buffer (executed from primary).
    Secondary,
}

impl Default for CommandBufferLevel {
    fn default() -> Self {
        CommandBufferLevel::Primary
    }
}

// ============================================================================
// Command Buffer State
// ============================================================================

/// Command buffer state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandBufferState {
    /// Initial state (not recording).
    Initial,
    /// Recording commands.
    Recording,
    /// Executable (ready for submission).
    Executable,
    /// Pending execution.
    Pending,
    /// Invalid (needs reset).
    Invalid,
}

// ============================================================================
// Command Buffer Flags
// ============================================================================

bitflags! {
    /// Command buffer usage flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CommandBufferFlags: u32 {
        /// Buffer will be submitted once.
        const ONE_TIME_SUBMIT = 1 << 0;
        /// Buffer can be resubmitted while pending.
        const SIMULTANEOUS_USE = 1 << 1;
        /// Secondary buffer will be entirely inside render pass.
        const RENDER_PASS_CONTINUE = 1 << 2;
    }
}

impl Default for CommandBufferFlags {
    fn default() -> Self {
        CommandBufferFlags::empty()
    }
}

// ============================================================================
// Command Buffer Description
// ============================================================================

/// Description for command buffer creation.
#[derive(Debug, Clone)]
pub struct CommandBufferDesc {
    /// Level.
    pub level: CommandBufferLevel,
    /// Flags.
    pub flags: CommandBufferFlags,
}

impl Default for CommandBufferDesc {
    fn default() -> Self {
        Self {
            level: CommandBufferLevel::Primary,
            flags: CommandBufferFlags::empty(),
        }
    }
}

// ============================================================================
// Command Pool
// ============================================================================

/// Handle to a command pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandPoolHandle(Handle<CommandPool>);

/// Command pool for allocating command buffers.
pub struct CommandPool {
    /// Handle.
    pub handle: CommandPoolHandle,
    /// Queue family index.
    pub queue_family: u32,
    /// Allocated buffer count.
    allocated_count: AtomicU32,
}

impl CommandPool {
    /// Create a new command pool.
    pub fn new(handle: CommandPoolHandle, queue_family: u32) -> Self {
        Self {
            handle,
            queue_family,
            allocated_count: AtomicU32::new(0),
        }
    }

    /// Get allocated count.
    pub fn allocated_count(&self) -> u32 {
        self.allocated_count.load(Ordering::Relaxed)
    }

    /// Allocate a command buffer.
    pub fn allocate(&self) -> u32 {
        self.allocated_count.fetch_add(1, Ordering::Relaxed)
    }

    /// Reset pool.
    pub fn reset(&self) {
        self.allocated_count.store(0, Ordering::Relaxed);
    }
}

// ============================================================================
// Command Buffer Handle
// ============================================================================

/// Handle to a command buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandBufferHandle(Handle<CommandBuffer>);

impl CommandBufferHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self(Handle::from_raw_parts(index, generation))
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.0.index()
    }
}

// ============================================================================
// Viewport
// ============================================================================

/// Viewport definition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// X offset.
    pub x: f32,
    /// Y offset.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
    /// Minimum depth.
    pub min_depth: f32,
    /// Maximum depth.
    pub max_depth: f32,
}

impl Viewport {
    /// Create a new viewport.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }

    /// Create from dimensions.
    pub fn from_size(width: u32, height: u32) -> Self {
        Self::new(0.0, 0.0, width as f32, height as f32)
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self::new(0.0, 0.0, 1.0, 1.0)
    }
}

// ============================================================================
// Scissor
// ============================================================================

/// Scissor rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scissor {
    /// X offset.
    pub x: i32,
    /// Y offset.
    pub y: i32,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

impl Scissor {
    /// Create a new scissor.
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Create from dimensions.
    pub fn from_size(width: u32, height: u32) -> Self {
        Self::new(0, 0, width, height)
    }
}

impl Default for Scissor {
    fn default() -> Self {
        Self::new(0, 0, 1, 1)
    }
}

// ============================================================================
// Load/Store Operations
// ============================================================================

/// Load operation for attachments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoadOp {
    /// Load existing contents.
    Load,
    /// Clear to a value.
    Clear,
    /// Don't care about contents.
    DontCare,
}

/// Store operation for attachments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoreOp {
    /// Store contents.
    Store,
    /// Don't care about contents.
    DontCare,
}

// ============================================================================
// Clear Value
// ============================================================================

/// Clear value for attachments.
#[derive(Debug, Clone, Copy)]
pub enum ClearValue {
    /// Color clear value.
    Color(f32, f32, f32, f32),
    /// Depth/stencil clear value.
    DepthStencil(f32, u32),
}

impl ClearValue {
    /// Create color clear value.
    pub fn color(r: f32, g: f32, b: f32, a: f32) -> Self {
        ClearValue::Color(r, g, b, a)
    }

    /// Create depth/stencil clear value.
    pub fn depth_stencil(depth: f32, stencil: u32) -> Self {
        ClearValue::DepthStencil(depth, stencil)
    }

    /// Black color.
    pub fn black() -> Self {
        ClearValue::Color(0.0, 0.0, 0.0, 1.0)
    }

    /// White color.
    pub fn white() -> Self {
        ClearValue::Color(1.0, 1.0, 1.0, 1.0)
    }

    /// Default depth.
    pub fn depth() -> Self {
        ClearValue::DepthStencil(1.0, 0)
    }
}

impl Default for ClearValue {
    fn default() -> Self {
        ClearValue::black()
    }
}

// ============================================================================
// Render Attachment
// ============================================================================

/// Render attachment description.
#[derive(Debug, Clone)]
pub struct RenderAttachment {
    /// Texture view.
    pub texture: TextureHandle,
    /// Resolve texture (for MSAA).
    pub resolve_target: Option<TextureHandle>,
    /// Load operation.
    pub load_op: LoadOp,
    /// Store operation.
    pub store_op: StoreOp,
    /// Clear value.
    pub clear_value: ClearValue,
}

impl RenderAttachment {
    /// Create a new attachment.
    pub fn new(texture: TextureHandle) -> Self {
        Self {
            texture,
            resolve_target: None,
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            clear_value: ClearValue::black(),
        }
    }

    /// Set clear value.
    pub fn with_clear(mut self, value: ClearValue) -> Self {
        self.clear_value = value;
        self
    }

    /// Set load operation.
    pub fn with_load(mut self, op: LoadOp) -> Self {
        self.load_op = op;
        self
    }

    /// Set store operation.
    pub fn with_store(mut self, op: StoreOp) -> Self {
        self.store_op = op;
        self
    }

    /// Set resolve target.
    pub fn with_resolve(mut self, target: TextureHandle) -> Self {
        self.resolve_target = Some(target);
        self
    }
}

// ============================================================================
// Render Pass Description
// ============================================================================

/// Description for a render pass.
#[derive(Debug, Clone)]
pub struct RenderPassDesc {
    /// Color attachments.
    pub color_attachments: Vec<RenderAttachment>,
    /// Depth/stencil attachment.
    pub depth_stencil_attachment: Option<RenderAttachment>,
    /// Render area.
    pub render_area: Scissor,
    /// Occlusion query set.
    pub occlusion_query_set: Option<u32>,
    /// Timestamp writes.
    pub timestamp_writes: Option<(u32, u32)>,
}

impl Default for RenderPassDesc {
    fn default() -> Self {
        Self {
            color_attachments: Vec::new(),
            depth_stencil_attachment: None,
            render_area: Scissor::default(),
            occlusion_query_set: None,
            timestamp_writes: None,
        }
    }
}

// ============================================================================
// Index Type
// ============================================================================

/// Index buffer type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexType {
    /// 16-bit indices.
    Uint16,
    /// 32-bit indices.
    Uint32,
}

impl IndexType {
    /// Get bytes per index.
    pub fn size(&self) -> u32 {
        match self {
            IndexType::Uint16 => 2,
            IndexType::Uint32 => 4,
        }
    }
}

// ============================================================================
// Draw Commands
// ============================================================================

/// Draw indexed command.
#[derive(Debug, Clone, Copy)]
pub struct DrawIndexedCommand {
    /// Index count.
    pub index_count: u32,
    /// Instance count.
    pub instance_count: u32,
    /// First index.
    pub first_index: u32,
    /// Vertex offset.
    pub vertex_offset: i32,
    /// First instance.
    pub first_instance: u32,
}

/// Draw command.
#[derive(Debug, Clone, Copy)]
pub struct DrawCommand {
    /// Vertex count.
    pub vertex_count: u32,
    /// Instance count.
    pub instance_count: u32,
    /// First vertex.
    pub first_vertex: u32,
    /// First instance.
    pub first_instance: u32,
}

/// Dispatch command.
#[derive(Debug, Clone, Copy)]
pub struct DispatchCommand {
    /// X groups.
    pub x: u32,
    /// Y groups.
    pub y: u32,
    /// Z groups.
    pub z: u32,
}

// ============================================================================
// Render Pass Encoder
// ============================================================================

/// Encoder for render pass commands.
pub struct RenderPassEncoder<'a> {
    /// Command buffer.
    command_buffer: &'a mut CommandBuffer,
    /// Is active.
    active: bool,
}

impl<'a> RenderPassEncoder<'a> {
    /// Create a new encoder.
    pub fn new(command_buffer: &'a mut CommandBuffer) -> Self {
        Self {
            command_buffer,
            active: true,
        }
    }

    /// Set pipeline.
    pub fn set_pipeline(&mut self, pipeline: RenderPipelineHandle) {
        self.command_buffer.commands.push(Command::SetRenderPipeline(pipeline));
    }

    /// Set viewport.
    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.command_buffer.commands.push(Command::SetViewport(viewport));
    }

    /// Set scissor.
    pub fn set_scissor(&mut self, scissor: Scissor) {
        self.command_buffer.commands.push(Command::SetScissor(scissor));
    }

    /// Set blend constant.
    pub fn set_blend_constant(&mut self, color: Vec4) {
        self.command_buffer.commands.push(Command::SetBlendConstant(color));
    }

    /// Set stencil reference.
    pub fn set_stencil_reference(&mut self, reference: u32) {
        self.command_buffer.commands.push(Command::SetStencilReference(reference));
    }

    /// Set vertex buffer.
    pub fn set_vertex_buffer(&mut self, slot: u32, buffer: BufferHandle, offset: u64) {
        self.command_buffer.commands.push(Command::SetVertexBuffer { slot, buffer, offset });
    }

    /// Set index buffer.
    pub fn set_index_buffer(&mut self, buffer: BufferHandle, index_type: IndexType, offset: u64) {
        self.command_buffer.commands.push(Command::SetIndexBuffer { buffer, index_type, offset });
    }

    /// Set bind group.
    pub fn set_bind_group(&mut self, index: u32, bind_group: DescriptorSetHandle, dynamic_offsets: &[u32]) {
        self.command_buffer.commands.push(Command::SetBindGroup {
            index,
            bind_group,
            dynamic_offsets: dynamic_offsets.to_vec(),
        });
    }

    /// Set push constants.
    pub fn set_push_constants(&mut self, stages: u32, offset: u32, data: &[u8]) {
        self.command_buffer.commands.push(Command::SetPushConstants {
            stages,
            offset,
            data: data.to_vec(),
        });
    }

    /// Draw.
    pub fn draw(&mut self, vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32) {
        self.command_buffer.commands.push(Command::Draw(DrawCommand {
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
        }));
        self.command_buffer.draw_count += 1;
    }

    /// Draw indexed.
    pub fn draw_indexed(
        &mut self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        self.command_buffer.commands.push(Command::DrawIndexed(DrawIndexedCommand {
            index_count,
            instance_count,
            first_index,
            vertex_offset,
            first_instance,
        }));
        self.command_buffer.draw_count += 1;
    }

    /// Draw indirect.
    pub fn draw_indirect(&mut self, buffer: BufferHandle, offset: u64) {
        self.command_buffer.commands.push(Command::DrawIndirect { buffer, offset });
        self.command_buffer.draw_count += 1;
    }

    /// Draw indexed indirect.
    pub fn draw_indexed_indirect(&mut self, buffer: BufferHandle, offset: u64) {
        self.command_buffer.commands.push(Command::DrawIndexedIndirect { buffer, offset });
        self.command_buffer.draw_count += 1;
    }

    /// Multi-draw indirect.
    pub fn multi_draw_indirect(&mut self, buffer: BufferHandle, offset: u64, count: u32, stride: u32) {
        self.command_buffer.commands.push(Command::MultiDrawIndirect { buffer, offset, count, stride });
        self.command_buffer.draw_count += count as u64;
    }

    /// Multi-draw indexed indirect.
    pub fn multi_draw_indexed_indirect(&mut self, buffer: BufferHandle, offset: u64, count: u32, stride: u32) {
        self.command_buffer.commands.push(Command::MultiDrawIndexedIndirect { buffer, offset, count, stride });
        self.command_buffer.draw_count += count as u64;
    }

    /// Multi-draw indirect count.
    pub fn multi_draw_indirect_count(
        &mut self,
        buffer: BufferHandle,
        offset: u64,
        count_buffer: BufferHandle,
        count_offset: u64,
        max_count: u32,
        stride: u32,
    ) {
        self.command_buffer.commands.push(Command::MultiDrawIndirectCount {
            buffer,
            offset,
            count_buffer,
            count_offset,
            max_count,
            stride,
        });
        self.command_buffer.draw_count += 1;
    }

    /// End render pass.
    pub fn end(mut self) {
        self.command_buffer.commands.push(Command::EndRenderPass);
        self.active = false;
    }
}

impl<'a> Drop for RenderPassEncoder<'a> {
    fn drop(&mut self) {
        if self.active {
            self.command_buffer.commands.push(Command::EndRenderPass);
        }
    }
}

// ============================================================================
// Compute Pass Encoder
// ============================================================================

/// Encoder for compute pass commands.
pub struct ComputePassEncoder<'a> {
    /// Command buffer.
    command_buffer: &'a mut CommandBuffer,
    /// Is active.
    active: bool,
}

impl<'a> ComputePassEncoder<'a> {
    /// Create a new encoder.
    pub fn new(command_buffer: &'a mut CommandBuffer) -> Self {
        Self {
            command_buffer,
            active: true,
        }
    }

    /// Set pipeline.
    pub fn set_pipeline(&mut self, pipeline: ComputePipelineHandle) {
        self.command_buffer.commands.push(Command::SetComputePipeline(pipeline));
    }

    /// Set bind group.
    pub fn set_bind_group(&mut self, index: u32, bind_group: DescriptorSetHandle, dynamic_offsets: &[u32]) {
        self.command_buffer.commands.push(Command::SetBindGroup {
            index,
            bind_group,
            dynamic_offsets: dynamic_offsets.to_vec(),
        });
    }

    /// Set push constants.
    pub fn set_push_constants(&mut self, stages: u32, offset: u32, data: &[u8]) {
        self.command_buffer.commands.push(Command::SetPushConstants {
            stages,
            offset,
            data: data.to_vec(),
        });
    }

    /// Dispatch.
    pub fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        self.command_buffer.commands.push(Command::Dispatch(DispatchCommand { x, y, z }));
        self.command_buffer.dispatch_count += 1;
    }

    /// Dispatch indirect.
    pub fn dispatch_indirect(&mut self, buffer: BufferHandle, offset: u64) {
        self.command_buffer.commands.push(Command::DispatchIndirect { buffer, offset });
        self.command_buffer.dispatch_count += 1;
    }

    /// End compute pass.
    pub fn end(mut self) {
        self.command_buffer.commands.push(Command::EndComputePass);
        self.active = false;
    }
}

impl<'a> Drop for ComputePassEncoder<'a> {
    fn drop(&mut self) {
        if self.active {
            self.command_buffer.commands.push(Command::EndComputePass);
        }
    }
}

// ============================================================================
// Transfer Encoder
// ============================================================================

/// Encoder for transfer commands.
pub struct TransferEncoder<'a> {
    /// Command buffer.
    command_buffer: &'a mut CommandBuffer,
}

impl<'a> TransferEncoder<'a> {
    /// Create a new encoder.
    pub fn new(command_buffer: &'a mut CommandBuffer) -> Self {
        Self { command_buffer }
    }

    /// Copy buffer to buffer.
    pub fn copy_buffer_to_buffer(
        &mut self,
        src: BufferHandle,
        src_offset: u64,
        dst: BufferHandle,
        dst_offset: u64,
        size: u64,
    ) {
        self.command_buffer.commands.push(Command::CopyBufferToBuffer {
            src,
            src_offset,
            dst,
            dst_offset,
            size,
        });
    }

    /// Copy buffer to texture.
    pub fn copy_buffer_to_texture(
        &mut self,
        src: BufferHandle,
        src_layout: BufferImageCopy,
        dst: TextureHandle,
        dst_origin: [u32; 3],
        extent: [u32; 3],
    ) {
        self.command_buffer.commands.push(Command::CopyBufferToTexture {
            src,
            src_layout,
            dst,
            dst_origin,
            extent,
        });
    }

    /// Copy texture to buffer.
    pub fn copy_texture_to_buffer(
        &mut self,
        src: TextureHandle,
        src_origin: [u32; 3],
        dst: BufferHandle,
        dst_layout: BufferImageCopy,
        extent: [u32; 3],
    ) {
        self.command_buffer.commands.push(Command::CopyTextureToBuffer {
            src,
            src_origin,
            dst,
            dst_layout,
            extent,
        });
    }

    /// Copy texture to texture.
    pub fn copy_texture_to_texture(
        &mut self,
        src: TextureHandle,
        src_origin: [u32; 3],
        dst: TextureHandle,
        dst_origin: [u32; 3],
        extent: [u32; 3],
    ) {
        self.command_buffer.commands.push(Command::CopyTextureToTexture {
            src,
            src_origin,
            dst,
            dst_origin,
            extent,
        });
    }

    /// Fill buffer.
    pub fn fill_buffer(&mut self, buffer: BufferHandle, offset: u64, size: u64, value: u32) {
        self.command_buffer.commands.push(Command::FillBuffer { buffer, offset, size, value });
    }

    /// Clear texture.
    pub fn clear_texture(&mut self, texture: TextureHandle, clear_value: ClearValue) {
        self.command_buffer.commands.push(Command::ClearTexture { texture, clear_value });
    }
}

/// Buffer image copy layout.
#[derive(Debug, Clone, Copy)]
pub struct BufferImageCopy {
    /// Offset in bytes.
    pub offset: u64,
    /// Bytes per row.
    pub bytes_per_row: u32,
    /// Rows per image.
    pub rows_per_image: u32,
}

// ============================================================================
// Command
// ============================================================================

/// GPU command.
#[derive(Debug, Clone)]
pub enum Command {
    // Render pass
    BeginRenderPass(RenderPassDesc),
    EndRenderPass,
    SetRenderPipeline(RenderPipelineHandle),
    SetViewport(Viewport),
    SetScissor(Scissor),
    SetBlendConstant(Vec4),
    SetStencilReference(u32),
    SetVertexBuffer { slot: u32, buffer: BufferHandle, offset: u64 },
    SetIndexBuffer { buffer: BufferHandle, index_type: IndexType, offset: u64 },
    SetBindGroup { index: u32, bind_group: DescriptorSetHandle, dynamic_offsets: Vec<u32> },
    SetPushConstants { stages: u32, offset: u32, data: Vec<u8> },

    // Draw commands
    Draw(DrawCommand),
    DrawIndexed(DrawIndexedCommand),
    DrawIndirect { buffer: BufferHandle, offset: u64 },
    DrawIndexedIndirect { buffer: BufferHandle, offset: u64 },
    MultiDrawIndirect { buffer: BufferHandle, offset: u64, count: u32, stride: u32 },
    MultiDrawIndexedIndirect { buffer: BufferHandle, offset: u64, count: u32, stride: u32 },
    MultiDrawIndirectCount { buffer: BufferHandle, offset: u64, count_buffer: BufferHandle, count_offset: u64, max_count: u32, stride: u32 },

    // Compute pass
    BeginComputePass,
    EndComputePass,
    SetComputePipeline(ComputePipelineHandle),
    Dispatch(DispatchCommand),
    DispatchIndirect { buffer: BufferHandle, offset: u64 },

    // Transfer commands
    CopyBufferToBuffer { src: BufferHandle, src_offset: u64, dst: BufferHandle, dst_offset: u64, size: u64 },
    CopyBufferToTexture { src: BufferHandle, src_layout: BufferImageCopy, dst: TextureHandle, dst_origin: [u32; 3], extent: [u32; 3] },
    CopyTextureToBuffer { src: TextureHandle, src_origin: [u32; 3], dst: BufferHandle, dst_layout: BufferImageCopy, extent: [u32; 3] },
    CopyTextureToTexture { src: TextureHandle, src_origin: [u32; 3], dst: TextureHandle, dst_origin: [u32; 3], extent: [u32; 3] },
    FillBuffer { buffer: BufferHandle, offset: u64, size: u64, value: u32 },
    ClearTexture { texture: TextureHandle, clear_value: ClearValue },

    // Synchronization
    PipelineBarrier,
    BufferBarrier { buffer: BufferHandle },
    TextureBarrier { texture: TextureHandle },

    // Debug
    PushDebugGroup(String),
    PopDebugGroup,
    InsertDebugMarker(String),
}

// ============================================================================
// Command Buffer
// ============================================================================

/// A GPU command buffer.
pub struct CommandBuffer {
    /// Handle.
    pub handle: CommandBufferHandle,
    /// Level.
    pub level: CommandBufferLevel,
    /// State.
    pub state: CommandBufferState,
    /// Flags.
    pub flags: CommandBufferFlags,
    /// Commands.
    pub commands: Vec<Command>,
    /// Draw count.
    pub draw_count: u64,
    /// Dispatch count.
    pub dispatch_count: u64,
}

impl CommandBuffer {
    /// Create a new command buffer.
    pub fn new(handle: CommandBufferHandle, desc: &CommandBufferDesc) -> Self {
        Self {
            handle,
            level: desc.level,
            state: CommandBufferState::Initial,
            flags: desc.flags,
            commands: Vec::new(),
            draw_count: 0,
            dispatch_count: 0,
        }
    }

    /// Begin recording.
    pub fn begin(&mut self) {
        self.state = CommandBufferState::Recording;
        self.commands.clear();
        self.draw_count = 0;
        self.dispatch_count = 0;
    }

    /// End recording.
    pub fn end(&mut self) {
        self.state = CommandBufferState::Executable;
    }

    /// Reset command buffer.
    pub fn reset(&mut self) {
        self.state = CommandBufferState::Initial;
        self.commands.clear();
        self.draw_count = 0;
        self.dispatch_count = 0;
    }

    /// Begin render pass.
    pub fn begin_render_pass(&mut self, desc: RenderPassDesc) -> RenderPassEncoder<'_> {
        self.commands.push(Command::BeginRenderPass(desc));
        RenderPassEncoder::new(self)
    }

    /// Begin compute pass.
    pub fn begin_compute_pass(&mut self) -> ComputePassEncoder<'_> {
        self.commands.push(Command::BeginComputePass);
        ComputePassEncoder::new(self)
    }

    /// Get transfer encoder.
    pub fn transfer(&mut self) -> TransferEncoder<'_> {
        TransferEncoder::new(self)
    }

    /// Push debug group.
    pub fn push_debug_group(&mut self, label: impl Into<String>) {
        self.commands.push(Command::PushDebugGroup(label.into()));
    }

    /// Pop debug group.
    pub fn pop_debug_group(&mut self) {
        self.commands.push(Command::PopDebugGroup);
    }

    /// Insert debug marker.
    pub fn insert_debug_marker(&mut self, label: impl Into<String>) {
        self.commands.push(Command::InsertDebugMarker(label.into()));
    }

    /// Insert pipeline barrier.
    pub fn pipeline_barrier(&mut self) {
        self.commands.push(Command::PipelineBarrier);
    }

    /// Insert buffer barrier.
    pub fn buffer_barrier(&mut self, buffer: BufferHandle) {
        self.commands.push(Command::BufferBarrier { buffer });
    }

    /// Insert texture barrier.
    pub fn texture_barrier(&mut self, texture: TextureHandle) {
        self.commands.push(Command::TextureBarrier { texture });
    }

    /// Get command count.
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
