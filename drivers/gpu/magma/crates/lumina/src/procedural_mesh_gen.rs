//! Procedural Mesh Generation Types for Lumina
//!
//! This module provides procedural geometry generation infrastructure
//! for runtime mesh creation and modification.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Procedural Mesh Handles
// ============================================================================

/// Procedural mesh generator handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ProceduralMeshHandle(pub u64);

impl ProceduralMeshHandle {
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

impl Default for ProceduralMeshHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Generated mesh handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GeneratedMeshHandle(pub u64);

impl GeneratedMeshHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for GeneratedMeshHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Mesh builder handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MeshBuilderHandle(pub u64);

impl MeshBuilderHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for MeshBuilderHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Procedural Mesh System
// ============================================================================

/// Procedural mesh system create info
#[derive(Clone, Debug)]
pub struct ProceduralMeshSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max generated meshes
    pub max_meshes: u32,
    /// Max vertices per mesh
    pub max_vertices: u32,
    /// Max indices per mesh
    pub max_indices: u32,
    /// Features
    pub features: ProceduralMeshFeatures,
}

impl ProceduralMeshSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_meshes: 256,
            max_vertices: 65536,
            max_indices: 262144,
            features: ProceduralMeshFeatures::all(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max meshes
    pub fn with_max_meshes(mut self, count: u32) -> Self {
        self.max_meshes = count;
        self
    }

    /// With max vertices
    pub fn with_max_vertices(mut self, count: u32) -> Self {
        self.max_vertices = count;
        self
    }

    /// With max indices
    pub fn with_max_indices(mut self, count: u32) -> Self {
        self.max_indices = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: ProceduralMeshFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Standard system
    pub fn standard() -> Self {
        Self::new()
    }

    /// High capacity
    pub fn high_capacity() -> Self {
        Self::new()
            .with_max_meshes(1024)
            .with_max_vertices(1048576)
            .with_max_indices(4194304)
    }
}

impl Default for ProceduralMeshSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Procedural mesh features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct ProceduralMeshFeatures: u32 {
        /// None
        const NONE = 0;
        /// Normals generation
        const NORMALS = 1 << 0;
        /// Tangents generation
        const TANGENTS = 1 << 1;
        /// UV generation
        const UVS = 1 << 2;
        /// LOD generation
        const LOD = 1 << 3;
        /// GPU generation
        const GPU_GENERATION = 1 << 4;
        /// Dynamic update
        const DYNAMIC = 1 << 5;
        /// All
        const ALL = 0x3F;
    }
}

impl Default for ProceduralMeshFeatures {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Primitive Generation
// ============================================================================

/// Primitive mesh type
#[derive(Clone, Debug)]
pub enum PrimitiveMesh {
    /// Box/Cube
    Box(BoxParams),
    /// Sphere
    Sphere(SphereParams),
    /// Cylinder
    Cylinder(CylinderParams),
    /// Cone
    Cone(ConeParams),
    /// Capsule
    Capsule(CapsuleParams),
    /// Torus
    Torus(TorusParams),
    /// Plane
    Plane(PlaneParams),
    /// Disc
    Disc(DiscParams),
    /// Grid
    Grid(GridParams),
    /// Arrow
    Arrow(ArrowParams),
    /// Custom
    Custom(CustomMeshParams),
}

impl Default for PrimitiveMesh {
    fn default() -> Self {
        Self::Box(BoxParams::default())
    }
}

/// Box parameters
#[derive(Clone, Copy, Debug)]
pub struct BoxParams {
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
    /// Depth
    pub depth: f32,
    /// Width segments
    pub width_segments: u32,
    /// Height segments
    pub height_segments: u32,
    /// Depth segments
    pub depth_segments: u32,
}

impl BoxParams {
    /// Creates unit box
    pub const fn unit() -> Self {
        Self {
            width: 1.0,
            height: 1.0,
            depth: 1.0,
            width_segments: 1,
            height_segments: 1,
            depth_segments: 1,
        }
    }

    /// Creates cube
    pub const fn cube(size: f32) -> Self {
        Self {
            width: size,
            height: size,
            depth: size,
            width_segments: 1,
            height_segments: 1,
            depth_segments: 1,
        }
    }

    /// With size
    pub const fn with_size(mut self, w: f32, h: f32, d: f32) -> Self {
        self.width = w;
        self.height = h;
        self.depth = d;
        self
    }

    /// With segments
    pub const fn with_segments(mut self, w: u32, h: u32, d: u32) -> Self {
        self.width_segments = w;
        self.height_segments = h;
        self.depth_segments = d;
        self
    }
}

impl Default for BoxParams {
    fn default() -> Self {
        Self::unit()
    }
}

/// Sphere parameters
#[derive(Clone, Copy, Debug)]
pub struct SphereParams {
    /// Radius
    pub radius: f32,
    /// Horizontal segments
    pub segments: u32,
    /// Vertical rings
    pub rings: u32,
    /// Phi start
    pub phi_start: f32,
    /// Phi length
    pub phi_length: f32,
    /// Theta start
    pub theta_start: f32,
    /// Theta length
    pub theta_length: f32,
}

impl SphereParams {
    /// Unit sphere
    pub const fn unit() -> Self {
        Self {
            radius: 1.0,
            segments: 32,
            rings: 16,
            phi_start: 0.0,
            phi_length: core::f32::consts::TAU,
            theta_start: 0.0,
            theta_length: core::f32::consts::PI,
        }
    }

    /// With radius
    pub const fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// With detail
    pub const fn with_detail(mut self, segments: u32, rings: u32) -> Self {
        self.segments = segments;
        self.rings = rings;
        self
    }

    /// Low detail
    pub const fn low_detail() -> Self {
        Self::unit().with_detail(16, 8)
    }

    /// High detail
    pub const fn high_detail() -> Self {
        Self::unit().with_detail(64, 32)
    }
}

impl Default for SphereParams {
    fn default() -> Self {
        Self::unit()
    }
}

/// Cylinder parameters
#[derive(Clone, Copy, Debug)]
pub struct CylinderParams {
    /// Top radius
    pub top_radius: f32,
    /// Bottom radius
    pub bottom_radius: f32,
    /// Height
    pub height: f32,
    /// Radial segments
    pub radial_segments: u32,
    /// Height segments
    pub height_segments: u32,
    /// Open ended
    pub open_ended: bool,
    /// Theta start
    pub theta_start: f32,
    /// Theta length
    pub theta_length: f32,
}

impl CylinderParams {
    /// Unit cylinder
    pub const fn unit() -> Self {
        Self {
            top_radius: 0.5,
            bottom_radius: 0.5,
            height: 1.0,
            radial_segments: 32,
            height_segments: 1,
            open_ended: false,
            theta_start: 0.0,
            theta_length: core::f32::consts::TAU,
        }
    }

    /// With radii
    pub const fn with_radii(mut self, top: f32, bottom: f32) -> Self {
        self.top_radius = top;
        self.bottom_radius = bottom;
        self
    }

    /// With height
    pub const fn with_height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// With segments
    pub const fn with_segments(mut self, radial: u32, height: u32) -> Self {
        self.radial_segments = radial;
        self.height_segments = height;
        self
    }

    /// Open ended
    pub const fn open(mut self) -> Self {
        self.open_ended = true;
        self
    }
}

impl Default for CylinderParams {
    fn default() -> Self {
        Self::unit()
    }
}

/// Cone parameters
#[derive(Clone, Copy, Debug)]
pub struct ConeParams {
    /// Radius
    pub radius: f32,
    /// Height
    pub height: f32,
    /// Radial segments
    pub radial_segments: u32,
    /// Height segments
    pub height_segments: u32,
    /// Open ended
    pub open_ended: bool,
}

impl ConeParams {
    /// Unit cone
    pub const fn unit() -> Self {
        Self {
            radius: 0.5,
            height: 1.0,
            radial_segments: 32,
            height_segments: 1,
            open_ended: false,
        }
    }

    /// With radius
    pub const fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// With height
    pub const fn with_height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl Default for ConeParams {
    fn default() -> Self {
        Self::unit()
    }
}

/// Capsule parameters
#[derive(Clone, Copy, Debug)]
pub struct CapsuleParams {
    /// Radius
    pub radius: f32,
    /// Cylinder length (not including caps)
    pub length: f32,
    /// Radial segments
    pub radial_segments: u32,
    /// Cap segments
    pub cap_segments: u32,
    /// Length segments
    pub length_segments: u32,
}

impl CapsuleParams {
    /// Unit capsule
    pub const fn unit() -> Self {
        Self {
            radius: 0.5,
            length: 1.0,
            radial_segments: 32,
            cap_segments: 8,
            length_segments: 1,
        }
    }

    /// With radius
    pub const fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// With length
    pub const fn with_length(mut self, length: f32) -> Self {
        self.length = length;
        self
    }
}

impl Default for CapsuleParams {
    fn default() -> Self {
        Self::unit()
    }
}

/// Torus parameters
#[derive(Clone, Copy, Debug)]
pub struct TorusParams {
    /// Major radius
    pub radius: f32,
    /// Tube radius
    pub tube_radius: f32,
    /// Radial segments
    pub radial_segments: u32,
    /// Tubular segments
    pub tubular_segments: u32,
    /// Arc
    pub arc: f32,
}

impl TorusParams {
    /// Unit torus
    pub const fn unit() -> Self {
        Self {
            radius: 1.0,
            tube_radius: 0.3,
            radial_segments: 32,
            tubular_segments: 16,
            arc: core::f32::consts::TAU,
        }
    }

    /// With radii
    pub const fn with_radii(mut self, radius: f32, tube: f32) -> Self {
        self.radius = radius;
        self.tube_radius = tube;
        self
    }

    /// With segments
    pub const fn with_segments(mut self, radial: u32, tubular: u32) -> Self {
        self.radial_segments = radial;
        self.tubular_segments = tubular;
        self
    }
}

impl Default for TorusParams {
    fn default() -> Self {
        Self::unit()
    }
}

/// Plane parameters
#[derive(Clone, Copy, Debug)]
pub struct PlaneParams {
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
    /// Width segments
    pub width_segments: u32,
    /// Height segments
    pub height_segments: u32,
}

impl PlaneParams {
    /// Unit plane
    pub const fn unit() -> Self {
        Self {
            width: 1.0,
            height: 1.0,
            width_segments: 1,
            height_segments: 1,
        }
    }

    /// With size
    pub const fn with_size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// With segments
    pub const fn with_segments(mut self, w: u32, h: u32) -> Self {
        self.width_segments = w;
        self.height_segments = h;
        self
    }

    /// Ground plane
    pub const fn ground(size: f32, segments: u32) -> Self {
        Self {
            width: size,
            height: size,
            width_segments: segments,
            height_segments: segments,
        }
    }
}

impl Default for PlaneParams {
    fn default() -> Self {
        Self::unit()
    }
}

/// Disc parameters
#[derive(Clone, Copy, Debug)]
pub struct DiscParams {
    /// Outer radius
    pub radius: f32,
    /// Inner radius (0 = solid disc)
    pub inner_radius: f32,
    /// Segments
    pub segments: u32,
    /// Theta start
    pub theta_start: f32,
    /// Theta length
    pub theta_length: f32,
}

impl DiscParams {
    /// Unit disc
    pub const fn unit() -> Self {
        Self {
            radius: 1.0,
            inner_radius: 0.0,
            segments: 32,
            theta_start: 0.0,
            theta_length: core::f32::consts::TAU,
        }
    }

    /// Ring
    pub const fn ring(outer: f32, inner: f32) -> Self {
        Self {
            radius: outer,
            inner_radius: inner,
            segments: 32,
            theta_start: 0.0,
            theta_length: core::f32::consts::TAU,
        }
    }
}

impl Default for DiscParams {
    fn default() -> Self {
        Self::unit()
    }
}

/// Grid parameters
#[derive(Clone, Copy, Debug)]
pub struct GridParams {
    /// Size X
    pub size_x: f32,
    /// Size Z
    pub size_z: f32,
    /// Divisions X
    pub divisions_x: u32,
    /// Divisions Z
    pub divisions_z: u32,
    /// Center
    pub centered: bool,
}

impl GridParams {
    /// Unit grid
    pub const fn unit() -> Self {
        Self {
            size_x: 10.0,
            size_z: 10.0,
            divisions_x: 10,
            divisions_z: 10,
            centered: true,
        }
    }

    /// With size
    pub const fn with_size(mut self, x: f32, z: f32) -> Self {
        self.size_x = x;
        self.size_z = z;
        self
    }

    /// With divisions
    pub const fn with_divisions(mut self, x: u32, z: u32) -> Self {
        self.divisions_x = x;
        self.divisions_z = z;
        self
    }
}

impl Default for GridParams {
    fn default() -> Self {
        Self::unit()
    }
}

/// Arrow parameters
#[derive(Clone, Copy, Debug)]
pub struct ArrowParams {
    /// Shaft radius
    pub shaft_radius: f32,
    /// Shaft length
    pub shaft_length: f32,
    /// Head radius
    pub head_radius: f32,
    /// Head length
    pub head_length: f32,
    /// Radial segments
    pub radial_segments: u32,
}

impl ArrowParams {
    /// Unit arrow
    pub const fn unit() -> Self {
        Self {
            shaft_radius: 0.05,
            shaft_length: 0.7,
            head_radius: 0.1,
            head_length: 0.3,
            radial_segments: 16,
        }
    }

    /// With size
    pub const fn with_size(mut self, shaft_r: f32, shaft_l: f32, head_r: f32, head_l: f32) -> Self {
        self.shaft_radius = shaft_r;
        self.shaft_length = shaft_l;
        self.head_radius = head_r;
        self.head_length = head_l;
        self
    }
}

impl Default for ArrowParams {
    fn default() -> Self {
        Self::unit()
    }
}

/// Custom mesh parameters
#[derive(Clone, Debug, Default)]
pub struct CustomMeshParams {
    /// Vertices
    pub vertices: Vec<ProceduralVertex>,
    /// Indices
    pub indices: Vec<u32>,
    /// Generate normals
    pub generate_normals: bool,
    /// Generate tangents
    pub generate_tangents: bool,
}

impl CustomMeshParams {
    /// Creates new params
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            generate_normals: true,
            generate_tangents: true,
        }
    }

    /// Add vertex
    pub fn add_vertex(mut self, vertex: ProceduralVertex) -> Self {
        self.vertices.push(vertex);
        self
    }

    /// Add triangle
    pub fn add_triangle(mut self, i0: u32, i1: u32, i2: u32) -> Self {
        self.indices.push(i0);
        self.indices.push(i1);
        self.indices.push(i2);
        self
    }

    /// Skip normal generation
    pub fn no_normals(mut self) -> Self {
        self.generate_normals = false;
        self
    }

    /// Skip tangent generation
    pub fn no_tangents(mut self) -> Self {
        self.generate_tangents = false;
        self
    }
}

// ============================================================================
// Procedural Vertex
// ============================================================================

/// Procedural vertex
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ProceduralVertex {
    /// Position
    pub position: [f32; 3],
    /// Normal
    pub normal: [f32; 3],
    /// Tangent
    pub tangent: [f32; 4],
    /// UV
    pub uv: [f32; 2],
    /// Color
    pub color: [f32; 4],
}

impl ProceduralVertex {
    /// Creates new vertex
    pub const fn new(position: [f32; 3]) -> Self {
        Self {
            position,
            normal: [0.0, 1.0, 0.0],
            tangent: [1.0, 0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// With normal
    pub const fn with_normal(mut self, normal: [f32; 3]) -> Self {
        self.normal = normal;
        self
    }

    /// With UV
    pub const fn with_uv(mut self, u: f32, v: f32) -> Self {
        self.uv = [u, v];
        self
    }

    /// With color
    pub const fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// Simple vertex
    pub const fn simple(x: f32, y: f32, z: f32) -> Self {
        Self::new([x, y, z])
    }
}

// ============================================================================
// Mesh Modification
// ============================================================================

/// Mesh modifier
#[derive(Clone, Debug)]
pub enum MeshModifier {
    /// Transform
    Transform(TransformModifier),
    /// Subdivide
    Subdivide(SubdivideModifier),
    /// Simplify
    Simplify(SimplifyModifier),
    /// Noise
    Noise(NoiseModifier),
    /// Extrude
    Extrude(ExtrudeModifier),
    /// Weld
    Weld(WeldModifier),
}

/// Transform modifier
#[derive(Clone, Copy, Debug)]
pub struct TransformModifier {
    /// Translation
    pub translation: [f32; 3],
    /// Rotation (euler)
    pub rotation: [f32; 3],
    /// Scale
    pub scale: [f32; 3],
}

impl TransformModifier {
    /// Identity
    pub const fn identity() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    /// Translation
    pub const fn translate(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: [x, y, z],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    /// Scale
    pub const fn scale(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [x, y, z],
        }
    }

    /// Uniform scale
    pub const fn uniform_scale(s: f32) -> Self {
        Self::scale(s, s, s)
    }
}

impl Default for TransformModifier {
    fn default() -> Self {
        Self::identity()
    }
}

/// Subdivide modifier
#[derive(Clone, Copy, Debug)]
pub struct SubdivideModifier {
    /// Iterations
    pub iterations: u32,
    /// Method
    pub method: SubdivideMethod,
}

impl SubdivideModifier {
    /// Simple subdivision
    pub const fn simple(iterations: u32) -> Self {
        Self {
            iterations,
            method: SubdivideMethod::Simple,
        }
    }

    /// Loop subdivision
    pub const fn loop_subdiv(iterations: u32) -> Self {
        Self {
            iterations,
            method: SubdivideMethod::Loop,
        }
    }

    /// Catmull-Clark subdivision
    pub const fn catmull_clark(iterations: u32) -> Self {
        Self {
            iterations,
            method: SubdivideMethod::CatmullClark,
        }
    }
}

impl Default for SubdivideModifier {
    fn default() -> Self {
        Self::simple(1)
    }
}

/// Subdivide method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SubdivideMethod {
    /// Simple (midpoint)
    #[default]
    Simple = 0,
    /// Loop
    Loop = 1,
    /// Catmull-Clark
    CatmullClark = 2,
}

/// Simplify modifier
#[derive(Clone, Copy, Debug)]
pub struct SimplifyModifier {
    /// Target ratio (0-1)
    pub ratio: f32,
    /// Target triangle count (0 = use ratio)
    pub target_triangles: u32,
    /// Preserve boundaries
    pub preserve_boundaries: bool,
}

impl SimplifyModifier {
    /// With ratio
    pub const fn ratio(ratio: f32) -> Self {
        Self {
            ratio,
            target_triangles: 0,
            preserve_boundaries: true,
        }
    }

    /// With target triangles
    pub const fn triangles(count: u32) -> Self {
        Self {
            ratio: 0.0,
            target_triangles: count,
            preserve_boundaries: true,
        }
    }
}

impl Default for SimplifyModifier {
    fn default() -> Self {
        Self::ratio(0.5)
    }
}

/// Noise modifier
#[derive(Clone, Copy, Debug)]
pub struct NoiseModifier {
    /// Amplitude
    pub amplitude: f32,
    /// Frequency
    pub frequency: f32,
    /// Octaves
    pub octaves: u32,
    /// Direction
    pub direction: NoiseDirection,
    /// Seed
    pub seed: u32,
}

impl NoiseModifier {
    /// Simple noise
    pub const fn simple(amplitude: f32) -> Self {
        Self {
            amplitude,
            frequency: 1.0,
            octaves: 1,
            direction: NoiseDirection::Normal,
            seed: 0,
        }
    }

    /// With frequency
    pub const fn with_frequency(mut self, freq: f32) -> Self {
        self.frequency = freq;
        self
    }

    /// With octaves
    pub const fn with_octaves(mut self, octaves: u32) -> Self {
        self.octaves = octaves;
        self
    }
}

impl Default for NoiseModifier {
    fn default() -> Self {
        Self::simple(0.1)
    }
}

/// Noise direction
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum NoiseDirection {
    /// Along normal
    #[default]
    Normal = 0,
    /// X axis
    X = 1,
    /// Y axis
    Y = 2,
    /// Z axis
    Z = 3,
    /// Random
    Random = 4,
}

/// Extrude modifier
#[derive(Clone, Copy, Debug)]
pub struct ExtrudeModifier {
    /// Distance
    pub distance: f32,
    /// Direction
    pub direction: [f32; 3],
    /// Cap
    pub cap: bool,
}

impl ExtrudeModifier {
    /// Along normal
    pub const fn normal(distance: f32) -> Self {
        Self {
            distance,
            direction: [0.0, 0.0, 0.0],
            cap: true,
        }
    }

    /// In direction
    pub const fn direction(distance: f32, dir: [f32; 3]) -> Self {
        Self {
            distance,
            direction: dir,
            cap: true,
        }
    }

    /// Without cap
    pub const fn no_cap(mut self) -> Self {
        self.cap = false;
        self
    }
}

impl Default for ExtrudeModifier {
    fn default() -> Self {
        Self::normal(1.0)
    }
}

/// Weld modifier
#[derive(Clone, Copy, Debug)]
pub struct WeldModifier {
    /// Distance threshold
    pub threshold: f32,
}

impl WeldModifier {
    /// New weld modifier
    pub const fn new(threshold: f32) -> Self {
        Self { threshold }
    }
}

impl Default for WeldModifier {
    fn default() -> Self {
        Self::new(0.0001)
    }
}

// ============================================================================
// Generated Mesh Info
// ============================================================================

/// Generated mesh info
#[derive(Clone, Debug, Default)]
pub struct GeneratedMeshInfo {
    /// Handle
    pub handle: GeneratedMeshHandle,
    /// Vertex count
    pub vertex_count: u32,
    /// Index count
    pub index_count: u32,
    /// Triangle count
    pub triangle_count: u32,
    /// Bounding box min
    pub bounds_min: [f32; 3],
    /// Bounding box max
    pub bounds_max: [f32; 3],
    /// Has normals
    pub has_normals: bool,
    /// Has tangents
    pub has_tangents: bool,
    /// Has UVs
    pub has_uvs: bool,
    /// Has colors
    pub has_colors: bool,
}

impl GeneratedMeshInfo {
    /// Vertex buffer size
    pub fn vertex_buffer_size(&self) -> u64 {
        self.vertex_count as u64 * core::mem::size_of::<ProceduralVertex>() as u64
    }

    /// Index buffer size
    pub fn index_buffer_size(&self) -> u64 {
        self.index_count as u64 * 4
    }

    /// Bounds center
    pub fn bounds_center(&self) -> [f32; 3] {
        [
            (self.bounds_min[0] + self.bounds_max[0]) * 0.5,
            (self.bounds_min[1] + self.bounds_max[1]) * 0.5,
            (self.bounds_min[2] + self.bounds_max[2]) * 0.5,
        ]
    }

    /// Bounds size
    pub fn bounds_size(&self) -> [f32; 3] {
        [
            self.bounds_max[0] - self.bounds_min[0],
            self.bounds_max[1] - self.bounds_min[1],
            self.bounds_max[2] - self.bounds_min[2],
        ]
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Procedural mesh statistics
#[derive(Clone, Debug, Default)]
pub struct ProceduralMeshStats {
    /// Meshes generated
    pub meshes_generated: u32,
    /// Total vertices
    pub total_vertices: u64,
    /// Total triangles
    pub total_triangles: u64,
    /// Modifiers applied
    pub modifiers_applied: u32,
    /// Generation time (ms)
    pub generation_time_ms: f32,
    /// Memory usage
    pub memory_usage: u64,
}

impl ProceduralMeshStats {
    /// Average vertices per mesh
    pub fn avg_vertices_per_mesh(&self) -> f32 {
        if self.meshes_generated == 0 {
            0.0
        } else {
            self.total_vertices as f32 / self.meshes_generated as f32
        }
    }
}
