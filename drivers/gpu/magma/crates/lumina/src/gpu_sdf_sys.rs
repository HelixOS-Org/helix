//! GPU Signed Distance Field (SDF) System for Lumina
//!
//! This module provides comprehensive GPU-accelerated SDF rendering including
//! volumetric shapes, text rendering, CSG operations, and ray marching.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// SDF System Handles
// ============================================================================

/// GPU SDF system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuSdfSystemHandle(pub u64);

impl GpuSdfSystemHandle {
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

impl Default for GpuSdfSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// SDF volume handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SdfVolumeHandle(pub u64);

impl SdfVolumeHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Is null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for SdfVolumeHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// SDF primitive handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SdfPrimitiveHandle(pub u64);

impl SdfPrimitiveHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SdfPrimitiveHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// SDF font handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SdfFontHandle(pub u64);

impl SdfFontHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SdfFontHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// SDF System Creation
// ============================================================================

/// GPU SDF system create info
#[derive(Clone, Debug)]
pub struct GpuSdfSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max volumes
    pub max_volumes: u32,
    /// Max primitives
    pub max_primitives: u32,
    /// Max fonts
    pub max_fonts: u32,
    /// Features
    pub features: SdfFeatures,
    /// Ray marcher settings
    pub ray_marcher: RayMarcherSettings,
}

impl GpuSdfSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_volumes: 1000,
            max_primitives: 10000,
            max_fonts: 64,
            features: SdfFeatures::all(),
            ray_marcher: RayMarcherSettings::default(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max volumes
    pub fn with_max_volumes(mut self, count: u32) -> Self {
        self.max_volumes = count;
        self
    }

    /// With max primitives
    pub fn with_max_primitives(mut self, count: u32) -> Self {
        self.max_primitives = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: SdfFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With ray marcher
    pub fn with_ray_marcher(mut self, settings: RayMarcherSettings) -> Self {
        self.ray_marcher = settings;
        self
    }

    /// Standard preset
    pub fn standard() -> Self {
        Self::new()
    }

    /// High quality preset
    pub fn high_quality() -> Self {
        Self::new().with_ray_marcher(RayMarcherSettings::high_quality())
    }

    /// Mobile preset
    pub fn mobile() -> Self {
        Self::new()
            .with_max_volumes(100)
            .with_max_primitives(1000)
            .with_features(SdfFeatures::BASIC)
            .with_ray_marcher(RayMarcherSettings::mobile())
    }
}

impl Default for GpuSdfSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// SDF features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct SdfFeatures: u32 {
        /// None
        const NONE = 0;
        /// Basic primitives
        const PRIMITIVES = 1 << 0;
        /// CSG operations
        const CSG = 1 << 1;
        /// Smooth blending
        const SMOOTH_BLEND = 1 << 2;
        /// Ray marching
        const RAY_MARCH = 1 << 3;
        /// SDF text
        const TEXT = 1 << 4;
        /// Animated SDFs
        const ANIMATION = 1 << 5;
        /// Ambient occlusion
        const AO = 1 << 6;
        /// Soft shadows
        const SOFT_SHADOWS = 1 << 7;
        /// Basic
        const BASIC = Self::PRIMITIVES.bits() | Self::RAY_MARCH.bits();
        /// All
        const ALL = 0xFF;
    }
}

impl Default for SdfFeatures {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Ray Marcher Settings
// ============================================================================

/// Ray marcher settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RayMarcherSettings {
    /// Max steps
    pub max_steps: u32,
    /// Max distance
    pub max_distance: f32,
    /// Surface threshold (epsilon)
    pub surface_threshold: f32,
    /// Step scale factor
    pub step_scale: f32,
    /// Relaxation factor
    pub relaxation: f32,
    /// Normal epsilon
    pub normal_epsilon: f32,
    /// AO steps
    pub ao_steps: u32,
    /// Shadow steps
    pub shadow_steps: u32,
}

impl RayMarcherSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            max_steps: 128,
            max_distance: 100.0,
            surface_threshold: 0.001,
            step_scale: 1.0,
            relaxation: 1.0,
            normal_epsilon: 0.001,
            ao_steps: 8,
            shadow_steps: 32,
        }
    }

    /// With max steps
    pub const fn with_max_steps(mut self, steps: u32) -> Self {
        self.max_steps = steps;
        self
    }

    /// With max distance
    pub const fn with_max_distance(mut self, dist: f32) -> Self {
        self.max_distance = dist;
        self
    }

    /// With surface threshold
    pub const fn with_threshold(mut self, threshold: f32) -> Self {
        self.surface_threshold = threshold;
        self
    }

    /// High quality preset
    pub const fn high_quality() -> Self {
        Self {
            max_steps: 256,
            max_distance: 200.0,
            surface_threshold: 0.0001,
            step_scale: 0.9,
            relaxation: 1.0,
            normal_epsilon: 0.0001,
            ao_steps: 16,
            shadow_steps: 64,
        }
    }

    /// Mobile preset
    pub const fn mobile() -> Self {
        Self {
            max_steps: 48,
            max_distance: 50.0,
            surface_threshold: 0.01,
            step_scale: 1.2,
            relaxation: 1.5,
            normal_epsilon: 0.01,
            ao_steps: 4,
            shadow_steps: 16,
        }
    }
}

impl Default for RayMarcherSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SDF Primitives
// ============================================================================

/// SDF primitive type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SdfPrimitiveType {
    /// Sphere
    #[default]
    Sphere     = 0,
    /// Box
    Box        = 1,
    /// Rounded box
    RoundedBox = 2,
    /// Torus
    Torus      = 3,
    /// Cylinder
    Cylinder   = 4,
    /// Cone
    Cone       = 5,
    /// Capsule
    Capsule    = 6,
    /// Plane
    Plane      = 7,
    /// Ellipsoid
    Ellipsoid  = 8,
    /// Prism
    Prism      = 9,
    /// Pyramid
    Pyramid    = 10,
    /// Octahedron
    Octahedron = 11,
}

/// SDF primitive create info
#[derive(Clone, Debug)]
pub struct SdfPrimitiveCreateInfo {
    /// Primitive type
    pub primitive_type: SdfPrimitiveType,
    /// Position
    pub position: [f32; 3],
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Scale
    pub scale: [f32; 3],
    /// Parameters (type-specific)
    pub params: SdfParams,
    /// Material
    pub material: SdfMaterial,
}

impl SdfPrimitiveCreateInfo {
    /// Creates new sphere
    pub fn sphere(radius: f32) -> Self {
        Self {
            primitive_type: SdfPrimitiveType::Sphere,
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            params: SdfParams::sphere(radius),
            material: SdfMaterial::default(),
        }
    }

    /// Creates new box
    pub fn cube(size: f32) -> Self {
        Self {
            primitive_type: SdfPrimitiveType::Box,
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            params: SdfParams::cube(size),
            material: SdfMaterial::default(),
        }
    }

    /// Creates new rounded box
    pub fn rounded_box(half_extents: [f32; 3], radius: f32) -> Self {
        Self {
            primitive_type: SdfPrimitiveType::RoundedBox,
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            params: SdfParams::rounded_box(half_extents, radius),
            material: SdfMaterial::default(),
        }
    }

    /// Creates new torus
    pub fn torus(major_radius: f32, minor_radius: f32) -> Self {
        Self {
            primitive_type: SdfPrimitiveType::Torus,
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            params: SdfParams::torus(major_radius, minor_radius),
            material: SdfMaterial::default(),
        }
    }

    /// Creates new cylinder
    pub fn cylinder(radius: f32, height: f32) -> Self {
        Self {
            primitive_type: SdfPrimitiveType::Cylinder,
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            params: SdfParams::cylinder(radius, height),
            material: SdfMaterial::default(),
        }
    }

    /// Creates new capsule
    pub fn capsule(radius: f32, height: f32) -> Self {
        Self {
            primitive_type: SdfPrimitiveType::Capsule,
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            params: SdfParams::capsule(radius, height),
            material: SdfMaterial::default(),
        }
    }

    /// With position
    pub fn at(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = [x, y, z];
        self
    }

    /// With rotation
    pub fn with_rotation(mut self, rotation: [f32; 4]) -> Self {
        self.rotation = rotation;
        self
    }

    /// With scale
    pub fn with_scale(mut self, scale: [f32; 3]) -> Self {
        self.scale = scale;
        self
    }

    /// With material
    pub fn with_material(mut self, material: SdfMaterial) -> Self {
        self.material = material;
        self
    }
}

impl Default for SdfPrimitiveCreateInfo {
    fn default() -> Self {
        Self::sphere(1.0)
    }
}

/// SDF parameters
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SdfParams {
    /// Parameter values
    pub values: [f32; 4],
}

impl SdfParams {
    /// Sphere params
    pub const fn sphere(radius: f32) -> Self {
        Self {
            values: [radius, 0.0, 0.0, 0.0],
        }
    }

    /// Cube params
    pub const fn cube(size: f32) -> Self {
        let half = size * 0.5;
        Self {
            values: [half, half, half, 0.0],
        }
    }

    /// Box params
    pub const fn box3d(half_extents: [f32; 3]) -> Self {
        Self {
            values: [half_extents[0], half_extents[1], half_extents[2], 0.0],
        }
    }

    /// Rounded box params
    pub const fn rounded_box(half_extents: [f32; 3], radius: f32) -> Self {
        Self {
            values: [half_extents[0], half_extents[1], half_extents[2], radius],
        }
    }

    /// Torus params
    pub const fn torus(major_radius: f32, minor_radius: f32) -> Self {
        Self {
            values: [major_radius, minor_radius, 0.0, 0.0],
        }
    }

    /// Cylinder params
    pub const fn cylinder(radius: f32, height: f32) -> Self {
        Self {
            values: [radius, height * 0.5, 0.0, 0.0],
        }
    }

    /// Capsule params
    pub const fn capsule(radius: f32, height: f32) -> Self {
        Self {
            values: [radius, height * 0.5, 0.0, 0.0],
        }
    }

    /// Cone params
    pub const fn cone(radius: f32, height: f32) -> Self {
        Self {
            values: [radius, height, 0.0, 0.0],
        }
    }
}

impl Default for SdfParams {
    fn default() -> Self {
        Self::sphere(1.0)
    }
}

// ============================================================================
// SDF Material
// ============================================================================

/// SDF material
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SdfMaterial {
    /// Albedo color
    pub albedo: [f32; 4],
    /// Roughness
    pub roughness: f32,
    /// Metallic
    pub metallic: f32,
    /// Emission strength
    pub emission: f32,
    /// Specular
    pub specular: f32,
}

impl SdfMaterial {
    /// Creates new material
    pub const fn new(color: [f32; 4]) -> Self {
        Self {
            albedo: color,
            roughness: 0.5,
            metallic: 0.0,
            emission: 0.0,
            specular: 0.5,
        }
    }

    /// With roughness
    pub const fn with_roughness(mut self, roughness: f32) -> Self {
        self.roughness = roughness;
        self
    }

    /// With metallic
    pub const fn with_metallic(mut self, metallic: f32) -> Self {
        self.metallic = metallic;
        self
    }

    /// With emission
    pub const fn with_emission(mut self, emission: f32) -> Self {
        self.emission = emission;
        self
    }

    /// White material
    pub const fn white() -> Self {
        Self::new([1.0, 1.0, 1.0, 1.0])
    }

    /// Red material
    pub const fn red() -> Self {
        Self::new([1.0, 0.0, 0.0, 1.0])
    }

    /// Green material
    pub const fn green() -> Self {
        Self::new([0.0, 1.0, 0.0, 1.0])
    }

    /// Blue material
    pub const fn blue() -> Self {
        Self::new([0.0, 0.0, 1.0, 1.0])
    }

    /// Metal material
    pub const fn metal() -> Self {
        Self {
            albedo: [0.8, 0.8, 0.85, 1.0],
            roughness: 0.2,
            metallic: 1.0,
            emission: 0.0,
            specular: 1.0,
        }
    }

    /// Plastic material
    pub const fn plastic() -> Self {
        Self {
            albedo: [0.8, 0.1, 0.1, 1.0],
            roughness: 0.4,
            metallic: 0.0,
            emission: 0.0,
            specular: 0.5,
        }
    }

    /// Emissive material
    pub const fn emissive(color: [f32; 3], strength: f32) -> Self {
        Self {
            albedo: [color[0], color[1], color[2], 1.0],
            roughness: 0.5,
            metallic: 0.0,
            emission: strength,
            specular: 0.0,
        }
    }
}

impl Default for SdfMaterial {
    fn default() -> Self {
        Self::white()
    }
}

// ============================================================================
// CSG Operations
// ============================================================================

/// CSG operation type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CsgOperation {
    /// Union
    #[default]
    Union           = 0,
    /// Subtraction
    Subtract        = 1,
    /// Intersection
    Intersect       = 2,
    /// Smooth union
    SmoothUnion     = 3,
    /// Smooth subtraction
    SmoothSubtract  = 4,
    /// Smooth intersection
    SmoothIntersect = 5,
}

/// CSG node
#[derive(Clone, Debug)]
pub struct CsgNode {
    /// Node type
    pub node_type: CsgNodeType,
    /// Operation (for operation nodes)
    pub operation: CsgOperation,
    /// Smoothness factor (for smooth operations)
    pub smoothness: f32,
    /// Primitive index (for primitive nodes)
    pub primitive_index: u32,
    /// Children indices
    pub children: [i32; 2],
}

impl CsgNode {
    /// Creates primitive node
    pub fn primitive(index: u32) -> Self {
        Self {
            node_type: CsgNodeType::Primitive,
            operation: CsgOperation::Union,
            smoothness: 0.0,
            primitive_index: index,
            children: [-1, -1],
        }
    }

    /// Creates operation node
    pub fn operation(op: CsgOperation, left: i32, right: i32) -> Self {
        Self {
            node_type: CsgNodeType::Operation,
            operation: op,
            smoothness: 0.0,
            primitive_index: 0,
            children: [left, right],
        }
    }

    /// Creates smooth operation node
    pub fn smooth_operation(op: CsgOperation, left: i32, right: i32, smoothness: f32) -> Self {
        Self {
            node_type: CsgNodeType::Operation,
            operation: op,
            smoothness,
            primitive_index: 0,
            children: [left, right],
        }
    }
}

impl Default for CsgNode {
    fn default() -> Self {
        Self::primitive(0)
    }
}

/// CSG node type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CsgNodeType {
    /// Primitive leaf
    #[default]
    Primitive = 0,
    /// Operation node
    Operation = 1,
}

// ============================================================================
// SDF Volume
// ============================================================================

/// SDF volume create info
#[derive(Clone, Debug)]
pub struct SdfVolumeCreateInfo {
    /// Name
    pub name: String,
    /// Primitives
    pub primitives: Vec<SdfPrimitiveCreateInfo>,
    /// CSG tree (optional)
    pub csg_tree: Option<Vec<CsgNode>>,
    /// Bounds
    pub bounds: SdfBounds,
    /// Volume settings
    pub settings: SdfVolumeSettings,
}

impl SdfVolumeCreateInfo {
    /// Creates new volume
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            primitives: Vec::new(),
            csg_tree: None,
            bounds: SdfBounds::default(),
            settings: SdfVolumeSettings::default(),
        }
    }

    /// Add primitive
    pub fn add_primitive(mut self, primitive: SdfPrimitiveCreateInfo) -> Self {
        self.primitives.push(primitive);
        self
    }

    /// With CSG tree
    pub fn with_csg(mut self, tree: Vec<CsgNode>) -> Self {
        self.csg_tree = Some(tree);
        self
    }

    /// With bounds
    pub fn with_bounds(mut self, bounds: SdfBounds) -> Self {
        self.bounds = bounds;
        self
    }

    /// With settings
    pub fn with_settings(mut self, settings: SdfVolumeSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Single sphere
    pub fn single_sphere(name: impl Into<String>, radius: f32) -> Self {
        Self::new(name).add_primitive(SdfPrimitiveCreateInfo::sphere(radius))
    }

    /// CSG example (sphere minus box)
    pub fn csg_sphere_minus_box(name: impl Into<String>) -> Self {
        Self::new(name)
            .add_primitive(SdfPrimitiveCreateInfo::sphere(1.0))
            .add_primitive(SdfPrimitiveCreateInfo::cube(1.2))
            .with_csg(vec![
                CsgNode::primitive(0),
                CsgNode::primitive(1),
                CsgNode::operation(CsgOperation::Subtract, 0, 1),
            ])
    }
}

impl Default for SdfVolumeCreateInfo {
    fn default() -> Self {
        Self::new("Volume")
    }
}

/// SDF bounds
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SdfBounds {
    /// Min corner
    pub min: [f32; 3],
    /// Max corner
    pub max: [f32; 3],
}

impl SdfBounds {
    /// Creates new bounds
    pub const fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    /// Unit bounds
    pub const fn unit() -> Self {
        Self {
            min: [-1.0, -1.0, -1.0],
            max: [1.0, 1.0, 1.0],
        }
    }

    /// Large bounds
    pub const fn large() -> Self {
        Self {
            min: [-100.0, -100.0, -100.0],
            max: [100.0, 100.0, 100.0],
        }
    }

    /// Center
    pub const fn center(&self) -> [f32; 3] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ]
    }

    /// Size
    pub const fn size(&self) -> [f32; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }
}

impl Default for SdfBounds {
    fn default() -> Self {
        Self::unit()
    }
}

/// SDF volume settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SdfVolumeSettings {
    /// Enable shadows
    pub shadows: bool,
    /// Enable ambient occlusion
    pub ambient_occlusion: bool,
    /// AO strength
    pub ao_strength: f32,
    /// Shadow softness
    pub shadow_softness: f32,
    /// Bias
    pub bias: f32,
}

impl SdfVolumeSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            shadows: true,
            ambient_occlusion: true,
            ao_strength: 0.5,
            shadow_softness: 8.0,
            bias: 0.001,
        }
    }

    /// High quality
    pub const fn high_quality() -> Self {
        Self {
            shadows: true,
            ambient_occlusion: true,
            ao_strength: 0.7,
            shadow_softness: 16.0,
            bias: 0.0001,
        }
    }

    /// Mobile
    pub const fn mobile() -> Self {
        Self {
            shadows: false,
            ambient_occlusion: false,
            ao_strength: 0.3,
            shadow_softness: 4.0,
            bias: 0.01,
        }
    }
}

impl Default for SdfVolumeSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SDF Text
// ============================================================================

/// SDF font create info
#[derive(Clone, Debug)]
pub struct SdfFontCreateInfo {
    /// Font name
    pub name: String,
    /// Texture resolution
    pub resolution: u32,
    /// SDF spread
    pub spread: f32,
    /// Font size
    pub font_size: f32,
}

impl SdfFontCreateInfo {
    /// Creates new font info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            resolution: 4096,
            spread: 8.0,
            font_size: 64.0,
        }
    }

    /// With resolution
    pub fn with_resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    /// With spread
    pub fn with_spread(mut self, spread: f32) -> Self {
        self.spread = spread;
        self
    }
}

impl Default for SdfFontCreateInfo {
    fn default() -> Self {
        Self::new("Default")
    }
}

/// SDF text render info
#[derive(Clone, Debug)]
pub struct SdfTextRenderInfo {
    /// Text content
    pub text: String,
    /// Position
    pub position: [f32; 2],
    /// Font size
    pub font_size: f32,
    /// Color
    pub color: [f32; 4],
    /// Outline width
    pub outline_width: f32,
    /// Outline color
    pub outline_color: [f32; 4],
    /// Shadow offset
    pub shadow_offset: [f32; 2],
    /// Shadow color
    pub shadow_color: [f32; 4],
    /// Edge softness
    pub softness: f32,
}

impl SdfTextRenderInfo {
    /// Creates new text render info
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            position: [0.0, 0.0],
            font_size: 32.0,
            color: [1.0, 1.0, 1.0, 1.0],
            outline_width: 0.0,
            outline_color: [0.0, 0.0, 0.0, 1.0],
            shadow_offset: [0.0, 0.0],
            shadow_color: [0.0, 0.0, 0.0, 0.5],
            softness: 0.5,
        }
    }

    /// At position
    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.position = [x, y];
        self
    }

    /// With font size
    pub fn with_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// With color
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// With outline
    pub fn with_outline(mut self, width: f32, color: [f32; 4]) -> Self {
        self.outline_width = width;
        self.outline_color = color;
        self
    }

    /// With shadow
    pub fn with_shadow(mut self, offset: [f32; 2], color: [f32; 4]) -> Self {
        self.shadow_offset = offset;
        self.shadow_color = color;
        self
    }
}

impl Default for SdfTextRenderInfo {
    fn default() -> Self {
        Self::new("")
    }
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// GPU SDF primitive
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuSdfPrimitive {
    /// Position
    pub position: [f32; 3],
    /// Primitive type
    pub primitive_type: u32,
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Scale
    pub scale: [f32; 3],
    /// Pad
    pub _pad0: f32,
    /// Params
    pub params: [f32; 4],
    /// Material index
    pub material_index: u32,
    /// Flags
    pub flags: u32,
    /// Pad
    pub _pad1: [f32; 2],
}

/// GPU SDF material
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuSdfMaterial {
    /// Albedo
    pub albedo: [f32; 4],
    /// Roughness
    pub roughness: f32,
    /// Metallic
    pub metallic: f32,
    /// Emission
    pub emission: f32,
    /// Specular
    pub specular: f32,
}

/// GPU SDF constants
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuSdfConstants {
    /// Camera position
    pub camera_position: [f32; 3],
    /// Max steps
    pub max_steps: u32,
    /// Max distance
    pub max_distance: f32,
    /// Surface threshold
    pub surface_threshold: f32,
    /// Step scale
    pub step_scale: f32,
    /// Normal epsilon
    pub normal_epsilon: f32,
    /// AO steps
    pub ao_steps: u32,
    /// Shadow steps
    pub shadow_steps: u32,
    /// Time
    pub time: f32,
    /// Flags
    pub flags: u32,
    /// Primitive count
    pub primitive_count: u32,
    /// CSG root
    pub csg_root: i32,
    /// Pad
    pub _pad: [f32; 2],
}

impl Default for GpuSdfConstants {
    fn default() -> Self {
        Self {
            camera_position: [0.0; 3],
            max_steps: 128,
            max_distance: 100.0,
            surface_threshold: 0.001,
            step_scale: 1.0,
            normal_epsilon: 0.001,
            ao_steps: 8,
            shadow_steps: 32,
            time: 0.0,
            flags: 0,
            primitive_count: 0,
            csg_root: -1,
            _pad: [0.0; 2],
        }
    }
}

/// GPU CSG node
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuCsgNode {
    /// Node type
    pub node_type: u32,
    /// Operation
    pub operation: u32,
    /// Primitive index
    pub primitive_index: u32,
    /// Smoothness
    pub smoothness: f32,
    /// Left child
    pub left_child: i32,
    /// Right child
    pub right_child: i32,
    /// Pad
    pub _pad: [f32; 2],
}

// ============================================================================
// SDF Statistics
// ============================================================================

/// SDF system statistics
#[derive(Clone, Debug, Default)]
pub struct GpuSdfStats {
    /// Total volumes
    pub total_volumes: u32,
    /// Total primitives
    pub total_primitives: u32,
    /// Total CSG nodes
    pub total_csg_nodes: u32,
    /// Average ray march steps
    pub avg_march_steps: f32,
    /// Max ray march steps
    pub max_march_steps: u32,
    /// Render time (ms)
    pub render_time_ms: f32,
    /// Pixels rendered
    pub pixels_rendered: u64,
    /// Cache hits
    pub cache_hits: u64,
}

impl GpuSdfStats {
    /// Rays per second (millions)
    pub fn rays_per_second_millions(&self) -> f32 {
        if self.render_time_ms > 0.0 {
            (self.pixels_rendered as f32 / self.render_time_ms) / 1000.0
        } else {
            0.0
        }
    }

    /// Cache hit ratio
    pub fn cache_hit_ratio(&self) -> f32 {
        let total = self.cache_hits + self.pixels_rendered;
        if total > 0 {
            self.cache_hits as f32 / total as f32
        } else {
            0.0
        }
    }
}
