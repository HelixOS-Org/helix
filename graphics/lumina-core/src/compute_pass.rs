//! Compute Pass Types for Lumina
//!
//! This module provides compute pass configuration and dispatch types
//! for executing compute workloads on the GPU.

// ============================================================================
// Compute Pass Handle
// ============================================================================

/// Compute pass handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ComputePassHandle(pub u64);

impl ComputePassHandle {
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

impl Default for ComputePassHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Compute Pass Configuration
// ============================================================================

/// Compute pass configuration
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ComputePassConfig {
    /// Debug label
    pub label: Option<&'static str>,
    /// Flags
    pub flags: ComputePassFlags,
    /// Timestamp write begin
    pub timestamp_begin: Option<TimestampWrite>,
    /// Timestamp write end
    pub timestamp_end: Option<TimestampWrite>,
}

impl ComputePassConfig {
    /// Creates new compute pass config
    #[inline]
    pub const fn new() -> Self {
        Self {
            label: None,
            flags: ComputePassFlags::NONE,
            timestamp_begin: None,
            timestamp_end: None,
        }
    }

    /// With label
    #[inline]
    pub const fn with_label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: ComputePassFlags) -> Self {
        self.flags = flags;
        self
    }

    /// With timestamp writes
    #[inline]
    pub const fn with_timestamps(mut self, begin: TimestampWrite, end: TimestampWrite) -> Self {
        self.timestamp_begin = Some(begin);
        self.timestamp_end = Some(end);
        self
    }
}

/// Compute pass flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ComputePassFlags(pub u32);

impl ComputePassFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Suspending pass (can be resumed)
    pub const SUSPENDING: Self = Self(1 << 0);
    /// Resuming pass (continues from suspended)
    pub const RESUMING: Self = Self(1 << 1);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Timestamp Write
// ============================================================================

/// Timestamp write configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TimestampWrite {
    /// Query pool handle (opaque)
    pub query_pool: u64,
    /// Query index
    pub query_index: u32,
}

impl TimestampWrite {
    /// Creates new timestamp write
    #[inline]
    pub const fn new(query_pool: u64, query_index: u32) -> Self {
        Self {
            query_pool,
            query_index,
        }
    }
}

// ============================================================================
// Dispatch Configuration
// ============================================================================

/// Compute dispatch configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DispatchConfig {
    /// Workgroup count X
    pub groups_x: u32,
    /// Workgroup count Y
    pub groups_y: u32,
    /// Workgroup count Z
    pub groups_z: u32,
}

impl DispatchConfig {
    /// Creates new dispatch config
    #[inline]
    pub const fn new(groups_x: u32, groups_y: u32, groups_z: u32) -> Self {
        Self {
            groups_x,
            groups_y,
            groups_z,
        }
    }

    /// 1D dispatch
    #[inline]
    pub const fn d1(groups_x: u32) -> Self {
        Self::new(groups_x, 1, 1)
    }

    /// 2D dispatch
    #[inline]
    pub const fn d2(groups_x: u32, groups_y: u32) -> Self {
        Self::new(groups_x, groups_y, 1)
    }

    /// 3D dispatch
    #[inline]
    pub const fn d3(groups_x: u32, groups_y: u32, groups_z: u32) -> Self {
        Self::new(groups_x, groups_y, groups_z)
    }

    /// From element count and workgroup size (1D)
    #[inline]
    pub const fn from_elements_1d(elements: u32, workgroup_size: u32) -> Self {
        let groups = (elements + workgroup_size - 1) / workgroup_size;
        Self::d1(groups)
    }

    /// From element count and workgroup size (2D)
    #[inline]
    pub const fn from_elements_2d(
        width: u32,
        height: u32,
        workgroup_size_x: u32,
        workgroup_size_y: u32,
    ) -> Self {
        let groups_x = (width + workgroup_size_x - 1) / workgroup_size_x;
        let groups_y = (height + workgroup_size_y - 1) / workgroup_size_y;
        Self::d2(groups_x, groups_y)
    }

    /// From element count and workgroup size (3D)
    #[inline]
    pub const fn from_elements_3d(
        width: u32,
        height: u32,
        depth: u32,
        workgroup_size_x: u32,
        workgroup_size_y: u32,
        workgroup_size_z: u32,
    ) -> Self {
        let groups_x = (width + workgroup_size_x - 1) / workgroup_size_x;
        let groups_y = (height + workgroup_size_y - 1) / workgroup_size_y;
        let groups_z = (depth + workgroup_size_z - 1) / workgroup_size_z;
        Self::d3(groups_x, groups_y, groups_z)
    }

    /// Total workgroup count
    #[inline]
    pub const fn total_groups(&self) -> u64 {
        self.groups_x as u64 * self.groups_y as u64 * self.groups_z as u64
    }

    /// Total invocation count (with workgroup size)
    #[inline]
    pub const fn total_invocations(
        &self,
        workgroup_size_x: u32,
        workgroup_size_y: u32,
        workgroup_size_z: u32,
    ) -> u64 {
        self.total_groups()
            * workgroup_size_x as u64
            * workgroup_size_y as u64
            * workgroup_size_z as u64
    }

    /// Common workgroup sizes
    pub const WORKGROUP_64: (u32, u32, u32) = (64, 1, 1);
    pub const WORKGROUP_128: (u32, u32, u32) = (128, 1, 1);
    pub const WORKGROUP_256: (u32, u32, u32) = (256, 1, 1);
    pub const WORKGROUP_8X8: (u32, u32, u32) = (8, 8, 1);
    pub const WORKGROUP_16X16: (u32, u32, u32) = (16, 16, 1);
    pub const WORKGROUP_32X32: (u32, u32, u32) = (32, 32, 1);
    pub const WORKGROUP_4X4X4: (u32, u32, u32) = (4, 4, 4);
    pub const WORKGROUP_8X8X8: (u32, u32, u32) = (8, 8, 8);
}

impl Default for DispatchConfig {
    fn default() -> Self {
        Self::d1(1)
    }
}

// ============================================================================
// Indirect Dispatch
// ============================================================================

/// Indirect dispatch arguments (GPU-driven)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct IndirectDispatchArgs {
    /// Workgroup count X
    pub groups_x: u32,
    /// Workgroup count Y
    pub groups_y: u32,
    /// Workgroup count Z
    pub groups_z: u32,
}

impl IndirectDispatchArgs {
    /// Size in bytes
    pub const SIZE: usize = 12;

    /// Creates new args
    #[inline]
    pub const fn new(groups_x: u32, groups_y: u32, groups_z: u32) -> Self {
        Self {
            groups_x,
            groups_y,
            groups_z,
        }
    }

    /// From dispatch config
    #[inline]
    pub const fn from_config(config: DispatchConfig) -> Self {
        Self {
            groups_x: config.groups_x,
            groups_y: config.groups_y,
            groups_z: config.groups_z,
        }
    }
}

impl Default for IndirectDispatchArgs {
    fn default() -> Self {
        Self::new(1, 1, 1)
    }
}

/// Indirect dispatch configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct IndirectDispatchConfig {
    /// Buffer handle (opaque)
    pub buffer: u64,
    /// Offset in buffer
    pub offset: u64,
}

impl IndirectDispatchConfig {
    /// Creates new indirect dispatch config
    #[inline]
    pub const fn new(buffer: u64, offset: u64) -> Self {
        Self { buffer, offset }
    }
}

// ============================================================================
// Compute Pipeline State
// ============================================================================

/// Compute pipeline handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ComputePipelineHandle(pub u64);

impl ComputePipelineHandle {
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

impl Default for ComputePipelineHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Compute pipeline configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ComputePipelineConfig {
    /// Shader module handle
    pub shader: u64,
    /// Entry point name
    pub entry_point: &'static str,
    /// Pipeline layout handle
    pub layout: u64,
    /// Specialization constants
    pub specialization: Option<SpecializationInfo>,
    /// Pipeline flags
    pub flags: ComputePipelineFlags,
    /// Base pipeline for derivatives
    pub base_pipeline: Option<ComputePipelineHandle>,
}

impl ComputePipelineConfig {
    /// Creates new compute pipeline config
    #[inline]
    pub const fn new(shader: u64, entry_point: &'static str, layout: u64) -> Self {
        Self {
            shader,
            entry_point,
            layout,
            specialization: None,
            flags: ComputePipelineFlags::NONE,
            base_pipeline: None,
        }
    }

    /// With specialization
    #[inline]
    pub const fn with_specialization(mut self, spec: SpecializationInfo) -> Self {
        self.specialization = Some(spec);
        self
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: ComputePipelineFlags) -> Self {
        self.flags = flags;
        self
    }

    /// As derivative base
    #[inline]
    pub const fn as_derivative_base(mut self) -> Self {
        self.flags = self.flags.union(ComputePipelineFlags::ALLOW_DERIVATIVES);
        self
    }

    /// As derivative of
    #[inline]
    pub const fn as_derivative_of(mut self, base: ComputePipelineHandle) -> Self {
        self.flags = self.flags.union(ComputePipelineFlags::DERIVATIVE);
        self.base_pipeline = Some(base);
        self
    }
}

/// Compute pipeline flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ComputePipelineFlags(pub u32);

impl ComputePipelineFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Disable optimization
    pub const DISABLE_OPTIMIZATION: Self = Self(1 << 0);
    /// Allow derivatives
    pub const ALLOW_DERIVATIVES: Self = Self(1 << 1);
    /// Derivative pipeline
    pub const DERIVATIVE: Self = Self(1 << 2);
    /// Capture statistics
    pub const CAPTURE_STATISTICS: Self = Self(1 << 3);
    /// Capture internal representations
    pub const CAPTURE_INTERNAL_REPRESENTATIONS: Self = Self(1 << 4);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Specialization info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SpecializationInfo {
    /// Map entries
    pub entries: &'static [SpecializationMapEntry],
    /// Data pointer
    pub data: *const u8,
    /// Data size
    pub data_size: usize,
}

impl SpecializationInfo {
    /// Creates new specialization info
    #[inline]
    pub const fn new(
        entries: &'static [SpecializationMapEntry],
        data: *const u8,
        data_size: usize,
    ) -> Self {
        Self {
            entries,
            data,
            data_size,
        }
    }
}

/// Specialization map entry
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SpecializationMapEntry {
    /// Constant ID
    pub constant_id: u32,
    /// Offset in data
    pub offset: u32,
    /// Size in bytes
    pub size: usize,
}

impl SpecializationMapEntry {
    /// Creates new entry
    #[inline]
    pub const fn new(constant_id: u32, offset: u32, size: usize) -> Self {
        Self {
            constant_id,
            offset,
            size,
        }
    }

    /// For u32 constant
    #[inline]
    pub const fn uint32(constant_id: u32, offset: u32) -> Self {
        Self::new(constant_id, offset, 4)
    }

    /// For i32 constant
    #[inline]
    pub const fn int32(constant_id: u32, offset: u32) -> Self {
        Self::new(constant_id, offset, 4)
    }

    /// For f32 constant
    #[inline]
    pub const fn float32(constant_id: u32, offset: u32) -> Self {
        Self::new(constant_id, offset, 4)
    }

    /// For bool constant
    #[inline]
    pub const fn bool(constant_id: u32, offset: u32) -> Self {
        Self::new(constant_id, offset, 4) // Vulkan uses VkBool32
    }
}

// ============================================================================
// Workgroup Configuration
// ============================================================================

/// Workgroup size configuration
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct WorkgroupSize {
    /// X dimension
    pub x: u32,
    /// Y dimension
    pub y: u32,
    /// Z dimension
    pub z: u32,
}

impl WorkgroupSize {
    /// Common sizes
    pub const S64: Self = Self { x: 64, y: 1, z: 1 };
    pub const S128: Self = Self { x: 128, y: 1, z: 1 };
    pub const S256: Self = Self { x: 256, y: 1, z: 1 };
    pub const S512: Self = Self { x: 512, y: 1, z: 1 };
    pub const S8X8: Self = Self { x: 8, y: 8, z: 1 };
    pub const S16X16: Self = Self { x: 16, y: 16, z: 1 };
    pub const S32X32: Self = Self { x: 32, y: 32, z: 1 };
    pub const S4X4X4: Self = Self { x: 4, y: 4, z: 4 };
    pub const S8X8X8: Self = Self { x: 8, y: 8, z: 8 };

    /// Creates new workgroup size
    #[inline]
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// 1D workgroup
    #[inline]
    pub const fn d1(x: u32) -> Self {
        Self { x, y: 1, z: 1 }
    }

    /// 2D workgroup
    #[inline]
    pub const fn d2(x: u32, y: u32) -> Self {
        Self { x, y, z: 1 }
    }

    /// 3D workgroup
    #[inline]
    pub const fn d3(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// Total invocations per workgroup
    #[inline]
    pub const fn total_invocations(&self) -> u32 {
        self.x * self.y * self.z
    }

    /// Is power of two in all dimensions
    #[inline]
    pub const fn is_power_of_two(&self) -> bool {
        self.x.is_power_of_two() && self.y.is_power_of_two() && self.z.is_power_of_two()
    }
}

impl Default for WorkgroupSize {
    fn default() -> Self {
        Self::S64
    }
}

// ============================================================================
// Compute Limits
// ============================================================================

/// Compute limits
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ComputeLimits {
    /// Maximum workgroup count X
    pub max_workgroup_count_x: u32,
    /// Maximum workgroup count Y
    pub max_workgroup_count_y: u32,
    /// Maximum workgroup count Z
    pub max_workgroup_count_z: u32,
    /// Maximum workgroup size X
    pub max_workgroup_size_x: u32,
    /// Maximum workgroup size Y
    pub max_workgroup_size_y: u32,
    /// Maximum workgroup size Z
    pub max_workgroup_size_z: u32,
    /// Maximum total workgroup invocations
    pub max_workgroup_invocations: u32,
    /// Maximum shared memory size
    pub max_shared_memory_size: u32,
    /// Subgroup size
    pub subgroup_size: u32,
}

impl ComputeLimits {
    /// Default conservative limits
    pub const DEFAULT: Self = Self {
        max_workgroup_count_x: 65535,
        max_workgroup_count_y: 65535,
        max_workgroup_count_z: 65535,
        max_workgroup_size_x: 1024,
        max_workgroup_size_y: 1024,
        max_workgroup_size_z: 64,
        max_workgroup_invocations: 1024,
        max_shared_memory_size: 32768,
        subgroup_size: 32,
    };

    /// NVIDIA-like limits
    pub const NVIDIA_LIKE: Self = Self {
        max_workgroup_count_x: 2147483647,
        max_workgroup_count_y: 65535,
        max_workgroup_count_z: 65535,
        max_workgroup_size_x: 1024,
        max_workgroup_size_y: 1024,
        max_workgroup_size_z: 64,
        max_workgroup_invocations: 1024,
        max_shared_memory_size: 49152,
        subgroup_size: 32,
    };

    /// AMD-like limits
    pub const AMD_LIKE: Self = Self {
        max_workgroup_count_x: 2147483647,
        max_workgroup_count_y: 65535,
        max_workgroup_count_z: 65535,
        max_workgroup_size_x: 1024,
        max_workgroup_size_y: 1024,
        max_workgroup_size_z: 1024,
        max_workgroup_invocations: 1024,
        max_shared_memory_size: 65536,
        subgroup_size: 64,
    };

    /// Intel-like limits
    pub const INTEL_LIKE: Self = Self {
        max_workgroup_count_x: 2147483647,
        max_workgroup_count_y: 2147483647,
        max_workgroup_count_z: 2147483647,
        max_workgroup_size_x: 1024,
        max_workgroup_size_y: 1024,
        max_workgroup_size_z: 1024,
        max_workgroup_invocations: 1024,
        max_shared_memory_size: 65536,
        subgroup_size: 16,
    };

    /// Validate workgroup size
    #[inline]
    pub const fn validate_workgroup_size(&self, size: WorkgroupSize) -> bool {
        size.x <= self.max_workgroup_size_x
            && size.y <= self.max_workgroup_size_y
            && size.z <= self.max_workgroup_size_z
            && size.total_invocations() <= self.max_workgroup_invocations
    }

    /// Validate dispatch
    #[inline]
    pub const fn validate_dispatch(&self, config: DispatchConfig) -> bool {
        config.groups_x <= self.max_workgroup_count_x
            && config.groups_y <= self.max_workgroup_count_y
            && config.groups_z <= self.max_workgroup_count_z
    }
}

impl Default for ComputeLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

// ============================================================================
// Memory Barrier
// ============================================================================

/// Memory barrier type for compute
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ComputeMemoryBarrier {
    /// No barrier
    #[default]
    None   = 0,
    /// Buffer barrier
    Buffer = 1,
    /// Image barrier
    Image  = 2,
    /// Memory barrier (global)
    Memory = 3,
}

/// Memory barrier scope
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum BarrierScope {
    /// Workgroup scope
    #[default]
    Workgroup   = 0,
    /// Subgroup scope
    Subgroup    = 1,
    /// Device scope
    Device      = 2,
    /// Queue family scope
    QueueFamily = 3,
}

// ============================================================================
// Push Constants
// ============================================================================

/// Push constant data for compute
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ComputePushConstants {
    /// Offset in bytes
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
    /// Data pointer
    pub data: *const u8,
}

impl ComputePushConstants {
    /// Maximum push constant size (common limit)
    pub const MAX_SIZE: u32 = 128;

    /// Creates new push constants
    #[inline]
    pub const fn new(offset: u32, size: u32, data: *const u8) -> Self {
        Self { offset, size, data }
    }

    /// From offset and data slice
    #[inline]
    pub fn from_slice(offset: u32, data: &[u8]) -> Self {
        Self {
            offset,
            size: data.len() as u32,
            data: data.as_ptr(),
        }
    }
}

// ============================================================================
// Subgroup Operations
// ============================================================================

/// Subgroup operation type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SubgroupOperation {
    /// Basic operations
    Basic           = 0,
    /// Vote operations
    Vote            = 1,
    /// Arithmetic operations
    Arithmetic      = 2,
    /// Ballot operations
    Ballot          = 3,
    /// Shuffle operations
    Shuffle         = 4,
    /// Shuffle relative operations
    ShuffleRelative = 5,
    /// Clustered operations
    Clustered       = 6,
    /// Quad operations
    Quad            = 7,
    /// Partitioned operations
    Partitioned     = 8,
}

/// Subgroup features flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SubgroupFeatures(pub u32);

impl SubgroupFeatures {
    /// No features
    pub const NONE: Self = Self(0);
    /// Basic subgroup operations
    pub const BASIC: Self = Self(1 << 0);
    /// Vote operations
    pub const VOTE: Self = Self(1 << 1);
    /// Arithmetic operations
    pub const ARITHMETIC: Self = Self(1 << 2);
    /// Ballot operations
    pub const BALLOT: Self = Self(1 << 3);
    /// Shuffle operations
    pub const SHUFFLE: Self = Self(1 << 4);
    /// Shuffle relative operations
    pub const SHUFFLE_RELATIVE: Self = Self(1 << 5);
    /// Clustered operations
    pub const CLUSTERED: Self = Self(1 << 6);
    /// Quad operations
    pub const QUAD: Self = Self(1 << 7);
    /// Partitioned operations (NV extension)
    pub const PARTITIONED: Self = Self(1 << 8);
    /// All standard operations
    pub const ALL: Self = Self(0x1FF);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Compute Queue Properties
// ============================================================================

/// Compute queue properties
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ComputeQueueProperties {
    /// Queue family index
    pub queue_family_index: u32,
    /// Number of queues in family
    pub queue_count: u32,
    /// Timestamp valid bits
    pub timestamp_valid_bits: u32,
    /// Minimum image transfer granularity
    pub min_image_transfer_granularity: Extent3D,
    /// Supports async compute
    pub async_compute: bool,
    /// Supports sparse binding
    pub sparse_binding: bool,
}

impl ComputeQueueProperties {
    /// Creates new properties
    #[inline]
    pub const fn new(queue_family_index: u32, queue_count: u32) -> Self {
        Self {
            queue_family_index,
            queue_count,
            timestamp_valid_bits: 64,
            min_image_transfer_granularity: Extent3D {
                width: 1,
                height: 1,
                depth: 1,
            },
            async_compute: true,
            sparse_binding: false,
        }
    }
}

impl Default for ComputeQueueProperties {
    fn default() -> Self {
        Self::new(0, 1)
    }
}

/// 3D extent
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct Extent3D {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth
    pub depth: u32,
}

impl Extent3D {
    /// Creates new extent
    #[inline]
    pub const fn new(width: u32, height: u32, depth: u32) -> Self {
        Self {
            width,
            height,
            depth,
        }
    }

    /// Unit extent
    pub const UNIT: Self = Self {
        width: 1,
        height: 1,
        depth: 1,
    };

    /// Volume
    #[inline]
    pub const fn volume(&self) -> u64 {
        self.width as u64 * self.height as u64 * self.depth as u64
    }
}
