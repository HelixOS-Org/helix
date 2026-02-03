//! Rasterization State Types for Lumina
//!
//! This module provides comprehensive rasterization configuration,
//! polygon modes, culling, and line/point rendering.

// ============================================================================
// Pipeline Rasterization State
// ============================================================================

/// Rasterization state create info
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct RasterizationStateCreateInfo {
    /// Flags
    pub flags: RasterizationStateCreateFlags,
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

impl RasterizationStateCreateInfo {
    /// Creates new info (default)
    #[inline]
    pub const fn new() -> Self {
        Self {
            flags: RasterizationStateCreateFlags::NONE,
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

    /// Default state
    pub const DEFAULT: Self = Self::new();

    /// No culling
    #[inline]
    pub const fn no_cull() -> Self {
        Self {
            flags: RasterizationStateCreateFlags::NONE,
            depth_clamp_enable: false,
            rasterizer_discard_enable: false,
            polygon_mode: PolygonMode::Fill,
            cull_mode: CullModeFlags::NONE,
            front_face: FrontFace::CounterClockwise,
            depth_bias_enable: false,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        }
    }

    /// Front face culling
    #[inline]
    pub const fn cull_front() -> Self {
        Self {
            flags: RasterizationStateCreateFlags::NONE,
            depth_clamp_enable: false,
            rasterizer_discard_enable: false,
            polygon_mode: PolygonMode::Fill,
            cull_mode: CullModeFlags::FRONT,
            front_face: FrontFace::CounterClockwise,
            depth_bias_enable: false,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        }
    }

    /// Back face culling (default)
    #[inline]
    pub const fn cull_back() -> Self {
        Self::new()
    }

    /// Wireframe mode
    #[inline]
    pub const fn wireframe() -> Self {
        Self {
            flags: RasterizationStateCreateFlags::NONE,
            depth_clamp_enable: false,
            rasterizer_discard_enable: false,
            polygon_mode: PolygonMode::Line,
            cull_mode: CullModeFlags::NONE,
            front_face: FrontFace::CounterClockwise,
            depth_bias_enable: false,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        }
    }

    /// Point mode
    #[inline]
    pub const fn point() -> Self {
        Self {
            flags: RasterizationStateCreateFlags::NONE,
            depth_clamp_enable: false,
            rasterizer_discard_enable: false,
            polygon_mode: PolygonMode::Point,
            cull_mode: CullModeFlags::NONE,
            front_face: FrontFace::CounterClockwise,
            depth_bias_enable: false,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        }
    }

    /// Shadow map rendering
    #[inline]
    pub const fn shadow_map() -> Self {
        Self {
            flags: RasterizationStateCreateFlags::NONE,
            depth_clamp_enable: true,
            rasterizer_discard_enable: false,
            polygon_mode: PolygonMode::Fill,
            cull_mode: CullModeFlags::FRONT,
            front_face: FrontFace::CounterClockwise,
            depth_bias_enable: true,
            depth_bias_constant_factor: 1.25,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 1.75,
            line_width: 1.0,
        }
    }

    /// With polygon mode
    #[inline]
    pub const fn with_polygon_mode(mut self, mode: PolygonMode) -> Self {
        self.polygon_mode = mode;
        self
    }

    /// With cull mode
    #[inline]
    pub const fn with_cull_mode(mut self, mode: CullModeFlags) -> Self {
        self.cull_mode = mode;
        self
    }

    /// With front face
    #[inline]
    pub const fn with_front_face(mut self, face: FrontFace) -> Self {
        self.front_face = face;
        self
    }

    /// Enable depth clamp
    #[inline]
    pub const fn enable_depth_clamp(mut self) -> Self {
        self.depth_clamp_enable = true;
        self
    }

    /// Enable rasterizer discard
    #[inline]
    pub const fn enable_rasterizer_discard(mut self) -> Self {
        self.rasterizer_discard_enable = true;
        self
    }

    /// With depth bias
    #[inline]
    pub const fn with_depth_bias(
        mut self,
        constant_factor: f32,
        clamp: f32,
        slope_factor: f32,
    ) -> Self {
        self.depth_bias_enable = true;
        self.depth_bias_constant_factor = constant_factor;
        self.depth_bias_clamp = clamp;
        self.depth_bias_slope_factor = slope_factor;
        self
    }

    /// With line width
    #[inline]
    pub const fn with_line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: RasterizationStateCreateFlags) -> Self {
        self.flags = flags;
        self
    }
}

impl Default for RasterizationStateCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Rasterization state create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct RasterizationStateCreateFlags(pub u32);

impl RasterizationStateCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);

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
// Polygon Mode
// ============================================================================

/// Polygon mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PolygonMode {
    /// Fill polygons
    #[default]
    Fill            = 0,
    /// Draw edges only (wireframe)
    Line            = 1,
    /// Draw vertices only
    Point           = 2,
    /// Fill with rectangles (NV)
    FillRectangleNv = 1000153000,
}

impl PolygonMode {
    /// Is filled mode
    #[inline]
    pub const fn is_filled(&self) -> bool {
        matches!(self, Self::Fill | Self::FillRectangleNv)
    }

    /// Is wireframe mode
    #[inline]
    pub const fn is_wireframe(&self) -> bool {
        matches!(self, Self::Line)
    }

    /// Is point mode
    #[inline]
    pub const fn is_point(&self) -> bool {
        matches!(self, Self::Point)
    }
}

// ============================================================================
// Cull Mode
// ============================================================================

/// Cull mode flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct CullModeFlags(pub u32);

impl CullModeFlags {
    /// No culling
    pub const NONE: Self = Self(0);
    /// Cull front faces
    pub const FRONT: Self = Self(1 << 0);
    /// Cull back faces
    pub const BACK: Self = Self(1 << 1);
    /// Cull all faces
    pub const FRONT_AND_BACK: Self = Self(Self::FRONT.0 | Self::BACK.0);

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

    /// Is front culled
    #[inline]
    pub const fn culls_front(&self) -> bool {
        self.contains(Self::FRONT)
    }

    /// Is back culled
    #[inline]
    pub const fn culls_back(&self) -> bool {
        self.contains(Self::BACK)
    }

    /// Is all culled
    #[inline]
    pub const fn culls_all(&self) -> bool {
        self.contains(Self::FRONT_AND_BACK)
    }
}

// ============================================================================
// Front Face
// ============================================================================

/// Front face winding order
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FrontFace {
    /// Counter-clockwise vertices are front-facing
    #[default]
    CounterClockwise = 0,
    /// Clockwise vertices are front-facing
    Clockwise        = 1,
}

impl FrontFace {
    /// Flip winding order
    #[inline]
    pub const fn flip(&self) -> Self {
        match self {
            Self::CounterClockwise => Self::Clockwise,
            Self::Clockwise => Self::CounterClockwise,
        }
    }
}

// ============================================================================
// Line Rasterization
// ============================================================================

/// Line rasterization mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LineRasterizationMode {
    /// Default (implementation defined)
    #[default]
    Default           = 0,
    /// Rectangular lines
    Rectangular       = 1,
    /// Bresenham lines
    Bresenham         = 2,
    /// Rectangular smooth lines
    RectangularSmooth = 3,
}

/// Line rasterization state
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct LineRasterizationState {
    /// Line rasterization mode
    pub line_rasterization_mode: LineRasterizationMode,
    /// Stippled line enable
    pub stippled_line_enable: bool,
    /// Line stipple factor
    pub line_stipple_factor: u32,
    /// Line stipple pattern
    pub line_stipple_pattern: u16,
}

impl LineRasterizationState {
    /// Default
    pub const DEFAULT: Self = Self {
        line_rasterization_mode: LineRasterizationMode::Default,
        stippled_line_enable: false,
        line_stipple_factor: 1,
        line_stipple_pattern: 0xFFFF,
    };

    /// Solid line
    pub const SOLID: Self = Self::DEFAULT;

    /// Dashed line
    pub const DASHED: Self = Self {
        line_rasterization_mode: LineRasterizationMode::Default,
        stippled_line_enable: true,
        line_stipple_factor: 1,
        line_stipple_pattern: 0xFF00,
    };

    /// Dotted line
    pub const DOTTED: Self = Self {
        line_rasterization_mode: LineRasterizationMode::Default,
        stippled_line_enable: true,
        line_stipple_factor: 1,
        line_stipple_pattern: 0xAAAA,
    };

    /// Dash-dot line
    pub const DASH_DOT: Self = Self {
        line_rasterization_mode: LineRasterizationMode::Default,
        stippled_line_enable: true,
        line_stipple_factor: 1,
        line_stipple_pattern: 0xFC30,
    };

    /// Creates new state
    #[inline]
    pub const fn new() -> Self {
        Self::DEFAULT
    }

    /// With mode
    #[inline]
    pub const fn with_mode(mut self, mode: LineRasterizationMode) -> Self {
        self.line_rasterization_mode = mode;
        self
    }

    /// With stipple
    #[inline]
    pub const fn with_stipple(mut self, factor: u32, pattern: u16) -> Self {
        self.stippled_line_enable = true;
        self.line_stipple_factor = factor;
        self.line_stipple_pattern = pattern;
        self
    }
}

impl Default for LineRasterizationState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Conservative Rasterization
// ============================================================================

/// Conservative rasterization mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ConservativeRasterizationMode {
    /// Disabled
    #[default]
    Disabled      = 0,
    /// Overestimate
    Overestimate  = 1,
    /// Underestimate
    Underestimate = 2,
}

/// Conservative rasterization state
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct ConservativeRasterizationState {
    /// Conservative rasterization mode
    pub conservative_rasterization_mode: ConservativeRasterizationMode,
    /// Extra primitive overestimation size
    pub extra_primitive_overestimation_size: f32,
}

impl ConservativeRasterizationState {
    /// Disabled
    pub const DISABLED: Self = Self {
        conservative_rasterization_mode: ConservativeRasterizationMode::Disabled,
        extra_primitive_overestimation_size: 0.0,
    };

    /// Overestimate
    pub const OVERESTIMATE: Self = Self {
        conservative_rasterization_mode: ConservativeRasterizationMode::Overestimate,
        extra_primitive_overestimation_size: 0.0,
    };

    /// Underestimate
    pub const UNDERESTIMATE: Self = Self {
        conservative_rasterization_mode: ConservativeRasterizationMode::Underestimate,
        extra_primitive_overestimation_size: 0.0,
    };

    /// Creates new state
    #[inline]
    pub const fn new(mode: ConservativeRasterizationMode) -> Self {
        Self {
            conservative_rasterization_mode: mode,
            extra_primitive_overestimation_size: 0.0,
        }
    }

    /// With extra overestimation
    #[inline]
    pub const fn with_extra_overestimation(mut self, size: f32) -> Self {
        self.extra_primitive_overestimation_size = size;
        self
    }
}

impl Default for ConservativeRasterizationState {
    fn default() -> Self {
        Self::DISABLED
    }
}

// ============================================================================
// Provoking Vertex
// ============================================================================

/// Provoking vertex mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ProvokingVertexMode {
    /// First vertex
    #[default]
    FirstVertex = 0,
    /// Last vertex
    LastVertex  = 1,
}

/// Provoking vertex state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ProvokingVertexState {
    /// Provoking vertex mode
    pub provoking_vertex_mode: ProvokingVertexMode,
}

impl ProvokingVertexState {
    /// First vertex (default, Vulkan convention)
    pub const FIRST_VERTEX: Self = Self {
        provoking_vertex_mode: ProvokingVertexMode::FirstVertex,
    };

    /// Last vertex (OpenGL convention)
    pub const LAST_VERTEX: Self = Self {
        provoking_vertex_mode: ProvokingVertexMode::LastVertex,
    };

    /// Creates new state
    #[inline]
    pub const fn new(mode: ProvokingVertexMode) -> Self {
        Self {
            provoking_vertex_mode: mode,
        }
    }
}

impl Default for ProvokingVertexState {
    fn default() -> Self {
        Self::FIRST_VERTEX
    }
}

// ============================================================================
// Depth Clip
// ============================================================================

/// Depth clip state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct DepthClipState {
    /// Depth clip enable
    pub depth_clip_enable: bool,
}

impl DepthClipState {
    /// Enabled (default)
    pub const ENABLED: Self = Self {
        depth_clip_enable: true,
    };

    /// Disabled
    pub const DISABLED: Self = Self {
        depth_clip_enable: false,
    };

    /// Creates new state
    #[inline]
    pub const fn new(enable: bool) -> Self {
        Self {
            depth_clip_enable: enable,
        }
    }
}

impl Default for DepthClipState {
    fn default() -> Self {
        Self::ENABLED
    }
}

/// Depth clip negative one to one state (Vulkan depth range vs OpenGL)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct DepthClipNegativeOneToOneState {
    /// Negative one to one (OpenGL convention)
    pub negative_one_to_one: bool,
}

impl DepthClipNegativeOneToOneState {
    /// Zero to one (Vulkan convention)
    pub const ZERO_TO_ONE: Self = Self {
        negative_one_to_one: false,
    };

    /// Negative one to one (OpenGL convention)
    pub const NEGATIVE_ONE_TO_ONE: Self = Self {
        negative_one_to_one: true,
    };

    /// Creates new state
    #[inline]
    pub const fn new(negative_one_to_one: bool) -> Self {
        Self {
            negative_one_to_one,
        }
    }
}

impl Default for DepthClipNegativeOneToOneState {
    fn default() -> Self {
        Self::ZERO_TO_ONE
    }
}

// ============================================================================
// Stream Rasterization
// ============================================================================

/// Rasterization stream state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct RasterizationStreamState {
    /// Rasterization stream
    pub rasterization_stream: u32,
}

impl RasterizationStreamState {
    /// Stream 0 (default)
    pub const STREAM_0: Self = Self {
        rasterization_stream: 0,
    };

    /// Creates new state
    #[inline]
    pub const fn new(stream: u32) -> Self {
        Self {
            rasterization_stream: stream,
        }
    }
}

impl Default for RasterizationStreamState {
    fn default() -> Self {
        Self::STREAM_0
    }
}

// ============================================================================
// Sample Locations
// ============================================================================

/// Sample location
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct SampleLocation {
    /// X position (0.0 to 1.0)
    pub x: f32,
    /// Y position (0.0 to 1.0)
    pub y: f32,
}

impl SampleLocation {
    /// Center (0.5, 0.5)
    pub const CENTER: Self = Self { x: 0.5, y: 0.5 };

    /// Creates new sample location
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl Default for SampleLocation {
    fn default() -> Self {
        Self::CENTER
    }
}

/// Sample locations state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct SampleLocationsState {
    /// Sample locations enable
    pub sample_locations_enable: bool,
}

impl SampleLocationsState {
    /// Disabled
    pub const DISABLED: Self = Self {
        sample_locations_enable: false,
    };

    /// Enabled
    pub const ENABLED: Self = Self {
        sample_locations_enable: true,
    };

    /// Creates new state
    #[inline]
    pub const fn new(enable: bool) -> Self {
        Self {
            sample_locations_enable: enable,
        }
    }
}

impl Default for SampleLocationsState {
    fn default() -> Self {
        Self::DISABLED
    }
}

/// Sample locations info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct SampleLocationsInfo<'a> {
    /// Sample locations per pixel
    pub sample_locations_per_pixel: SampleCountFlags,
    /// Sample location grid size
    pub sample_location_grid_size: Extent2D,
    /// Sample locations
    pub sample_locations: &'a [SampleLocation],
}

impl<'a> SampleLocationsInfo<'a> {
    /// Creates new info
    #[inline]
    pub const fn new(
        samples_per_pixel: SampleCountFlags,
        grid_size: Extent2D,
        locations: &'a [SampleLocation],
    ) -> Self {
        Self {
            sample_locations_per_pixel: samples_per_pixel,
            sample_location_grid_size: grid_size,
            sample_locations: locations,
        }
    }
}

// ============================================================================
// Extent 2D (local copy for independence)
// ============================================================================

/// 2D extent (for sample locations)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Extent2D {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

impl Extent2D {
    /// Creates new extent
    #[inline]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Unit (1x1)
    pub const UNIT: Self = Self::new(1, 1);
}

// ============================================================================
// Sample Count Flags
// ============================================================================

/// Sample count flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SampleCountFlags(pub u32);

impl SampleCountFlags {
    /// 1 sample
    pub const COUNT_1: Self = Self(1 << 0);
    /// 2 samples
    pub const COUNT_2: Self = Self(1 << 1);
    /// 4 samples
    pub const COUNT_4: Self = Self(1 << 2);
    /// 8 samples
    pub const COUNT_8: Self = Self(1 << 3);
    /// 16 samples
    pub const COUNT_16: Self = Self(1 << 4);
    /// 32 samples
    pub const COUNT_32: Self = Self(1 << 5);
    /// 64 samples
    pub const COUNT_64: Self = Self(1 << 6);

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

    /// Sample count as number
    #[inline]
    pub const fn as_count(&self) -> u32 {
        match self.0 {
            1 => 1,
            2 => 2,
            4 => 4,
            8 => 8,
            16 => 16,
            32 => 32,
            64 => 64,
            _ => 1,
        }
    }

    /// From count
    #[inline]
    pub const fn from_count(count: u32) -> Self {
        match count {
            1 => Self::COUNT_1,
            2 => Self::COUNT_2,
            4 => Self::COUNT_4,
            8 => Self::COUNT_8,
            16 => Self::COUNT_16,
            32 => Self::COUNT_32,
            64 => Self::COUNT_64,
            _ => Self::COUNT_1,
        }
    }
}
