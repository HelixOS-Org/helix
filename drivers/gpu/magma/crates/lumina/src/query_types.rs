//! Query Pool Types for Lumina
//!
//! This module provides query pool configuration, query types,
//! and pipeline statistics queries.

// ============================================================================
// Query Pool Handle
// ============================================================================

/// Query pool handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct QueryPoolHandle(pub u64);

impl QueryPoolHandle {
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

impl Default for QueryPoolHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Query Pool Create Info
// ============================================================================

/// Query pool create info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct QueryPoolCreateInfo {
    /// Flags
    pub flags: QueryPoolCreateFlags,
    /// Query type
    pub query_type: QueryType,
    /// Query count
    pub query_count: u32,
    /// Pipeline statistics (for pipeline statistics queries)
    pub pipeline_statistics: QueryPipelineStatisticFlags,
}

impl QueryPoolCreateInfo {
    /// Creates new info
    #[inline]
    pub const fn new(query_type: QueryType, count: u32) -> Self {
        Self {
            flags: QueryPoolCreateFlags::NONE,
            query_type,
            query_count: count,
            pipeline_statistics: QueryPipelineStatisticFlags::NONE,
        }
    }

    /// Occlusion query pool
    #[inline]
    pub const fn occlusion(count: u32) -> Self {
        Self::new(QueryType::Occlusion, count)
    }

    /// Timestamp query pool
    #[inline]
    pub const fn timestamp(count: u32) -> Self {
        Self::new(QueryType::Timestamp, count)
    }

    /// Pipeline statistics query pool
    #[inline]
    pub const fn pipeline_statistics(
        count: u32,
        stats: QueryPipelineStatisticFlags,
    ) -> Self {
        Self {
            flags: QueryPoolCreateFlags::NONE,
            query_type: QueryType::PipelineStatistics,
            query_count: count,
            pipeline_statistics: stats,
        }
    }

    /// Transform feedback stream query pool
    #[inline]
    pub const fn transform_feedback(count: u32) -> Self {
        Self::new(QueryType::TransformFeedbackStream, count)
    }

    /// Performance query pool
    #[inline]
    pub const fn performance(count: u32) -> Self {
        Self::new(QueryType::PerformanceQuery, count)
    }

    /// Acceleration structure compacted size query pool
    #[inline]
    pub const fn acceleration_structure_compacted_size(count: u32) -> Self {
        Self::new(QueryType::AccelerationStructureCompactedSize, count)
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: QueryPoolCreateFlags) -> Self {
        self.flags = flags;
        self
    }
}

impl Default for QueryPoolCreateInfo {
    fn default() -> Self {
        Self::new(QueryType::Occlusion, 1)
    }
}

/// Query pool create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct QueryPoolCreateFlags(pub u32);

impl QueryPoolCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);

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
// Query Type
// ============================================================================

/// Query type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum QueryType {
    /// Occlusion query
    #[default]
    Occlusion = 0,
    /// Pipeline statistics query
    PipelineStatistics = 1,
    /// Timestamp query
    Timestamp = 2,
    /// Transform feedback stream query
    TransformFeedbackStream = 1000028004,
    /// Performance query (KHR)
    PerformanceQuery = 1000116000,
    /// Acceleration structure compacted size
    AccelerationStructureCompactedSize = 1000150000,
    /// Acceleration structure serialization size
    AccelerationStructureSerializationSize = 1000150001,
    /// Acceleration structure size
    AccelerationStructureSize = 1000386016,
    /// Acceleration structure serialization bottom level pointers
    AccelerationStructureSerializationBottomLevelPointers = 1000386017,
    /// Primitives generated
    PrimitivesGenerated = 1000382000,
    /// Mesh primitives generated
    MeshPrimitivesGenerated = 1000328000,
    /// Video encode feedback
    VideoEncodeFeedback = 1000299000,
    /// Result status only
    ResultStatusOnly = 1000023000,
    /// Micromap serialization size
    MicromapSerializationSize = 1000396001,
    /// Micromap compacted size
    MicromapCompactedSize = 1000396002,
}

impl QueryType {
    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Occlusion => "Occlusion",
            Self::PipelineStatistics => "Pipeline Statistics",
            Self::Timestamp => "Timestamp",
            Self::TransformFeedbackStream => "Transform Feedback Stream",
            Self::PerformanceQuery => "Performance Query",
            Self::AccelerationStructureCompactedSize => "Acceleration Structure Compacted Size",
            Self::AccelerationStructureSerializationSize => {
                "Acceleration Structure Serialization Size"
            }
            Self::AccelerationStructureSize => "Acceleration Structure Size",
            Self::AccelerationStructureSerializationBottomLevelPointers => {
                "Acceleration Structure Serialization Bottom Level Pointers"
            }
            Self::PrimitivesGenerated => "Primitives Generated",
            Self::MeshPrimitivesGenerated => "Mesh Primitives Generated",
            Self::VideoEncodeFeedback => "Video Encode Feedback",
            Self::ResultStatusOnly => "Result Status Only",
            Self::MicromapSerializationSize => "Micromap Serialization Size",
            Self::MicromapCompactedSize => "Micromap Compacted Size",
        }
    }

    /// Result size in bytes (per query)
    #[inline]
    pub const fn result_size(&self) -> u32 {
        match self {
            Self::Occlusion => 8,
            Self::PipelineStatistics => 8, // Per statistic
            Self::Timestamp => 8,
            Self::TransformFeedbackStream => 16,
            _ => 8,
        }
    }

    /// Is acceleration structure query
    #[inline]
    pub const fn is_acceleration_structure(&self) -> bool {
        matches!(
            self,
            Self::AccelerationStructureCompactedSize
                | Self::AccelerationStructureSerializationSize
                | Self::AccelerationStructureSize
                | Self::AccelerationStructureSerializationBottomLevelPointers
        )
    }
}

// ============================================================================
// Query Pipeline Statistic Flags
// ============================================================================

/// Query pipeline statistic flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct QueryPipelineStatisticFlags(pub u32);

impl QueryPipelineStatisticFlags {
    /// No statistics
    pub const NONE: Self = Self(0);
    /// Input assembly vertices
    pub const INPUT_ASSEMBLY_VERTICES: Self = Self(1 << 0);
    /// Input assembly primitives
    pub const INPUT_ASSEMBLY_PRIMITIVES: Self = Self(1 << 1);
    /// Vertex shader invocations
    pub const VERTEX_SHADER_INVOCATIONS: Self = Self(1 << 2);
    /// Geometry shader invocations
    pub const GEOMETRY_SHADER_INVOCATIONS: Self = Self(1 << 3);
    /// Geometry shader primitives
    pub const GEOMETRY_SHADER_PRIMITIVES: Self = Self(1 << 4);
    /// Clipping invocations
    pub const CLIPPING_INVOCATIONS: Self = Self(1 << 5);
    /// Clipping primitives
    pub const CLIPPING_PRIMITIVES: Self = Self(1 << 6);
    /// Fragment shader invocations
    pub const FRAGMENT_SHADER_INVOCATIONS: Self = Self(1 << 7);
    /// Tessellation control shader patches
    pub const TESSELLATION_CONTROL_SHADER_PATCHES: Self = Self(1 << 8);
    /// Tessellation evaluation shader invocations
    pub const TESSELLATION_EVALUATION_SHADER_INVOCATIONS: Self = Self(1 << 9);
    /// Compute shader invocations
    pub const COMPUTE_SHADER_INVOCATIONS: Self = Self(1 << 10);
    /// Task shader invocations
    pub const TASK_SHADER_INVOCATIONS: Self = Self(1 << 11);
    /// Mesh shader invocations
    pub const MESH_SHADER_INVOCATIONS: Self = Self(1 << 12);
    /// Cluster culling shader invocations
    pub const CLUSTER_CULLING_SHADER_INVOCATIONS: Self = Self(1 << 13);

    /// All graphics statistics
    pub const ALL_GRAPHICS: Self = Self(0x07FF);
    /// All statistics
    pub const ALL: Self = Self(0x3FFF);

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

    /// Intersection
    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Count
    #[inline]
    pub const fn count(&self) -> u32 {
        self.0.count_ones()
    }

    /// Result size in bytes
    #[inline]
    pub const fn result_size(&self) -> u32 {
        self.count() * 8 // 8 bytes per statistic
    }
}

// ============================================================================
// Query Control Flags
// ============================================================================

/// Query control flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct QueryControlFlags(pub u32);

impl QueryControlFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Precise
    pub const PRECISE: Self = Self(1 << 0);

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
// Query Result Flags
// ============================================================================

/// Query result flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct QueryResultFlags(pub u32);

impl QueryResultFlags {
    /// No flags (32-bit, wait)
    pub const NONE: Self = Self(0);
    /// 64-bit results
    pub const N64_BIT: Self = Self(1 << 0);
    /// Wait for results
    pub const WAIT: Self = Self(1 << 1);
    /// With availability
    pub const WITH_AVAILABILITY: Self = Self(1 << 2);
    /// Partial results
    pub const PARTIAL: Self = Self(1 << 3);
    /// With status (for video)
    pub const WITH_STATUS: Self = Self(1 << 4);

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

    /// Is 64-bit
    #[inline]
    pub const fn is_64_bit(&self) -> bool {
        self.contains(Self::N64_BIT)
    }

    /// Result stride
    #[inline]
    pub const fn stride(&self) -> usize {
        if self.is_64_bit() { 8 } else { 4 }
    }
}

// ============================================================================
// Query Commands
// ============================================================================

/// Begin query info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BeginQueryInfo {
    /// Query pool
    pub query_pool: QueryPoolHandle,
    /// Query index
    pub query: u32,
    /// Control flags
    pub flags: QueryControlFlags,
}

impl BeginQueryInfo {
    /// Creates new info
    #[inline]
    pub const fn new(pool: QueryPoolHandle, query: u32) -> Self {
        Self {
            query_pool: pool,
            query,
            flags: QueryControlFlags::NONE,
        }
    }

    /// Precise query
    #[inline]
    pub const fn precise(pool: QueryPoolHandle, query: u32) -> Self {
        Self {
            query_pool: pool,
            query,
            flags: QueryControlFlags::PRECISE,
        }
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: QueryControlFlags) -> Self {
        self.flags = flags;
        self
    }
}

impl Default for BeginQueryInfo {
    fn default() -> Self {
        Self::new(QueryPoolHandle::NULL, 0)
    }
}

/// End query info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct EndQueryInfo {
    /// Query pool
    pub query_pool: QueryPoolHandle,
    /// Query index
    pub query: u32,
}

impl EndQueryInfo {
    /// Creates new info
    #[inline]
    pub const fn new(pool: QueryPoolHandle, query: u32) -> Self {
        Self { query_pool: pool, query }
    }
}

impl Default for EndQueryInfo {
    fn default() -> Self {
        Self::new(QueryPoolHandle::NULL, 0)
    }
}

/// Reset query pool info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ResetQueryPoolInfo {
    /// Query pool
    pub query_pool: QueryPoolHandle,
    /// First query
    pub first_query: u32,
    /// Query count
    pub query_count: u32,
}

impl ResetQueryPoolInfo {
    /// Creates new info
    #[inline]
    pub const fn new(pool: QueryPoolHandle, first: u32, count: u32) -> Self {
        Self {
            query_pool: pool,
            first_query: first,
            query_count: count,
        }
    }

    /// Reset all
    #[inline]
    pub const fn all(pool: QueryPoolHandle, count: u32) -> Self {
        Self::new(pool, 0, count)
    }
}

impl Default for ResetQueryPoolInfo {
    fn default() -> Self {
        Self::new(QueryPoolHandle::NULL, 0, 0)
    }
}

/// Write timestamp info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct WriteTimestampInfo {
    /// Pipeline stage
    pub pipeline_stage: PipelineStage,
    /// Query pool
    pub query_pool: QueryPoolHandle,
    /// Query index
    pub query: u32,
}

impl WriteTimestampInfo {
    /// Creates new info
    #[inline]
    pub const fn new(stage: PipelineStage, pool: QueryPoolHandle, query: u32) -> Self {
        Self {
            pipeline_stage: stage,
            query_pool: pool,
            query,
        }
    }

    /// At top of pipe
    #[inline]
    pub const fn top_of_pipe(pool: QueryPoolHandle, query: u32) -> Self {
        Self::new(PipelineStage::TopOfPipe, pool, query)
    }

    /// At bottom of pipe
    #[inline]
    pub const fn bottom_of_pipe(pool: QueryPoolHandle, query: u32) -> Self {
        Self::new(PipelineStage::BottomOfPipe, pool, query)
    }
}

impl Default for WriteTimestampInfo {
    fn default() -> Self {
        Self::new(PipelineStage::TopOfPipe, QueryPoolHandle::NULL, 0)
    }
}

/// Pipeline stage (for timestamps)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PipelineStage {
    /// Top of pipe
    #[default]
    TopOfPipe = 1,
    /// Draw indirect
    DrawIndirect = 2,
    /// Vertex input
    VertexInput = 4,
    /// Vertex shader
    VertexShader = 8,
    /// Tessellation control shader
    TessellationControlShader = 16,
    /// Tessellation evaluation shader
    TessellationEvaluationShader = 32,
    /// Geometry shader
    GeometryShader = 64,
    /// Fragment shader
    FragmentShader = 128,
    /// Early fragment tests
    EarlyFragmentTests = 256,
    /// Late fragment tests
    LateFragmentTests = 512,
    /// Color attachment output
    ColorAttachmentOutput = 1024,
    /// Compute shader
    ComputeShader = 2048,
    /// Transfer
    Transfer = 4096,
    /// Bottom of pipe
    BottomOfPipe = 8192,
    /// Host
    Host = 16384,
    /// All graphics
    AllGraphics = 32768,
    /// All commands
    AllCommands = 65536,
}

// ============================================================================
// Get Query Results
// ============================================================================

/// Get query pool results info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct GetQueryPoolResultsInfo {
    /// Query pool
    pub query_pool: QueryPoolHandle,
    /// First query
    pub first_query: u32,
    /// Query count
    pub query_count: u32,
    /// Flags
    pub flags: QueryResultFlags,
}

impl GetQueryPoolResultsInfo {
    /// Creates new info
    #[inline]
    pub const fn new(pool: QueryPoolHandle, first: u32, count: u32) -> Self {
        Self {
            query_pool: pool,
            first_query: first,
            query_count: count,
            flags: QueryResultFlags::N64_BIT.union(QueryResultFlags::WAIT),
        }
    }

    /// Single query
    #[inline]
    pub const fn single(pool: QueryPoolHandle, query: u32) -> Self {
        Self::new(pool, query, 1)
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: QueryResultFlags) -> Self {
        self.flags = flags;
        self
    }

    /// No wait
    #[inline]
    pub const fn no_wait(mut self) -> Self {
        self.flags = QueryResultFlags::N64_BIT;
        self
    }
}

impl Default for GetQueryPoolResultsInfo {
    fn default() -> Self {
        Self::new(QueryPoolHandle::NULL, 0, 0)
    }
}

// ============================================================================
// Copy Query Results
// ============================================================================

/// Copy query pool results info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CopyQueryPoolResultsInfo {
    /// Query pool
    pub query_pool: QueryPoolHandle,
    /// First query
    pub first_query: u32,
    /// Query count
    pub query_count: u32,
    /// Destination buffer
    pub dst_buffer: u64,
    /// Destination offset
    pub dst_offset: u64,
    /// Stride
    pub stride: u64,
    /// Flags
    pub flags: QueryResultFlags,
}

impl CopyQueryPoolResultsInfo {
    /// Creates new info
    #[inline]
    pub const fn new(
        pool: QueryPoolHandle,
        first: u32,
        count: u32,
        dst_buffer: u64,
        dst_offset: u64,
        stride: u64,
    ) -> Self {
        Self {
            query_pool: pool,
            first_query: first,
            query_count: count,
            dst_buffer,
            dst_offset,
            stride,
            flags: QueryResultFlags::N64_BIT.union(QueryResultFlags::WAIT),
        }
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: QueryResultFlags) -> Self {
        self.flags = flags;
        self
    }
}

impl Default for CopyQueryPoolResultsInfo {
    fn default() -> Self {
        Self::new(QueryPoolHandle::NULL, 0, 0, 0, 0, 8)
    }
}

// ============================================================================
// Performance Query
// ============================================================================

/// Performance counter result
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub union PerformanceCounterResult {
    /// Int32 result
    pub int32: i32,
    /// Int64 result
    pub int64: i64,
    /// Uint32 result
    pub uint32: u32,
    /// Uint64 result
    pub uint64: u64,
    /// Float32 result
    pub float32: f32,
    /// Float64 result
    pub float64: f64,
}

impl Default for PerformanceCounterResult {
    fn default() -> Self {
        Self { uint64: 0 }
    }
}

/// Performance counter unit
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PerformanceCounterUnit {
    /// Generic
    #[default]
    Generic = 0,
    /// Percentage
    Percentage = 1,
    /// Nanoseconds
    Nanoseconds = 2,
    /// Bytes
    Bytes = 3,
    /// Bytes per second
    BytesPerSecond = 4,
    /// Kelvin
    Kelvin = 5,
    /// Watts
    Watts = 6,
    /// Volts
    Volts = 7,
    /// Amps
    Amps = 8,
    /// Hertz
    Hertz = 9,
    /// Cycles
    Cycles = 10,
}

impl PerformanceCounterUnit {
    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Generic => "Generic",
            Self::Percentage => "%",
            Self::Nanoseconds => "ns",
            Self::Bytes => "B",
            Self::BytesPerSecond => "B/s",
            Self::Kelvin => "K",
            Self::Watts => "W",
            Self::Volts => "V",
            Self::Amps => "A",
            Self::Hertz => "Hz",
            Self::Cycles => "cycles",
        }
    }
}

/// Performance counter scope
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PerformanceCounterScope {
    /// Command buffer
    #[default]
    CommandBuffer = 0,
    /// Render pass
    RenderPass = 1,
    /// Command
    Command = 2,
}

/// Performance counter storage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PerformanceCounterStorage {
    /// Int32
    Int32 = 0,
    /// Int64
    Int64 = 1,
    /// Uint32
    Uint32 = 2,
    /// Uint64
    #[default]
    Uint64 = 3,
    /// Float32
    Float32 = 4,
    /// Float64
    Float64 = 5,
}

/// Performance counter description
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PerformanceCounterDescription {
    /// Flags
    pub flags: PerformanceCounterDescriptionFlags,
    /// Name
    pub name: [u8; 256],
    /// Category
    pub category: [u8; 256],
    /// Description
    pub description: [u8; 256],
}

impl Default for PerformanceCounterDescription {
    fn default() -> Self {
        Self {
            flags: PerformanceCounterDescriptionFlags::NONE,
            name: [0; 256],
            category: [0; 256],
            description: [0; 256],
        }
    }
}

/// Performance counter description flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PerformanceCounterDescriptionFlags(pub u32);

impl PerformanceCounterDescriptionFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Performance impacting
    pub const PERFORMANCE_IMPACTING: Self = Self(1 << 0);
    /// Concurrently impacted
    pub const CONCURRENTLY_IMPACTED: Self = Self(1 << 1);

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
// Occlusion Query Results
// ============================================================================

/// Occlusion query result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct OcclusionQueryResult {
    /// Samples passed
    pub samples_passed: u64,
}

impl OcclusionQueryResult {
    /// Is visible (any samples passed)
    #[inline]
    pub const fn is_visible(&self) -> bool {
        self.samples_passed > 0
    }
}

/// Pipeline statistics result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PipelineStatisticsResult {
    /// Input assembly vertices
    pub input_assembly_vertices: u64,
    /// Input assembly primitives
    pub input_assembly_primitives: u64,
    /// Vertex shader invocations
    pub vertex_shader_invocations: u64,
    /// Geometry shader invocations
    pub geometry_shader_invocations: u64,
    /// Geometry shader primitives
    pub geometry_shader_primitives: u64,
    /// Clipping invocations
    pub clipping_invocations: u64,
    /// Clipping primitives
    pub clipping_primitives: u64,
    /// Fragment shader invocations
    pub fragment_shader_invocations: u64,
    /// Tessellation control shader patches
    pub tessellation_control_shader_patches: u64,
    /// Tessellation evaluation shader invocations
    pub tessellation_evaluation_shader_invocations: u64,
    /// Compute shader invocations
    pub compute_shader_invocations: u64,
    /// Task shader invocations
    pub task_shader_invocations: u64,
    /// Mesh shader invocations
    pub mesh_shader_invocations: u64,
}

impl PipelineStatisticsResult {
    /// Total shader invocations
    #[inline]
    pub const fn total_shader_invocations(&self) -> u64 {
        self.vertex_shader_invocations
            + self.geometry_shader_invocations
            + self.fragment_shader_invocations
            + self.tessellation_control_shader_patches
            + self.tessellation_evaluation_shader_invocations
            + self.compute_shader_invocations
            + self.task_shader_invocations
            + self.mesh_shader_invocations
    }
}

/// Timestamp result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TimestampResult {
    /// Timestamp value
    pub timestamp: u64,
}

impl TimestampResult {
    /// Time in nanoseconds (given timestamp period)
    #[inline]
    pub const fn to_nanoseconds(&self, timestamp_period: f32) -> f64 {
        self.timestamp as f64 * timestamp_period as f64
    }

    /// Time in milliseconds
    #[inline]
    pub const fn to_milliseconds(&self, timestamp_period: f32) -> f64 {
        self.to_nanoseconds(timestamp_period) / 1_000_000.0
    }

    /// Delta time
    #[inline]
    pub const fn delta(&self, end: &TimestampResult) -> u64 {
        end.timestamp.saturating_sub(self.timestamp)
    }

    /// Delta time in nanoseconds
    #[inline]
    pub const fn delta_nanoseconds(&self, end: &TimestampResult, period: f32) -> f64 {
        self.delta(end) as f64 * period as f64
    }
}
