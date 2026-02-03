//! Ray tracing types and utilities
//!
//! This module provides types for hardware-accelerated ray tracing.

extern crate alloc;
use alloc::vec::Vec;

/// Handle to an acceleration structure
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AccelerationStructureHandle(pub u64);

impl AccelerationStructureHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Checks if this is a null handle
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Type of acceleration structure
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AccelerationStructureType {
    /// Bottom-level AS (contains geometry)
    BottomLevel,
    /// Top-level AS (contains instances)
    TopLevel,
    /// Generic (can be either)
    Generic,
}

/// Build flags for acceleration structures
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AccelerationStructureBuildFlags(pub u32);

impl AccelerationStructureBuildFlags {
    /// No special flags
    pub const NONE: Self = Self(0);
    /// Allow updating the structure
    pub const ALLOW_UPDATE: Self = Self(1 << 0);
    /// Allow compacting the structure
    pub const ALLOW_COMPACTION: Self = Self(1 << 1);
    /// Prefer fast trace over fast build
    pub const PREFER_FAST_TRACE: Self = Self(1 << 2);
    /// Prefer fast build over fast trace
    pub const PREFER_FAST_BUILD: Self = Self(1 << 3);
    /// Minimize memory usage
    pub const LOW_MEMORY: Self = Self(1 << 4);
}

impl core::ops::BitOr for AccelerationStructureBuildFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for AccelerationStructureBuildFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Geometry type for BLAS
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GeometryType {
    /// Triangle mesh
    Triangles,
    /// Axis-aligned bounding boxes
    Aabbs,
    /// Instances (for TLAS)
    Instances,
}

/// Geometry flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GeometryFlags(pub u32);

impl GeometryFlags {
    /// No special flags
    pub const NONE: Self = Self(0);
    /// Geometry is opaque (skip any-hit shader)
    pub const OPAQUE: Self = Self(1 << 0);
    /// Disable culling for this geometry
    pub const NO_DUPLICATE_ANY_HIT: Self = Self(1 << 1);
}

impl core::ops::BitOr for GeometryFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Instance flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct InstanceFlags(pub u32);

impl InstanceFlags {
    /// No special flags
    pub const NONE: Self = Self(0);
    /// Disable triangle culling
    pub const TRIANGLE_CULL_DISABLE: Self = Self(1 << 0);
    /// Flip triangle facing
    pub const TRIANGLE_FLIP_FACING: Self = Self(1 << 1);
    /// Force opaque
    pub const FORCE_OPAQUE: Self = Self(1 << 2);
    /// Force non-opaque
    pub const FORCE_NON_OPAQUE: Self = Self(1 << 3);
}

impl core::ops::BitOr for InstanceFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Vertex format for ray tracing geometry
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RtVertexFormat {
    /// 32-bit float, 3 components
    Float32x3,
    /// 32-bit float, 2 components
    Float32x2,
    /// 16-bit float, 4 components
    Float16x4,
    /// 16-bit float, 2 components
    Float16x2,
    /// 16-bit signed normalized, 4 components
    Snorm16x4,
    /// 16-bit signed normalized, 2 components
    Snorm16x2,
}

impl RtVertexFormat {
    /// Returns the size in bytes
    pub const fn size(&self) -> u32 {
        match self {
            Self::Float32x3 => 12,
            Self::Float32x2 => 8,
            Self::Float16x4 => 8,
            Self::Float16x2 => 4,
            Self::Snorm16x4 => 8,
            Self::Snorm16x2 => 4,
        }
    }
}

/// Index type for ray tracing geometry
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RtIndexType {
    /// No indices (non-indexed geometry)
    None,
    /// 16-bit indices
    Uint16,
    /// 32-bit indices
    Uint32,
}

impl RtIndexType {
    /// Returns the size in bytes
    pub const fn size(&self) -> u32 {
        match self {
            Self::None => 0,
            Self::Uint16 => 2,
            Self::Uint32 => 4,
        }
    }
}

/// Triangle geometry description
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TrianglesGeometry {
    /// Vertex buffer address
    pub vertex_buffer: u64,
    /// Vertex stride
    pub vertex_stride: u32,
    /// Vertex format
    pub vertex_format: RtVertexFormat,
    /// Number of vertices
    pub vertex_count: u32,
    /// Index buffer address (0 for non-indexed)
    pub index_buffer: u64,
    /// Index type
    pub index_type: RtIndexType,
    /// Number of indices
    pub index_count: u32,
    /// Transform buffer address (0 for identity)
    pub transform_buffer: u64,
    /// Geometry flags
    pub flags: GeometryFlags,
}

impl TrianglesGeometry {
    /// Creates a new triangles geometry description
    pub const fn new(
        vertex_buffer: u64,
        vertex_stride: u32,
        vertex_format: RtVertexFormat,
        vertex_count: u32,
    ) -> Self {
        Self {
            vertex_buffer,
            vertex_stride,
            vertex_format,
            vertex_count,
            index_buffer: 0,
            index_type: RtIndexType::None,
            index_count: 0,
            transform_buffer: 0,
            flags: GeometryFlags::NONE,
        }
    }

    /// Adds index buffer
    pub const fn with_indices(
        mut self,
        buffer: u64,
        index_type: RtIndexType,
        count: u32,
    ) -> Self {
        self.index_buffer = buffer;
        self.index_type = index_type;
        self.index_count = count;
        self
    }

    /// Adds transform buffer
    pub const fn with_transform(mut self, buffer: u64) -> Self {
        self.transform_buffer = buffer;
        self
    }

    /// Sets geometry flags
    pub const fn with_flags(mut self, flags: GeometryFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Returns the number of primitives (triangles)
    pub const fn primitive_count(&self) -> u32 {
        if self.index_count > 0 {
            self.index_count / 3
        } else {
            self.vertex_count / 3
        }
    }
}

/// AABB geometry description
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AabbsGeometry {
    /// AABB buffer address
    pub aabb_buffer: u64,
    /// Stride between AABBs
    pub stride: u32,
    /// Number of AABBs
    pub count: u32,
    /// Geometry flags
    pub flags: GeometryFlags,
}

impl AabbsGeometry {
    /// Creates a new AABBs geometry description
    pub const fn new(buffer: u64, stride: u32, count: u32) -> Self {
        Self {
            aabb_buffer: buffer,
            stride,
            count,
            flags: GeometryFlags::NONE,
        }
    }

    /// Sets geometry flags
    pub const fn with_flags(mut self, flags: GeometryFlags) -> Self {
        self.flags = flags;
        self
    }
}

/// Geometry description for BLAS
#[derive(Clone, Copy, Debug)]
pub enum BlasGeometry {
    /// Triangle mesh
    Triangles(TrianglesGeometry),
    /// Procedural AABBs
    Aabbs(AabbsGeometry),
}

/// Bottom-level acceleration structure description
#[derive(Clone, Debug)]
pub struct BlasDesc {
    /// Geometry descriptions
    pub geometries: Vec<BlasGeometry>,
    /// Build flags
    pub flags: AccelerationStructureBuildFlags,
}

impl BlasDesc {
    /// Creates a new BLAS description
    pub fn new() -> Self {
        Self {
            geometries: Vec::new(),
            flags: AccelerationStructureBuildFlags::PREFER_FAST_TRACE,
        }
    }

    /// Adds triangle geometry
    pub fn add_triangles(mut self, geometry: TrianglesGeometry) -> Self {
        self.geometries.push(BlasGeometry::Triangles(geometry));
        self
    }

    /// Adds AABB geometry
    pub fn add_aabbs(mut self, geometry: AabbsGeometry) -> Self {
        self.geometries.push(BlasGeometry::Aabbs(geometry));
        self
    }

    /// Sets build flags
    pub fn with_flags(mut self, flags: AccelerationStructureBuildFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Total number of primitives
    pub fn primitive_count(&self) -> u32 {
        self.geometries
            .iter()
            .map(|g| match g {
                BlasGeometry::Triangles(t) => t.primitive_count(),
                BlasGeometry::Aabbs(a) => a.count,
            })
            .sum()
    }
}

impl Default for BlasDesc {
    fn default() -> Self {
        Self::new()
    }
}

/// Instance description for TLAS
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TlasInstance {
    /// 3x4 row-major transform matrix
    pub transform: [[f32; 4]; 3],
    /// Instance custom index (24 bits) and mask (8 bits)
    pub instance_custom_index_and_mask: u32,
    /// Shader binding table offset (24 bits) and flags (8 bits)
    pub sbt_offset_and_flags: u32,
    /// Bottom-level AS device address
    pub blas_address: u64,
}

impl TlasInstance {
    /// Creates a new instance with identity transform
    pub const fn new(blas_address: u64) -> Self {
        Self {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
            ],
            instance_custom_index_and_mask: 0xFF << 24, // mask = 0xFF
            sbt_offset_and_flags: 0,
            blas_address,
        }
    }

    /// Sets the 3x4 transform matrix
    pub fn with_transform(mut self, transform: [[f32; 4]; 3]) -> Self {
        self.transform = transform;
        self
    }

    /// Sets the custom index (0-16777215)
    pub fn with_custom_index(mut self, index: u32) -> Self {
        self.instance_custom_index_and_mask =
            (self.instance_custom_index_and_mask & 0xFF000000) | (index & 0x00FFFFFF);
        self
    }

    /// Sets the visibility mask (0-255)
    pub fn with_mask(mut self, mask: u8) -> Self {
        self.instance_custom_index_and_mask =
            (self.instance_custom_index_and_mask & 0x00FFFFFF) | ((mask as u32) << 24);
        self
    }

    /// Sets the SBT offset (0-16777215)
    pub fn with_sbt_offset(mut self, offset: u32) -> Self {
        self.sbt_offset_and_flags =
            (self.sbt_offset_and_flags & 0xFF000000) | (offset & 0x00FFFFFF);
        self
    }

    /// Sets instance flags
    pub fn with_flags(mut self, flags: InstanceFlags) -> Self {
        self.sbt_offset_and_flags =
            (self.sbt_offset_and_flags & 0x00FFFFFF) | ((flags.0 as u32) << 24);
        self
    }
}

/// Top-level acceleration structure description
#[derive(Clone, Debug)]
pub struct TlasDesc {
    /// Instance buffer address
    pub instance_buffer: u64,
    /// Number of instances
    pub instance_count: u32,
    /// Build flags
    pub flags: AccelerationStructureBuildFlags,
}

impl TlasDesc {
    /// Creates a new TLAS description
    pub const fn new(instance_buffer: u64, instance_count: u32) -> Self {
        Self {
            instance_buffer,
            instance_count,
            flags: AccelerationStructureBuildFlags::PREFER_FAST_TRACE,
        }
    }

    /// Sets build flags
    pub const fn with_flags(mut self, flags: AccelerationStructureBuildFlags) -> Self {
        self.flags = flags;
        self
    }
}

/// Build mode for acceleration structures
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BuildMode {
    /// Full build
    Build,
    /// Update existing structure
    Update,
}

/// Scratch buffer sizes for AS build
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AccelerationStructureSizes {
    /// Size of the acceleration structure itself
    pub acceleration_structure_size: u64,
    /// Size of scratch buffer for build
    pub build_scratch_size: u64,
    /// Size of scratch buffer for update
    pub update_scratch_size: u64,
}

/// Build info for acceleration structure
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AccelerationStructureBuildInfo {
    /// Type of structure to build
    pub structure_type: AccelerationStructureType,
    /// Build mode
    pub mode: BuildMode,
    /// Destination structure
    pub dst: AccelerationStructureHandle,
    /// Source structure (for updates)
    pub src: AccelerationStructureHandle,
    /// Scratch buffer address
    pub scratch_address: u64,
}

impl AccelerationStructureBuildInfo {
    /// Creates build info for a new structure
    pub const fn new_build(
        structure_type: AccelerationStructureType,
        dst: AccelerationStructureHandle,
        scratch_address: u64,
    ) -> Self {
        Self {
            structure_type,
            mode: BuildMode::Build,
            dst,
            src: AccelerationStructureHandle::NULL,
            scratch_address,
        }
    }

    /// Creates build info for an update
    pub const fn new_update(
        structure_type: AccelerationStructureType,
        dst: AccelerationStructureHandle,
        src: AccelerationStructureHandle,
        scratch_address: u64,
    ) -> Self {
        Self {
            structure_type,
            mode: BuildMode::Update,
            dst,
            src,
            scratch_address,
        }
    }
}

/// Ray flags for tracing
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RayFlags(pub u32);

impl RayFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Force opaque (skip any-hit shaders)
    pub const FORCE_OPAQUE: Self = Self(1 << 0);
    /// Force non-opaque
    pub const FORCE_NON_OPAQUE: Self = Self(1 << 1);
    /// Terminate on first hit
    pub const TERMINATE_ON_FIRST_HIT: Self = Self(1 << 2);
    /// Skip closest hit shader
    pub const SKIP_CLOSEST_HIT: Self = Self(1 << 3);
    /// Cull back-facing triangles
    pub const CULL_BACK_FACING: Self = Self(1 << 4);
    /// Cull front-facing triangles
    pub const CULL_FRONT_FACING: Self = Self(1 << 5);
    /// Cull opaque geometry
    pub const CULL_OPAQUE: Self = Self(1 << 6);
    /// Cull non-opaque geometry
    pub const CULL_NON_OPAQUE: Self = Self(1 << 7);
    /// Skip triangles
    pub const SKIP_TRIANGLES: Self = Self(1 << 8);
    /// Skip AABBs
    pub const SKIP_AABBS: Self = Self(1 << 9);
}

impl core::ops::BitOr for RayFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for RayFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Ray description for tracing
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RayDesc {
    /// Ray origin
    pub origin: [f32; 3],
    /// Minimum T value
    pub t_min: f32,
    /// Ray direction
    pub direction: [f32; 3],
    /// Maximum T value
    pub t_max: f32,
}

impl RayDesc {
    /// Creates a new ray description
    pub const fn new(origin: [f32; 3], direction: [f32; 3]) -> Self {
        Self {
            origin,
            t_min: 0.001,
            direction,
            t_max: f32::MAX,
        }
    }

    /// Sets T min/max bounds
    pub const fn with_bounds(mut self, t_min: f32, t_max: f32) -> Self {
        self.t_min = t_min;
        self.t_max = t_max;
        self
    }
}

/// Hit kind for shader reporting
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum HitKind {
    /// Front-facing triangle
    FrontFace = 0xFE,
    /// Back-facing triangle
    BackFace = 0xFF,
    /// Custom hit (procedural)
    Custom(u8) = 0,
}

/// Shader binding table entry
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ShaderBindingTableEntry {
    /// Shader group handle
    pub shader_group_handle: [u8; 32],
    /// Local root signature data (optional)
    pub local_data_offset: u32,
    /// Size of local data
    pub local_data_size: u32,
}

/// Shader binding table region
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ShaderBindingTableRegion {
    /// Device address of the region
    pub device_address: u64,
    /// Stride between entries
    pub stride: u64,
    /// Total size of the region
    pub size: u64,
}

impl ShaderBindingTableRegion {
    /// Empty region
    pub const EMPTY: Self = Self {
        device_address: 0,
        stride: 0,
        size: 0,
    };

    /// Creates a new SBT region
    pub const fn new(device_address: u64, stride: u64, size: u64) -> Self {
        Self {
            device_address,
            stride,
            size,
        }
    }
}

/// Shader binding table description
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ShaderBindingTableDesc {
    /// Ray generation shaders
    pub raygen: ShaderBindingTableRegion,
    /// Miss shaders
    pub miss: ShaderBindingTableRegion,
    /// Hit group shaders
    pub hit: ShaderBindingTableRegion,
    /// Callable shaders
    pub callable: ShaderBindingTableRegion,
}

impl ShaderBindingTableDesc {
    /// Creates a new empty SBT description
    pub const fn new() -> Self {
        Self {
            raygen: ShaderBindingTableRegion::EMPTY,
            miss: ShaderBindingTableRegion::EMPTY,
            hit: ShaderBindingTableRegion::EMPTY,
            callable: ShaderBindingTableRegion::EMPTY,
        }
    }

    /// Sets raygen region
    pub const fn with_raygen(mut self, region: ShaderBindingTableRegion) -> Self {
        self.raygen = region;
        self
    }

    /// Sets miss region
    pub const fn with_miss(mut self, region: ShaderBindingTableRegion) -> Self {
        self.miss = region;
        self
    }

    /// Sets hit region
    pub const fn with_hit(mut self, region: ShaderBindingTableRegion) -> Self {
        self.hit = region;
        self
    }

    /// Sets callable region
    pub const fn with_callable(mut self, region: ShaderBindingTableRegion) -> Self {
        self.callable = region;
        self
    }
}

impl Default for ShaderBindingTableDesc {
    fn default() -> Self {
        Self::new()
    }
}

/// Ray tracing pipeline shader group type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ShaderGroupType {
    /// General (raygen, miss, callable)
    General,
    /// Triangle hit group
    TrianglesHitGroup,
    /// Procedural hit group
    ProceduralHitGroup,
}

/// Ray tracing shader group description
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RtShaderGroup {
    /// Group type
    pub group_type: ShaderGroupType,
    /// General shader index (or UNUSED)
    pub general_shader: u32,
    /// Closest hit shader index (or UNUSED)
    pub closest_hit_shader: u32,
    /// Any hit shader index (or UNUSED)
    pub any_hit_shader: u32,
    /// Intersection shader index (or UNUSED)
    pub intersection_shader: u32,
}

impl RtShaderGroup {
    /// Unused shader index
    pub const UNUSED: u32 = u32::MAX;

    /// Creates a general shader group (raygen, miss, callable)
    pub const fn general(shader_index: u32) -> Self {
        Self {
            group_type: ShaderGroupType::General,
            general_shader: shader_index,
            closest_hit_shader: Self::UNUSED,
            any_hit_shader: Self::UNUSED,
            intersection_shader: Self::UNUSED,
        }
    }

    /// Creates a triangle hit group
    pub const fn triangles_hit(closest_hit: u32, any_hit: u32) -> Self {
        Self {
            group_type: ShaderGroupType::TrianglesHitGroup,
            general_shader: Self::UNUSED,
            closest_hit_shader: closest_hit,
            any_hit_shader: any_hit,
            intersection_shader: Self::UNUSED,
        }
    }

    /// Creates a procedural hit group
    pub const fn procedural_hit(closest_hit: u32, any_hit: u32, intersection: u32) -> Self {
        Self {
            group_type: ShaderGroupType::ProceduralHitGroup,
            general_shader: Self::UNUSED,
            closest_hit_shader: closest_hit,
            any_hit_shader: any_hit,
            intersection_shader: intersection,
        }
    }
}

/// Ray tracing pipeline state description
#[derive(Clone, Debug)]
pub struct RtPipelineDesc {
    /// Shader stages
    pub stages: Vec<u32>, // Shader module indices
    /// Shader groups
    pub groups: Vec<RtShaderGroup>,
    /// Maximum ray recursion depth
    pub max_recursion_depth: u32,
    /// Maximum payload size in bytes
    pub max_payload_size: u32,
    /// Maximum attribute size in bytes
    pub max_attribute_size: u32,
}

impl RtPipelineDesc {
    /// Creates a new ray tracing pipeline description
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            groups: Vec::new(),
            max_recursion_depth: 1,
            max_payload_size: 16,
            max_attribute_size: 8,
        }
    }

    /// Adds a shader stage
    pub fn add_stage(mut self, shader_module: u32) -> Self {
        self.stages.push(shader_module);
        self
    }

    /// Adds a shader group
    pub fn add_group(mut self, group: RtShaderGroup) -> Self {
        self.groups.push(group);
        self
    }

    /// Sets max recursion depth
    pub fn with_max_recursion_depth(mut self, depth: u32) -> Self {
        self.max_recursion_depth = depth;
        self
    }

    /// Sets max payload size
    pub fn with_max_payload_size(mut self, size: u32) -> Self {
        self.max_payload_size = size;
        self
    }

    /// Sets max attribute size
    pub fn with_max_attribute_size(mut self, size: u32) -> Self {
        self.max_attribute_size = size;
        self
    }
}

impl Default for RtPipelineDesc {
    fn default() -> Self {
        Self::new()
    }
}

/// Trace rays parameters
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TraceRaysDesc {
    /// Width of the ray launch grid
    pub width: u32,
    /// Height of the ray launch grid
    pub height: u32,
    /// Depth of the ray launch grid
    pub depth: u32,
}

impl TraceRaysDesc {
    /// Creates a 2D trace (common case)
    pub const fn dispatch_2d(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            depth: 1,
        }
    }

    /// Creates a 3D trace
    pub const fn dispatch_3d(width: u32, height: u32, depth: u32) -> Self {
        Self {
            width,
            height,
            depth,
        }
    }

    /// Creates a 1D trace
    pub const fn dispatch_1d(width: u32) -> Self {
        Self {
            width,
            height: 1,
            depth: 1,
        }
    }

    /// Total number of rays
    pub const fn ray_count(&self) -> u64 {
        self.width as u64 * self.height as u64 * self.depth as u64
    }
}

/// Compaction info query result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AccelerationStructureCompactedSize {
    /// Compacted size in bytes
    pub compacted_size: u64,
}

/// Serialization info query result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AccelerationStructureSerializationInfo {
    /// Serialized size in bytes
    pub serialized_size: u64,
    /// Number of bottom-level AS handles
    pub bottom_level_count: u64,
}
