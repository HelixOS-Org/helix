//! Pipeline State Objects
//!
//! Render, compute, and ray tracing pipeline management.

use alloc::{string::String, vec::Vec};
use core::sync::atomic::{AtomicU32, Ordering};

use bitflags::bitflags;
use lumina_core::Handle;

use crate::device::TextureFormat;
use crate::shader_module::{ShaderModuleHandle, ShaderStage};
use crate::sampler::CompareOp;
use crate::descriptor::PipelineLayoutHandle;

// ============================================================================
// Primitive Topology
// ============================================================================

/// Primitive topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveTopology {
    /// Point list.
    PointList,
    /// Line list.
    LineList,
    /// Line strip.
    LineStrip,
    /// Triangle list.
    TriangleList,
    /// Triangle strip.
    TriangleStrip,
    /// Triangle fan.
    TriangleFan,
    /// Line list with adjacency.
    LineListWithAdjacency,
    /// Line strip with adjacency.
    LineStripWithAdjacency,
    /// Triangle list with adjacency.
    TriangleListWithAdjacency,
    /// Triangle strip with adjacency.
    TriangleStripWithAdjacency,
    /// Patch list (tessellation).
    PatchList,
}

impl Default for PrimitiveTopology {
    fn default() -> Self {
        PrimitiveTopology::TriangleList
    }
}

// ============================================================================
// Front Face
// ============================================================================

/// Front face winding order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrontFace {
    /// Counter-clockwise.
    Ccw,
    /// Clockwise.
    Cw,
}

impl Default for FrontFace {
    fn default() -> Self {
        FrontFace::Ccw
    }
}

// ============================================================================
// Cull Mode
// ============================================================================

bitflags! {
    /// Face culling mode.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CullMode: u32 {
        /// No culling.
        const NONE = 0;
        /// Cull front faces.
        const FRONT = 1 << 0;
        /// Cull back faces.
        const BACK = 1 << 1;
        /// Cull all faces.
        const FRONT_AND_BACK = Self::FRONT.bits() | Self::BACK.bits();
    }
}

impl Default for CullMode {
    fn default() -> Self {
        CullMode::BACK
    }
}

// ============================================================================
// Polygon Mode
// ============================================================================

/// Polygon rasterization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PolygonMode {
    /// Fill polygons.
    Fill,
    /// Draw lines (wireframe).
    Line,
    /// Draw points.
    Point,
}

impl Default for PolygonMode {
    fn default() -> Self {
        PolygonMode::Fill
    }
}

// ============================================================================
// Blend Factor
// ============================================================================

/// Blend factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendFactor {
    /// 0.
    Zero,
    /// 1.
    One,
    /// Source color.
    SrcColor,
    /// 1 - source color.
    OneMinusSrcColor,
    /// Destination color.
    DstColor,
    /// 1 - destination color.
    OneMinusDstColor,
    /// Source alpha.
    SrcAlpha,
    /// 1 - source alpha.
    OneMinusSrcAlpha,
    /// Destination alpha.
    DstAlpha,
    /// 1 - destination alpha.
    OneMinusDstAlpha,
    /// Constant color.
    ConstantColor,
    /// 1 - constant color.
    OneMinusConstantColor,
    /// Constant alpha.
    ConstantAlpha,
    /// 1 - constant alpha.
    OneMinusConstantAlpha,
    /// Saturate source alpha.
    SrcAlphaSaturate,
    /// Source 1 color (dual-source blending).
    Src1Color,
    /// 1 - source 1 color.
    OneMinusSrc1Color,
    /// Source 1 alpha.
    Src1Alpha,
    /// 1 - source 1 alpha.
    OneMinusSrc1Alpha,
}

impl Default for BlendFactor {
    fn default() -> Self {
        BlendFactor::One
    }
}

// ============================================================================
// Blend Operation
// ============================================================================

/// Blend operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendOp {
    /// Add.
    Add,
    /// Subtract.
    Subtract,
    /// Reverse subtract.
    ReverseSubtract,
    /// Minimum.
    Min,
    /// Maximum.
    Max,
}

impl Default for BlendOp {
    fn default() -> Self {
        BlendOp::Add
    }
}

// ============================================================================
// Color Write Mask
// ============================================================================

bitflags! {
    /// Color write mask.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ColorWriteMask: u32 {
        /// Write red.
        const RED = 1 << 0;
        /// Write green.
        const GREEN = 1 << 1;
        /// Write blue.
        const BLUE = 1 << 2;
        /// Write alpha.
        const ALPHA = 1 << 3;
        /// Write all.
        const ALL = Self::RED.bits() | Self::GREEN.bits() | Self::BLUE.bits() | Self::ALPHA.bits();
    }
}

impl Default for ColorWriteMask {
    fn default() -> Self {
        ColorWriteMask::ALL
    }
}

// ============================================================================
// Blend State
// ============================================================================

/// Blend state for a color attachment.
#[derive(Debug, Clone, Copy)]
pub struct BlendState {
    /// Enable blending.
    pub enabled: bool,
    /// Source color factor.
    pub src_color: BlendFactor,
    /// Destination color factor.
    pub dst_color: BlendFactor,
    /// Color blend operation.
    pub color_op: BlendOp,
    /// Source alpha factor.
    pub src_alpha: BlendFactor,
    /// Destination alpha factor.
    pub dst_alpha: BlendFactor,
    /// Alpha blend operation.
    pub alpha_op: BlendOp,
    /// Color write mask.
    pub write_mask: ColorWriteMask,
}

impl Default for BlendState {
    fn default() -> Self {
        Self {
            enabled: false,
            src_color: BlendFactor::One,
            dst_color: BlendFactor::Zero,
            color_op: BlendOp::Add,
            src_alpha: BlendFactor::One,
            dst_alpha: BlendFactor::Zero,
            alpha_op: BlendOp::Add,
            write_mask: ColorWriteMask::ALL,
        }
    }
}

impl BlendState {
    /// Alpha blending preset.
    pub fn alpha_blending() -> Self {
        Self {
            enabled: true,
            src_color: BlendFactor::SrcAlpha,
            dst_color: BlendFactor::OneMinusSrcAlpha,
            color_op: BlendOp::Add,
            src_alpha: BlendFactor::One,
            dst_alpha: BlendFactor::OneMinusSrcAlpha,
            alpha_op: BlendOp::Add,
            write_mask: ColorWriteMask::ALL,
        }
    }

    /// Additive blending preset.
    pub fn additive() -> Self {
        Self {
            enabled: true,
            src_color: BlendFactor::One,
            dst_color: BlendFactor::One,
            color_op: BlendOp::Add,
            src_alpha: BlendFactor::One,
            dst_alpha: BlendFactor::One,
            alpha_op: BlendOp::Add,
            write_mask: ColorWriteMask::ALL,
        }
    }

    /// Premultiplied alpha preset.
    pub fn premultiplied_alpha() -> Self {
        Self {
            enabled: true,
            src_color: BlendFactor::One,
            dst_color: BlendFactor::OneMinusSrcAlpha,
            color_op: BlendOp::Add,
            src_alpha: BlendFactor::One,
            dst_alpha: BlendFactor::OneMinusSrcAlpha,
            alpha_op: BlendOp::Add,
            write_mask: ColorWriteMask::ALL,
        }
    }
}

// ============================================================================
// Stencil Operation
// ============================================================================

/// Stencil operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StencilOp {
    /// Keep current value.
    Keep,
    /// Set to zero.
    Zero,
    /// Replace with reference.
    Replace,
    /// Increment and clamp.
    IncrementClamp,
    /// Decrement and clamp.
    DecrementClamp,
    /// Bitwise invert.
    Invert,
    /// Increment and wrap.
    IncrementWrap,
    /// Decrement and wrap.
    DecrementWrap,
}

impl Default for StencilOp {
    fn default() -> Self {
        StencilOp::Keep
    }
}

// ============================================================================
// Stencil Face State
// ============================================================================

/// Stencil state for one face.
#[derive(Debug, Clone, Copy)]
pub struct StencilFaceState {
    /// Compare operation.
    pub compare: CompareOp,
    /// Stencil fail operation.
    pub fail_op: StencilOp,
    /// Depth fail operation.
    pub depth_fail_op: StencilOp,
    /// Pass operation.
    pub pass_op: StencilOp,
}

impl Default for StencilFaceState {
    fn default() -> Self {
        Self {
            compare: CompareOp::Always,
            fail_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
            pass_op: StencilOp::Keep,
        }
    }
}

// ============================================================================
// Depth Stencil State
// ============================================================================

/// Depth/stencil state.
#[derive(Debug, Clone, Copy)]
pub struct DepthStencilState {
    /// Enable depth testing.
    pub depth_test_enable: bool,
    /// Enable depth writing.
    pub depth_write_enable: bool,
    /// Depth compare operation.
    pub depth_compare: CompareOp,
    /// Enable stencil testing.
    pub stencil_test_enable: bool,
    /// Stencil read mask.
    pub stencil_read_mask: u32,
    /// Stencil write mask.
    pub stencil_write_mask: u32,
    /// Front face stencil state.
    pub front: StencilFaceState,
    /// Back face stencil state.
    pub back: StencilFaceState,
    /// Enable depth bounds testing.
    pub depth_bounds_test_enable: bool,
    /// Minimum depth bound.
    pub min_depth_bounds: f32,
    /// Maximum depth bound.
    pub max_depth_bounds: f32,
}

impl Default for DepthStencilState {
    fn default() -> Self {
        Self {
            depth_test_enable: true,
            depth_write_enable: true,
            depth_compare: CompareOp::Less,
            stencil_test_enable: false,
            stencil_read_mask: 0xFF,
            stencil_write_mask: 0xFF,
            front: StencilFaceState::default(),
            back: StencilFaceState::default(),
            depth_bounds_test_enable: false,
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
        }
    }
}

impl DepthStencilState {
    /// Disabled depth/stencil.
    pub fn disabled() -> Self {
        Self {
            depth_test_enable: false,
            depth_write_enable: false,
            ..Default::default()
        }
    }

    /// Read-only depth.
    pub fn read_only() -> Self {
        Self {
            depth_test_enable: true,
            depth_write_enable: false,
            ..Default::default()
        }
    }
}

// ============================================================================
// Multisample State
// ============================================================================

/// Multisample state.
#[derive(Debug, Clone, Copy)]
pub struct MultisampleState {
    /// Sample count.
    pub count: u32,
    /// Sample mask.
    pub mask: u32,
    /// Enable alpha to coverage.
    pub alpha_to_coverage_enable: bool,
    /// Enable alpha to one.
    pub alpha_to_one_enable: bool,
}

impl Default for MultisampleState {
    fn default() -> Self {
        Self {
            count: 1,
            mask: !0,
            alpha_to_coverage_enable: false,
            alpha_to_one_enable: false,
        }
    }
}

// ============================================================================
// Vertex Attribute
// ============================================================================

/// Vertex attribute format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VertexFormat {
    /// 2x 8-bit unsigned.
    Uint8x2,
    /// 4x 8-bit unsigned.
    Uint8x4,
    /// 2x 8-bit signed.
    Sint8x2,
    /// 4x 8-bit signed.
    Sint8x4,
    /// 2x 8-bit unsigned normalized.
    Unorm8x2,
    /// 4x 8-bit unsigned normalized.
    Unorm8x4,
    /// 2x 8-bit signed normalized.
    Snorm8x2,
    /// 4x 8-bit signed normalized.
    Snorm8x4,
    /// 2x 16-bit unsigned.
    Uint16x2,
    /// 4x 16-bit unsigned.
    Uint16x4,
    /// 2x 16-bit signed.
    Sint16x2,
    /// 4x 16-bit signed.
    Sint16x4,
    /// 2x 16-bit unsigned normalized.
    Unorm16x2,
    /// 4x 16-bit unsigned normalized.
    Unorm16x4,
    /// 2x 16-bit signed normalized.
    Snorm16x2,
    /// 4x 16-bit signed normalized.
    Snorm16x4,
    /// 2x 16-bit float.
    Float16x2,
    /// 4x 16-bit float.
    Float16x4,
    /// 1x 32-bit float.
    Float32,
    /// 2x 32-bit float.
    Float32x2,
    /// 3x 32-bit float.
    Float32x3,
    /// 4x 32-bit float.
    Float32x4,
    /// 1x 32-bit unsigned.
    Uint32,
    /// 2x 32-bit unsigned.
    Uint32x2,
    /// 3x 32-bit unsigned.
    Uint32x3,
    /// 4x 32-bit unsigned.
    Uint32x4,
    /// 1x 32-bit signed.
    Sint32,
    /// 2x 32-bit signed.
    Sint32x2,
    /// 3x 32-bit signed.
    Sint32x3,
    /// 4x 32-bit signed.
    Sint32x4,
}

impl VertexFormat {
    /// Get size in bytes.
    pub fn size(&self) -> u32 {
        match self {
            Self::Uint8x2 | Self::Sint8x2 | Self::Unorm8x2 | Self::Snorm8x2 => 2,
            Self::Uint8x4 | Self::Sint8x4 | Self::Unorm8x4 | Self::Snorm8x4 => 4,
            Self::Uint16x2 | Self::Sint16x2 | Self::Unorm16x2 | Self::Snorm16x2 | Self::Float16x2 => 4,
            Self::Uint16x4 | Self::Sint16x4 | Self::Unorm16x4 | Self::Snorm16x4 | Self::Float16x4 => 8,
            Self::Float32 | Self::Uint32 | Self::Sint32 => 4,
            Self::Float32x2 | Self::Uint32x2 | Self::Sint32x2 => 8,
            Self::Float32x3 | Self::Uint32x3 | Self::Sint32x3 => 12,
            Self::Float32x4 | Self::Uint32x4 | Self::Sint32x4 => 16,
        }
    }
}

/// Vertex input rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VertexStepMode {
    /// Per-vertex.
    Vertex,
    /// Per-instance.
    Instance,
}

impl Default for VertexStepMode {
    fn default() -> Self {
        VertexStepMode::Vertex
    }
}

/// Vertex buffer layout.
#[derive(Debug, Clone)]
pub struct VertexBufferLayout {
    /// Stride in bytes.
    pub stride: u32,
    /// Step mode.
    pub step_mode: VertexStepMode,
    /// Attributes.
    pub attributes: Vec<VertexAttribute>,
}

/// Vertex attribute.
#[derive(Debug, Clone, Copy)]
pub struct VertexAttribute {
    /// Format.
    pub format: VertexFormat,
    /// Offset in bytes.
    pub offset: u32,
    /// Shader location.
    pub shader_location: u32,
}

// ============================================================================
// Render Pipeline Description
// ============================================================================

/// Description for render pipeline creation.
#[derive(Debug, Clone)]
pub struct RenderPipelineDesc {
    /// Pipeline layout.
    pub layout: Option<PipelineLayoutHandle>,
    /// Vertex shader.
    pub vertex: ShaderModuleHandle,
    /// Fragment shader.
    pub fragment: Option<ShaderModuleHandle>,
    /// Vertex buffers.
    pub vertex_buffers: Vec<VertexBufferLayout>,
    /// Primitive topology.
    pub primitive_topology: PrimitiveTopology,
    /// Primitive restart.
    pub primitive_restart: bool,
    /// Front face.
    pub front_face: FrontFace,
    /// Cull mode.
    pub cull_mode: CullMode,
    /// Polygon mode.
    pub polygon_mode: PolygonMode,
    /// Depth bias.
    pub depth_bias: f32,
    /// Depth bias slope scale.
    pub depth_bias_slope_scale: f32,
    /// Depth bias clamp.
    pub depth_bias_clamp: f32,
    /// Depth/stencil state.
    pub depth_stencil: Option<DepthStencilState>,
    /// Multisample state.
    pub multisample: MultisampleState,
    /// Color attachment formats.
    pub color_formats: Vec<TextureFormat>,
    /// Color blend states.
    pub color_blend_states: Vec<BlendState>,
    /// Depth format.
    pub depth_format: Option<TextureFormat>,
    /// Stencil format.
    pub stencil_format: Option<TextureFormat>,
    /// Debug label.
    pub label: Option<String>,
}

impl Default for RenderPipelineDesc {
    fn default() -> Self {
        Self {
            layout: None,
            vertex: ShaderModuleHandle::new(0, 0),
            fragment: None,
            vertex_buffers: Vec::new(),
            primitive_topology: PrimitiveTopology::TriangleList,
            primitive_restart: false,
            front_face: FrontFace::Ccw,
            cull_mode: CullMode::BACK,
            polygon_mode: PolygonMode::Fill,
            depth_bias: 0.0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
            depth_stencil: Some(DepthStencilState::default()),
            multisample: MultisampleState::default(),
            color_formats: Vec::new(),
            color_blend_states: Vec::new(),
            depth_format: None,
            stencil_format: None,
            label: None,
        }
    }
}

// ============================================================================
// Render Pipeline Handle
// ============================================================================

/// Handle to a render pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderPipelineHandle(Handle<RenderPipeline>);

impl RenderPipelineHandle {
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
// Render Pipeline
// ============================================================================

/// A render pipeline state object.
pub struct RenderPipeline {
    /// Handle.
    pub handle: RenderPipelineHandle,
    /// Layout.
    pub layout: Option<PipelineLayoutHandle>,
    /// Debug label.
    pub label: Option<String>,
}

// ============================================================================
// Compute Pipeline Description
// ============================================================================

/// Description for compute pipeline creation.
#[derive(Debug, Clone)]
pub struct ComputePipelineDesc {
    /// Pipeline layout.
    pub layout: Option<PipelineLayoutHandle>,
    /// Compute shader.
    pub compute: ShaderModuleHandle,
    /// Debug label.
    pub label: Option<String>,
}

// ============================================================================
// Compute Pipeline Handle
// ============================================================================

/// Handle to a compute pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComputePipelineHandle(Handle<ComputePipeline>);

impl ComputePipelineHandle {
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
// Compute Pipeline
// ============================================================================

/// A compute pipeline state object.
pub struct ComputePipeline {
    /// Handle.
    pub handle: ComputePipelineHandle,
    /// Layout.
    pub layout: Option<PipelineLayoutHandle>,
    /// Debug label.
    pub label: Option<String>,
}

// ============================================================================
// Ray Tracing Pipeline Description
// ============================================================================

/// Shader group type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderGroupType {
    /// General shader group.
    General,
    /// Triangle hit group.
    TrianglesHitGroup,
    /// Procedural hit group.
    ProceduralHitGroup,
}

/// Shader group.
#[derive(Debug, Clone)]
pub struct ShaderGroup {
    /// Group type.
    pub group_type: ShaderGroupType,
    /// General shader index.
    pub general_shader: Option<u32>,
    /// Closest hit shader index.
    pub closest_hit_shader: Option<u32>,
    /// Any hit shader index.
    pub any_hit_shader: Option<u32>,
    /// Intersection shader index.
    pub intersection_shader: Option<u32>,
}

/// Description for ray tracing pipeline creation.
#[derive(Debug, Clone)]
pub struct RayTracingPipelineDesc {
    /// Pipeline layout.
    pub layout: Option<PipelineLayoutHandle>,
    /// Shader stages.
    pub stages: Vec<ShaderModuleHandle>,
    /// Shader groups.
    pub groups: Vec<ShaderGroup>,
    /// Maximum ray recursion depth.
    pub max_ray_recursion_depth: u32,
    /// Maximum ray payload size.
    pub max_ray_payload_size: u32,
    /// Maximum ray hit attribute size.
    pub max_ray_hit_attribute_size: u32,
    /// Debug label.
    pub label: Option<String>,
}

// ============================================================================
// Ray Tracing Pipeline Handle
// ============================================================================

/// Handle to a ray tracing pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RayTracingPipelineHandle(Handle<RayTracingPipeline>);

impl RayTracingPipelineHandle {
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
// Ray Tracing Pipeline
// ============================================================================

/// A ray tracing pipeline state object.
pub struct RayTracingPipeline {
    /// Handle.
    pub handle: RayTracingPipelineHandle,
    /// Layout.
    pub layout: Option<PipelineLayoutHandle>,
    /// Shader group count.
    pub group_count: u32,
    /// Debug label.
    pub label: Option<String>,
}

// ============================================================================
// Pipeline Manager
// ============================================================================

/// Manages pipeline state objects.
pub struct PipelineManager {
    /// Render pipelines.
    render_pipelines: Vec<Option<RenderPipeline>>,
    /// Compute pipelines.
    compute_pipelines: Vec<Option<ComputePipeline>>,
    /// Ray tracing pipelines.
    ray_tracing_pipelines: Vec<Option<RayTracingPipeline>>,
    /// Free indices.
    free_render: Vec<u32>,
    free_compute: Vec<u32>,
    free_ray_tracing: Vec<u32>,
    /// Generations.
    gen_render: Vec<u32>,
    gen_compute: Vec<u32>,
    gen_ray_tracing: Vec<u32>,
    /// Counts.
    render_count: AtomicU32,
    compute_count: AtomicU32,
    ray_tracing_count: AtomicU32,
}

impl PipelineManager {
    /// Create a new pipeline manager.
    pub fn new() -> Self {
        Self {
            render_pipelines: Vec::new(),
            compute_pipelines: Vec::new(),
            ray_tracing_pipelines: Vec::new(),
            free_render: Vec::new(),
            free_compute: Vec::new(),
            free_ray_tracing: Vec::new(),
            gen_render: Vec::new(),
            gen_compute: Vec::new(),
            gen_ray_tracing: Vec::new(),
            render_count: AtomicU32::new(0),
            compute_count: AtomicU32::new(0),
            ray_tracing_count: AtomicU32::new(0),
        }
    }

    /// Create render pipeline.
    pub fn create_render(&mut self, desc: &RenderPipelineDesc) -> RenderPipelineHandle {
        let index = if let Some(index) = self.free_render.pop() {
            index
        } else {
            let index = self.render_pipelines.len() as u32;
            self.render_pipelines.push(None);
            self.gen_render.push(0);
            index
        };

        let generation = self.gen_render[index as usize];
        let handle = RenderPipelineHandle::new(index, generation);
        let pipeline = RenderPipeline {
            handle,
            layout: desc.layout,
            label: desc.label.clone(),
        };

        self.render_pipelines[index as usize] = Some(pipeline);
        self.render_count.fetch_add(1, Ordering::Relaxed);

        handle
    }

    /// Create compute pipeline.
    pub fn create_compute(&mut self, desc: &ComputePipelineDesc) -> ComputePipelineHandle {
        let index = if let Some(index) = self.free_compute.pop() {
            index
        } else {
            let index = self.compute_pipelines.len() as u32;
            self.compute_pipelines.push(None);
            self.gen_compute.push(0);
            index
        };

        let generation = self.gen_compute[index as usize];
        let handle = ComputePipelineHandle::new(index, generation);
        let pipeline = ComputePipeline {
            handle,
            layout: desc.layout,
            label: desc.label.clone(),
        };

        self.compute_pipelines[index as usize] = Some(pipeline);
        self.compute_count.fetch_add(1, Ordering::Relaxed);

        handle
    }

    /// Get render pipeline.
    pub fn get_render(&self, handle: RenderPipelineHandle) -> Option<&RenderPipeline> {
        let index = handle.index() as usize;
        self.render_pipelines.get(index)?.as_ref()
    }

    /// Get compute pipeline.
    pub fn get_compute(&self, handle: ComputePipelineHandle) -> Option<&ComputePipeline> {
        let index = handle.index() as usize;
        self.compute_pipelines.get(index)?.as_ref()
    }

    /// Destroy render pipeline.
    pub fn destroy_render(&mut self, handle: RenderPipelineHandle) {
        let index = handle.index() as usize;
        if index < self.render_pipelines.len() {
            if self.render_pipelines[index].take().is_some() {
                self.render_count.fetch_sub(1, Ordering::Relaxed);
            }
            self.gen_render[index] = self.gen_render[index].wrapping_add(1);
            self.free_render.push(index as u32);
        }
    }

    /// Destroy compute pipeline.
    pub fn destroy_compute(&mut self, handle: ComputePipelineHandle) {
        let index = handle.index() as usize;
        if index < self.compute_pipelines.len() {
            if self.compute_pipelines[index].take().is_some() {
                self.compute_count.fetch_sub(1, Ordering::Relaxed);
            }
            self.gen_compute[index] = self.gen_compute[index].wrapping_add(1);
            self.free_compute.push(index as u32);
        }
    }

    /// Get counts.
    pub fn render_count(&self) -> u32 {
        self.render_count.load(Ordering::Relaxed)
    }

    pub fn compute_count(&self) -> u32 {
        self.compute_count.load(Ordering::Relaxed)
    }
}

impl Default for PipelineManager {
    fn default() -> Self {
        Self::new()
    }
}
