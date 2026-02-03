//! GPU Pipeline State Manager for Lumina
//!
//! This module provides comprehensive pipeline state management including
//! state caching, dynamic state, and state inheritance.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Pipeline State Manager Handles
// ============================================================================

/// Pipeline state manager handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuPipelineStateManagerHandle(pub u64);

impl GpuPipelineStateManagerHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for GpuPipelineStateManagerHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Pipeline state handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineStateHandle(pub u64);

impl PipelineStateHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Is null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for PipelineStateHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// State block handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct StateBlockHandle(pub u64);

impl StateBlockHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for StateBlockHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// State cache handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct StateCacheHandle(pub u64);

impl StateCacheHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for StateCacheHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Pipeline State Manager Creation
// ============================================================================

/// Pipeline state manager create info
#[derive(Clone, Debug)]
pub struct GpuPipelineStateManagerCreateInfo {
    /// Name
    pub name: String,
    /// Max states
    pub max_states: u32,
    /// Max state blocks
    pub max_state_blocks: u32,
    /// Cache size
    pub cache_size: u32,
    /// Features
    pub features: PipelineStateFeatures,
}

impl GpuPipelineStateManagerCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_states: 10000,
            max_state_blocks: 1000,
            cache_size: 1000,
            features: PipelineStateFeatures::all(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max states
    pub fn with_max_states(mut self, count: u32) -> Self {
        self.max_states = count;
        self
    }

    /// With cache size
    pub fn with_cache_size(mut self, size: u32) -> Self {
        self.cache_size = size;
        self
    }

    /// With features
    pub fn with_features(mut self, features: PipelineStateFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Standard preset
    pub fn standard() -> Self {
        Self::new()
    }

    /// Large preset
    pub fn large() -> Self {
        Self::new().with_max_states(100000).with_cache_size(10000)
    }
}

impl Default for GpuPipelineStateManagerCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Pipeline state features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct PipelineStateFeatures: u32 {
        /// None
        const NONE = 0;
        /// State caching
        const CACHING = 1 << 0;
        /// State inheritance
        const INHERITANCE = 1 << 1;
        /// Dynamic state
        const DYNAMIC_STATE = 1 << 2;
        /// State blocks
        const STATE_BLOCKS = 1 << 3;
        /// State validation
        const VALIDATION = 1 << 4;
        /// State diffing
        const DIFFING = 1 << 5;
        /// All
        const ALL = 0x3F;
    }
}

impl Default for PipelineStateFeatures {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Rasterization State
// ============================================================================

/// Rasterization state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RasterizationState {
    /// Fill mode
    pub fill_mode: FillMode,
    /// Cull mode
    pub cull_mode: CullMode,
    /// Front face
    pub front_face: FrontFace,
    /// Depth clamp enable
    pub depth_clamp_enable: bool,
    /// Rasterizer discard
    pub rasterizer_discard: bool,
    /// Depth bias enable
    pub depth_bias_enable: bool,
    /// Depth bias constant
    pub depth_bias_constant: f32,
    /// Depth bias clamp
    pub depth_bias_clamp: f32,
    /// Depth bias slope
    pub depth_bias_slope: f32,
    /// Line width
    pub line_width: f32,
    /// Conservative rasterization
    pub conservative_raster: ConservativeRaster,
}

impl RasterizationState {
    /// Creates default state
    pub const fn new() -> Self {
        Self {
            fill_mode: FillMode::Fill,
            cull_mode: CullMode::Back,
            front_face: FrontFace::CounterClockwise,
            depth_clamp_enable: false,
            rasterizer_discard: false,
            depth_bias_enable: false,
            depth_bias_constant: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope: 0.0,
            line_width: 1.0,
            conservative_raster: ConservativeRaster::Disabled,
        }
    }

    /// With fill mode
    pub const fn with_fill_mode(mut self, mode: FillMode) -> Self {
        self.fill_mode = mode;
        self
    }

    /// With cull mode
    pub const fn with_cull_mode(mut self, mode: CullMode) -> Self {
        self.cull_mode = mode;
        self
    }

    /// With front face
    pub const fn with_front_face(mut self, face: FrontFace) -> Self {
        self.front_face = face;
        self
    }

    /// With depth bias
    pub const fn with_depth_bias(mut self, constant: f32, clamp: f32, slope: f32) -> Self {
        self.depth_bias_enable = true;
        self.depth_bias_constant = constant;
        self.depth_bias_clamp = clamp;
        self.depth_bias_slope = slope;
        self
    }

    /// With line width
    pub const fn with_line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }

    /// Solid preset
    pub const fn solid() -> Self {
        Self::new()
    }

    /// Wireframe preset
    pub const fn wireframe() -> Self {
        Self::new().with_fill_mode(FillMode::Line)
    }

    /// No culling preset
    pub const fn no_cull() -> Self {
        Self::new().with_cull_mode(CullMode::None)
    }

    /// Front cull preset
    pub const fn front_cull() -> Self {
        Self::new().with_cull_mode(CullMode::Front)
    }

    /// Shadow preset
    pub const fn shadow() -> Self {
        Self::new().with_depth_bias(1.5, 0.0, 1.75)
    }
}

impl Default for RasterizationState {
    fn default() -> Self {
        Self::new()
    }
}

/// Fill mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FillMode {
    /// Filled
    #[default]
    Fill  = 0,
    /// Lines
    Line  = 1,
    /// Points
    Point = 2,
}

/// Cull mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CullMode {
    /// None
    None         = 0,
    /// Front
    Front        = 1,
    /// Back
    #[default]
    Back         = 2,
    /// Both
    FrontAndBack = 3,
}

/// Front face
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FrontFace {
    /// Counter clockwise
    #[default]
    CounterClockwise = 0,
    /// Clockwise
    Clockwise        = 1,
}

/// Conservative rasterization
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ConservativeRaster {
    /// Disabled
    #[default]
    Disabled      = 0,
    /// Overestimate
    Overestimate  = 1,
    /// Underestimate
    Underestimate = 2,
}

// ============================================================================
// Depth Stencil State
// ============================================================================

/// Depth stencil state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DepthStencilState {
    /// Depth test enable
    pub depth_test_enable: bool,
    /// Depth write enable
    pub depth_write_enable: bool,
    /// Depth compare op
    pub depth_compare_op: CompareOp,
    /// Depth bounds test enable
    pub depth_bounds_enable: bool,
    /// Min depth bounds
    pub min_depth_bounds: f32,
    /// Max depth bounds
    pub max_depth_bounds: f32,
    /// Stencil test enable
    pub stencil_test_enable: bool,
    /// Front stencil op
    pub front: StencilOpState,
    /// Back stencil op
    pub back: StencilOpState,
}

impl DepthStencilState {
    /// Creates default state
    pub const fn new() -> Self {
        Self {
            depth_test_enable: true,
            depth_write_enable: true,
            depth_compare_op: CompareOp::Less,
            depth_bounds_enable: false,
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
            stencil_test_enable: false,
            front: StencilOpState::default_const(),
            back: StencilOpState::default_const(),
        }
    }

    /// With depth test
    pub const fn with_depth_test(mut self, enable: bool) -> Self {
        self.depth_test_enable = enable;
        self
    }

    /// With depth write
    pub const fn with_depth_write(mut self, enable: bool) -> Self {
        self.depth_write_enable = enable;
        self
    }

    /// With depth compare
    pub const fn with_depth_compare(mut self, op: CompareOp) -> Self {
        self.depth_compare_op = op;
        self
    }

    /// With stencil test
    pub const fn with_stencil(mut self, front: StencilOpState, back: StencilOpState) -> Self {
        self.stencil_test_enable = true;
        self.front = front;
        self.back = back;
        self
    }

    /// Disabled preset
    pub const fn disabled() -> Self {
        Self {
            depth_test_enable: false,
            depth_write_enable: false,
            depth_compare_op: CompareOp::Always,
            depth_bounds_enable: false,
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
            stencil_test_enable: false,
            front: StencilOpState::default_const(),
            back: StencilOpState::default_const(),
        }
    }

    /// Depth read only preset
    pub const fn depth_read() -> Self {
        Self::new().with_depth_write(false)
    }

    /// Depth write only preset
    pub const fn depth_write() -> Self {
        Self::new().with_depth_test(false)
    }

    /// Equal test preset (for deferred)
    pub const fn depth_equal() -> Self {
        Self::new()
            .with_depth_compare(CompareOp::Equal)
            .with_depth_write(false)
    }

    /// Reverse Z preset
    pub const fn reverse_z() -> Self {
        Self::new().with_depth_compare(CompareOp::Greater)
    }
}

impl Default for DepthStencilState {
    fn default() -> Self {
        Self::new()
    }
}

/// Compare operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CompareOp {
    /// Never
    Never          = 0,
    /// Less than
    #[default]
    Less           = 1,
    /// Equal
    Equal          = 2,
    /// Less or equal
    LessOrEqual    = 3,
    /// Greater
    Greater        = 4,
    /// Not equal
    NotEqual       = 5,
    /// Greater or equal
    GreaterOrEqual = 6,
    /// Always
    Always         = 7,
}

/// Stencil operation state
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
    /// Default const
    pub const fn default_const() -> Self {
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
        Self::default_const()
    }
}

/// Stencil operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StencilOp {
    /// Keep
    #[default]
    Keep           = 0,
    /// Zero
    Zero           = 1,
    /// Replace
    Replace        = 2,
    /// Increment and clamp
    IncrementClamp = 3,
    /// Decrement and clamp
    DecrementClamp = 4,
    /// Invert
    Invert         = 5,
    /// Increment and wrap
    IncrementWrap  = 6,
    /// Decrement and wrap
    DecrementWrap  = 7,
}

// ============================================================================
// Blend State
// ============================================================================

/// Blend state
#[derive(Clone, Debug)]
#[repr(C)]
pub struct BlendState {
    /// Logic op enable
    pub logic_op_enable: bool,
    /// Logic op
    pub logic_op: LogicOp,
    /// Blend constants
    pub blend_constants: [f32; 4],
    /// Attachments
    pub attachments: Vec<BlendAttachment>,
}

impl BlendState {
    /// Creates default state
    pub fn new() -> Self {
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            blend_constants: [0.0; 4],
            attachments: Vec::new(),
        }
    }

    /// With attachment
    pub fn with_attachment(mut self, attachment: BlendAttachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// With blend constants
    pub fn with_constants(mut self, constants: [f32; 4]) -> Self {
        self.blend_constants = constants;
        self
    }

    /// Opaque preset
    pub fn opaque() -> Self {
        Self::new().with_attachment(BlendAttachment::opaque())
    }

    /// Alpha blend preset
    pub fn alpha_blend() -> Self {
        Self::new().with_attachment(BlendAttachment::alpha_blend())
    }

    /// Additive preset
    pub fn additive() -> Self {
        Self::new().with_attachment(BlendAttachment::additive())
    }

    /// Premultiplied alpha preset
    pub fn premultiplied() -> Self {
        Self::new().with_attachment(BlendAttachment::premultiplied())
    }
}

impl Default for BlendState {
    fn default() -> Self {
        Self::new()
    }
}

/// Blend attachment
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BlendAttachment {
    /// Blend enable
    pub blend_enable: bool,
    /// Src color factor
    pub src_color_factor: BlendFactor,
    /// Dst color factor
    pub dst_color_factor: BlendFactor,
    /// Color blend op
    pub color_blend_op: BlendOp,
    /// Src alpha factor
    pub src_alpha_factor: BlendFactor,
    /// Dst alpha factor
    pub dst_alpha_factor: BlendFactor,
    /// Alpha blend op
    pub alpha_blend_op: BlendOp,
    /// Color write mask
    pub color_write_mask: ColorWriteMask,
}

impl BlendAttachment {
    /// Creates default
    pub const fn new() -> Self {
        Self {
            blend_enable: false,
            src_color_factor: BlendFactor::One,
            dst_color_factor: BlendFactor::Zero,
            color_blend_op: BlendOp::Add,
            src_alpha_factor: BlendFactor::One,
            dst_alpha_factor: BlendFactor::Zero,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorWriteMask::ALL,
        }
    }

    /// Opaque preset
    pub const fn opaque() -> Self {
        Self::new()
    }

    /// Alpha blend preset
    pub const fn alpha_blend() -> Self {
        Self {
            blend_enable: true,
            src_color_factor: BlendFactor::SrcAlpha,
            dst_color_factor: BlendFactor::OneMinusSrcAlpha,
            color_blend_op: BlendOp::Add,
            src_alpha_factor: BlendFactor::One,
            dst_alpha_factor: BlendFactor::OneMinusSrcAlpha,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorWriteMask::ALL,
        }
    }

    /// Additive preset
    pub const fn additive() -> Self {
        Self {
            blend_enable: true,
            src_color_factor: BlendFactor::One,
            dst_color_factor: BlendFactor::One,
            color_blend_op: BlendOp::Add,
            src_alpha_factor: BlendFactor::One,
            dst_alpha_factor: BlendFactor::One,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorWriteMask::ALL,
        }
    }

    /// Premultiplied alpha preset
    pub const fn premultiplied() -> Self {
        Self {
            blend_enable: true,
            src_color_factor: BlendFactor::One,
            dst_color_factor: BlendFactor::OneMinusSrcAlpha,
            color_blend_op: BlendOp::Add,
            src_alpha_factor: BlendFactor::One,
            dst_alpha_factor: BlendFactor::OneMinusSrcAlpha,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorWriteMask::ALL,
        }
    }

    /// Multiply preset
    pub const fn multiply() -> Self {
        Self {
            blend_enable: true,
            src_color_factor: BlendFactor::DstColor,
            dst_color_factor: BlendFactor::Zero,
            color_blend_op: BlendOp::Add,
            src_alpha_factor: BlendFactor::DstAlpha,
            dst_alpha_factor: BlendFactor::Zero,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorWriteMask::ALL,
        }
    }
}

impl Default for BlendAttachment {
    fn default() -> Self {
        Self::new()
    }
}

/// Blend factor
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BlendFactor {
    /// Zero
    Zero             = 0,
    /// One
    #[default]
    One              = 1,
    /// Src color
    SrcColor         = 2,
    /// One minus src color
    OneMinusSrcColor = 3,
    /// Dst color
    DstColor         = 4,
    /// One minus dst color
    OneMinusDstColor = 5,
    /// Src alpha
    SrcAlpha         = 6,
    /// One minus src alpha
    OneMinusSrcAlpha = 7,
    /// Dst alpha
    DstAlpha         = 8,
    /// One minus dst alpha
    OneMinusDstAlpha = 9,
    /// Constant color
    ConstantColor    = 10,
    /// One minus constant color
    OneMinusConstantColor = 11,
    /// Constant alpha
    ConstantAlpha    = 12,
    /// One minus constant alpha
    OneMinusConstantAlpha = 13,
    /// Src alpha saturate
    SrcAlphaSaturate = 14,
}

/// Blend operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
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

/// Logic operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
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

bitflags::bitflags! {
    /// Color write mask
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct ColorWriteMask: u8 {
        /// Red
        const R = 1 << 0;
        /// Green
        const G = 1 << 1;
        /// Blue
        const B = 1 << 2;
        /// Alpha
        const A = 1 << 3;
        /// RGB
        const RGB = Self::R.bits() | Self::G.bits() | Self::B.bits();
        /// All
        const ALL = Self::R.bits() | Self::G.bits() | Self::B.bits() | Self::A.bits();
    }
}

impl Default for ColorWriteMask {
    fn default() -> Self {
        Self::ALL
    }
}

// ============================================================================
// Dynamic State
// ============================================================================

bitflags::bitflags! {
    /// Dynamic state flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct DynamicStateFlags: u32 {
        /// None
        const NONE = 0;
        /// Viewport
        const VIEWPORT = 1 << 0;
        /// Scissor
        const SCISSOR = 1 << 1;
        /// Line width
        const LINE_WIDTH = 1 << 2;
        /// Depth bias
        const DEPTH_BIAS = 1 << 3;
        /// Blend constants
        const BLEND_CONSTANTS = 1 << 4;
        /// Depth bounds
        const DEPTH_BOUNDS = 1 << 5;
        /// Stencil compare mask
        const STENCIL_COMPARE_MASK = 1 << 6;
        /// Stencil write mask
        const STENCIL_WRITE_MASK = 1 << 7;
        /// Stencil reference
        const STENCIL_REFERENCE = 1 << 8;
        /// Cull mode
        const CULL_MODE = 1 << 9;
        /// Front face
        const FRONT_FACE = 1 << 10;
        /// Primitive topology
        const PRIMITIVE_TOPOLOGY = 1 << 11;
        /// Viewport with count
        const VIEWPORT_WITH_COUNT = 1 << 12;
        /// Scissor with count
        const SCISSOR_WITH_COUNT = 1 << 13;
        /// Depth test enable
        const DEPTH_TEST_ENABLE = 1 << 14;
        /// Depth write enable
        const DEPTH_WRITE_ENABLE = 1 << 15;
        /// Depth compare op
        const DEPTH_COMPARE_OP = 1 << 16;
        /// Stencil test enable
        const STENCIL_TEST_ENABLE = 1 << 17;
        /// Stencil op
        const STENCIL_OP = 1 << 18;
        /// Common dynamic state
        const COMMON = Self::VIEWPORT.bits() | Self::SCISSOR.bits();
        /// Extended dynamic state
        const EXTENDED = Self::CULL_MODE.bits() | Self::FRONT_FACE.bits() |
                        Self::PRIMITIVE_TOPOLOGY.bits() | Self::DEPTH_TEST_ENABLE.bits() |
                        Self::DEPTH_WRITE_ENABLE.bits() | Self::DEPTH_COMPARE_OP.bits();
        /// All
        const ALL = 0x7FFFF;
    }
}

impl Default for DynamicStateFlags {
    fn default() -> Self {
        Self::COMMON
    }
}

// ============================================================================
// Complete Pipeline State
// ============================================================================

/// Complete pipeline state
#[derive(Clone, Debug)]
pub struct CompletePipelineState {
    /// Rasterization
    pub rasterization: RasterizationState,
    /// Depth stencil
    pub depth_stencil: DepthStencilState,
    /// Blend
    pub blend: BlendState,
    /// Dynamic state
    pub dynamic_state: DynamicStateFlags,
    /// Multisample
    pub multisample: MultisampleState,
}

impl CompletePipelineState {
    /// Creates new state
    pub fn new() -> Self {
        Self {
            rasterization: RasterizationState::new(),
            depth_stencil: DepthStencilState::new(),
            blend: BlendState::opaque(),
            dynamic_state: DynamicStateFlags::COMMON,
            multisample: MultisampleState::new(),
        }
    }

    /// With rasterization
    pub fn with_rasterization(mut self, state: RasterizationState) -> Self {
        self.rasterization = state;
        self
    }

    /// With depth stencil
    pub fn with_depth_stencil(mut self, state: DepthStencilState) -> Self {
        self.depth_stencil = state;
        self
    }

    /// With blend
    pub fn with_blend(mut self, state: BlendState) -> Self {
        self.blend = state;
        self
    }

    /// With dynamic state
    pub fn with_dynamic_state(mut self, flags: DynamicStateFlags) -> Self {
        self.dynamic_state = flags;
        self
    }

    /// Opaque preset
    pub fn opaque() -> Self {
        Self::new()
    }

    /// Transparent preset
    pub fn transparent() -> Self {
        Self::new()
            .with_blend(BlendState::alpha_blend())
            .with_depth_stencil(DepthStencilState::depth_read())
    }

    /// UI preset
    pub fn ui() -> Self {
        Self::new()
            .with_blend(BlendState::premultiplied())
            .with_depth_stencil(DepthStencilState::disabled())
            .with_rasterization(RasterizationState::no_cull())
    }

    /// Shadow preset
    pub fn shadow() -> Self {
        Self::new().with_rasterization(RasterizationState::shadow())
    }
}

impl Default for CompletePipelineState {
    fn default() -> Self {
        Self::new()
    }
}

/// Multisample state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MultisampleState {
    /// Sample count
    pub sample_count: u32,
    /// Sample shading enable
    pub sample_shading_enable: bool,
    /// Min sample shading
    pub min_sample_shading: f32,
    /// Alpha to coverage enable
    pub alpha_to_coverage_enable: bool,
    /// Alpha to one enable
    pub alpha_to_one_enable: bool,
}

impl MultisampleState {
    /// Creates new state
    pub const fn new() -> Self {
        Self {
            sample_count: 1,
            sample_shading_enable: false,
            min_sample_shading: 1.0,
            alpha_to_coverage_enable: false,
            alpha_to_one_enable: false,
        }
    }

    /// With sample count
    pub const fn with_samples(mut self, count: u32) -> Self {
        self.sample_count = count;
        self
    }

    /// With sample shading
    pub const fn with_sample_shading(mut self, min: f32) -> Self {
        self.sample_shading_enable = true;
        self.min_sample_shading = min;
        self
    }

    /// Msaa 2x
    pub const fn msaa_2x() -> Self {
        Self::new().with_samples(2)
    }

    /// Msaa 4x
    pub const fn msaa_4x() -> Self {
        Self::new().with_samples(4)
    }

    /// Msaa 8x
    pub const fn msaa_8x() -> Self {
        Self::new().with_samples(8)
    }
}

impl Default for MultisampleState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// State Cache
// ============================================================================

/// State cache entry
#[derive(Clone, Debug)]
pub struct StateCacheEntry {
    /// Hash
    pub hash: u64,
    /// State handle
    pub state_handle: PipelineStateHandle,
    /// Last access frame
    pub last_access: u64,
    /// Hit count
    pub hit_count: u64,
}

/// State cache statistics
#[derive(Clone, Debug, Default)]
pub struct StateCacheStats {
    /// Cache size
    pub size: u32,
    /// Capacity
    pub capacity: u32,
    /// Hits
    pub hits: u64,
    /// Misses
    pub misses: u64,
    /// Evictions
    pub evictions: u64,
}

impl StateCacheStats {
    /// Hit rate
    pub fn hit_rate(&self) -> f32 {
        let total = self.hits + self.misses;
        if total > 0 {
            self.hits as f32 / total as f32
        } else {
            0.0
        }
    }

    /// Usage ratio
    pub fn usage_ratio(&self) -> f32 {
        if self.capacity > 0 {
            self.size as f32 / self.capacity as f32
        } else {
            0.0
        }
    }
}

// ============================================================================
// State Block
// ============================================================================

/// State block create info
#[derive(Clone, Debug)]
pub struct StateBlockCreateInfo {
    /// Name
    pub name: String,
    /// Rasterization (optional)
    pub rasterization: Option<RasterizationState>,
    /// Depth stencil (optional)
    pub depth_stencil: Option<DepthStencilState>,
    /// Blend (optional)
    pub blend: Option<BlendState>,
}

impl StateBlockCreateInfo {
    /// Creates new info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            rasterization: None,
            depth_stencil: None,
            blend: None,
        }
    }

    /// With rasterization
    pub fn with_rasterization(mut self, state: RasterizationState) -> Self {
        self.rasterization = Some(state);
        self
    }

    /// With depth stencil
    pub fn with_depth_stencil(mut self, state: DepthStencilState) -> Self {
        self.depth_stencil = Some(state);
        self
    }

    /// With blend
    pub fn with_blend(mut self, state: BlendState) -> Self {
        self.blend = Some(state);
        self
    }
}

// ============================================================================
// GPU Params
// ============================================================================

/// GPU rasterization state params
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuRasterizationParams {
    /// Fill mode
    pub fill_mode: u32,
    /// Cull mode
    pub cull_mode: u32,
    /// Front face
    pub front_face: u32,
    /// Depth bias enabled
    pub depth_bias_enabled: u32,
    /// Depth bias constant
    pub depth_bias_constant: f32,
    /// Depth bias clamp
    pub depth_bias_clamp: f32,
    /// Depth bias slope
    pub depth_bias_slope: f32,
    /// Line width
    pub line_width: f32,
}

impl Default for GpuRasterizationParams {
    fn default() -> Self {
        Self {
            fill_mode: 0,
            cull_mode: 2,
            front_face: 0,
            depth_bias_enabled: 0,
            depth_bias_constant: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope: 0.0,
            line_width: 1.0,
        }
    }
}

/// GPU depth stencil params
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuDepthStencilParams {
    /// Depth test enabled
    pub depth_test_enabled: u32,
    /// Depth write enabled
    pub depth_write_enabled: u32,
    /// Depth compare op
    pub depth_compare_op: u32,
    /// Stencil test enabled
    pub stencil_test_enabled: u32,
    /// Front stencil
    pub front_fail: u32,
    /// Front pass
    pub front_pass: u32,
    /// Front depth fail
    pub front_depth_fail: u32,
    /// Front compare
    pub front_compare: u32,
}

impl Default for GpuDepthStencilParams {
    fn default() -> Self {
        Self {
            depth_test_enabled: 1,
            depth_write_enabled: 1,
            depth_compare_op: 1, // Less
            stencil_test_enabled: 0,
            front_fail: 0,
            front_pass: 0,
            front_depth_fail: 0,
            front_compare: 7, // Always
        }
    }
}

/// GPU blend params
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuBlendParams {
    /// Blend enabled
    pub blend_enabled: u32,
    /// Src color factor
    pub src_color_factor: u32,
    /// Dst color factor
    pub dst_color_factor: u32,
    /// Color blend op
    pub color_blend_op: u32,
    /// Src alpha factor
    pub src_alpha_factor: u32,
    /// Dst alpha factor
    pub dst_alpha_factor: u32,
    /// Alpha blend op
    pub alpha_blend_op: u32,
    /// Color write mask
    pub color_write_mask: u32,
}

impl Default for GpuBlendParams {
    fn default() -> Self {
        Self {
            blend_enabled: 0,
            src_color_factor: 1, // One
            dst_color_factor: 0, // Zero
            color_blend_op: 0,   // Add
            src_alpha_factor: 1,
            dst_alpha_factor: 0,
            alpha_blend_op: 0,
            color_write_mask: 0xF, // All
        }
    }
}
