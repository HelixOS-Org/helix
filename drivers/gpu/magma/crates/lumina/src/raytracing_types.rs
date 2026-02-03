//! Ray Tracing Types for Lumina
//!
//! This module provides ray tracing infrastructure including
//! acceleration structures, pipelines, and shader binding tables.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Ray Tracing Handles
// ============================================================================

/// Acceleration structure handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AccelerationStructureHandle(pub u64);

impl AccelerationStructureHandle {
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

impl Default for AccelerationStructureHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Ray tracing pipeline handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RtPipelineHandle(pub u64);

impl RtPipelineHandle {
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

impl Default for RtPipelineHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Shader binding table handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShaderBindingTableHandle(pub u64);

impl ShaderBindingTableHandle {
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

impl Default for ShaderBindingTableHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Acceleration Structures
// ============================================================================

/// Acceleration structure type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AccelerationStructureType {
    /// Bottom-level (BLAS) - geometry
    #[default]
    BottomLevel = 0,
    /// Top-level (TLAS) - instances
    TopLevel    = 1,
}

/// BLAS create info
#[derive(Clone, Debug)]
pub struct BlasCreateInfo {
    /// Name
    pub name: String,
    /// Geometries
    pub geometries: Vec<BlasGeometry>,
    /// Build flags
    pub flags: AccelerationStructureFlags,
    /// Allow compaction
    pub allow_compaction: bool,
    /// Update mode
    pub update_mode: BlasUpdateMode,
}

impl BlasCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            geometries: Vec::new(),
            flags: AccelerationStructureFlags::PREFER_FAST_TRACE,
            allow_compaction: true,
            update_mode: BlasUpdateMode::FullRebuild,
        }
    }

    /// Static geometry (optimized for tracing)
    pub fn static_geometry() -> Self {
        Self {
            flags: AccelerationStructureFlags::PREFER_FAST_TRACE,
            allow_compaction: true,
            update_mode: BlasUpdateMode::FullRebuild,
            ..Self::new()
        }
    }

    /// Dynamic geometry (optimized for updates)
    pub fn dynamic() -> Self {
        Self {
            flags: AccelerationStructureFlags::PREFER_FAST_BUILD
                | AccelerationStructureFlags::ALLOW_UPDATE,
            allow_compaction: false,
            update_mode: BlasUpdateMode::Refit,
            ..Self::new()
        }
    }

    /// With geometry
    pub fn with_geometry(mut self, geometry: BlasGeometry) -> Self {
        self.geometries.push(geometry);
        self
    }

    /// With name
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = String::from(name);
        self
    }
}

impl Default for BlasCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// BLAS geometry
#[derive(Clone, Debug)]
pub struct BlasGeometry {
    /// Geometry type
    pub geometry_type: GeometryType,
    /// Triangle data (if triangles)
    pub triangles: Option<TriangleGeometry>,
    /// AABB data (if procedural)
    pub aabbs: Option<AabbGeometry>,
    /// Geometry flags
    pub flags: GeometryFlags,
}

impl BlasGeometry {
    /// Creates triangle geometry
    pub fn triangles(data: TriangleGeometry) -> Self {
        Self {
            geometry_type: GeometryType::Triangles,
            triangles: Some(data),
            aabbs: None,
            flags: GeometryFlags::OPAQUE,
        }
    }

    /// Creates AABB geometry
    pub fn aabbs(data: AabbGeometry) -> Self {
        Self {
            geometry_type: GeometryType::Aabbs,
            triangles: None,
            aabbs: Some(data),
            flags: GeometryFlags::NONE,
        }
    }

    /// Make opaque
    pub fn opaque(mut self) -> Self {
        self.flags = self.flags | GeometryFlags::OPAQUE;
        self
    }

    /// No duplicate any hit
    pub fn no_duplicate_anyhit(mut self) -> Self {
        self.flags = self.flags | GeometryFlags::NO_DUPLICATE_ANYHIT;
        self
    }
}

impl Default for BlasGeometry {
    fn default() -> Self {
        Self::triangles(TriangleGeometry::default())
    }
}

/// Geometry type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum GeometryType {
    /// Triangle geometry
    #[default]
    Triangles = 0,
    /// AABB (procedural) geometry
    Aabbs     = 1,
}

/// Triangle geometry
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TriangleGeometry {
    /// Vertex buffer address
    pub vertex_buffer: u64,
    /// Vertex stride
    pub vertex_stride: u32,
    /// Vertex count
    pub vertex_count: u32,
    /// Vertex format
    pub vertex_format: VertexFormat,
    /// Index buffer address
    pub index_buffer: u64,
    /// Index type
    pub index_type: IndexType,
    /// Index count
    pub index_count: u32,
    /// Transform buffer (optional)
    pub transform_buffer: u64,
}

impl TriangleGeometry {
    /// Creates geometry
    pub const fn new() -> Self {
        Self {
            vertex_buffer: 0,
            vertex_stride: 12,
            vertex_count: 0,
            vertex_format: VertexFormat::Float3,
            index_buffer: 0,
            index_type: IndexType::U32,
            index_count: 0,
            transform_buffer: 0,
        }
    }

    /// With vertices
    pub const fn with_vertices(mut self, buffer: u64, count: u32, stride: u32) -> Self {
        self.vertex_buffer = buffer;
        self.vertex_count = count;
        self.vertex_stride = stride;
        self
    }

    /// With indices
    pub const fn with_indices(mut self, buffer: u64, count: u32, index_type: IndexType) -> Self {
        self.index_buffer = buffer;
        self.index_count = count;
        self.index_type = index_type;
        self
    }
}

/// AABB geometry
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AabbGeometry {
    /// AABB buffer address
    pub aabb_buffer: u64,
    /// AABB count
    pub aabb_count: u32,
    /// Stride
    pub stride: u32,
}

impl AabbGeometry {
    /// Creates geometry
    pub const fn new(buffer: u64, count: u32) -> Self {
        Self {
            aabb_buffer: buffer,
            aabb_count: count,
            stride: 24, // 6 floats
        }
    }
}

/// Vertex format for RT
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum VertexFormat {
    /// Float3
    #[default]
    Float3    = 0,
    /// Float2
    Float2    = 1,
    /// Half2
    Half2     = 2,
    /// Half4
    Half4     = 3,
    /// Snorm16x2
    Snorm16x2 = 4,
}

impl VertexFormat {
    /// Size in bytes
    pub const fn size(&self) -> u32 {
        match self {
            Self::Float3 => 12,
            Self::Float2 => 8,
            Self::Half2 => 4,
            Self::Half4 | Self::Snorm16x2 => 8,
        }
    }
}

/// Index type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum IndexType {
    /// 16-bit indices
    U16  = 0,
    /// 32-bit indices
    #[default]
    U32  = 1,
    /// No indices (non-indexed)
    None = 2,
}

/// Geometry flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GeometryFlags(pub u32);

impl GeometryFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Opaque geometry
    pub const OPAQUE: Self = Self(1 << 0);
    /// No duplicate any-hit invocation
    pub const NO_DUPLICATE_ANYHIT: Self = Self(1 << 1);

    /// Has flag
    pub const fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl Default for GeometryFlags {
    fn default() -> Self {
        Self::NONE
    }
}

impl core::ops::BitOr for GeometryFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Acceleration structure flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AccelerationStructureFlags(pub u32);

impl AccelerationStructureFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Prefer fast trace performance
    pub const PREFER_FAST_TRACE: Self = Self(1 << 0);
    /// Prefer fast build
    pub const PREFER_FAST_BUILD: Self = Self(1 << 1);
    /// Allow compaction
    pub const ALLOW_COMPACTION: Self = Self(1 << 2);
    /// Allow update
    pub const ALLOW_UPDATE: Self = Self(1 << 3);
    /// Low memory
    pub const LOW_MEMORY: Self = Self(1 << 4);

    /// Has flag
    pub const fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl Default for AccelerationStructureFlags {
    fn default() -> Self {
        Self::PREFER_FAST_TRACE
    }
}

impl core::ops::BitOr for AccelerationStructureFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// BLAS update mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BlasUpdateMode {
    /// Full rebuild
    #[default]
    FullRebuild = 0,
    /// Refit (update in place)
    Refit       = 1,
}

// ============================================================================
// TLAS (Top-Level Acceleration Structure)
// ============================================================================

/// TLAS create info
#[derive(Clone, Debug)]
pub struct TlasCreateInfo {
    /// Name
    pub name: String,
    /// Max instances
    pub max_instances: u32,
    /// Build flags
    pub flags: AccelerationStructureFlags,
}

impl TlasCreateInfo {
    /// Creates info
    pub fn new(max_instances: u32) -> Self {
        Self {
            name: String::new(),
            max_instances,
            flags: AccelerationStructureFlags::PREFER_FAST_TRACE,
        }
    }

    /// Dynamic scene
    pub fn dynamic(max_instances: u32) -> Self {
        Self {
            flags: AccelerationStructureFlags::PREFER_FAST_BUILD
                | AccelerationStructureFlags::ALLOW_UPDATE,
            ..Self::new(max_instances)
        }
    }
}

impl Default for TlasCreateInfo {
    fn default() -> Self {
        Self::new(1024)
    }
}

/// TLAS instance
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TlasInstance {
    /// Transform (3x4 row-major)
    pub transform: [[f32; 4]; 3],
    /// Instance custom index (24 bits)
    pub instance_custom_index: u32,
    /// Mask (8 bits)
    pub mask: u8,
    /// Shader binding table offset (24 bits)
    pub sbt_offset: u32,
    /// Flags
    pub flags: InstanceFlags,
    /// BLAS address
    pub blas_address: u64,
}

impl TlasInstance {
    /// Creates instance
    pub fn new(blas: AccelerationStructureHandle) -> Self {
        Self {
            transform: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [
                0.0, 0.0, 1.0, 0.0,
            ]],
            instance_custom_index: 0,
            mask: 0xFF,
            sbt_offset: 0,
            flags: InstanceFlags::NONE,
            blas_address: blas.0,
        }
    }

    /// With transform
    pub fn with_transform(mut self, transform: [[f32; 4]; 3]) -> Self {
        self.transform = transform;
        self
    }

    /// With position
    pub fn at(mut self, x: f32, y: f32, z: f32) -> Self {
        self.transform[0][3] = x;
        self.transform[1][3] = y;
        self.transform[2][3] = z;
        self
    }

    /// With custom index
    pub fn with_custom_index(mut self, index: u32) -> Self {
        self.instance_custom_index = index & 0xFFFFFF;
        self
    }

    /// With mask
    pub fn with_mask(mut self, mask: u8) -> Self {
        self.mask = mask;
        self
    }

    /// With SBT offset
    pub fn with_sbt_offset(mut self, offset: u32) -> Self {
        self.sbt_offset = offset & 0xFFFFFF;
        self
    }

    /// Force opaque
    pub fn force_opaque(mut self) -> Self {
        self.flags = self.flags | InstanceFlags::FORCE_OPAQUE;
        self
    }

    /// Disable face culling
    pub fn disable_face_culling(mut self) -> Self {
        self.flags = self.flags | InstanceFlags::DISABLE_FACE_CULLING;
        self
    }
}

impl Default for TlasInstance {
    fn default() -> Self {
        Self::new(AccelerationStructureHandle::NULL)
    }
}

/// Instance flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct InstanceFlags(pub u8);

impl InstanceFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Force opaque
    pub const FORCE_OPAQUE: Self = Self(1 << 0);
    /// Force non-opaque
    pub const FORCE_NON_OPAQUE: Self = Self(1 << 1);
    /// Disable triangle face culling
    pub const DISABLE_FACE_CULLING: Self = Self(1 << 2);
    /// Flip facing
    pub const FLIP_FACING: Self = Self(1 << 3);

    /// Has flag
    pub const fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl Default for InstanceFlags {
    fn default() -> Self {
        Self::NONE
    }
}

impl core::ops::BitOr for InstanceFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

// ============================================================================
// Ray Tracing Pipeline
// ============================================================================

/// RT pipeline create info
#[derive(Clone, Debug)]
pub struct RtPipelineCreateInfo {
    /// Name
    pub name: String,
    /// Shaders
    pub shaders: Vec<RtShader>,
    /// Shader groups
    pub groups: Vec<ShaderGroup>,
    /// Max recursion depth
    pub max_recursion_depth: u32,
    /// Max payload size
    pub max_payload_size: u32,
    /// Max attribute size
    pub max_attribute_size: u32,
}

impl RtPipelineCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            shaders: Vec::new(),
            groups: Vec::new(),
            max_recursion_depth: 2,
            max_payload_size: 32,
            max_attribute_size: 8,
        }
    }

    /// Simple path tracer
    pub fn simple_pathtracer() -> Self {
        Self {
            max_recursion_depth: 8,
            max_payload_size: 48,
            ..Self::new()
        }
    }

    /// With shader
    pub fn with_shader(mut self, shader: RtShader) -> Self {
        self.shaders.push(shader);
        self
    }

    /// With group
    pub fn with_group(mut self, group: ShaderGroup) -> Self {
        self.groups.push(group);
        self
    }

    /// With recursion depth
    pub fn with_recursion(mut self, depth: u32) -> Self {
        self.max_recursion_depth = depth;
        self
    }
}

impl Default for RtPipelineCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// RT shader
#[derive(Clone, Debug)]
pub struct RtShader {
    /// Shader stage
    pub stage: RtShaderStage,
    /// Entry point
    pub entry_point: String,
    /// Shader code handle
    pub code: u64,
}

impl RtShader {
    /// Creates shader
    pub fn new(stage: RtShaderStage, code: u64) -> Self {
        Self {
            stage,
            entry_point: String::from("main"),
            code,
        }
    }

    /// Ray generation shader
    pub fn raygen(code: u64) -> Self {
        Self::new(RtShaderStage::RayGeneration, code)
    }

    /// Miss shader
    pub fn miss(code: u64) -> Self {
        Self::new(RtShaderStage::Miss, code)
    }

    /// Closest hit shader
    pub fn closest_hit(code: u64) -> Self {
        Self::new(RtShaderStage::ClosestHit, code)
    }

    /// Any hit shader
    pub fn any_hit(code: u64) -> Self {
        Self::new(RtShaderStage::AnyHit, code)
    }

    /// Intersection shader
    pub fn intersection(code: u64) -> Self {
        Self::new(RtShaderStage::Intersection, code)
    }
}

impl Default for RtShader {
    fn default() -> Self {
        Self::raygen(0)
    }
}

/// RT shader stage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum RtShaderStage {
    /// Ray generation
    #[default]
    RayGeneration = 0,
    /// Miss
    Miss          = 1,
    /// Closest hit
    ClosestHit    = 2,
    /// Any hit
    AnyHit        = 3,
    /// Intersection
    Intersection  = 4,
    /// Callable
    Callable      = 5,
}

/// Shader group
#[derive(Clone, Copy, Debug)]
pub struct ShaderGroup {
    /// Group type
    pub group_type: ShaderGroupType,
    /// General shader index (-1 for unused)
    pub general_shader: i32,
    /// Closest hit shader index (-1 for unused)
    pub closest_hit_shader: i32,
    /// Any hit shader index (-1 for unused)
    pub any_hit_shader: i32,
    /// Intersection shader index (-1 for unused)
    pub intersection_shader: i32,
}

impl ShaderGroup {
    /// No shader
    const UNUSED: i32 = -1;

    /// General group (raygen, miss, callable)
    pub fn general(shader_index: u32) -> Self {
        Self {
            group_type: ShaderGroupType::General,
            general_shader: shader_index as i32,
            closest_hit_shader: Self::UNUSED,
            any_hit_shader: Self::UNUSED,
            intersection_shader: Self::UNUSED,
        }
    }

    /// Triangle hit group
    pub fn triangles_hit(closest_hit: Option<u32>, any_hit: Option<u32>) -> Self {
        Self {
            group_type: ShaderGroupType::TrianglesHit,
            general_shader: Self::UNUSED,
            closest_hit_shader: closest_hit.map(|i| i as i32).unwrap_or(Self::UNUSED),
            any_hit_shader: any_hit.map(|i| i as i32).unwrap_or(Self::UNUSED),
            intersection_shader: Self::UNUSED,
        }
    }

    /// Procedural hit group
    pub fn procedural_hit(
        intersection: u32,
        closest_hit: Option<u32>,
        any_hit: Option<u32>,
    ) -> Self {
        Self {
            group_type: ShaderGroupType::ProceduralHit,
            general_shader: Self::UNUSED,
            closest_hit_shader: closest_hit.map(|i| i as i32).unwrap_or(Self::UNUSED),
            any_hit_shader: any_hit.map(|i| i as i32).unwrap_or(Self::UNUSED),
            intersection_shader: intersection as i32,
        }
    }
}

impl Default for ShaderGroup {
    fn default() -> Self {
        Self::general(0)
    }
}

/// Shader group type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShaderGroupType {
    /// General (raygen, miss, callable)
    #[default]
    General       = 0,
    /// Triangles hit group
    TrianglesHit  = 1,
    /// Procedural hit group
    ProceduralHit = 2,
}

// ============================================================================
// Shader Binding Table
// ============================================================================

/// SBT create info
#[derive(Clone, Debug)]
pub struct SbtCreateInfo {
    /// Pipeline
    pub pipeline: RtPipelineHandle,
    /// Raygen region
    pub raygen_region: SbtRegion,
    /// Miss region
    pub miss_region: SbtRegion,
    /// Hit region
    pub hit_region: SbtRegion,
    /// Callable region
    pub callable_region: SbtRegion,
}

impl SbtCreateInfo {
    /// Creates info
    pub fn new(pipeline: RtPipelineHandle) -> Self {
        Self {
            pipeline,
            raygen_region: SbtRegion::default(),
            miss_region: SbtRegion::default(),
            hit_region: SbtRegion::default(),
            callable_region: SbtRegion::default(),
        }
    }
}

impl Default for SbtCreateInfo {
    fn default() -> Self {
        Self::new(RtPipelineHandle::NULL)
    }
}

/// SBT region
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SbtRegion {
    /// Device address
    pub device_address: u64,
    /// Stride
    pub stride: u64,
    /// Size
    pub size: u64,
}

impl SbtRegion {
    /// Creates region
    pub const fn new(address: u64, stride: u64, size: u64) -> Self {
        Self {
            device_address: address,
            stride,
            size,
        }
    }

    /// Empty region
    pub const fn empty() -> Self {
        Self::new(0, 0, 0)
    }
}

// ============================================================================
// Ray Tracing Commands
// ============================================================================

/// Ray trace dispatch info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TraceRaysInfo {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth
    pub depth: u32,
}

impl TraceRaysInfo {
    /// Creates info
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            depth: 1,
        }
    }

    /// Full screen (1080p)
    pub const fn fullscreen_1080p() -> Self {
        Self::new(1920, 1080)
    }

    /// Full screen (4K)
    pub const fn fullscreen_4k() -> Self {
        Self::new(3840, 2160)
    }

    /// 3D dispatch
    pub const fn dispatch_3d(width: u32, height: u32, depth: u32) -> Self {
        Self {
            width,
            height,
            depth,
        }
    }
}

impl Default for TraceRaysInfo {
    fn default() -> Self {
        Self::new(1, 1)
    }
}

/// Indirect trace rays command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TraceRaysIndirectCommand {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth
    pub depth: u32,
}

// ============================================================================
// Ray Query
// ============================================================================

/// Ray query flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RayFlags(pub u32);

impl RayFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Force opaque
    pub const FORCE_OPAQUE: Self = Self(1 << 0);
    /// Force non-opaque
    pub const FORCE_NON_OPAQUE: Self = Self(1 << 1);
    /// Accept first hit and end search
    pub const TERMINATE_ON_FIRST_HIT: Self = Self(1 << 2);
    /// Skip closest hit shader
    pub const SKIP_CLOSEST_HIT_SHADER: Self = Self(1 << 3);
    /// Cull back facing triangles
    pub const CULL_BACK_FACING: Self = Self(1 << 4);
    /// Cull front facing triangles
    pub const CULL_FRONT_FACING: Self = Self(1 << 5);
    /// Cull opaque geometry
    pub const CULL_OPAQUE: Self = Self(1 << 6);
    /// Cull non-opaque geometry
    pub const CULL_NON_OPAQUE: Self = Self(1 << 7);
    /// Skip triangles
    pub const SKIP_TRIANGLES: Self = Self(1 << 8);
    /// Skip AABBs
    pub const SKIP_AABBS: Self = Self(1 << 9);

    /// Shadow ray (terminate on first hit, skip shaders)
    pub const SHADOW: Self = Self(Self::TERMINATE_ON_FIRST_HIT.0 | Self::SKIP_CLOSEST_HIT_SHADER.0);

    /// Has flag
    pub const fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl Default for RayFlags {
    fn default() -> Self {
        Self::NONE
    }
}

impl core::ops::BitOr for RayFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Ray description
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct RayDesc {
    /// Origin
    pub origin: [f32; 3],
    /// Minimum distance
    pub t_min: f32,
    /// Direction
    pub direction: [f32; 3],
    /// Maximum distance
    pub t_max: f32,
}

impl RayDesc {
    /// Creates ray
    pub const fn new(origin: [f32; 3], direction: [f32; 3]) -> Self {
        Self {
            origin,
            t_min: 0.001,
            direction,
            t_max: 10000.0,
        }
    }

    /// With range
    pub const fn with_range(mut self, t_min: f32, t_max: f32) -> Self {
        self.t_min = t_min;
        self.t_max = t_max;
        self
    }
}

/// Ray hit
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct RayHit {
    /// Hit distance (t)
    pub t: f32,
    /// Barycentric U
    pub barycentrics_u: f32,
    /// Barycentric V
    pub barycentrics_v: f32,
    /// Primitive index
    pub primitive_index: u32,
    /// Instance index
    pub instance_index: u32,
    /// Instance custom index
    pub instance_custom_index: u32,
    /// Geometry index
    pub geometry_index: u32,
    /// Front face
    pub front_face: u32,
}

impl RayHit {
    /// Is valid hit
    pub const fn is_hit(&self) -> bool {
        self.t < 10000.0
    }

    /// Get barycentrics
    pub fn barycentrics(&self) -> [f32; 3] {
        let w = 1.0 - self.barycentrics_u - self.barycentrics_v;
        [w, self.barycentrics_u, self.barycentrics_v]
    }

    /// Is front face
    pub const fn is_front_face(&self) -> bool {
        self.front_face != 0
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// RT statistics
#[derive(Clone, Debug, Default)]
pub struct RtStats {
    /// BLAS count
    pub blas_count: u32,
    /// TLAS count
    pub tlas_count: u32,
    /// Total instances
    pub instance_count: u32,
    /// Total triangles
    pub triangle_count: u64,
    /// AS memory usage (bytes)
    pub memory_usage: u64,
    /// Build time (microseconds)
    pub build_time_us: u64,
    /// Rays traced per frame
    pub rays_per_frame: u64,
}
