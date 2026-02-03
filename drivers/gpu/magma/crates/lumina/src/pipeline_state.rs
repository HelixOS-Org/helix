//! Pipeline layout and state types
//!
//! This module provides types for pipeline layout configuration.

use core::num::NonZeroU32;

/// Pipeline layout handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineLayoutHandle(pub NonZeroU32);

impl PipelineLayoutHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

/// Pipeline layout create info
#[derive(Clone, Debug)]
pub struct PipelineLayoutCreateInfo {
    /// Descriptor set layouts
    pub set_layouts: alloc::vec::Vec<DescriptorSetLayoutHandle>,
    /// Push constant ranges
    pub push_constant_ranges: alloc::vec::Vec<PushConstantRange>,
    /// Creation flags
    pub flags: PipelineLayoutCreateFlags,
}

use alloc::vec::Vec;

/// Descriptor set layout handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorSetLayoutHandle(pub NonZeroU32);

impl DescriptorSetLayoutHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

impl PipelineLayoutCreateInfo {
    /// Creates an empty layout
    pub fn new() -> Self {
        Self {
            set_layouts: Vec::new(),
            push_constant_ranges: Vec::new(),
            flags: PipelineLayoutCreateFlags::empty(),
        }
    }

    /// Simple layout with one descriptor set
    pub fn single_set(layout: DescriptorSetLayoutHandle) -> Self {
        Self {
            set_layouts: alloc::vec![layout],
            push_constant_ranges: Vec::new(),
            flags: PipelineLayoutCreateFlags::empty(),
        }
    }

    /// Adds a descriptor set layout
    pub fn add_set_layout(mut self, layout: DescriptorSetLayoutHandle) -> Self {
        self.set_layouts.push(layout);
        self
    }

    /// Adds a push constant range
    pub fn add_push_constant(mut self, range: PushConstantRange) -> Self {
        self.push_constant_ranges.push(range);
        self
    }

    /// With push constants for all graphics stages
    pub fn with_graphics_push_constants(self, size: u32) -> Self {
        self.add_push_constant(PushConstantRange::graphics(0, size))
    }

    /// With push constants for compute
    pub fn with_compute_push_constants(self, size: u32) -> Self {
        self.add_push_constant(PushConstantRange::compute(0, size))
    }
}

impl Default for PipelineLayoutCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Push constant range
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PushConstantRange {
    /// Shader stages that can access this range
    pub stage_flags: ShaderStageFlags,
    /// Offset in bytes
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
}

impl PushConstantRange {
    /// Creates a new range
    pub const fn new(stages: ShaderStageFlags, offset: u32, size: u32) -> Self {
        Self {
            stage_flags: stages,
            offset,
            size,
        }
    }

    /// For all graphics stages
    pub const fn graphics(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::ALL_GRAPHICS, offset, size)
    }

    /// For compute
    pub const fn compute(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::COMPUTE, offset, size)
    }

    /// For vertex and fragment
    pub const fn vertex_fragment(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::VERTEX_FRAGMENT, offset, size)
    }

    /// For all stages
    pub const fn all(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::ALL, offset, size)
    }
}

bitflags::bitflags! {
    /// Shader stage flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ShaderStageFlags: u32 {
        /// Vertex shader
        const VERTEX = 1 << 0;
        /// Tessellation control shader
        const TESSELLATION_CONTROL = 1 << 1;
        /// Tessellation evaluation shader
        const TESSELLATION_EVALUATION = 1 << 2;
        /// Geometry shader
        const GEOMETRY = 1 << 3;
        /// Fragment shader
        const FRAGMENT = 1 << 4;
        /// Compute shader
        const COMPUTE = 1 << 5;
        /// All graphics stages
        const ALL_GRAPHICS = Self::VERTEX.bits()
            | Self::TESSELLATION_CONTROL.bits()
            | Self::TESSELLATION_EVALUATION.bits()
            | Self::GEOMETRY.bits()
            | Self::FRAGMENT.bits();
        /// All stages
        const ALL = 0x7FFFFFFF;
        /// Task shader
        const TASK = 1 << 6;
        /// Mesh shader
        const MESH = 1 << 7;
        /// Ray generation
        const RAYGEN = 1 << 8;
        /// Any hit
        const ANY_HIT = 1 << 9;
        /// Closest hit
        const CLOSEST_HIT = 1 << 10;
        /// Miss
        const MISS = 1 << 11;
        /// Intersection
        const INTERSECTION = 1 << 12;
        /// Callable
        const CALLABLE = 1 << 13;
    }
}

impl ShaderStageFlags {
    /// Vertex and fragment
    pub const VERTEX_FRAGMENT: Self =
        Self::from_bits_truncate(Self::VERTEX.bits() | Self::FRAGMENT.bits());

    /// All ray tracing stages
    pub const ALL_RAY_TRACING: Self = Self::from_bits_truncate(
        Self::RAYGEN.bits()
            | Self::ANY_HIT.bits()
            | Self::CLOSEST_HIT.bits()
            | Self::MISS.bits()
            | Self::INTERSECTION.bits()
            | Self::CALLABLE.bits(),
    );
}

bitflags::bitflags! {
    /// Pipeline layout creation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PipelineLayoutCreateFlags: u32 {
        /// Independent sets
        const INDEPENDENT_SETS = 1 << 1;
    }
}

impl PipelineLayoutCreateFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Graphics pipeline state info
#[derive(Clone, Debug)]
pub struct GraphicsPipelineStateInfo {
    /// Vertex input state
    pub vertex_input: VertexInputStateInfo,
    /// Input assembly state
    pub input_assembly: InputAssemblyStateInfo,
    /// Tessellation state (optional)
    pub tessellation: Option<TessellationStateInfo>,
    /// Viewport state
    pub viewport: ViewportStateInfo,
    /// Rasterization state
    pub rasterization: RasterizationStateInfo,
    /// Multisample state
    pub multisample: MultisampleStateInfo,
    /// Depth stencil state (optional)
    pub depth_stencil: Option<DepthStencilStateInfo>,
    /// Color blend state
    pub color_blend: ColorBlendStateInfo,
    /// Dynamic state
    pub dynamic_state: DynamicStateInfo,
}

impl GraphicsPipelineStateInfo {
    /// Creates default state
    pub fn new() -> Self {
        Self {
            vertex_input: VertexInputStateInfo::new(),
            input_assembly: InputAssemblyStateInfo::triangle_list(),
            tessellation: None,
            viewport: ViewportStateInfo::single(),
            rasterization: RasterizationStateInfo::default(),
            multisample: MultisampleStateInfo::disabled(),
            depth_stencil: None,
            color_blend: ColorBlendStateInfo::disabled(),
            dynamic_state: DynamicStateInfo::viewport_scissor(),
        }
    }

    /// With depth testing
    pub fn with_depth(mut self) -> Self {
        self.depth_stencil = Some(DepthStencilStateInfo::depth_test());
        self
    }

    /// With depth testing and writing
    pub fn with_depth_write(mut self) -> Self {
        self.depth_stencil = Some(DepthStencilStateInfo::depth_write());
        self
    }

    /// With blending
    pub fn with_blend(mut self) -> Self {
        self.color_blend = ColorBlendStateInfo::alpha_blend();
        self
    }
}

impl Default for GraphicsPipelineStateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Vertex input state info
#[derive(Clone, Debug, Default)]
pub struct VertexInputStateInfo {
    /// Vertex binding descriptions
    pub bindings: Vec<VertexInputBindingDescription>,
    /// Vertex attribute descriptions
    pub attributes: Vec<VertexInputAttributeDescription>,
}

impl VertexInputStateInfo {
    /// Creates empty vertex input state
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            attributes: Vec::new(),
        }
    }

    /// Adds a binding
    pub fn add_binding(mut self, binding: VertexInputBindingDescription) -> Self {
        self.bindings.push(binding);
        self
    }

    /// Adds an attribute
    pub fn add_attribute(mut self, attribute: VertexInputAttributeDescription) -> Self {
        self.attributes.push(attribute);
        self
    }
}

/// Vertex input binding description
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VertexInputBindingDescription {
    /// Binding index
    pub binding: u32,
    /// Stride in bytes
    pub stride: u32,
    /// Input rate
    pub input_rate: VertexInputRate,
}

impl VertexInputBindingDescription {
    /// Per-vertex input
    pub const fn vertex(binding: u32, stride: u32) -> Self {
        Self {
            binding,
            stride,
            input_rate: VertexInputRate::Vertex,
        }
    }

    /// Per-instance input
    pub const fn instance(binding: u32, stride: u32) -> Self {
        Self {
            binding,
            stride,
            input_rate: VertexInputRate::Instance,
        }
    }
}

/// Vertex input rate
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum VertexInputRate {
    /// Per vertex
    #[default]
    Vertex   = 0,
    /// Per instance
    Instance = 1,
}

/// Vertex input attribute description
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VertexInputAttributeDescription {
    /// Location
    pub location: u32,
    /// Binding
    pub binding: u32,
    /// Format
    pub format: VertexFormat,
    /// Offset in bytes
    pub offset: u32,
}

impl VertexInputAttributeDescription {
    /// Creates an attribute
    pub const fn new(location: u32, binding: u32, format: VertexFormat, offset: u32) -> Self {
        Self {
            location,
            binding,
            format,
            offset,
        }
    }
}

/// Vertex format
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum VertexFormat {
    /// Float
    #[default]
    Float      = 100,
    /// Vec2
    Float2     = 103,
    /// Vec3
    Float3     = 106,
    /// Vec4
    Float4     = 109,
    /// Signed int
    Int        = 98,
    /// IVec2
    Int2       = 101,
    /// IVec3
    Int3       = 104,
    /// IVec4
    Int4       = 107,
    /// Unsigned int
    Uint       = 99,
    /// UVec2
    Uint2      = 102,
    /// UVec3
    Uint3      = 105,
    /// UVec4
    Uint4      = 108,
    /// Byte4 normalized
    Byte4Norm  = 37,
    /// Short2 normalized
    Short2Norm = 77,
    /// Short4 normalized
    Short4Norm = 91,
    /// Half2
    Half2      = 83,
    /// Half4
    Half4      = 97,
}

impl VertexFormat {
    /// Size in bytes
    pub const fn size(self) -> u32 {
        match self {
            Self::Float | Self::Int | Self::Uint | Self::Byte4Norm => 4,
            Self::Float2 | Self::Int2 | Self::Uint2 | Self::Half2 | Self::Short2Norm => 8,
            Self::Float3 | Self::Int3 | Self::Uint3 => 12,
            Self::Float4 | Self::Int4 | Self::Uint4 | Self::Half4 | Self::Short4Norm => 16,
        }
    }
}

/// Input assembly state info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct InputAssemblyStateInfo {
    /// Primitive topology
    pub topology: PrimitiveTopology,
    /// Enable primitive restart
    pub primitive_restart_enable: bool,
}

impl InputAssemblyStateInfo {
    /// Triangle list
    pub const fn triangle_list() -> Self {
        Self {
            topology: PrimitiveTopology::TriangleList,
            primitive_restart_enable: false,
        }
    }

    /// Triangle strip
    pub const fn triangle_strip() -> Self {
        Self {
            topology: PrimitiveTopology::TriangleStrip,
            primitive_restart_enable: false,
        }
    }

    /// Line list
    pub const fn line_list() -> Self {
        Self {
            topology: PrimitiveTopology::LineList,
            primitive_restart_enable: false,
        }
    }

    /// Point list
    pub const fn point_list() -> Self {
        Self {
            topology: PrimitiveTopology::PointList,
            primitive_restart_enable: false,
        }
    }

    /// With primitive restart
    pub const fn with_restart(mut self) -> Self {
        self.primitive_restart_enable = true;
        self
    }
}

/// Primitive topology
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum PrimitiveTopology {
    /// Point list
    PointList     = 0,
    /// Line list
    LineList      = 1,
    /// Line strip
    LineStrip     = 2,
    /// Triangle list
    #[default]
    TriangleList  = 3,
    /// Triangle strip
    TriangleStrip = 4,
    /// Triangle fan
    TriangleFan   = 5,
    /// Line list with adjacency
    LineListWithAdjacency = 6,
    /// Line strip with adjacency
    LineStripWithAdjacency = 7,
    /// Triangle list with adjacency
    TriangleListWithAdjacency = 8,
    /// Triangle strip with adjacency
    TriangleStripWithAdjacency = 9,
    /// Patch list
    PatchList     = 10,
}

/// Tessellation state info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TessellationStateInfo {
    /// Patch control points
    pub patch_control_points: u32,
}

impl TessellationStateInfo {
    /// Creates tessellation state
    pub const fn new(control_points: u32) -> Self {
        Self {
            patch_control_points: control_points,
        }
    }
}

/// Viewport state info
#[derive(Clone, Debug)]
pub struct ViewportStateInfo {
    /// Viewport count
    pub viewport_count: u32,
    /// Scissor count
    pub scissor_count: u32,
}

impl ViewportStateInfo {
    /// Single viewport and scissor
    pub const fn single() -> Self {
        Self {
            viewport_count: 1,
            scissor_count: 1,
        }
    }

    /// Multiple viewports
    pub const fn multiple(count: u32) -> Self {
        Self {
            viewport_count: count,
            scissor_count: count,
        }
    }
}

/// Rasterization state info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RasterizationStateInfo {
    /// Depth clamp enable
    pub depth_clamp_enable: bool,
    /// Rasterizer discard enable
    pub rasterizer_discard_enable: bool,
    /// Polygon mode
    pub polygon_mode: PolygonMode,
    /// Cull mode
    pub cull_mode: CullModeFlags,
    /// Front face
    pub front_face: FrontFace,
    /// Depth bias enable
    pub depth_bias_enable: bool,
    /// Depth bias constant factor
    pub depth_bias_constant_factor: f32,
    /// Depth bias clamp
    pub depth_bias_clamp: f32,
    /// Depth bias slope factor
    pub depth_bias_slope_factor: f32,
    /// Line width
    pub line_width: f32,
}

impl RasterizationStateInfo {
    /// Creates default state
    pub const fn default() -> Self {
        Self {
            depth_clamp_enable: false,
            rasterizer_discard_enable: false,
            polygon_mode: PolygonMode::Fill,
            cull_mode: CullModeFlags::BACK,
            front_face: FrontFace::CounterClockwise,
            depth_bias_enable: false,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        }
    }

    /// No culling
    pub const fn no_cull() -> Self {
        Self {
            cull_mode: CullModeFlags::NONE,
            ..Self::default()
        }
    }

    /// Wireframe mode
    pub const fn wireframe() -> Self {
        Self {
            polygon_mode: PolygonMode::Line,
            cull_mode: CullModeFlags::NONE,
            ..Self::default()
        }
    }

    /// With depth bias
    pub const fn with_depth_bias(mut self, constant: f32, slope: f32) -> Self {
        self.depth_bias_enable = true;
        self.depth_bias_constant_factor = constant;
        self.depth_bias_slope_factor = slope;
        self
    }
}

impl Default for RasterizationStateInfo {
    fn default() -> Self {
        Self::default()
    }
}

/// Polygon mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum PolygonMode {
    /// Fill polygons
    #[default]
    Fill  = 0,
    /// Line (wireframe)
    Line  = 1,
    /// Point
    Point = 2,
}

bitflags::bitflags! {
    /// Cull mode flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct CullModeFlags: u32 {
        /// No culling
        const NONE = 0;
        /// Cull front faces
        const FRONT = 1 << 0;
        /// Cull back faces
        const BACK = 1 << 1;
        /// Cull both
        const FRONT_AND_BACK = Self::FRONT.bits() | Self::BACK.bits();
    }
}

/// Front face
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum FrontFace {
    /// Counter-clockwise
    #[default]
    CounterClockwise = 0,
    /// Clockwise
    Clockwise        = 1,
}

/// Multisample state info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MultisampleStateInfo {
    /// Rasterization samples
    pub rasterization_samples: SampleCount,
    /// Sample shading enable
    pub sample_shading_enable: bool,
    /// Minimum sample shading
    pub min_sample_shading: f32,
    /// Sample mask
    pub sample_mask: u32,
    /// Alpha to coverage enable
    pub alpha_to_coverage_enable: bool,
    /// Alpha to one enable
    pub alpha_to_one_enable: bool,
}

impl MultisampleStateInfo {
    /// Disabled (1 sample)
    pub const fn disabled() -> Self {
        Self {
            rasterization_samples: SampleCount::S1,
            sample_shading_enable: false,
            min_sample_shading: 1.0,
            sample_mask: !0,
            alpha_to_coverage_enable: false,
            alpha_to_one_enable: false,
        }
    }

    /// MSAA 4x
    pub const fn msaa_4x() -> Self {
        Self {
            rasterization_samples: SampleCount::S4,
            ..Self::disabled()
        }
    }

    /// MSAA 8x
    pub const fn msaa_8x() -> Self {
        Self {
            rasterization_samples: SampleCount::S8,
            ..Self::disabled()
        }
    }

    /// With sample shading
    pub const fn with_sample_shading(mut self, min: f32) -> Self {
        self.sample_shading_enable = true;
        self.min_sample_shading = min;
        self
    }

    /// With alpha to coverage
    pub const fn with_alpha_to_coverage(mut self) -> Self {
        self.alpha_to_coverage_enable = true;
        self
    }
}

/// Sample count
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum SampleCount {
    /// 1 sample
    #[default]
    S1  = 1,
    /// 2 samples
    S2  = 2,
    /// 4 samples
    S4  = 4,
    /// 8 samples
    S8  = 8,
    /// 16 samples
    S16 = 16,
    /// 32 samples
    S32 = 32,
    /// 64 samples
    S64 = 64,
}

/// Depth stencil state info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DepthStencilStateInfo {
    /// Depth test enable
    pub depth_test_enable: bool,
    /// Depth write enable
    pub depth_write_enable: bool,
    /// Depth compare op
    pub depth_compare_op: CompareOp,
    /// Depth bounds test enable
    pub depth_bounds_test_enable: bool,
    /// Stencil test enable
    pub stencil_test_enable: bool,
    /// Front stencil op state
    pub front: StencilOpState,
    /// Back stencil op state
    pub back: StencilOpState,
    /// Min depth bounds
    pub min_depth_bounds: f32,
    /// Max depth bounds
    pub max_depth_bounds: f32,
}

impl DepthStencilStateInfo {
    /// Depth test only (no write)
    pub const fn depth_test() -> Self {
        Self {
            depth_test_enable: true,
            depth_write_enable: false,
            depth_compare_op: CompareOp::Less,
            depth_bounds_test_enable: false,
            stencil_test_enable: false,
            front: StencilOpState::default(),
            back: StencilOpState::default(),
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
        }
    }

    /// Depth test and write
    pub const fn depth_write() -> Self {
        Self {
            depth_write_enable: true,
            ..Self::depth_test()
        }
    }

    /// Depth test with less-or-equal
    pub const fn depth_less_equal() -> Self {
        Self {
            depth_compare_op: CompareOp::LessOrEqual,
            ..Self::depth_write()
        }
    }

    /// With stencil test
    pub const fn with_stencil(mut self, front: StencilOpState, back: StencilOpState) -> Self {
        self.stencil_test_enable = true;
        self.front = front;
        self.back = back;
        self
    }
}

/// Compare operation
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum CompareOp {
    /// Never pass
    Never          = 0,
    /// Less than
    #[default]
    Less           = 1,
    /// Equal
    Equal          = 2,
    /// Less than or equal
    LessOrEqual    = 3,
    /// Greater than
    Greater        = 4,
    /// Not equal
    NotEqual       = 5,
    /// Greater than or equal
    GreaterOrEqual = 6,
    /// Always pass
    Always         = 7,
}

/// Stencil op state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct StencilOpState {
    /// Fail op
    pub fail_op: StencilOp,
    /// Pass op
    pub pass_op: StencilOp,
    /// Depth fail op
    pub depth_fail_op: StencilOp,
    /// Compare op
    pub compare_op: CompareOp,
    /// Compare mask
    pub compare_mask: u32,
    /// Write mask
    pub write_mask: u32,
    /// Reference
    pub reference: u32,
}

impl StencilOpState {
    /// Default (no stencil)
    pub const fn default() -> Self {
        Self {
            fail_op: StencilOp::Keep,
            pass_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
            compare_op: CompareOp::Always,
            compare_mask: 0xFF,
            write_mask: 0xFF,
            reference: 0,
        }
    }
}

impl Default for StencilOpState {
    fn default() -> Self {
        Self::default()
    }
}

/// Stencil operation
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum StencilOp {
    /// Keep value
    #[default]
    Keep              = 0,
    /// Zero
    Zero              = 1,
    /// Replace
    Replace           = 2,
    /// Increment and clamp
    IncrementAndClamp = 3,
    /// Decrement and clamp
    DecrementAndClamp = 4,
    /// Invert
    Invert            = 5,
    /// Increment and wrap
    IncrementAndWrap  = 6,
    /// Decrement and wrap
    DecrementAndWrap  = 7,
}

/// Color blend state info
#[derive(Clone, Debug)]
pub struct ColorBlendStateInfo {
    /// Logic op enable
    pub logic_op_enable: bool,
    /// Logic op
    pub logic_op: LogicOp,
    /// Attachment states
    pub attachments: Vec<ColorBlendAttachmentState>,
    /// Blend constants
    pub blend_constants: [f32; 4],
}

impl ColorBlendStateInfo {
    /// Disabled blending
    pub fn disabled() -> Self {
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            attachments: alloc::vec![ColorBlendAttachmentState::disabled()],
            blend_constants: [0.0; 4],
        }
    }

    /// Alpha blending
    pub fn alpha_blend() -> Self {
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            attachments: alloc::vec![ColorBlendAttachmentState::alpha_blend()],
            blend_constants: [0.0; 4],
        }
    }

    /// Premultiplied alpha
    pub fn premultiplied() -> Self {
        Self {
            attachments: alloc::vec![ColorBlendAttachmentState::premultiplied()],
            ..Self::disabled()
        }
    }

    /// Additive blending
    pub fn additive() -> Self {
        Self {
            attachments: alloc::vec![ColorBlendAttachmentState::additive()],
            ..Self::disabled()
        }
    }

    /// With custom attachments
    pub fn with_attachments(attachments: Vec<ColorBlendAttachmentState>) -> Self {
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            attachments,
            blend_constants: [0.0; 4],
        }
    }
}

/// Logic operation
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum LogicOp {
    /// Clear
    Clear        = 0,
    /// And
    And          = 1,
    /// And reverse
    AndReverse   = 2,
    /// Copy
    #[default]
    Copy         = 3,
    /// And inverted
    AndInverted  = 4,
    /// No op
    NoOp         = 5,
    /// Xor
    Xor          = 6,
    /// Or
    Or           = 7,
    /// Nor
    Nor          = 8,
    /// Equivalent
    Equivalent   = 9,
    /// Invert
    Invert       = 10,
    /// Or reverse
    OrReverse    = 11,
    /// Copy inverted
    CopyInverted = 12,
    /// Or inverted
    OrInverted   = 13,
    /// Nand
    Nand         = 14,
    /// Set
    Set          = 15,
}

/// Color blend attachment state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ColorBlendAttachmentState {
    /// Blend enable
    pub blend_enable: bool,
    /// Source color blend factor
    pub src_color_blend_factor: BlendFactor,
    /// Destination color blend factor
    pub dst_color_blend_factor: BlendFactor,
    /// Color blend op
    pub color_blend_op: BlendOp,
    /// Source alpha blend factor
    pub src_alpha_blend_factor: BlendFactor,
    /// Destination alpha blend factor
    pub dst_alpha_blend_factor: BlendFactor,
    /// Alpha blend op
    pub alpha_blend_op: BlendOp,
    /// Color write mask
    pub color_write_mask: ColorComponentFlags,
}

impl ColorBlendAttachmentState {
    /// Disabled blending
    pub const fn disabled() -> Self {
        Self {
            blend_enable: false,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::Zero,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::Zero,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorComponentFlags::RGBA,
        }
    }

    /// Standard alpha blending
    pub const fn alpha_blend() -> Self {
        Self {
            blend_enable: true,
            src_color_blend_factor: BlendFactor::SrcAlpha,
            dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorComponentFlags::RGBA,
        }
    }

    /// Premultiplied alpha blending
    pub const fn premultiplied() -> Self {
        Self {
            blend_enable: true,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorComponentFlags::RGBA,
        }
    }

    /// Additive blending
    pub const fn additive() -> Self {
        Self {
            blend_enable: true,
            src_color_blend_factor: BlendFactor::SrcAlpha,
            dst_color_blend_factor: BlendFactor::One,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::One,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorComponentFlags::RGBA,
        }
    }
}

/// Blend factor
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum BlendFactor {
    /// Zero
    Zero              = 0,
    /// One
    #[default]
    One               = 1,
    /// Source color
    SrcColor          = 2,
    /// One minus source color
    OneMinusSrcColor  = 3,
    /// Destination color
    DstColor          = 4,
    /// One minus destination color
    OneMinusDstColor  = 5,
    /// Source alpha
    SrcAlpha          = 6,
    /// One minus source alpha
    OneMinusSrcAlpha  = 7,
    /// Destination alpha
    DstAlpha          = 8,
    /// One minus destination alpha
    OneMinusDstAlpha  = 9,
    /// Constant color
    ConstantColor     = 10,
    /// One minus constant color
    OneMinusConstantColor = 11,
    /// Constant alpha
    ConstantAlpha     = 12,
    /// One minus constant alpha
    OneMinusConstantAlpha = 13,
    /// Source alpha saturate
    SrcAlphaSaturate  = 14,
    /// Source 1 color
    Src1Color         = 15,
    /// One minus source 1 color
    OneMinusSrc1Color = 16,
    /// Source 1 alpha
    Src1Alpha         = 17,
    /// One minus source 1 alpha
    OneMinusSrc1Alpha = 18,
}

/// Blend operation
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum BlendOp {
    /// Add
    #[default]
    Add             = 0,
    /// Subtract
    Subtract        = 1,
    /// Reverse subtract
    ReverseSubtract = 2,
    /// Min
    Min             = 3,
    /// Max
    Max             = 4,
}

bitflags::bitflags! {
    /// Color component flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ColorComponentFlags: u32 {
        /// Red component
        const R = 1 << 0;
        /// Green component
        const G = 1 << 1;
        /// Blue component
        const B = 1 << 2;
        /// Alpha component
        const A = 1 << 3;
        /// RGB components
        const RGB = Self::R.bits() | Self::G.bits() | Self::B.bits();
        /// All components
        const RGBA = Self::R.bits() | Self::G.bits() | Self::B.bits() | Self::A.bits();
    }
}

/// Dynamic state info
#[derive(Clone, Debug, Default)]
pub struct DynamicStateInfo {
    /// Dynamic states
    pub dynamic_states: Vec<DynamicState>,
}

impl DynamicStateInfo {
    /// No dynamic state
    pub fn none() -> Self {
        Self {
            dynamic_states: Vec::new(),
        }
    }

    /// Viewport and scissor dynamic
    pub fn viewport_scissor() -> Self {
        Self {
            dynamic_states: alloc::vec![DynamicState::Viewport, DynamicState::Scissor],
        }
    }

    /// Full dynamic state (Vulkan 1.3)
    pub fn extended() -> Self {
        Self {
            dynamic_states: alloc::vec![
                DynamicState::Viewport,
                DynamicState::Scissor,
                DynamicState::LineWidth,
                DynamicState::DepthBias,
                DynamicState::BlendConstants,
                DynamicState::DepthBounds,
                DynamicState::StencilCompareMask,
                DynamicState::StencilWriteMask,
                DynamicState::StencilReference,
                DynamicState::CullMode,
                DynamicState::FrontFace,
                DynamicState::PrimitiveTopology,
                DynamicState::DepthTestEnable,
                DynamicState::DepthWriteEnable,
                DynamicState::DepthCompareOp,
                DynamicState::DepthBoundsTestEnable,
                DynamicState::StencilTestEnable,
                DynamicState::StencilOp,
                DynamicState::RasterizerDiscardEnable,
                DynamicState::DepthBiasEnable,
                DynamicState::PrimitiveRestartEnable,
            ],
        }
    }

    /// Adds a dynamic state
    pub fn add(mut self, state: DynamicState) -> Self {
        self.dynamic_states.push(state);
        self
    }
}

/// Dynamic state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum DynamicState {
    /// Viewport
    Viewport           = 0,
    /// Scissor
    Scissor            = 1,
    /// Line width
    LineWidth          = 2,
    /// Depth bias
    DepthBias          = 3,
    /// Blend constants
    BlendConstants     = 4,
    /// Depth bounds
    DepthBounds        = 5,
    /// Stencil compare mask
    StencilCompareMask = 6,
    /// Stencil write mask
    StencilWriteMask   = 7,
    /// Stencil reference
    StencilReference   = 8,
    /// Cull mode
    CullMode           = 1000267000,
    /// Front face
    FrontFace          = 1000267001,
    /// Primitive topology
    PrimitiveTopology  = 1000267002,
    /// Viewport with count
    ViewportWithCount  = 1000267003,
    /// Scissor with count
    ScissorWithCount   = 1000267004,
    /// Vertex input binding stride
    VertexInputBindingStride = 1000267005,
    /// Depth test enable
    DepthTestEnable    = 1000267006,
    /// Depth write enable
    DepthWriteEnable   = 1000267007,
    /// Depth compare op
    DepthCompareOp     = 1000267008,
    /// Depth bounds test enable
    DepthBoundsTestEnable = 1000267009,
    /// Stencil test enable
    StencilTestEnable  = 1000267010,
    /// Stencil op
    StencilOp          = 1000267011,
    /// Rasterizer discard enable
    RasterizerDiscardEnable = 1000377001,
    /// Depth bias enable
    DepthBiasEnable    = 1000377002,
    /// Primitive restart enable
    PrimitiveRestartEnable = 1000377004,
}
