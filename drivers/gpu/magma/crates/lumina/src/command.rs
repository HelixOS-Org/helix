//! Command recording and encoding
//!
//! This module provides types for recording GPU commands.

extern crate alloc;

use alloc::vec::Vec;

use crate::bind_group::BindGroupHandle;
use crate::draw::IndexFormat;
use crate::types::{BufferHandle, PipelineHandle, TextureHandle};

/// Command pool handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CommandPoolHandle(pub u64);

impl CommandPoolHandle {
    /// Null/invalid command pool
    pub const NULL: Self = Self(0);

    /// Creates from raw value
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns raw value
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Checks if valid
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

/// Command buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CommandBufferHandle(pub u64);

impl CommandBufferHandle {
    /// Null/invalid command buffer
    pub const NULL: Self = Self(0);

    /// Creates from raw value
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns raw value
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Checks if valid
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

/// Queue family type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum QueueFamily {
    /// Graphics queue
    #[default]
    Graphics,
    /// Compute queue
    Compute,
    /// Transfer queue
    Transfer,
}

/// Command pool flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct CommandPoolFlags(pub u32);

impl CommandPoolFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Buffers can be reset individually
    pub const RESET_COMMAND_BUFFER: Self = Self(1 << 0);
    /// Short-lived command buffers
    pub const TRANSIENT: Self = Self(1 << 1);
}

impl core::ops::BitOr for CommandPoolFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Command pool descriptor
#[derive(Clone, Debug)]
pub struct CommandPoolDesc<'a> {
    /// Debug label
    pub label: Option<&'a str>,
    /// Queue family
    pub queue_family: QueueFamily,
    /// Flags
    pub flags: CommandPoolFlags,
}

impl<'a> CommandPoolDesc<'a> {
    /// Creates graphics command pool
    pub const fn graphics() -> Self {
        Self {
            label: None,
            queue_family: QueueFamily::Graphics,
            flags: CommandPoolFlags::RESET_COMMAND_BUFFER,
        }
    }

    /// Creates compute command pool
    pub const fn compute() -> Self {
        Self {
            label: None,
            queue_family: QueueFamily::Compute,
            flags: CommandPoolFlags::RESET_COMMAND_BUFFER,
        }
    }

    /// Creates transfer command pool
    pub const fn transfer() -> Self {
        Self {
            label: None,
            queue_family: QueueFamily::Transfer,
            flags: CommandPoolFlags::TRANSIENT,
        }
    }

    /// Sets label
    pub const fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
}

/// Command buffer level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CommandBufferLevel {
    /// Primary (can submit directly)
    #[default]
    Primary,
    /// Secondary (called from primary)
    Secondary,
}

/// Command buffer usage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct CommandBufferUsageFlags(pub u32);

impl CommandBufferUsageFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// One-time submit
    pub const ONE_TIME_SUBMIT: Self = Self(1 << 0);
    /// Render pass continue
    pub const RENDER_PASS_CONTINUE: Self = Self(1 << 1);
    /// Simultaneous use
    pub const SIMULTANEOUS_USE: Self = Self(1 << 2);
}

impl core::ops::BitOr for CommandBufferUsageFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// A command encoder for recording GPU commands
pub struct CommandEncoder {
    commands: Vec<Command>,
    label: Option<&'static str>,
}

impl CommandEncoder {
    /// Creates a new command encoder
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            label: None,
        }
    }

    /// Creates with label
    pub fn with_label(label: &'static str) -> Self {
        Self {
            commands: Vec::new(),
            label: Some(label),
        }
    }

    /// Returns the recorded commands
    pub fn finish(self) -> Vec<Command> {
        self.commands
    }

    /// Clears the encoder for reuse
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// Records a command
    pub fn record(&mut self, command: Command) {
        self.commands.push(command);
    }

    /// Sets viewport
    pub fn set_viewport(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        min_depth: f32,
        max_depth: f32,
    ) {
        self.commands.push(Command::SetViewport {
            x,
            y,
            width,
            height,
            min_depth,
            max_depth,
        });
    }

    /// Sets scissor
    pub fn set_scissor(&mut self, x: i32, y: i32, width: u32, height: u32) {
        self.commands.push(Command::SetScissor {
            x,
            y,
            width,
            height,
        });
    }

    /// Binds graphics pipeline
    pub fn bind_graphics_pipeline(&mut self, pipeline: PipelineHandle) {
        self.commands
            .push(Command::BindGraphicsPipeline { pipeline });
    }

    /// Binds compute pipeline
    pub fn bind_compute_pipeline(&mut self, pipeline: PipelineHandle) {
        self.commands
            .push(Command::BindComputePipeline { pipeline });
    }

    /// Binds vertex buffers
    pub fn bind_vertex_buffers(
        &mut self,
        first_binding: u32,
        buffers: &[BufferHandle],
        offsets: &[u64],
    ) {
        self.commands.push(Command::BindVertexBuffers {
            first_binding,
            buffers: buffers.to_vec(),
            offsets: offsets.to_vec(),
        });
    }

    /// Binds index buffer
    pub fn bind_index_buffer(
        &mut self,
        buffer: BufferHandle,
        offset: u64,
        index_type: IndexFormat,
    ) {
        self.commands.push(Command::BindIndexBuffer {
            buffer,
            offset,
            index_type,
        });
    }

    /// Binds descriptor sets
    pub fn bind_descriptor_sets(
        &mut self,
        first_set: u32,
        sets: &[BindGroupHandle],
        dynamic_offsets: &[u32],
    ) {
        self.commands.push(Command::BindDescriptorSets {
            first_set,
            sets: sets.to_vec(),
            dynamic_offsets: dynamic_offsets.to_vec(),
        });
    }

    /// Sets push constants
    pub fn push_constants(&mut self, stages: u32, offset: u32, data: &[u8]) {
        self.commands.push(Command::SetPushConstants {
            stages,
            offset,
            data: data.to_vec(),
        });
    }

    /// Draws vertices
    pub fn draw(
        &mut self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        self.commands.push(Command::Draw {
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
        });
    }

    /// Draws indexed vertices
    pub fn draw_indexed(
        &mut self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        self.commands.push(Command::DrawIndexed {
            index_count,
            instance_count,
            first_index,
            vertex_offset,
            first_instance,
        });
    }

    /// Dispatches compute
    pub fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        self.commands.push(Command::Dispatch { x, y, z });
    }

    /// Dispatches compute indirect
    pub fn dispatch_indirect(&mut self, buffer: BufferHandle, offset: u64) {
        self.commands
            .push(Command::DispatchIndirect { buffer, offset });
    }

    /// Copies buffer to buffer
    pub fn copy_buffer(
        &mut self,
        src: BufferHandle,
        dst: BufferHandle,
        regions: &[BufferCopyRegion],
    ) {
        self.commands.push(Command::CopyBuffer {
            src,
            dst,
            regions: regions.to_vec(),
        });
    }

    /// Copies buffer to texture
    pub fn copy_buffer_to_texture(
        &mut self,
        src: BufferHandle,
        dst: TextureHandle,
        regions: &[BufferTextureCopyRegion],
    ) {
        self.commands.push(Command::CopyBufferToTexture {
            src,
            dst,
            regions: regions.to_vec(),
        });
    }

    /// Copies texture to buffer
    pub fn copy_texture_to_buffer(
        &mut self,
        src: TextureHandle,
        dst: BufferHandle,
        regions: &[BufferTextureCopyRegion],
    ) {
        self.commands.push(Command::CopyTextureToBuffer {
            src,
            dst,
            regions: regions.to_vec(),
        });
    }

    /// Pipeline barrier
    pub fn pipeline_barrier(
        &mut self,
        src_stage: u32,
        dst_stage: u32,
        memory_barriers: &[MemoryBarrier],
        buffer_barriers: &[BufferBarrier],
        image_barriers: &[ImageBarrier],
    ) {
        self.commands.push(Command::PipelineBarrier {
            src_stage,
            dst_stage,
            memory_barriers: memory_barriers.to_vec(),
            buffer_barriers: buffer_barriers.to_vec(),
            image_barriers: image_barriers.to_vec(),
        });
    }

    /// Begins render pass
    pub fn begin_render_pass(&mut self, desc: RenderPassBeginDesc) {
        self.commands.push(Command::BeginRenderPass(desc));
    }

    /// Ends render pass
    pub fn end_render_pass(&mut self) {
        self.commands.push(Command::EndRenderPass);
    }

    /// Sets blend constants
    pub fn set_blend_constants(&mut self, constants: [f32; 4]) {
        self.commands.push(Command::SetBlendConstants(constants));
    }

    /// Sets stencil reference
    pub fn set_stencil_reference(&mut self, reference: u32) {
        self.commands.push(Command::SetStencilReference(reference));
    }

    /// Sets depth bias
    pub fn set_depth_bias(&mut self, constant_factor: f32, clamp: f32, slope_factor: f32) {
        self.commands.push(Command::SetDepthBias {
            constant_factor,
            clamp,
            slope_factor,
        });
    }

    /// Sets line width
    pub fn set_line_width(&mut self, width: f32) {
        self.commands.push(Command::SetLineWidth(width));
    }

    /// Number of recorded commands
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }
}

impl Default for CommandEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Render pass begin descriptor
#[derive(Clone, Debug)]
pub struct RenderPassBeginDesc {
    /// Render area x
    pub x: i32,
    /// Render area y
    pub y: i32,
    /// Render area width
    pub width: u32,
    /// Render area height
    pub height: u32,
    /// Clear values
    pub clear_values: Vec<ClearValue>,
}

impl RenderPassBeginDesc {
    /// Creates render pass begin descriptor
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
            clear_values: Vec::new(),
        }
    }

    /// Sets offset
    pub fn with_offset(mut self, x: i32, y: i32) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    /// Adds color clear value
    pub fn with_color_clear(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.clear_values.push(ClearValue::Color([r, g, b, a]));
        self
    }

    /// Adds depth clear value
    pub fn with_depth_clear(mut self, depth: f32, stencil: u32) -> Self {
        self.clear_values
            .push(ClearValue::DepthStencil { depth, stencil });
        self
    }
}

/// Clear value
#[derive(Clone, Copy, Debug)]
pub enum ClearValue {
    /// Color clear value
    Color([f32; 4]),
    /// Depth/stencil clear value
    DepthStencil { depth: f32, stencil: u32 },
}

impl ClearValue {
    /// Black
    pub const BLACK: Self = Self::Color([0.0, 0.0, 0.0, 1.0]);
    /// White
    pub const WHITE: Self = Self::Color([1.0, 1.0, 1.0, 1.0]);
    /// Default depth
    pub const DEPTH_ONE: Self = Self::DepthStencil {
        depth: 1.0,
        stencil: 0,
    };
    /// Reversed depth
    pub const DEPTH_ZERO: Self = Self::DepthStencil {
        depth: 0.0,
        stencil: 0,
    };
}

/// Buffer copy region
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferCopyRegion {
    /// Source offset
    pub src_offset: u64,
    /// Destination offset
    pub dst_offset: u64,
    /// Size
    pub size: u64,
}

impl BufferCopyRegion {
    /// Creates copy region
    pub const fn new(src_offset: u64, dst_offset: u64, size: u64) -> Self {
        Self {
            src_offset,
            dst_offset,
            size,
        }
    }

    /// Full copy from start
    pub const fn full(size: u64) -> Self {
        Self {
            src_offset: 0,
            dst_offset: 0,
            size,
        }
    }
}

/// Buffer to texture copy region
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferTextureCopyRegion {
    /// Buffer offset
    pub buffer_offset: u64,
    /// Buffer row length (0 = tightly packed)
    pub buffer_row_length: u32,
    /// Buffer image height (0 = tightly packed)
    pub buffer_image_height: u32,
    /// Texture mip level
    pub mip_level: u32,
    /// Texture array layer
    pub array_layer: u32,
    /// Texture offset x
    pub offset_x: i32,
    /// Texture offset y
    pub offset_y: i32,
    /// Texture offset z
    pub offset_z: i32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth
    pub depth: u32,
}

impl BufferTextureCopyRegion {
    /// Creates 2D copy region
    pub const fn texture_2d(buffer_offset: u64, width: u32, height: u32, mip_level: u32) -> Self {
        Self {
            buffer_offset,
            buffer_row_length: 0,
            buffer_image_height: 0,
            mip_level,
            array_layer: 0,
            offset_x: 0,
            offset_y: 0,
            offset_z: 0,
            width,
            height,
            depth: 1,
        }
    }
}

/// Memory barrier
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryBarrier {
    /// Source access mask
    pub src_access: u32,
    /// Destination access mask
    pub dst_access: u32,
}

impl MemoryBarrier {
    /// Creates memory barrier
    pub const fn new(src_access: u32, dst_access: u32) -> Self {
        Self {
            src_access,
            dst_access,
        }
    }
}

/// Buffer barrier
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferBarrier {
    /// Source access mask
    pub src_access: u32,
    /// Destination access mask
    pub dst_access: u32,
    /// Source queue family
    pub src_queue_family: u32,
    /// Destination queue family
    pub dst_queue_family: u32,
    /// Buffer
    pub buffer: BufferHandle,
    /// Offset
    pub offset: u64,
    /// Size
    pub size: u64,
}

impl BufferBarrier {
    /// Creates buffer barrier
    pub const fn new(buffer: BufferHandle, src_access: u32, dst_access: u32) -> Self {
        Self {
            src_access,
            dst_access,
            src_queue_family: 0xFFFFFFFF,
            dst_queue_family: 0xFFFFFFFF,
            buffer,
            offset: 0,
            size: u64::MAX,
        }
    }
}

/// Image barrier
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageBarrier {
    /// Source access mask
    pub src_access: u32,
    /// Destination access mask
    pub dst_access: u32,
    /// Old layout
    pub old_layout: ImageLayout,
    /// New layout
    pub new_layout: ImageLayout,
    /// Source queue family
    pub src_queue_family: u32,
    /// Destination queue family
    pub dst_queue_family: u32,
    /// Texture
    pub texture: TextureHandle,
    /// Base mip level
    pub base_mip_level: u32,
    /// Mip level count
    pub level_count: u32,
    /// Base array layer
    pub base_array_layer: u32,
    /// Layer count
    pub layer_count: u32,
}

impl ImageBarrier {
    /// Creates image layout transition
    pub const fn transition(
        texture: TextureHandle,
        old_layout: ImageLayout,
        new_layout: ImageLayout,
    ) -> Self {
        Self {
            src_access: 0,
            dst_access: 0,
            old_layout,
            new_layout,
            src_queue_family: 0xFFFFFFFF,
            dst_queue_family: 0xFFFFFFFF,
            texture,
            base_mip_level: 0,
            level_count: 0xFFFFFFFF,
            base_array_layer: 0,
            layer_count: 0xFFFFFFFF,
        }
    }
}

/// Image layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ImageLayout {
    /// Undefined
    #[default]
    Undefined,
    /// General
    General,
    /// Color attachment
    ColorAttachment,
    /// Depth stencil attachment
    DepthStencilAttachment,
    /// Depth stencil read-only
    DepthStencilReadOnly,
    /// Shader read-only
    ShaderReadOnly,
    /// Transfer source
    TransferSrc,
    /// Transfer destination
    TransferDst,
    /// Present
    Present,
}

/// A GPU command
#[derive(Clone, Debug)]
pub enum Command {
    /// Set viewport
    SetViewport {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        min_depth: f32,
        max_depth: f32,
    },

    /// Set scissor rectangle
    SetScissor {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },

    /// Bind graphics pipeline
    BindGraphicsPipeline { pipeline: PipelineHandle },

    /// Bind compute pipeline
    BindComputePipeline { pipeline: PipelineHandle },

    /// Bind vertex buffers
    BindVertexBuffers {
        first_binding: u32,
        buffers: Vec<BufferHandle>,
        offsets: Vec<u64>,
    },

    /// Bind index buffer
    BindIndexBuffer {
        buffer: BufferHandle,
        offset: u64,
        index_type: IndexFormat,
    },

    /// Bind descriptor sets
    BindDescriptorSets {
        first_set: u32,
        sets: Vec<BindGroupHandle>,
        dynamic_offsets: Vec<u32>,
    },

    /// Set push constants
    SetPushConstants {
        stages: u32,
        offset: u32,
        data: Vec<u8>,
    },

    /// Draw vertices
    Draw {
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    },

    /// Draw indexed vertices
    DrawIndexed {
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    },

    /// Dispatch compute shader
    Dispatch { x: u32, y: u32, z: u32 },

    /// Dispatch indirect
    DispatchIndirect { buffer: BufferHandle, offset: u64 },

    /// Draw indirect
    DrawIndirect {
        buffer: BufferHandle,
        offset: u64,
        draw_count: u32,
        stride: u32,
    },

    /// Draw indexed indirect
    DrawIndexedIndirect {
        buffer: BufferHandle,
        offset: u64,
        draw_count: u32,
        stride: u32,
    },

    /// Copy buffer
    CopyBuffer {
        src: BufferHandle,
        dst: BufferHandle,
        regions: Vec<BufferCopyRegion>,
    },

    /// Copy buffer to texture
    CopyBufferToTexture {
        src: BufferHandle,
        dst: TextureHandle,
        regions: Vec<BufferTextureCopyRegion>,
    },

    /// Copy texture to buffer
    CopyTextureToBuffer {
        src: TextureHandle,
        dst: BufferHandle,
        regions: Vec<BufferTextureCopyRegion>,
    },

    /// Copy texture to texture
    CopyTexture {
        src: TextureHandle,
        dst: TextureHandle,
    },

    /// Pipeline barrier
    PipelineBarrier {
        src_stage: u32,
        dst_stage: u32,
        memory_barriers: Vec<MemoryBarrier>,
        buffer_barriers: Vec<BufferBarrier>,
        image_barriers: Vec<ImageBarrier>,
    },

    /// Begin render pass
    BeginRenderPass(RenderPassBeginDesc),

    /// End render pass
    EndRenderPass,

    /// Set blend constants
    SetBlendConstants([f32; 4]),

    /// Set stencil reference
    SetStencilReference(u32),

    /// Set depth bias
    SetDepthBias {
        constant_factor: f32,
        clamp: f32,
        slope_factor: f32,
    },

    /// Set line width
    SetLineWidth(f32),

    /// Begin query
    BeginQuery {
        query_pool: crate::query::QueryPoolHandle,
        query: u32,
    },

    /// End query
    EndQuery {
        query_pool: crate::query::QueryPoolHandle,
        query: u32,
    },

    /// Write timestamp
    WriteTimestamp {
        pipeline_stage: u32,
        query_pool: crate::query::QueryPoolHandle,
        query: u32,
    },

    /// Reset query pool
    ResetQueryPool {
        query_pool: crate::query::QueryPoolHandle,
        first_query: u32,
        query_count: u32,
    },

    /// Execute secondary command buffers
    ExecuteCommands {
        command_buffers: Vec<CommandBufferHandle>,
    },

    /// Push debug group
    PushDebugGroup { name: &'static str, color: [f32; 4] },

    /// Pop debug group
    PopDebugGroup,

    /// Insert debug label
    InsertDebugLabel { name: &'static str, color: [f32; 4] },

    /// Fill buffer
    FillBuffer {
        buffer: BufferHandle,
        offset: u64,
        size: u64,
        data: u32,
    },

    /// Update buffer (small inline update)
    UpdateBuffer {
        buffer: BufferHandle,
        offset: u64,
        data: Vec<u8>,
    },

    /// Clear color image
    ClearColorImage {
        image: TextureHandle,
        layout: ImageLayout,
        color: [f32; 4],
    },

    /// Clear depth stencil image
    ClearDepthStencilImage {
        image: TextureHandle,
        layout: ImageLayout,
        depth: f32,
        stencil: u32,
    },

    /// Resolve multisampled image
    ResolveImage {
        src: TextureHandle,
        dst: TextureHandle,
    },

    /// Blit image
    BlitImage {
        src: TextureHandle,
        dst: TextureHandle,
        filter: crate::types::Filter,
    },

    /// Set event
    SetEvent {
        event: crate::sync::EventHandle,
        stage_mask: u32,
    },

    /// Reset event
    ResetEvent {
        event: crate::sync::EventHandle,
        stage_mask: u32,
    },

    /// Wait events
    WaitEvents {
        events: Vec<crate::sync::EventHandle>,
        src_stage: u32,
        dst_stage: u32,
    },
}
