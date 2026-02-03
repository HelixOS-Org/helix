//! GPU Query Types for Lumina
//!
//! This module provides GPU query infrastructure for performance
//! profiling including timestamp, occlusion, and pipeline queries.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Query Handles
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

/// Timestamp query handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TimestampQueryHandle(pub u64);

impl TimestampQueryHandle {
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

impl Default for TimestampQueryHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Occlusion query handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct OcclusionQueryHandle(pub u64);

impl OcclusionQueryHandle {
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

impl Default for OcclusionQueryHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Pipeline statistics query handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineStatsQueryHandle(pub u64);

impl PipelineStatsQueryHandle {
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

impl Default for PipelineStatsQueryHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Query Pool
// ============================================================================

/// Query pool create info
#[derive(Clone, Debug)]
pub struct QueryPoolCreateInfo {
    /// Name
    pub name: String,
    /// Query type
    pub query_type: QueryType,
    /// Query count
    pub query_count: u32,
    /// Pipeline statistics flags (for pipeline stats queries)
    pub pipeline_statistics: PipelineStatisticsFlags,
}

impl QueryPoolCreateInfo {
    /// Creates info
    pub fn new(query_type: QueryType, count: u32) -> Self {
        Self {
            name: String::new(),
            query_type,
            query_count: count,
            pipeline_statistics: PipelineStatisticsFlags::NONE,
        }
    }

    /// Timestamp query pool
    pub fn timestamps(count: u32) -> Self {
        Self::new(QueryType::Timestamp, count)
    }

    /// Occlusion query pool
    pub fn occlusion(count: u32) -> Self {
        Self::new(QueryType::Occlusion, count)
    }

    /// Binary occlusion query pool
    pub fn occlusion_binary(count: u32) -> Self {
        Self::new(QueryType::OcclusionBinary, count)
    }

    /// Pipeline statistics query pool
    pub fn pipeline_stats(count: u32, stats: PipelineStatisticsFlags) -> Self {
        Self {
            pipeline_statistics: stats,
            ..Self::new(QueryType::PipelineStatistics, count)
        }
    }

    /// With name
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = String::from(name);
        self
    }
}

impl Default for QueryPoolCreateInfo {
    fn default() -> Self {
        Self::timestamps(64)
    }
}

/// Query type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum QueryType {
    /// Timestamp
    #[default]
    Timestamp = 0,
    /// Occlusion (sample count)
    Occlusion = 1,
    /// Binary occlusion (visible/not visible)
    OcclusionBinary = 2,
    /// Pipeline statistics
    PipelineStatistics = 3,
    /// Primitives generated (geometry/tessellation)
    PrimitivesGenerated = 4,
    /// Transform feedback primitives written
    TransformFeedbackPrimitives = 5,
}

/// Pipeline statistics flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineStatisticsFlags(pub u32);

impl PipelineStatisticsFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Input assembly vertices
    pub const IA_VERTICES: Self = Self(1 << 0);
    /// Input assembly primitives
    pub const IA_PRIMITIVES: Self = Self(1 << 1);
    /// Vertex shader invocations
    pub const VS_INVOCATIONS: Self = Self(1 << 2);
    /// Geometry shader invocations
    pub const GS_INVOCATIONS: Self = Self(1 << 3);
    /// Geometry shader primitives
    pub const GS_PRIMITIVES: Self = Self(1 << 4);
    /// Clipping invocations
    pub const CLIP_INVOCATIONS: Self = Self(1 << 5);
    /// Clipping primitives
    pub const CLIP_PRIMITIVES: Self = Self(1 << 6);
    /// Fragment shader invocations
    pub const FS_INVOCATIONS: Self = Self(1 << 7);
    /// Tessellation control patches
    pub const TCS_PATCHES: Self = Self(1 << 8);
    /// Tessellation evaluation invocations
    pub const TES_INVOCATIONS: Self = Self(1 << 9);
    /// Compute shader invocations
    pub const CS_INVOCATIONS: Self = Self(1 << 10);
    /// Task shader invocations
    pub const TASK_INVOCATIONS: Self = Self(1 << 11);
    /// Mesh shader invocations
    pub const MESH_INVOCATIONS: Self = Self(1 << 12);

    /// All graphics statistics
    pub const ALL_GRAPHICS: Self = Self(
        Self::IA_VERTICES.0
            | Self::IA_PRIMITIVES.0
            | Self::VS_INVOCATIONS.0
            | Self::FS_INVOCATIONS.0
            | Self::CLIP_INVOCATIONS.0
            | Self::CLIP_PRIMITIVES.0,
    );

    /// All statistics
    pub const ALL: Self = Self(0x1FFF);

    /// Has flag
    pub const fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    /// Count active flags
    pub const fn count(&self) -> u32 {
        let mut count = 0;
        let mut v = self.0;
        while v != 0 {
            count += 1;
            v &= v - 1;
        }
        count
    }
}

impl Default for PipelineStatisticsFlags {
    fn default() -> Self {
        Self::NONE
    }
}

impl core::ops::BitOr for PipelineStatisticsFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

// ============================================================================
// Query Results
// ============================================================================

/// Query result flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct QueryResultFlags(pub u32);

impl QueryResultFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// 64-bit results
    pub const WITH_64_BIT: Self = Self(1 << 0);
    /// Wait for results
    pub const WAIT: Self = Self(1 << 1);
    /// Include availability
    pub const WITH_AVAILABILITY: Self = Self(1 << 2);
    /// Partial results OK
    pub const PARTIAL: Self = Self(1 << 3);

    /// Has flag
    pub const fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl Default for QueryResultFlags {
    fn default() -> Self {
        Self::WITH_64_BIT | Self::WAIT
    }
}

impl core::ops::BitOr for QueryResultFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Timestamp result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TimestampResult {
    /// Timestamp value (GPU ticks)
    pub timestamp: u64,
    /// Availability (if requested)
    pub available: bool,
}

impl TimestampResult {
    /// Creates result
    pub const fn new(timestamp: u64) -> Self {
        Self {
            timestamp,
            available: true,
        }
    }

    /// Convert to nanoseconds
    pub fn to_nanoseconds(&self, period: f64) -> f64 {
        self.timestamp as f64 * period
    }

    /// Convert to microseconds
    pub fn to_microseconds(&self, period: f64) -> f64 {
        self.to_nanoseconds(period) / 1000.0
    }

    /// Convert to milliseconds
    pub fn to_milliseconds(&self, period: f64) -> f64 {
        self.to_nanoseconds(period) / 1_000_000.0
    }
}

/// Occlusion result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct OcclusionResult {
    /// Sample count (or 0/non-zero for binary)
    pub samples: u64,
    /// Availability
    pub available: bool,
}

impl OcclusionResult {
    /// Creates result
    pub const fn new(samples: u64) -> Self {
        Self {
            samples,
            available: true,
        }
    }

    /// Is visible (any samples passed)
    pub const fn is_visible(&self) -> bool {
        self.samples > 0
    }

    /// Is fully occluded
    pub const fn is_occluded(&self) -> bool {
        self.samples == 0
    }
}

/// Pipeline statistics result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PipelineStatsResult {
    /// Input assembly vertices
    pub ia_vertices: u64,
    /// Input assembly primitives
    pub ia_primitives: u64,
    /// Vertex shader invocations
    pub vs_invocations: u64,
    /// Geometry shader invocations
    pub gs_invocations: u64,
    /// Geometry shader primitives
    pub gs_primitives: u64,
    /// Clipping invocations
    pub clip_invocations: u64,
    /// Clipping primitives
    pub clip_primitives: u64,
    /// Fragment shader invocations
    pub fs_invocations: u64,
    /// Tessellation control patches
    pub tcs_patches: u64,
    /// Tessellation evaluation invocations
    pub tes_invocations: u64,
    /// Compute shader invocations
    pub cs_invocations: u64,
    /// Task shader invocations
    pub task_invocations: u64,
    /// Mesh shader invocations
    pub mesh_invocations: u64,
}

impl PipelineStatsResult {
    /// Total shader invocations
    pub fn total_shader_invocations(&self) -> u64 {
        self.vs_invocations
            + self.gs_invocations
            + self.fs_invocations
            + self.tcs_patches
            + self.tes_invocations
            + self.cs_invocations
            + self.task_invocations
            + self.mesh_invocations
    }

    /// Vertex processing efficiency (primitives/vertices)
    pub fn vertex_efficiency(&self) -> f64 {
        if self.ia_vertices > 0 {
            self.ia_primitives as f64 / self.ia_vertices as f64
        } else {
            0.0
        }
    }

    /// Clipping efficiency (output/input)
    pub fn clip_efficiency(&self) -> f64 {
        if self.clip_invocations > 0 {
            self.clip_primitives as f64 / self.clip_invocations as f64
        } else {
            1.0
        }
    }
}

// ============================================================================
// GPU Profiler
// ============================================================================

/// GPU profiler create info
#[derive(Clone, Debug)]
pub struct GpuProfilerCreateInfo {
    /// Name
    pub name: String,
    /// Max timestamp queries per frame
    pub max_timestamps: u32,
    /// Max occlusion queries per frame
    pub max_occlusion_queries: u32,
    /// Max pipeline stats queries
    pub max_pipeline_stats: u32,
    /// Frame count (double/triple buffering)
    pub frame_count: u32,
    /// History size (frames to keep)
    pub history_size: u32,
}

impl GpuProfilerCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_timestamps: 128,
            max_occlusion_queries: 256,
            max_pipeline_stats: 16,
            frame_count: 2,
            history_size: 60,
        }
    }

    /// Minimal profiler
    pub fn minimal() -> Self {
        Self {
            max_timestamps: 32,
            max_occlusion_queries: 0,
            max_pipeline_stats: 0,
            frame_count: 2,
            history_size: 10,
            ..Self::new()
        }
    }

    /// Full profiler
    pub fn full() -> Self {
        Self {
            max_timestamps: 512,
            max_occlusion_queries: 1024,
            max_pipeline_stats: 64,
            frame_count: 3,
            history_size: 120,
            ..Self::new()
        }
    }
}

impl Default for GpuProfilerCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// GPU timing scope
#[derive(Clone, Debug)]
pub struct GpuTimingScope {
    /// Name
    pub name: String,
    /// Start query index
    pub start_query: u32,
    /// End query index
    pub end_query: u32,
    /// Parent scope (for hierarchy)
    pub parent: Option<u32>,
    /// Child scopes
    pub children: Vec<u32>,
}

impl GpuTimingScope {
    /// Creates scope
    pub fn new(name: &str, start_query: u32) -> Self {
        Self {
            name: String::from(name),
            start_query,
            end_query: 0,
            parent: None,
            children: Vec::new(),
        }
    }
}

impl Default for GpuTimingScope {
    fn default() -> Self {
        Self::new("Scope", 0)
    }
}

/// GPU timing result
#[derive(Clone, Debug)]
pub struct GpuTimingResult {
    /// Scope name
    pub name: String,
    /// Duration in nanoseconds
    pub duration_ns: u64,
    /// Duration in microseconds
    pub duration_us: f64,
    /// Duration in milliseconds
    pub duration_ms: f64,
    /// Child results
    pub children: Vec<GpuTimingResult>,
}

impl GpuTimingResult {
    /// Creates result
    pub fn new(name: &str, duration_ns: u64) -> Self {
        Self {
            name: String::from(name),
            duration_ns,
            duration_us: duration_ns as f64 / 1000.0,
            duration_ms: duration_ns as f64 / 1_000_000.0,
            children: Vec::new(),
        }
    }

    /// Total time including children
    pub fn total_time_ms(&self) -> f64 {
        self.duration_ms
    }

    /// Self time (excluding children)
    pub fn self_time_ms(&self) -> f64 {
        let children_time: f64 = self.children.iter().map(|c| c.duration_ms).sum();
        (self.duration_ms - children_time).max(0.0)
    }
}

impl Default for GpuTimingResult {
    fn default() -> Self {
        Self::new("Result", 0)
    }
}

// ============================================================================
// Frame Timing
// ============================================================================

/// Frame timing info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FrameTiming {
    /// Frame index
    pub frame_index: u64,
    /// CPU frame time (microseconds)
    pub cpu_time_us: u64,
    /// GPU frame time (microseconds)
    pub gpu_time_us: u64,
    /// Present latency (microseconds)
    pub present_latency_us: u64,
    /// GPU work start (relative to frame start)
    pub gpu_start_us: u64,
    /// GPU work end (relative to frame start)
    pub gpu_end_us: u64,
}

impl FrameTiming {
    /// Creates timing
    pub const fn new(frame_index: u64) -> Self {
        Self {
            frame_index,
            cpu_time_us: 0,
            gpu_time_us: 0,
            present_latency_us: 0,
            gpu_start_us: 0,
            gpu_end_us: 0,
        }
    }

    /// Frame time in milliseconds
    pub fn frame_time_ms(&self) -> f64 {
        self.cpu_time_us.max(self.gpu_time_us) as f64 / 1000.0
    }

    /// FPS
    pub fn fps(&self) -> f64 {
        let frame_time_ms = self.frame_time_ms();
        if frame_time_ms > 0.0 {
            1000.0 / frame_time_ms
        } else {
            0.0
        }
    }

    /// Is GPU bound
    pub fn is_gpu_bound(&self) -> bool {
        self.gpu_time_us > self.cpu_time_us
    }

    /// Is CPU bound
    pub fn is_cpu_bound(&self) -> bool {
        self.cpu_time_us > self.gpu_time_us
    }
}

/// Frame timing history
#[derive(Clone, Debug)]
pub struct FrameTimingHistory {
    /// Timings
    pub timings: Vec<FrameTiming>,
    /// Max history size
    pub max_size: usize,
}

impl FrameTimingHistory {
    /// Creates history
    pub fn new(max_size: usize) -> Self {
        Self {
            timings: Vec::with_capacity(max_size),
            max_size,
        }
    }

    /// Add timing
    pub fn add(&mut self, timing: FrameTiming) {
        if self.timings.len() >= self.max_size {
            self.timings.remove(0);
        }
        self.timings.push(timing);
    }

    /// Average CPU time
    pub fn avg_cpu_time_us(&self) -> u64 {
        if self.timings.is_empty() {
            return 0;
        }
        let sum: u64 = self.timings.iter().map(|t| t.cpu_time_us).sum();
        sum / self.timings.len() as u64
    }

    /// Average GPU time
    pub fn avg_gpu_time_us(&self) -> u64 {
        if self.timings.is_empty() {
            return 0;
        }
        let sum: u64 = self.timings.iter().map(|t| t.gpu_time_us).sum();
        sum / self.timings.len() as u64
    }

    /// Average FPS
    pub fn avg_fps(&self) -> f64 {
        if self.timings.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.timings.iter().map(|t| t.fps()).sum();
        sum / self.timings.len() as f64
    }

    /// 1% low FPS
    pub fn one_percent_low_fps(&self) -> f64 {
        if self.timings.is_empty() {
            return 0.0;
        }
        let mut sorted: Vec<f64> = self.timings.iter().map(|t| t.fps()).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = (self.timings.len() as f64 * 0.01).ceil() as usize;
        if idx < sorted.len() {
            sorted[idx]
        } else {
            sorted[0]
        }
    }
}

impl Default for FrameTimingHistory {
    fn default() -> Self {
        Self::new(60)
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Query statistics
#[derive(Clone, Debug, Default)]
pub struct QueryStats {
    /// Active query pools
    pub pool_count: u32,
    /// Total queries allocated
    pub query_count: u32,
    /// Queries used this frame
    pub queries_used: u32,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}
