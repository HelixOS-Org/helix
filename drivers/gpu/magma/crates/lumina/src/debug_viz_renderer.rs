//! Debug Visualization Renderer Types for Lumina
//!
//! This module provides debug rendering infrastructure for
//! visualizing physics, collision, performance data, etc.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Debug Renderer Handles
// ============================================================================

/// Debug renderer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DebugRendererHandle(pub u64);

impl DebugRendererHandle {
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

impl Default for DebugRendererHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Debug line batch handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DebugLineBatchHandle(pub u64);

impl DebugLineBatchHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for DebugLineBatchHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Debug shape handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DebugShapeHandle(pub u64);

impl DebugShapeHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for DebugShapeHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Debug text handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DebugTextHandle(pub u64);

impl DebugTextHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for DebugTextHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Debug Renderer Creation
// ============================================================================

/// Debug renderer create info
#[derive(Clone, Debug)]
pub struct DebugRendererCreateInfo {
    /// Name
    pub name: String,
    /// Max lines
    pub max_lines: u32,
    /// Max triangles
    pub max_triangles: u32,
    /// Max text characters
    pub max_text_chars: u32,
    /// Max shapes
    pub max_shapes: u32,
    /// Features
    pub features: DebugRenderFeatures,
    /// Default line width
    pub default_line_width: f32,
    /// Depth test mode
    pub depth_test: DebugDepthTest,
}

impl DebugRendererCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_lines: 65536,
            max_triangles: 32768,
            max_text_chars: 8192,
            max_shapes: 1024,
            features: DebugRenderFeatures::all(),
            default_line_width: 1.0,
            depth_test: DebugDepthTest::TestAndWrite,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max lines
    pub fn with_max_lines(mut self, count: u32) -> Self {
        self.max_lines = count;
        self
    }

    /// With max triangles
    pub fn with_max_triangles(mut self, count: u32) -> Self {
        self.max_triangles = count;
        self
    }

    /// With max text
    pub fn with_max_text(mut self, chars: u32) -> Self {
        self.max_text_chars = chars;
        self
    }

    /// With max shapes
    pub fn with_max_shapes(mut self, count: u32) -> Self {
        self.max_shapes = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: DebugRenderFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With line width
    pub fn with_line_width(mut self, width: f32) -> Self {
        self.default_line_width = width;
        self
    }

    /// With depth test
    pub fn with_depth_test(mut self, test: DebugDepthTest) -> Self {
        self.depth_test = test;
        self
    }

    /// Standard debug renderer
    pub fn standard() -> Self {
        Self::new()
    }

    /// Lightweight (less capacity)
    pub fn lightweight() -> Self {
        Self::new()
            .with_max_lines(16384)
            .with_max_triangles(8192)
            .with_max_text(2048)
            .with_max_shapes(256)
    }

    /// High capacity
    pub fn high_capacity() -> Self {
        Self::new()
            .with_max_lines(262144)
            .with_max_triangles(131072)
            .with_max_text(32768)
            .with_max_shapes(4096)
    }

    /// No depth (always on top)
    pub fn overlay() -> Self {
        Self::new().with_depth_test(DebugDepthTest::AlwaysPass)
    }
}

impl Default for DebugRendererCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Debug render features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct DebugRenderFeatures: u32 {
        /// None
        const NONE = 0;
        /// Lines
        const LINES = 1 << 0;
        /// Triangles
        const TRIANGLES = 1 << 1;
        /// Text
        const TEXT = 1 << 2;
        /// Shapes
        const SHAPES = 1 << 3;
        /// Anti-aliasing
        const ANTI_ALIASING = 1 << 4;
        /// Dashed lines
        const DASHED_LINES = 1 << 5;
        /// 3D text
        const TEXT_3D = 1 << 6;
        /// Persistent primitives
        const PERSISTENT = 1 << 7;
        /// All features
        const ALL = 0xFF;
    }
}

impl Default for DebugRenderFeatures {
    fn default() -> Self {
        Self::all()
    }
}

/// Debug depth test mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DebugDepthTest {
    /// Always pass (overlay)
    AlwaysPass   = 0,
    /// Test only (read depth)
    #[default]
    TestOnly     = 1,
    /// Test and write
    TestAndWrite = 2,
}

// ============================================================================
// Debug Primitives
// ============================================================================

/// Debug line
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DebugLine {
    /// Start position
    pub start: [f32; 3],
    /// Start color
    pub start_color: DebugColor,
    /// End position
    pub end: [f32; 3],
    /// End color
    pub end_color: DebugColor,
    /// Width
    pub width: f32,
    /// Duration (0 = one frame, -1 = persistent)
    pub duration: f32,
}

impl DebugLine {
    /// Creates new line
    pub const fn new(start: [f32; 3], end: [f32; 3]) -> Self {
        Self {
            start,
            start_color: DebugColor::WHITE,
            end,
            end_color: DebugColor::WHITE,
            width: 1.0,
            duration: 0.0,
        }
    }

    /// With color
    pub const fn with_color(mut self, color: DebugColor) -> Self {
        self.start_color = color;
        self.end_color = color;
        self
    }

    /// With gradient
    pub const fn with_gradient(mut self, start: DebugColor, end: DebugColor) -> Self {
        self.start_color = start;
        self.end_color = end;
        self
    }

    /// With width
    pub const fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// With duration
    pub const fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    /// One frame
    pub const fn one_frame(start: [f32; 3], end: [f32; 3], color: DebugColor) -> Self {
        Self::new(start, end).with_color(color)
    }

    /// Persistent
    pub const fn persistent(start: [f32; 3], end: [f32; 3], color: DebugColor) -> Self {
        Self::new(start, end).with_color(color).with_duration(-1.0)
    }
}

impl Default for DebugLine {
    fn default() -> Self {
        Self::new([0.0; 3], [0.0; 3])
    }
}

/// Debug color
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct DebugColor {
    /// Red
    pub r: f32,
    /// Green
    pub g: f32,
    /// Blue
    pub b: f32,
    /// Alpha
    pub a: f32,
}

impl DebugColor {
    /// White
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    /// Black
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    /// Red
    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    /// Green
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    /// Blue
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    /// Yellow
    pub const YELLOW: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    /// Cyan
    pub const CYAN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    /// Magenta
    pub const MAGENTA: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    /// Orange
    pub const ORANGE: Self = Self {
        r: 1.0,
        g: 0.5,
        b: 0.0,
        a: 1.0,
    };
    /// Purple
    pub const PURPLE: Self = Self {
        r: 0.5,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };

    /// Creates new color
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// From RGB
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// From u8 values
    pub const fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// With alpha
    pub const fn with_alpha(mut self, a: f32) -> Self {
        self.a = a;
        self
    }
}

impl Default for DebugColor {
    fn default() -> Self {
        Self::WHITE
    }
}

// ============================================================================
// Debug Shapes
// ============================================================================

/// Debug shape
#[derive(Clone, Debug)]
pub struct DebugShape {
    /// Shape type
    pub shape_type: DebugShapeType,
    /// Transform
    pub transform: DebugTransform,
    /// Color
    pub color: DebugColor,
    /// Fill mode
    pub fill_mode: DebugFillMode,
    /// Duration
    pub duration: f32,
}

impl DebugShape {
    /// Creates new shape
    pub fn new(shape_type: DebugShapeType) -> Self {
        Self {
            shape_type,
            transform: DebugTransform::identity(),
            color: DebugColor::WHITE,
            fill_mode: DebugFillMode::Wireframe,
            duration: 0.0,
        }
    }

    /// With transform
    pub fn with_transform(mut self, transform: DebugTransform) -> Self {
        self.transform = transform;
        self
    }

    /// With color
    pub fn with_color(mut self, color: DebugColor) -> Self {
        self.color = color;
        self
    }

    /// With fill mode
    pub fn with_fill(mut self, mode: DebugFillMode) -> Self {
        self.fill_mode = mode;
        self
    }

    /// With duration
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    /// Sphere
    pub fn sphere(center: [f32; 3], radius: f32) -> Self {
        Self::new(DebugShapeType::Sphere { radius })
            .with_transform(DebugTransform::translation(center))
    }

    /// Box
    pub fn aabb(min: [f32; 3], max: [f32; 3]) -> Self {
        let center = [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ];
        let half_extents = [
            (max[0] - min[0]) * 0.5,
            (max[1] - min[1]) * 0.5,
            (max[2] - min[2]) * 0.5,
        ];
        Self::new(DebugShapeType::Box { half_extents })
            .with_transform(DebugTransform::translation(center))
    }

    /// Cylinder
    pub fn cylinder(base: [f32; 3], height: f32, radius: f32) -> Self {
        Self::new(DebugShapeType::Cylinder { height, radius })
            .with_transform(DebugTransform::translation(base))
    }

    /// Capsule
    pub fn capsule(center: [f32; 3], height: f32, radius: f32) -> Self {
        Self::new(DebugShapeType::Capsule { height, radius })
            .with_transform(DebugTransform::translation(center))
    }

    /// Arrow
    pub fn arrow(from: [f32; 3], to: [f32; 3], head_size: f32) -> Self {
        Self::new(DebugShapeType::Arrow {
            start: from,
            end: to,
            head_size,
        })
    }

    /// Cone
    pub fn cone(apex: [f32; 3], height: f32, radius: f32) -> Self {
        Self::new(DebugShapeType::Cone { height, radius })
            .with_transform(DebugTransform::translation(apex))
    }

    /// Plane
    pub fn plane(center: [f32; 3], normal: [f32; 3], size: f32) -> Self {
        Self::new(DebugShapeType::Plane { normal, size })
            .with_transform(DebugTransform::translation(center))
    }

    /// Frustum
    pub fn frustum(view_proj: [[f32; 4]; 4]) -> Self {
        Self::new(DebugShapeType::Frustum { view_proj })
    }

    /// Axis
    pub fn axis(center: [f32; 3], size: f32) -> Self {
        Self::new(DebugShapeType::Axis { size }).with_transform(DebugTransform::translation(center))
    }

    /// Grid
    pub fn grid(center: [f32; 3], size: f32, divisions: u32) -> Self {
        Self::new(DebugShapeType::Grid { size, divisions })
            .with_transform(DebugTransform::translation(center))
    }
}

impl Default for DebugShape {
    fn default() -> Self {
        Self::new(DebugShapeType::Point)
    }
}

/// Debug shape type
#[derive(Clone, Debug)]
pub enum DebugShapeType {
    /// Point
    Point,
    /// Sphere
    Sphere { radius: f32 },
    /// Box
    Box { half_extents: [f32; 3] },
    /// Cylinder
    Cylinder { height: f32, radius: f32 },
    /// Capsule
    Capsule { height: f32, radius: f32 },
    /// Cone
    Cone { height: f32, radius: f32 },
    /// Arrow
    Arrow {
        start: [f32; 3],
        end: [f32; 3],
        head_size: f32,
    },
    /// Plane
    Plane { normal: [f32; 3], size: f32 },
    /// Frustum
    Frustum { view_proj: [[f32; 4]; 4] },
    /// Axis (X/Y/Z)
    Axis { size: f32 },
    /// Grid
    Grid { size: f32, divisions: u32 },
    /// Triangle
    Triangle {
        v0: [f32; 3],
        v1: [f32; 3],
        v2: [f32; 3],
    },
    /// Circle
    Circle { radius: f32, segments: u32 },
    /// Arc
    Arc {
        radius: f32,
        start_angle: f32,
        end_angle: f32,
    },
}

impl Default for DebugShapeType {
    fn default() -> Self {
        Self::Point
    }
}

/// Debug transform
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DebugTransform {
    /// Position
    pub position: [f32; 3],
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Scale
    pub scale: [f32; 3],
}

impl DebugTransform {
    /// Identity transform
    pub const fn identity() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    /// Translation only
    pub const fn translation(pos: [f32; 3]) -> Self {
        Self {
            position: pos,
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    /// With scale
    pub const fn with_scale(mut self, scale: [f32; 3]) -> Self {
        self.scale = scale;
        self
    }

    /// With uniform scale
    pub const fn with_uniform_scale(mut self, scale: f32) -> Self {
        self.scale = [scale, scale, scale];
        self
    }

    /// With rotation
    pub const fn with_rotation(mut self, rotation: [f32; 4]) -> Self {
        self.rotation = rotation;
        self
    }
}

impl Default for DebugTransform {
    fn default() -> Self {
        Self::identity()
    }
}

/// Debug fill mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DebugFillMode {
    /// Wireframe
    #[default]
    Wireframe      = 0,
    /// Solid
    Solid          = 1,
    /// Solid with wireframe overlay
    SolidWireframe = 2,
}

// ============================================================================
// Debug Text
// ============================================================================

/// Debug text
#[derive(Clone, Debug)]
pub struct DebugText {
    /// Text content
    pub text: String,
    /// Position (screen or world)
    pub position: DebugTextPosition,
    /// Color
    pub color: DebugColor,
    /// Font size
    pub font_size: f32,
    /// Alignment
    pub alignment: DebugTextAlignment,
    /// Duration
    pub duration: f32,
}

impl DebugText {
    /// Creates new text
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            position: DebugTextPosition::Screen { x: 0.0, y: 0.0 },
            color: DebugColor::WHITE,
            font_size: 14.0,
            alignment: DebugTextAlignment::TopLeft,
            duration: 0.0,
        }
    }

    /// At screen position
    pub fn at_screen(mut self, x: f32, y: f32) -> Self {
        self.position = DebugTextPosition::Screen { x, y };
        self
    }

    /// At world position
    pub fn at_world(mut self, pos: [f32; 3]) -> Self {
        self.position = DebugTextPosition::World { position: pos };
        self
    }

    /// With color
    pub fn with_color(mut self, color: DebugColor) -> Self {
        self.color = color;
        self
    }

    /// With font size
    pub fn with_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// With alignment
    pub fn with_alignment(mut self, alignment: DebugTextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// With duration
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    /// Screen text
    pub fn screen(text: impl Into<String>, x: f32, y: f32) -> Self {
        Self::new(text).at_screen(x, y)
    }

    /// World text
    pub fn world(text: impl Into<String>, pos: [f32; 3]) -> Self {
        Self::new(text).at_world(pos)
    }
}

impl Default for DebugText {
    fn default() -> Self {
        Self::new("")
    }
}

/// Debug text position
#[derive(Clone, Copy, Debug)]
pub enum DebugTextPosition {
    /// Screen space
    Screen { x: f32, y: f32 },
    /// World space
    World { position: [f32; 3] },
}

impl Default for DebugTextPosition {
    fn default() -> Self {
        Self::Screen { x: 0.0, y: 0.0 }
    }
}

/// Debug text alignment
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DebugTextAlignment {
    /// Top left
    #[default]
    TopLeft      = 0,
    /// Top center
    TopCenter    = 1,
    /// Top right
    TopRight     = 2,
    /// Center left
    CenterLeft   = 3,
    /// Center
    Center       = 4,
    /// Center right
    CenterRight  = 5,
    /// Bottom left
    BottomLeft   = 6,
    /// Bottom center
    BottomCenter = 7,
    /// Bottom right
    BottomRight  = 8,
}

// ============================================================================
// Debug Categories
// ============================================================================

/// Debug draw category
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum DebugCategory {
    /// General
    General     = 0,
    /// Physics
    Physics     = 1,
    /// Collision
    Collision   = 2,
    /// AI / Navigation
    Navigation  = 3,
    /// Audio
    Audio       = 4,
    /// Animation
    Animation   = 5,
    /// Performance
    Performance = 6,
    /// Network
    Network     = 7,
    /// UI
    Ui          = 8,
    /// Custom (0-15)
    Custom(u8),
}

impl Default for DebugCategory {
    fn default() -> Self {
        Self::General
    }
}

bitflags::bitflags! {
    /// Debug category mask
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct DebugCategoryMask: u32 {
        /// None
        const NONE = 0;
        /// General
        const GENERAL = 1 << 0;
        /// Physics
        const PHYSICS = 1 << 1;
        /// Collision
        const COLLISION = 1 << 2;
        /// Navigation
        const NAVIGATION = 1 << 3;
        /// Audio
        const AUDIO = 1 << 4;
        /// Animation
        const ANIMATION = 1 << 5;
        /// Performance
        const PERFORMANCE = 1 << 6;
        /// Network
        const NETWORK = 1 << 7;
        /// UI
        const UI = 1 << 8;
        /// All
        const ALL = 0xFFFF;
    }
}

// ============================================================================
// GPU Structures
// ============================================================================

/// GPU debug vertex
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuDebugVertex {
    /// Position
    pub position: [f32; 3],
    /// Color
    pub color: [f32; 4],
}

impl GpuDebugVertex {
    /// Creates new vertex
    pub const fn new(position: [f32; 3], color: [f32; 4]) -> Self {
        Self { position, color }
    }
}

/// GPU debug line
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuDebugLine {
    /// Start position
    pub start: [f32; 3],
    /// Padding
    pub _pad0: f32,
    /// Start color
    pub start_color: [f32; 4],
    /// End position
    pub end: [f32; 3],
    /// Width
    pub width: f32,
    /// End color
    pub end_color: [f32; 4],
}

/// GPU debug text glyph
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuDebugGlyph {
    /// Position (screen space)
    pub position: [f32; 2],
    /// Size
    pub size: [f32; 2],
    /// UV min
    pub uv_min: [f32; 2],
    /// UV max
    pub uv_max: [f32; 2],
    /// Color
    pub color: [f32; 4],
}

/// GPU debug constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuDebugConstants {
    /// View-projection matrix
    pub view_proj: [[f32; 4]; 4],
    /// Screen size
    pub screen_size: [f32; 2],
    /// Time
    pub time: f32,
    /// Line anti-aliasing
    pub line_aa: f32,
}

// ============================================================================
// Statistics
// ============================================================================

/// Debug renderer statistics
#[derive(Clone, Debug, Default)]
pub struct DebugRendererStats {
    /// Lines drawn
    pub lines_drawn: u32,
    /// Triangles drawn
    pub triangles_drawn: u32,
    /// Text glyphs drawn
    pub glyphs_drawn: u32,
    /// Shapes drawn
    pub shapes_drawn: u32,
    /// Persistent primitives
    pub persistent_count: u32,
    /// Vertex buffer usage
    pub vertex_buffer_usage: u64,
    /// Draw calls
    pub draw_calls: u32,
}

impl DebugRendererStats {
    /// Total primitives
    pub fn total_primitives(&self) -> u32 {
        self.lines_drawn + self.triangles_drawn + self.shapes_drawn
    }
}
