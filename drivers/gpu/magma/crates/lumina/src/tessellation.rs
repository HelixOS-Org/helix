//! Tessellation types
//!
//! This module provides types for tessellation shaders.

/// Tessellation domain
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TessellationDomain {
    /// Triangle domain
    Triangles,
    /// Quad domain
    Quads,
    /// Isoline domain
    Isolines,
}

impl Default for TessellationDomain {
    fn default() -> Self {
        Self::Triangles
    }
}

/// Tessellation spacing
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TessellationSpacing {
    /// Equal spacing
    Equal,
    /// Fractional even spacing
    FractionalEven,
    /// Fractional odd spacing
    FractionalOdd,
}

impl Default for TessellationSpacing {
    fn default() -> Self {
        Self::Equal
    }
}

/// Tessellation output primitive order
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TessellationOrder {
    /// Counter-clockwise winding
    Ccw,
    /// Clockwise winding
    Cw,
}

impl Default for TessellationOrder {
    fn default() -> Self {
        Self::Ccw
    }
}

/// Tessellation state
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TessellationState {
    /// Number of control points per patch
    pub patch_control_points: u32,
    /// Domain type
    pub domain: TessellationDomain,
    /// Spacing mode
    pub spacing: TessellationSpacing,
    /// Vertex order
    pub order: TessellationOrder,
    /// Whether to generate point mode output
    pub point_mode: bool,
}

impl TessellationState {
    /// Creates new tessellation state
    pub const fn new(patch_control_points: u32) -> Self {
        Self {
            patch_control_points,
            domain: TessellationDomain::Triangles,
            spacing: TessellationSpacing::Equal,
            order: TessellationOrder::Ccw,
            point_mode: false,
        }
    }

    /// Triangle tessellation with 3 control points
    pub const fn triangles() -> Self {
        Self::new(3)
    }

    /// Quad tessellation with 4 control points
    pub const fn quads() -> Self {
        Self {
            patch_control_points: 4,
            domain: TessellationDomain::Quads,
            spacing: TessellationSpacing::Equal,
            order: TessellationOrder::Ccw,
            point_mode: false,
        }
    }

    /// Isoline tessellation
    pub const fn isolines(control_points: u32) -> Self {
        Self {
            patch_control_points: control_points,
            domain: TessellationDomain::Isolines,
            spacing: TessellationSpacing::Equal,
            order: TessellationOrder::Ccw,
            point_mode: false,
        }
    }

    /// Sets spacing mode
    pub const fn with_spacing(mut self, spacing: TessellationSpacing) -> Self {
        self.spacing = spacing;
        self
    }

    /// Sets winding order
    pub const fn with_order(mut self, order: TessellationOrder) -> Self {
        self.order = order;
        self
    }
}

/// Tessellation levels for a patch
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TessellationLevels {
    /// Outer tessellation levels
    pub outer: [f32; 4],
    /// Inner tessellation levels
    pub inner: [f32; 2],
}

impl TessellationLevels {
    /// Uniform tessellation
    pub const fn uniform(level: f32) -> Self {
        Self {
            outer: [level; 4],
            inner: [level; 2],
        }
    }

    /// Adaptive tessellation levels
    pub const fn adaptive(outer: [f32; 4], inner: [f32; 2]) -> Self {
        Self { outer, inner }
    }

    /// Triangle tessellation levels
    pub const fn triangle(outer: [f32; 3], inner: f32) -> Self {
        Self {
            outer: [outer[0], outer[1], outer[2], 1.0],
            inner: [inner, 1.0],
        }
    }

    /// Quad tessellation levels
    pub const fn quad(outer: [f32; 4], inner: [f32; 2]) -> Self {
        Self { outer, inner }
    }

    /// Isoline tessellation levels
    pub const fn isoline(outer_0: f32, outer_1: f32) -> Self {
        Self {
            outer: [outer_0, outer_1, 1.0, 1.0],
            inner: [1.0, 1.0],
        }
    }
}

/// Tessellation capabilities
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TessellationCapabilities {
    /// Maximum tessellation level
    pub max_tessellation_level: f32,
    /// Maximum patch size
    pub max_patch_size: u32,
    /// Maximum tessellation control total output components
    pub max_tessellation_control_total_output_components: u32,
    /// Maximum tessellation evaluation input components
    pub max_tessellation_evaluation_input_components: u32,
    /// Maximum tessellation control per-vertex output components
    pub max_tessellation_control_per_vertex_output_components: u32,
    /// Maximum tessellation control per-patch output components
    pub max_tessellation_control_per_patch_output_components: u32,
    /// Maximum tessellation evaluation output components
    pub max_tessellation_evaluation_output_components: u32,
}

/// Tessellation factor calculation mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TessellationFactorMode {
    /// Fixed tessellation level
    Fixed,
    /// Distance-based LOD
    DistanceBased,
    /// Screen-space edge-based
    ScreenSpaceEdge,
    /// Custom factors from buffer
    Custom,
}

impl Default for TessellationFactorMode {
    fn default() -> Self {
        Self::Fixed
    }
}

/// Distance-based LOD parameters
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DistanceLodParams {
    /// Minimum distance (full tessellation)
    pub min_distance: f32,
    /// Maximum distance (minimum tessellation)
    pub max_distance: f32,
    /// Minimum tessellation level
    pub min_level: f32,
    /// Maximum tessellation level
    pub max_level: f32,
}

impl DistanceLodParams {
    /// Creates new distance LOD params
    pub const fn new(min_distance: f32, max_distance: f32, min_level: f32, max_level: f32) -> Self {
        Self {
            min_distance,
            max_distance,
            min_level,
            max_level,
        }
    }

    /// Default terrain LOD
    pub const fn terrain() -> Self {
        Self::new(10.0, 500.0, 1.0, 64.0)
    }

    /// Calculates tessellation level from distance
    pub fn level_for_distance(&self, distance: f32) -> f32 {
        if distance <= self.min_distance {
            self.max_level
        } else if distance >= self.max_distance {
            self.min_level
        } else {
            let t = (distance - self.min_distance) / (self.max_distance - self.min_distance);
            self.max_level + (self.min_level - self.max_level) * t
        }
    }
}

impl Default for DistanceLodParams {
    fn default() -> Self {
        Self::terrain()
    }
}

/// Screen-space LOD parameters
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ScreenSpaceLodParams {
    /// Target edge length in pixels
    pub target_edge_pixels: f32,
    /// Minimum tessellation level
    pub min_level: f32,
    /// Maximum tessellation level
    pub max_level: f32,
    /// Screen width
    pub screen_width: f32,
    /// Screen height
    pub screen_height: f32,
}

impl ScreenSpaceLodParams {
    /// Creates new screen-space LOD params
    pub const fn new(target_edge_pixels: f32) -> Self {
        Self {
            target_edge_pixels,
            min_level: 1.0,
            max_level: 64.0,
            screen_width: 1920.0,
            screen_height: 1080.0,
        }
    }

    /// Calculates tessellation level from edge length
    pub fn level_for_edge(&self, edge_length_pixels: f32) -> f32 {
        let level = edge_length_pixels / self.target_edge_pixels;
        level.clamp(self.min_level, self.max_level)
    }
}

impl Default for ScreenSpaceLodParams {
    fn default() -> Self {
        Self::new(16.0)
    }
}

/// Patch primitive type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PatchPrimitiveType {
    /// Bezier patches
    Bezier,
    /// B-spline patches
    BSpline,
    /// Catmull-Clark subdivision
    CatmullClark,
    /// Loop subdivision
    Loop,
    /// Gregory patches
    Gregory,
    /// PN triangles
    PnTriangle,
    /// Phong tessellation
    Phong,
}

/// Bezier patch (bicubic, 16 control points)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BezierPatch {
    /// 4x4 control points
    pub control_points: [[f32; 3]; 16],
}

impl BezierPatch {
    /// Evaluates the patch at (u, v)
    pub fn evaluate(&self, u: f32, v: f32) -> [f32; 3] {
        // Bernstein basis
        let bu = bezier_basis(u);
        let bv = bezier_basis(v);

        let mut result = [0.0f32; 3];
        for i in 0..4 {
            for j in 0..4 {
                let weight = bu[i] * bv[j];
                let cp = &self.control_points[i * 4 + j];
                result[0] += cp[0] * weight;
                result[1] += cp[1] * weight;
                result[2] += cp[2] * weight;
            }
        }
        result
    }
}

impl Default for BezierPatch {
    fn default() -> Self {
        Self {
            control_points: [[0.0; 3]; 16],
        }
    }
}

/// Bernstein basis functions
fn bezier_basis(t: f32) -> [f32; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;

    [mt3, 3.0 * mt2 * t, 3.0 * mt * t2, t3]
}

/// PN Triangle parameters
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PnTriangleParams {
    /// Enable PN triangles
    pub enabled: bool,
    /// Interpolate normals (Phong interpolation)
    pub phong: bool,
    /// Phong blend factor (0-1)
    pub phong_blend: f32,
}

impl PnTriangleParams {
    /// Default PN triangles
    pub const fn default_pn() -> Self {
        Self {
            enabled: true,
            phong: true,
            phong_blend: 0.75,
        }
    }
}

impl Default for PnTriangleParams {
    fn default() -> Self {
        Self::default_pn()
    }
}

/// Displacement mapping parameters
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DisplacementParams {
    /// Displacement scale
    pub scale: f32,
    /// Displacement bias
    pub bias: f32,
    /// UV scale
    pub uv_scale: [f32; 2],
}

impl DisplacementParams {
    /// Creates new displacement params
    pub const fn new(scale: f32) -> Self {
        Self {
            scale,
            bias: 0.0,
            uv_scale: [1.0, 1.0],
        }
    }

    /// With bias (for centered displacement)
    pub const fn centered(scale: f32) -> Self {
        Self {
            scale,
            bias: -0.5,
            uv_scale: [1.0, 1.0],
        }
    }
}

impl Default for DisplacementParams {
    fn default() -> Self {
        Self::new(1.0)
    }
}
