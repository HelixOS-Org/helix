//! Graphics Pass Types for Lumina
//!
//! This module provides graphics/render pass configuration and state types
//! for executing rasterization workloads on the GPU.

// ============================================================================
// Graphics Pass Handle
// ============================================================================

/// Graphics pass handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GraphicsPassHandle(pub u64);

impl GraphicsPassHandle {
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

impl Default for GraphicsPassHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Graphics Pass Configuration
// ============================================================================

/// Graphics pass configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct GraphicsPassConfig {
    /// Debug label
    pub label: Option<&'static str>,
    /// Flags
    pub flags: GraphicsPassFlags,
    /// Render area
    pub render_area: RenderArea,
    /// Occlusion query
    pub occlusion_query: Option<OcclusionQueryConfig>,
    /// Timestamp writes
    pub timestamp_begin: Option<TimestampWriteConfig>,
    pub timestamp_end: Option<TimestampWriteConfig>,
    /// Multiview configuration
    pub multiview: Option<MultiviewConfig>,
}

impl GraphicsPassConfig {
    /// Creates new graphics pass config
    #[inline]
    pub const fn new() -> Self {
        Self {
            label: None,
            flags: GraphicsPassFlags::NONE,
            render_area: RenderArea::FULL,
            occlusion_query: None,
            timestamp_begin: None,
            timestamp_end: None,
            multiview: None,
        }
    }

    /// With label
    #[inline]
    pub const fn with_label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: GraphicsPassFlags) -> Self {
        self.flags = flags;
        self
    }

    /// With render area
    #[inline]
    pub const fn with_render_area(mut self, area: RenderArea) -> Self {
        self.render_area = area;
        self
    }

    /// With occlusion query
    #[inline]
    pub const fn with_occlusion_query(mut self, query: OcclusionQueryConfig) -> Self {
        self.occlusion_query = Some(query);
        self
    }

    /// With multiview
    #[inline]
    pub const fn with_multiview(mut self, config: MultiviewConfig) -> Self {
        self.multiview = Some(config);
        self
    }
}

impl Default for GraphicsPassConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Graphics pass flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct GraphicsPassFlags(pub u32);

impl GraphicsPassFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Suspending pass
    pub const SUSPENDING: Self = Self(1 << 0);
    /// Resuming pass
    pub const RESUMING: Self = Self(1 << 1);
    /// Secondary command buffers allowed
    pub const SECONDARY_COMMAND_BUFFERS: Self = Self(1 << 2);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Render Area
// ============================================================================

/// Render area configuration
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct RenderArea {
    /// X offset
    pub x: i32,
    /// Y offset
    pub y: i32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

impl RenderArea {
    /// Full framebuffer (0, 0, max, max)
    pub const FULL: Self = Self {
        x: 0,
        y: 0,
        width: u32::MAX,
        height: u32::MAX,
    };

    /// Creates new render area
    #[inline]
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// From size
    #[inline]
    pub const fn from_size(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    /// Common resolutions
    pub const HD_720P: Self = Self::from_size(1280, 720);
    pub const FULL_HD: Self = Self::from_size(1920, 1080);
    pub const QHD_1440P: Self = Self::from_size(2560, 1440);
    pub const UHD_4K: Self = Self::from_size(3840, 2160);

    /// Pixel count
    #[inline]
    pub const fn pixel_count(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Aspect ratio
    #[inline]
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
}

impl Default for RenderArea {
    fn default() -> Self {
        Self::FULL
    }
}

// ============================================================================
// Viewport and Scissor
// ============================================================================

/// Viewport configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Viewport {
    /// X position
    pub x: f32,
    /// Y position
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
    /// Minimum depth
    pub min_depth: f32,
    /// Maximum depth
    pub max_depth: f32,
}

impl Viewport {
    /// Creates new viewport
    #[inline]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }

    /// From size
    #[inline]
    pub const fn from_size(width: f32, height: f32) -> Self {
        Self::new(0.0, 0.0, width, height)
    }

    /// From render area
    #[inline]
    pub const fn from_render_area(area: RenderArea) -> Self {
        Self {
            x: area.x as f32,
            y: area.y as f32,
            width: area.width as f32,
            height: area.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }

    /// Flipped viewport (for Vulkan-style Y-up)
    #[inline]
    pub const fn flipped(mut self) -> Self {
        self.y = self.y + self.height;
        self.height = -self.height;
        self
    }

    /// With depth range
    #[inline]
    pub const fn with_depth(mut self, min: f32, max: f32) -> Self {
        self.min_depth = min;
        self.max_depth = max;
        self
    }

    /// Reverse-Z depth range
    #[inline]
    pub const fn reverse_z(mut self) -> Self {
        self.min_depth = 1.0;
        self.max_depth = 0.0;
        self
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self::from_size(1920.0, 1080.0)
    }
}

/// Scissor rectangle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ScissorRect {
    /// X offset
    pub x: i32,
    /// Y offset
    pub y: i32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

impl ScissorRect {
    /// Full scissor
    pub const FULL: Self = Self {
        x: 0,
        y: 0,
        width: i32::MAX as u32,
        height: i32::MAX as u32,
    };

    /// Creates new scissor
    #[inline]
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// From size
    #[inline]
    pub const fn from_size(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    /// From render area
    #[inline]
    pub const fn from_render_area(area: RenderArea) -> Self {
        Self {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height,
        }
    }
}

impl Default for ScissorRect {
    fn default() -> Self {
        Self::FULL
    }
}

// ============================================================================
// Rasterization State
// ============================================================================

/// Rasterization state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RasterizationState {
    /// Polygon mode
    pub polygon_mode: PolygonMode,
    /// Cull mode
    pub cull_mode: CullMode,
    /// Front face
    pub front_face: FrontFace,
    /// Depth bias enable
    pub depth_bias_enable: bool,
    /// Depth bias constant factor
    pub depth_bias_constant: f32,
    /// Depth bias clamp
    pub depth_bias_clamp: f32,
    /// Depth bias slope factor
    pub depth_bias_slope: f32,
    /// Line width
    pub line_width: f32,
    /// Depth clamp enable
    pub depth_clamp_enable: bool,
    /// Rasterizer discard enable
    pub rasterizer_discard: bool,
    /// Conservative rasterization mode
    pub conservative_mode: ConservativeRasterizationMode,
}

impl RasterizationState {
    /// Default state
    pub const DEFAULT: Self = Self {
        polygon_mode: PolygonMode::Fill,
        cull_mode: CullMode::Back,
        front_face: FrontFace::CounterClockwise,
        depth_bias_enable: false,
        depth_bias_constant: 0.0,
        depth_bias_clamp: 0.0,
        depth_bias_slope: 0.0,
        line_width: 1.0,
        depth_clamp_enable: false,
        rasterizer_discard: false,
        conservative_mode: ConservativeRasterizationMode::Disabled,
    };

    /// Wireframe mode
    pub const WIREFRAME: Self = Self {
        polygon_mode: PolygonMode::Line,
        cull_mode: CullMode::None,
        front_face: FrontFace::CounterClockwise,
        depth_bias_enable: false,
        depth_bias_constant: 0.0,
        depth_bias_clamp: 0.0,
        depth_bias_slope: 0.0,
        line_width: 1.0,
        depth_clamp_enable: false,
        rasterizer_discard: false,
        conservative_mode: ConservativeRasterizationMode::Disabled,
    };

    /// Shadow map state (with depth bias)
    pub const SHADOW_MAP: Self = Self {
        polygon_mode: PolygonMode::Fill,
        cull_mode: CullMode::Front,
        front_face: FrontFace::CounterClockwise,
        depth_bias_enable: true,
        depth_bias_constant: 4.0,
        depth_bias_clamp: 0.0,
        depth_bias_slope: 1.5,
        line_width: 1.0,
        depth_clamp_enable: true,
        rasterizer_discard: false,
        conservative_mode: ConservativeRasterizationMode::Disabled,
    };

    /// Double-sided (no culling)
    pub const DOUBLE_SIDED: Self = Self {
        polygon_mode: PolygonMode::Fill,
        cull_mode: CullMode::None,
        front_face: FrontFace::CounterClockwise,
        depth_bias_enable: false,
        depth_bias_constant: 0.0,
        depth_bias_clamp: 0.0,
        depth_bias_slope: 0.0,
        line_width: 1.0,
        depth_clamp_enable: false,
        rasterizer_discard: false,
        conservative_mode: ConservativeRasterizationMode::Disabled,
    };

    /// With cull mode
    #[inline]
    pub const fn with_cull_mode(mut self, mode: CullMode) -> Self {
        self.cull_mode = mode;
        self
    }

    /// With polygon mode
    #[inline]
    pub const fn with_polygon_mode(mut self, mode: PolygonMode) -> Self {
        self.polygon_mode = mode;
        self
    }

    /// With depth bias
    #[inline]
    pub const fn with_depth_bias(mut self, constant: f32, slope: f32, clamp: f32) -> Self {
        self.depth_bias_enable = true;
        self.depth_bias_constant = constant;
        self.depth_bias_slope = slope;
        self.depth_bias_clamp = clamp;
        self
    }

    /// With line width
    #[inline]
    pub const fn with_line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }

    /// With conservative rasterization
    #[inline]
    pub const fn with_conservative(mut self, mode: ConservativeRasterizationMode) -> Self {
        self.conservative_mode = mode;
        self
    }
}

impl Default for RasterizationState {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Polygon mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PolygonMode {
    /// Fill polygons
    #[default]
    Fill            = 0,
    /// Draw lines
    Line            = 1,
    /// Draw points
    Point           = 2,
    /// Fill rectangles (for NV extension)
    FillRectangleNV = 3,
}

/// Cull mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum CullMode {
    /// No culling
    None         = 0,
    /// Cull front faces
    Front        = 1,
    /// Cull back faces
    #[default]
    Back         = 2,
    /// Cull all faces
    FrontAndBack = 3,
}

/// Front face winding
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum FrontFace {
    /// Counter-clockwise
    #[default]
    CounterClockwise = 0,
    /// Clockwise
    Clockwise        = 1,
}

/// Conservative rasterization mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ConservativeRasterizationMode {
    /// Disabled
    #[default]
    Disabled      = 0,
    /// Overestimate
    Overestimate  = 1,
    /// Underestimate
    Underestimate = 2,
}

// ============================================================================
// Depth-Stencil State
// ============================================================================

/// Depth-stencil state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DepthStencilState {
    /// Depth test enable
    pub depth_test_enable: bool,
    /// Depth write enable
    pub depth_write_enable: bool,
    /// Depth compare operation
    pub depth_compare_op: CompareOp,
    /// Depth bounds test enable
    pub depth_bounds_test_enable: bool,
    /// Minimum depth bounds
    pub min_depth_bounds: f32,
    /// Maximum depth bounds
    pub max_depth_bounds: f32,
    /// Stencil test enable
    pub stencil_test_enable: bool,
    /// Front stencil state
    pub stencil_front: StencilOpState,
    /// Back stencil state
    pub stencil_back: StencilOpState,
}

impl DepthStencilState {
    /// Disabled
    pub const DISABLED: Self = Self {
        depth_test_enable: false,
        depth_write_enable: false,
        depth_compare_op: CompareOp::Always,
        depth_bounds_test_enable: false,
        min_depth_bounds: 0.0,
        max_depth_bounds: 1.0,
        stencil_test_enable: false,
        stencil_front: StencilOpState::KEEP,
        stencil_back: StencilOpState::KEEP,
    };

    /// Depth test only (no write)
    pub const DEPTH_READ: Self = Self {
        depth_test_enable: true,
        depth_write_enable: false,
        depth_compare_op: CompareOp::LessOrEqual,
        depth_bounds_test_enable: false,
        min_depth_bounds: 0.0,
        max_depth_bounds: 1.0,
        stencil_test_enable: false,
        stencil_front: StencilOpState::KEEP,
        stencil_back: StencilOpState::KEEP,
    };

    /// Depth test and write
    pub const DEPTH_WRITE: Self = Self {
        depth_test_enable: true,
        depth_write_enable: true,
        depth_compare_op: CompareOp::LessOrEqual,
        depth_bounds_test_enable: false,
        min_depth_bounds: 0.0,
        max_depth_bounds: 1.0,
        stencil_test_enable: false,
        stencil_front: StencilOpState::KEEP,
        stencil_back: StencilOpState::KEEP,
    };

    /// Reverse-Z depth (greater is closer)
    pub const REVERSE_Z: Self = Self {
        depth_test_enable: true,
        depth_write_enable: true,
        depth_compare_op: CompareOp::GreaterOrEqual,
        depth_bounds_test_enable: false,
        min_depth_bounds: 0.0,
        max_depth_bounds: 1.0,
        stencil_test_enable: false,
        stencil_front: StencilOpState::KEEP,
        stencil_back: StencilOpState::KEEP,
    };

    /// With depth compare operation
    #[inline]
    pub const fn with_compare_op(mut self, op: CompareOp) -> Self {
        self.depth_compare_op = op;
        self
    }

    /// With stencil
    #[inline]
    pub const fn with_stencil(mut self, front: StencilOpState, back: StencilOpState) -> Self {
        self.stencil_test_enable = true;
        self.stencil_front = front;
        self.stencil_back = back;
        self
    }

    /// With depth bounds
    #[inline]
    pub const fn with_depth_bounds(mut self, min: f32, max: f32) -> Self {
        self.depth_bounds_test_enable = true;
        self.min_depth_bounds = min;
        self.max_depth_bounds = max;
        self
    }
}

impl Default for DepthStencilState {
    fn default() -> Self {
        Self::DEPTH_WRITE
    }
}

/// Compare operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum CompareOp {
    /// Never pass
    Never          = 0,
    /// Less than
    Less           = 1,
    /// Equal
    Equal          = 2,
    /// Less than or equal
    #[default]
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

/// Stencil operation state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct StencilOpState {
    /// Fail operation
    pub fail_op: StencilOp,
    /// Pass operation
    pub pass_op: StencilOp,
    /// Depth fail operation
    pub depth_fail_op: StencilOp,
    /// Compare operation
    pub compare_op: CompareOp,
    /// Compare mask
    pub compare_mask: u32,
    /// Write mask
    pub write_mask: u32,
    /// Reference value
    pub reference: u32,
}

impl StencilOpState {
    /// Keep all (no stencil operations)
    pub const KEEP: Self = Self {
        fail_op: StencilOp::Keep,
        pass_op: StencilOp::Keep,
        depth_fail_op: StencilOp::Keep,
        compare_op: CompareOp::Always,
        compare_mask: 0xFF,
        write_mask: 0xFF,
        reference: 0,
    };

    /// Replace on pass
    pub const REPLACE_ON_PASS: Self = Self {
        fail_op: StencilOp::Keep,
        pass_op: StencilOp::Replace,
        depth_fail_op: StencilOp::Keep,
        compare_op: CompareOp::Always,
        compare_mask: 0xFF,
        write_mask: 0xFF,
        reference: 1,
    };

    /// Increment on pass
    pub const INCREMENT_ON_PASS: Self = Self {
        fail_op: StencilOp::Keep,
        pass_op: StencilOp::IncrementAndClamp,
        depth_fail_op: StencilOp::Keep,
        compare_op: CompareOp::Always,
        compare_mask: 0xFF,
        write_mask: 0xFF,
        reference: 0,
    };
}

impl Default for StencilOpState {
    fn default() -> Self {
        Self::KEEP
    }
}

/// Stencil operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum StencilOp {
    /// Keep current value
    #[default]
    Keep              = 0,
    /// Set to zero
    Zero              = 1,
    /// Set to reference
    Replace           = 2,
    /// Increment and clamp
    IncrementAndClamp = 3,
    /// Decrement and clamp
    DecrementAndClamp = 4,
    /// Bitwise invert
    Invert            = 5,
    /// Increment and wrap
    IncrementAndWrap  = 6,
    /// Decrement and wrap
    DecrementAndWrap  = 7,
}

// ============================================================================
// Multisample State
// ============================================================================

/// Multisample state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MultisampleState {
    /// Sample count
    pub sample_count: SampleCount,
    /// Sample shading enable
    pub sample_shading_enable: bool,
    /// Minimum sample shading
    pub min_sample_shading: f32,
    /// Sample mask
    pub sample_mask: u64,
    /// Alpha to coverage enable
    pub alpha_to_coverage_enable: bool,
    /// Alpha to one enable
    pub alpha_to_one_enable: bool,
}

impl MultisampleState {
    /// No multisampling
    pub const DISABLED: Self = Self {
        sample_count: SampleCount::S1,
        sample_shading_enable: false,
        min_sample_shading: 0.0,
        sample_mask: u64::MAX,
        alpha_to_coverage_enable: false,
        alpha_to_one_enable: false,
    };

    /// MSAA 2x
    pub const MSAA_2X: Self = Self {
        sample_count: SampleCount::S2,
        sample_shading_enable: false,
        min_sample_shading: 0.0,
        sample_mask: u64::MAX,
        alpha_to_coverage_enable: false,
        alpha_to_one_enable: false,
    };

    /// MSAA 4x
    pub const MSAA_4X: Self = Self {
        sample_count: SampleCount::S4,
        sample_shading_enable: false,
        min_sample_shading: 0.0,
        sample_mask: u64::MAX,
        alpha_to_coverage_enable: false,
        alpha_to_one_enable: false,
    };

    /// MSAA 8x
    pub const MSAA_8X: Self = Self {
        sample_count: SampleCount::S8,
        sample_shading_enable: false,
        min_sample_shading: 0.0,
        sample_mask: u64::MAX,
        alpha_to_coverage_enable: false,
        alpha_to_one_enable: false,
    };

    /// Sample shading (supersampling)
    pub const fn sample_shading(sample_count: SampleCount) -> Self {
        Self {
            sample_count,
            sample_shading_enable: true,
            min_sample_shading: 1.0,
            sample_mask: u64::MAX,
            alpha_to_coverage_enable: false,
            alpha_to_one_enable: false,
        }
    }

    /// With alpha to coverage
    #[inline]
    pub const fn with_alpha_to_coverage(mut self) -> Self {
        self.alpha_to_coverage_enable = true;
        self
    }

    /// With sample mask
    #[inline]
    pub const fn with_sample_mask(mut self, mask: u64) -> Self {
        self.sample_mask = mask;
        self
    }
}

impl Default for MultisampleState {
    fn default() -> Self {
        Self::DISABLED
    }
}

/// Sample count
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
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

impl SampleCount {
    /// Value
    #[inline]
    pub const fn value(&self) -> u32 {
        *self as u32
    }
}

// ============================================================================
// Input Assembly State
// ============================================================================

/// Input assembly state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct InputAssemblyState {
    /// Primitive topology
    pub topology: PrimitiveTopology,
    /// Primitive restart enable
    pub primitive_restart_enable: bool,
}

impl InputAssemblyState {
    /// Triangle list
    pub const TRIANGLE_LIST: Self = Self {
        topology: PrimitiveTopology::TriangleList,
        primitive_restart_enable: false,
    };

    /// Triangle strip
    pub const TRIANGLE_STRIP: Self = Self {
        topology: PrimitiveTopology::TriangleStrip,
        primitive_restart_enable: true,
    };

    /// Line list
    pub const LINE_LIST: Self = Self {
        topology: PrimitiveTopology::LineList,
        primitive_restart_enable: false,
    };

    /// Line strip
    pub const LINE_STRIP: Self = Self {
        topology: PrimitiveTopology::LineStrip,
        primitive_restart_enable: true,
    };

    /// Point list
    pub const POINT_LIST: Self = Self {
        topology: PrimitiveTopology::PointList,
        primitive_restart_enable: false,
    };

    /// Patch list (for tessellation)
    pub const PATCH_LIST: Self = Self {
        topology: PrimitiveTopology::PatchList,
        primitive_restart_enable: false,
    };
}

impl Default for InputAssemblyState {
    fn default() -> Self {
        Self::TRIANGLE_LIST
    }
}

/// Primitive topology
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
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

impl PrimitiveTopology {
    /// Is list topology
    #[inline]
    pub const fn is_list(&self) -> bool {
        matches!(
            self,
            Self::PointList
                | Self::LineList
                | Self::TriangleList
                | Self::LineListWithAdjacency
                | Self::TriangleListWithAdjacency
                | Self::PatchList
        )
    }

    /// Has adjacency
    #[inline]
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

// ============================================================================
// Vertex Input State
// ============================================================================

/// Vertex input state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VertexInputState {
    /// Vertex bindings
    pub bindings: [VertexBinding; 16],
    /// Number of bindings
    pub binding_count: u32,
    /// Vertex attributes
    pub attributes: [VertexAttribute; 32],
    /// Number of attributes
    pub attribute_count: u32,
}

impl VertexInputState {
    /// Empty vertex input
    pub const EMPTY: Self = Self {
        bindings: [VertexBinding::EMPTY; 16],
        binding_count: 0,
        attributes: [VertexAttribute::EMPTY; 32],
        attribute_count: 0,
    };

    /// Add binding
    pub fn add_binding(&mut self, binding: VertexBinding) {
        if self.binding_count < 16 {
            self.bindings[self.binding_count as usize] = binding;
            self.binding_count += 1;
        }
    }

    /// Add attribute
    pub fn add_attribute(&mut self, attribute: VertexAttribute) {
        if self.attribute_count < 32 {
            self.attributes[self.attribute_count as usize] = attribute;
            self.attribute_count += 1;
        }
    }
}

impl Default for VertexInputState {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Vertex binding
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VertexBinding {
    /// Binding index
    pub binding: u32,
    /// Stride in bytes
    pub stride: u32,
    /// Input rate
    pub input_rate: VertexInputRate,
    /// Divisor (for instancing)
    pub divisor: u32,
}

impl VertexBinding {
    /// Empty binding
    pub const EMPTY: Self = Self {
        binding: 0,
        stride: 0,
        input_rate: VertexInputRate::Vertex,
        divisor: 1,
    };

    /// Creates new binding
    #[inline]
    pub const fn new(binding: u32, stride: u32) -> Self {
        Self {
            binding,
            stride,
            input_rate: VertexInputRate::Vertex,
            divisor: 1,
        }
    }

    /// Per-instance binding
    #[inline]
    pub const fn per_instance(mut self) -> Self {
        self.input_rate = VertexInputRate::Instance;
        self
    }

    /// With divisor
    #[inline]
    pub const fn with_divisor(mut self, divisor: u32) -> Self {
        self.divisor = divisor;
        self
    }
}

/// Vertex input rate
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum VertexInputRate {
    /// Per-vertex
    #[default]
    Vertex   = 0,
    /// Per-instance
    Instance = 1,
}

/// Vertex attribute
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VertexAttribute {
    /// Location
    pub location: u32,
    /// Binding index
    pub binding: u32,
    /// Format
    pub format: VertexFormat,
    /// Offset in bytes
    pub offset: u32,
}

impl VertexAttribute {
    /// Empty attribute
    pub const EMPTY: Self = Self {
        location: 0,
        binding: 0,
        format: VertexFormat::Float32,
        offset: 0,
    };

    /// Creates new attribute
    #[inline]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum VertexFormat {
    /// Float32
    #[default]
    Float32   = 0,
    /// Float32x2
    Float32x2 = 1,
    /// Float32x3
    Float32x3 = 2,
    /// Float32x4
    Float32x4 = 3,
    /// Uint8x2
    Uint8x2   = 4,
    /// Uint8x4
    Uint8x4   = 5,
    /// Sint8x2
    Sint8x2   = 6,
    /// Sint8x4
    Sint8x4   = 7,
    /// Unorm8x2
    Unorm8x2  = 8,
    /// Unorm8x4
    Unorm8x4  = 9,
    /// Snorm8x2
    Snorm8x2  = 10,
    /// Snorm8x4
    Snorm8x4  = 11,
    /// Uint16x2
    Uint16x2  = 12,
    /// Uint16x4
    Uint16x4  = 13,
    /// Sint16x2
    Sint16x2  = 14,
    /// Sint16x4
    Sint16x4  = 15,
    /// Unorm16x2
    Unorm16x2 = 16,
    /// Unorm16x4
    Unorm16x4 = 17,
    /// Snorm16x2
    Snorm16x2 = 18,
    /// Snorm16x4
    Snorm16x4 = 19,
    /// Float16x2
    Float16x2 = 20,
    /// Float16x4
    Float16x4 = 21,
    /// Uint32
    Uint32    = 22,
    /// Uint32x2
    Uint32x2  = 23,
    /// Uint32x3
    Uint32x3  = 24,
    /// Uint32x4
    Uint32x4  = 25,
    /// Sint32
    Sint32    = 26,
    /// Sint32x2
    Sint32x2  = 27,
    /// Sint32x3
    Sint32x3  = 28,
    /// Sint32x4
    Sint32x4  = 29,
}

impl VertexFormat {
    /// Size in bytes
    #[inline]
    pub const fn size(&self) -> u32 {
        match self {
            Self::Uint8x2 | Self::Sint8x2 | Self::Unorm8x2 | Self::Snorm8x2 => 2,
            Self::Uint8x4 | Self::Sint8x4 | Self::Unorm8x4 | Self::Snorm8x4 | Self::Float32 => 4,
            Self::Uint16x2
            | Self::Sint16x2
            | Self::Unorm16x2
            | Self::Snorm16x2
            | Self::Float16x2 => 4,
            Self::Float32x2
            | Self::Uint16x4
            | Self::Sint16x4
            | Self::Unorm16x4
            | Self::Snorm16x4
            | Self::Float16x4
            | Self::Uint32
            | Self::Sint32
            | Self::Uint32x2
            | Self::Sint32x2 => 8,
            Self::Float32x3 | Self::Uint32x3 | Self::Sint32x3 => 12,
            Self::Float32x4 | Self::Uint32x4 | Self::Sint32x4 => 16,
        }
    }
}

// ============================================================================
// Multiview Configuration
// ============================================================================

/// Multiview configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MultiviewConfig {
    /// View mask
    pub view_mask: u32,
    /// Correlation mask
    pub correlation_mask: u32,
}

impl MultiviewConfig {
    /// Stereo (2 views)
    pub const STEREO: Self = Self {
        view_mask: 0b11,
        correlation_mask: 0b11,
    };

    /// Creates for n views
    #[inline]
    pub const fn views(n: u32) -> Self {
        let mask = (1u32 << n) - 1;
        Self {
            view_mask: mask,
            correlation_mask: mask,
        }
    }
}

// ============================================================================
// Occlusion Query Configuration
// ============================================================================

/// Occlusion query configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct OcclusionQueryConfig {
    /// Query pool handle
    pub query_pool: u64,
    /// Query index
    pub query_index: u32,
    /// Precise occlusion query
    pub precise: bool,
}

impl OcclusionQueryConfig {
    /// Creates new config
    #[inline]
    pub const fn new(query_pool: u64, query_index: u32, precise: bool) -> Self {
        Self {
            query_pool,
            query_index,
            precise,
        }
    }
}

/// Timestamp write configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TimestampWriteConfig {
    /// Query pool handle
    pub query_pool: u64,
    /// Query index
    pub query_index: u32,
}

impl TimestampWriteConfig {
    /// Creates new config
    #[inline]
    pub const fn new(query_pool: u64, query_index: u32) -> Self {
        Self {
            query_pool,
            query_index,
        }
    }
}
