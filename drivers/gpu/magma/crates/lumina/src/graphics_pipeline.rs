//! Graphics pipeline state
//!
//! This module provides types for configuring the graphics pipeline.

use crate::types::{ShaderHandle, PipelineHandle, VertexAttribute};
use crate::draw::{PrimitiveTopology, FrontFace, CullMode, PolygonMode, DepthBias};
use crate::compute::{TextureFormat, ShaderStageFlags};
use crate::bind_group::BindGroupLayoutHandle;

/// Graphics pipeline descriptor
#[derive(Clone, Debug)]
pub struct GraphicsPipelineDesc<'a> {
    /// Label for debugging
    pub label: Option<&'a str>,
    /// Vertex stage
    pub vertex: VertexState<'a>,
    /// Fragment stage (optional for depth-only passes)
    pub fragment: Option<FragmentState<'a>>,
    /// Primitive state
    pub primitive: PrimitiveState,
    /// Depth/stencil state
    pub depth_stencil: Option<DepthStencilState>,
    /// Multisample state
    pub multisample: MultisampleState,
    /// Bind group layouts
    pub bind_group_layouts: &'a [BindGroupLayoutHandle],
    /// Push constant ranges
    pub push_constant_ranges: &'a [crate::bind_group::PushConstantRange],
}

impl<'a> GraphicsPipelineDesc<'a> {
    /// Creates a minimal pipeline descriptor
    pub const fn new(vertex: VertexState<'a>) -> Self {
        Self {
            label: None,
            vertex,
            fragment: None,
            primitive: PrimitiveState::DEFAULT,
            depth_stencil: None,
            multisample: MultisampleState::DEFAULT,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        }
    }

    /// Sets the label
    pub const fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Sets the fragment state
    pub const fn with_fragment(mut self, fragment: FragmentState<'a>) -> Self {
        self.fragment = Some(fragment);
        self
    }

    /// Sets the primitive state
    pub const fn with_primitive(mut self, primitive: PrimitiveState) -> Self {
        self.primitive = primitive;
        self
    }

    /// Sets the depth/stencil state
    pub const fn with_depth_stencil(mut self, depth_stencil: DepthStencilState) -> Self {
        self.depth_stencil = Some(depth_stencil);
        self
    }

    /// Sets the multisample state
    pub const fn with_multisample(mut self, multisample: MultisampleState) -> Self {
        self.multisample = multisample;
        self
    }

    /// Sets bind group layouts
    pub const fn with_bind_groups(mut self, layouts: &'a [BindGroupLayoutHandle]) -> Self {
        self.bind_group_layouts = layouts;
        self
    }
}

/// Vertex stage configuration
#[derive(Clone, Debug)]
pub struct VertexState<'a> {
    /// Shader module
    pub shader: ShaderModule<'a>,
    /// Entry point function name
    pub entry_point: &'a str,
    /// Vertex buffer layouts
    pub buffers: &'a [VertexBufferLayout<'a>],
}

impl<'a> VertexState<'a> {
    /// Creates a vertex state
    pub const fn new(shader: ShaderModule<'a>, entry_point: &'a str) -> Self {
        Self {
            shader,
            entry_point,
            buffers: &[],
        }
    }

    /// Sets vertex buffer layouts
    pub const fn with_buffers(mut self, buffers: &'a [VertexBufferLayout<'a>]) -> Self {
        self.buffers = buffers;
        self
    }
}

/// Fragment stage configuration
#[derive(Clone, Debug)]
pub struct FragmentState<'a> {
    /// Shader module
    pub shader: ShaderModule<'a>,
    /// Entry point function name
    pub entry_point: &'a str,
    /// Color target states
    pub targets: &'a [ColorTargetState],
}

impl<'a> FragmentState<'a> {
    /// Creates a fragment state
    pub const fn new(shader: ShaderModule<'a>, entry_point: &'a str, targets: &'a [ColorTargetState]) -> Self {
        Self {
            shader,
            entry_point,
            targets,
        }
    }
}

/// Shader module source
#[derive(Clone, Debug)]
pub enum ShaderModule<'a> {
    /// SPIR-V binary
    SpirV(&'a [u32]),
    /// Pre-compiled shader handle
    Handle(ShaderHandle),
}

/// Vertex buffer layout
#[derive(Clone, Debug)]
pub struct VertexBufferLayout<'a> {
    /// Stride in bytes between vertices
    pub stride: u64,
    /// Step mode (per-vertex or per-instance)
    pub step_mode: VertexStepMode,
    /// Vertex attributes
    pub attributes: &'a [VertexAttribute],
}

impl<'a> VertexBufferLayout<'a> {
    /// Creates a per-vertex buffer layout
    pub const fn per_vertex(stride: u64, attributes: &'a [VertexAttribute]) -> Self {
        Self {
            stride,
            step_mode: VertexStepMode::Vertex,
            attributes,
        }
    }

    /// Creates a per-instance buffer layout
    pub const fn per_instance(stride: u64, attributes: &'a [VertexAttribute]) -> Self {
        Self {
            stride,
            step_mode: VertexStepMode::Instance,
            attributes,
        }
    }
}

/// Vertex step mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VertexStepMode {
    /// Advance per vertex
    #[default]
    Vertex,
    /// Advance per instance
    Instance,
}

/// Primitive state
#[derive(Clone, Copy, Debug)]
pub struct PrimitiveState {
    /// Primitive topology
    pub topology: PrimitiveTopology,
    /// Index strip cut value (for strip topologies with restart)
    pub strip_index_format: Option<crate::draw::IndexFormat>,
    /// Front face winding
    pub front_face: FrontFace,
    /// Cull mode
    pub cull_mode: CullMode,
    /// Polygon mode
    pub polygon_mode: PolygonMode,
    /// Enable conservative rasterization
    pub conservative: bool,
    /// Unclipped depth (requires feature)
    pub unclipped_depth: bool,
}

impl PrimitiveState {
    /// Default primitive state (filled triangles, CCW front, back-face culling)
    pub const DEFAULT: Self = Self {
        topology: PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: FrontFace::Ccw,
        cull_mode: CullMode::Back,
        polygon_mode: PolygonMode::Fill,
        conservative: false,
        unclipped_depth: false,
    };

    /// No culling
    pub const fn with_no_cull(mut self) -> Self {
        self.cull_mode = CullMode::None;
        self
    }

    /// Triangle strip topology
    pub const fn triangle_strip(mut self) -> Self {
        self.topology = PrimitiveTopology::TriangleStrip;
        self
    }

    /// Line list topology
    pub const fn line_list(mut self) -> Self {
        self.topology = PrimitiveTopology::LineList;
        self
    }

    /// Wireframe mode
    pub const fn wireframe(mut self) -> Self {
        self.polygon_mode = PolygonMode::Line;
        self.cull_mode = CullMode::None;
        self
    }
}

impl Default for PrimitiveState {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Depth/stencil state
#[derive(Clone, Copy, Debug)]
pub struct DepthStencilState {
    /// Depth/stencil format
    pub format: TextureFormat,
    /// Enable depth write
    pub depth_write_enabled: bool,
    /// Depth comparison function
    pub depth_compare: CompareFunction,
    /// Stencil state
    pub stencil: StencilState,
    /// Depth bias
    pub bias: DepthBias,
}

impl DepthStencilState {
    /// Creates a depth-only state with less-or-equal comparison
    pub const fn depth_less_equal(format: TextureFormat) -> Self {
        Self {
            format,
            depth_write_enabled: true,
            depth_compare: CompareFunction::LessEqual,
            stencil: StencilState::IGNORE,
            bias: DepthBias::NONE,
        }
    }

    /// Creates a depth-only state with less comparison
    pub const fn depth_less(format: TextureFormat) -> Self {
        Self {
            format,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
            stencil: StencilState::IGNORE,
            bias: DepthBias::NONE,
        }
    }

    /// Creates a read-only depth state
    pub const fn depth_read_only(format: TextureFormat) -> Self {
        Self {
            format,
            depth_write_enabled: false,
            depth_compare: CompareFunction::LessEqual,
            stencil: StencilState::IGNORE,
            bias: DepthBias::NONE,
        }
    }

    /// Sets depth bias
    pub const fn with_bias(mut self, bias: DepthBias) -> Self {
        self.bias = bias;
        self
    }

    /// Disables depth write
    pub const fn read_only(mut self) -> Self {
        self.depth_write_enabled = false;
        self
    }
}

/// Comparison function
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CompareFunction {
    /// Never pass
    Never,
    /// Pass if less
    Less,
    /// Pass if equal
    Equal,
    /// Pass if less or equal
    #[default]
    LessEqual,
    /// Pass if greater
    Greater,
    /// Pass if not equal
    NotEqual,
    /// Pass if greater or equal
    GreaterEqual,
    /// Always pass
    Always,
}

/// Stencil state
#[derive(Clone, Copy, Debug)]
pub struct StencilState {
    /// Front face stencil operations
    pub front: StencilFaceState,
    /// Back face stencil operations
    pub back: StencilFaceState,
    /// Stencil read mask
    pub read_mask: u32,
    /// Stencil write mask
    pub write_mask: u32,
}

impl StencilState {
    /// Ignore stencil (disabled)
    pub const IGNORE: Self = Self {
        front: StencilFaceState::IGNORE,
        back: StencilFaceState::IGNORE,
        read_mask: 0,
        write_mask: 0,
    };

    /// Default stencil (keep all)
    pub const DEFAULT: Self = Self {
        front: StencilFaceState::DEFAULT,
        back: StencilFaceState::DEFAULT,
        read_mask: 0xFFFFFFFF,
        write_mask: 0xFFFFFFFF,
    };
}

/// Stencil operations for one face
#[derive(Clone, Copy, Debug)]
pub struct StencilFaceState {
    /// Compare function
    pub compare: CompareFunction,
    /// Operation on stencil fail
    pub fail_op: StencilOperation,
    /// Operation on depth fail
    pub depth_fail_op: StencilOperation,
    /// Operation on pass
    pub pass_op: StencilOperation,
}

impl StencilFaceState {
    /// Ignore stencil
    pub const IGNORE: Self = Self {
        compare: CompareFunction::Always,
        fail_op: StencilOperation::Keep,
        depth_fail_op: StencilOperation::Keep,
        pass_op: StencilOperation::Keep,
    };

    /// Default stencil
    pub const DEFAULT: Self = Self::IGNORE;
}

/// Stencil operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StencilOperation {
    /// Keep current value
    #[default]
    Keep,
    /// Set to zero
    Zero,
    /// Replace with reference value
    Replace,
    /// Increment and clamp
    IncrementClamp,
    /// Decrement and clamp
    DecrementClamp,
    /// Bitwise invert
    Invert,
    /// Increment and wrap
    IncrementWrap,
    /// Decrement and wrap
    DecrementWrap,
}

/// Multisample state
#[derive(Clone, Copy, Debug)]
pub struct MultisampleState {
    /// Number of samples per pixel
    pub count: u32,
    /// Sample mask
    pub mask: u64,
    /// Enable alpha-to-coverage
    pub alpha_to_coverage_enabled: bool,
}

impl MultisampleState {
    /// No multisampling (1x)
    pub const DEFAULT: Self = Self {
        count: 1,
        mask: !0,
        alpha_to_coverage_enabled: false,
    };

    /// 4x MSAA
    pub const MSAA_4X: Self = Self {
        count: 4,
        mask: !0,
        alpha_to_coverage_enabled: false,
    };

    /// 8x MSAA
    pub const MSAA_8X: Self = Self {
        count: 8,
        mask: !0,
        alpha_to_coverage_enabled: false,
    };

    /// Creates multisampling with specified sample count
    pub const fn samples(count: u32) -> Self {
        Self {
            count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        }
    }

    /// Enables alpha-to-coverage
    pub const fn with_alpha_to_coverage(mut self) -> Self {
        self.alpha_to_coverage_enabled = true;
        self
    }
}

impl Default for MultisampleState {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Color target state
#[derive(Clone, Copy, Debug)]
pub struct ColorTargetState {
    /// Target format
    pub format: TextureFormat,
    /// Blending configuration
    pub blend: Option<BlendState>,
    /// Write mask
    pub write_mask: ColorWrites,
}

impl ColorTargetState {
    /// Creates a color target with no blending
    pub const fn new(format: TextureFormat) -> Self {
        Self {
            format,
            blend: None,
            write_mask: ColorWrites::ALL,
        }
    }

    /// Creates a color target with alpha blending
    pub const fn alpha_blend(format: TextureFormat) -> Self {
        Self {
            format,
            blend: Some(BlendState::ALPHA_BLENDING),
            write_mask: ColorWrites::ALL,
        }
    }

    /// Creates a color target with premultiplied alpha blending
    pub const fn premultiplied_alpha(format: TextureFormat) -> Self {
        Self {
            format,
            blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            write_mask: ColorWrites::ALL,
        }
    }

    /// Sets the blend state
    pub const fn with_blend(mut self, blend: BlendState) -> Self {
        self.blend = Some(blend);
        self
    }

    /// Sets the write mask
    pub const fn with_write_mask(mut self, mask: ColorWrites) -> Self {
        self.write_mask = mask;
        self
    }
}

/// Blend state
#[derive(Clone, Copy, Debug)]
pub struct BlendState {
    /// Color blend component
    pub color: BlendComponent,
    /// Alpha blend component
    pub alpha: BlendComponent,
}

impl BlendState {
    /// Replace (no blending)
    pub const REPLACE: Self = Self {
        color: BlendComponent::REPLACE,
        alpha: BlendComponent::REPLACE,
    };

    /// Standard alpha blending
    pub const ALPHA_BLENDING: Self = Self {
        color: BlendComponent {
            src_factor: BlendFactor::SrcAlpha,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
        alpha: BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
    };

    /// Premultiplied alpha blending
    pub const PREMULTIPLIED_ALPHA_BLENDING: Self = Self {
        color: BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
        alpha: BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
    };

    /// Additive blending
    pub const ADDITIVE: Self = Self {
        color: BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        },
        alpha: BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        },
    };
}

/// Blend component configuration
#[derive(Clone, Copy, Debug)]
pub struct BlendComponent {
    /// Source factor
    pub src_factor: BlendFactor,
    /// Destination factor
    pub dst_factor: BlendFactor,
    /// Blend operation
    pub operation: BlendOperation,
}

impl BlendComponent {
    /// Replace (overwrites destination)
    pub const REPLACE: Self = Self {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::Zero,
        operation: BlendOperation::Add,
    };
}

/// Blend factor
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendFactor {
    /// 0
    Zero,
    /// 1
    One,
    /// Source color
    Src,
    /// 1 - source color
    OneMinusSrc,
    /// Source alpha
    SrcAlpha,
    /// 1 - source alpha
    OneMinusSrcAlpha,
    /// Destination color
    Dst,
    /// 1 - destination color
    OneMinusDst,
    /// Destination alpha
    DstAlpha,
    /// 1 - destination alpha
    OneMinusDstAlpha,
    /// min(src_alpha, 1 - dst_alpha)
    SrcAlphaSaturated,
    /// Constant color
    Constant,
    /// 1 - constant color
    OneMinusConstant,
}

/// Blend operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BlendOperation {
    /// Add
    #[default]
    Add,
    /// Subtract (src - dst)
    Subtract,
    /// Reverse subtract (dst - src)
    ReverseSubtract,
    /// Minimum
    Min,
    /// Maximum
    Max,
}

/// Color write mask flags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorWrites(pub u32);

impl ColorWrites {
    /// Write red
    pub const RED: Self = Self(1 << 0);
    /// Write green
    pub const GREEN: Self = Self(1 << 1);
    /// Write blue
    pub const BLUE: Self = Self(1 << 2);
    /// Write alpha
    pub const ALPHA: Self = Self(1 << 3);
    /// Write all channels
    pub const ALL: Self = Self(0xF);
    /// Write no channels
    pub const NONE: Self = Self(0);
}

impl core::ops::BitOr for ColorWrites {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for ColorWrites {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}
