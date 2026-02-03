//! Rasterization state types
//!
//! This module provides types for rasterization configuration.

/// Polygon mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PolygonMode {
    /// Fill polygons
    #[default]
    Fill,
    /// Draw lines
    Line,
    /// Draw points
    Point,
    /// Fill rectangle (NV)
    FillRectangleNV,
}

/// Cull mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CullMode {
    /// No culling
    None,
    /// Cull front faces
    Front,
    /// Cull back faces
    #[default]
    Back,
    /// Cull front and back
    FrontAndBack,
}

/// Front face
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FrontFace {
    /// Counter-clockwise
    #[default]
    CounterClockwise,
    /// Clockwise
    Clockwise,
}

/// Rasterization state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RasterizationState {
    /// Enable depth clamp
    pub depth_clamp_enable: bool,
    /// Discard rasterizer output
    pub rasterizer_discard_enable: bool,
    /// Polygon mode
    pub polygon_mode: PolygonMode,
    /// Cull mode
    pub cull_mode: CullMode,
    /// Front face
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
        }
    }
}

impl RasterizationState {
    /// Default state
    pub const DEFAULT: Self = Self {
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
    };

    /// No culling
    pub const fn no_cull() -> Self {
        Self {
            depth_clamp_enable: false,
            rasterizer_discard_enable: false,
            polygon_mode: PolygonMode::Fill,
            cull_mode: CullMode::None,
            front_face: FrontFace::CounterClockwise,
            depth_bias_enable: false,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        }
    }

    /// Wireframe mode
    pub const fn wireframe() -> Self {
        Self {
            depth_clamp_enable: false,
            rasterizer_discard_enable: false,
            polygon_mode: PolygonMode::Line,
            cull_mode: CullMode::None,
            front_face: FrontFace::CounterClockwise,
            depth_bias_enable: false,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        }
    }

    /// Shadow pass (for shadow mapping)
    pub const fn shadow() -> Self {
        Self {
            depth_clamp_enable: true,
            rasterizer_discard_enable: false,
            polygon_mode: PolygonMode::Fill,
            cull_mode: CullMode::Front,
            front_face: FrontFace::CounterClockwise,
            depth_bias_enable: true,
            depth_bias_constant_factor: 1.25,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 1.75,
            line_width: 1.0,
        }
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

    /// With polygon mode
    pub const fn with_polygon_mode(mut self, mode: PolygonMode) -> Self {
        self.polygon_mode = mode;
        self
    }

    /// With depth bias
    pub const fn with_depth_bias(mut self, constant: f32, slope: f32) -> Self {
        self.depth_bias_enable = true;
        self.depth_bias_constant_factor = constant;
        self.depth_bias_slope_factor = slope;
        self
    }

    /// With line width
    pub const fn with_line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }

    /// Depth clamp enabled
    pub const fn depth_clamp(mut self) -> Self {
        self.depth_clamp_enable = true;
        self
    }
}

/// Conservative rasterization mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ConservativeRasterizationMode {
    /// Disabled
    #[default]
    Disabled,
    /// Overestimate
    Overestimate,
    /// Underestimate
    Underestimate,
}

/// Conservative rasterization state
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ConservativeRasterizationState {
    /// Mode
    pub mode: ConservativeRasterizationMode,
    /// Extra primitive overestimation size
    pub extra_primitive_overestimation_size: f32,
}

impl ConservativeRasterizationState {
    /// Overestimate mode
    pub const fn overestimate() -> Self {
        Self {
            mode: ConservativeRasterizationMode::Overestimate,
            extra_primitive_overestimation_size: 0.0,
        }
    }

    /// Underestimate mode
    pub const fn underestimate() -> Self {
        Self {
            mode: ConservativeRasterizationMode::Underestimate,
            extra_primitive_overestimation_size: 0.0,
        }
    }
}

/// Line rasterization mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LineRasterizationMode {
    /// Default
    #[default]
    Default,
    /// Rectangular
    Rectangular,
    /// Bresenham
    Bresenham,
    /// Rectangular smooth
    RectangularSmooth,
}

/// Line rasterization state
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct LineRasterizationState {
    /// Mode
    pub mode: LineRasterizationMode,
    /// Stippled line enable
    pub stippled_line_enable: bool,
    /// Line stipple factor
    pub line_stipple_factor: u32,
    /// Line stipple pattern
    pub line_stipple_pattern: u16,
}

impl LineRasterizationState {
    /// Bresenham lines
    pub const fn bresenham() -> Self {
        Self {
            mode: LineRasterizationMode::Bresenham,
            stippled_line_enable: false,
            line_stipple_factor: 1,
            line_stipple_pattern: 0xFFFF,
        }
    }

    /// Smooth lines
    pub const fn smooth() -> Self {
        Self {
            mode: LineRasterizationMode::RectangularSmooth,
            stippled_line_enable: false,
            line_stipple_factor: 1,
            line_stipple_pattern: 0xFFFF,
        }
    }

    /// Dashed lines
    pub const fn dashed(factor: u32, pattern: u16) -> Self {
        Self {
            mode: LineRasterizationMode::Default,
            stippled_line_enable: true,
            line_stipple_factor: factor,
            line_stipple_pattern: pattern,
        }
    }
}

/// Depth state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DepthState {
    /// Depth test enable
    pub test_enable: bool,
    /// Depth write enable
    pub write_enable: bool,
    /// Compare operation
    pub compare_op: DepthCompareOp,
    /// Depth bounds test enable
    pub bounds_test_enable: bool,
    /// Min depth bounds
    pub min_depth_bounds: f32,
    /// Max depth bounds
    pub max_depth_bounds: f32,
}

impl Default for DepthState {
    fn default() -> Self {
        Self {
            test_enable: true,
            write_enable: true,
            compare_op: DepthCompareOp::Less,
            bounds_test_enable: false,
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
        }
    }
}

impl DepthState {
    /// Default depth state
    pub const DEFAULT: Self = Self {
        test_enable: true,
        write_enable: true,
        compare_op: DepthCompareOp::Less,
        bounds_test_enable: false,
        min_depth_bounds: 0.0,
        max_depth_bounds: 1.0,
    };

    /// No depth testing
    pub const NONE: Self = Self {
        test_enable: false,
        write_enable: false,
        compare_op: DepthCompareOp::Always,
        bounds_test_enable: false,
        min_depth_bounds: 0.0,
        max_depth_bounds: 1.0,
    };

    /// Read only (test but no write)
    pub const READ_ONLY: Self = Self {
        test_enable: true,
        write_enable: false,
        compare_op: DepthCompareOp::Less,
        bounds_test_enable: false,
        min_depth_bounds: 0.0,
        max_depth_bounds: 1.0,
    };

    /// Reversed Z (greater is closer)
    pub const REVERSED: Self = Self {
        test_enable: true,
        write_enable: true,
        compare_op: DepthCompareOp::Greater,
        bounds_test_enable: false,
        min_depth_bounds: 0.0,
        max_depth_bounds: 1.0,
    };

    /// Reversed Z read only
    pub const REVERSED_READ_ONLY: Self = Self {
        test_enable: true,
        write_enable: false,
        compare_op: DepthCompareOp::Greater,
        bounds_test_enable: false,
        min_depth_bounds: 0.0,
        max_depth_bounds: 1.0,
    };

    /// Equal test (for decals)
    pub const EQUAL: Self = Self {
        test_enable: true,
        write_enable: false,
        compare_op: DepthCompareOp::Equal,
        bounds_test_enable: false,
        min_depth_bounds: 0.0,
        max_depth_bounds: 1.0,
    };

    /// With compare op
    pub const fn with_compare_op(mut self, op: DepthCompareOp) -> Self {
        self.compare_op = op;
        self
    }

    /// Without write
    pub const fn read_only(mut self) -> Self {
        self.write_enable = false;
        self
    }

    /// Without test
    pub const fn disabled(mut self) -> Self {
        self.test_enable = false;
        self.write_enable = false;
        self
    }
}

/// Depth compare operation
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DepthCompareOp {
    /// Never pass
    Never,
    /// Less than
    #[default]
    Less,
    /// Equal
    Equal,
    /// Less or equal
    LessOrEqual,
    /// Greater
    Greater,
    /// Not equal
    NotEqual,
    /// Greater or equal
    GreaterOrEqual,
    /// Always pass
    Always,
}

/// Stencil state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct StencilState {
    /// Enable stencil test
    pub test_enable: bool,
    /// Front face ops
    pub front: StencilFaceState,
    /// Back face ops
    pub back: StencilFaceState,
}

impl Default for StencilState {
    fn default() -> Self {
        Self {
            test_enable: false,
            front: StencilFaceState::default(),
            back: StencilFaceState::default(),
        }
    }
}

impl StencilState {
    /// Disabled
    pub const DISABLED: Self = Self {
        test_enable: false,
        front: StencilFaceState::KEEP,
        back: StencilFaceState::KEEP,
    };

    /// Enable with same ops for front and back
    pub const fn enabled(ops: StencilFaceState) -> Self {
        Self {
            test_enable: true,
            front: ops,
            back: ops,
        }
    }
}

/// Stencil face state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct StencilFaceState {
    /// Fail operation
    pub fail_op: StencilOp,
    /// Pass operation
    pub pass_op: StencilOp,
    /// Depth fail operation
    pub depth_fail_op: StencilOp,
    /// Compare operation
    pub compare_op: StencilCompareOp,
    /// Compare mask
    pub compare_mask: u32,
    /// Write mask
    pub write_mask: u32,
    /// Reference value
    pub reference: u32,
}

impl Default for StencilFaceState {
    fn default() -> Self {
        Self {
            fail_op: StencilOp::Keep,
            pass_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
            compare_op: StencilCompareOp::Always,
            compare_mask: 0xFF,
            write_mask: 0xFF,
            reference: 0,
        }
    }
}

impl StencilFaceState {
    /// Keep all
    pub const KEEP: Self = Self {
        fail_op: StencilOp::Keep,
        pass_op: StencilOp::Keep,
        depth_fail_op: StencilOp::Keep,
        compare_op: StencilCompareOp::Always,
        compare_mask: 0xFF,
        write_mask: 0xFF,
        reference: 0,
    };

    /// Replace on pass
    pub const fn replace(reference: u32) -> Self {
        Self {
            fail_op: StencilOp::Keep,
            pass_op: StencilOp::Replace,
            depth_fail_op: StencilOp::Keep,
            compare_op: StencilCompareOp::Always,
            compare_mask: 0xFF,
            write_mask: 0xFF,
            reference,
        }
    }

    /// Increment on pass
    pub const fn increment() -> Self {
        Self {
            fail_op: StencilOp::Keep,
            pass_op: StencilOp::IncrementClamp,
            depth_fail_op: StencilOp::Keep,
            compare_op: StencilCompareOp::Always,
            compare_mask: 0xFF,
            write_mask: 0xFF,
            reference: 0,
        }
    }

    /// Test equal
    pub const fn test_equal(reference: u32) -> Self {
        Self {
            fail_op: StencilOp::Keep,
            pass_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
            compare_op: StencilCompareOp::Equal,
            compare_mask: 0xFF,
            write_mask: 0,
            reference,
        }
    }
}

/// Stencil operation
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum StencilOp {
    /// Keep
    #[default]
    Keep,
    /// Zero
    Zero,
    /// Replace
    Replace,
    /// Increment and clamp
    IncrementClamp,
    /// Decrement and clamp
    DecrementClamp,
    /// Invert
    Invert,
    /// Increment and wrap
    IncrementWrap,
    /// Decrement and wrap
    DecrementWrap,
}

/// Stencil compare operation
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum StencilCompareOp {
    /// Never
    Never,
    /// Less
    Less,
    /// Equal
    Equal,
    /// Less or equal
    LessOrEqual,
    /// Greater
    Greater,
    /// Not equal
    NotEqual,
    /// Greater or equal
    GreaterOrEqual,
    /// Always
    #[default]
    Always,
}

/// Multisample state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MultisampleState {
    /// Rasterization samples
    pub rasterization_samples: SampleCount,
    /// Sample shading enable
    pub sample_shading_enable: bool,
    /// Min sample shading
    pub min_sample_shading: f32,
    /// Sample mask
    pub sample_mask: u32,
    /// Alpha to coverage enable
    pub alpha_to_coverage_enable: bool,
    /// Alpha to one enable
    pub alpha_to_one_enable: bool,
}

impl Default for MultisampleState {
    fn default() -> Self {
        Self {
            rasterization_samples: SampleCount::S1,
            sample_shading_enable: false,
            min_sample_shading: 1.0,
            sample_mask: 0xFFFFFFFF,
            alpha_to_coverage_enable: false,
            alpha_to_one_enable: false,
        }
    }
}

impl MultisampleState {
    /// No multisampling
    pub const NONE: Self = Self {
        rasterization_samples: SampleCount::S1,
        sample_shading_enable: false,
        min_sample_shading: 1.0,
        sample_mask: 0xFFFFFFFF,
        alpha_to_coverage_enable: false,
        alpha_to_one_enable: false,
    };

    /// 4x MSAA
    pub const MSAA_4X: Self = Self {
        rasterization_samples: SampleCount::S4,
        sample_shading_enable: false,
        min_sample_shading: 1.0,
        sample_mask: 0xFFFFFFFF,
        alpha_to_coverage_enable: false,
        alpha_to_one_enable: false,
    };

    /// 8x MSAA
    pub const MSAA_8X: Self = Self {
        rasterization_samples: SampleCount::S8,
        sample_shading_enable: false,
        min_sample_shading: 1.0,
        sample_mask: 0xFFFFFFFF,
        alpha_to_coverage_enable: false,
        alpha_to_one_enable: false,
    };

    /// With sample count
    pub const fn with_samples(mut self, count: SampleCount) -> Self {
        self.rasterization_samples = count;
        self
    }

    /// With sample shading
    pub const fn with_sample_shading(mut self, min_sample_shading: f32) -> Self {
        self.sample_shading_enable = true;
        self.min_sample_shading = min_sample_shading;
        self
    }

    /// With alpha to coverage
    pub const fn with_alpha_to_coverage(mut self) -> Self {
        self.alpha_to_coverage_enable = true;
        self
    }
}

/// Sample count
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SampleCount {
    /// 1 sample
    #[default]
    S1 = 1,
    /// 2 samples
    S2 = 2,
    /// 4 samples
    S4 = 4,
    /// 8 samples
    S8 = 8,
    /// 16 samples
    S16 = 16,
    /// 32 samples
    S32 = 32,
    /// 64 samples
    S64 = 64,
}

impl SampleCount {
    /// Gets sample count as u32
    pub const fn count(&self) -> u32 {
        *self as u32
    }
}

/// Input assembly state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct InputAssemblyState {
    /// Topology
    pub topology: PrimitiveTopology,
    /// Primitive restart enable
    pub primitive_restart_enable: bool,
}

impl Default for InputAssemblyState {
    fn default() -> Self {
        Self {
            topology: PrimitiveTopology::TriangleList,
            primitive_restart_enable: false,
        }
    }
}

impl InputAssemblyState {
    /// Triangle list
    pub const TRIANGLES: Self = Self {
        topology: PrimitiveTopology::TriangleList,
        primitive_restart_enable: false,
    };

    /// Triangle strip
    pub const TRIANGLE_STRIP: Self = Self {
        topology: PrimitiveTopology::TriangleStrip,
        primitive_restart_enable: true,
    };

    /// Line list
    pub const LINES: Self = Self {
        topology: PrimitiveTopology::LineList,
        primitive_restart_enable: false,
    };

    /// Point list
    pub const POINTS: Self = Self {
        topology: PrimitiveTopology::PointList,
        primitive_restart_enable: false,
    };

    /// With topology
    pub const fn with_topology(mut self, topology: PrimitiveTopology) -> Self {
        self.topology = topology;
        self
    }
}

/// Primitive topology
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PrimitiveTopology {
    /// Point list
    PointList,
    /// Line list
    LineList,
    /// Line strip
    LineStrip,
    /// Triangle list
    #[default]
    TriangleList,
    /// Triangle strip
    TriangleStrip,
    /// Triangle fan
    TriangleFan,
    /// Line list with adjacency
    LineListWithAdjacency,
    /// Line strip with adjacency
    LineStripWithAdjacency,
    /// Triangle list with adjacency
    TriangleListWithAdjacency,
    /// Triangle strip with adjacency
    TriangleStripWithAdjacency,
    /// Patch list (tessellation)
    PatchList,
}

impl PrimitiveTopology {
    /// Vertices per primitive
    pub const fn vertices_per_primitive(&self) -> u32 {
        match self {
            Self::PointList => 1,
            Self::LineList | Self::LineStrip => 2,
            Self::TriangleList | Self::TriangleStrip | Self::TriangleFan => 3,
            Self::LineListWithAdjacency | Self::LineStripWithAdjacency => 4,
            Self::TriangleListWithAdjacency | Self::TriangleStripWithAdjacency => 6,
            Self::PatchList => 0, // Variable
        }
    }

    /// Is list topology
    pub const fn is_list(&self) -> bool {
        matches!(
            self,
            Self::PointList
                | Self::LineList
                | Self::TriangleList
                | Self::LineListWithAdjacency
                | Self::TriangleListWithAdjacency
        )
    }

    /// Is strip topology
    pub const fn is_strip(&self) -> bool {
        matches!(
            self,
            Self::LineStrip
                | Self::TriangleStrip
                | Self::LineStripWithAdjacency
                | Self::TriangleStripWithAdjacency
        )
    }

    /// Has adjacency data
    pub const fn has_adjacency(&self) -> bool {
        matches!(
            self,
            Self::LineListWithAdjacency
                | Self::LineStripWithAdjacency
                | Self::TriangleListWithAdjacency
                | Self::TriangleStripWithAdjacency
        )
    }
}
