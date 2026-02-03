//! GPU Profiling and Performance Queries for LUMINA
//!
//! This module provides comprehensive GPU profiling capabilities:
//!
//! - Timestamp queries for measuring GPU execution time
//! - Pipeline statistics (vertex count, fragment count, etc.)
//! - Occlusion queries
//! - Performance counters (vendor-specific)
//!
//! ## Usage
//!
//! ```rust,ignore
//! // Create a profiler
//! let profiler = GpuProfiler::new(device, ProfilerConfig::default());
//!
//! // In render loop
//! profiler.begin_frame();
//!
//! {
//!     let _scope = profiler.begin_scope(cmd, "Shadow Pass");
//!     // ... shadow rendering
//! }
//!
//! {
//!     let _scope = profiler.begin_scope(cmd, "Main Pass");
//!     // ... main rendering
//! }
//!
//! profiler.end_frame();
//!
//! // Get results
//! for result in profiler.get_results() {
//!     println!("{}: {:.2}ms", result.name, result.gpu_time_ms);
//! }
//! ```

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, string::String, vec::Vec};
use core::ops::Range;

use crate::error::{Error, Result};

// ============================================================================
// Timestamp Queries
// ============================================================================

/// Handle to a query pool
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct QueryPool {
    handle: u64,
}

impl QueryPool {
    pub const fn null() -> Self {
        Self { handle: 0 }
    }

    pub const fn is_valid(&self) -> bool {
        self.handle != 0
    }

    pub const fn raw(&self) -> u64 {
        self.handle
    }
}

/// Type of queries in a pool
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QueryType {
    /// Occlusion query (number of samples that passed depth/stencil)
    Occlusion,
    /// Pipeline statistics (vertex count, etc.)
    PipelineStatistics,
    /// Timestamp
    Timestamp,
    /// Transform feedback (primitives written)
    TransformFeedback,
    /// Acceleration structure compacted size
    AccelerationStructureCompactedSize,
    /// Acceleration structure serialization size
    AccelerationStructureSerializationSize,
    /// Mesh primitives generated
    MeshPrimitivesGenerated,
    /// Performance query (vendor-specific)
    PerformanceQuery,
}

/// Create info for query pool
#[derive(Clone, Debug)]
pub struct QueryPoolCreateInfo {
    /// Type of queries
    pub query_type: QueryType,
    /// Number of queries in the pool
    pub query_count: u32,
    /// Pipeline statistics flags (only for PipelineStatistics type)
    pub pipeline_statistics: PipelineStatisticsFlags,
}

impl Default for QueryPoolCreateInfo {
    fn default() -> Self {
        Self {
            query_type: QueryType::Timestamp,
            query_count: 256,
            pipeline_statistics: PipelineStatisticsFlags::NONE,
        }
    }
}

/// Pipeline statistics flags
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PipelineStatisticsFlags(u32);

impl PipelineStatisticsFlags {
    pub const NONE: Self = Self(0);
    pub const INPUT_ASSEMBLY_VERTICES: Self = Self(1 << 0);
    pub const INPUT_ASSEMBLY_PRIMITIVES: Self = Self(1 << 1);
    pub const VERTEX_SHADER_INVOCATIONS: Self = Self(1 << 2);
    pub const GEOMETRY_SHADER_INVOCATIONS: Self = Self(1 << 3);
    pub const GEOMETRY_SHADER_PRIMITIVES: Self = Self(1 << 4);
    pub const CLIPPING_INVOCATIONS: Self = Self(1 << 5);
    pub const CLIPPING_PRIMITIVES: Self = Self(1 << 6);
    pub const FRAGMENT_SHADER_INVOCATIONS: Self = Self(1 << 7);
    pub const TESSELLATION_CONTROL_PATCHES: Self = Self(1 << 8);
    pub const TESSELLATION_EVALUATION_INVOCATIONS: Self = Self(1 << 9);
    pub const COMPUTE_SHADER_INVOCATIONS: Self = Self(1 << 10);
    pub const TASK_SHADER_INVOCATIONS: Self = Self(1 << 11);
    pub const MESH_SHADER_INVOCATIONS: Self = Self(1 << 12);

    pub const ALL: Self = Self(0x1FFF);

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Number of statistics enabled
    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }
}

// ============================================================================
// Query Results
// ============================================================================

/// Flags for getting query results
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueryResultFlags(u32);

impl QueryResultFlags {
    pub const NONE: Self = Self(0);
    /// Results are 64-bit
    pub const RESULT_64: Self = Self(1 << 0);
    /// Wait for results to be available
    pub const WAIT: Self = Self(1 << 1);
    /// Write availability status
    pub const WITH_AVAILABILITY: Self = Self(1 << 2);
    /// Partial results are acceptable
    pub const PARTIAL: Self = Self(1 << 3);
    /// Write results with status
    pub const WITH_STATUS: Self = Self(1 << 4);
}

/// Result of a timestamp query
#[derive(Clone, Copy, Debug, Default)]
pub struct TimestampResult {
    /// Raw timestamp value
    pub timestamp: u64,
    /// Whether the result is available
    pub available: bool,
}

/// Result of an occlusion query
#[derive(Clone, Copy, Debug, Default)]
pub struct OcclusionResult {
    /// Number of samples that passed
    pub samples_passed: u64,
    /// Whether the result is available
    pub available: bool,
}

/// Result of pipeline statistics query
#[derive(Clone, Copy, Debug, Default)]
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
    /// Task shader invocations
    pub task_shader_invocations: u64,
    /// Mesh shader invocations
    pub mesh_shader_invocations: u64,
    /// Whether the result is available
    pub available: bool,
}

// ============================================================================
// Performance Counters
// ============================================================================

/// A performance counter description
#[derive(Clone, Debug)]
pub struct PerformanceCounter {
    /// Counter name
    pub name: CounterName,
    /// Counter description
    pub description: CounterDescription,
    /// Counter unit
    pub unit: PerformanceCounterUnit,
    /// Counter storage type
    pub storage: PerformanceCounterStorage,
    /// Counter UUID (for vendor identification)
    pub uuid: [u8; 16],
}

/// Counter name (fixed size)
#[derive(Clone, Debug)]
pub struct CounterName {
    data: [u8; 256],
    len: usize,
}

impl CounterName {
    pub fn new(name: &str) -> Self {
        let mut cn = Self {
            data: [0; 256],
            len: name.len().min(255),
        };
        cn.data[..cn.len].copy_from_slice(name.as_bytes());
        cn
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.data[..self.len]).unwrap_or("")
    }
}

/// Counter description (fixed size)
#[derive(Clone, Debug)]
pub struct CounterDescription {
    data: [u8; 512],
    len: usize,
}

impl CounterDescription {
    pub fn new(desc: &str) -> Self {
        let mut cd = Self {
            data: [0; 512],
            len: desc.len().min(511),
        };
        cd.data[..cd.len].copy_from_slice(desc.as_bytes());
        cd
    }
}

/// Unit of a performance counter
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerformanceCounterUnit {
    Generic,
    Percentage,
    Nanoseconds,
    Bytes,
    BytesPerSecond,
    Kelvin,
    Watts,
    Volts,
    Amps,
    Hertz,
    Cycles,
}

/// Storage type for counter values
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerformanceCounterStorage {
    Int32,
    Int64,
    Uint32,
    Uint64,
    Float32,
    Float64,
}

/// Result of a performance counter
#[derive(Clone, Copy, Debug)]
pub union PerformanceCounterResult {
    pub int32: i32,
    pub int64: i64,
    pub uint32: u32,
    pub uint64: u64,
    pub float32: f32,
    pub float64: f64,
}

impl Default for PerformanceCounterResult {
    fn default() -> Self {
        Self { uint64: 0 }
    }
}

// ============================================================================
// GPU Profiler
// ============================================================================

/// Configuration for the GPU profiler
#[derive(Clone, Debug)]
pub struct ProfilerConfig {
    /// Maximum number of scopes per frame
    pub max_scopes_per_frame: u32,
    /// Number of frames to buffer (for latency hiding)
    pub frame_buffer_count: u32,
    /// Enable pipeline statistics collection
    pub collect_pipeline_stats: bool,
    /// Enable performance counters (vendor-specific)
    pub collect_performance_counters: bool,
    /// Timestamp period in nanoseconds (from device)
    pub timestamp_period_ns: f32,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self {
            max_scopes_per_frame: 256,
            frame_buffer_count: 3,
            collect_pipeline_stats: false,
            collect_performance_counters: false,
            timestamp_period_ns: 1.0,
        }
    }
}

/// A profiling scope
#[derive(Clone, Debug)]
pub struct ProfileScope {
    /// Scope name
    pub name: ScopeName,
    /// Start query index
    pub start_query: u32,
    /// End query index
    pub end_query: u32,
    /// Parent scope index (-1 for root)
    pub parent: i32,
    /// Depth in the hierarchy
    pub depth: u32,
    /// Color for visualization
    pub color: [f32; 4],
}

/// Scope name (fixed size)
#[derive(Clone, Debug)]
pub struct ScopeName {
    data: [u8; 64],
    len: usize,
}

impl ScopeName {
    pub fn new(name: &str) -> Self {
        let mut sn = Self {
            data: [0; 64],
            len: name.len().min(63),
        };
        sn.data[..sn.len].copy_from_slice(name.as_bytes());
        sn
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.data[..self.len]).unwrap_or("")
    }
}

/// Result of a profiling scope
#[derive(Clone, Debug)]
pub struct ProfileScopeResult {
    /// Scope name
    pub name: ScopeName,
    /// GPU time in milliseconds
    pub gpu_time_ms: f64,
    /// GPU time in nanoseconds
    pub gpu_time_ns: u64,
    /// Parent scope index
    pub parent: i32,
    /// Depth in hierarchy
    pub depth: u32,
    /// Pipeline statistics (if enabled)
    pub pipeline_stats: Option<PipelineStatisticsResult>,
}

/// Frame profiling data
#[derive(Clone, Debug, Default)]
pub struct FrameProfile {
    /// Frame number
    pub frame_number: u64,
    /// Total GPU time in milliseconds
    pub total_gpu_time_ms: f64,
    /// Scope results
    pub scopes: [Option<ProfileScopeResult>; 256],
    /// Number of scopes
    pub scope_count: u32,
    /// CPU timestamp at frame start
    pub cpu_frame_start_ns: u64,
    /// CPU timestamp at frame end
    pub cpu_frame_end_ns: u64,
}

impl FrameProfile {
    /// Get scope results iterator
    pub fn scopes(&self) -> impl Iterator<Item = &ProfileScopeResult> {
        self.scopes.iter().take(self.scope_count as usize).flatten()
    }
}

/// GPU Profiler state
#[derive(Debug)]
pub struct GpuProfiler {
    config: ProfilerConfig,
    /// Current frame index
    current_frame: u64,
    /// Current buffer index
    buffer_index: u32,
    /// Query pools (one per buffered frame)
    query_pools: [QueryPool; 4],
    /// Current scope stack
    scope_stack: [i32; 32],
    /// Stack depth
    stack_depth: u32,
    /// Current frame scopes
    current_scopes: [Option<ProfileScope>; 256],
    /// Number of scopes in current frame
    current_scope_count: u32,
    /// Next query index
    next_query: u32,
    /// Results from previous frames
    pending_results: [Option<FrameProfile>; 4],
}

impl GpuProfiler {
    /// Maximum supported buffer count
    pub const MAX_BUFFERS: usize = 4;
    /// Maximum scopes per frame
    pub const MAX_SCOPES: usize = 256;
    /// Maximum scope stack depth
    pub const MAX_STACK_DEPTH: usize = 32;

    /// Create a new profiler (call create_query_pools separately)
    pub fn new(config: ProfilerConfig) -> Self {
        Self {
            config,
            current_frame: 0,
            buffer_index: 0,
            query_pools: [QueryPool::null(); 4],
            scope_stack: [-1; 32],
            stack_depth: 0,
            current_scopes: [const { None }; 256],
            current_scope_count: 0,
            next_query: 0,
            pending_results: [const { None }; 4],
        }
    }

    /// Get configuration
    pub fn config(&self) -> &ProfilerConfig {
        &self.config
    }

    /// Get current frame number
    pub fn current_frame(&self) -> u64 {
        self.current_frame
    }

    /// Begin a new frame
    pub fn begin_frame(&mut self) {
        // Reset for new frame
        self.current_scopes = [const { None }; 256];
        self.current_scope_count = 0;
        self.next_query = 0;
        self.scope_stack = [-1; 32];
        self.stack_depth = 0;
    }

    /// End the current frame
    pub fn end_frame(&mut self) {
        self.current_frame += 1;
        self.buffer_index = (self.buffer_index + 1) % self.config.frame_buffer_count;
    }

    /// Begin a profiling scope
    pub fn begin_scope(&mut self, name: &str, color: [f32; 4]) -> u32 {
        if self.current_scope_count >= Self::MAX_SCOPES as u32 {
            return u32::MAX;
        }

        let scope_index = self.current_scope_count;
        let start_query = self.next_query;
        self.next_query += 1;

        let parent = if self.stack_depth > 0 {
            self.scope_stack[self.stack_depth as usize - 1]
        } else {
            -1
        };

        self.current_scopes[scope_index as usize] = Some(ProfileScope {
            name: ScopeName::new(name),
            start_query,
            end_query: u32::MAX, // Set on end
            parent,
            depth: self.stack_depth,
            color,
        });

        // Push to stack
        if self.stack_depth < Self::MAX_STACK_DEPTH as u32 {
            self.scope_stack[self.stack_depth as usize] = scope_index as i32;
            self.stack_depth += 1;
        }

        self.current_scope_count += 1;
        scope_index
    }

    /// End a profiling scope
    pub fn end_scope(&mut self, scope_index: u32) {
        if let Some(ref mut scope) = self
            .current_scopes
            .get_mut(scope_index as usize)
            .and_then(|s| s.as_mut())
        {
            scope.end_query = self.next_query;
            self.next_query += 1;
        }

        // Pop from stack
        if self.stack_depth > 0 {
            self.stack_depth -= 1;
        }
    }

    /// Get the query pool for the current frame
    pub fn current_query_pool(&self) -> QueryPool {
        self.query_pools[self.buffer_index as usize]
    }

    /// Check if results are available for a frame
    pub fn has_results(&self, frame: u64) -> bool {
        let buffer_idx = (frame % self.config.frame_buffer_count as u64) as usize;
        self.pending_results[buffer_idx]
            .as_ref()
            .map(|r| r.frame_number == frame)
            .unwrap_or(false)
    }

    /// Calculate GPU time from timestamps
    pub fn calculate_time_ms(&self, start: u64, end: u64) -> f64 {
        let delta = end.saturating_sub(start);
        (delta as f64) * (self.config.timestamp_period_ns as f64) / 1_000_000.0
    }
}

/// RAII scope guard for profiling
pub struct ProfileScopeGuard<'a> {
    profiler: &'a mut GpuProfiler,
    scope_index: u32,
}

impl<'a> ProfileScopeGuard<'a> {
    /// Create a new scope guard
    pub fn new(profiler: &'a mut GpuProfiler, name: &str, color: [f32; 4]) -> Self {
        let scope_index = profiler.begin_scope(name, color);
        Self {
            profiler,
            scope_index,
        }
    }
}

impl<'a> Drop for ProfileScopeGuard<'a> {
    fn drop(&mut self) {
        self.profiler.end_scope(self.scope_index);
    }
}

/// Macro for easy profiling scopes
#[macro_export]
macro_rules! profile_scope {
    ($profiler:expr, $name:expr) => {
        let _scope =
            $crate::profiling::ProfileScopeGuard::new($profiler, $name, [1.0, 1.0, 1.0, 1.0]);
    };
    ($profiler:expr, $name:expr, $color:expr) => {
        let _scope = $crate::profiling::ProfileScopeGuard::new($profiler, $name, $color);
    };
}

// ============================================================================
// Command Buffer Extensions
// ============================================================================

/// Extension trait for profiling commands on command buffers
pub trait ProfilingCommands {
    /// Write a timestamp
    fn write_timestamp(&mut self, pool: QueryPool, query: u32, stage: PipelineStage);

    /// Reset query range
    fn reset_query_pool(&mut self, pool: QueryPool, first_query: u32, query_count: u32);

    /// Begin a query
    fn begin_query(&mut self, pool: QueryPool, query: u32, flags: QueryControlFlags);

    /// End a query
    fn end_query(&mut self, pool: QueryPool, query: u32);

    /// Copy query results to buffer
    fn copy_query_pool_results(
        &mut self,
        pool: QueryPool,
        first_query: u32,
        query_count: u32,
        dst_buffer: u64,
        dst_offset: u64,
        stride: u64,
        flags: QueryResultFlags,
    );
}

/// Pipeline stage for timestamps
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PipelineStage {
    TopOfPipe,
    DrawIndirect,
    VertexInput,
    VertexShader,
    TessellationControl,
    TessellationEvaluation,
    GeometryShader,
    FragmentShader,
    EarlyFragmentTests,
    LateFragmentTests,
    ColorAttachmentOutput,
    ComputeShader,
    Transfer,
    BottomOfPipe,
    Host,
    AllGraphics,
    AllCommands,
    TaskShader,
    MeshShader,
    RayTracingShader,
    AccelerationStructureBuild,
}

/// Query control flags
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueryControlFlags(u32);

impl QueryControlFlags {
    pub const NONE: Self = Self(0);
    /// Precise occlusion query
    pub const PRECISE: Self = Self(1 << 0);
}

// ============================================================================
// GPU Timing Utilities
// ============================================================================

/// Simple GPU timer for single measurements
#[derive(Debug)]
pub struct GpuTimer {
    query_pool: QueryPool,
    start_query: u32,
    end_query: u32,
    timestamp_period_ns: f32,
}

impl GpuTimer {
    /// Create a new GPU timer
    pub fn new(query_pool: QueryPool, start: u32, end: u32, period_ns: f32) -> Self {
        Self {
            query_pool,
            start_query: start,
            end_query: end,
            timestamp_period_ns: period_ns,
        }
    }

    /// Get the query pool
    pub fn query_pool(&self) -> QueryPool {
        self.query_pool
    }

    /// Get start query index
    pub fn start_query(&self) -> u32 {
        self.start_query
    }

    /// Get end query index
    pub fn end_query(&self) -> u32 {
        self.end_query
    }

    /// Calculate elapsed time from raw timestamps
    pub fn elapsed_ms(&self, start_timestamp: u64, end_timestamp: u64) -> f64 {
        let delta = end_timestamp.saturating_sub(start_timestamp);
        (delta as f64) * (self.timestamp_period_ns as f64) / 1_000_000.0
    }
}

// ============================================================================
// Bandwidth/Throughput Tracking
// ============================================================================

/// GPU bandwidth statistics
#[derive(Clone, Copy, Debug, Default)]
pub struct BandwidthStats {
    /// Bytes read from global memory
    pub bytes_read: u64,
    /// Bytes written to global memory
    pub bytes_written: u64,
    /// L2 cache hit rate (0.0-1.0)
    pub l2_hit_rate: f32,
    /// Texture cache hit rate (0.0-1.0)
    pub texture_hit_rate: f32,
    /// Measured time in nanoseconds
    pub time_ns: u64,
}

impl BandwidthStats {
    /// Calculate read bandwidth in GB/s
    pub fn read_bandwidth_gbps(&self) -> f64 {
        if self.time_ns == 0 {
            return 0.0;
        }
        (self.bytes_read as f64) / (self.time_ns as f64)
    }

    /// Calculate write bandwidth in GB/s
    pub fn write_bandwidth_gbps(&self) -> f64 {
        if self.time_ns == 0 {
            return 0.0;
        }
        (self.bytes_written as f64) / (self.time_ns as f64)
    }

    /// Calculate total bandwidth in GB/s
    pub fn total_bandwidth_gbps(&self) -> f64 {
        self.read_bandwidth_gbps() + self.write_bandwidth_gbps()
    }
}

// ============================================================================
// Occupancy Tracking
// ============================================================================

/// Shader occupancy statistics
#[derive(Clone, Copy, Debug, Default)]
pub struct OccupancyStats {
    /// Theoretical maximum warps per SM
    pub max_warps_per_sm: u32,
    /// Achieved warps per SM
    pub achieved_warps_per_sm: f32,
    /// Theoretical occupancy (0.0-1.0)
    pub theoretical_occupancy: f32,
    /// Achieved occupancy (0.0-1.0)
    pub achieved_occupancy: f32,
    /// Limiter (what's limiting occupancy)
    pub limiter: OccupancyLimiter,
}

/// What limits shader occupancy
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OccupancyLimiter {
    #[default]
    None,
    Registers,
    SharedMemory,
    ThreadBlockSize,
    MaxWarps,
}

impl OccupancyLimiter {
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Registers => "Registers",
            Self::SharedMemory => "Shared Memory",
            Self::ThreadBlockSize => "Thread Block Size",
            Self::MaxWarps => "Max Warps",
        }
    }
}
