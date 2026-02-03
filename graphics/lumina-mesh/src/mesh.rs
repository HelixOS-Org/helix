//! Core Mesh System
//!
//! This module provides the core mesh abstraction including vertex data,
//! index buffers, and mesh management.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

// ============================================================================
// Mesh Handle
// ============================================================================

/// Handle to a mesh resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshHandle {
    /// Index.
    index: u32,
    /// Generation.
    generation: u32,
}

impl MeshHandle {
    /// Invalid handle.
    pub const INVALID: Self = Self {
        index: u32::MAX,
        generation: 0,
    };

    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        self.index != u32::MAX
    }
}

// ============================================================================
// Vertex Types
// ============================================================================

/// Standard vertex with position, normal, tangent, UV.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vertex {
    /// Position (x, y, z).
    pub position: [f32; 3],
    /// Normal (x, y, z).
    pub normal: [f32; 3],
    /// Tangent (x, y, z, w) - w is handedness.
    pub tangent: [f32; 4],
    /// UV coordinates.
    pub uv0: [f32; 2],
}

impl Vertex {
    /// Size in bytes.
    pub const SIZE: usize = 48;

    /// Create a new vertex.
    pub fn new(position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        Self {
            position,
            normal,
            tangent: [1.0, 0.0, 0.0, 1.0],
            uv0: uv,
        }
    }

    /// Set tangent.
    pub fn with_tangent(mut self, tangent: [f32; 4]) -> Self {
        self.tangent = tangent;
        self
    }
}

/// Skinned vertex with bone weights.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SkinnedVertex {
    /// Base vertex data.
    pub base: Vertex,
    /// Bone indices.
    pub bone_indices: [u8; 4],
    /// Bone weights.
    pub bone_weights: [f32; 4],
}

impl SkinnedVertex {
    /// Size in bytes.
    pub const SIZE: usize = 68;
}

/// Vertex with multiple UV sets.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MultiUVVertex {
    /// Base vertex data.
    pub base: Vertex,
    /// Second UV set.
    pub uv1: [f32; 2],
    /// Third UV set.
    pub uv2: [f32; 2],
}

/// Vertex with vertex colors.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ColoredVertex {
    /// Base vertex data.
    pub base: Vertex,
    /// Vertex color (RGBA).
    pub color: [f32; 4],
}

/// Compact vertex for simple meshes.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CompactVertex {
    /// Position.
    pub position: [f32; 3],
    /// Packed normal (octahedral encoding).
    pub normal: [i16; 2],
    /// UV (half precision).
    pub uv: [u16; 2],
}

impl CompactVertex {
    /// Size in bytes.
    pub const SIZE: usize = 20;
}

// ============================================================================
// Vertex Attributes
// ============================================================================

/// Vertex attribute semantic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VertexAttribute {
    /// Position (vec3).
    Position,
    /// Normal (vec3).
    Normal,
    /// Tangent (vec4).
    Tangent,
    /// Bitangent (vec3).
    Bitangent,
    /// UV set 0.
    TexCoord0,
    /// UV set 1.
    TexCoord1,
    /// UV set 2.
    TexCoord2,
    /// UV set 3.
    TexCoord3,
    /// Vertex color 0.
    Color0,
    /// Vertex color 1.
    Color1,
    /// Bone indices.
    Joints0,
    /// Bone weights.
    Weights0,
    /// Custom attribute.
    Custom(u8),
}

/// Attribute format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttributeFormat {
    /// Float.
    Float,
    /// Float2.
    Float2,
    /// Float3.
    Float3,
    /// Float4.
    Float4,
    /// Int.
    Int,
    /// Int2.
    Int2,
    /// Int3.
    Int3,
    /// Int4.
    Int4,
    /// UInt.
    UInt,
    /// UInt2.
    UInt2,
    /// UInt3.
    UInt3,
    /// UInt4.
    UInt4,
    /// UNorm8x4.
    UNorm8x4,
    /// SNorm8x4.
    SNorm8x4,
    /// UNorm16x2.
    UNorm16x2,
    /// SNorm16x2.
    SNorm16x2,
    /// Half2.
    Half2,
    /// Half4.
    Half4,
}

impl AttributeFormat {
    /// Get size in bytes.
    pub fn size(&self) -> usize {
        match self {
            Self::Float => 4,
            Self::Float2 => 8,
            Self::Float3 => 12,
            Self::Float4 => 16,
            Self::Int | Self::UInt => 4,
            Self::Int2 | Self::UInt2 => 8,
            Self::Int3 | Self::UInt3 => 12,
            Self::Int4 | Self::UInt4 => 16,
            Self::UNorm8x4 | Self::SNorm8x4 => 4,
            Self::UNorm16x2 | Self::SNorm16x2 => 4,
            Self::Half2 => 4,
            Self::Half4 => 8,
        }
    }
}

/// Vertex attribute descriptor.
#[derive(Debug, Clone, Copy)]
pub struct VertexAttributeDesc {
    /// Semantic.
    pub attribute: VertexAttribute,
    /// Format.
    pub format: AttributeFormat,
    /// Offset in vertex.
    pub offset: u32,
}

/// Vertex layout.
#[derive(Debug, Clone)]
pub struct VertexLayout {
    /// Stride in bytes.
    pub stride: u32,
    /// Attributes.
    pub attributes: Vec<VertexAttributeDesc>,
}

impl Default for VertexLayout {
    fn default() -> Self {
        Self::standard()
    }
}

impl VertexLayout {
    /// Standard vertex layout.
    pub fn standard() -> Self {
        Self {
            stride: Vertex::SIZE as u32,
            attributes: alloc::vec![
                VertexAttributeDesc {
                    attribute: VertexAttribute::Position,
                    format: AttributeFormat::Float3,
                    offset: 0,
                },
                VertexAttributeDesc {
                    attribute: VertexAttribute::Normal,
                    format: AttributeFormat::Float3,
                    offset: 12,
                },
                VertexAttributeDesc {
                    attribute: VertexAttribute::Tangent,
                    format: AttributeFormat::Float4,
                    offset: 24,
                },
                VertexAttributeDesc {
                    attribute: VertexAttribute::TexCoord0,
                    format: AttributeFormat::Float2,
                    offset: 40,
                },
            ],
        }
    }

    /// Compact vertex layout.
    pub fn compact() -> Self {
        Self {
            stride: CompactVertex::SIZE as u32,
            attributes: alloc::vec![
                VertexAttributeDesc {
                    attribute: VertexAttribute::Position,
                    format: AttributeFormat::Float3,
                    offset: 0,
                },
                VertexAttributeDesc {
                    attribute: VertexAttribute::Normal,
                    format: AttributeFormat::SNorm16x2,
                    offset: 12,
                },
                VertexAttributeDesc {
                    attribute: VertexAttribute::TexCoord0,
                    format: AttributeFormat::UNorm16x2,
                    offset: 16,
                },
            ],
        }
    }

    /// Skinned vertex layout.
    pub fn skinned() -> Self {
        Self {
            stride: SkinnedVertex::SIZE as u32,
            attributes: alloc::vec![
                VertexAttributeDesc {
                    attribute: VertexAttribute::Position,
                    format: AttributeFormat::Float3,
                    offset: 0,
                },
                VertexAttributeDesc {
                    attribute: VertexAttribute::Normal,
                    format: AttributeFormat::Float3,
                    offset: 12,
                },
                VertexAttributeDesc {
                    attribute: VertexAttribute::Tangent,
                    format: AttributeFormat::Float4,
                    offset: 24,
                },
                VertexAttributeDesc {
                    attribute: VertexAttribute::TexCoord0,
                    format: AttributeFormat::Float2,
                    offset: 40,
                },
                VertexAttributeDesc {
                    attribute: VertexAttribute::Joints0,
                    format: AttributeFormat::UNorm8x4,
                    offset: 48,
                },
                VertexAttributeDesc {
                    attribute: VertexAttribute::Weights0,
                    format: AttributeFormat::Float4,
                    offset: 52,
                },
            ],
        }
    }
}

// ============================================================================
// Index Types
// ============================================================================

/// Index format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IndexFormat {
    /// 16-bit indices.
    #[default]
    U16,
    /// 32-bit indices.
    U32,
}

impl IndexFormat {
    /// Get size in bytes.
    pub fn size(&self) -> usize {
        match self {
            Self::U16 => 2,
            Self::U32 => 4,
        }
    }
}

/// Index buffer data.
#[derive(Debug, Clone)]
pub enum IndexData {
    /// 16-bit indices.
    U16(Vec<u16>),
    /// 32-bit indices.
    U32(Vec<u32>),
}

impl IndexData {
    /// Get index count.
    pub fn count(&self) -> usize {
        match self {
            Self::U16(v) => v.len(),
            Self::U32(v) => v.len(),
        }
    }

    /// Get format.
    pub fn format(&self) -> IndexFormat {
        match self {
            Self::U16(_) => IndexFormat::U16,
            Self::U32(_) => IndexFormat::U32,
        }
    }

    /// Get index at position.
    pub fn get(&self, index: usize) -> Option<u32> {
        match self {
            Self::U16(v) => v.get(index).map(|&i| i as u32),
            Self::U32(v) => v.get(index).copied(),
        }
    }

    /// Get as bytes.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::U16(v) => unsafe {
                core::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 2)
            },
            Self::U32(v) => unsafe {
                core::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 4)
            },
        }
    }
}

// ============================================================================
// Mesh Primitive
// ============================================================================

/// Primitive topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MeshPrimitive {
    /// Point list.
    Points,
    /// Line list.
    Lines,
    /// Line strip.
    LineStrip,
    /// Triangle list.
    #[default]
    Triangles,
    /// Triangle strip.
    TriangleStrip,
    /// Triangle fan.
    TriangleFan,
    /// Patch list (for tessellation).
    Patches(u8),
}

impl MeshPrimitive {
    /// Get vertices per primitive.
    pub fn vertices_per_primitive(&self) -> usize {
        match self {
            Self::Points => 1,
            Self::Lines => 2,
            Self::LineStrip => 2,
            Self::Triangles => 3,
            Self::TriangleStrip => 3,
            Self::TriangleFan => 3,
            Self::Patches(n) => *n as usize,
        }
    }
}

// ============================================================================
// Bounding Volumes
// ============================================================================

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy, Default)]
pub struct AABB {
    /// Minimum corner.
    pub min: [f32; 3],
    /// Maximum corner.
    pub max: [f32; 3],
}

impl AABB {
    /// Invalid/empty AABB.
    pub const INVALID: Self = Self {
        min: [f32::MAX, f32::MAX, f32::MAX],
        max: [f32::MIN, f32::MIN, f32::MIN],
    };

    /// Create a new AABB.
    pub fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    /// Create from center and half-extents.
    pub fn from_center_extents(center: [f32; 3], extents: [f32; 3]) -> Self {
        Self {
            min: [
                center[0] - extents[0],
                center[1] - extents[1],
                center[2] - extents[2],
            ],
            max: [
                center[0] + extents[0],
                center[1] + extents[1],
                center[2] + extents[2],
            ],
        }
    }

    /// Get center.
    pub fn center(&self) -> [f32; 3] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ]
    }

    /// Get half-extents.
    pub fn extents(&self) -> [f32; 3] {
        [
            (self.max[0] - self.min[0]) * 0.5,
            (self.max[1] - self.min[1]) * 0.5,
            (self.max[2] - self.min[2]) * 0.5,
        ]
    }

    /// Get size.
    pub fn size(&self) -> [f32; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    /// Expand to include point.
    pub fn expand_point(&mut self, point: [f32; 3]) {
        self.min[0] = self.min[0].min(point[0]);
        self.min[1] = self.min[1].min(point[1]);
        self.min[2] = self.min[2].min(point[2]);
        self.max[0] = self.max[0].max(point[0]);
        self.max[1] = self.max[1].max(point[1]);
        self.max[2] = self.max[2].max(point[2]);
    }

    /// Expand to include another AABB.
    pub fn expand_aabb(&mut self, other: &AABB) {
        self.min[0] = self.min[0].min(other.min[0]);
        self.min[1] = self.min[1].min(other.min[1]);
        self.min[2] = self.min[2].min(other.min[2]);
        self.max[0] = self.max[0].max(other.max[0]);
        self.max[1] = self.max[1].max(other.max[1]);
        self.max[2] = self.max[2].max(other.max[2]);
    }

    /// Check if contains point.
    pub fn contains_point(&self, point: [f32; 3]) -> bool {
        point[0] >= self.min[0]
            && point[0] <= self.max[0]
            && point[1] >= self.min[1]
            && point[1] <= self.max[1]
            && point[2] >= self.min[2]
            && point[2] <= self.max[2]
    }

    /// Check if intersects another AABB.
    pub fn intersects(&self, other: &AABB) -> bool {
        self.min[0] <= other.max[0]
            && self.max[0] >= other.min[0]
            && self.min[1] <= other.max[1]
            && self.max[1] >= other.min[1]
            && self.min[2] <= other.max[2]
            && self.max[2] >= other.min[2]
    }

    /// Get surface area.
    pub fn surface_area(&self) -> f32 {
        let size = self.size();
        2.0 * (size[0] * size[1] + size[1] * size[2] + size[2] * size[0])
    }

    /// Get volume.
    pub fn volume(&self) -> f32 {
        let size = self.size();
        size[0] * size[1] * size[2]
    }
}

/// Bounding sphere.
#[derive(Debug, Clone, Copy, Default)]
pub struct BoundingSphere {
    /// Center.
    pub center: [f32; 3],
    /// Radius.
    pub radius: f32,
}

impl BoundingSphere {
    /// Create a new bounding sphere.
    pub fn new(center: [f32; 3], radius: f32) -> Self {
        Self { center, radius }
    }

    /// Create from AABB.
    pub fn from_aabb(aabb: &AABB) -> Self {
        let center = aabb.center();
        let extents = aabb.extents();
        let radius =
            (extents[0] * extents[0] + extents[1] * extents[1] + extents[2] * extents[2]).sqrt();
        Self { center, radius }
    }

    /// Check if contains point.
    pub fn contains_point(&self, point: [f32; 3]) -> bool {
        let dx = point[0] - self.center[0];
        let dy = point[1] - self.center[1];
        let dz = point[2] - self.center[2];
        dx * dx + dy * dy + dz * dz <= self.radius * self.radius
    }

    /// Check if intersects another sphere.
    pub fn intersects(&self, other: &BoundingSphere) -> bool {
        let dx = other.center[0] - self.center[0];
        let dy = other.center[1] - self.center[1];
        let dz = other.center[2] - self.center[2];
        let dist_sq = dx * dx + dy * dy + dz * dz;
        let radius_sum = self.radius + other.radius;
        dist_sq <= radius_sum * radius_sum
    }
}

// ============================================================================
// Submesh
// ============================================================================

/// A submesh within a mesh.
#[derive(Debug, Clone)]
pub struct Submesh {
    /// Name.
    pub name: String,
    /// Material index.
    pub material: u32,
    /// First index.
    pub index_offset: u32,
    /// Index count.
    pub index_count: u32,
    /// First vertex (base vertex add).
    pub vertex_offset: u32,
    /// Bounding box.
    pub bounds: AABB,
}

impl Submesh {
    /// Create a new submesh.
    pub fn new(material: u32, index_offset: u32, index_count: u32) -> Self {
        Self {
            name: String::new(),
            material,
            index_offset,
            index_count,
            vertex_offset: 0,
            bounds: AABB::INVALID,
        }
    }

    /// Triangle count.
    pub fn triangle_count(&self) -> u32 {
        self.index_count / 3
    }
}

// ============================================================================
// Mesh Data
// ============================================================================

bitflags::bitflags! {
    /// Mesh flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct MeshFlags: u32 {
        /// Has normals.
        const HAS_NORMALS = 1 << 0;
        /// Has tangents.
        const HAS_TANGENTS = 1 << 1;
        /// Has UV set 0.
        const HAS_UV0 = 1 << 2;
        /// Has UV set 1.
        const HAS_UV1 = 1 << 3;
        /// Has vertex colors.
        const HAS_COLORS = 1 << 4;
        /// Has skinning data.
        const HAS_SKINNING = 1 << 5;
        /// Has morph targets.
        const HAS_MORPHS = 1 << 6;
        /// Is dynamic (CPU write, GPU read).
        const DYNAMIC = 1 << 7;
        /// Has been meshletized.
        const MESHLETIZED = 1 << 8;
        /// Uses 32-bit indices.
        const USES_32BIT_INDICES = 1 << 9;
        /// Has ray tracing BLAS.
        const HAS_BLAS = 1 << 10;
        /// Is compressed.
        const COMPRESSED = 1 << 11;
        /// Standard mesh with all features.
        const STANDARD = Self::HAS_NORMALS.bits() | Self::HAS_TANGENTS.bits() | Self::HAS_UV0.bits();
    }
}

/// Mesh description.
#[derive(Debug, Clone)]
pub struct MeshDesc {
    /// Name.
    pub name: String,
    /// Vertex layout.
    pub layout: VertexLayout,
    /// Primitive type.
    pub primitive: MeshPrimitive,
    /// Flags.
    pub flags: MeshFlags,
    /// Expected vertex count.
    pub vertex_count: u32,
    /// Expected index count.
    pub index_count: u32,
}

impl Default for MeshDesc {
    fn default() -> Self {
        Self {
            name: String::new(),
            layout: VertexLayout::standard(),
            primitive: MeshPrimitive::Triangles,
            flags: MeshFlags::STANDARD,
            vertex_count: 0,
            index_count: 0,
        }
    }
}

/// Mesh data container.
#[derive(Debug, Clone)]
pub struct MeshData {
    /// Vertex buffer.
    pub vertices: Vec<u8>,
    /// Index buffer.
    pub indices: IndexData,
    /// Vertex layout.
    pub layout: VertexLayout,
    /// Primitive type.
    pub primitive: MeshPrimitive,
    /// Bounding box.
    pub bounds: AABB,
    /// Bounding sphere.
    pub sphere: BoundingSphere,
}

impl MeshData {
    /// Get vertex count.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len() / self.layout.stride as usize
    }

    /// Get index count.
    pub fn index_count(&self) -> usize {
        self.indices.count()
    }

    /// Get triangle count.
    pub fn triangle_count(&self) -> usize {
        self.indices.count() / 3
    }

    /// Get vertex at index.
    pub fn get_vertex<V: Copy>(&self, index: usize) -> Option<&V> {
        let offset = index * self.layout.stride as usize;
        if offset + core::mem::size_of::<V>() <= self.vertices.len() {
            Some(unsafe { &*(self.vertices.as_ptr().add(offset) as *const V) })
        } else {
            None
        }
    }
}

// ============================================================================
// Mesh
// ============================================================================

/// A mesh resource.
pub struct Mesh {
    /// Handle.
    handle: MeshHandle,
    /// Name.
    name: String,
    /// Mesh data.
    data: MeshData,
    /// Submeshes.
    submeshes: Vec<Submesh>,
    /// Flags.
    flags: MeshFlags,
    /// GPU vertex buffer handle (platform-specific).
    vertex_buffer: u64,
    /// GPU index buffer handle.
    index_buffer: u64,
}

impl Mesh {
    /// Create a new mesh.
    pub fn new(handle: MeshHandle, name: impl Into<String>, data: MeshData) -> Self {
        let flags = MeshFlags::STANDARD
            | if data.indices.format() == IndexFormat::U32 {
                MeshFlags::USES_32BIT_INDICES
            } else {
                MeshFlags::empty()
            };

        Self {
            handle,
            name: name.into(),
            data,
            submeshes: Vec::new(),
            flags,
            vertex_buffer: 0,
            index_buffer: 0,
        }
    }

    /// Get handle.
    pub fn handle(&self) -> MeshHandle {
        self.handle
    }

    /// Get name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get data.
    pub fn data(&self) -> &MeshData {
        &self.data
    }

    /// Get mutable data.
    pub fn data_mut(&mut self) -> &mut MeshData {
        &mut self.data
    }

    /// Get submeshes.
    pub fn submeshes(&self) -> &[Submesh] {
        &self.submeshes
    }

    /// Add a submesh.
    pub fn add_submesh(&mut self, submesh: Submesh) -> usize {
        let index = self.submeshes.len();
        self.submeshes.push(submesh);
        index
    }

    /// Get flags.
    pub fn flags(&self) -> MeshFlags {
        self.flags
    }

    /// Get vertex count.
    pub fn vertex_count(&self) -> usize {
        self.data.vertex_count()
    }

    /// Get index count.
    pub fn index_count(&self) -> usize {
        self.data.index_count()
    }

    /// Get triangle count.
    pub fn triangle_count(&self) -> usize {
        self.data.triangle_count()
    }

    /// Get bounds.
    pub fn bounds(&self) -> &AABB {
        &self.data.bounds
    }

    /// Get bounding sphere.
    pub fn bounding_sphere(&self) -> &BoundingSphere {
        &self.data.sphere
    }

    /// Set GPU buffer handles.
    pub fn set_gpu_buffers(&mut self, vertex: u64, index: u64) {
        self.vertex_buffer = vertex;
        self.index_buffer = index;
    }

    /// Get vertex buffer handle.
    pub fn vertex_buffer(&self) -> u64 {
        self.vertex_buffer
    }

    /// Get index buffer handle.
    pub fn index_buffer(&self) -> u64 {
        self.index_buffer
    }
}

// ============================================================================
// Mesh Builder
// ============================================================================

/// Builder for constructing meshes.
pub struct MeshBuilder {
    name: String,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    tangents: Vec<[f32; 4]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
    submeshes: Vec<(u32, u32, u32)>, // (material, offset, count)
}

impl MeshBuilder {
    /// Create a new builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            positions: Vec::new(),
            normals: Vec::new(),
            tangents: Vec::new(),
            uvs: Vec::new(),
            colors: Vec::new(),
            indices: Vec::new(),
            submeshes: Vec::new(),
        }
    }

    /// Add a vertex.
    pub fn add_vertex(&mut self, position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> u32 {
        let index = self.positions.len() as u32;
        self.positions.push(position);
        self.normals.push(normal);
        self.uvs.push(uv);
        index
    }

    /// Add a triangle.
    pub fn add_triangle(&mut self, v0: u32, v1: u32, v2: u32) {
        self.indices.push(v0);
        self.indices.push(v1);
        self.indices.push(v2);
    }

    /// Add a quad (two triangles).
    pub fn add_quad(&mut self, v0: u32, v1: u32, v2: u32, v3: u32) {
        self.add_triangle(v0, v1, v2);
        self.add_triangle(v0, v2, v3);
    }

    /// Begin a submesh.
    pub fn begin_submesh(&mut self, material: u32) {
        let offset = self.indices.len() as u32;
        self.submeshes.push((material, offset, 0));
    }

    /// End current submesh.
    pub fn end_submesh(&mut self) {
        if let Some(last) = self.submeshes.last_mut() {
            last.2 = self.indices.len() as u32 - last.1;
        }
    }

    /// Set tangent for vertex.
    pub fn set_tangent(&mut self, index: usize, tangent: [f32; 4]) {
        if self.tangents.len() <= index {
            self.tangents
                .resize(self.positions.len(), [1.0, 0.0, 0.0, 1.0]);
        }
        self.tangents[index] = tangent;
    }

    /// Set vertex color.
    pub fn set_color(&mut self, index: usize, color: [f32; 4]) {
        if self.colors.len() <= index {
            self.colors
                .resize(self.positions.len(), [1.0, 1.0, 1.0, 1.0]);
        }
        self.colors[index] = color;
    }

    /// Calculate normals from triangles.
    pub fn calculate_normals(&mut self) {
        self.normals.clear();
        self.normals.resize(self.positions.len(), [0.0, 0.0, 0.0]);

        for i in (0..self.indices.len()).step_by(3) {
            let i0 = self.indices[i] as usize;
            let i1 = self.indices[i + 1] as usize;
            let i2 = self.indices[i + 2] as usize;

            let v0 = self.positions[i0];
            let v1 = self.positions[i1];
            let v2 = self.positions[i2];

            let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

            let n = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];

            self.normals[i0][0] += n[0];
            self.normals[i0][1] += n[1];
            self.normals[i0][2] += n[2];
            self.normals[i1][0] += n[0];
            self.normals[i1][1] += n[1];
            self.normals[i1][2] += n[2];
            self.normals[i2][0] += n[0];
            self.normals[i2][1] += n[1];
            self.normals[i2][2] += n[2];
        }

        for n in &mut self.normals {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len > 0.0 {
                n[0] /= len;
                n[1] /= len;
                n[2] /= len;
            }
        }
    }

    /// Calculate tangents (Mikktspace-like).
    pub fn calculate_tangents(&mut self) {
        self.tangents.clear();
        self.tangents
            .resize(self.positions.len(), [0.0, 0.0, 0.0, 0.0]);

        let mut bitangents = vec![[0.0f32; 3]; self.positions.len()];

        for i in (0..self.indices.len()).step_by(3) {
            let i0 = self.indices[i] as usize;
            let i1 = self.indices[i + 1] as usize;
            let i2 = self.indices[i + 2] as usize;

            let v0 = self.positions[i0];
            let v1 = self.positions[i1];
            let v2 = self.positions[i2];

            let uv0 = self.uvs.get(i0).copied().unwrap_or([0.0, 0.0]);
            let uv1 = self.uvs.get(i1).copied().unwrap_or([1.0, 0.0]);
            let uv2 = self.uvs.get(i2).copied().unwrap_or([0.0, 1.0]);

            let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

            let duv1 = [uv1[0] - uv0[0], uv1[1] - uv0[1]];
            let duv2 = [uv2[0] - uv0[0], uv2[1] - uv0[1]];

            let r = 1.0 / (duv1[0] * duv2[1] - duv2[0] * duv1[1]).max(0.0001);

            let tangent = [
                (e1[0] * duv2[1] - e2[0] * duv1[1]) * r,
                (e1[1] * duv2[1] - e2[1] * duv1[1]) * r,
                (e1[2] * duv2[1] - e2[2] * duv1[1]) * r,
            ];

            let bitangent = [
                (e2[0] * duv1[0] - e1[0] * duv2[0]) * r,
                (e2[1] * duv1[0] - e1[1] * duv2[0]) * r,
                (e2[2] * duv1[0] - e1[2] * duv2[0]) * r,
            ];

            for idx in [i0, i1, i2] {
                self.tangents[idx][0] += tangent[0];
                self.tangents[idx][1] += tangent[1];
                self.tangents[idx][2] += tangent[2];
                bitangents[idx][0] += bitangent[0];
                bitangents[idx][1] += bitangent[1];
                bitangents[idx][2] += bitangent[2];
            }
        }

        // Orthonormalize and calculate handedness
        for i in 0..self.positions.len() {
            let n = self.normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]);
            let t = self.tangents[i];
            let b = bitangents[i];

            // Gram-Schmidt orthonormalize
            let dot = n[0] * t[0] + n[1] * t[1] + n[2] * t[2];
            let tangent = [t[0] - n[0] * dot, t[1] - n[1] * dot, t[2] - n[2] * dot];

            let len = (tangent[0] * tangent[0] + tangent[1] * tangent[1] + tangent[2] * tangent[2])
                .sqrt();
            if len > 0.0 {
                self.tangents[i][0] = tangent[0] / len;
                self.tangents[i][1] = tangent[1] / len;
                self.tangents[i][2] = tangent[2] / len;
            }

            // Calculate handedness
            let cross = [
                n[1] * tangent[2] - n[2] * tangent[1],
                n[2] * tangent[0] - n[0] * tangent[2],
                n[0] * tangent[1] - n[1] * tangent[0],
            ];
            let dot = cross[0] * b[0] + cross[1] * b[1] + cross[2] * b[2];
            self.tangents[i][3] = if dot < 0.0 { -1.0 } else { 1.0 };
        }
    }

    /// Build the mesh.
    pub fn build(mut self, handle: MeshHandle) -> Mesh {
        // Calculate tangents if not set
        if self.tangents.is_empty() {
            self.tangents
                .resize(self.positions.len(), [1.0, 0.0, 0.0, 1.0]);
        }

        // Build vertex buffer
        let mut vertices = Vec::with_capacity(self.positions.len() * Vertex::SIZE);
        let mut bounds = AABB::INVALID;

        for i in 0..self.positions.len() {
            let pos = self.positions[i];
            bounds.expand_point(pos);

            let vertex = Vertex {
                position: pos,
                normal: self.normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]),
                tangent: self
                    .tangents
                    .get(i)
                    .copied()
                    .unwrap_or([1.0, 0.0, 0.0, 1.0]),
                uv0: self.uvs.get(i).copied().unwrap_or([0.0, 0.0]),
            };

            let bytes = unsafe {
                core::slice::from_raw_parts(&vertex as *const Vertex as *const u8, Vertex::SIZE)
            };
            vertices.extend_from_slice(bytes);
        }

        // Build index buffer
        let indices = if self.positions.len() > 65535 {
            IndexData::U32(self.indices)
        } else {
            IndexData::U16(self.indices.iter().map(|&i| i as u16).collect())
        };

        let sphere = BoundingSphere::from_aabb(&bounds);

        let data = MeshData {
            vertices,
            indices,
            layout: VertexLayout::standard(),
            primitive: MeshPrimitive::Triangles,
            bounds,
            sphere,
        };

        let mut mesh = Mesh::new(handle, self.name, data);

        // Add submeshes
        for (material, offset, count) in self.submeshes {
            if count > 0 {
                mesh.add_submesh(Submesh::new(material, offset, count));
            }
        }

        mesh
    }
}

// ============================================================================
// Primitive Generators
// ============================================================================

impl MeshBuilder {
    /// Create a cube.
    pub fn cube(size: f32) -> Self {
        let h = size * 0.5;
        let mut builder = Self::new("Cube");

        // Front face
        let v0 = builder.add_vertex([-h, -h, h], [0.0, 0.0, 1.0], [0.0, 0.0]);
        let v1 = builder.add_vertex([h, -h, h], [0.0, 0.0, 1.0], [1.0, 0.0]);
        let v2 = builder.add_vertex([h, h, h], [0.0, 0.0, 1.0], [1.0, 1.0]);
        let v3 = builder.add_vertex([-h, h, h], [0.0, 0.0, 1.0], [0.0, 1.0]);
        builder.add_quad(v0, v1, v2, v3);

        // Back face
        let v4 = builder.add_vertex([h, -h, -h], [0.0, 0.0, -1.0], [0.0, 0.0]);
        let v5 = builder.add_vertex([-h, -h, -h], [0.0, 0.0, -1.0], [1.0, 0.0]);
        let v6 = builder.add_vertex([-h, h, -h], [0.0, 0.0, -1.0], [1.0, 1.0]);
        let v7 = builder.add_vertex([h, h, -h], [0.0, 0.0, -1.0], [0.0, 1.0]);
        builder.add_quad(v4, v5, v6, v7);

        // Top face
        let v8 = builder.add_vertex([-h, h, h], [0.0, 1.0, 0.0], [0.0, 0.0]);
        let v9 = builder.add_vertex([h, h, h], [0.0, 1.0, 0.0], [1.0, 0.0]);
        let v10 = builder.add_vertex([h, h, -h], [0.0, 1.0, 0.0], [1.0, 1.0]);
        let v11 = builder.add_vertex([-h, h, -h], [0.0, 1.0, 0.0], [0.0, 1.0]);
        builder.add_quad(v8, v9, v10, v11);

        // Bottom face
        let v12 = builder.add_vertex([-h, -h, -h], [0.0, -1.0, 0.0], [0.0, 0.0]);
        let v13 = builder.add_vertex([h, -h, -h], [0.0, -1.0, 0.0], [1.0, 0.0]);
        let v14 = builder.add_vertex([h, -h, h], [0.0, -1.0, 0.0], [1.0, 1.0]);
        let v15 = builder.add_vertex([-h, -h, h], [0.0, -1.0, 0.0], [0.0, 1.0]);
        builder.add_quad(v12, v13, v14, v15);

        // Right face
        let v16 = builder.add_vertex([h, -h, h], [1.0, 0.0, 0.0], [0.0, 0.0]);
        let v17 = builder.add_vertex([h, -h, -h], [1.0, 0.0, 0.0], [1.0, 0.0]);
        let v18 = builder.add_vertex([h, h, -h], [1.0, 0.0, 0.0], [1.0, 1.0]);
        let v19 = builder.add_vertex([h, h, h], [1.0, 0.0, 0.0], [0.0, 1.0]);
        builder.add_quad(v16, v17, v18, v19);

        // Left face
        let v20 = builder.add_vertex([-h, -h, -h], [-1.0, 0.0, 0.0], [0.0, 0.0]);
        let v21 = builder.add_vertex([-h, -h, h], [-1.0, 0.0, 0.0], [1.0, 0.0]);
        let v22 = builder.add_vertex([-h, h, h], [-1.0, 0.0, 0.0], [1.0, 1.0]);
        let v23 = builder.add_vertex([-h, h, -h], [-1.0, 0.0, 0.0], [0.0, 1.0]);
        builder.add_quad(v20, v21, v22, v23);

        builder.calculate_tangents();
        builder
    }

    /// Create a sphere.
    pub fn sphere(radius: f32, segments: u32, rings: u32) -> Self {
        use core::f32::consts::PI;
        let mut builder = Self::new("Sphere");

        for ring in 0..=rings {
            let phi = PI * ring as f32 / rings as f32;
            let y = phi.cos();
            let sin_phi = phi.sin();

            for seg in 0..=segments {
                let theta = 2.0 * PI * seg as f32 / segments as f32;
                let x = theta.cos() * sin_phi;
                let z = theta.sin() * sin_phi;

                let pos = [x * radius, y * radius, z * radius];
                let normal = [x, y, z];
                let uv = [seg as f32 / segments as f32, ring as f32 / rings as f32];

                builder.add_vertex(pos, normal, uv);
            }
        }

        for ring in 0..rings {
            for seg in 0..segments {
                let i0 = ring * (segments + 1) + seg;
                let i1 = i0 + 1;
                let i2 = i0 + segments + 1;
                let i3 = i2 + 1;

                if ring > 0 {
                    builder.add_triangle(i0, i2, i1);
                }
                if ring < rings - 1 {
                    builder.add_triangle(i1, i2, i3);
                }
            }
        }

        builder.calculate_tangents();
        builder
    }

    /// Create a plane.
    pub fn plane(width: f32, height: f32, subdivisions: u32) -> Self {
        let mut builder = Self::new("Plane");
        let hw = width * 0.5;
        let hh = height * 0.5;
        let step_x = width / subdivisions as f32;
        let step_z = height / subdivisions as f32;

        for z in 0..=subdivisions {
            for x in 0..=subdivisions {
                let px = -hw + x as f32 * step_x;
                let pz = -hh + z as f32 * step_z;
                let u = x as f32 / subdivisions as f32;
                let v = z as f32 / subdivisions as f32;

                builder.add_vertex([px, 0.0, pz], [0.0, 1.0, 0.0], [u, v]);
            }
        }

        for z in 0..subdivisions {
            for x in 0..subdivisions {
                let i0 = z * (subdivisions + 1) + x;
                let i1 = i0 + 1;
                let i2 = i0 + subdivisions + 1;
                let i3 = i2 + 1;

                builder.add_quad(i0, i1, i3, i2);
            }
        }

        builder.calculate_tangents();
        builder
    }

    /// Create a cylinder.
    pub fn cylinder(radius: f32, height: f32, segments: u32) -> Self {
        use core::f32::consts::PI;
        let mut builder = Self::new("Cylinder");
        let hh = height * 0.5;

        // Side vertices
        for i in 0..=segments {
            let theta = 2.0 * PI * i as f32 / segments as f32;
            let x = theta.cos();
            let z = theta.sin();
            let u = i as f32 / segments as f32;

            builder.add_vertex([x * radius, -hh, z * radius], [x, 0.0, z], [u, 0.0]);
            builder.add_vertex([x * radius, hh, z * radius], [x, 0.0, z], [u, 1.0]);
        }

        // Side faces
        for i in 0..segments {
            let i0 = i * 2;
            let i1 = i0 + 1;
            let i2 = i0 + 2;
            let i3 = i0 + 3;
            builder.add_quad(i0, i2, i3, i1);
        }

        // Top cap center
        let top_center = builder.add_vertex([0.0, hh, 0.0], [0.0, 1.0, 0.0], [0.5, 0.5]);
        // Bottom cap center
        let bottom_center = builder.add_vertex([0.0, -hh, 0.0], [0.0, -1.0, 0.0], [0.5, 0.5]);

        // Cap vertices
        let top_start = builder.positions.len() as u32;
        for i in 0..=segments {
            let theta = 2.0 * PI * i as f32 / segments as f32;
            let x = theta.cos();
            let z = theta.sin();
            let u = (x + 1.0) * 0.5;
            let v = (z + 1.0) * 0.5;

            builder.add_vertex([x * radius, hh, z * radius], [0.0, 1.0, 0.0], [u, v]);
            builder.add_vertex([x * radius, -hh, z * radius], [0.0, -1.0, 0.0], [u, v]);
        }

        // Cap faces
        for i in 0..segments {
            let t0 = top_start + i * 2;
            let t1 = top_start + (i + 1) * 2;
            builder.add_triangle(top_center, t0, t1);

            let b0 = top_start + i * 2 + 1;
            let b1 = top_start + (i + 1) * 2 + 1;
            builder.add_triangle(bottom_center, b1, b0);
        }

        builder.calculate_tangents();
        builder
    }
}

// ============================================================================
// Mesh Manager
// ============================================================================

/// Mesh manager.
pub struct MeshManager {
    /// Meshes.
    meshes: BTreeMap<u32, Mesh>,
    /// Name to handle map.
    name_map: BTreeMap<String, MeshHandle>,
    /// Next index.
    next_index: AtomicU32,
    /// Next generation.
    next_generation: AtomicU32,
    /// Statistics.
    stats: MeshStats,
}

/// Mesh statistics.
#[derive(Debug, Clone, Default)]
pub struct MeshStats {
    /// Total meshes.
    pub mesh_count: u32,
    /// Total vertices.
    pub total_vertices: u64,
    /// Total triangles.
    pub total_triangles: u64,
    /// Total memory (bytes).
    pub memory_bytes: u64,
}

impl MeshManager {
    /// Create a new manager.
    pub fn new() -> Self {
        Self {
            meshes: BTreeMap::new(),
            name_map: BTreeMap::new(),
            next_index: AtomicU32::new(0),
            next_generation: AtomicU32::new(1),
            stats: MeshStats::default(),
        }
    }

    /// Create a mesh.
    pub fn create(&mut self, builder: MeshBuilder) -> MeshHandle {
        let index = self.next_index.fetch_add(1, Ordering::Relaxed);
        let generation = self.next_generation.fetch_add(1, Ordering::Relaxed);
        let handle = MeshHandle::new(index, generation);

        let name = builder.name.clone();
        let mesh = builder.build(handle);

        // Update stats
        self.stats.mesh_count += 1;
        self.stats.total_vertices += mesh.vertex_count() as u64;
        self.stats.total_triangles += mesh.triangle_count() as u64;
        self.stats.memory_bytes += mesh.data.vertices.len() as u64;
        self.stats.memory_bytes += match &mesh.data.indices {
            IndexData::U16(v) => v.len() as u64 * 2,
            IndexData::U32(v) => v.len() as u64 * 4,
        };

        self.name_map.insert(name, handle);
        self.meshes.insert(index, mesh);

        handle
    }

    /// Get a mesh.
    pub fn get(&self, handle: MeshHandle) -> Option<&Mesh> {
        let mesh = self.meshes.get(&handle.index)?;
        if mesh.handle.generation == handle.generation {
            Some(mesh)
        } else {
            None
        }
    }

    /// Get mutable mesh.
    pub fn get_mut(&mut self, handle: MeshHandle) -> Option<&mut Mesh> {
        let mesh = self.meshes.get_mut(&handle.index)?;
        if mesh.handle.generation == handle.generation {
            Some(mesh)
        } else {
            None
        }
    }

    /// Get by name.
    pub fn get_by_name(&self, name: &str) -> Option<&Mesh> {
        let handle = self.name_map.get(name)?;
        self.get(*handle)
    }

    /// Destroy a mesh.
    pub fn destroy(&mut self, handle: MeshHandle) -> bool {
        if let Some(mesh) = self.meshes.remove(&handle.index) {
            if mesh.handle.generation == handle.generation {
                self.name_map.remove(&mesh.name);
                self.stats.mesh_count -= 1;
                self.stats.total_vertices -= mesh.vertex_count() as u64;
                self.stats.total_triangles -= mesh.triangle_count() as u64;
                return true;
            }
        }
        false
    }

    /// Get statistics.
    pub fn stats(&self) -> &MeshStats {
        &self.stats
    }

    /// Iterate over meshes.
    pub fn iter(&self) -> impl Iterator<Item = &Mesh> {
        self.meshes.values()
    }
}

impl Default for MeshManager {
    fn default() -> Self {
        Self::new()
    }
}
