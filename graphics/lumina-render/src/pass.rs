//! Render Pass System - Modular Rendering Pipeline
//!
//! This module provides a flexible pass system for building complex rendering pipelines.
//! Each pass encapsulates a specific rendering operation with well-defined inputs and outputs.

use alloc::{boxed::Box, string::String, vec::Vec};
use core::fmt;

use crate::barrier::{AccessFlags, PipelineStage};
use crate::graph::{ResourceId, SubresourceRange, VirtualBufferHandle, VirtualTextureHandle};
use crate::resource::{BufferHandle, ResourceState, TextureHandle};
use crate::target::{Attachment, RenderTarget};
use crate::view::View;

/// Types of render passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassType {
    /// Graphics rendering pass.
    Graphics,
    /// Compute dispatch pass.
    Compute,
    /// Data transfer pass.
    Transfer,
    /// Ray tracing pass.
    RayTracing,
    /// Presentation pass.
    Present,
    /// Custom pass.
    Custom,
}

impl PassType {
    /// Get the primary pipeline stage for this pass type.
    pub fn primary_stage(&self) -> PipelineStage {
        match self {
            PassType::Graphics => PipelineStage::COLOR_ATTACHMENT_OUTPUT,
            PassType::Compute => PipelineStage::COMPUTE_SHADER,
            PassType::Transfer => PipelineStage::TRANSFER,
            PassType::RayTracing => PipelineStage::RAY_TRACING_SHADER,
            PassType::Present => PipelineStage::BOTTOM_OF_PIPE,
            PassType::Custom => PipelineStage::ALL_COMMANDS,
        }
    }
}

/// Render pass trait for custom pass implementations.
pub trait RenderPass: Send + Sync {
    /// Get the pass name.
    fn name(&self) -> &str;

    /// Get the pass type.
    fn pass_type(&self) -> PassType;

    /// Configure the pass builder.
    fn setup(&mut self, builder: &mut PassBuilder);

    /// Execute the pass.
    fn execute(&self, ctx: &mut PassContext);

    /// Check if the pass should be culled.
    fn should_cull(&self) -> bool {
        false
    }

    /// Get pass priority for ordering.
    fn priority(&self) -> i32 {
        0
    }
}

/// Builder for configuring render passes.
pub struct PassBuilder {
    /// Pass name.
    pub name: String,
    /// Pass type.
    pub pass_type: PassType,
    /// Color attachments.
    pub color_attachments: Vec<AttachmentBinding>,
    /// Depth attachment.
    pub depth_attachment: Option<AttachmentBinding>,
    /// Stencil attachment.
    pub stencil_attachment: Option<AttachmentBinding>,
    /// Texture reads.
    pub texture_reads: Vec<TextureBinding>,
    /// Buffer reads.
    pub buffer_reads: Vec<BufferBinding>,
    /// Storage image bindings.
    pub storage_images: Vec<StorageImageBinding>,
    /// Storage buffer bindings.
    pub storage_buffers: Vec<StorageBufferBinding>,
    /// Render area.
    pub render_area: RenderArea,
    /// Viewport.
    pub viewport: Option<Viewport>,
    /// Scissor.
    pub scissor: Option<Scissor>,
    /// Multi-view configuration.
    pub multi_view: Option<MultiViewConfig>,
    /// Secondary command buffer usage.
    pub use_secondary_commands: bool,
    /// Enable occlusion queries.
    pub occlusion_queries: bool,
    /// Enable pipeline statistics.
    pub pipeline_statistics: bool,
}

impl PassBuilder {
    /// Create a new pass builder.
    pub fn new(name: impl Into<String>, pass_type: PassType) -> Self {
        Self {
            name: name.into(),
            pass_type,
            color_attachments: Vec::new(),
            depth_attachment: None,
            stencil_attachment: None,
            texture_reads: Vec::new(),
            buffer_reads: Vec::new(),
            storage_images: Vec::new(),
            storage_buffers: Vec::new(),
            render_area: RenderArea::default(),
            viewport: None,
            scissor: None,
            multi_view: None,
            use_secondary_commands: false,
            occlusion_queries: false,
            pipeline_statistics: false,
        }
    }

    /// Add a color attachment.
    pub fn color_attachment(
        &mut self,
        texture: VirtualTextureHandle,
        config: AttachmentConfig,
    ) -> &mut Self {
        self.color_attachments.push(AttachmentBinding {
            texture,
            config,
            array_layer: 0,
            mip_level: 0,
        });
        self
    }

    /// Add a color attachment at specific array layer.
    pub fn color_attachment_layer(
        &mut self,
        texture: VirtualTextureHandle,
        layer: u32,
        config: AttachmentConfig,
    ) -> &mut Self {
        self.color_attachments.push(AttachmentBinding {
            texture,
            config,
            array_layer: layer,
            mip_level: 0,
        });
        self
    }

    /// Set depth attachment.
    pub fn depth_attachment(
        &mut self,
        texture: VirtualTextureHandle,
        config: AttachmentConfig,
    ) -> &mut Self {
        self.depth_attachment = Some(AttachmentBinding {
            texture,
            config,
            array_layer: 0,
            mip_level: 0,
        });
        self
    }

    /// Set depth attachment as read-only.
    pub fn depth_read(&mut self, texture: VirtualTextureHandle) -> &mut Self {
        self.depth_attachment = Some(AttachmentBinding {
            texture,
            config: AttachmentConfig::read_only(),
            array_layer: 0,
            mip_level: 0,
        });
        self
    }

    /// Add a texture read.
    pub fn read_texture(&mut self, texture: VirtualTextureHandle, binding: u32) -> &mut Self {
        self.texture_reads.push(TextureBinding {
            texture,
            binding,
            set: 0,
            stages: ShaderStages::FRAGMENT,
            sampler: SamplerBinding::Default,
        });
        self
    }

    /// Add a texture read with specific stages.
    pub fn read_texture_stages(
        &mut self,
        texture: VirtualTextureHandle,
        binding: u32,
        stages: ShaderStages,
    ) -> &mut Self {
        self.texture_reads.push(TextureBinding {
            texture,
            binding,
            set: 0,
            stages,
            sampler: SamplerBinding::Default,
        });
        self
    }

    /// Add a buffer read.
    pub fn read_buffer(&mut self, buffer: VirtualBufferHandle, binding: u32) -> &mut Self {
        self.buffer_reads.push(BufferBinding {
            buffer,
            binding,
            set: 0,
            stages: ShaderStages::ALL,
            offset: 0,
            range: BufferRange::Whole,
        });
        self
    }

    /// Add a storage image binding.
    pub fn storage_image(
        &mut self,
        texture: VirtualTextureHandle,
        binding: u32,
        access: StorageAccess,
    ) -> &mut Self {
        self.storage_images.push(StorageImageBinding {
            texture,
            binding,
            set: 0,
            stages: ShaderStages::COMPUTE,
            access,
            mip_level: 0,
        });
        self
    }

    /// Add a storage buffer binding.
    pub fn storage_buffer(
        &mut self,
        buffer: VirtualBufferHandle,
        binding: u32,
        access: StorageAccess,
    ) -> &mut Self {
        self.storage_buffers.push(StorageBufferBinding {
            buffer,
            binding,
            set: 0,
            stages: ShaderStages::COMPUTE,
            access,
            offset: 0,
            range: BufferRange::Whole,
        });
        self
    }

    /// Set the render area.
    pub fn render_area(&mut self, x: i32, y: i32, width: u32, height: u32) -> &mut Self {
        self.render_area = RenderArea {
            x,
            y,
            width,
            height,
        };
        self
    }

    /// Set the viewport.
    pub fn viewport(&mut self, viewport: Viewport) -> &mut Self {
        self.viewport = Some(viewport);
        self
    }

    /// Set the scissor.
    pub fn scissor(&mut self, scissor: Scissor) -> &mut Self {
        self.scissor = Some(scissor);
        self
    }

    /// Enable multi-view rendering.
    pub fn multi_view(&mut self, config: MultiViewConfig) -> &mut Self {
        self.multi_view = Some(config);
        self
    }

    /// Use secondary command buffers.
    pub fn secondary_commands(&mut self, enabled: bool) -> &mut Self {
        self.use_secondary_commands = enabled;
        self
    }
}

/// Attachment binding for render targets.
#[derive(Debug, Clone)]
pub struct AttachmentBinding {
    /// Texture handle.
    pub texture: VirtualTextureHandle,
    /// Attachment configuration.
    pub config: AttachmentConfig,
    /// Array layer.
    pub array_layer: u32,
    /// Mip level.
    pub mip_level: u32,
}

/// Attachment configuration.
#[derive(Debug, Clone)]
pub struct AttachmentConfig {
    /// Load operation.
    pub load_op: LoadOp,
    /// Store operation.
    pub store_op: StoreOp,
    /// Clear value.
    pub clear_value: ClearValue,
    /// Resolve target.
    pub resolve_target: Option<VirtualTextureHandle>,
    /// Read-only access.
    pub read_only: bool,
}

impl AttachmentConfig {
    /// Clear attachment configuration.
    pub fn clear(value: ClearValue) -> Self {
        Self {
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            clear_value: value,
            resolve_target: None,
            read_only: false,
        }
    }

    /// Load attachment configuration.
    pub fn load() -> Self {
        Self {
            load_op: LoadOp::Load,
            store_op: StoreOp::Store,
            clear_value: ClearValue::default(),
            resolve_target: None,
            read_only: false,
        }
    }

    /// Read-only configuration.
    pub fn read_only() -> Self {
        Self {
            load_op: LoadOp::Load,
            store_op: StoreOp::None,
            clear_value: ClearValue::default(),
            resolve_target: None,
            read_only: true,
        }
    }

    /// Don't care configuration.
    pub fn dont_care() -> Self {
        Self {
            load_op: LoadOp::DontCare,
            store_op: StoreOp::DontCare,
            clear_value: ClearValue::default(),
            resolve_target: None,
            read_only: false,
        }
    }

    /// With MSAA resolve.
    pub fn with_resolve(mut self, target: VirtualTextureHandle) -> Self {
        self.resolve_target = Some(target);
        self
    }
}

impl Default for AttachmentConfig {
    fn default() -> Self {
        Self::load()
    }
}

/// Load operation for attachments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadOp {
    /// Load existing contents.
    Load,
    /// Clear to a value.
    Clear,
    /// Don't care about previous contents.
    DontCare,
}

/// Store operation for attachments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreOp {
    /// Store contents.
    Store,
    /// Don't store (discard).
    DontCare,
    /// No store operation.
    None,
}

/// Clear value for attachments.
#[derive(Debug, Clone, Copy)]
pub enum ClearValue {
    /// Color clear value.
    Color([f32; 4]),
    /// Depth clear value.
    Depth(f32),
    /// Stencil clear value.
    Stencil(u32),
    /// Depth-stencil clear value.
    DepthStencil(f32, u32),
    /// Integer color.
    ColorInt([i32; 4]),
    /// Unsigned integer color.
    ColorUint([u32; 4]),
}

impl Default for ClearValue {
    fn default() -> Self {
        ClearValue::Color([0.0, 0.0, 0.0, 1.0])
    }
}

/// Texture binding for shader access.
#[derive(Debug, Clone)]
pub struct TextureBinding {
    /// Texture handle.
    pub texture: VirtualTextureHandle,
    /// Binding slot.
    pub binding: u32,
    /// Descriptor set.
    pub set: u32,
    /// Shader stages.
    pub stages: ShaderStages,
    /// Sampler binding.
    pub sampler: SamplerBinding,
}

/// Sampler binding options.
#[derive(Debug, Clone)]
pub enum SamplerBinding {
    /// Use default sampler.
    Default,
    /// Use specific sampler.
    Handle(u32),
    /// Use immutable sampler.
    Immutable(SamplerDesc),
}

/// Buffer binding for shader access.
#[derive(Debug, Clone)]
pub struct BufferBinding {
    /// Buffer handle.
    pub buffer: VirtualBufferHandle,
    /// Binding slot.
    pub binding: u32,
    /// Descriptor set.
    pub set: u32,
    /// Shader stages.
    pub stages: ShaderStages,
    /// Offset into buffer.
    pub offset: u64,
    /// Range of buffer.
    pub range: BufferRange,
}

/// Buffer range specification.
#[derive(Debug, Clone, Copy)]
pub enum BufferRange {
    /// Whole buffer.
    Whole,
    /// Specific size.
    Size(u64),
}

/// Storage image binding.
#[derive(Debug, Clone)]
pub struct StorageImageBinding {
    /// Texture handle.
    pub texture: VirtualTextureHandle,
    /// Binding slot.
    pub binding: u32,
    /// Descriptor set.
    pub set: u32,
    /// Shader stages.
    pub stages: ShaderStages,
    /// Access mode.
    pub access: StorageAccess,
    /// Mip level.
    pub mip_level: u32,
}

/// Storage buffer binding.
#[derive(Debug, Clone)]
pub struct StorageBufferBinding {
    /// Buffer handle.
    pub buffer: VirtualBufferHandle,
    /// Binding slot.
    pub binding: u32,
    /// Descriptor set.
    pub set: u32,
    /// Shader stages.
    pub stages: ShaderStages,
    /// Access mode.
    pub access: StorageAccess,
    /// Offset.
    pub offset: u64,
    /// Range.
    pub range: BufferRange,
}

/// Storage access mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageAccess {
    /// Read only.
    Read,
    /// Write only.
    Write,
    /// Read and write.
    ReadWrite,
}

impl StorageAccess {
    /// Get access flags.
    pub fn to_access_flags(self) -> AccessFlags {
        match self {
            StorageAccess::Read => AccessFlags::SHADER_READ,
            StorageAccess::Write => AccessFlags::SHADER_WRITE,
            StorageAccess::ReadWrite => AccessFlags::SHADER_READ | AccessFlags::SHADER_WRITE,
        }
    }
}

/// Shader stages flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShaderStages(u32);

impl ShaderStages {
    /// Vertex shader.
    pub const VERTEX: Self = Self(1 << 0);
    /// Fragment shader.
    pub const FRAGMENT: Self = Self(1 << 1);
    /// Compute shader.
    pub const COMPUTE: Self = Self(1 << 2);
    /// Geometry shader.
    pub const GEOMETRY: Self = Self(1 << 3);
    /// Tessellation control.
    pub const TESSELLATION_CONTROL: Self = Self(1 << 4);
    /// Tessellation evaluation.
    pub const TESSELLATION_EVALUATION: Self = Self(1 << 5);
    /// Mesh shader.
    pub const MESH: Self = Self(1 << 6);
    /// Task shader.
    pub const TASK: Self = Self(1 << 7);
    /// Ray generation.
    pub const RAY_GENERATION: Self = Self(1 << 8);
    /// Any hit.
    pub const ANY_HIT: Self = Self(1 << 9);
    /// Closest hit.
    pub const CLOSEST_HIT: Self = Self(1 << 10);
    /// Miss.
    pub const MISS: Self = Self(1 << 11);
    /// Intersection.
    pub const INTERSECTION: Self = Self(1 << 12);
    /// Callable.
    pub const CALLABLE: Self = Self(1 << 13);
    /// All graphics stages.
    pub const ALL_GRAPHICS: Self = Self(0x3F);
    /// All stages.
    pub const ALL: Self = Self(0x3FFF);

    /// Check if contains a stage.
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Combine stages.
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl core::ops::BitOr for ShaderStages {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// Render area.
#[derive(Debug, Clone, Copy)]
pub struct RenderArea {
    /// X offset.
    pub x: i32,
    /// Y offset.
    pub y: i32,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

impl Default for RenderArea {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
}

impl RenderArea {
    /// Create from dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    /// Create with offset.
    pub fn with_offset(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }
}

/// Viewport specification.
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    /// X position.
    pub x: f32,
    /// Y position.
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

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }
}

impl Viewport {
    /// Create a viewport.
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width,
            height,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }

    /// Create with position.
    pub fn with_position(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }

    /// Set depth range.
    pub fn with_depth(mut self, min: f32, max: f32) -> Self {
        self.min_depth = min;
        self.max_depth = max;
        self
    }
}

/// Scissor specification.
#[derive(Debug, Clone, Copy)]
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
    /// Create a scissor.
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Full scissor (no clipping).
    pub fn full(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }
}

/// Multi-view configuration for VR/stereo rendering.
#[derive(Debug, Clone)]
pub struct MultiViewConfig {
    /// View mask.
    pub view_mask: u32,
    /// Correlation mask.
    pub correlation_mask: u32,
    /// Number of views.
    pub view_count: u32,
}

impl MultiViewConfig {
    /// Stereo configuration (2 views).
    pub fn stereo() -> Self {
        Self {
            view_mask: 0b11,
            correlation_mask: 0b11,
            view_count: 2,
        }
    }

    /// Custom view count.
    pub fn views(count: u32) -> Self {
        Self {
            view_mask: (1 << count) - 1,
            correlation_mask: (1 << count) - 1,
            view_count: count,
        }
    }
}

/// Sampler description.
#[derive(Debug, Clone)]
pub struct SamplerDesc {
    /// Minification filter.
    pub min_filter: Filter,
    /// Magnification filter.
    pub mag_filter: Filter,
    /// Mipmap mode.
    pub mipmap_mode: MipmapMode,
    /// Address mode U.
    pub address_u: AddressMode,
    /// Address mode V.
    pub address_v: AddressMode,
    /// Address mode W.
    pub address_w: AddressMode,
    /// Mip LOD bias.
    pub mip_lod_bias: f32,
    /// Enable anisotropy.
    pub anisotropy_enable: bool,
    /// Max anisotropy.
    pub max_anisotropy: f32,
    /// Compare operation.
    pub compare_op: Option<CompareOp>,
    /// Min LOD.
    pub min_lod: f32,
    /// Max LOD.
    pub max_lod: f32,
    /// Border color.
    pub border_color: BorderColor,
    /// Use unnormalized coordinates.
    pub unnormalized_coordinates: bool,
}

impl Default for SamplerDesc {
    fn default() -> Self {
        Self {
            min_filter: Filter::Linear,
            mag_filter: Filter::Linear,
            mipmap_mode: MipmapMode::Linear,
            address_u: AddressMode::Repeat,
            address_v: AddressMode::Repeat,
            address_w: AddressMode::Repeat,
            mip_lod_bias: 0.0,
            anisotropy_enable: false,
            max_anisotropy: 1.0,
            compare_op: None,
            min_lod: 0.0,
            max_lod: 1000.0,
            border_color: BorderColor::FloatOpaqueBlack,
            unnormalized_coordinates: false,
        }
    }
}

impl SamplerDesc {
    /// Nearest neighbor sampling.
    pub fn nearest() -> Self {
        Self {
            min_filter: Filter::Nearest,
            mag_filter: Filter::Nearest,
            mipmap_mode: MipmapMode::Nearest,
            ..Default::default()
        }
    }

    /// Linear sampling.
    pub fn linear() -> Self {
        Self::default()
    }

    /// Anisotropic sampling.
    pub fn anisotropic(max: f32) -> Self {
        Self {
            anisotropy_enable: true,
            max_anisotropy: max,
            ..Default::default()
        }
    }

    /// Shadow/depth comparison sampler.
    pub fn shadow() -> Self {
        Self {
            min_filter: Filter::Linear,
            mag_filter: Filter::Linear,
            compare_op: Some(CompareOp::LessOrEqual),
            ..Default::default()
        }
    }
}

/// Texture filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    /// Nearest neighbor.
    Nearest,
    /// Linear interpolation.
    Linear,
}

/// Mipmap mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MipmapMode {
    /// Nearest mip level.
    Nearest,
    /// Linear interpolation between mip levels.
    Linear,
}

/// Texture address mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressMode {
    /// Repeat texture.
    Repeat,
    /// Mirror repeat.
    MirroredRepeat,
    /// Clamp to edge.
    ClampToEdge,
    /// Clamp to border.
    ClampToBorder,
    /// Mirror clamp to edge.
    MirrorClampToEdge,
}

/// Comparison operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    /// Never pass.
    Never,
    /// Pass if less.
    Less,
    /// Pass if equal.
    Equal,
    /// Pass if less or equal.
    LessOrEqual,
    /// Pass if greater.
    Greater,
    /// Pass if not equal.
    NotEqual,
    /// Pass if greater or equal.
    GreaterOrEqual,
    /// Always pass.
    Always,
}

/// Border color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderColor {
    /// Transparent black (float).
    FloatTransparentBlack,
    /// Transparent black (int).
    IntTransparentBlack,
    /// Opaque black (float).
    FloatOpaqueBlack,
    /// Opaque black (int).
    IntOpaqueBlack,
    /// Opaque white (float).
    FloatOpaqueWhite,
    /// Opaque white (int).
    IntOpaqueWhite,
}

/// Pass execution context.
pub struct PassContext {
    /// Current pass name.
    pub pass_name: String,
    /// Current view.
    pub view: Option<View>,
    /// Render target.
    pub render_target: Option<RenderTarget>,
    /// Command recorder.
    commands: CommandRecorder,
    /// Frame index.
    pub frame_index: u32,
    /// Delta time.
    pub delta_time: f32,
    /// Total time.
    pub total_time: f32,
}

impl PassContext {
    /// Create a new pass context.
    pub fn new() -> Self {
        Self {
            pass_name: String::new(),
            view: None,
            render_target: None,
            commands: CommandRecorder::new(),
            frame_index: 0,
            delta_time: 0.0,
            total_time: 0.0,
        }
    }

    /// Set the current pass.
    pub fn set_pass(&mut self, name: impl Into<String>) {
        self.pass_name = name.into();
    }

    /// Get command recorder.
    pub fn commands(&mut self) -> &mut CommandRecorder {
        &mut self.commands
    }

    /// Set viewport.
    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.commands.set_viewport(viewport);
    }

    /// Set scissor.
    pub fn set_scissor(&mut self, scissor: Scissor) {
        self.commands.set_scissor(scissor);
    }

    /// Bind pipeline.
    pub fn bind_pipeline(&mut self, pipeline: PipelineHandle) {
        self.commands.bind_pipeline(pipeline);
    }

    /// Draw call.
    pub fn draw(&mut self, vertex_count: u32, instance_count: u32) {
        self.commands.draw(vertex_count, instance_count, 0, 0);
    }

    /// Indexed draw call.
    pub fn draw_indexed(&mut self, index_count: u32, instance_count: u32) {
        self.commands
            .draw_indexed(index_count, instance_count, 0, 0, 0);
    }

    /// Dispatch compute.
    pub fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        self.commands.dispatch(x, y, z);
    }

    /// Indirect draw.
    pub fn draw_indirect(&mut self, buffer: BufferHandle, offset: u64, count: u32) {
        self.commands.draw_indirect(buffer, offset, count);
    }

    /// Trace rays.
    pub fn trace_rays(&mut self, width: u32, height: u32, depth: u32) {
        self.commands.trace_rays(width, height, depth);
    }
}

impl Default for PassContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Command recorder for GPU commands.
pub struct CommandRecorder {
    /// Recorded commands.
    commands: Vec<RecordedCommand>,
}

impl CommandRecorder {
    /// Create new recorder.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Set viewport.
    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.commands.push(RecordedCommand::SetViewport(viewport));
    }

    /// Set scissor.
    pub fn set_scissor(&mut self, scissor: Scissor) {
        self.commands.push(RecordedCommand::SetScissor(scissor));
    }

    /// Bind pipeline.
    pub fn bind_pipeline(&mut self, pipeline: PipelineHandle) {
        self.commands.push(RecordedCommand::BindPipeline(pipeline));
    }

    /// Draw call.
    pub fn draw(
        &mut self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        self.commands.push(RecordedCommand::Draw {
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
        });
    }

    /// Indexed draw.
    pub fn draw_indexed(
        &mut self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        self.commands.push(RecordedCommand::DrawIndexed {
            index_count,
            instance_count,
            first_index,
            vertex_offset,
            first_instance,
        });
    }

    /// Dispatch compute.
    pub fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        self.commands
            .push(RecordedCommand::Dispatch { x, y, z });
    }

    /// Indirect draw.
    pub fn draw_indirect(&mut self, buffer: BufferHandle, offset: u64, count: u32) {
        self.commands.push(RecordedCommand::DrawIndirect {
            buffer,
            offset,
            count,
        });
    }

    /// Trace rays.
    pub fn trace_rays(&mut self, width: u32, height: u32, depth: u32) {
        self.commands.push(RecordedCommand::TraceRays {
            width,
            height,
            depth,
        });
    }

    /// Get recorded commands.
    pub fn commands(&self) -> &[RecordedCommand] {
        &self.commands
    }

    /// Clear recorded commands.
    pub fn clear(&mut self) {
        self.commands.clear();
    }
}

impl Default for CommandRecorder {
    fn default() -> Self {
        Self::new()
    }
}

/// Recorded GPU command.
#[derive(Debug, Clone)]
pub enum RecordedCommand {
    /// Set viewport.
    SetViewport(Viewport),
    /// Set scissor.
    SetScissor(Scissor),
    /// Bind pipeline.
    BindPipeline(PipelineHandle),
    /// Draw call.
    Draw {
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    },
    /// Indexed draw.
    DrawIndexed {
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    },
    /// Dispatch compute.
    Dispatch { x: u32, y: u32, z: u32 },
    /// Indirect draw.
    DrawIndirect {
        buffer: BufferHandle,
        offset: u64,
        count: u32,
    },
    /// Trace rays.
    TraceRays {
        width: u32,
        height: u32,
        depth: u32,
    },
}

/// Pipeline handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineHandle(pub u32);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_stages() {
        let stages = ShaderStages::VERTEX | ShaderStages::FRAGMENT;
        assert!(stages.contains(ShaderStages::VERTEX));
        assert!(stages.contains(ShaderStages::FRAGMENT));
        assert!(!stages.contains(ShaderStages::COMPUTE));
    }

    #[test]
    fn test_sampler_desc() {
        let shadow = SamplerDesc::shadow();
        assert!(shadow.compare_op.is_some());

        let aniso = SamplerDesc::anisotropic(16.0);
        assert!(aniso.anisotropy_enable);
    }
}
