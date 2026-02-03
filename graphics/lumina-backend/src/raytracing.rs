//! Ray Tracing Support
//!
//! Hardware-accelerated ray tracing for real-time rendering.
//! Supports RTX (NVIDIA), DXR (Microsoft), and Metal Ray Tracing.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                     Ray Tracing Pipeline                         │
//! ├──────────────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐  │
//! │  │ Ray Gen     │───▶│ Traversal   │───▶│ Hit/Miss Shaders   │  │
//! │  │ Shader      │    │ (BVH)       │    │                     │  │
//! │  └─────────────┘    └─────────────┘    └─────────────────────┘  │
//! ├──────────────────────────────────────────────────────────────────┤
//! │  ┌───────────────────────────────────────────────────────────┐  │
//! │  │              Acceleration Structures                       │  │
//! │  │  ┌─────────────────┐     ┌─────────────────────────────┐  │  │
//! │  │  │ BLAS (Bottom)   │────▶│ TLAS (Top-Level)           │  │  │
//! │  │  │ Per-Geometry    │     │ Scene Instancing           │  │  │
//! │  │  └─────────────────┘     └─────────────────────────────┘  │  │
//! │  └───────────────────────────────────────────────────────────┘  │
//! └──────────────────────────────────────────────────────────────────┘
//! ```

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::buffer::BufferHandle;
use crate::pipeline::PipelineHandle;

// ============================================================================
// Acceleration Structure Types
// ============================================================================

/// Handle to an acceleration structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccelerationStructureHandle(pub u64);

impl AccelerationStructureHandle {
    /// Invalid handle.
    pub const INVALID: Self = Self(u64::MAX);

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        self.0 != u64::MAX
    }
}

/// Type of acceleration structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccelerationStructureType {
    /// Bottom-level (geometry).
    BottomLevel,
    /// Top-level (instances).
    TopLevel,
}

/// Acceleration structure build flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccelerationStructureBuildFlags(u32);

impl AccelerationStructureBuildFlags {
    /// Allow updates after build.
    pub const ALLOW_UPDATE: Self = Self(1 << 0);
    /// Allow compaction after build.
    pub const ALLOW_COMPACTION: Self = Self(1 << 1);
    /// Prefer fast trace over fast build.
    pub const PREFER_FAST_TRACE: Self = Self(1 << 2);
    /// Prefer fast build over fast trace.
    pub const PREFER_FAST_BUILD: Self = Self(1 << 3);
    /// Minimize memory usage.
    pub const LOW_MEMORY: Self = Self(1 << 4);

    /// Combine flags.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Check if flag is set.
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl Default for AccelerationStructureBuildFlags {
    fn default() -> Self {
        Self::PREFER_FAST_TRACE
    }
}

/// Geometry flags for BLAS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GeometryFlags(u32);

impl GeometryFlags {
    /// Geometry is opaque.
    pub const OPAQUE: Self = Self(1 << 0);
    /// No duplicate any-hit invocations.
    pub const NO_DUPLICATE_ANY_HIT: Self = Self(1 << 1);

    /// Combine flags.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl Default for GeometryFlags {
    fn default() -> Self {
        Self::OPAQUE
    }
}

/// Instance flags for TLAS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InstanceFlags(u32);

impl InstanceFlags {
    /// Disable triangle culling.
    pub const TRIANGLE_CULL_DISABLE: Self = Self(1 << 0);
    /// Flip triangle facing.
    pub const TRIANGLE_FRONT_CCW: Self = Self(1 << 1);
    /// Force opaque.
    pub const FORCE_OPAQUE: Self = Self(1 << 2);
    /// Force non-opaque.
    pub const FORCE_NON_OPAQUE: Self = Self(1 << 3);

    /// Combine flags.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl Default for InstanceFlags {
    fn default() -> Self {
        Self(0)
    }
}

// ============================================================================
// Geometry Descriptions
// ============================================================================

/// Vertex format for ray tracing geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RtVertexFormat {
    /// 32-bit float x3.
    Float3,
    /// 32-bit float x2.
    Float2,
    /// 16-bit float x3.
    Half3,
    /// 16-bit float x4.
    Half4,
    /// 16-bit signed normalized x3.
    Snorm16x3,
    /// 16-bit signed normalized x4.
    Snorm16x4,
}

impl RtVertexFormat {
    /// Get byte size.
    pub fn byte_size(&self) -> u32 {
        match self {
            RtVertexFormat::Float3 => 12,
            RtVertexFormat::Float2 => 8,
            RtVertexFormat::Half3 => 6,
            RtVertexFormat::Half4 => 8,
            RtVertexFormat::Snorm16x3 => 6,
            RtVertexFormat::Snorm16x4 => 8,
        }
    }
}

/// Index format for ray tracing geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RtIndexFormat {
    /// 16-bit unsigned.
    Uint16,
    /// 32-bit unsigned.
    Uint32,
    /// No indices (non-indexed geometry).
    None,
}

impl RtIndexFormat {
    /// Get byte size.
    pub fn byte_size(&self) -> u32 {
        match self {
            RtIndexFormat::Uint16 => 2,
            RtIndexFormat::Uint32 => 4,
            RtIndexFormat::None => 0,
        }
    }
}

/// Triangle geometry for BLAS.
#[derive(Debug, Clone)]
pub struct TriangleGeometry {
    /// Vertex buffer.
    pub vertex_buffer: BufferHandle,
    /// Vertex buffer offset.
    pub vertex_offset: u64,
    /// Vertex format.
    pub vertex_format: RtVertexFormat,
    /// Vertex stride.
    pub vertex_stride: u32,
    /// Vertex count.
    pub vertex_count: u32,
    /// Index buffer.
    pub index_buffer: Option<BufferHandle>,
    /// Index buffer offset.
    pub index_offset: u64,
    /// Index format.
    pub index_format: RtIndexFormat,
    /// Index count.
    pub index_count: u32,
    /// Transform buffer (optional 3x4 matrix).
    pub transform_buffer: Option<BufferHandle>,
    /// Transform buffer offset.
    pub transform_offset: u64,
    /// Geometry flags.
    pub flags: GeometryFlags,
}

impl Default for TriangleGeometry {
    fn default() -> Self {
        Self {
            vertex_buffer: BufferHandle::INVALID,
            vertex_offset: 0,
            vertex_format: RtVertexFormat::Float3,
            vertex_stride: 12,
            vertex_count: 0,
            index_buffer: None,
            index_offset: 0,
            index_format: RtIndexFormat::None,
            index_count: 0,
            transform_buffer: None,
            transform_offset: 0,
            flags: GeometryFlags::default(),
        }
    }
}

/// AABB geometry for procedural primitives.
#[derive(Debug, Clone)]
pub struct AabbGeometry {
    /// AABB buffer (stride = 24 bytes, min xyz + max xyz).
    pub aabb_buffer: BufferHandle,
    /// Buffer offset.
    pub offset: u64,
    /// Stride between AABBs.
    pub stride: u32,
    /// Number of AABBs.
    pub count: u32,
    /// Geometry flags.
    pub flags: GeometryFlags,
}

impl Default for AabbGeometry {
    fn default() -> Self {
        Self {
            aabb_buffer: BufferHandle::INVALID,
            offset: 0,
            stride: 24,
            count: 0,
            flags: GeometryFlags::default(),
        }
    }
}

/// Geometry type for BLAS.
#[derive(Debug, Clone)]
pub enum GeometryDesc {
    /// Triangles.
    Triangles(TriangleGeometry),
    /// Procedural AABBs.
    Aabbs(AabbGeometry),
}

// ============================================================================
// Acceleration Structure Instance
// ============================================================================

/// 3x4 transform matrix (row major).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Transform3x4 {
    /// Matrix data (row major).
    pub data: [[f32; 4]; 3],
}

impl Default for Transform3x4 {
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform3x4 {
    /// Identity matrix.
    pub const fn identity() -> Self {
        Self {
            data: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [
                0.0, 0.0, 1.0, 0.0,
            ]],
        }
    }

    /// Create from translation.
    pub fn translation(x: f32, y: f32, z: f32) -> Self {
        Self {
            data: [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]],
        }
    }

    /// Create from scale.
    pub fn scale(x: f32, y: f32, z: f32) -> Self {
        Self {
            data: [[x, 0.0, 0.0, 0.0], [0.0, y, 0.0, 0.0], [0.0, 0.0, z, 0.0]],
        }
    }

    /// Create from uniform scale.
    pub fn uniform_scale(s: f32) -> Self {
        Self::scale(s, s, s)
    }

    /// Create rotation around X axis.
    pub fn rotate_x(radians: f32) -> Self {
        let c = radians.cos();
        let s = radians.sin();
        Self {
            data: [[1.0, 0.0, 0.0, 0.0], [0.0, c, -s, 0.0], [0.0, s, c, 0.0]],
        }
    }

    /// Create rotation around Y axis.
    pub fn rotate_y(radians: f32) -> Self {
        let c = radians.cos();
        let s = radians.sin();
        Self {
            data: [[c, 0.0, s, 0.0], [0.0, 1.0, 0.0, 0.0], [-s, 0.0, c, 0.0]],
        }
    }

    /// Create rotation around Z axis.
    pub fn rotate_z(radians: f32) -> Self {
        let c = radians.cos();
        let s = radians.sin();
        Self {
            data: [[c, -s, 0.0, 0.0], [s, c, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]],
        }
    }
}

/// Instance in TLAS.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct AccelerationStructureInstance {
    /// 3x4 transform matrix.
    pub transform: Transform3x4,
    /// Custom instance ID (24 bits).
    pub instance_custom_index: u32,
    /// Visibility mask (8 bits).
    pub mask: u8,
    /// Shader binding table offset.
    pub instance_shader_binding_offset: u32,
    /// Instance flags.
    pub flags: InstanceFlags,
    /// Handle to BLAS.
    pub blas: AccelerationStructureHandle,
}

impl Default for AccelerationStructureInstance {
    fn default() -> Self {
        Self {
            transform: Transform3x4::identity(),
            instance_custom_index: 0,
            mask: 0xFF,
            instance_shader_binding_offset: 0,
            flags: InstanceFlags::default(),
            blas: AccelerationStructureHandle::INVALID,
        }
    }
}

// ============================================================================
// Acceleration Structure Descriptions
// ============================================================================

/// Description for BLAS build.
#[derive(Debug, Clone)]
pub struct BlasDesc {
    /// Debug name.
    pub name: Option<String>,
    /// Geometries.
    pub geometries: Vec<GeometryDesc>,
    /// Build flags.
    pub flags: AccelerationStructureBuildFlags,
}

impl Default for BlasDesc {
    fn default() -> Self {
        Self {
            name: None,
            geometries: Vec::new(),
            flags: AccelerationStructureBuildFlags::default(),
        }
    }
}

impl BlasDesc {
    /// Create new BLAS descriptor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add triangle geometry.
    pub fn with_triangles(mut self, geometry: TriangleGeometry) -> Self {
        self.geometries.push(GeometryDesc::Triangles(geometry));
        self
    }

    /// Add AABB geometry.
    pub fn with_aabbs(mut self, geometry: AabbGeometry) -> Self {
        self.geometries.push(GeometryDesc::Aabbs(geometry));
        self
    }

    /// Set build flags.
    pub fn with_flags(mut self, flags: AccelerationStructureBuildFlags) -> Self {
        self.flags = flags;
        self
    }
}

/// Description for TLAS build.
#[derive(Debug, Clone)]
pub struct TlasDesc {
    /// Debug name.
    pub name: Option<String>,
    /// Maximum instance count.
    pub max_instance_count: u32,
    /// Build flags.
    pub flags: AccelerationStructureBuildFlags,
}

impl Default for TlasDesc {
    fn default() -> Self {
        Self {
            name: None,
            max_instance_count: 1024,
            flags: AccelerationStructureBuildFlags::default(),
        }
    }
}

impl TlasDesc {
    /// Create new TLAS descriptor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set max instance count.
    pub fn with_max_instances(mut self, count: u32) -> Self {
        self.max_instance_count = count;
        self
    }

    /// Set build flags.
    pub fn with_flags(mut self, flags: AccelerationStructureBuildFlags) -> Self {
        self.flags = flags;
        self
    }
}

// ============================================================================
// Build Information
// ============================================================================

/// Sizes for acceleration structure build.
#[derive(Debug, Clone, Copy, Default)]
pub struct AccelerationStructureSizes {
    /// Size of acceleration structure.
    pub acceleration_structure_size: u64,
    /// Size of build scratch buffer.
    pub build_scratch_size: u64,
    /// Size of update scratch buffer.
    pub update_scratch_size: u64,
}

/// Build mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccelerationStructureBuildMode {
    /// Full build.
    Build,
    /// Update existing structure.
    Update,
}

/// Build input for BLAS.
#[derive(Debug, Clone)]
pub struct BlasBuildInput {
    /// Source acceleration structure (for updates).
    pub src: Option<AccelerationStructureHandle>,
    /// Destination acceleration structure.
    pub dst: AccelerationStructureHandle,
    /// Scratch buffer.
    pub scratch_buffer: BufferHandle,
    /// Scratch buffer offset.
    pub scratch_offset: u64,
    /// Build mode.
    pub mode: AccelerationStructureBuildMode,
    /// Geometries.
    pub geometries: Vec<GeometryDesc>,
}

/// Build input for TLAS.
#[derive(Debug, Clone)]
pub struct TlasBuildInput {
    /// Source acceleration structure (for updates).
    pub src: Option<AccelerationStructureHandle>,
    /// Destination acceleration structure.
    pub dst: AccelerationStructureHandle,
    /// Scratch buffer.
    pub scratch_buffer: BufferHandle,
    /// Scratch buffer offset.
    pub scratch_offset: u64,
    /// Build mode.
    pub mode: AccelerationStructureBuildMode,
    /// Instance buffer.
    pub instance_buffer: BufferHandle,
    /// Instance buffer offset.
    pub instance_offset: u64,
    /// Instance count.
    pub instance_count: u32,
}

// ============================================================================
// Ray Tracing Pipeline
// ============================================================================

/// Ray tracing shader stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RayTracingShaderStage {
    /// Ray generation shader.
    RayGeneration,
    /// Miss shader.
    Miss,
    /// Closest hit shader.
    ClosestHit,
    /// Any hit shader.
    AnyHit,
    /// Intersection shader (procedural).
    Intersection,
    /// Callable shader.
    Callable,
}

/// Shader group type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderGroupType {
    /// General (ray gen, miss, callable).
    General,
    /// Triangles hit group.
    TrianglesHitGroup,
    /// Procedural hit group.
    ProceduralHitGroup,
}

/// Shader group definition.
#[derive(Debug, Clone)]
pub struct ShaderGroup {
    /// Group type.
    pub group_type: ShaderGroupType,
    /// General shader index (ray gen, miss, callable).
    pub general_shader: Option<u32>,
    /// Closest hit shader index.
    pub closest_hit_shader: Option<u32>,
    /// Any hit shader index.
    pub any_hit_shader: Option<u32>,
    /// Intersection shader index.
    pub intersection_shader: Option<u32>,
}

impl ShaderGroup {
    /// Create ray generation group.
    pub fn ray_generation(shader_index: u32) -> Self {
        Self {
            group_type: ShaderGroupType::General,
            general_shader: Some(shader_index),
            closest_hit_shader: None,
            any_hit_shader: None,
            intersection_shader: None,
        }
    }

    /// Create miss group.
    pub fn miss(shader_index: u32) -> Self {
        Self {
            group_type: ShaderGroupType::General,
            general_shader: Some(shader_index),
            closest_hit_shader: None,
            any_hit_shader: None,
            intersection_shader: None,
        }
    }

    /// Create triangles hit group.
    pub fn triangles_hit(closest_hit: Option<u32>, any_hit: Option<u32>) -> Self {
        Self {
            group_type: ShaderGroupType::TrianglesHitGroup,
            general_shader: None,
            closest_hit_shader: closest_hit,
            any_hit_shader: any_hit,
            intersection_shader: None,
        }
    }

    /// Create procedural hit group.
    pub fn procedural_hit(
        intersection: u32,
        closest_hit: Option<u32>,
        any_hit: Option<u32>,
    ) -> Self {
        Self {
            group_type: ShaderGroupType::ProceduralHitGroup,
            general_shader: None,
            closest_hit_shader: closest_hit,
            any_hit_shader: any_hit,
            intersection_shader: Some(intersection),
        }
    }

    /// Create callable group.
    pub fn callable(shader_index: u32) -> Self {
        Self {
            group_type: ShaderGroupType::General,
            general_shader: Some(shader_index),
            closest_hit_shader: None,
            any_hit_shader: None,
            intersection_shader: None,
        }
    }
}

/// Shader stage for ray tracing pipeline.
#[derive(Debug, Clone)]
pub struct RayTracingShaderStageDesc {
    /// Stage type.
    pub stage: RayTracingShaderStage,
    /// Shader module handle.
    pub module: u64,
    /// Entry point name.
    pub entry_point: String,
}

/// Ray tracing pipeline description.
#[derive(Debug, Clone)]
pub struct RayTracingPipelineDesc2 {
    /// Debug name.
    pub name: Option<String>,
    /// Shader stages.
    pub stages: Vec<RayTracingShaderStageDesc>,
    /// Shader groups.
    pub groups: Vec<ShaderGroup>,
    /// Maximum ray recursion depth.
    pub max_recursion_depth: u32,
    /// Maximum ray payload size.
    pub max_payload_size: u32,
    /// Maximum hit attribute size.
    pub max_attribute_size: u32,
    /// Pipeline layout handle.
    pub layout: u64,
}

impl Default for RayTracingPipelineDesc2 {
    fn default() -> Self {
        Self {
            name: None,
            stages: Vec::new(),
            groups: Vec::new(),
            max_recursion_depth: 1,
            max_payload_size: 32,
            max_attribute_size: 8,
            layout: 0,
        }
    }
}

// ============================================================================
// Shader Binding Table
// ============================================================================

/// Region in shader binding table.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShaderBindingTableRegion {
    /// Buffer handle.
    pub buffer: BufferHandle,
    /// Offset in buffer.
    pub offset: u64,
    /// Stride between records.
    pub stride: u64,
    /// Size of region.
    pub size: u64,
}

/// Shader binding table.
#[derive(Debug, Clone)]
pub struct ShaderBindingTable {
    /// Ray generation region.
    pub ray_gen: ShaderBindingTableRegion,
    /// Miss region.
    pub miss: ShaderBindingTableRegion,
    /// Hit region.
    pub hit: ShaderBindingTableRegion,
    /// Callable region.
    pub callable: ShaderBindingTableRegion,
}

impl Default for ShaderBindingTable {
    fn default() -> Self {
        Self {
            ray_gen: ShaderBindingTableRegion::default(),
            miss: ShaderBindingTableRegion::default(),
            hit: ShaderBindingTableRegion::default(),
            callable: ShaderBindingTableRegion::default(),
        }
    }
}

/// Shader binding table builder.
pub struct ShaderBindingTableBuilder {
    /// Handle size.
    handle_size: u32,
    /// Handle alignment.
    handle_alignment: u32,
    /// Base alignment.
    base_alignment: u32,
    /// Ray gen entries.
    ray_gen_entries: Vec<Vec<u8>>,
    /// Miss entries.
    miss_entries: Vec<Vec<u8>>,
    /// Hit entries.
    hit_entries: Vec<Vec<u8>>,
    /// Callable entries.
    callable_entries: Vec<Vec<u8>>,
}

impl ShaderBindingTableBuilder {
    /// Create new builder.
    pub fn new(handle_size: u32, handle_alignment: u32, base_alignment: u32) -> Self {
        Self {
            handle_size,
            handle_alignment,
            base_alignment,
            ray_gen_entries: Vec::new(),
            miss_entries: Vec::new(),
            hit_entries: Vec::new(),
            callable_entries: Vec::new(),
        }
    }

    /// Add ray generation entry.
    pub fn add_ray_gen(&mut self, handle: &[u8], parameters: Option<&[u8]>) {
        let mut entry = handle.to_vec();
        if let Some(params) = parameters {
            entry.extend_from_slice(params);
        }
        self.ray_gen_entries.push(entry);
    }

    /// Add miss entry.
    pub fn add_miss(&mut self, handle: &[u8], parameters: Option<&[u8]>) {
        let mut entry = handle.to_vec();
        if let Some(params) = parameters {
            entry.extend_from_slice(params);
        }
        self.miss_entries.push(entry);
    }

    /// Add hit entry.
    pub fn add_hit(&mut self, handle: &[u8], parameters: Option<&[u8]>) {
        let mut entry = handle.to_vec();
        if let Some(params) = parameters {
            entry.extend_from_slice(params);
        }
        self.hit_entries.push(entry);
    }

    /// Add callable entry.
    pub fn add_callable(&mut self, handle: &[u8], parameters: Option<&[u8]>) {
        let mut entry = handle.to_vec();
        if let Some(params) = parameters {
            entry.extend_from_slice(params);
        }
        self.callable_entries.push(entry);
    }

    /// Align value up.
    fn align_up(value: u64, alignment: u64) -> u64 {
        (value + alignment - 1) & !(alignment - 1)
    }

    /// Calculate required buffer size.
    pub fn calculate_size(&self) -> u64 {
        let base_align = self.base_alignment as u64;
        let handle_align = self.handle_alignment as u64;

        let ray_gen_size = if !self.ray_gen_entries.is_empty() {
            let max_entry = self
                .ray_gen_entries
                .iter()
                .map(|e| e.len())
                .max()
                .unwrap_or(0);
            let stride = Self::align_up(max_entry as u64, handle_align);
            Self::align_up(stride * self.ray_gen_entries.len() as u64, base_align)
        } else {
            0
        };

        let miss_size = if !self.miss_entries.is_empty() {
            let max_entry = self.miss_entries.iter().map(|e| e.len()).max().unwrap_or(0);
            let stride = Self::align_up(max_entry as u64, handle_align);
            Self::align_up(stride * self.miss_entries.len() as u64, base_align)
        } else {
            0
        };

        let hit_size = if !self.hit_entries.is_empty() {
            let max_entry = self.hit_entries.iter().map(|e| e.len()).max().unwrap_or(0);
            let stride = Self::align_up(max_entry as u64, handle_align);
            Self::align_up(stride * self.hit_entries.len() as u64, base_align)
        } else {
            0
        };

        let callable_size = if !self.callable_entries.is_empty() {
            let max_entry = self
                .callable_entries
                .iter()
                .map(|e| e.len())
                .max()
                .unwrap_or(0);
            let stride = Self::align_up(max_entry as u64, handle_align);
            Self::align_up(stride * self.callable_entries.len() as u64, base_align)
        } else {
            0
        };

        ray_gen_size + miss_size + hit_size + callable_size
    }
}

// ============================================================================
// Ray Query
// ============================================================================

/// Ray query flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RayFlags(u32);

impl RayFlags {
    /// No flags.
    pub const NONE: Self = Self(0);
    /// Force opaque.
    pub const FORCE_OPAQUE: Self = Self(1 << 0);
    /// Force non-opaque.
    pub const FORCE_NON_OPAQUE: Self = Self(1 << 1);
    /// Accept first hit and terminate.
    pub const ACCEPT_FIRST_HIT_AND_END_SEARCH: Self = Self(1 << 2);
    /// Skip closest hit shader.
    pub const SKIP_CLOSEST_HIT_SHADER: Self = Self(1 << 3);
    /// Cull back faces.
    pub const CULL_BACK_FACING_TRIANGLES: Self = Self(1 << 4);
    /// Cull front faces.
    pub const CULL_FRONT_FACING_TRIANGLES: Self = Self(1 << 5);
    /// Cull opaque.
    pub const CULL_OPAQUE: Self = Self(1 << 6);
    /// Cull non-opaque.
    pub const CULL_NON_OPAQUE: Self = Self(1 << 7);
    /// Skip triangles.
    pub const SKIP_TRIANGLES: Self = Self(1 << 8);
    /// Skip AABBs.
    pub const SKIP_AABBS: Self = Self(1 << 9);

    /// Combine flags.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl Default for RayFlags {
    fn default() -> Self {
        Self::NONE
    }
}

/// Ray description for tracing.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RayDesc {
    /// Ray origin.
    pub origin: [f32; 3],
    /// Minimum ray distance.
    pub t_min: f32,
    /// Ray direction (normalized).
    pub direction: [f32; 3],
    /// Maximum ray distance.
    pub t_max: f32,
}

impl Default for RayDesc {
    fn default() -> Self {
        Self {
            origin: [0.0; 3],
            t_min: 0.001,
            direction: [0.0, 0.0, 1.0],
            t_max: 10000.0,
        }
    }
}

impl RayDesc {
    /// Create new ray.
    pub fn new(origin: [f32; 3], direction: [f32; 3]) -> Self {
        Self {
            origin,
            direction,
            ..Default::default()
        }
    }

    /// Set range.
    pub fn with_range(mut self, t_min: f32, t_max: f32) -> Self {
        self.t_min = t_min;
        self.t_max = t_max;
        self
    }

    /// Get point at distance t.
    pub fn at(&self, t: f32) -> [f32; 3] {
        [
            self.origin[0] + self.direction[0] * t,
            self.origin[1] + self.direction[1] * t,
            self.origin[2] + self.direction[2] * t,
        ]
    }
}

// ============================================================================
// Trace Ray Command
// ============================================================================

/// Parameters for trace rays command.
#[derive(Debug, Clone)]
pub struct TraceRaysDesc {
    /// Shader binding table.
    pub sbt: ShaderBindingTable,
    /// Width (number of rays).
    pub width: u32,
    /// Height.
    pub height: u32,
    /// Depth.
    pub depth: u32,
}

impl Default for TraceRaysDesc {
    fn default() -> Self {
        Self {
            sbt: ShaderBindingTable::default(),
            width: 1,
            height: 1,
            depth: 1,
        }
    }
}

/// Parameters for indirect trace rays command.
#[derive(Debug, Clone)]
pub struct TraceRaysIndirectDesc {
    /// Shader binding table.
    pub sbt: ShaderBindingTable,
    /// Indirect buffer.
    pub indirect_buffer: BufferHandle,
    /// Indirect buffer offset.
    pub indirect_offset: u64,
}

// ============================================================================
// Ray Tracing Manager
// ============================================================================

/// Statistics for ray tracing.
#[derive(Debug, Clone, Copy, Default)]
pub struct RayTracingStatistics {
    /// Number of BLAS.
    pub blas_count: u32,
    /// Number of TLAS.
    pub tlas_count: u32,
    /// Total BLAS size.
    pub total_blas_size: u64,
    /// Total TLAS size.
    pub total_tlas_size: u64,
    /// Total scratch size.
    pub total_scratch_size: u64,
    /// Rays traced this frame.
    pub rays_traced: u64,
}

/// Acceleration structure info.
#[derive(Debug, Clone)]
pub struct AccelerationStructureInfo {
    /// Handle.
    pub handle: AccelerationStructureHandle,
    /// Type.
    pub structure_type: AccelerationStructureType,
    /// Size in bytes.
    pub size: u64,
    /// GPU address.
    pub gpu_address: u64,
    /// Compacted size (if compaction performed).
    pub compacted_size: Option<u64>,
    /// Build flags.
    pub flags: AccelerationStructureBuildFlags,
    /// Debug name.
    pub name: Option<String>,
}

/// Ray tracing manager.
pub struct RayTracingManager {
    /// Next handle ID.
    next_handle: u64,
    /// BLAS list.
    blas_list: Vec<AccelerationStructureInfo>,
    /// TLAS list.
    tlas_list: Vec<AccelerationStructureInfo>,
    /// Statistics.
    statistics: RayTracingStatistics,
    /// Is supported.
    supported: bool,
    /// Pipeline handle size.
    pub pipeline_handle_size: u32,
    /// Pipeline handle alignment.
    pub pipeline_handle_alignment: u32,
    /// Pipeline base alignment.
    pub pipeline_base_alignment: u32,
}

impl RayTracingManager {
    /// Create new ray tracing manager.
    pub fn new() -> Self {
        Self {
            next_handle: 1,
            blas_list: Vec::new(),
            tlas_list: Vec::new(),
            statistics: RayTracingStatistics::default(),
            supported: false,
            pipeline_handle_size: 32,
            pipeline_handle_alignment: 64,
            pipeline_base_alignment: 64,
        }
    }

    /// Initialize with device capabilities.
    pub fn initialize(&mut self, supported: bool) {
        self.supported = supported;
    }

    /// Check if ray tracing is supported.
    pub fn is_supported(&self) -> bool {
        self.supported
    }

    /// Allocate new handle.
    fn allocate_handle(&mut self) -> AccelerationStructureHandle {
        let handle = AccelerationStructureHandle(self.next_handle);
        self.next_handle += 1;
        handle
    }

    /// Get BLAS build sizes.
    pub fn get_blas_build_sizes(&self, desc: &BlasDesc) -> AccelerationStructureSizes {
        // Calculate estimated sizes based on geometry
        let mut vertex_count = 0u64;
        let mut triangle_count = 0u64;

        for geom in &desc.geometries {
            match geom {
                GeometryDesc::Triangles(tri) => {
                    vertex_count += tri.vertex_count as u64;
                    triangle_count += if tri.index_count > 0 {
                        (tri.index_count / 3) as u64
                    } else {
                        (tri.vertex_count / 3) as u64
                    };
                },
                GeometryDesc::Aabbs(aabb) => {
                    triangle_count += (aabb.count * 12) as u64; // Estimate
                },
            }
        }

        // Rough estimates (actual sizes depend on driver)
        let bvh_size = triangle_count * 64 + vertex_count * 16;
        let scratch_size = triangle_count * 128;

        AccelerationStructureSizes {
            acceleration_structure_size: bvh_size.max(1024),
            build_scratch_size: scratch_size.max(1024),
            update_scratch_size: scratch_size / 2,
        }
    }

    /// Get TLAS build sizes.
    pub fn get_tlas_build_sizes(&self, desc: &TlasDesc) -> AccelerationStructureSizes {
        let instance_count = desc.max_instance_count as u64;

        AccelerationStructureSizes {
            acceleration_structure_size: instance_count * 128 + 1024,
            build_scratch_size: instance_count * 64 + 1024,
            update_scratch_size: instance_count * 32 + 512,
        }
    }

    /// Create BLAS.
    pub fn create_blas(&mut self, desc: &BlasDesc) -> AccelerationStructureHandle {
        let handle = self.allocate_handle();
        let sizes = self.get_blas_build_sizes(desc);

        let info = AccelerationStructureInfo {
            handle,
            structure_type: AccelerationStructureType::BottomLevel,
            size: sizes.acceleration_structure_size,
            gpu_address: 0, // Set during build
            compacted_size: None,
            flags: desc.flags,
            name: desc.name.clone(),
        };

        self.blas_list.push(info);
        self.statistics.blas_count += 1;
        self.statistics.total_blas_size += sizes.acceleration_structure_size;

        handle
    }

    /// Create TLAS.
    pub fn create_tlas(&mut self, desc: &TlasDesc) -> AccelerationStructureHandle {
        let handle = self.allocate_handle();
        let sizes = self.get_tlas_build_sizes(desc);

        let info = AccelerationStructureInfo {
            handle,
            structure_type: AccelerationStructureType::TopLevel,
            size: sizes.acceleration_structure_size,
            gpu_address: 0, // Set during build
            compacted_size: None,
            flags: desc.flags,
            name: desc.name.clone(),
        };

        self.tlas_list.push(info);
        self.statistics.tlas_count += 1;
        self.statistics.total_tlas_size += sizes.acceleration_structure_size;

        handle
    }

    /// Destroy acceleration structure.
    pub fn destroy(&mut self, handle: AccelerationStructureHandle) {
        if let Some(idx) = self.blas_list.iter().position(|b| b.handle == handle) {
            let info = self.blas_list.remove(idx);
            self.statistics.blas_count -= 1;
            self.statistics.total_blas_size -= info.size;
        } else if let Some(idx) = self.tlas_list.iter().position(|t| t.handle == handle) {
            let info = self.tlas_list.remove(idx);
            self.statistics.tlas_count -= 1;
            self.statistics.total_tlas_size -= info.size;
        }
    }

    /// Get acceleration structure info.
    pub fn get_info(
        &self,
        handle: AccelerationStructureHandle,
    ) -> Option<&AccelerationStructureInfo> {
        self.blas_list
            .iter()
            .find(|b| b.handle == handle)
            .or_else(|| self.tlas_list.iter().find(|t| t.handle == handle))
    }

    /// Get statistics.
    pub fn statistics(&self) -> &RayTracingStatistics {
        &self.statistics
    }

    /// Create shader binding table builder.
    pub fn create_sbt_builder(&self) -> ShaderBindingTableBuilder {
        ShaderBindingTableBuilder::new(
            self.pipeline_handle_size,
            self.pipeline_handle_alignment,
            self.pipeline_base_alignment,
        )
    }

    /// Reset frame statistics.
    pub fn reset_frame_statistics(&mut self) {
        self.statistics.rays_traced = 0;
    }
}

impl Default for RayTracingManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Ray Tracing Features
// ============================================================================

/// Ray tracing feature flags.
#[derive(Debug, Clone, Copy, Default)]
pub struct RayTracingFeatures {
    /// Basic ray tracing support.
    pub ray_tracing: bool,
    /// Ray query support (inline ray tracing).
    pub ray_query: bool,
    /// Acceleration structure indirect builds.
    pub indirect_build: bool,
    /// Acceleration structure host commands.
    pub host_commands: bool,
    /// Ray tracing motion blur.
    pub motion_blur: bool,
    /// Ray tracing opacity micromap.
    pub opacity_micromap: bool,
    /// Ray tracing displacement micromap.
    pub displacement_micromap: bool,
    /// Ray tracing invocation reorder.
    pub invocation_reorder: bool,
    /// Ray tracing position fetch.
    pub position_fetch: bool,
}

/// Ray tracing limits.
#[derive(Debug, Clone, Copy)]
pub struct RayTracingLimits {
    /// Maximum ray recursion depth.
    pub max_recursion_depth: u32,
    /// Maximum ray hit attribute size.
    pub max_ray_hit_attribute_size: u32,
    /// Maximum ray payload size.
    pub max_ray_payload_size: u32,
    /// Maximum ray dispatch invocation count.
    pub max_ray_dispatch_invocation_count: u32,
    /// Maximum geometry count per BLAS.
    pub max_geometry_count: u32,
    /// Maximum instance count per TLAS.
    pub max_instance_count: u32,
    /// Maximum primitives per BLAS.
    pub max_primitive_count: u64,
    /// Shader group handle size.
    pub shader_group_handle_size: u32,
    /// Shader group handle alignment.
    pub shader_group_handle_alignment: u32,
    /// Shader group base alignment.
    pub shader_group_base_alignment: u32,
}

impl Default for RayTracingLimits {
    fn default() -> Self {
        Self {
            max_recursion_depth: 31,
            max_ray_hit_attribute_size: 32,
            max_ray_payload_size: 128,
            max_ray_dispatch_invocation_count: 1073741824, // 2^30
            max_geometry_count: 16777216,                  // 2^24
            max_instance_count: 16777216,                  // 2^24
            max_primitive_count: 536870912,                // 2^29
            shader_group_handle_size: 32,
            shader_group_handle_alignment: 64,
            shader_group_base_alignment: 64,
        }
    }
}

// ============================================================================
// Denoiser Support
// ============================================================================

/// Denoiser type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DenoiserType {
    /// Temporal denoiser.
    Temporal,
    /// Spatial denoiser.
    Spatial,
    /// AI denoiser (OptiX-style).
    AI,
}

/// Denoiser input.
#[derive(Debug, Clone)]
pub struct DenoiserInput {
    /// Color texture.
    pub color: u64,
    /// Albedo texture.
    pub albedo: Option<u64>,
    /// Normal texture.
    pub normal: Option<u64>,
    /// Motion vectors texture.
    pub motion: Option<u64>,
    /// Depth texture.
    pub depth: Option<u64>,
}

/// Denoiser settings.
#[derive(Debug, Clone, Copy)]
pub struct DenoiserSettings {
    /// Denoiser type.
    pub denoiser_type: DenoiserType,
    /// Blend factor.
    pub blend_factor: f32,
    /// Use temporal accumulation.
    pub temporal_accumulation: bool,
    /// Kernel radius.
    pub kernel_radius: u32,
}

impl Default for DenoiserSettings {
    fn default() -> Self {
        Self {
            denoiser_type: DenoiserType::Temporal,
            blend_factor: 0.05,
            temporal_accumulation: true,
            kernel_radius: 5,
        }
    }
}
