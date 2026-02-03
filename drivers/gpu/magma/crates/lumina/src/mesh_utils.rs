//! Mesh Utilities for Lumina
//!
//! This module provides mesh types, vertex formats, mesh processing,
//! and mesh generation utilities.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Mesh Handle
// ============================================================================

/// Mesh handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MeshHandle(pub u64);

impl MeshHandle {
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

impl Default for MeshHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Mesh Create Info
// ============================================================================

/// Mesh create info
#[derive(Clone, Debug)]
pub struct MeshCreateInfo {
    /// Vertex data
    pub vertices: Vec<u8>,
    /// Index data
    pub indices: Vec<u8>,
    /// Vertex layout
    pub vertex_layout: VertexLayout,
    /// Index format
    pub index_format: IndexFormat,
    /// Primitive topology
    pub topology: PrimitiveTopology,
    /// Submeshes
    pub submeshes: Vec<Submesh>,
    /// Bounding box
    pub bounds: Option<BoundingBox>,
    /// Debug name
    pub debug_name: Option<String>,
}

impl MeshCreateInfo {
    /// Creates new mesh
    pub fn new(vertex_layout: VertexLayout) -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            vertex_layout,
            index_format: IndexFormat::U16,
            topology: PrimitiveTopology::TriangleList,
            submeshes: Vec::new(),
            bounds: None,
            debug_name: None,
        }
    }

    /// With vertices
    pub fn with_vertices(mut self, data: Vec<u8>) -> Self {
        self.vertices = data;
        self
    }

    /// With indices
    pub fn with_indices(mut self, data: Vec<u8>, format: IndexFormat) -> Self {
        self.indices = data;
        self.index_format = format;
        self
    }

    /// With u16 indices
    pub fn with_u16_indices(mut self, indices: &[u16]) -> Self {
        self.indices = indices.iter().flat_map(|i| i.to_le_bytes()).collect();
        self.index_format = IndexFormat::U16;
        self
    }

    /// With u32 indices
    pub fn with_u32_indices(mut self, indices: &[u32]) -> Self {
        self.indices = indices.iter().flat_map(|i| i.to_le_bytes()).collect();
        self.index_format = IndexFormat::U32;
        self
    }

    /// With topology
    pub fn with_topology(mut self, topology: PrimitiveTopology) -> Self {
        self.topology = topology;
        self
    }

    /// Add submesh
    pub fn add_submesh(mut self, submesh: Submesh) -> Self {
        self.submeshes.push(submesh);
        self
    }

    /// With bounds
    pub fn with_bounds(mut self, bounds: BoundingBox) -> Self {
        self.bounds = Some(bounds);
        self
    }

    /// With debug name
    pub fn with_name(mut self, name: &str) -> Self {
        self.debug_name = Some(String::from(name));
        self
    }

    /// Vertex count
    pub fn vertex_count(&self) -> u32 {
        let stride = self.vertex_layout.stride();
        if stride == 0 {
            0
        } else {
            (self.vertices.len() / stride as usize) as u32
        }
    }

    /// Index count
    pub fn index_count(&self) -> u32 {
        let size = self.index_format.size();
        (self.indices.len() / size as usize) as u32
    }
}

impl Default for MeshCreateInfo {
    fn default() -> Self {
        Self::new(VertexLayout::default())
    }
}

// ============================================================================
// Index Format
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
    /// 8-bit indices (extension)
    U8 = 2,
}

impl IndexFormat {
    /// Size in bytes
    #[inline]
    pub const fn size(&self) -> u32 {
        match self {
            Self::U8 => 1,
            Self::U16 => 2,
            Self::U32 => 4,
        }
    }

    /// Max index value
    #[inline]
    pub const fn max_index(&self) -> u32 {
        match self {
            Self::U8 => u8::MAX as u32,
            Self::U16 => u16::MAX as u32,
            Self::U32 => u32::MAX,
        }
    }
}

// ============================================================================
// Primitive Topology
// ============================================================================

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
    LineListWithAdjacency = 6,
    /// Line strip with adjacency
    LineStripWithAdjacency = 7,
    /// Triangle list with adjacency
    TriangleListWithAdjacency = 8,
    /// Triangle strip with adjacency
    TriangleStripWithAdjacency = 9,
    /// Patch list
    PatchList = 10,
}

impl PrimitiveTopology {
    /// Vertices per primitive
    #[inline]
    pub const fn vertices_per_primitive(&self) -> u32 {
        match self {
            Self::PointList => 1,
            Self::LineList | Self::LineStrip => 2,
            Self::TriangleList | Self::TriangleStrip | Self::TriangleFan => 3,
            Self::LineListWithAdjacency => 4,
            Self::LineStripWithAdjacency => 4,
            Self::TriangleListWithAdjacency => 6,
            Self::TriangleStripWithAdjacency => 6,
            Self::PatchList => 3, // Default, can vary
        }
    }

    /// Is list (not strip)
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
// Vertex Layout
// ============================================================================

/// Vertex layout
#[derive(Clone, Debug, Default)]
pub struct VertexLayout {
    /// Attributes
    pub attributes: Vec<VertexAttribute>,
    /// Stride (0 for auto-calculated)
    pub stride: u32,
}

impl VertexLayout {
    /// Creates new layout
    pub fn new() -> Self {
        Self {
            attributes: Vec::new(),
            stride: 0,
        }
    }

    /// Standard PBR layout: position, normal, tangent, uv
    pub fn standard_pbr() -> Self {
        Self::new()
            .add(VertexAttribute::position())
            .add(VertexAttribute::normal())
            .add(VertexAttribute::tangent())
            .add(VertexAttribute::tex_coord(0))
    }

    /// Simple layout: position, normal, uv
    pub fn simple() -> Self {
        Self::new()
            .add(VertexAttribute::position())
            .add(VertexAttribute::normal())
            .add(VertexAttribute::tex_coord(0))
    }

    /// Position only
    pub fn position_only() -> Self {
        Self::new().add(VertexAttribute::position())
    }

    /// Position and color
    pub fn position_color() -> Self {
        Self::new()
            .add(VertexAttribute::position())
            .add(VertexAttribute::color(0))
    }

    /// Skinned mesh layout
    pub fn skinned() -> Self {
        Self::new()
            .add(VertexAttribute::position())
            .add(VertexAttribute::normal())
            .add(VertexAttribute::tangent())
            .add(VertexAttribute::tex_coord(0))
            .add(VertexAttribute::bone_indices())
            .add(VertexAttribute::bone_weights())
    }

    /// Add attribute
    pub fn add(mut self, attr: VertexAttribute) -> Self {
        self.attributes.push(attr);
        self
    }

    /// With stride
    pub fn with_stride(mut self, stride: u32) -> Self {
        self.stride = stride;
        self
    }

    /// Calculate stride
    pub fn stride(&self) -> u32 {
        if self.stride > 0 {
            self.stride
        } else {
            self.attributes.iter().map(|a| a.format.size()).sum()
        }
    }

    /// Get attribute by semantic
    pub fn get_attribute(&self, semantic: VertexSemantic) -> Option<&VertexAttribute> {
        self.attributes.iter().find(|a| a.semantic == semantic)
    }

    /// Has attribute
    pub fn has_attribute(&self, semantic: VertexSemantic) -> bool {
        self.get_attribute(semantic).is_some()
    }

    /// Attribute count
    pub fn attribute_count(&self) -> usize {
        self.attributes.len()
    }
}

/// Vertex attribute
#[derive(Clone, Debug)]
pub struct VertexAttribute {
    /// Semantic
    pub semantic: VertexSemantic,
    /// Format
    pub format: VertexFormat,
    /// Offset (0 for auto-calculated)
    pub offset: u32,
    /// Location (binding)
    pub location: u32,
}

impl VertexAttribute {
    /// Creates new attribute
    pub fn new(semantic: VertexSemantic, format: VertexFormat, location: u32) -> Self {
        Self {
            semantic,
            format,
            offset: 0,
            location,
        }
    }

    /// Position attribute
    pub fn position() -> Self {
        Self::new(VertexSemantic::Position, VertexFormat::Float3, 0)
    }

    /// Normal attribute
    pub fn normal() -> Self {
        Self::new(VertexSemantic::Normal, VertexFormat::Float3, 1)
    }

    /// Tangent attribute (with handedness)
    pub fn tangent() -> Self {
        Self::new(VertexSemantic::Tangent, VertexFormat::Float4, 2)
    }

    /// Texture coordinate attribute
    pub fn tex_coord(index: u32) -> Self {
        Self::new(
            VertexSemantic::TexCoord(index as u8),
            VertexFormat::Float2,
            3 + index,
        )
    }

    /// Color attribute
    pub fn color(index: u32) -> Self {
        Self::new(
            VertexSemantic::Color(index as u8),
            VertexFormat::Float4,
            3 + index,
        )
    }

    /// Bone indices attribute
    pub fn bone_indices() -> Self {
        Self::new(VertexSemantic::BoneIndices, VertexFormat::UByte4, 5)
    }

    /// Bone weights attribute
    pub fn bone_weights() -> Self {
        Self::new(VertexSemantic::BoneWeights, VertexFormat::Float4, 6)
    }

    /// With offset
    pub fn with_offset(mut self, offset: u32) -> Self {
        self.offset = offset;
        self
    }

    /// With location
    pub fn with_location(mut self, location: u32) -> Self {
        self.location = location;
        self
    }
}

/// Vertex semantic
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VertexSemantic {
    /// Position
    Position,
    /// Normal
    Normal,
    /// Tangent
    Tangent,
    /// Bitangent
    Bitangent,
    /// Texture coordinate
    TexCoord(u8),
    /// Color
    Color(u8),
    /// Bone indices
    BoneIndices,
    /// Bone weights
    BoneWeights,
    /// Custom
    Custom(u8),
}

impl Default for VertexSemantic {
    fn default() -> Self {
        Self::Position
    }
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
    /// Int
    Int = 4,
    /// Int2
    Int2 = 5,
    /// Int3
    Int3 = 6,
    /// Int4
    Int4 = 7,
    /// UInt
    UInt = 8,
    /// UInt2
    UInt2 = 9,
    /// UInt3
    UInt3 = 10,
    /// UInt4
    UInt4 = 11,
    /// Half
    Half = 12,
    /// Half2
    Half2 = 13,
    /// Half4
    Half4 = 14,
    /// Byte4 normalized
    Byte4Norm = 15,
    /// UByte4 normalized
    UByte4Norm = 16,
    /// UByte4
    UByte4 = 17,
    /// Short2
    Short2 = 18,
    /// Short2 normalized
    Short2Norm = 19,
    /// Short4
    Short4 = 20,
    /// Short4 normalized
    Short4Norm = 21,
    /// RGB10A2
    Rgb10A2 = 22,
}

impl VertexFormat {
    /// Size in bytes
    pub const fn size(&self) -> u32 {
        match self {
            Self::Float | Self::Int | Self::UInt => 4,
            Self::Float2 | Self::Int2 | Self::UInt2 | Self::Half4 => 8,
            Self::Float3 | Self::Int3 | Self::UInt3 => 12,
            Self::Float4 | Self::Int4 | Self::UInt4 => 16,
            Self::Half => 2,
            Self::Half2 | Self::Byte4Norm | Self::UByte4Norm | Self::UByte4 | Self::Short2 | Self::Short2Norm | Self::Rgb10A2 => 4,
            Self::Short4 | Self::Short4Norm => 8,
        }
    }

    /// Component count
    pub const fn components(&self) -> u32 {
        match self {
            Self::Float | Self::Int | Self::UInt | Self::Half => 1,
            Self::Float2 | Self::Int2 | Self::UInt2 | Self::Half2 | Self::Short2 | Self::Short2Norm => 2,
            Self::Float3 | Self::Int3 | Self::UInt3 => 3,
            Self::Float4 | Self::Int4 | Self::UInt4 | Self::Half4 | Self::Byte4Norm | Self::UByte4Norm | Self::UByte4 | Self::Short4 | Self::Short4Norm | Self::Rgb10A2 => 4,
        }
    }
}

// ============================================================================
// Submesh
// ============================================================================

/// Submesh
#[derive(Clone, Debug, Default)]
pub struct Submesh {
    /// Start index
    pub start_index: u32,
    /// Index count
    pub index_count: u32,
    /// Base vertex
    pub base_vertex: i32,
    /// Material index
    pub material_index: u32,
    /// Bounding box
    pub bounds: Option<BoundingBox>,
    /// Name
    pub name: Option<String>,
}

impl Submesh {
    /// Creates new submesh
    pub fn new(start_index: u32, index_count: u32) -> Self {
        Self {
            start_index,
            index_count,
            base_vertex: 0,
            material_index: 0,
            bounds: None,
            name: None,
        }
    }

    /// With base vertex
    pub fn with_base_vertex(mut self, base: i32) -> Self {
        self.base_vertex = base;
        self
    }

    /// With material
    pub fn with_material(mut self, index: u32) -> Self {
        self.material_index = index;
        self
    }

    /// With bounds
    pub fn with_bounds(mut self, bounds: BoundingBox) -> Self {
        self.bounds = Some(bounds);
        self
    }

    /// With name
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = Some(String::from(name));
        self
    }
}

// ============================================================================
// Bounding Box
// ============================================================================

/// Axis-aligned bounding box
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct BoundingBox {
    /// Minimum corner
    pub min: [f32; 3],
    /// Maximum corner
    pub max: [f32; 3],
}

impl BoundingBox {
    /// Empty bounding box
    pub const EMPTY: Self = Self {
        min: [f32::MAX, f32::MAX, f32::MAX],
        max: [f32::MIN, f32::MIN, f32::MIN],
    };

    /// Unit cube centered at origin
    pub const UNIT: Self = Self {
        min: [-0.5, -0.5, -0.5],
        max: [0.5, 0.5, 0.5],
    };

    /// Creates from min/max
    pub const fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    /// Creates from center and half extents
    pub fn from_center_extents(center: [f32; 3], half_extents: [f32; 3]) -> Self {
        Self {
            min: [
                center[0] - half_extents[0],
                center[1] - half_extents[1],
                center[2] - half_extents[2],
            ],
            max: [
                center[0] + half_extents[0],
                center[1] + half_extents[1],
                center[2] + half_extents[2],
            ],
        }
    }

    /// Center
    pub fn center(&self) -> [f32; 3] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ]
    }

    /// Extents (half size)
    pub fn extents(&self) -> [f32; 3] {
        [
            (self.max[0] - self.min[0]) * 0.5,
            (self.max[1] - self.min[1]) * 0.5,
            (self.max[2] - self.min[2]) * 0.5,
        ]
    }

    /// Size
    pub fn size(&self) -> [f32; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    /// Expand to include point
    pub fn expand(&mut self, point: [f32; 3]) {
        self.min[0] = self.min[0].min(point[0]);
        self.min[1] = self.min[1].min(point[1]);
        self.min[2] = self.min[2].min(point[2]);
        self.max[0] = self.max[0].max(point[0]);
        self.max[1] = self.max[1].max(point[1]);
        self.max[2] = self.max[2].max(point[2]);
    }

    /// Merge with another box
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            min: [
                self.min[0].min(other.min[0]),
                self.min[1].min(other.min[1]),
                self.min[2].min(other.min[2]),
            ],
            max: [
                self.max[0].max(other.max[0]),
                self.max[1].max(other.max[1]),
                self.max[2].max(other.max[2]),
            ],
        }
    }

    /// Contains point
    pub fn contains(&self, point: [f32; 3]) -> bool {
        point[0] >= self.min[0]
            && point[0] <= self.max[0]
            && point[1] >= self.min[1]
            && point[1] <= self.max[1]
            && point[2] >= self.min[2]
            && point[2] <= self.max[2]
    }

    /// Intersects other box
    pub fn intersects(&self, other: &Self) -> bool {
        self.min[0] <= other.max[0]
            && self.max[0] >= other.min[0]
            && self.min[1] <= other.max[1]
            && self.max[1] >= other.min[1]
            && self.min[2] <= other.max[2]
            && self.max[2] >= other.min[2]
    }

    /// Volume
    pub fn volume(&self) -> f32 {
        let size = self.size();
        size[0] * size[1] * size[2]
    }

    /// Surface area
    pub fn surface_area(&self) -> f32 {
        let size = self.size();
        2.0 * (size[0] * size[1] + size[1] * size[2] + size[2] * size[0])
    }

    /// Is valid (max >= min)
    pub fn is_valid(&self) -> bool {
        self.max[0] >= self.min[0] && self.max[1] >= self.min[1] && self.max[2] >= self.min[2]
    }
}

// ============================================================================
// Bounding Sphere
// ============================================================================

/// Bounding sphere
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct BoundingSphere {
    /// Center
    pub center: [f32; 3],
    /// Radius
    pub radius: f32,
}

impl BoundingSphere {
    /// Creates from center and radius
    pub const fn new(center: [f32; 3], radius: f32) -> Self {
        Self { center, radius }
    }

    /// From bounding box
    pub fn from_aabb(aabb: &BoundingBox) -> Self {
        let center = aabb.center();
        let extents = aabb.extents();
        let radius = (extents[0] * extents[0] + extents[1] * extents[1] + extents[2] * extents[2]).sqrt();
        Self { center, radius }
    }

    /// Contains point
    pub fn contains(&self, point: [f32; 3]) -> bool {
        let dx = point[0] - self.center[0];
        let dy = point[1] - self.center[1];
        let dz = point[2] - self.center[2];
        let dist_sq = dx * dx + dy * dy + dz * dz;
        dist_sq <= self.radius * self.radius
    }

    /// Intersects other sphere
    pub fn intersects(&self, other: &Self) -> bool {
        let dx = other.center[0] - self.center[0];
        let dy = other.center[1] - self.center[1];
        let dz = other.center[2] - self.center[2];
        let dist_sq = dx * dx + dy * dy + dz * dz;
        let radii_sum = self.radius + other.radius;
        dist_sq <= radii_sum * radii_sum
    }

    /// Merge with another sphere
    pub fn merge(&self, other: &Self) -> Self {
        let dx = other.center[0] - self.center[0];
        let dy = other.center[1] - self.center[1];
        let dz = other.center[2] - self.center[2];
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();

        if dist + other.radius <= self.radius {
            // Other is inside self
            return *self;
        }
        if dist + self.radius <= other.radius {
            // Self is inside other
            return *other;
        }

        let new_radius = (dist + self.radius + other.radius) * 0.5;
        let t = (new_radius - self.radius) / dist;

        Self {
            center: [
                self.center[0] + dx * t,
                self.center[1] + dy * t,
                self.center[2] + dz * t,
            ],
            radius: new_radius,
        }
    }
}

// ============================================================================
// Mesh Generator
// ============================================================================

/// Mesh generator for procedural primitives
pub struct MeshGenerator;

impl MeshGenerator {
    /// Generate quad (2 triangles)
    pub fn quad(width: f32, height: f32) -> MeshData {
        let hw = width * 0.5;
        let hh = height * 0.5;

        let positions = vec![
            -hw, -hh, 0.0,
            hw, -hh, 0.0,
            hw, hh, 0.0,
            -hw, hh, 0.0,
        ];

        let normals = vec![
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
        ];

        let uvs = vec![
            0.0, 1.0,
            1.0, 1.0,
            1.0, 0.0,
            0.0, 0.0,
        ];

        let indices = vec![0, 1, 2, 0, 2, 3];

        MeshData {
            positions,
            normals,
            tangents: Vec::new(),
            uvs,
            colors: Vec::new(),
            indices,
            bounds: BoundingBox::new([-hw, -hh, 0.0], [hw, hh, 0.0]),
        }
    }

    /// Generate cube
    pub fn cube(size: f32) -> MeshData {
        let s = size * 0.5;

        // 24 vertices (4 per face, 6 faces)
        #[rustfmt::skip]
        let positions = vec![
            // Front
            -s, -s, s, s, -s, s, s, s, s, -s, s, s,
            // Back
            s, -s, -s, -s, -s, -s, -s, s, -s, s, s, -s,
            // Left
            -s, -s, -s, -s, -s, s, -s, s, s, -s, s, -s,
            // Right
            s, -s, s, s, -s, -s, s, s, -s, s, s, s,
            // Top
            -s, s, s, s, s, s, s, s, -s, -s, s, -s,
            // Bottom
            -s, -s, -s, s, -s, -s, s, -s, s, -s, -s, s,
        ];

        #[rustfmt::skip]
        let normals = vec![
            // Front
            0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
            // Back
            0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0,
            // Left
            -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0,
            // Right
            1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
            // Top
            0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0,
            // Bottom
            0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0,
        ];

        #[rustfmt::skip]
        let uvs = vec![
            // Each face
            0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0,
        ];

        let mut indices = Vec::with_capacity(36);
        for face in 0..6u32 {
            let base = face * 4;
            indices.extend_from_slice(&[
                base, base + 1, base + 2,
                base, base + 2, base + 3,
            ]);
        }

        MeshData {
            positions,
            normals,
            tangents: Vec::new(),
            uvs,
            colors: Vec::new(),
            indices,
            bounds: BoundingBox::new([-s, -s, -s], [s, s, s]),
        }
    }

    /// Generate sphere (UV sphere)
    pub fn sphere(radius: f32, segments: u32, rings: u32) -> MeshData {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        let segments = segments.max(3);
        let rings = rings.max(2);

        // Generate vertices
        for ring in 0..=rings {
            let v = ring as f32 / rings as f32;
            let phi = v * core::f32::consts::PI;

            for segment in 0..=segments {
                let u = segment as f32 / segments as f32;
                let theta = u * core::f32::consts::TAU;

                let x = (phi.sin()) * (theta.cos());
                let y = phi.cos();
                let z = (phi.sin()) * (theta.sin());

                positions.extend_from_slice(&[x * radius, y * radius, z * radius]);
                normals.extend_from_slice(&[x, y, z]);
                uvs.extend_from_slice(&[u, v]);
            }
        }

        // Generate indices
        for ring in 0..rings {
            for segment in 0..segments {
                let current = ring * (segments + 1) + segment;
                let next = current + segments + 1;

                indices.extend_from_slice(&[
                    current,
                    next,
                    current + 1,
                    current + 1,
                    next,
                    next + 1,
                ]);
            }
        }

        MeshData {
            positions,
            normals,
            tangents: Vec::new(),
            uvs,
            colors: Vec::new(),
            indices,
            bounds: BoundingBox::new([-radius, -radius, -radius], [radius, radius, radius]),
        }
    }

    /// Generate cylinder
    pub fn cylinder(radius: f32, height: f32, segments: u32) -> MeshData {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        let segments = segments.max(3);
        let half_height = height * 0.5;

        // Side vertices
        for i in 0..=segments {
            let u = i as f32 / segments as f32;
            let theta = u * core::f32::consts::TAU;
            let x = theta.cos();
            let z = theta.sin();

            // Bottom
            positions.extend_from_slice(&[x * radius, -half_height, z * radius]);
            normals.extend_from_slice(&[x, 0.0, z]);
            uvs.extend_from_slice(&[u, 0.0]);

            // Top
            positions.extend_from_slice(&[x * radius, half_height, z * radius]);
            normals.extend_from_slice(&[x, 0.0, z]);
            uvs.extend_from_slice(&[u, 1.0]);
        }

        // Side indices
        let verts_per_ring = (segments + 1) * 2;
        for i in 0..segments {
            let base = i * 2;
            indices.extend_from_slice(&[
                base, base + 2, base + 1,
                base + 1, base + 2, base + 3,
            ]);
        }

        // Top cap center
        let top_center_idx = (positions.len() / 3) as u32;
        positions.extend_from_slice(&[0.0, half_height, 0.0]);
        normals.extend_from_slice(&[0.0, 1.0, 0.0]);
        uvs.extend_from_slice(&[0.5, 0.5]);

        // Top cap ring
        for i in 0..=segments {
            let theta = (i as f32 / segments as f32) * core::f32::consts::TAU;
            let x = theta.cos();
            let z = theta.sin();

            positions.extend_from_slice(&[x * radius, half_height, z * radius]);
            normals.extend_from_slice(&[0.0, 1.0, 0.0]);
            uvs.extend_from_slice(&[x * 0.5 + 0.5, z * 0.5 + 0.5]);
        }

        // Top cap indices
        for i in 0..segments {
            indices.extend_from_slice(&[
                top_center_idx,
                top_center_idx + 1 + i,
                top_center_idx + 2 + i,
            ]);
        }

        // Bottom cap center
        let bottom_center_idx = (positions.len() / 3) as u32;
        positions.extend_from_slice(&[0.0, -half_height, 0.0]);
        normals.extend_from_slice(&[0.0, -1.0, 0.0]);
        uvs.extend_from_slice(&[0.5, 0.5]);

        // Bottom cap ring
        for i in 0..=segments {
            let theta = (i as f32 / segments as f32) * core::f32::consts::TAU;
            let x = theta.cos();
            let z = theta.sin();

            positions.extend_from_slice(&[x * radius, -half_height, z * radius]);
            normals.extend_from_slice(&[0.0, -1.0, 0.0]);
            uvs.extend_from_slice(&[x * 0.5 + 0.5, z * 0.5 + 0.5]);
        }

        // Bottom cap indices
        for i in 0..segments {
            indices.extend_from_slice(&[
                bottom_center_idx,
                bottom_center_idx + 2 + i,
                bottom_center_idx + 1 + i,
            ]);
        }

        MeshData {
            positions,
            normals,
            tangents: Vec::new(),
            uvs,
            colors: Vec::new(),
            indices,
            bounds: BoundingBox::new([-radius, -half_height, -radius], [radius, half_height, radius]),
        }
    }

    /// Generate cone
    pub fn cone(radius: f32, height: f32, segments: u32) -> MeshData {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        let segments = segments.max(3);
        let half_height = height * 0.5;
        let slope = radius / height;

        // Apex
        let apex_idx = 0u32;
        positions.extend_from_slice(&[0.0, half_height, 0.0]);
        normals.extend_from_slice(&[0.0, 1.0, 0.0]);
        uvs.extend_from_slice(&[0.5, 0.0]);

        // Side vertices
        for i in 0..=segments {
            let u = i as f32 / segments as f32;
            let theta = u * core::f32::consts::TAU;
            let x = theta.cos();
            let z = theta.sin();

            // Normal calculation for cone
            let ny = slope / (1.0 + slope * slope).sqrt();
            let nr = 1.0 / (1.0 + slope * slope).sqrt();

            positions.extend_from_slice(&[x * radius, -half_height, z * radius]);
            normals.extend_from_slice(&[x * nr, ny, z * nr]);
            uvs.extend_from_slice(&[u, 1.0]);
        }

        // Side indices
        for i in 0..segments {
            indices.extend_from_slice(&[apex_idx, 1 + i, 2 + i]);
        }

        // Base center
        let base_center_idx = (positions.len() / 3) as u32;
        positions.extend_from_slice(&[0.0, -half_height, 0.0]);
        normals.extend_from_slice(&[0.0, -1.0, 0.0]);
        uvs.extend_from_slice(&[0.5, 0.5]);

        // Base ring
        for i in 0..=segments {
            let theta = (i as f32 / segments as f32) * core::f32::consts::TAU;
            let x = theta.cos();
            let z = theta.sin();

            positions.extend_from_slice(&[x * radius, -half_height, z * radius]);
            normals.extend_from_slice(&[0.0, -1.0, 0.0]);
            uvs.extend_from_slice(&[x * 0.5 + 0.5, z * 0.5 + 0.5]);
        }

        // Base indices
        for i in 0..segments {
            indices.extend_from_slice(&[
                base_center_idx,
                base_center_idx + 2 + i,
                base_center_idx + 1 + i,
            ]);
        }

        MeshData {
            positions,
            normals,
            tangents: Vec::new(),
            uvs,
            colors: Vec::new(),
            indices,
            bounds: BoundingBox::new([-radius, -half_height, -radius], [radius, half_height, radius]),
        }
    }

    /// Generate plane (subdivided grid)
    pub fn plane(width: f32, depth: f32, segments_x: u32, segments_z: u32) -> MeshData {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        let hw = width * 0.5;
        let hd = depth * 0.5;
        let segments_x = segments_x.max(1);
        let segments_z = segments_z.max(1);

        // Vertices
        for z in 0..=segments_z {
            let v = z as f32 / segments_z as f32;
            let pz = v * depth - hd;

            for x in 0..=segments_x {
                let u = x as f32 / segments_x as f32;
                let px = u * width - hw;

                positions.extend_from_slice(&[px, 0.0, pz]);
                normals.extend_from_slice(&[0.0, 1.0, 0.0]);
                uvs.extend_from_slice(&[u, v]);
            }
        }

        // Indices
        for z in 0..segments_z {
            for x in 0..segments_x {
                let current = z * (segments_x + 1) + x;
                let next = current + segments_x + 1;

                indices.extend_from_slice(&[
                    current,
                    next,
                    current + 1,
                    current + 1,
                    next,
                    next + 1,
                ]);
            }
        }

        MeshData {
            positions,
            normals,
            tangents: Vec::new(),
            uvs,
            colors: Vec::new(),
            indices,
            bounds: BoundingBox::new([-hw, 0.0, -hd], [hw, 0.0, hd]),
        }
    }

    /// Generate torus
    pub fn torus(major_radius: f32, minor_radius: f32, major_segments: u32, minor_segments: u32) -> MeshData {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        let major_segments = major_segments.max(3);
        let minor_segments = minor_segments.max(3);

        for major in 0..=major_segments {
            let u = major as f32 / major_segments as f32;
            let theta = u * core::f32::consts::TAU;

            let cos_theta = theta.cos();
            let sin_theta = theta.sin();

            for minor in 0..=minor_segments {
                let v = minor as f32 / minor_segments as f32;
                let phi = v * core::f32::consts::TAU;

                let cos_phi = phi.cos();
                let sin_phi = phi.sin();

                let x = (major_radius + minor_radius * cos_phi) * cos_theta;
                let y = minor_radius * sin_phi;
                let z = (major_radius + minor_radius * cos_phi) * sin_theta;

                let nx = cos_phi * cos_theta;
                let ny = sin_phi;
                let nz = cos_phi * sin_theta;

                positions.extend_from_slice(&[x, y, z]);
                normals.extend_from_slice(&[nx, ny, nz]);
                uvs.extend_from_slice(&[u, v]);
            }
        }

        let stride = minor_segments + 1;
        for major in 0..major_segments {
            for minor in 0..minor_segments {
                let current = major * stride + minor;
                let next = current + stride;

                indices.extend_from_slice(&[
                    current,
                    next,
                    current + 1,
                    current + 1,
                    next,
                    next + 1,
                ]);
            }
        }

        let extent = major_radius + minor_radius;
        MeshData {
            positions,
            normals,
            tangents: Vec::new(),
            uvs,
            colors: Vec::new(),
            indices,
            bounds: BoundingBox::new(
                [-extent, -minor_radius, -extent],
                [extent, minor_radius, extent],
            ),
        }
    }
}

// ============================================================================
// Mesh Data
// ============================================================================

/// Raw mesh data
#[derive(Clone, Debug, Default)]
pub struct MeshData {
    /// Positions (x, y, z)
    pub positions: Vec<f32>,
    /// Normals (x, y, z)
    pub normals: Vec<f32>,
    /// Tangents (x, y, z, w)
    pub tangents: Vec<f32>,
    /// UVs (u, v)
    pub uvs: Vec<f32>,
    /// Colors (r, g, b, a)
    pub colors: Vec<f32>,
    /// Indices
    pub indices: Vec<u32>,
    /// Bounding box
    pub bounds: BoundingBox,
}

impl MeshData {
    /// Vertex count
    pub fn vertex_count(&self) -> u32 {
        (self.positions.len() / 3) as u32
    }

    /// Index count
    pub fn index_count(&self) -> u32 {
        self.indices.len() as u32
    }

    /// Triangle count
    pub fn triangle_count(&self) -> u32 {
        self.index_count() / 3
    }

    /// Calculate tangents from positions, normals, and UVs
    pub fn calculate_tangents(&mut self) {
        let vertex_count = self.vertex_count() as usize;
        let triangle_count = self.triangle_count() as usize;

        if self.positions.is_empty() || self.normals.is_empty() || self.uvs.is_empty() {
            return;
        }

        let mut tangents = alloc::vec![[0.0f32; 3]; vertex_count];
        let mut bitangents = alloc::vec![[0.0f32; 3]; vertex_count];

        for tri in 0..triangle_count {
            let i0 = self.indices[tri * 3] as usize;
            let i1 = self.indices[tri * 3 + 1] as usize;
            let i2 = self.indices[tri * 3 + 2] as usize;

            let p0 = [self.positions[i0 * 3], self.positions[i0 * 3 + 1], self.positions[i0 * 3 + 2]];
            let p1 = [self.positions[i1 * 3], self.positions[i1 * 3 + 1], self.positions[i1 * 3 + 2]];
            let p2 = [self.positions[i2 * 3], self.positions[i2 * 3 + 1], self.positions[i2 * 3 + 2]];

            let uv0 = [self.uvs[i0 * 2], self.uvs[i0 * 2 + 1]];
            let uv1 = [self.uvs[i1 * 2], self.uvs[i1 * 2 + 1]];
            let uv2 = [self.uvs[i2 * 2], self.uvs[i2 * 2 + 1]];

            let edge1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
            let edge2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

            let delta_uv1 = [uv1[0] - uv0[0], uv1[1] - uv0[1]];
            let delta_uv2 = [uv2[0] - uv0[0], uv2[1] - uv0[1]];

            let r = 1.0 / (delta_uv1[0] * delta_uv2[1] - delta_uv2[0] * delta_uv1[1]);

            let tangent = [
                (delta_uv2[1] * edge1[0] - delta_uv1[1] * edge2[0]) * r,
                (delta_uv2[1] * edge1[1] - delta_uv1[1] * edge2[1]) * r,
                (delta_uv2[1] * edge1[2] - delta_uv1[1] * edge2[2]) * r,
            ];

            let bitangent = [
                (delta_uv1[0] * edge2[0] - delta_uv2[0] * edge1[0]) * r,
                (delta_uv1[0] * edge2[1] - delta_uv2[0] * edge1[1]) * r,
                (delta_uv1[0] * edge2[2] - delta_uv2[0] * edge1[2]) * r,
            ];

            for i in [i0, i1, i2] {
                tangents[i][0] += tangent[0];
                tangents[i][1] += tangent[1];
                tangents[i][2] += tangent[2];
                bitangents[i][0] += bitangent[0];
                bitangents[i][1] += bitangent[1];
                bitangents[i][2] += bitangent[2];
            }
        }

        self.tangents.clear();
        self.tangents.reserve(vertex_count * 4);

        for i in 0..vertex_count {
            let n = [self.normals[i * 3], self.normals[i * 3 + 1], self.normals[i * 3 + 2]];
            let t = tangents[i];
            let b = bitangents[i];

            // Gram-Schmidt orthogonalize
            let dot = n[0] * t[0] + n[1] * t[1] + n[2] * t[2];
            let tangent = [t[0] - n[0] * dot, t[1] - n[1] * dot, t[2] - n[2] * dot];

            // Normalize
            let len = (tangent[0] * tangent[0] + tangent[1] * tangent[1] + tangent[2] * tangent[2]).sqrt();
            let tangent = if len > 0.0 {
                [tangent[0] / len, tangent[1] / len, tangent[2] / len]
            } else {
                [1.0, 0.0, 0.0]
            };

            // Calculate handedness
            let cross = [
                n[1] * t[2] - n[2] * t[1],
                n[2] * t[0] - n[0] * t[2],
                n[0] * t[1] - n[1] * t[0],
            ];
            let handedness = if cross[0] * b[0] + cross[1] * b[1] + cross[2] * b[2] < 0.0 {
                -1.0
            } else {
                1.0
            };

            self.tangents.extend_from_slice(&[tangent[0], tangent[1], tangent[2], handedness]);
        }
    }

    /// Recalculate bounding box
    pub fn recalculate_bounds(&mut self) {
        let mut bounds = BoundingBox::EMPTY;

        for i in 0..self.vertex_count() as usize {
            let point = [
                self.positions[i * 3],
                self.positions[i * 3 + 1],
                self.positions[i * 3 + 2],
            ];
            bounds.expand(point);
        }

        self.bounds = bounds;
    }
}
