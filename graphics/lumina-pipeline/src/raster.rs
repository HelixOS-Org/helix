//! Rasterizer State
//!
//! This module provides rasterization configuration for the graphics pipeline.

// ============================================================================
// Cull Mode
// ============================================================================

/// Face culling mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CullMode {
    /// No culling.
    None,
    /// Cull front faces.
    Front,
    /// Cull back faces.
    #[default]
    Back,
    /// Cull both faces.
    FrontAndBack,
}

impl CullMode {
    /// Check if any culling is enabled.
    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Invert the cull mode.
    pub fn invert(&self) -> Self {
        match self {
            Self::None => Self::FrontAndBack,
            Self::Front => Self::Back,
            Self::Back => Self::Front,
            Self::FrontAndBack => Self::None,
        }
    }
}

// ============================================================================
// Front Face
// ============================================================================

/// Front face winding order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FrontFace {
    /// Counter-clockwise is front.
    #[default]
    CounterClockwise,
    /// Clockwise is front.
    Clockwise,
}

impl FrontFace {
    /// Invert the winding order.
    pub fn invert(&self) -> Self {
        match self {
            Self::CounterClockwise => Self::Clockwise,
            Self::Clockwise => Self::CounterClockwise,
        }
    }
}

// ============================================================================
// Polygon Mode
// ============================================================================

/// Polygon fill mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PolygonMode {
    /// Fill polygons.
    #[default]
    Fill,
    /// Draw lines only.
    Line,
    /// Draw points only.
    Point,
}

// ============================================================================
// Depth Bias
// ============================================================================

/// Depth bias configuration.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DepthBias {
    /// Enable depth bias.
    pub enable: bool,
    /// Constant depth bias.
    pub constant_factor: f32,
    /// Clamp value.
    pub clamp: f32,
    /// Slope-scaled depth bias.
    pub slope_factor: f32,
}

impl DepthBias {
    /// Create disabled depth bias.
    pub fn disabled() -> Self {
        Self {
            enable: false,
            constant_factor: 0.0,
            clamp: 0.0,
            slope_factor: 0.0,
        }
    }

    /// Create enabled depth bias.
    pub fn enabled(constant: f32, clamp: f32, slope: f32) -> Self {
        Self {
            enable: true,
            constant_factor: constant,
            clamp,
            slope_factor: slope,
        }
    }

    /// Create shadow map depth bias.
    pub fn shadow() -> Self {
        Self::enabled(1.25, 0.0, 1.75)
    }

    /// Create decal depth bias.
    pub fn decal() -> Self {
        Self::enabled(-1.0, 0.0, -1.0)
    }
}

// ============================================================================
// Conservative Rasterization
// ============================================================================

/// Conservative rasterization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ConservativeRasterMode {
    /// Disabled.
    #[default]
    Disabled,
    /// Overestimate (include all partially covered pixels).
    Overestimate,
    /// Underestimate (only fully covered pixels).
    Underestimate,
}

// ============================================================================
// Line Rasterization
// ============================================================================

/// Line rasterization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LineRasterizationMode {
    /// Default rasterization.
    #[default]
    Default,
    /// Rectangular lines.
    Rectangular,
    /// Bresenham lines.
    Bresenham,
    /// Smooth lines with AA.
    RectangularSmooth,
}

// ============================================================================
// Raster State
// ============================================================================

/// Complete rasterizer state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RasterState {
    /// Polygon fill mode.
    pub polygon_mode: PolygonMode,
    /// Face culling mode.
    pub cull_mode: CullMode,
    /// Front face winding.
    pub front_face: FrontFace,
    /// Enable depth clamping.
    pub depth_clamp_enable: bool,
    /// Discard all primitives (no rasterization).
    pub rasterizer_discard_enable: bool,
    /// Depth bias configuration.
    pub depth_bias: DepthBias,
    /// Line width.
    pub line_width: f32,
    /// Conservative rasterization mode.
    pub conservative_mode: ConservativeRasterMode,
    /// Line rasterization mode.
    pub line_mode: LineRasterizationMode,
    /// Enable stippled lines.
    pub line_stipple_enable: bool,
    /// Line stipple factor.
    pub line_stipple_factor: u32,
    /// Line stipple pattern.
    pub line_stipple_pattern: u16,
}

impl RasterState {
    /// Create default raster state.
    pub fn new() -> Self {
        Self {
            polygon_mode: PolygonMode::Fill,
            cull_mode: CullMode::Back,
            front_face: FrontFace::CounterClockwise,
            depth_clamp_enable: false,
            rasterizer_discard_enable: false,
            depth_bias: DepthBias::disabled(),
            line_width: 1.0,
            conservative_mode: ConservativeRasterMode::Disabled,
            line_mode: LineRasterizationMode::Default,
            line_stipple_enable: false,
            line_stipple_factor: 1,
            line_stipple_pattern: 0xFFFF,
        }
    }

    /// Create raster state with no culling.
    pub fn no_cull() -> Self {
        Self {
            cull_mode: CullMode::None,
            ..Self::new()
        }
    }

    /// Create raster state for front face culling.
    pub fn cull_front() -> Self {
        Self {
            cull_mode: CullMode::Front,
            ..Self::new()
        }
    }

    /// Create wireframe raster state.
    pub fn wireframe() -> Self {
        Self {
            polygon_mode: PolygonMode::Line,
            cull_mode: CullMode::None,
            ..Self::new()
        }
    }

    /// Create point raster state.
    pub fn points() -> Self {
        Self {
            polygon_mode: PolygonMode::Point,
            cull_mode: CullMode::None,
            ..Self::new()
        }
    }

    /// Create shadow map raster state.
    pub fn shadow() -> Self {
        Self {
            cull_mode: CullMode::Front, // Front face culling for shadow maps
            depth_bias: DepthBias::shadow(),
            depth_clamp_enable: true,
            ..Self::new()
        }
    }

    /// Create decal raster state.
    pub fn decal() -> Self {
        Self {
            cull_mode: CullMode::Back,
            depth_bias: DepthBias::decal(),
            ..Self::new()
        }
    }

    /// Set cull mode.
    pub fn with_cull_mode(mut self, mode: CullMode) -> Self {
        self.cull_mode = mode;
        self
    }

    /// Set front face.
    pub fn with_front_face(mut self, face: FrontFace) -> Self {
        self.front_face = face;
        self
    }

    /// Set polygon mode.
    pub fn with_polygon_mode(mut self, mode: PolygonMode) -> Self {
        self.polygon_mode = mode;
        self
    }

    /// Set depth bias.
    pub fn with_depth_bias(mut self, bias: DepthBias) -> Self {
        self.depth_bias = bias;
        self
    }

    /// Set line width.
    pub fn with_line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }

    /// Enable depth clamping.
    pub fn with_depth_clamp(mut self, enable: bool) -> Self {
        self.depth_clamp_enable = enable;
        self
    }

    /// Enable rasterizer discard.
    pub fn with_rasterizer_discard(mut self, enable: bool) -> Self {
        self.rasterizer_discard_enable = enable;
        self
    }

    /// Set conservative rasterization mode.
    pub fn with_conservative(mut self, mode: ConservativeRasterMode) -> Self {
        self.conservative_mode = mode;
        self
    }

    /// Set line rasterization mode.
    pub fn with_line_mode(mut self, mode: LineRasterizationMode) -> Self {
        self.line_mode = mode;
        self
    }

    /// Set line stipple.
    pub fn with_line_stipple(mut self, factor: u32, pattern: u16) -> Self {
        self.line_stipple_enable = true;
        self.line_stipple_factor = factor;
        self.line_stipple_pattern = pattern;
        self
    }
}

impl Default for RasterState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Multisample State
// ============================================================================

/// Sample mask type.
pub type SampleMask = u64;

/// Multisample state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MultisampleState {
    /// Number of samples.
    pub sample_count: u32,
    /// Enable sample shading.
    pub sample_shading_enable: bool,
    /// Minimum sample shading (0.0 - 1.0).
    pub min_sample_shading: f32,
    /// Sample mask.
    pub sample_mask: SampleMask,
    /// Enable alpha to coverage.
    pub alpha_to_coverage_enable: bool,
    /// Enable alpha to one.
    pub alpha_to_one_enable: bool,
}

impl MultisampleState {
    /// Create no multisampling state.
    pub fn none() -> Self {
        Self {
            sample_count: 1,
            sample_shading_enable: false,
            min_sample_shading: 1.0,
            sample_mask: u64::MAX,
            alpha_to_coverage_enable: false,
            alpha_to_one_enable: false,
        }
    }

    /// Create 2x MSAA state.
    pub fn msaa_2x() -> Self {
        Self {
            sample_count: 2,
            ..Self::none()
        }
    }

    /// Create 4x MSAA state.
    pub fn msaa_4x() -> Self {
        Self {
            sample_count: 4,
            ..Self::none()
        }
    }

    /// Create 8x MSAA state.
    pub fn msaa_8x() -> Self {
        Self {
            sample_count: 8,
            ..Self::none()
        }
    }

    /// Create with sample count.
    pub fn with_samples(sample_count: u32) -> Self {
        Self {
            sample_count,
            ..Self::none()
        }
    }

    /// Enable sample shading.
    pub fn with_sample_shading(mut self, min_shading: f32) -> Self {
        self.sample_shading_enable = true;
        self.min_sample_shading = min_shading;
        self
    }

    /// Set sample mask.
    pub fn with_sample_mask(mut self, mask: SampleMask) -> Self {
        self.sample_mask = mask;
        self
    }

    /// Enable alpha to coverage.
    pub fn with_alpha_to_coverage(mut self) -> Self {
        self.alpha_to_coverage_enable = true;
        self
    }

    /// Enable alpha to one.
    pub fn with_alpha_to_one(mut self) -> Self {
        self.alpha_to_one_enable = true;
        self
    }
}

impl Default for MultisampleState {
    fn default() -> Self {
        Self::none()
    }
}

// ============================================================================
// Raster State Builder
// ============================================================================

/// Builder for raster state.
pub struct RasterStateBuilder {
    state: RasterState,
}

impl RasterStateBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            state: RasterState::new(),
        }
    }

    /// Set cull mode.
    pub fn cull_mode(mut self, mode: CullMode) -> Self {
        self.state.cull_mode = mode;
        self
    }

    /// Set no culling.
    pub fn no_cull(mut self) -> Self {
        self.state.cull_mode = CullMode::None;
        self
    }

    /// Set front face.
    pub fn front_face(mut self, face: FrontFace) -> Self {
        self.state.front_face = face;
        self
    }

    /// Set polygon mode.
    pub fn polygon_mode(mut self, mode: PolygonMode) -> Self {
        self.state.polygon_mode = mode;
        self
    }

    /// Set wireframe mode.
    pub fn wireframe(mut self) -> Self {
        self.state.polygon_mode = PolygonMode::Line;
        self
    }

    /// Set depth bias.
    pub fn depth_bias(mut self, constant: f32, clamp: f32, slope: f32) -> Self {
        self.state.depth_bias = DepthBias::enabled(constant, clamp, slope);
        self
    }

    /// Set line width.
    pub fn line_width(mut self, width: f32) -> Self {
        self.state.line_width = width;
        self
    }

    /// Enable depth clamp.
    pub fn depth_clamp(mut self) -> Self {
        self.state.depth_clamp_enable = true;
        self
    }

    /// Enable rasterizer discard.
    pub fn rasterizer_discard(mut self) -> Self {
        self.state.rasterizer_discard_enable = true;
        self
    }

    /// Set conservative rasterization.
    pub fn conservative(mut self, mode: ConservativeRasterMode) -> Self {
        self.state.conservative_mode = mode;
        self
    }

    /// Build the raster state.
    pub fn build(self) -> RasterState {
        self.state
    }
}

impl Default for RasterStateBuilder {
    fn default() -> Self {
        Self::new()
    }
}
