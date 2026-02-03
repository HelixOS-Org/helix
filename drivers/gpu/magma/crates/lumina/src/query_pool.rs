//! Query types for performance and occlusion queries
//!
//! This module provides GPU query pool types for timing, occlusion, and statistics.

/// Query pool handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct QueryPoolHandle(pub u32);

impl QueryPoolHandle {
    /// Invalid handle
    pub const INVALID: Self = Self(u32::MAX);

    /// Creates new handle
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Checks if valid
    pub const fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }
}

impl Default for QueryPoolHandle {
    fn default() -> Self {
        Self::INVALID
    }
}

/// Query type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum QueryType {
    /// Occlusion query (sample counts)
    Occlusion = 0,
    /// Pipeline statistics
    PipelineStatistics = 1,
    /// Timestamp query
    Timestamp = 2,
    /// Transform feedback primitives written
    TransformFeedbackStream = 3,
    /// Primitives generated
    PrimitivesGenerated = 4,
    /// Acceleration structure compacted size
    AccelerationStructureCompactedSize = 5,
    /// Acceleration structure serialization size
    AccelerationStructureSerializationSize = 6,
    /// Mesh primitives generated
    MeshPrimitivesGenerated = 7,
}

/// Pipeline statistics flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PipelineStatisticsFlags(pub u32);

impl PipelineStatisticsFlags {
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

    /// All vertex-related statistics
    pub const ALL_VERTEX: Self = Self(
        Self::INPUT_ASSEMBLY_VERTICES.0
            | Self::INPUT_ASSEMBLY_PRIMITIVES.0
            | Self::VERTEX_SHADER_INVOCATIONS.0,
    );

    /// All fragment-related statistics
    pub const ALL_FRAGMENT: Self = Self(
        Self::CLIPPING_INVOCATIONS.0
            | Self::CLIPPING_PRIMITIVES.0
            | Self::FRAGMENT_SHADER_INVOCATIONS.0,
    );

    /// All statistics
    pub const ALL: Self = Self(0x1FFF);

    /// Checks if contains flag
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    /// Count of active statistics
    pub const fn count(&self) -> u32 {
        self.0.count_ones()
    }
}

impl core::ops::BitOr for PipelineStatisticsFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Query pool description
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct QueryPoolDesc {
    /// Query type
    pub query_type: QueryType,
    /// Query count
    pub query_count: u32,
    /// Pipeline statistics flags (for PipelineStatistics type)
    pub statistics_flags: PipelineStatisticsFlags,
}

impl QueryPoolDesc {
    /// Creates occlusion query pool
    pub const fn occlusion(count: u32) -> Self {
        Self {
            query_type: QueryType::Occlusion,
            query_count: count,
            statistics_flags: PipelineStatisticsFlags::NONE,
        }
    }

    /// Creates timestamp query pool
    pub const fn timestamp(count: u32) -> Self {
        Self {
            query_type: QueryType::Timestamp,
            query_count: count,
            statistics_flags: PipelineStatisticsFlags::NONE,
        }
    }

    /// Creates pipeline statistics query pool
    pub const fn pipeline_statistics(count: u32, flags: PipelineStatisticsFlags) -> Self {
        Self {
            query_type: QueryType::PipelineStatistics,
            query_count: count,
            statistics_flags: flags,
        }
    }
}

/// Query result flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct QueryResultFlags(pub u32);

impl QueryResultFlags {
    /// No flags (wait for result)
    pub const NONE: Self = Self(0);
    /// Return 64-bit results
    pub const RESULT_64: Self = Self(1 << 0);
    /// Wait for results
    pub const WAIT: Self = Self(1 << 1);
    /// Include availability status
    pub const WITH_AVAILABILITY: Self = Self(1 << 2);
    /// Partial results allowed
    pub const PARTIAL: Self = Self(1 << 3);

    /// Standard 64-bit wait
    pub const WAIT_64: Self = Self(Self::RESULT_64.0 | Self::WAIT.0);
}

impl core::ops::BitOr for QueryResultFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Query control flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct QueryControlFlags(pub u32);

impl QueryControlFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Precise occlusion query
    pub const PRECISE: Self = Self(1 << 0);
}

/// Occlusion query result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct OcclusionQueryResult {
    /// Sample count
    pub sample_count: u64,
    /// Availability (if requested)
    pub available: bool,
}

impl OcclusionQueryResult {
    /// Checks if visible (any samples passed)
    pub const fn is_visible(&self) -> bool {
        self.sample_count > 0
    }
}

/// Timestamp query result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TimestampQueryResult {
    /// Timestamp value in GPU ticks
    pub timestamp: u64,
    /// Availability
    pub available: bool,
}

impl TimestampQueryResult {
    /// Converts to nanoseconds
    pub const fn to_nanoseconds(&self, timestamp_period: f32) -> u64 {
        (self.timestamp as f64 * timestamp_period as f64) as u64
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
    /// Tessellation control patches
    pub tessellation_control_patches: u64,
    /// Tessellation evaluation invocations
    pub tessellation_evaluation_invocations: u64,
    /// Compute shader invocations
    pub compute_shader_invocations: u64,
}

impl PipelineStatisticsResult {
    /// Vertex efficiency (average vertices per primitive)
    pub fn vertex_efficiency(&self) -> f64 {
        if self.input_assembly_primitives > 0 {
            self.input_assembly_vertices as f64 / self.input_assembly_primitives as f64
        } else {
            0.0
        }
    }

    /// Clipping efficiency (primitives surviving clipping)
    pub fn clipping_efficiency(&self) -> f64 {
        if self.clipping_invocations > 0 {
            self.clipping_primitives as f64 / self.clipping_invocations as f64
        } else {
            1.0
        }
    }

    /// Fragment overdraw (fragments per pixel at given resolution)
    pub fn overdraw(&self, pixel_count: u64) -> f64 {
        if pixel_count > 0 {
            self.fragment_shader_invocations as f64 / pixel_count as f64
        } else {
            0.0
        }
    }
}

/// GPU timing scope
#[derive(Clone, Copy, Debug)]
pub struct TimingScope {
    /// Query pool
    pub pool: QueryPoolHandle,
    /// Start query index
    pub start_query: u32,
    /// End query index
    pub end_query: u32,
}

impl TimingScope {
    /// Creates timing scope
    pub const fn new(pool: QueryPoolHandle, start: u32, end: u32) -> Self {
        Self {
            pool,
            start_query: start,
            end_query: end,
        }
    }

    /// Calculates duration from results
    pub fn duration_ns(&self, start_ts: u64, end_ts: u64, timestamp_period: f32) -> u64 {
        let delta = end_ts.saturating_sub(start_ts);
        (delta as f64 * timestamp_period as f64) as u64
    }
}

/// Hierarchical profiler zone
#[derive(Clone, Debug)]
pub struct ProfilerZone {
    /// Zone name
    pub name: &'static str,
    /// Timing scope
    pub scope: TimingScope,
    /// Parent zone index (u32::MAX for root)
    pub parent: u32,
    /// Depth level
    pub depth: u32,
}

/// Profiler frame data
#[derive(Clone, Debug, Default)]
pub struct ProfilerFrame {
    /// Frame index
    pub frame_index: u64,
    /// Timestamp period in nanoseconds
    pub timestamp_period: f32,
    /// Total frame time in nanoseconds
    pub total_time_ns: u64,
}

impl ProfilerFrame {
    /// Frame time in milliseconds
    pub fn frame_time_ms(&self) -> f64 {
        self.total_time_ns as f64 / 1_000_000.0
    }

    /// Frames per second
    pub fn fps(&self) -> f64 {
        if self.total_time_ns > 0 {
            1_000_000_000.0 / self.total_time_ns as f64
        } else {
            0.0
        }
    }
}

/// Performance counters for benchmarking
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PerformanceCounters {
    /// Draw calls
    pub draw_calls: u32,
    /// Dispatch calls
    pub dispatch_calls: u32,
    /// Vertices submitted
    pub vertices: u64,
    /// Triangles submitted
    pub triangles: u64,
    /// Instances drawn
    pub instances: u64,
    /// Bytes uploaded
    pub bytes_uploaded: u64,
    /// Bytes downloaded
    pub bytes_downloaded: u64,
    /// Buffer binds
    pub buffer_binds: u32,
    /// Texture binds
    pub texture_binds: u32,
    /// Pipeline binds
    pub pipeline_binds: u32,
    /// Render passes
    pub render_passes: u32,
}

impl PerformanceCounters {
    /// Resets all counters
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Merges counters
    pub fn merge(&mut self, other: &Self) {
        self.draw_calls += other.draw_calls;
        self.dispatch_calls += other.dispatch_calls;
        self.vertices += other.vertices;
        self.triangles += other.triangles;
        self.instances += other.instances;
        self.bytes_uploaded += other.bytes_uploaded;
        self.bytes_downloaded += other.bytes_downloaded;
        self.buffer_binds += other.buffer_binds;
        self.texture_binds += other.texture_binds;
        self.pipeline_binds += other.pipeline_binds;
        self.render_passes += other.render_passes;
    }
}
