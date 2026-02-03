//! GPU Profiler Types for Lumina
//!
//! This module provides GPU profiling infrastructure including timestamp queries,
//! pipeline statistics, performance counters, and profiling scopes.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Profiler Handle
// ============================================================================

/// Profiler handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ProfilerHandle(pub u64);

impl ProfilerHandle {
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

impl Default for ProfilerHandle {
    fn default() -> Self {
        Self::NULL
    }
}

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
#[derive(Clone, Debug)]
pub struct QueryPoolCreateInfo {
    /// Query type
    pub query_type: QueryType,
    /// Query count
    pub query_count: u32,
    /// Pipeline statistics (for pipeline statistics queries)
    pub pipeline_statistics: PipelineStatisticsFlags,
    /// Debug name
    pub debug_name: Option<String>,
}

impl QueryPoolCreateInfo {
    /// Creates timestamp query pool
    pub fn timestamp(count: u32) -> Self {
        Self {
            query_type: QueryType::Timestamp,
            query_count: count,
            pipeline_statistics: PipelineStatisticsFlags::NONE,
            debug_name: None,
        }
    }

    /// Creates occlusion query pool
    pub fn occlusion(count: u32) -> Self {
        Self {
            query_type: QueryType::Occlusion,
            query_count: count,
            pipeline_statistics: PipelineStatisticsFlags::NONE,
            debug_name: None,
        }
    }

    /// Creates pipeline statistics query pool
    pub fn pipeline_statistics(count: u32, statistics: PipelineStatisticsFlags) -> Self {
        Self {
            query_type: QueryType::PipelineStatistics,
            query_count: count,
            pipeline_statistics: statistics,
            debug_name: None,
        }
    }

    /// Creates performance query pool
    pub fn performance(count: u32) -> Self {
        Self {
            query_type: QueryType::Performance,
            query_count: count,
            pipeline_statistics: PipelineStatisticsFlags::NONE,
            debug_name: None,
        }
    }

    /// With debug name
    pub fn with_name(mut self, name: &str) -> Self {
        self.debug_name = Some(String::from(name));
        self
    }
}

impl Default for QueryPoolCreateInfo {
    fn default() -> Self {
        Self::timestamp(64)
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
    Occlusion          = 0,
    /// Pipeline statistics
    PipelineStatistics = 1,
    /// Timestamp
    #[default]
    Timestamp          = 2,
    /// Performance query
    Performance        = 1000116000,
    /// Acceleration structure compacted size
    AccelerationStructureCompactedSize = 1000150000,
    /// Acceleration structure serialization size
    AccelerationStructureSerializationSize = 1000150001,
    /// Acceleration structure size
    AccelerationStructureSize = 1000386000,
}

// ============================================================================
// Pipeline Statistics Flags
// ============================================================================

/// Pipeline statistics flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PipelineStatisticsFlags(pub u32);

impl PipelineStatisticsFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Input assembly vertices
    pub const INPUT_ASSEMBLY_VERTICES: Self = Self(0x00000001);
    /// Input assembly primitives
    pub const INPUT_ASSEMBLY_PRIMITIVES: Self = Self(0x00000002);
    /// Vertex shader invocations
    pub const VERTEX_SHADER_INVOCATIONS: Self = Self(0x00000004);
    /// Geometry shader invocations
    pub const GEOMETRY_SHADER_INVOCATIONS: Self = Self(0x00000008);
    /// Geometry shader primitives
    pub const GEOMETRY_SHADER_PRIMITIVES: Self = Self(0x00000010);
    /// Clipping invocations
    pub const CLIPPING_INVOCATIONS: Self = Self(0x00000020);
    /// Clipping primitives
    pub const CLIPPING_PRIMITIVES: Self = Self(0x00000040);
    /// Fragment shader invocations
    pub const FRAGMENT_SHADER_INVOCATIONS: Self = Self(0x00000080);
    /// Tessellation control shader patches
    pub const TESSELLATION_CONTROL_SHADER_PATCHES: Self = Self(0x00000100);
    /// Tessellation evaluation shader invocations
    pub const TESSELLATION_EVALUATION_SHADER_INVOCATIONS: Self = Self(0x00000200);
    /// Compute shader invocations
    pub const COMPUTE_SHADER_INVOCATIONS: Self = Self(0x00000400);
    /// Task shader invocations
    pub const TASK_SHADER_INVOCATIONS: Self = Self(0x00000800);
    /// Mesh shader invocations
    pub const MESH_SHADER_INVOCATIONS: Self = Self(0x00001000);

    /// All graphics statistics
    pub const ALL_GRAPHICS: Self = Self(0x000003FF);
    /// All statistics
    pub const ALL: Self = Self(0x00001FFF);

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

    /// Count of enabled statistics
    pub const fn count(&self) -> u32 {
        self.0.count_ones()
    }
}

impl core::ops::BitOr for PipelineStatisticsFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

// ============================================================================
// Query Result
// ============================================================================

/// Query result flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct QueryResultFlags(pub u32);

impl QueryResultFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// 64-bit results
    pub const RESULT_64: Self = Self(0x00000001);
    /// Wait for results
    pub const WAIT: Self = Self(0x00000002);
    /// With availability
    pub const WITH_AVAILABILITY: Self = Self(0x00000004);
    /// Partial results
    pub const PARTIAL: Self = Self(0x00000008);
    /// With status
    pub const WITH_STATUS: Self = Self(0x00000010);

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

impl core::ops::BitOr for QueryResultFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

// ============================================================================
// Timestamp Query Results
// ============================================================================

/// Timestamp result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TimestampResult {
    /// Timestamp value in GPU cycles
    pub timestamp: u64,
    /// Availability (if queried)
    pub available: bool,
}

impl TimestampResult {
    /// Convert to nanoseconds given timestamp period
    #[inline]
    pub fn to_nanoseconds(&self, timestamp_period: f32) -> f64 {
        self.timestamp as f64 * timestamp_period as f64
    }

    /// Convert to microseconds
    #[inline]
    pub fn to_microseconds(&self, timestamp_period: f32) -> f64 {
        self.to_nanoseconds(timestamp_period) / 1000.0
    }

    /// Convert to milliseconds
    #[inline]
    pub fn to_milliseconds(&self, timestamp_period: f32) -> f64 {
        self.to_nanoseconds(timestamp_period) / 1_000_000.0
    }
}

/// GPU timing result (pair of timestamps)
#[derive(Clone, Copy, Debug, Default)]
pub struct GpuTimingResult {
    /// Start timestamp
    pub start: u64,
    /// End timestamp
    pub end: u64,
    /// Timestamp period (ns per tick)
    pub timestamp_period: f32,
}

impl GpuTimingResult {
    /// Duration in nanoseconds
    #[inline]
    pub fn duration_ns(&self) -> f64 {
        (self.end.saturating_sub(self.start)) as f64 * self.timestamp_period as f64
    }

    /// Duration in microseconds
    #[inline]
    pub fn duration_us(&self) -> f64 {
        self.duration_ns() / 1000.0
    }

    /// Duration in milliseconds
    #[inline]
    pub fn duration_ms(&self) -> f64 {
        self.duration_ns() / 1_000_000.0
    }
}

// ============================================================================
// Pipeline Statistics Results
// ============================================================================

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
    /// Vertex shader to fragment shader ratio
    #[inline]
    pub fn vs_to_fs_ratio(&self) -> f64 {
        if self.fragment_shader_invocations == 0 {
            0.0
        } else {
            self.vertex_shader_invocations as f64 / self.fragment_shader_invocations as f64
        }
    }

    /// Overdraw ratio (approximate)
    #[inline]
    pub fn overdraw_ratio(&self, pixel_count: u64) -> f64 {
        if pixel_count == 0 {
            0.0
        } else {
            self.fragment_shader_invocations as f64 / pixel_count as f64
        }
    }

    /// Clipping efficiency
    #[inline]
    pub fn clipping_efficiency(&self) -> f64 {
        if self.clipping_invocations == 0 {
            1.0
        } else {
            self.clipping_primitives as f64 / self.clipping_invocations as f64
        }
    }
}

// ============================================================================
// Occlusion Query Results
// ============================================================================

/// Occlusion query result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct OcclusionResult {
    /// Sample count that passed depth test
    pub samples_passed: u64,
    /// Is available
    pub available: bool,
}

impl OcclusionResult {
    /// Is visible (any samples passed)
    #[inline]
    pub fn is_visible(&self) -> bool {
        self.samples_passed > 0
    }
}

// ============================================================================
// Profiler Scope
// ============================================================================

/// Profiler scope for hierarchical profiling
#[derive(Clone, Debug)]
pub struct ProfilerScope {
    /// Scope name
    pub name: String,
    /// Start query index
    pub start_query: u32,
    /// End query index
    pub end_query: u32,
    /// Parent scope index (if any)
    pub parent: Option<u32>,
    /// Child scope indices
    pub children: Vec<u32>,
    /// Color for visualization
    pub color: [f32; 4],
    /// Depth level
    pub depth: u32,
}

impl ProfilerScope {
    /// Creates new scope
    pub fn new(name: &str, start_query: u32) -> Self {
        Self {
            name: String::from(name),
            start_query,
            end_query: 0,
            parent: None,
            children: Vec::new(),
            color: [1.0, 1.0, 1.0, 1.0],
            depth: 0,
        }
    }

    /// With color
    pub fn with_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.color = [r, g, b, 1.0];
        self
    }

    /// With parent
    pub fn with_parent(mut self, parent: u32) -> Self {
        self.parent = Some(parent);
        self
    }
}

// ============================================================================
// GPU Frame Profiler
// ============================================================================

/// GPU frame profiler
#[derive(Debug, Default)]
pub struct GpuFrameProfiler {
    /// Scopes
    scopes: Vec<ProfilerScope>,
    /// Current scope stack
    scope_stack: Vec<u32>,
    /// Next query index
    next_query: u32,
    /// Frame index
    frame_index: u64,
    /// Timestamp period
    timestamp_period: f32,
    /// Is recording
    is_recording: bool,
    /// Max scopes per frame
    max_scopes: u32,
}

impl GpuFrameProfiler {
    /// Creates new profiler
    pub fn new(timestamp_period: f32) -> Self {
        Self {
            scopes: Vec::new(),
            scope_stack: Vec::new(),
            next_query: 0,
            frame_index: 0,
            timestamp_period,
            is_recording: false,
            max_scopes: 256,
        }
    }

    /// Begin frame
    pub fn begin_frame(&mut self, frame_index: u64) {
        self.scopes.clear();
        self.scope_stack.clear();
        self.next_query = 0;
        self.frame_index = frame_index;
        self.is_recording = true;
    }

    /// End frame
    pub fn end_frame(&mut self) {
        // Close any open scopes
        while !self.scope_stack.is_empty() {
            self.end_scope();
        }
        self.is_recording = false;
    }

    /// Begin scope
    pub fn begin_scope(&mut self, name: &str) -> u32 {
        if !self.is_recording || self.scopes.len() >= self.max_scopes as usize {
            return 0;
        }

        let scope_index = self.scopes.len() as u32;
        let query_index = self.next_query;
        self.next_query += 1;

        let mut scope = ProfilerScope::new(name, query_index);
        scope.depth = self.scope_stack.len() as u32;

        if let Some(&parent_index) = self.scope_stack.last() {
            scope.parent = Some(parent_index);
            self.scopes[parent_index as usize]
                .children
                .push(scope_index);
        }

        self.scopes.push(scope);
        self.scope_stack.push(scope_index);

        query_index
    }

    /// End scope
    pub fn end_scope(&mut self) -> u32 {
        if !self.is_recording {
            return 0;
        }

        if let Some(scope_index) = self.scope_stack.pop() {
            let query_index = self.next_query;
            self.next_query += 1;
            self.scopes[scope_index as usize].end_query = query_index;
            query_index
        } else {
            0
        }
    }

    /// Get scope count
    pub fn scope_count(&self) -> usize {
        self.scopes.len()
    }

    /// Get query count
    pub fn query_count(&self) -> u32 {
        self.next_query
    }

    /// Get scopes
    pub fn scopes(&self) -> &[ProfilerScope] {
        &self.scopes
    }

    /// Get frame index
    pub fn frame_index(&self) -> u64 {
        self.frame_index
    }
}

// ============================================================================
// Profiler Zone
// ============================================================================

/// Profiler zone for named regions
#[derive(Clone, Debug)]
pub struct ProfilerZone {
    /// Zone name
    pub name: String,
    /// Zone color
    pub color: ZoneColor,
    /// Category
    pub category: ZoneCategory,
    /// Is active
    pub active: bool,
}

impl ProfilerZone {
    /// Creates new zone
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            color: ZoneColor::Default,
            category: ZoneCategory::General,
            active: true,
        }
    }

    /// With color
    pub fn with_color(mut self, color: ZoneColor) -> Self {
        self.color = color;
        self
    }

    /// With category
    pub fn with_category(mut self, category: ZoneCategory) -> Self {
        self.category = category;
        self
    }

    /// Rendering zone
    pub fn rendering(name: &str) -> Self {
        Self::new(name)
            .with_color(ZoneColor::Rendering)
            .with_category(ZoneCategory::Rendering)
    }

    /// Compute zone
    pub fn compute(name: &str) -> Self {
        Self::new(name)
            .with_color(ZoneColor::Compute)
            .with_category(ZoneCategory::Compute)
    }

    /// Transfer zone
    pub fn transfer(name: &str) -> Self {
        Self::new(name)
            .with_color(ZoneColor::Transfer)
            .with_category(ZoneCategory::Transfer)
    }
}

/// Zone color
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ZoneColor {
    /// Default
    #[default]
    Default   = 0,
    /// Rendering
    Rendering = 1,
    /// Compute
    Compute   = 2,
    /// Transfer
    Transfer  = 3,
    /// UI
    UI        = 4,
    /// Physics
    Physics   = 5,
    /// Audio
    Audio     = 6,
    /// Network
    Network   = 7,
    /// AI
    AI        = 8,
    /// Custom
    Custom    = 255,
}

impl ZoneColor {
    /// To RGBA
    pub const fn to_rgba(&self) -> [f32; 4] {
        match self {
            Self::Default => [0.5, 0.5, 0.5, 1.0],
            Self::Rendering => [0.2, 0.6, 0.9, 1.0],
            Self::Compute => [0.9, 0.5, 0.2, 1.0],
            Self::Transfer => [0.5, 0.9, 0.2, 1.0],
            Self::UI => [0.9, 0.2, 0.6, 1.0],
            Self::Physics => [0.2, 0.9, 0.6, 1.0],
            Self::Audio => [0.6, 0.2, 0.9, 1.0],
            Self::Network => [0.9, 0.9, 0.2, 1.0],
            Self::AI => [0.2, 0.2, 0.9, 1.0],
            Self::Custom => [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// Zone category
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ZoneCategory {
    /// General
    #[default]
    General         = 0,
    /// Rendering
    Rendering       = 1,
    /// Compute
    Compute         = 2,
    /// Transfer
    Transfer        = 3,
    /// Memory
    Memory          = 4,
    /// Synchronization
    Synchronization = 5,
}

// ============================================================================
// Performance Counter
// ============================================================================

/// Performance counter type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum PerformanceCounterType {
    /// GPU time
    GpuTime             = 0,
    /// GPU cycles
    GpuCycles           = 1,
    /// Shader invocations
    ShaderInvocations   = 2,
    /// Memory bandwidth
    MemoryBandwidth     = 3,
    /// Cache hit rate
    CacheHitRate        = 4,
    /// Occupancy
    Occupancy           = 5,
    /// Warp efficiency
    WarpEfficiency      = 6,
    /// Register usage
    RegisterUsage       = 7,
    /// Shared memory usage
    SharedMemoryUsage   = 8,
    /// L1 cache usage
    L1CacheUsage        = 9,
    /// L2 cache usage
    L2CacheUsage        = 10,
    /// Memory throughput
    MemoryThroughput    = 11,
    /// Texture cache hit rate
    TextureCacheHitRate = 12,
    /// Compute utilization
    ComputeUtilization  = 13,
    /// Graphics utilization
    GraphicsUtilization = 14,
    /// Memory utilization
    MemoryUtilization   = 15,
}

/// Performance counter value
#[derive(Clone, Copy, Debug)]
pub struct PerformanceCounterValue {
    /// Counter type
    pub counter_type: PerformanceCounterType,
    /// Value
    pub value: f64,
    /// Unit
    pub unit: PerformanceCounterUnit,
}

impl PerformanceCounterValue {
    /// Creates new counter value
    pub const fn new(
        counter_type: PerformanceCounterType,
        value: f64,
        unit: PerformanceCounterUnit,
    ) -> Self {
        Self {
            counter_type,
            value,
            unit,
        }
    }
}

/// Performance counter unit
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PerformanceCounterUnit {
    /// Generic value
    #[default]
    Generic        = 0,
    /// Nanoseconds
    Nanoseconds    = 1,
    /// Microseconds
    Microseconds   = 2,
    /// Milliseconds
    Milliseconds   = 3,
    /// Cycles
    Cycles         = 4,
    /// Bytes
    Bytes          = 5,
    /// Kilobytes
    Kilobytes      = 6,
    /// Megabytes
    Megabytes      = 7,
    /// Gigabytes
    Gigabytes      = 8,
    /// Percentage
    Percentage     = 9,
    /// Count
    Count          = 10,
    /// BytesPerSecond
    BytesPerSecond = 11,
    /// Hertz
    Hertz          = 12,
}

// ============================================================================
// Frame Statistics
// ============================================================================

/// Frame statistics
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameStatistics {
    /// Frame index
    pub frame_index: u64,
    /// GPU time in milliseconds
    pub gpu_time_ms: f64,
    /// CPU time in milliseconds
    pub cpu_time_ms: f64,
    /// Draw call count
    pub draw_calls: u32,
    /// Dispatch count
    pub dispatches: u32,
    /// Triangle count
    pub triangles: u64,
    /// Vertex count
    pub vertices: u64,
    /// State changes
    pub state_changes: u32,
    /// Pipeline binds
    pub pipeline_binds: u32,
    /// Descriptor set binds
    pub descriptor_set_binds: u32,
    /// Buffer binds
    pub buffer_binds: u32,
    /// Memory allocated
    pub memory_allocated: u64,
    /// Memory freed
    pub memory_freed: u64,
    /// Upload bytes
    pub upload_bytes: u64,
    /// Render targets
    pub render_targets: u32,
}

impl FrameStatistics {
    /// Creates new frame statistics
    pub const fn new(frame_index: u64) -> Self {
        Self {
            frame_index,
            gpu_time_ms: 0.0,
            cpu_time_ms: 0.0,
            draw_calls: 0,
            dispatches: 0,
            triangles: 0,
            vertices: 0,
            state_changes: 0,
            pipeline_binds: 0,
            descriptor_set_binds: 0,
            buffer_binds: 0,
            memory_allocated: 0,
            memory_freed: 0,
            upload_bytes: 0,
            render_targets: 0,
        }
    }

    /// Frame time (max of GPU/CPU)
    #[inline]
    pub fn frame_time_ms(&self) -> f64 {
        self.gpu_time_ms.max(self.cpu_time_ms)
    }

    /// Estimated FPS
    #[inline]
    pub fn estimated_fps(&self) -> f64 {
        let frame_time = self.frame_time_ms();
        if frame_time > 0.0 {
            1000.0 / frame_time
        } else {
            0.0
        }
    }

    /// Is GPU bound
    #[inline]
    pub fn is_gpu_bound(&self) -> bool {
        self.gpu_time_ms > self.cpu_time_ms
    }

    /// Is CPU bound
    #[inline]
    pub fn is_cpu_bound(&self) -> bool {
        self.cpu_time_ms > self.gpu_time_ms
    }
}

// ============================================================================
// Rolling Average
// ============================================================================

/// Rolling average for statistics
#[derive(Clone, Debug)]
pub struct RollingAverage {
    /// Values
    values: Vec<f64>,
    /// Current index
    index: usize,
    /// Count
    count: usize,
    /// Sum
    sum: f64,
}

impl RollingAverage {
    /// Creates new rolling average
    pub fn new(size: usize) -> Self {
        Self {
            values: alloc::vec![0.0; size],
            index: 0,
            count: 0,
            sum: 0.0,
        }
    }

    /// Add value
    pub fn add(&mut self, value: f64) {
        let size = self.values.len();

        // Subtract old value from sum
        if self.count >= size {
            self.sum -= self.values[self.index];
        }

        // Add new value
        self.values[self.index] = value;
        self.sum += value;

        // Advance
        self.index = (self.index + 1) % size;
        if self.count < size {
            self.count += 1;
        }
    }

    /// Average
    #[inline]
    pub fn average(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }

    /// Min value
    pub fn min(&self) -> f64 {
        self.values
            .iter()
            .take(self.count)
            .copied()
            .fold(f64::MAX, f64::min)
    }

    /// Max value
    pub fn max(&self) -> f64 {
        self.values
            .iter()
            .take(self.count)
            .copied()
            .fold(f64::MIN, f64::max)
    }

    /// Latest value
    pub fn latest(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            let idx = if self.index == 0 {
                self.values.len() - 1
            } else {
                self.index - 1
            };
            self.values[idx]
        }
    }

    /// Reset
    pub fn reset(&mut self) {
        self.values.fill(0.0);
        self.index = 0;
        self.count = 0;
        self.sum = 0.0;
    }
}

impl Default for RollingAverage {
    fn default() -> Self {
        Self::new(60)
    }
}

// ============================================================================
// Profiler Configuration
// ============================================================================

/// Profiler configuration
#[derive(Clone, Debug)]
pub struct ProfilerConfig {
    /// Enable GPU profiling
    pub enable_gpu_profiling: bool,
    /// Enable CPU profiling
    pub enable_cpu_profiling: bool,
    /// Enable pipeline statistics
    pub enable_pipeline_statistics: bool,
    /// Enable memory tracking
    pub enable_memory_tracking: bool,
    /// Query pool size
    pub query_pool_size: u32,
    /// Max scopes per frame
    pub max_scopes_per_frame: u32,
    /// History size
    pub history_size: u32,
    /// Sample rate (frames between samples)
    pub sample_rate: u32,
}

impl ProfilerConfig {
    /// Minimal profiling
    pub const MINIMAL: Self = Self {
        enable_gpu_profiling: true,
        enable_cpu_profiling: false,
        enable_pipeline_statistics: false,
        enable_memory_tracking: false,
        query_pool_size: 64,
        max_scopes_per_frame: 32,
        history_size: 60,
        sample_rate: 1,
    };

    /// Standard profiling
    pub const STANDARD: Self = Self {
        enable_gpu_profiling: true,
        enable_cpu_profiling: true,
        enable_pipeline_statistics: false,
        enable_memory_tracking: true,
        query_pool_size: 256,
        max_scopes_per_frame: 128,
        history_size: 120,
        sample_rate: 1,
    };

    /// Full profiling
    pub const FULL: Self = Self {
        enable_gpu_profiling: true,
        enable_cpu_profiling: true,
        enable_pipeline_statistics: true,
        enable_memory_tracking: true,
        query_pool_size: 512,
        max_scopes_per_frame: 256,
        history_size: 300,
        sample_rate: 1,
    };
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self::STANDARD
    }
}
