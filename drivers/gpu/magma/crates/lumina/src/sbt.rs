//! Shader binding table types for ray tracing
//!
//! This module provides types for shader binding tables used in ray tracing pipelines.

use core::mem::size_of;

/// Shader group type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ShaderGroupType {
    /// Ray generation shader
    RayGen = 0,
    /// Miss shader
    Miss = 1,
    /// Hit group (closest hit, any hit, intersection)
    HitGroup = 2,
    /// Callable shader
    Callable = 3,
}

/// Shader binding table entry
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SbtEntry {
    /// Shader group handle
    pub shader_group_handle: [u8; 32],
    /// Custom data (optional)
    pub data: [u8; 32],
}

impl SbtEntry {
    /// Creates a new entry with just a handle
    pub const fn new(handle: [u8; 32]) -> Self {
        Self {
            shader_group_handle: handle,
            data: [0; 32],
        }
    }

    /// Creates with custom data
    pub const fn with_data(handle: [u8; 32], data: [u8; 32]) -> Self {
        Self {
            shader_group_handle: handle,
            data,
        }
    }

    /// Size of a minimal entry (handle only)
    pub const HANDLE_SIZE: u32 = 32;

    /// Size of entry with custom data
    pub const FULL_SIZE: u32 = 64;
}

/// Shader binding table region
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SbtRegion {
    /// Device address of the region
    pub device_address: u64,
    /// Stride between entries
    pub stride: u64,
    /// Total size of the region
    pub size: u64,
}

impl SbtRegion {
    /// Creates a new region
    pub const fn new(device_address: u64, stride: u64, size: u64) -> Self {
        Self {
            device_address,
            stride,
            size,
        }
    }

    /// Creates an empty region
    pub const fn empty() -> Self {
        Self {
            device_address: 0,
            stride: 0,
            size: 0,
        }
    }

    /// Creates region for single entry
    pub const fn single(device_address: u64, entry_size: u64) -> Self {
        Self {
            device_address,
            stride: entry_size,
            size: entry_size,
        }
    }

    /// Checks if region is empty
    pub const fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Gets number of entries
    pub const fn entry_count(&self) -> u64 {
        if self.stride > 0 {
            self.size / self.stride
        } else {
            0
        }
    }

    /// Gets address of entry at index
    pub const fn entry_address(&self, index: u64) -> u64 {
        self.device_address + index * self.stride
    }
}

/// Shader binding table layout
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SbtLayout {
    /// Ray generation region
    pub raygen: SbtRegion,
    /// Miss shader region
    pub miss: SbtRegion,
    /// Hit group region
    pub hit: SbtRegion,
    /// Callable shader region
    pub callable: SbtRegion,
}

impl SbtLayout {
    /// Creates a new layout
    pub const fn new(
        raygen: SbtRegion,
        miss: SbtRegion,
        hit: SbtRegion,
        callable: SbtRegion,
    ) -> Self {
        Self {
            raygen,
            miss,
            hit,
            callable,
        }
    }

    /// Creates layout with only required regions
    pub const fn minimal(raygen: SbtRegion, miss: SbtRegion, hit: SbtRegion) -> Self {
        Self {
            raygen,
            miss,
            hit,
            callable: SbtRegion::empty(),
        }
    }

    /// Total size of all regions
    pub const fn total_size(&self) -> u64 {
        self.raygen.size + self.miss.size + self.hit.size + self.callable.size
    }
}

/// Shader group definition
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ShaderGroupInfo {
    /// Group type
    pub group_type: ShaderGroupType,
    /// General shader index (for raygen, miss, callable)
    pub general_shader: u32,
    /// Closest hit shader index
    pub closest_hit_shader: u32,
    /// Any hit shader index
    pub any_hit_shader: u32,
    /// Intersection shader index (for procedural geometry)
    pub intersection_shader: u32,
}

impl Default for ShaderGroupInfo {
    fn default() -> Self {
        Self {
            group_type: ShaderGroupType::RayGen,
            general_shader: SHADER_UNUSED,
            closest_hit_shader: SHADER_UNUSED,
            any_hit_shader: SHADER_UNUSED,
            intersection_shader: SHADER_UNUSED,
        }
    }
}

/// Unused shader constant
pub const SHADER_UNUSED: u32 = !0;

impl ShaderGroupInfo {
    /// Creates a ray generation group
    pub const fn raygen(shader_index: u32) -> Self {
        Self {
            group_type: ShaderGroupType::RayGen,
            general_shader: shader_index,
            closest_hit_shader: SHADER_UNUSED,
            any_hit_shader: SHADER_UNUSED,
            intersection_shader: SHADER_UNUSED,
        }
    }

    /// Creates a miss group
    pub const fn miss(shader_index: u32) -> Self {
        Self {
            group_type: ShaderGroupType::Miss,
            general_shader: shader_index,
            closest_hit_shader: SHADER_UNUSED,
            any_hit_shader: SHADER_UNUSED,
            intersection_shader: SHADER_UNUSED,
        }
    }

    /// Creates a triangles hit group
    pub const fn triangles_hit(closest_hit: u32) -> Self {
        Self {
            group_type: ShaderGroupType::HitGroup,
            general_shader: SHADER_UNUSED,
            closest_hit_shader: closest_hit,
            any_hit_shader: SHADER_UNUSED,
            intersection_shader: SHADER_UNUSED,
        }
    }

    /// Creates a triangles hit group with any-hit
    pub const fn triangles_hit_with_any_hit(closest_hit: u32, any_hit: u32) -> Self {
        Self {
            group_type: ShaderGroupType::HitGroup,
            general_shader: SHADER_UNUSED,
            closest_hit_shader: closest_hit,
            any_hit_shader: any_hit,
            intersection_shader: SHADER_UNUSED,
        }
    }

    /// Creates a procedural hit group
    pub const fn procedural_hit(closest_hit: u32, intersection: u32) -> Self {
        Self {
            group_type: ShaderGroupType::HitGroup,
            general_shader: SHADER_UNUSED,
            closest_hit_shader: closest_hit,
            any_hit_shader: SHADER_UNUSED,
            intersection_shader: intersection,
        }
    }

    /// Creates a callable group
    pub const fn callable(shader_index: u32) -> Self {
        Self {
            group_type: ShaderGroupType::Callable,
            general_shader: shader_index,
            closest_hit_shader: SHADER_UNUSED,
            any_hit_shader: SHADER_UNUSED,
            intersection_shader: SHADER_UNUSED,
        }
    }

    /// Checks if this is a general shader group
    pub const fn is_general(&self) -> bool {
        self.general_shader != SHADER_UNUSED
    }

    /// Checks if this is a hit group
    pub const fn is_hit_group(&self) -> bool {
        matches!(self.group_type, ShaderGroupType::HitGroup)
    }

    /// Checks if this is a procedural hit group
    pub const fn is_procedural(&self) -> bool {
        self.intersection_shader != SHADER_UNUSED
    }
}

/// Hit group record layout
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HitGroupRecord {
    /// Shader group handle
    pub handle: [u8; 32],
    /// Vertex buffer address
    pub vertex_buffer_address: u64,
    /// Index buffer address
    pub index_buffer_address: u64,
    /// Material index
    pub material_index: u32,
    /// Geometry flags
    pub geometry_flags: u32,
}

impl Default for HitGroupRecord {
    fn default() -> Self {
        Self {
            handle: [0; 32],
            vertex_buffer_address: 0,
            index_buffer_address: 0,
            material_index: 0,
            geometry_flags: 0,
        }
    }
}

impl HitGroupRecord {
    /// Size in bytes
    pub const SIZE: u64 = size_of::<Self>() as u64;

    /// Creates a new record
    pub const fn new(
        handle: [u8; 32],
        vertex_buffer: u64,
        index_buffer: u64,
        material_index: u32,
    ) -> Self {
        Self {
            handle,
            vertex_buffer_address: vertex_buffer,
            index_buffer_address: index_buffer,
            material_index,
            geometry_flags: 0,
        }
    }
}

/// Geometry instance data for SBT
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GeometryInstanceData {
    /// Instance custom index (24 bits)
    pub instance_custom_index: u32,
    /// Geometry mask (8 bits)
    pub mask: u8,
    /// SBT record offset
    pub sbt_record_offset: u32,
    /// Instance flags
    pub flags: GeometryInstanceFlags,
    /// Transform (3x4 row-major)
    pub transform: [[f32; 4]; 3],
    /// Acceleration structure address
    pub acceleration_structure_address: u64,
}

impl GeometryInstanceData {
    /// Creates identity instance
    pub const fn identity() -> Self {
        Self {
            instance_custom_index: 0,
            mask: 0xFF,
            sbt_record_offset: 0,
            flags: GeometryInstanceFlags::TRIANGLE_FACING_CULL_DISABLE,
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
            ],
            acceleration_structure_address: 0,
        }
    }

    /// With transform
    pub const fn with_transform(mut self, transform: [[f32; 4]; 3]) -> Self {
        self.transform = transform;
        self
    }

    /// With custom index
    pub const fn with_custom_index(mut self, index: u32) -> Self {
        self.instance_custom_index = index & 0x00FF_FFFF;
        self
    }

    /// With mask
    pub const fn with_mask(mut self, mask: u8) -> Self {
        self.mask = mask;
        self
    }

    /// With SBT offset
    pub const fn with_sbt_offset(mut self, offset: u32) -> Self {
        self.sbt_record_offset = offset;
        self
    }

    /// With flags
    pub const fn with_flags(mut self, flags: GeometryInstanceFlags) -> Self {
        self.flags = flags;
        self
    }

    /// With BLAS address
    pub const fn with_blas(mut self, address: u64) -> Self {
        self.acceleration_structure_address = address;
        self
    }
}

/// Geometry instance flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct GeometryInstanceFlags(pub u32);

impl GeometryInstanceFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Disable face culling
    pub const TRIANGLE_FACING_CULL_DISABLE: Self = Self(1 << 0);
    /// Flip triangle winding
    pub const TRIANGLE_FLIP_FACING: Self = Self(1 << 1);
    /// Force opaque
    pub const FORCE_OPAQUE: Self = Self(1 << 2);
    /// Force no opaque
    pub const FORCE_NO_OPAQUE: Self = Self(1 << 3);
}

impl core::ops::BitOr for GeometryInstanceFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// SBT builder configuration
#[derive(Clone, Copy, Debug)]
pub struct SbtConfig {
    /// Handle size from device
    pub handle_size: u32,
    /// Handle alignment from device
    pub handle_alignment: u32,
    /// Base alignment for regions
    pub base_alignment: u32,
    /// Custom data size per entry
    pub custom_data_size: u32,
}

impl Default for SbtConfig {
    fn default() -> Self {
        Self {
            handle_size: 32,
            handle_alignment: 64,
            base_alignment: 64,
            custom_data_size: 0,
        }
    }
}

impl SbtConfig {
    /// NVIDIA typical config
    pub const NVIDIA: Self = Self {
        handle_size: 32,
        handle_alignment: 64,
        base_alignment: 64,
        custom_data_size: 0,
    };

    /// AMD typical config
    pub const AMD: Self = Self {
        handle_size: 32,
        handle_alignment: 64,
        base_alignment: 64,
        custom_data_size: 0,
    };

    /// Calculates entry stride
    pub const fn entry_stride(&self) -> u32 {
        let size = self.handle_size + self.custom_data_size;
        // Round up to alignment
        (size + self.handle_alignment - 1) / self.handle_alignment * self.handle_alignment
    }

    /// Calculates region size
    pub const fn region_size(&self, entry_count: u32) -> u32 {
        let stride = self.entry_stride();
        let size = stride * entry_count;
        // Round up to base alignment
        (size + self.base_alignment - 1) / self.base_alignment * self.base_alignment
    }

    /// Calculates total SBT size
    pub const fn total_size(&self, raygen: u32, miss: u32, hit: u32, callable: u32) -> u32 {
        self.region_size(raygen)
            + self.region_size(miss)
            + self.region_size(hit)
            + self.region_size(callable)
    }
}

/// Trace ray parameters
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TraceRayParams {
    /// Ray origin
    pub origin: [f32; 3],
    /// Minimum T
    pub t_min: f32,
    /// Ray direction
    pub direction: [f32; 3],
    /// Maximum T
    pub t_max: f32,
}

impl Default for TraceRayParams {
    fn default() -> Self {
        Self {
            origin: [0.0, 0.0, 0.0],
            t_min: 0.001,
            direction: [0.0, 0.0, -1.0],
            t_max: 10000.0,
        }
    }
}

impl TraceRayParams {
    /// Creates from origin and direction
    pub const fn new(origin: [f32; 3], direction: [f32; 3]) -> Self {
        Self {
            origin,
            t_min: 0.001,
            direction,
            t_max: 10000.0,
        }
    }

    /// With T range
    pub const fn with_range(mut self, t_min: f32, t_max: f32) -> Self {
        self.t_min = t_min;
        self.t_max = t_max;
        self
    }
}

/// Ray payload for hit shaders
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct RayPayload {
    /// Hit distance (-1 if miss)
    pub hit_t: f32,
    /// Barycentric coordinates
    pub barycentrics: [f32; 2],
    /// Instance index
    pub instance_index: u32,
    /// Primitive index
    pub primitive_index: u32,
    /// Hit kind
    pub hit_kind: u32,
    /// Front face
    pub front_face: u32,
}

impl RayPayload {
    /// Checks if ray hit something
    pub const fn is_hit(&self) -> bool {
        self.hit_t >= 0.0
    }

    /// Creates a miss payload
    pub const fn miss() -> Self {
        Self {
            hit_t: -1.0,
            barycentrics: [0.0, 0.0],
            instance_index: 0,
            primitive_index: 0,
            hit_kind: 0,
            front_face: 0,
        }
    }
}
