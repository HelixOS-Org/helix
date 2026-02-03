//! Mesh Shader Pipeline for LUMINA
//!
//! Mesh shaders are a modern GPU pipeline that replaces the traditional
//! vertex/geometry shader stages with a more flexible compute-like model:
//!
//! ```text
//! Traditional Pipeline:
//!   Input Assembly → Vertex Shader → [Tessellation] → [Geometry] → Rasterizer
//!
//! Mesh Shader Pipeline:
//!   Task Shader (optional) → Mesh Shader → Rasterizer
//! ```
//!
//! ## Benefits
//!
//! - **Flexible Geometry**: Generate geometry on-the-fly
//! - **Better Culling**: Per-meshlet culling in shader
//! - **Reduced CPU Overhead**: No input assembly stage
//! - **Meshlet-Based**: Natural fit for modern LOD systems

use core::ops::Range;

#[cfg(feature = "alloc")]
use alloc::{vec::Vec, boxed::Box};

use crate::error::{Error, Result};
use crate::pipeline::{PipelineLayout, ShaderStageFlags};
use crate::types::Format;

// ============================================================================
// Mesh Shader Limits
// ============================================================================

/// Hardware limits for mesh shaders
#[derive(Clone, Copy, Debug)]
pub struct MeshShaderLimits {
    /// Maximum task work group count X
    pub max_task_work_group_count_x: u32,
    /// Maximum task work group count Y
    pub max_task_work_group_count_y: u32,
    /// Maximum task work group count Z
    pub max_task_work_group_count_z: u32,
    /// Maximum task work group invocations
    pub max_task_work_group_invocations: u32,
    /// Maximum task work group size X
    pub max_task_work_group_size_x: u32,
    /// Maximum task work group size Y
    pub max_task_work_group_size_y: u32,
    /// Maximum task work group size Z
    pub max_task_work_group_size_z: u32,
    /// Maximum task payload size in bytes
    pub max_task_payload_size: u32,
    /// Maximum task shared memory size in bytes
    pub max_task_shared_memory_size: u32,
    /// Maximum task payload and shared memory size combined
    pub max_task_payload_and_shared_memory_size: u32,
    /// Maximum mesh work group count X
    pub max_mesh_work_group_count_x: u32,
    /// Maximum mesh work group count Y
    pub max_mesh_work_group_count_y: u32,
    /// Maximum mesh work group count Z
    pub max_mesh_work_group_count_z: u32,
    /// Maximum mesh work group invocations
    pub max_mesh_work_group_invocations: u32,
    /// Maximum mesh work group size X
    pub max_mesh_work_group_size_x: u32,
    /// Maximum mesh work group size Y
    pub max_mesh_work_group_size_y: u32,
    /// Maximum mesh work group size Z
    pub max_mesh_work_group_size_z: u32,
    /// Maximum mesh shared memory size in bytes
    pub max_mesh_shared_memory_size: u32,
    /// Maximum mesh payload and shared memory size combined
    pub max_mesh_payload_and_shared_memory_size: u32,
    /// Maximum mesh output memory size in bytes
    pub max_mesh_output_memory_size: u32,
    /// Maximum mesh payload and output memory size combined
    pub max_mesh_payload_and_output_memory_size: u32,
    /// Maximum mesh output components
    pub max_mesh_output_components: u32,
    /// Maximum mesh output vertices
    pub max_mesh_output_vertices: u32,
    /// Maximum mesh output primitives
    pub max_mesh_output_primitives: u32,
    /// Maximum mesh output layers
    pub max_mesh_output_layers: u32,
    /// Maximum mesh multiview view count
    pub max_mesh_multiview_view_count: u32,
    /// Mesh output per vertex granularity
    pub mesh_output_per_vertex_granularity: u32,
    /// Mesh output per primitive granularity
    pub mesh_output_per_primitive_granularity: u32,
    /// Maximum preferred task work group invocations
    pub max_preferred_task_work_group_invocations: u32,
    /// Maximum preferred mesh work group invocations
    pub max_preferred_mesh_work_group_invocations: u32,
    /// Prefers local invocation vertex output
    pub prefers_local_invocation_vertex_output: bool,
    /// Prefers local invocation primitive output
    pub prefers_local_invocation_primitive_output: bool,
    /// Prefers compact vertex output
    pub prefers_compact_vertex_output: bool,
    /// Prefers compact primitive output
    pub prefers_compact_primitive_output: bool,
}

impl Default for MeshShaderLimits {
    fn default() -> Self {
        // Default to NVIDIA Ampere-class limits
        Self {
            max_task_work_group_count_x: 65535,
            max_task_work_group_count_y: 65535,
            max_task_work_group_count_z: 65535,
            max_task_work_group_invocations: 128,
            max_task_work_group_size_x: 128,
            max_task_work_group_size_y: 128,
            max_task_work_group_size_z: 128,
            max_task_payload_size: 16384,
            max_task_shared_memory_size: 32768,
            max_task_payload_and_shared_memory_size: 32768,
            max_mesh_work_group_count_x: 65535,
            max_mesh_work_group_count_y: 65535,
            max_mesh_work_group_count_z: 65535,
            max_mesh_work_group_invocations: 128,
            max_mesh_work_group_size_x: 128,
            max_mesh_work_group_size_y: 128,
            max_mesh_work_group_size_z: 128,
            max_mesh_shared_memory_size: 28672,
            max_mesh_payload_and_shared_memory_size: 28672,
            max_mesh_output_memory_size: 32768,
            max_mesh_payload_and_output_memory_size: 47104,
            max_mesh_output_components: 128,
            max_mesh_output_vertices: 256,
            max_mesh_output_primitives: 256,
            max_mesh_output_layers: 8,
            max_mesh_multiview_view_count: 4,
            mesh_output_per_vertex_granularity: 32,
            mesh_output_per_primitive_granularity: 32,
            max_preferred_task_work_group_invocations: 32,
            max_preferred_mesh_work_group_invocations: 128,
            prefers_local_invocation_vertex_output: true,
            prefers_local_invocation_primitive_output: true,
            prefers_compact_vertex_output: false,
            prefers_compact_primitive_output: false,
        }
    }
}

// ============================================================================
// Meshlet Structure
// ============================================================================

/// A meshlet is a small cluster of vertices/primitives
///
/// This is the fundamental unit of work for mesh shaders.
/// Typically 64-128 vertices and 64-128 triangles per meshlet.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Meshlet {
    /// Offset into the vertex index buffer
    pub vertex_offset: u32,
    /// Offset into the primitive index buffer
    pub primitive_offset: u32,
    /// Number of vertices in this meshlet
    pub vertex_count: u32,
    /// Number of primitives (triangles) in this meshlet
    pub primitive_count: u32,
}

impl Meshlet {
    /// Maximum vertices per meshlet (hardware limit)
    pub const MAX_VERTICES: u32 = 256;
    /// Maximum primitives per meshlet (hardware limit)
    pub const MAX_PRIMITIVES: u32 = 256;
    /// Recommended vertices per meshlet for efficiency
    pub const RECOMMENDED_VERTICES: u32 = 64;
    /// Recommended primitives per meshlet for efficiency
    pub const RECOMMENDED_PRIMITIVES: u32 = 124; // 126 for NV, 124 for AMD

    /// Create a new meshlet
    pub const fn new(
        vertex_offset: u32,
        primitive_offset: u32,
        vertex_count: u32,
        primitive_count: u32,
    ) -> Self {
        Self {
            vertex_offset,
            primitive_offset,
            vertex_count,
            primitive_count,
        }
    }

    /// Check if meshlet is valid
    pub const fn is_valid(&self) -> bool {
        self.vertex_count > 0 
            && self.vertex_count <= Self::MAX_VERTICES
            && self.primitive_count > 0 
            && self.primitive_count <= Self::MAX_PRIMITIVES
    }
}

/// Meshlet bounding information for culling
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct MeshletBounds {
    /// Bounding sphere center (object space)
    pub center: [f32; 3],
    /// Bounding sphere radius
    pub radius: f32,
    /// Cone apex (for backface culling)
    pub cone_apex: [f32; 3],
    /// Cone axis (normalized)
    pub cone_axis: [f32; 3],
    /// Cone cutoff (cos of half-angle, negative for >90°)
    pub cone_cutoff: f32,
    /// Axis-aligned bounding box min
    pub aabb_min: [f32; 3],
    /// Axis-aligned bounding box max
    pub aabb_max: [f32; 3],
}

impl MeshletBounds {
    /// Check if meshlet is potentially visible from camera
    pub fn is_potentially_visible(
        &self,
        view_position: [f32; 3],
        frustum_planes: &[[f32; 4]; 6],
    ) -> bool {
        // Frustum culling against bounding sphere
        for plane in frustum_planes {
            let dist = plane[0] * self.center[0]
                + plane[1] * self.center[1]
                + plane[2] * self.center[2]
                + plane[3];
            if dist < -self.radius {
                return false;
            }
        }

        // Backface cone culling
        if self.cone_cutoff < 1.0 {
            let dir = [
                self.cone_apex[0] - view_position[0],
                self.cone_apex[1] - view_position[1],
                self.cone_apex[2] - view_position[2],
            ];
            let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
            if len > 0.0 {
                let dir_norm = [dir[0] / len, dir[1] / len, dir[2] / len];
                let dot = dir_norm[0] * self.cone_axis[0]
                    + dir_norm[1] * self.cone_axis[1]
                    + dir_norm[2] * self.cone_axis[2];
                if dot >= self.cone_cutoff {
                    return false; // Entire meshlet is backfacing
                }
            }
        }

        true
    }
}

// ============================================================================
// Mesh Shader Stage Info
// ============================================================================

/// Mesh shader stage create info
#[derive(Clone, Debug)]
pub struct MeshShaderStageCreateInfo {
    /// SPIR-V bytecode
    pub code: ShaderCode,
    /// Entry point name
    pub entry_point: EntryPoint,
    /// Specialization constants
    pub specialization: Option<SpecializationInfo>,
    /// Required subgroup size (0 for default)
    pub required_subgroup_size: u32,
}

/// Task shader stage create info
#[derive(Clone, Debug)]
pub struct TaskShaderStageCreateInfo {
    /// SPIR-V bytecode
    pub code: ShaderCode,
    /// Entry point name
    pub entry_point: EntryPoint,
    /// Specialization constants
    pub specialization: Option<SpecializationInfo>,
    /// Required subgroup size (0 for default)
    pub required_subgroup_size: u32,
}

/// Shader code (SPIR-V or native)
#[derive(Clone, Debug)]
pub enum ShaderCode {
    /// SPIR-V bytecode
    SpirV(ShaderBytecode),
    /// Native GPU code (pre-compiled)
    Native(ShaderBytecode),
}

/// Raw shader bytecode
#[derive(Clone, Debug)]
pub struct ShaderBytecode {
    /// Raw bytes
    data: [u8; Self::MAX_SIZE],
    /// Actual size
    size: usize,
}

impl ShaderBytecode {
    /// Maximum shader size (64KB)
    pub const MAX_SIZE: usize = 64 * 1024;

    /// Create from slice
    pub fn from_slice(data: &[u8]) -> Result<Self> {
        if data.len() > Self::MAX_SIZE {
            return Err(Error::InvalidParameter);
        }
        let mut bytecode = Self {
            data: [0; Self::MAX_SIZE],
            size: data.len(),
        };
        bytecode.data[..data.len()].copy_from_slice(data);
        Ok(bytecode)
    }

    /// Get the bytecode as a slice
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.size]
    }

    /// Get size in bytes
    pub fn size(&self) -> usize {
        self.size
    }
}

/// Entry point name
#[derive(Clone, Debug)]
pub struct EntryPoint {
    name: [u8; 64],
    len: usize,
}

impl EntryPoint {
    /// Create from string
    pub fn new(name: &str) -> Self {
        let mut entry = Self {
            name: [0; 64],
            len: name.len().min(63),
        };
        entry.name[..entry.len].copy_from_slice(name.as_bytes());
        entry
    }

    /// Get as string slice
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.len]).unwrap_or("main")
    }

    /// Default "main" entry point
    pub const fn main() -> Self {
        let mut name = [0u8; 64];
        name[0] = b'm';
        name[1] = b'a';
        name[2] = b'i';
        name[3] = b'n';
        Self { name, len: 4 }
    }
}

impl Default for EntryPoint {
    fn default() -> Self {
        Self::main()
    }
}

/// Specialization constant info
#[derive(Clone, Debug)]
pub struct SpecializationInfo {
    /// Map entries
    pub entries: [SpecializationMapEntry; 16],
    /// Number of entries
    pub entry_count: u32,
    /// Data for all constants
    pub data: [u8; 256],
    /// Data size
    pub data_size: usize,
}

/// A single specialization constant
#[derive(Clone, Copy, Debug, Default)]
pub struct SpecializationMapEntry {
    /// Constant ID
    pub constant_id: u32,
    /// Offset in data
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
}

// ============================================================================
// Mesh Shader Pipeline Create Info
// ============================================================================

/// Create info for mesh shader pipeline
#[derive(Clone, Debug)]
pub struct MeshShaderPipelineCreateInfo {
    /// Pipeline layout
    pub layout: u64, // PipelineLayout handle
    /// Render pass
    pub render_pass: u64,
    /// Subpass index
    pub subpass: u32,
    /// Task shader stage (optional)
    pub task_shader: Option<TaskShaderStageCreateInfo>,
    /// Mesh shader stage (required)
    pub mesh_shader: MeshShaderStageCreateInfo,
    /// Fragment shader stage (required for rasterization)
    pub fragment_shader: Option<FragmentShaderStageCreateInfo>,
    /// Viewport state
    pub viewport_state: ViewportState,
    /// Rasterization state
    pub rasterization_state: RasterizationState,
    /// Multisample state
    pub multisample_state: MultisampleState,
    /// Depth stencil state
    pub depth_stencil_state: Option<DepthStencilState>,
    /// Color blend state
    pub color_blend_state: ColorBlendState,
    /// Dynamic state
    pub dynamic_state: DynamicState,
    /// Pipeline flags
    pub flags: MeshPipelineFlags,
}

/// Fragment shader stage create info
#[derive(Clone, Debug)]
pub struct FragmentShaderStageCreateInfo {
    /// Shader code
    pub code: ShaderCode,
    /// Entry point name
    pub entry_point: EntryPoint,
    /// Specialization constants
    pub specialization: Option<SpecializationInfo>,
}

/// Viewport state
#[derive(Clone, Debug, Default)]
pub struct ViewportState {
    /// Number of viewports
    pub viewport_count: u32,
    /// Number of scissors
    pub scissor_count: u32,
}

/// Rasterization state
#[derive(Clone, Debug)]
pub struct RasterizationState {
    /// Enable depth clamp
    pub depth_clamp_enable: bool,
    /// Discard all primitives
    pub rasterizer_discard_enable: bool,
    /// Polygon mode
    pub polygon_mode: PolygonMode,
    /// Cull mode
    pub cull_mode: CullMode,
    /// Front face winding
    pub front_face: FrontFace,
    /// Enable depth bias
    pub depth_bias_enable: bool,
    /// Depth bias constant factor
    pub depth_bias_constant_factor: f32,
    /// Depth bias clamp
    pub depth_bias_clamp: f32,
    /// Depth bias slope factor
    pub depth_bias_slope_factor: f32,
    /// Line width
    pub line_width: f32,
    /// Conservative rasterization mode
    pub conservative_rasterization: ConservativeRasterization,
}

impl Default for RasterizationState {
    fn default() -> Self {
        Self {
            depth_clamp_enable: false,
            rasterizer_discard_enable: false,
            polygon_mode: PolygonMode::Fill,
            cull_mode: CullMode::Back,
            front_face: FrontFace::CounterClockwise,
            depth_bias_enable: false,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
            conservative_rasterization: ConservativeRasterization::Disabled,
        }
    }
}

/// Polygon fill mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PolygonMode {
    #[default]
    Fill,
    Line,
    Point,
}

/// Face culling mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CullMode {
    None,
    Front,
    #[default]
    Back,
    FrontAndBack,
}

/// Front face winding order
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FrontFace {
    Clockwise,
    #[default]
    CounterClockwise,
}

/// Conservative rasterization mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ConservativeRasterization {
    #[default]
    Disabled,
    Overestimate,
    Underestimate,
}

/// Multisample state
#[derive(Clone, Debug)]
pub struct MultisampleState {
    /// Sample count
    pub rasterization_samples: SampleCount,
    /// Enable sample shading
    pub sample_shading_enable: bool,
    /// Minimum sample shading fraction
    pub min_sample_shading: f32,
    /// Sample mask
    pub sample_mask: u32,
    /// Enable alpha to coverage
    pub alpha_to_coverage_enable: bool,
    /// Enable alpha to one
    pub alpha_to_one_enable: bool,
}

impl Default for MultisampleState {
    fn default() -> Self {
        Self {
            rasterization_samples: SampleCount::S1,
            sample_shading_enable: false,
            min_sample_shading: 1.0,
            sample_mask: !0,
            alpha_to_coverage_enable: false,
            alpha_to_one_enable: false,
        }
    }
}

/// Sample count
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum SampleCount {
    #[default]
    S1 = 1,
    S2 = 2,
    S4 = 4,
    S8 = 8,
    S16 = 16,
    S32 = 32,
    S64 = 64,
}

/// Depth stencil state
#[derive(Clone, Debug)]
pub struct DepthStencilState {
    /// Enable depth testing
    pub depth_test_enable: bool,
    /// Enable depth writing
    pub depth_write_enable: bool,
    /// Depth compare operation
    pub depth_compare_op: CompareOp,
    /// Enable depth bounds testing
    pub depth_bounds_test_enable: bool,
    /// Enable stencil testing
    pub stencil_test_enable: bool,
    /// Front stencil state
    pub front: StencilOpState,
    /// Back stencil state
    pub back: StencilOpState,
    /// Minimum depth bound
    pub min_depth_bounds: f32,
    /// Maximum depth bound
    pub max_depth_bounds: f32,
}

impl Default for DepthStencilState {
    fn default() -> Self {
        Self {
            depth_test_enable: true,
            depth_write_enable: true,
            depth_compare_op: CompareOp::Less,
            depth_bounds_test_enable: false,
            stencil_test_enable: false,
            front: StencilOpState::default(),
            back: StencilOpState::default(),
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
        }
    }
}

/// Compare operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CompareOp {
    Never,
    #[default]
    Less,
    Equal,
    LessOrEqual,
    Greater,
    NotEqual,
    GreaterOrEqual,
    Always,
}

/// Stencil operation state
#[derive(Clone, Copy, Debug, Default)]
pub struct StencilOpState {
    pub fail_op: StencilOp,
    pub pass_op: StencilOp,
    pub depth_fail_op: StencilOp,
    pub compare_op: CompareOp,
    pub compare_mask: u32,
    pub write_mask: u32,
    pub reference: u32,
}

/// Stencil operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StencilOp {
    #[default]
    Keep,
    Zero,
    Replace,
    IncrementAndClamp,
    DecrementAndClamp,
    Invert,
    IncrementAndWrap,
    DecrementAndWrap,
}

/// Color blend state
#[derive(Clone, Debug)]
pub struct ColorBlendState {
    /// Enable logic operations
    pub logic_op_enable: bool,
    /// Logic operation
    pub logic_op: LogicOp,
    /// Per-attachment blend states
    pub attachments: [ColorBlendAttachmentState; 8],
    /// Number of attachments
    pub attachment_count: u32,
    /// Blend constants
    pub blend_constants: [f32; 4],
}

impl Default for ColorBlendState {
    fn default() -> Self {
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            attachments: [ColorBlendAttachmentState::default(); 8],
            attachment_count: 1,
            blend_constants: [0.0; 4],
        }
    }
}

/// Logic operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LogicOp {
    Clear,
    And,
    AndReverse,
    #[default]
    Copy,
    AndInverted,
    NoOp,
    Xor,
    Or,
    Nor,
    Equivalent,
    Invert,
    OrReverse,
    CopyInverted,
    OrInverted,
    Nand,
    Set,
}

/// Color blend attachment state
#[derive(Clone, Copy, Debug)]
pub struct ColorBlendAttachmentState {
    /// Enable blending
    pub blend_enable: bool,
    /// Source color blend factor
    pub src_color_blend_factor: BlendFactor,
    /// Destination color blend factor
    pub dst_color_blend_factor: BlendFactor,
    /// Color blend operation
    pub color_blend_op: BlendOp,
    /// Source alpha blend factor
    pub src_alpha_blend_factor: BlendFactor,
    /// Destination alpha blend factor
    pub dst_alpha_blend_factor: BlendFactor,
    /// Alpha blend operation
    pub alpha_blend_op: BlendOp,
    /// Color write mask
    pub color_write_mask: ColorComponentFlags,
}

impl Default for ColorBlendAttachmentState {
    fn default() -> Self {
        Self {
            blend_enable: false,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::Zero,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::Zero,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorComponentFlags::all(),
        }
    }
}

/// Blend factor
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BlendFactor {
    Zero,
    #[default]
    One,
    SrcColor,
    OneMinusSrcColor,
    DstColor,
    OneMinusDstColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
    ConstantColor,
    OneMinusConstantColor,
    ConstantAlpha,
    OneMinusConstantAlpha,
    SrcAlphaSaturate,
    Src1Color,
    OneMinusSrc1Color,
    Src1Alpha,
    OneMinusSrc1Alpha,
}

/// Blend operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BlendOp {
    #[default]
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

/// Color component flags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorComponentFlags(u8);

impl ColorComponentFlags {
    pub const R: Self = Self(0x01);
    pub const G: Self = Self(0x02);
    pub const B: Self = Self(0x04);
    pub const A: Self = Self(0x08);

    pub const fn all() -> Self {
        Self(0x0F)
    }

    pub const fn none() -> Self {
        Self(0)
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl Default for ColorComponentFlags {
    fn default() -> Self {
        Self::all()
    }
}

/// Dynamic state configuration
#[derive(Clone, Debug, Default)]
pub struct DynamicState {
    /// Dynamic states to enable
    pub states: DynamicStateFlags,
}

/// Dynamic state flags
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DynamicStateFlags(u32);

impl DynamicStateFlags {
    pub const NONE: Self = Self(0);
    pub const VIEWPORT: Self = Self(1 << 0);
    pub const SCISSOR: Self = Self(1 << 1);
    pub const LINE_WIDTH: Self = Self(1 << 2);
    pub const DEPTH_BIAS: Self = Self(1 << 3);
    pub const BLEND_CONSTANTS: Self = Self(1 << 4);
    pub const DEPTH_BOUNDS: Self = Self(1 << 5);
    pub const STENCIL_COMPARE_MASK: Self = Self(1 << 6);
    pub const STENCIL_WRITE_MASK: Self = Self(1 << 7);
    pub const STENCIL_REFERENCE: Self = Self(1 << 8);
    pub const CULL_MODE: Self = Self(1 << 9);
    pub const FRONT_FACE: Self = Self(1 << 10);
    pub const PRIMITIVE_TOPOLOGY: Self = Self(1 << 11);
    pub const VIEWPORT_WITH_COUNT: Self = Self(1 << 12);
    pub const SCISSOR_WITH_COUNT: Self = Self(1 << 13);

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Mesh pipeline creation flags
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MeshPipelineFlags(u32);

impl MeshPipelineFlags {
    pub const NONE: Self = Self(0);
    pub const DISABLE_OPTIMIZATION: Self = Self(1 << 0);
    pub const ALLOW_DERIVATIVES: Self = Self(1 << 1);
    pub const DERIVATIVE: Self = Self(1 << 2);
}

// ============================================================================
// Mesh Shader Pipeline Handle
// ============================================================================

/// Handle to a mesh shader pipeline
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MeshShaderPipeline {
    handle: u64,
}

impl MeshShaderPipeline {
    /// Null handle
    pub const fn null() -> Self {
        Self { handle: 0 }
    }

    /// Check if valid
    pub const fn is_valid(&self) -> bool {
        self.handle != 0
    }

    /// Raw handle
    pub const fn raw(&self) -> u64 {
        self.handle
    }

    /// Create from raw handle
    pub const unsafe fn from_raw(handle: u64) -> Self {
        Self { handle }
    }
}

// ============================================================================
// Draw Mesh Tasks Commands
// ============================================================================

/// Draw mesh tasks command
#[derive(Clone, Copy, Debug)]
pub struct DrawMeshTasksCommand {
    /// Number of task shader work groups in X
    pub group_count_x: u32,
    /// Number of task shader work groups in Y
    pub group_count_y: u32,
    /// Number of task shader work groups in Z
    pub group_count_z: u32,
}

impl DrawMeshTasksCommand {
    /// Create a new draw mesh tasks command
    pub const fn new(group_count_x: u32, group_count_y: u32, group_count_z: u32) -> Self {
        Self {
            group_count_x,
            group_count_y,
            group_count_z,
        }
    }

    /// Create for 1D dispatch
    pub const fn dispatch_1d(count: u32) -> Self {
        Self::new(count, 1, 1)
    }

    /// Total work groups
    pub const fn total_groups(&self) -> u32 {
        self.group_count_x * self.group_count_y * self.group_count_z
    }
}

/// Indirect draw mesh tasks command structure (for GPU-driven rendering)
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct DrawMeshTasksIndirectCommand {
    /// Number of task shader work groups in X
    pub group_count_x: u32,
    /// Number of task shader work groups in Y
    pub group_count_y: u32,
    /// Number of task shader work groups in Z
    pub group_count_z: u32,
}

// ============================================================================
// Command Buffer Extensions for Mesh Shaders
// ============================================================================

/// Extension trait for mesh shader commands on command buffers
pub trait MeshShaderCommands {
    /// Bind a mesh shader pipeline
    fn bind_mesh_pipeline(&mut self, pipeline: MeshShaderPipeline);

    /// Draw mesh tasks
    fn draw_mesh_tasks(&mut self, group_count_x: u32, group_count_y: u32, group_count_z: u32);

    /// Draw mesh tasks indirect
    fn draw_mesh_tasks_indirect(
        &mut self,
        buffer: u64, // Buffer handle
        offset: u64,
        draw_count: u32,
        stride: u32,
    );

    /// Draw mesh tasks indirect count
    fn draw_mesh_tasks_indirect_count(
        &mut self,
        buffer: u64,
        offset: u64,
        count_buffer: u64,
        count_offset: u64,
        max_draw_count: u32,
        stride: u32,
    );
}

// ============================================================================
// Meshlet Builder
// ============================================================================

/// Configuration for meshlet generation
#[derive(Clone, Debug)]
pub struct MeshletConfig {
    /// Maximum vertices per meshlet
    pub max_vertices: u32,
    /// Maximum primitives per meshlet
    pub max_primitives: u32,
    /// Cone weight for cluster cone culling (0.0-1.0)
    pub cone_weight: f32,
}

impl Default for MeshletConfig {
    fn default() -> Self {
        Self {
            max_vertices: Meshlet::RECOMMENDED_VERTICES,
            max_primitives: Meshlet::RECOMMENDED_PRIMITIVES,
            cone_weight: 0.5,
        }
    }
}

/// Result of meshlet generation
#[derive(Clone, Debug)]
pub struct MeshletBuildResult {
    /// Generated meshlets
    pub meshlets: [Meshlet; Self::MAX_MESHLETS],
    /// Number of meshlets generated
    pub meshlet_count: u32,
    /// Meshlet vertex indices (into original vertex buffer)
    pub meshlet_vertices: [u32; Self::MAX_VERTICES],
    /// Number of meshlet vertices
    pub meshlet_vertex_count: u32,
    /// Meshlet primitive indices (packed as u8 triplets)
    pub meshlet_primitives: [u8; Self::MAX_PRIMITIVES],
    /// Number of meshlet primitives
    pub meshlet_primitive_count: u32,
    /// Meshlet bounds for culling
    pub meshlet_bounds: [MeshletBounds; Self::MAX_MESHLETS],
}

impl MeshletBuildResult {
    /// Maximum meshlets per mesh
    pub const MAX_MESHLETS: usize = 4096;
    /// Maximum vertices across all meshlets
    pub const MAX_VERTICES: usize = 256 * 1024;
    /// Maximum primitives across all meshlets (packed as bytes)
    pub const MAX_PRIMITIVES: usize = 256 * 1024 * 3;
}
