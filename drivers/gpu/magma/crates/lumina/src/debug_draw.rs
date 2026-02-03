//! Debug Draw Types for Lumina
//!
//! This module provides debug visualization primitives including
//! lines, shapes, text, and gizmos for development and debugging.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Debug Draw Context
// ============================================================================

/// Debug draw context handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DebugDrawHandle(pub u64);

impl DebugDrawHandle {
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

impl Default for DebugDrawHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Debug Draw Settings
// ============================================================================

/// Debug draw configuration
#[derive(Clone, Debug)]
pub struct DebugDrawSettings {
    /// Enable debug drawing
    pub enabled: bool,
    /// Depth testing
    pub depth_test: bool,
    /// Wireframe mode
    pub wireframe: bool,
    /// Line width
    pub line_width: f32,
    /// Default color
    pub default_color: DebugColor,
    /// Duration for persistent draws
    pub default_duration: f32,
    /// Maximum vertices
    pub max_vertices: u32,
    /// Maximum draw commands
    pub max_commands: u32,
}

impl DebugDrawSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            enabled: true,
            depth_test: true,
            wireframe: true,
            line_width: 1.0,
            default_color: DebugColor::WHITE,
            default_duration: 0.0,
            max_vertices: 100_000,
            max_commands: 10_000,
        }
    }

    /// Overlay mode (no depth test)
    pub fn overlay() -> Self {
        Self {
            depth_test: false,
            ..Self::new()
        }
    }

    /// With depth test
    pub fn with_depth_test(mut self, enabled: bool) -> Self {
        self.depth_test = enabled;
        self
    }

    /// With line width
    pub fn with_line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }
}

impl Default for DebugDrawSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Debug Color
// ============================================================================

/// Debug draw color
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
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    /// Black
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    /// Red
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    /// Green
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    /// Blue
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    /// Yellow
    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.0);
    /// Cyan
    pub const CYAN: Self = Self::rgb(0.0, 1.0, 1.0);
    /// Magenta
    pub const MAGENTA: Self = Self::rgb(1.0, 0.0, 1.0);
    /// Orange
    pub const ORANGE: Self = Self::rgb(1.0, 0.5, 0.0);
    /// Pink
    pub const PINK: Self = Self::rgb(1.0, 0.4, 0.7);
    /// Gray
    pub const GRAY: Self = Self::rgb(0.5, 0.5, 0.5);
    /// Light gray
    pub const LIGHT_GRAY: Self = Self::rgb(0.75, 0.75, 0.75);
    /// Dark gray
    pub const DARK_GRAY: Self = Self::rgb(0.25, 0.25, 0.25);

    /// Creates RGB color
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Creates RGBA color
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Creates from u8 values
    pub fn from_u8(r: u8, g: u8, b: u8) -> Self {
        Self::rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
    }

    /// Creates from hex value
    pub fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let b = (hex & 0xFF) as f32 / 255.0;
        Self::rgb(r, g, b)
    }

    /// With alpha
    pub const fn with_alpha(mut self, a: f32) -> Self {
        self.a = a;
        self
    }

    /// Faded color
    pub const fn faded(mut self) -> Self {
        self.a = 0.5;
        self
    }

    /// To array
    pub const fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Lerp between colors
    pub fn lerp(a: Self, b: Self, t: f32) -> Self {
        Self {
            r: a.r + (b.r - a.r) * t,
            g: a.g + (b.g - a.g) * t,
            b: a.b + (b.b - a.b) * t,
            a: a.a + (b.a - a.a) * t,
        }
    }
}

impl Default for DebugColor {
    fn default() -> Self {
        Self::WHITE
    }
}

// ============================================================================
// Debug Draw Commands
// ============================================================================

/// Debug draw command
#[derive(Clone, Debug)]
pub enum DebugDrawCommand {
    /// Draw line
    Line(DebugLine),
    /// Draw lines
    Lines(Vec<DebugLine>),
    /// Draw point
    Point(DebugPoint),
    /// Draw points
    Points(Vec<DebugPoint>),
    /// Draw triangle
    Triangle(DebugTriangle),
    /// Draw triangles
    Triangles(Vec<DebugTriangle>),
    /// Draw box
    Box(DebugBox),
    /// Draw sphere
    Sphere(DebugSphere),
    /// Draw capsule
    Capsule(DebugCapsule),
    /// Draw cylinder
    Cylinder(DebugCylinder),
    /// Draw cone
    Cone(DebugCone),
    /// Draw circle
    Circle(DebugCircle),
    /// Draw arc
    Arc(DebugArc),
    /// Draw arrow
    Arrow(DebugArrow),
    /// Draw frustum
    Frustum(DebugFrustum),
    /// Draw axis
    Axis(DebugAxis),
    /// Draw grid
    Grid(DebugGrid),
    /// Draw text
    Text(DebugText),
    /// Draw bone
    Bone(DebugBone),
    /// Draw bounds
    Bounds(DebugBounds),
}

// ============================================================================
// Debug Primitives
// ============================================================================

/// Debug line
#[derive(Clone, Copy, Debug)]
pub struct DebugLine {
    /// Start point
    pub start: [f32; 3],
    /// End point
    pub end: [f32; 3],
    /// Start color
    pub start_color: DebugColor,
    /// End color
    pub end_color: DebugColor,
    /// Duration
    pub duration: f32,
    /// Depth test
    pub depth_test: bool,
}

impl DebugLine {
    /// Creates line
    pub fn new(start: [f32; 3], end: [f32; 3], color: DebugColor) -> Self {
        Self {
            start,
            end,
            start_color: color,
            end_color: color,
            duration: 0.0,
            depth_test: true,
        }
    }

    /// With gradient
    pub fn gradient(start: [f32; 3], end: [f32; 3], start_color: DebugColor, end_color: DebugColor) -> Self {
        Self {
            start,
            end,
            start_color,
            end_color,
            duration: 0.0,
            depth_test: true,
        }
    }

    /// With duration
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    /// No depth test
    pub fn overlay(mut self) -> Self {
        self.depth_test = false;
        self
    }
}

/// Debug point
#[derive(Clone, Copy, Debug)]
pub struct DebugPoint {
    /// Position
    pub position: [f32; 3],
    /// Color
    pub color: DebugColor,
    /// Size
    pub size: f32,
    /// Duration
    pub duration: f32,
    /// Depth test
    pub depth_test: bool,
}

impl DebugPoint {
    /// Creates point
    pub fn new(position: [f32; 3], color: DebugColor) -> Self {
        Self {
            position,
            color,
            size: 5.0,
            duration: 0.0,
            depth_test: true,
        }
    }

    /// With size
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// With duration
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }
}

/// Debug triangle
#[derive(Clone, Copy, Debug)]
pub struct DebugTriangle {
    /// Vertices
    pub vertices: [[f32; 3]; 3],
    /// Color
    pub color: DebugColor,
    /// Filled or wireframe
    pub filled: bool,
    /// Duration
    pub duration: f32,
    /// Depth test
    pub depth_test: bool,
}

impl DebugTriangle {
    /// Creates triangle
    pub fn new(v0: [f32; 3], v1: [f32; 3], v2: [f32; 3], color: DebugColor) -> Self {
        Self {
            vertices: [v0, v1, v2],
            color,
            filled: false,
            duration: 0.0,
            depth_test: true,
        }
    }

    /// Filled triangle
    pub fn filled(mut self) -> Self {
        self.filled = true;
        self
    }
}

/// Debug box
#[derive(Clone, Copy, Debug)]
pub struct DebugBox {
    /// Center
    pub center: [f32; 3],
    /// Size (half extents)
    pub size: [f32; 3],
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Color
    pub color: DebugColor,
    /// Filled or wireframe
    pub filled: bool,
    /// Duration
    pub duration: f32,
    /// Depth test
    pub depth_test: bool,
}

impl DebugBox {
    /// Creates axis-aligned box
    pub fn new(center: [f32; 3], size: [f32; 3], color: DebugColor) -> Self {
        Self {
            center,
            size,
            rotation: [0.0, 0.0, 0.0, 1.0],
            color,
            filled: false,
            duration: 0.0,
            depth_test: true,
        }
    }

    /// Creates unit box
    pub fn unit(center: [f32; 3], color: DebugColor) -> Self {
        Self::new(center, [0.5, 0.5, 0.5], color)
    }

    /// Creates from min/max
    pub fn from_min_max(min: [f32; 3], max: [f32; 3], color: DebugColor) -> Self {
        let center = [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ];
        let size = [
            (max[0] - min[0]) * 0.5,
            (max[1] - min[1]) * 0.5,
            (max[2] - min[2]) * 0.5,
        ];
        Self::new(center, size, color)
    }

    /// With rotation
    pub fn with_rotation(mut self, rotation: [f32; 4]) -> Self {
        self.rotation = rotation;
        self
    }

    /// Filled
    pub fn filled(mut self) -> Self {
        self.filled = true;
        self
    }

    /// With duration
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }
}

/// Debug sphere
#[derive(Clone, Copy, Debug)]
pub struct DebugSphere {
    /// Center
    pub center: [f32; 3],
    /// Radius
    pub radius: f32,
    /// Color
    pub color: DebugColor,
    /// Segments
    pub segments: u32,
    /// Filled or wireframe
    pub filled: bool,
    /// Duration
    pub duration: f32,
    /// Depth test
    pub depth_test: bool,
}

impl DebugSphere {
    /// Creates sphere
    pub fn new(center: [f32; 3], radius: f32, color: DebugColor) -> Self {
        Self {
            center,
            radius,
            color,
            segments: 16,
            filled: false,
            duration: 0.0,
            depth_test: true,
        }
    }

    /// With segments
    pub fn with_segments(mut self, segments: u32) -> Self {
        self.segments = segments;
        self
    }

    /// Filled
    pub fn filled(mut self) -> Self {
        self.filled = true;
        self
    }

    /// With duration
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }
}

/// Debug capsule
#[derive(Clone, Copy, Debug)]
pub struct DebugCapsule {
    /// Start point
    pub start: [f32; 3],
    /// End point
    pub end: [f32; 3],
    /// Radius
    pub radius: f32,
    /// Color
    pub color: DebugColor,
    /// Segments
    pub segments: u32,
    /// Duration
    pub duration: f32,
    /// Depth test
    pub depth_test: bool,
}

impl DebugCapsule {
    /// Creates capsule
    pub fn new(start: [f32; 3], end: [f32; 3], radius: f32, color: DebugColor) -> Self {
        Self {
            start,
            end,
            radius,
            color,
            segments: 16,
            duration: 0.0,
            depth_test: true,
        }
    }

    /// Creates from center and height
    pub fn from_height(center: [f32; 3], height: f32, radius: f32, color: DebugColor) -> Self {
        let half = height * 0.5 - radius;
        let start = [center[0], center[1] - half, center[2]];
        let end = [center[0], center[1] + half, center[2]];
        Self::new(start, end, radius, color)
    }

    /// With segments
    pub fn with_segments(mut self, segments: u32) -> Self {
        self.segments = segments;
        self
    }
}

/// Debug cylinder
#[derive(Clone, Copy, Debug)]
pub struct DebugCylinder {
    /// Base center
    pub base: [f32; 3],
    /// Top center
    pub top: [f32; 3],
    /// Radius
    pub radius: f32,
    /// Color
    pub color: DebugColor,
    /// Segments
    pub segments: u32,
    /// Duration
    pub duration: f32,
    /// Depth test
    pub depth_test: bool,
}

impl DebugCylinder {
    /// Creates cylinder
    pub fn new(base: [f32; 3], top: [f32; 3], radius: f32, color: DebugColor) -> Self {
        Self {
            base,
            top,
            radius,
            color,
            segments: 16,
            duration: 0.0,
            depth_test: true,
        }
    }

    /// Creates from center and height
    pub fn from_height(center: [f32; 3], height: f32, radius: f32, color: DebugColor) -> Self {
        let half = height * 0.5;
        let base = [center[0], center[1] - half, center[2]];
        let top = [center[0], center[1] + half, center[2]];
        Self::new(base, top, radius, color)
    }
}

/// Debug cone
#[derive(Clone, Copy, Debug)]
pub struct DebugCone {
    /// Base center
    pub base: [f32; 3],
    /// Tip
    pub tip: [f32; 3],
    /// Radius
    pub radius: f32,
    /// Color
    pub color: DebugColor,
    /// Segments
    pub segments: u32,
    /// Duration
    pub duration: f32,
    /// Depth test
    pub depth_test: bool,
}

impl DebugCone {
    /// Creates cone
    pub fn new(base: [f32; 3], tip: [f32; 3], radius: f32, color: DebugColor) -> Self {
        Self {
            base,
            tip,
            radius,
            color,
            segments: 16,
            duration: 0.0,
            depth_test: true,
        }
    }

    /// Creates from center and height
    pub fn from_height(center: [f32; 3], height: f32, radius: f32, color: DebugColor) -> Self {
        let half = height * 0.5;
        let base = [center[0], center[1] - half, center[2]];
        let tip = [center[0], center[1] + half, center[2]];
        Self::new(base, tip, radius, color)
    }
}

/// Debug circle
#[derive(Clone, Copy, Debug)]
pub struct DebugCircle {
    /// Center
    pub center: [f32; 3],
    /// Normal
    pub normal: [f32; 3],
    /// Radius
    pub radius: f32,
    /// Color
    pub color: DebugColor,
    /// Segments
    pub segments: u32,
    /// Filled or wireframe
    pub filled: bool,
    /// Duration
    pub duration: f32,
    /// Depth test
    pub depth_test: bool,
}

impl DebugCircle {
    /// Creates circle
    pub fn new(center: [f32; 3], normal: [f32; 3], radius: f32, color: DebugColor) -> Self {
        Self {
            center,
            normal,
            radius,
            color,
            segments: 32,
            filled: false,
            duration: 0.0,
            depth_test: true,
        }
    }

    /// Creates XZ circle (horizontal)
    pub fn horizontal(center: [f32; 3], radius: f32, color: DebugColor) -> Self {
        Self::new(center, [0.0, 1.0, 0.0], radius, color)
    }

    /// Creates XY circle (vertical facing Z)
    pub fn vertical_z(center: [f32; 3], radius: f32, color: DebugColor) -> Self {
        Self::new(center, [0.0, 0.0, 1.0], radius, color)
    }

    /// Creates YZ circle (vertical facing X)
    pub fn vertical_x(center: [f32; 3], radius: f32, color: DebugColor) -> Self {
        Self::new(center, [1.0, 0.0, 0.0], radius, color)
    }

    /// With segments
    pub fn with_segments(mut self, segments: u32) -> Self {
        self.segments = segments;
        self
    }

    /// Filled
    pub fn filled(mut self) -> Self {
        self.filled = true;
        self
    }
}

/// Debug arc
#[derive(Clone, Copy, Debug)]
pub struct DebugArc {
    /// Center
    pub center: [f32; 3],
    /// Normal
    pub normal: [f32; 3],
    /// Radius
    pub radius: f32,
    /// Start angle (radians)
    pub start_angle: f32,
    /// End angle (radians)
    pub end_angle: f32,
    /// Color
    pub color: DebugColor,
    /// Segments
    pub segments: u32,
    /// Duration
    pub duration: f32,
    /// Depth test
    pub depth_test: bool,
}

impl DebugArc {
    /// Creates arc
    pub fn new(
        center: [f32; 3],
        normal: [f32; 3],
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        color: DebugColor,
    ) -> Self {
        Self {
            center,
            normal,
            radius,
            start_angle,
            end_angle,
            color,
            segments: 32,
            duration: 0.0,
            depth_test: true,
        }
    }
}

/// Debug arrow
#[derive(Clone, Copy, Debug)]
pub struct DebugArrow {
    /// Start point
    pub start: [f32; 3],
    /// End point
    pub end: [f32; 3],
    /// Color
    pub color: DebugColor,
    /// Head size
    pub head_size: f32,
    /// Duration
    pub duration: f32,
    /// Depth test
    pub depth_test: bool,
}

impl DebugArrow {
    /// Creates arrow
    pub fn new(start: [f32; 3], end: [f32; 3], color: DebugColor) -> Self {
        Self {
            start,
            end,
            color,
            head_size: 0.1,
            duration: 0.0,
            depth_test: true,
        }
    }

    /// Creates from direction
    pub fn from_direction(start: [f32; 3], direction: [f32; 3], length: f32, color: DebugColor) -> Self {
        let end = [
            start[0] + direction[0] * length,
            start[1] + direction[1] * length,
            start[2] + direction[2] * length,
        ];
        Self::new(start, end, color)
    }

    /// With head size
    pub fn with_head_size(mut self, size: f32) -> Self {
        self.head_size = size;
        self
    }

    /// With duration
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }
}

/// Debug frustum
#[derive(Clone, Copy, Debug)]
pub struct DebugFrustum {
    /// View-projection matrix inverse
    pub inverse_vp: [[f32; 4]; 4],
    /// Color
    pub color: DebugColor,
    /// Duration
    pub duration: f32,
    /// Depth test
    pub depth_test: bool,
}

impl DebugFrustum {
    /// Creates frustum
    pub fn new(inverse_vp: [[f32; 4]; 4], color: DebugColor) -> Self {
        Self {
            inverse_vp,
            color,
            duration: 0.0,
            depth_test: true,
        }
    }

    /// Creates from camera parameters
    pub fn from_params(
        fov: f32,
        aspect: f32,
        near: f32,
        far: f32,
        position: [f32; 3],
        forward: [f32; 3],
        up: [f32; 3],
        color: DebugColor,
    ) -> Self {
        // Simplified - actual implementation would compute proper matrix
        let _ = (fov, aspect, near, far, position, forward, up);
        Self {
            inverse_vp: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
            color,
            duration: 0.0,
            depth_test: true,
        }
    }
}

/// Debug axis gizmo
#[derive(Clone, Copy, Debug)]
pub struct DebugAxis {
    /// Origin
    pub origin: [f32; 3],
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Scale
    pub scale: f32,
    /// X color
    pub x_color: DebugColor,
    /// Y color
    pub y_color: DebugColor,
    /// Z color
    pub z_color: DebugColor,
    /// Duration
    pub duration: f32,
    /// Depth test
    pub depth_test: bool,
}

impl DebugAxis {
    /// Creates axis
    pub fn new(origin: [f32; 3], scale: f32) -> Self {
        Self {
            origin,
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale,
            x_color: DebugColor::RED,
            y_color: DebugColor::GREEN,
            z_color: DebugColor::BLUE,
            duration: 0.0,
            depth_test: true,
        }
    }

    /// With rotation
    pub fn with_rotation(mut self, rotation: [f32; 4]) -> Self {
        self.rotation = rotation;
        self
    }

    /// With colors
    pub fn with_colors(mut self, x: DebugColor, y: DebugColor, z: DebugColor) -> Self {
        self.x_color = x;
        self.y_color = y;
        self.z_color = z;
        self
    }
}

/// Debug grid
#[derive(Clone, Copy, Debug)]
pub struct DebugGrid {
    /// Center
    pub center: [f32; 3],
    /// Normal
    pub normal: [f32; 3],
    /// Size
    pub size: f32,
    /// Divisions
    pub divisions: u32,
    /// Main color
    pub color: DebugColor,
    /// Axis color (center lines)
    pub axis_color: DebugColor,
    /// Duration
    pub duration: f32,
    /// Depth test
    pub depth_test: bool,
}

impl DebugGrid {
    /// Creates grid
    pub fn new(center: [f32; 3], size: f32, divisions: u32, color: DebugColor) -> Self {
        Self {
            center,
            normal: [0.0, 1.0, 0.0],
            size,
            divisions,
            color,
            axis_color: color,
            duration: 0.0,
            depth_test: true,
        }
    }

    /// Ground grid
    pub fn ground(size: f32, divisions: u32) -> Self {
        Self::new([0.0, 0.0, 0.0], size, divisions, DebugColor::GRAY)
            .with_axis_color(DebugColor::DARK_GRAY)
    }

    /// With axis color
    pub fn with_axis_color(mut self, color: DebugColor) -> Self {
        self.axis_color = color;
        self
    }

    /// With normal
    pub fn with_normal(mut self, normal: [f32; 3]) -> Self {
        self.normal = normal;
        self
    }
}

/// Debug text
#[derive(Clone, Debug)]
pub struct DebugText {
    /// Text content
    pub text: String,
    /// Position
    pub position: TextPosition,
    /// Color
    pub color: DebugColor,
    /// Font size
    pub font_size: f32,
    /// Duration
    pub duration: f32,
}

impl DebugText {
    /// Creates 3D text
    pub fn world(text: &str, position: [f32; 3], color: DebugColor) -> Self {
        Self {
            text: String::from(text),
            position: TextPosition::World(position),
            color,
            font_size: 14.0,
            duration: 0.0,
        }
    }

    /// Creates 2D screen text
    pub fn screen(text: &str, x: f32, y: f32, color: DebugColor) -> Self {
        Self {
            text: String::from(text),
            position: TextPosition::Screen([x, y]),
            color,
            font_size: 14.0,
            duration: 0.0,
        }
    }

    /// With font size
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// With duration
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }
}

/// Text position type
#[derive(Clone, Copy, Debug)]
pub enum TextPosition {
    /// World space
    World([f32; 3]),
    /// Screen space
    Screen([f32; 2]),
}

/// Debug bone
#[derive(Clone, Copy, Debug)]
pub struct DebugBone {
    /// Start (joint)
    pub start: [f32; 3],
    /// End (child joint)
    pub end: [f32; 3],
    /// Color
    pub color: DebugColor,
    /// Bone width
    pub width: f32,
    /// Duration
    pub duration: f32,
    /// Depth test
    pub depth_test: bool,
}

impl DebugBone {
    /// Creates bone
    pub fn new(start: [f32; 3], end: [f32; 3], color: DebugColor) -> Self {
        Self {
            start,
            end,
            color,
            width: 0.02,
            duration: 0.0,
            depth_test: true,
        }
    }

    /// With width
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
}

/// Debug bounds (AABB)
#[derive(Clone, Copy, Debug)]
pub struct DebugBounds {
    /// Min corner
    pub min: [f32; 3],
    /// Max corner
    pub max: [f32; 3],
    /// Color
    pub color: DebugColor,
    /// Duration
    pub duration: f32,
    /// Depth test
    pub depth_test: bool,
}

impl DebugBounds {
    /// Creates bounds
    pub fn new(min: [f32; 3], max: [f32; 3], color: DebugColor) -> Self {
        Self {
            min,
            max,
            color,
            duration: 0.0,
            depth_test: true,
        }
    }

    /// Creates from center and size
    pub fn from_center_size(center: [f32; 3], size: [f32; 3], color: DebugColor) -> Self {
        Self::new(
            [center[0] - size[0] * 0.5, center[1] - size[1] * 0.5, center[2] - size[2] * 0.5],
            [center[0] + size[0] * 0.5, center[1] + size[1] * 0.5, center[2] + size[2] * 0.5],
            color,
        )
    }

    /// With duration
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }
}

// ============================================================================
// Debug Draw Buffer
// ============================================================================

/// Debug draw buffer
#[derive(Clone, Debug, Default)]
pub struct DebugDrawBuffer {
    /// Commands
    pub commands: Vec<DebugDrawCommand>,
    /// Persistent commands (with duration)
    pub persistent: Vec<(DebugDrawCommand, f32)>,
}

impl DebugDrawBuffer {
    /// Creates new buffer
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears immediate commands
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// Updates persistent commands
    pub fn update(&mut self, delta_time: f32) {
        self.persistent.retain_mut(|(_, remaining)| {
            *remaining -= delta_time;
            *remaining > 0.0
        });
    }

    /// Adds command
    pub fn push(&mut self, command: DebugDrawCommand) {
        self.commands.push(command);
    }

    /// Adds persistent command
    pub fn push_persistent(&mut self, command: DebugDrawCommand, duration: f32) {
        self.persistent.push((command, duration));
    }

    /// Total command count
    pub fn command_count(&self) -> usize {
        self.commands.len() + self.persistent.len()
    }

    /// Draws line
    pub fn line(&mut self, start: [f32; 3], end: [f32; 3], color: DebugColor) {
        self.push(DebugDrawCommand::Line(DebugLine::new(start, end, color)));
    }

    /// Draws point
    pub fn point(&mut self, position: [f32; 3], color: DebugColor) {
        self.push(DebugDrawCommand::Point(DebugPoint::new(position, color)));
    }

    /// Draws box
    pub fn debug_box(&mut self, center: [f32; 3], size: [f32; 3], color: DebugColor) {
        self.push(DebugDrawCommand::Box(DebugBox::new(center, size, color)));
    }

    /// Draws sphere
    pub fn sphere(&mut self, center: [f32; 3], radius: f32, color: DebugColor) {
        self.push(DebugDrawCommand::Sphere(DebugSphere::new(center, radius, color)));
    }

    /// Draws arrow
    pub fn arrow(&mut self, start: [f32; 3], end: [f32; 3], color: DebugColor) {
        self.push(DebugDrawCommand::Arrow(DebugArrow::new(start, end, color)));
    }

    /// Draws axis
    pub fn axis(&mut self, origin: [f32; 3], scale: f32) {
        self.push(DebugDrawCommand::Axis(DebugAxis::new(origin, scale)));
    }

    /// Draws grid
    pub fn grid(&mut self, center: [f32; 3], size: f32, divisions: u32, color: DebugColor) {
        self.push(DebugDrawCommand::Grid(DebugGrid::new(center, size, divisions, color)));
    }

    /// Draws text
    pub fn text_world(&mut self, text: &str, position: [f32; 3], color: DebugColor) {
        self.push(DebugDrawCommand::Text(DebugText::world(text, position, color)));
    }

    /// Draws screen text
    pub fn text_screen(&mut self, text: &str, x: f32, y: f32, color: DebugColor) {
        self.push(DebugDrawCommand::Text(DebugText::screen(text, x, y, color)));
    }
}

// ============================================================================
// GPU Debug Vertex
// ============================================================================

/// GPU debug vertex
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DebugVertex {
    /// Position
    pub position: [f32; 3],
    /// Color
    pub color: [f32; 4],
}

impl DebugVertex {
    /// Creates vertex
    pub const fn new(position: [f32; 3], color: [f32; 4]) -> Self {
        Self { position, color }
    }
}

/// Debug draw batch for GPU rendering
#[derive(Clone, Debug, Default)]
pub struct DebugDrawBatch {
    /// Line vertices
    pub line_vertices: Vec<DebugVertex>,
    /// Triangle vertices
    pub triangle_vertices: Vec<DebugVertex>,
    /// Point vertices
    pub point_vertices: Vec<DebugVertex>,
}

impl DebugDrawBatch {
    /// Creates new batch
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears batch
    pub fn clear(&mut self) {
        self.line_vertices.clear();
        self.triangle_vertices.clear();
        self.point_vertices.clear();
    }

    /// Adds line
    pub fn add_line(&mut self, start: [f32; 3], end: [f32; 3], color: [f32; 4]) {
        self.line_vertices.push(DebugVertex::new(start, color));
        self.line_vertices.push(DebugVertex::new(end, color));
    }

    /// Adds triangle
    pub fn add_triangle(&mut self, v0: [f32; 3], v1: [f32; 3], v2: [f32; 3], color: [f32; 4]) {
        self.triangle_vertices.push(DebugVertex::new(v0, color));
        self.triangle_vertices.push(DebugVertex::new(v1, color));
        self.triangle_vertices.push(DebugVertex::new(v2, color));
    }

    /// Adds point
    pub fn add_point(&mut self, position: [f32; 3], color: [f32; 4]) {
        self.point_vertices.push(DebugVertex::new(position, color));
    }

    /// Total vertex count
    pub fn vertex_count(&self) -> usize {
        self.line_vertices.len() + self.triangle_vertices.len() + self.point_vertices.len()
    }
}
