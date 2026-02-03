//! Acceleration structure types for ray tracing
//!
//! This module provides types for building and managing acceleration structures.

use core::mem::size_of;

/// Acceleration structure type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AccelerationStructureType {
    /// Bottom-level (contains geometry)
    BottomLevel = 0,
    /// Top-level (contains instances)
    TopLevel = 1,
}

/// Acceleration structure handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AccelerationStructureHandle(pub u64);

impl AccelerationStructureHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates a new handle
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Checks if null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Geometry type for BLAS
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GeometryType {
    /// Triangle geometry
    Triangles = 0,
    /// AABB geometry (procedural)
    Aabbs = 1,
    /// Instances (for TLAS)
    Instances = 2,
}

/// Geometry flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct GeometryFlags(pub u32);

impl GeometryFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Geometry is opaque (no any-hit shader)
    pub const OPAQUE: Self = Self(1 << 0);
    /// No duplicate any-hit invocation
    pub const NO_DUPLICATE_ANY_HIT: Self = Self(1 << 1);

    /// Contains flag
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl core::ops::BitOr for GeometryFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Build flags for acceleration structures
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct BuildAccelerationStructureFlags(pub u32);

impl BuildAccelerationStructureFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Allow updates
    pub const ALLOW_UPDATE: Self = Self(1 << 0);
    /// Allow compaction
    pub const ALLOW_COMPACTION: Self = Self(1 << 1);
    /// Prefer fast trace
    pub const PREFER_FAST_TRACE: Self = Self(1 << 2);
    /// Prefer fast build
    pub const PREFER_FAST_BUILD: Self = Self(1 << 3);
    /// Low memory
    pub const LOW_MEMORY: Self = Self(1 << 4);

    /// Static geometry (optimized for trace)
    pub const STATIC: Self = Self(Self::PREFER_FAST_TRACE.0 | Self::ALLOW_COMPACTION.0);

    /// Dynamic geometry (optimized for rebuild)
    pub const DYNAMIC: Self = Self(Self::PREFER_FAST_BUILD.0 | Self::ALLOW_UPDATE.0);
}

impl core::ops::BitOr for BuildAccelerationStructureFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Build mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BuildMode {
    /// Full build
    Build = 0,
    /// Update (refit)
    Update = 1,
}

/// Triangle geometry description
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TriangleGeometry {
    /// Vertex buffer address
    pub vertex_buffer_address: u64,
    /// Index buffer address (0 for non-indexed)
    pub index_buffer_address: u64,
    /// Transform buffer address (0 for identity)
    pub transform_buffer_address: u64,
    /// Vertex stride
    pub vertex_stride: u32,
    /// Vertex count
    pub vertex_count: u32,
    /// Index count (0 for non-indexed)
    pub index_count: u32,
    /// Max vertex (for bounds)
    pub max_vertex: u32,
    /// Vertex format
    pub vertex_format: VertexFormat,
    /// Index type
    pub index_type: IndexType,
    /// Geometry flags
    pub flags: GeometryFlags,
    /// Padding
    _pad: u32,
}

impl Default for TriangleGeometry {
    fn default() -> Self {
        Self {
            vertex_buffer_address: 0,
            index_buffer_address: 0,
            transform_buffer_address: 0,
            vertex_stride: 12,
            vertex_count: 0,
            index_count: 0,
            max_vertex: 0,
            vertex_format: VertexFormat::Float3,
            index_type: IndexType::Uint32,
            flags: GeometryFlags::OPAQUE,
            _pad: 0,
        }
    }
}

impl TriangleGeometry {
    /// Creates indexed triangle geometry
    pub const fn indexed(
        vertex_buffer: u64,
        vertex_stride: u32,
        vertex_count: u32,
        index_buffer: u64,
        index_count: u32,
    ) -> Self {
        Self {
            vertex_buffer_address: vertex_buffer,
            index_buffer_address: index_buffer,
            transform_buffer_address: 0,
            vertex_stride,
            vertex_count,
            index_count,
            max_vertex: vertex_count.saturating_sub(1),
            vertex_format: VertexFormat::Float3,
            index_type: IndexType::Uint32,
            flags: GeometryFlags::OPAQUE,
            _pad: 0,
        }
    }

    /// Creates non-indexed triangle geometry
    pub const fn non_indexed(
        vertex_buffer: u64,
        vertex_stride: u32,
        vertex_count: u32,
    ) -> Self {
        Self {
            vertex_buffer_address: vertex_buffer,
            index_buffer_address: 0,
            transform_buffer_address: 0,
            vertex_stride,
            vertex_count,
            index_count: 0,
            max_vertex: vertex_count.saturating_sub(1),
            vertex_format: VertexFormat::Float3,
            index_type: IndexType::None,
            flags: GeometryFlags::OPAQUE,
            _pad: 0,
        }
    }

    /// With transform
    pub const fn with_transform(mut self, transform_buffer: u64) -> Self {
        self.transform_buffer_address = transform_buffer;
        self
    }

    /// With flags
    pub const fn with_flags(mut self, flags: GeometryFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Triangle count
    pub const fn triangle_count(&self) -> u32 {
        if self.index_count > 0 {
            self.index_count / 3
        } else {
            self.vertex_count / 3
        }
    }
}

/// Vertex format for acceleration structure
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum VertexFormat {
    /// 3x 32-bit float
    #[default]
    Float3 = 0,
    /// 2x 32-bit float (XY, Z=0)
    Float2 = 1,
    /// 3x 16-bit float
    Half3 = 2,
    /// 2x 16-bit float
    Half2 = 3,
    /// 3x 16-bit signed normalized
    Snorm16x3 = 4,
    /// 2x 16-bit signed normalized
    Snorm16x2 = 5,
}

impl VertexFormat {
    /// Size in bytes
    pub const fn size(&self) -> u32 {
        match self {
            Self::Float3 => 12,
            Self::Float2 => 8,
            Self::Half3 => 6,
            Self::Half2 => 4,
            Self::Snorm16x3 => 6,
            Self::Snorm16x2 => 4,
        }
    }
}

/// Index type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum IndexType {
    /// No indices
    None = 0,
    /// 16-bit indices
    Uint16 = 1,
    /// 32-bit indices
    #[default]
    Uint32 = 2,
}

impl IndexType {
    /// Size in bytes
    pub const fn size(&self) -> u32 {
        match self {
            Self::None => 0,
            Self::Uint16 => 2,
            Self::Uint32 => 4,
        }
    }
}

/// AABB geometry for procedural geometry
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AabbGeometry {
    /// AABB data buffer address
    pub data_address: u64,
    /// Number of AABBs
    pub count: u32,
    /// Stride between AABBs
    pub stride: u32,
    /// Geometry flags
    pub flags: GeometryFlags,
    /// Padding
    _pad: u32,
}

impl Default for AabbGeometry {
    fn default() -> Self {
        Self {
            data_address: 0,
            count: 0,
            stride: 24, // 6 floats
            flags: GeometryFlags::NONE,
            _pad: 0,
        }
    }
}

impl AabbGeometry {
    /// Creates AABB geometry
    pub const fn new(data_address: u64, count: u32, stride: u32) -> Self {
        Self {
            data_address,
            count,
            stride,
            flags: GeometryFlags::NONE,
            _pad: 0,
        }
    }
}

/// AABB data for procedural geometry
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AabbData {
    /// Minimum corner
    pub min: [f32; 3],
    /// Maximum corner
    pub max: [f32; 3],
}

impl AabbData {
    /// Size in bytes
    pub const SIZE: u32 = 24;

    /// Creates new AABB data
    pub const fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }
}

/// Instance description for TLAS
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct InstanceDesc {
    /// 3x4 row-major transform matrix
    pub transform: [[f32; 4]; 3],
    /// Instance custom index (24 bits) and mask (8 bits)
    pub instance_custom_index_and_mask: u32,
    /// SBT record offset (24 bits) and flags (8 bits)
    pub sbt_offset_and_flags: u32,
    /// BLAS device address
    pub acceleration_structure_reference: u64,
}

impl Default for InstanceDesc {
    fn default() -> Self {
        Self::identity()
    }
}

impl InstanceDesc {
    /// Size in bytes
    pub const SIZE: u32 = 64;

    /// Creates identity instance
    pub const fn identity() -> Self {
        Self {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
            ],
            instance_custom_index_and_mask: 0xFF << 24,
            sbt_offset_and_flags: 0,
            acceleration_structure_reference: 0,
        }
    }

    /// Creates with BLAS reference
    pub const fn with_blas(mut self, blas_address: u64) -> Self {
        self.acceleration_structure_reference = blas_address;
        self
    }

    /// With transform
    pub const fn with_transform(mut self, transform: [[f32; 4]; 3]) -> Self {
        self.transform = transform;
        self
    }

    /// With custom index (0-16777215)
    pub const fn with_custom_index(mut self, index: u32) -> Self {
        let mask = self.instance_custom_index_and_mask & 0xFF00_0000;
        self.instance_custom_index_and_mask = mask | (index & 0x00FF_FFFF);
        self
    }

    /// With mask (0-255)
    pub const fn with_mask(mut self, mask: u8) -> Self {
        let index = self.instance_custom_index_and_mask & 0x00FF_FFFF;
        self.instance_custom_index_and_mask = ((mask as u32) << 24) | index;
        self
    }

    /// With SBT offset (0-16777215)
    pub const fn with_sbt_offset(mut self, offset: u32) -> Self {
        let flags = self.sbt_offset_and_flags & 0xFF00_0000;
        self.sbt_offset_and_flags = flags | (offset & 0x00FF_FFFF);
        self
    }

    /// With instance flags (0-255)
    pub const fn with_flags(mut self, flags: u8) -> Self {
        let offset = self.sbt_offset_and_flags & 0x00FF_FFFF;
        self.sbt_offset_and_flags = ((flags as u32) << 24) | offset;
        self
    }

    /// Gets custom index
    pub const fn custom_index(&self) -> u32 {
        self.instance_custom_index_and_mask & 0x00FF_FFFF
    }

    /// Gets mask
    pub const fn mask(&self) -> u8 {
        (self.instance_custom_index_and_mask >> 24) as u8
    }

    /// Gets SBT offset
    pub const fn sbt_offset(&self) -> u32 {
        self.sbt_offset_and_flags & 0x00FF_FFFF
    }

    /// Gets flags
    pub const fn flags(&self) -> u8 {
        (self.sbt_offset_and_flags >> 24) as u8
    }
}

/// Instance flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct InstanceFlags(pub u8);

impl InstanceFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Disable triangle culling
    pub const TRIANGLE_FACING_CULL_DISABLE: Self = Self(1 << 0);
    /// Flip triangle winding
    pub const TRIANGLE_FLIP_FACING: Self = Self(1 << 1);
    /// Force opaque
    pub const FORCE_OPAQUE: Self = Self(1 << 2);
    /// Force non-opaque
    pub const FORCE_NO_OPAQUE: Self = Self(1 << 3);
}

impl core::ops::BitOr for InstanceFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Acceleration structure build sizes
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AccelerationStructureBuildSizes {
    /// Size of acceleration structure buffer
    pub acceleration_structure_size: u64,
    /// Size of build scratch buffer
    pub build_scratch_size: u64,
    /// Size of update scratch buffer
    pub update_scratch_size: u64,
}

impl AccelerationStructureBuildSizes {
    /// Total memory required for build
    pub const fn total_build_memory(&self) -> u64 {
        self.acceleration_structure_size + self.build_scratch_size
    }
}

/// Acceleration structure create info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AccelerationStructureCreateInfo {
    /// Type
    pub ty: AccelerationStructureType,
    /// Size
    pub size: u64,
    /// Buffer offset
    pub offset: u64,
}

impl AccelerationStructureCreateInfo {
    /// Creates for BLAS
    pub const fn blas(size: u64) -> Self {
        Self {
            ty: AccelerationStructureType::BottomLevel,
            size,
            offset: 0,
        }
    }

    /// Creates for TLAS
    pub const fn tlas(size: u64) -> Self {
        Self {
            ty: AccelerationStructureType::TopLevel,
            size,
            offset: 0,
        }
    }

    /// With buffer offset
    pub const fn with_offset(mut self, offset: u64) -> Self {
        self.offset = offset;
        self
    }
}

/// Compaction query result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct CompactionSizeQuery {
    /// Compacted size
    pub compacted_size: u64,
}

/// Serialization info for AS
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AccelerationStructureSerializationInfo {
    /// Serialized size in bytes
    pub size: u64,
    /// Number of bottom-level handles (for TLAS)
    pub num_bottom_level_handles: u64,
}

/// Copy mode for acceleration structures
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CopyAccelerationStructureMode {
    /// Clone
    Clone = 0,
    /// Compact
    Compact = 1,
    /// Serialize
    Serialize = 2,
    /// Deserialize
    Deserialize = 3,
}
