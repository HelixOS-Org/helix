//! Mesh Processing Types for Lumina
//!
//! This module provides mesh processing infrastructure including
//! mesh optimization, vertex cache optimization, and mesh generation.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Mesh Processing Handles
// ============================================================================

/// Mesh processor handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MeshProcessorHandle(pub u64);

impl MeshProcessorHandle {
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

impl Default for MeshProcessorHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Processed mesh handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ProcessedMeshHandle(pub u64);

impl ProcessedMeshHandle {
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

impl Default for ProcessedMeshHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Vertex Format
// ============================================================================

/// Vertex attribute
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VertexAttribute {
    /// Semantic
    pub semantic: VertexSemantic,
    /// Format
    pub format: VertexFormat,
    /// Offset in vertex
    pub offset: u32,
}

impl VertexAttribute {
    /// Creates attribute
    pub const fn new(semantic: VertexSemantic, format: VertexFormat, offset: u32) -> Self {
        Self {
            semantic,
            format,
            offset,
        }
    }

    /// Position float3
    pub const fn position3() -> Self {
        Self::new(VertexSemantic::Position, VertexFormat::Float3, 0)
    }

    /// Normal float3
    pub const fn normal3(offset: u32) -> Self {
        Self::new(VertexSemantic::Normal, VertexFormat::Float3, offset)
    }

    /// Texcoord float2
    pub const fn texcoord2(offset: u32) -> Self {
        Self::new(VertexSemantic::TexCoord0, VertexFormat::Float2, offset)
    }

    /// Size in bytes
    pub fn size(&self) -> u32 {
        self.format.size()
    }
}

/// Vertex semantic
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum VertexSemantic {
    /// Position
    #[default]
    Position = 0,
    /// Normal
    Normal = 1,
    /// Tangent
    Tangent = 2,
    /// Bitangent
    Bitangent = 3,
    /// TexCoord 0
    TexCoord0 = 4,
    /// TexCoord 1
    TexCoord1 = 5,
    /// Color 0
    Color0 = 6,
    /// Color 1
    Color1 = 7,
    /// Bone indices
    BoneIndices = 8,
    /// Bone weights
    BoneWeights = 9,
    /// Custom 0
    Custom0 = 10,
    /// Custom 1
    Custom1 = 11,
}

/// Vertex format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum VertexFormat {
    /// Float
    Float = 0,
    /// Float2
    Float2 = 1,
    /// Float3
    #[default]
    Float3 = 2,
    /// Float4
    Float4 = 3,
    /// Half2
    Half2 = 4,
    /// Half4
    Half4 = 5,
    /// UByte4
    UByte4 = 6,
    /// UByte4 normalized
    UByte4Norm = 7,
    /// Short2
    Short2 = 8,
    /// Short2 normalized
    Short2Norm = 9,
    /// Short4
    Short4 = 10,
    /// Short4 normalized
    Short4Norm = 11,
    /// UInt
    UInt = 12,
    /// Int10-10-10-2
    Int1010102 = 13,
}

impl VertexFormat {
    /// Size in bytes
    pub const fn size(&self) -> u32 {
        match self {
            Self::Float => 4,
            Self::Float2 | Self::Half4 | Self::Short4 | Self::Short4Norm => 8,
            Self::Float3 => 12,
            Self::Float4 => 16,
            Self::Half2 | Self::UByte4 | Self::UByte4Norm | Self::Short2 | Self::Short2Norm
            | Self::UInt | Self::Int1010102 => 4,
        }
    }

    /// Component count
    pub const fn components(&self) -> u32 {
        match self {
            Self::Float | Self::UInt => 1,
            Self::Float2 | Self::Half2 | Self::Short2 | Self::Short2Norm => 2,
            Self::Float3 => 3,
            Self::Float4 | Self::Half4 | Self::UByte4 | Self::UByte4Norm | Self::Short4
            | Self::Short4Norm | Self::Int1010102 => 4,
        }
    }
}

/// Vertex layout
#[derive(Clone, Debug)]
pub struct VertexLayout {
    /// Attributes
    pub attributes: Vec<VertexAttribute>,
    /// Stride
    pub stride: u32,
}

impl VertexLayout {
    /// Creates layout
    pub fn new() -> Self {
        Self {
            attributes: Vec::new(),
            stride: 0,
        }
    }

    /// Standard P3N3T2 layout
    pub fn p3n3t2() -> Self {
        Self {
            attributes: alloc::vec![
                VertexAttribute::position3(),
                VertexAttribute::normal3(12),
                VertexAttribute::texcoord2(24),
            ],
            stride: 32,
        }
    }

    /// Standard P3N3T4T2 layout (with tangent)
    pub fn p3n3t4t2() -> Self {
        Self {
            attributes: alloc::vec![
                VertexAttribute::new(VertexSemantic::Position, VertexFormat::Float3, 0),
                VertexAttribute::new(VertexSemantic::Normal, VertexFormat::Float3, 12),
                VertexAttribute::new(VertexSemantic::Tangent, VertexFormat::Float4, 24),
                VertexAttribute::new(VertexSemantic::TexCoord0, VertexFormat::Float2, 40),
            ],
            stride: 48,
        }
    }

    /// Add attribute
    pub fn add(&mut self, attribute: VertexAttribute) {
        let new_end = attribute.offset + attribute.size();
        self.stride = self.stride.max(new_end);
        self.attributes.push(attribute);
    }

    /// Find attribute by semantic
    pub fn find(&self, semantic: VertexSemantic) -> Option<&VertexAttribute> {
        self.attributes.iter().find(|a| a.semantic == semantic)
    }
}

impl Default for VertexLayout {
    fn default() -> Self {
        Self::p3n3t2()
    }
}

// ============================================================================
// Mesh Optimization
// ============================================================================

/// Mesh optimization settings
#[derive(Clone, Debug)]
pub struct MeshOptimizationSettings {
    /// Vertex cache optimization
    pub vertex_cache: bool,
    /// Overdraw optimization
    pub overdraw: bool,
    /// Vertex fetch optimization
    pub vertex_fetch: bool,
    /// Generate meshlets
    pub meshlets: bool,
    /// Target cache size
    pub cache_size: u32,
    /// Threshold for overdraw
    pub overdraw_threshold: f32,
}

impl MeshOptimizationSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            vertex_cache: true,
            overdraw: true,
            vertex_fetch: true,
            meshlets: false,
            cache_size: 16,
            overdraw_threshold: 1.05,
        }
    }

    /// Full optimization
    pub fn full() -> Self {
        Self {
            meshlets: true,
            ..Self::new()
        }
    }

    /// Fast optimization (vertex cache only)
    pub fn fast() -> Self {
        Self {
            overdraw: false,
            vertex_fetch: false,
            ..Self::new()
        }
    }

    /// With cache size
    pub fn with_cache_size(mut self, size: u32) -> Self {
        self.cache_size = size;
        self
    }
}

impl Default for MeshOptimizationSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Vertex cache statistics
#[derive(Clone, Copy, Debug, Default)]
pub struct VertexCacheStats {
    /// ACMR (Average Cache Miss Ratio)
    pub acmr: f32,
    /// ATVR (Average Transformed Vertex Ratio)
    pub atvr: f32,
    /// Cache size used for calculation
    pub cache_size: u32,
}

impl VertexCacheStats {
    /// Is optimized (ACMR < 0.7)
    pub fn is_optimized(&self) -> bool {
        self.acmr < 0.7
    }
}

/// Overdraw statistics
#[derive(Clone, Copy, Debug, Default)]
pub struct OverdrawStats {
    /// Overdraw ratio
    pub overdraw: f32,
    /// Pixels shaded
    pub pixels_shaded: u64,
    /// Pixels covered
    pub pixels_covered: u64,
}

// ============================================================================
// Meshlet Generation
// ============================================================================

/// Meshlet settings
#[derive(Clone, Debug)]
pub struct MeshletSettings {
    /// Max vertices per meshlet
    pub max_vertices: u32,
    /// Max triangles per meshlet
    pub max_triangles: u32,
    /// Cone weight (for culling)
    pub cone_weight: f32,
}

impl MeshletSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            max_vertices: 64,
            max_triangles: 126,
            cone_weight: 0.0,
        }
    }

    /// NVidia optimized
    pub fn nvidia() -> Self {
        Self {
            max_vertices: 64,
            max_triangles: 126,
            ..Self::new()
        }
    }

    /// AMD optimized
    pub fn amd() -> Self {
        Self {
            max_vertices: 64,
            max_triangles: 64,
            ..Self::new()
        }
    }
}

impl Default for MeshletSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Meshlet
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Meshlet {
    /// Vertex offset
    pub vertex_offset: u32,
    /// Triangle offset
    pub triangle_offset: u32,
    /// Vertex count
    pub vertex_count: u32,
    /// Triangle count
    pub triangle_count: u32,
}

impl Meshlet {
    /// Creates meshlet
    pub fn new(
        vertex_offset: u32,
        triangle_offset: u32,
        vertex_count: u32,
        triangle_count: u32,
    ) -> Self {
        Self {
            vertex_offset,
            triangle_offset,
            vertex_count,
            triangle_count,
        }
    }
}

/// Meshlet bounds (for culling)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MeshletBounds {
    /// Bounding sphere center
    pub center: [f32; 3],
    /// Bounding sphere radius
    pub radius: f32,
    /// Normal cone axis
    pub cone_axis: [f32; 3],
    /// Normal cone cutoff
    pub cone_cutoff: f32,
}

impl MeshletBounds {
    /// Is backface for view direction
    pub fn is_backface(&self, view_dir: [f32; 3]) -> bool {
        let dot = self.cone_axis[0] * view_dir[0]
            + self.cone_axis[1] * view_dir[1]
            + self.cone_axis[2] * view_dir[2];
        dot >= self.cone_cutoff
    }
}

// ============================================================================
// Index Buffer
// ============================================================================

/// Index format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum IndexFormat {
    /// 16-bit indices
    #[default]
    U16 = 0,
    /// 32-bit indices
    U32 = 1,
}

impl IndexFormat {
    /// Size in bytes
    pub const fn size(&self) -> u32 {
        match self {
            Self::U16 => 2,
            Self::U32 => 4,
        }
    }

    /// Max value
    pub const fn max_value(&self) -> u32 {
        match self {
            Self::U16 => u16::MAX as u32,
            Self::U32 => u32::MAX,
        }
    }

    /// Choose format for vertex count
    pub fn for_vertex_count(count: u32) -> Self {
        if count <= u16::MAX as u32 {
            Self::U16
        } else {
            Self::U32
        }
    }
}

/// Primitive topology
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PrimitiveTopology {
    /// Point list
    PointList = 0,
    /// Line list
    LineList = 1,
    /// Line strip
    LineStrip = 2,
    /// Triangle list
    #[default]
    TriangleList = 3,
    /// Triangle strip
    TriangleStrip = 4,
    /// Triangle fan
    TriangleFan = 5,
    /// Line list with adjacency
    LineListAdj = 6,
    /// Triangle list with adjacency
    TriangleListAdj = 7,
    /// Patch list
    PatchList = 8,
}

impl PrimitiveTopology {
    /// Indices per primitive
    pub const fn indices_per_primitive(&self) -> u32 {
        match self {
            Self::PointList => 1,
            Self::LineList | Self::LineStrip => 2,
            Self::TriangleList | Self::TriangleStrip | Self::TriangleFan => 3,
            Self::LineListAdj => 4,
            Self::TriangleListAdj => 6,
            Self::PatchList => 0, // Variable
        }
    }
}

// ============================================================================
// Mesh Generation
// ============================================================================

/// Mesh generator
#[derive(Clone, Debug)]
pub struct MeshGenerator {
    /// Output layout
    pub layout: VertexLayout,
}

impl MeshGenerator {
    /// Creates generator
    pub fn new(layout: VertexLayout) -> Self {
        Self { layout }
    }

    /// Default generator
    pub fn default_generator() -> Self {
        Self::new(VertexLayout::p3n3t2())
    }
}

impl Default for MeshGenerator {
    fn default() -> Self {
        Self::default_generator()
    }
}

/// Cube generation params
#[derive(Clone, Copy, Debug)]
pub struct CubeParams {
    /// Size
    pub size: [f32; 3],
    /// Segments per axis
    pub segments: [u32; 3],
    /// Invert normals (for skybox)
    pub invert_normals: bool,
}

impl CubeParams {
    /// Creates params
    pub fn new(size: f32) -> Self {
        Self {
            size: [size, size, size],
            segments: [1, 1, 1],
            invert_normals: false,
        }
    }

    /// Unit cube
    pub fn unit() -> Self {
        Self::new(1.0)
    }

    /// Skybox cube
    pub fn skybox(size: f32) -> Self {
        Self {
            invert_normals: true,
            ..Self::new(size)
        }
    }

    /// Vertex count
    pub fn vertex_count(&self) -> u32 {
        6 * (self.segments[0] + 1) * (self.segments[1] + 1)
    }

    /// Index count
    pub fn index_count(&self) -> u32 {
        6 * 6 * self.segments[0] * self.segments[1]
    }
}

impl Default for CubeParams {
    fn default() -> Self {
        Self::unit()
    }
}

/// Sphere generation params
#[derive(Clone, Copy, Debug)]
pub struct SphereParams {
    /// Radius
    pub radius: f32,
    /// Horizontal segments
    pub segments: u32,
    /// Vertical rings
    pub rings: u32,
    /// UV mode
    pub uv_mode: SphereUvMode,
}

impl SphereParams {
    /// Creates params
    pub fn new(radius: f32, segments: u32, rings: u32) -> Self {
        Self {
            radius,
            segments,
            rings,
            uv_mode: SphereUvMode::Spherical,
        }
    }

    /// Low poly
    pub fn low_poly() -> Self {
        Self::new(1.0, 16, 8)
    }

    /// Medium poly
    pub fn medium_poly() -> Self {
        Self::new(1.0, 32, 16)
    }

    /// High poly
    pub fn high_poly() -> Self {
        Self::new(1.0, 64, 32)
    }

    /// Vertex count
    pub fn vertex_count(&self) -> u32 {
        (self.segments + 1) * (self.rings + 1)
    }

    /// Index count
    pub fn index_count(&self) -> u32 {
        6 * self.segments * self.rings
    }
}

impl Default for SphereParams {
    fn default() -> Self {
        Self::medium_poly()
    }
}

/// Sphere UV mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SphereUvMode {
    /// Spherical mapping
    #[default]
    Spherical = 0,
    /// Cubemap
    Cubemap = 1,
    /// Octahedral
    Octahedral = 2,
}

/// Plane generation params
#[derive(Clone, Copy, Debug)]
pub struct PlaneParams {
    /// Size
    pub size: [f32; 2],
    /// Segments
    pub segments: [u32; 2],
    /// Normal direction
    pub normal: PlaneNormal,
}

impl PlaneParams {
    /// Creates params
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            size: [width, height],
            segments: [1, 1],
            normal: PlaneNormal::Up,
        }
    }

    /// Ground plane
    pub fn ground(size: f32) -> Self {
        Self::new(size, size)
    }

    /// With segments
    pub fn with_segments(mut self, x: u32, y: u32) -> Self {
        self.segments = [x, y];
        self
    }

    /// Vertex count
    pub fn vertex_count(&self) -> u32 {
        (self.segments[0] + 1) * (self.segments[1] + 1)
    }

    /// Index count
    pub fn index_count(&self) -> u32 {
        6 * self.segments[0] * self.segments[1]
    }
}

impl Default for PlaneParams {
    fn default() -> Self {
        Self::ground(10.0)
    }
}

/// Plane normal direction
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PlaneNormal {
    /// +Y
    #[default]
    Up = 0,
    /// -Y
    Down = 1,
    /// +X
    Right = 2,
    /// -X
    Left = 3,
    /// +Z
    Forward = 4,
    /// -Z
    Back = 5,
}

/// Cylinder generation params
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
    /// Open ended (no caps)
    pub open_ended: bool,
}

impl CylinderParams {
    /// Creates params
    pub fn new(radius: f32, height: f32) -> Self {
        Self {
            top_radius: radius,
            bottom_radius: radius,
            height,
            radial_segments: 32,
            height_segments: 1,
            open_ended: false,
        }
    }

    /// Cone
    pub fn cone(radius: f32, height: f32) -> Self {
        Self {
            top_radius: 0.0,
            bottom_radius: radius,
            height,
            ..Self::new(radius, height)
        }
    }

    /// Tube (open ended)
    pub fn tube(radius: f32, height: f32) -> Self {
        Self {
            open_ended: true,
            ..Self::new(radius, height)
        }
    }
}

impl Default for CylinderParams {
    fn default() -> Self {
        Self::new(1.0, 2.0)
    }
}

/// Torus generation params
#[derive(Clone, Copy, Debug)]
pub struct TorusParams {
    /// Major radius
    pub major_radius: f32,
    /// Minor radius (tube)
    pub minor_radius: f32,
    /// Major segments
    pub major_segments: u32,
    /// Minor segments
    pub minor_segments: u32,
}

impl TorusParams {
    /// Creates params
    pub fn new(major_radius: f32, minor_radius: f32) -> Self {
        Self {
            major_radius,
            minor_radius,
            major_segments: 32,
            minor_segments: 16,
        }
    }

    /// Vertex count
    pub fn vertex_count(&self) -> u32 {
        self.major_segments * self.minor_segments
    }

    /// Index count
    pub fn index_count(&self) -> u32 {
        6 * self.major_segments * self.minor_segments
    }
}

impl Default for TorusParams {
    fn default() -> Self {
        Self::new(1.0, 0.25)
    }
}

// ============================================================================
// Mesh Data
// ============================================================================

/// Mesh bounds
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MeshBounds {
    /// AABB min
    pub min: [f32; 3],
    /// Padding
    pub _pad0: f32,
    /// AABB max
    pub max: [f32; 3],
    /// Padding
    pub _pad1: f32,
    /// Bounding sphere center
    pub center: [f32; 3],
    /// Bounding sphere radius
    pub radius: f32,
}

impl MeshBounds {
    /// Creates bounds
    pub fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        let center = [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ];
        let dx = max[0] - min[0];
        let dy = max[1] - min[1];
        let dz = max[2] - min[2];
        let radius = (dx * dx + dy * dy + dz * dz).sqrt() * 0.5;

        Self {
            min,
            _pad0: 0.0,
            max,
            _pad1: 0.0,
            center,
            radius,
        }
    }

    /// Merge with other bounds
    pub fn merge(&mut self, other: &Self) {
        self.min[0] = self.min[0].min(other.min[0]);
        self.min[1] = self.min[1].min(other.min[1]);
        self.min[2] = self.min[2].min(other.min[2]);
        self.max[0] = self.max[0].max(other.max[0]);
        self.max[1] = self.max[1].max(other.max[1]);
        self.max[2] = self.max[2].max(other.max[2]);
        *self = Self::new(self.min, self.max);
    }
}

/// Mesh info
#[derive(Clone, Debug, Default)]
pub struct MeshInfo {
    /// Vertex count
    pub vertex_count: u32,
    /// Index count
    pub index_count: u32,
    /// Triangle count
    pub triangle_count: u32,
    /// Vertex layout
    pub layout: VertexLayout,
    /// Index format
    pub index_format: IndexFormat,
    /// Bounds
    pub bounds: MeshBounds,
    /// Meshlet count
    pub meshlet_count: u32,
}

// ============================================================================
// Statistics
// ============================================================================

/// Mesh processing statistics
#[derive(Clone, Debug, Default)]
pub struct MeshProcessingStats {
    /// Vertices processed
    pub vertices_processed: u32,
    /// Indices processed
    pub indices_processed: u32,
    /// Meshlets generated
    pub meshlets_generated: u32,
    /// Processing time (microseconds)
    pub processing_time_us: u64,
    /// Before/after ACMR
    pub acmr_before: f32,
    /// After optimization
    pub acmr_after: f32,
}

impl MeshProcessingStats {
    /// Improvement ratio
    pub fn improvement(&self) -> f32 {
        if self.acmr_after == 0.0 {
            0.0
        } else {
            1.0 - (self.acmr_after / self.acmr_before)
        }
    }
}
