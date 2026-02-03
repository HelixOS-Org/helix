//! Pipeline Statistics and Performance Metrics for Lumina
//!
//! This module provides comprehensive pipeline statistics collection
//! and performance analysis infrastructure.

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Pipeline Statistics
// ============================================================================

/// Pipeline statistics query pool handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineStatisticsQueryHandle(pub u64);

impl PipelineStatisticsQueryHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates a new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Checks if the handle is valid
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

/// Pipeline statistics query configuration
#[derive(Clone, Debug)]
#[repr(C)]
pub struct PipelineStatisticsConfig {
    /// Query count
    pub query_count: u32,
    /// Statistics flags
    pub statistics: PipelineStatisticsFlags,
    /// Enable precise occlusion queries
    pub precise_occlusion: bool,
}

impl PipelineStatisticsConfig {
    /// Creates a new configuration
    #[inline]
    pub const fn new(query_count: u32, statistics: PipelineStatisticsFlags) -> Self {
        Self {
            query_count,
            statistics,
            precise_occlusion: false,
        }
    }

    /// All graphics statistics
    #[inline]
    pub const fn all_graphics(query_count: u32) -> Self {
        Self {
            query_count,
            statistics: PipelineStatisticsFlags::ALL_GRAPHICS,
            precise_occlusion: true,
        }
    }

    /// All compute statistics
    #[inline]
    pub const fn all_compute(query_count: u32) -> Self {
        Self {
            query_count,
            statistics: PipelineStatisticsFlags::COMPUTE_SHADER_INVOCATIONS,
            precise_occlusion: false,
        }
    }

    /// Full statistics (all flags)
    #[inline]
    pub const fn full(query_count: u32) -> Self {
        Self {
            query_count,
            statistics: PipelineStatisticsFlags::ALL,
            precise_occlusion: true,
        }
    }

    /// With precise occlusion
    #[inline]
    pub const fn with_precise_occlusion(mut self) -> Self {
        self.precise_occlusion = true;
        self
    }
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

    /// All graphics pipeline statistics
    pub const ALL_GRAPHICS: Self = Self(
        Self::INPUT_ASSEMBLY_VERTICES.0
            | Self::INPUT_ASSEMBLY_PRIMITIVES.0
            | Self::VERTEX_SHADER_INVOCATIONS.0
            | Self::GEOMETRY_SHADER_INVOCATIONS.0
            | Self::GEOMETRY_SHADER_PRIMITIVES.0
            | Self::CLIPPING_INVOCATIONS.0
            | Self::CLIPPING_PRIMITIVES.0
            | Self::FRAGMENT_SHADER_INVOCATIONS.0
            | Self::TESSELLATION_CONTROL_SHADER_PATCHES.0
            | Self::TESSELLATION_EVALUATION_SHADER_INVOCATIONS.0,
    );

    /// All mesh shader statistics
    pub const ALL_MESH: Self =
        Self(Self::TASK_SHADER_INVOCATIONS.0 | Self::MESH_SHADER_INVOCATIONS.0);

    /// All statistics
    pub const ALL: Self = Self(0x1FFF);

    /// Checks if flag is set
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Combines flags
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Counts enabled statistics
    #[inline]
    pub const fn count(&self) -> u32 {
        self.0.count_ones()
    }
}

// ============================================================================
// Pipeline Statistics Results
// ============================================================================

/// Pipeline statistics query result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PipelineStatisticsResult {
    /// Input assembly vertices processed
    pub input_assembly_vertices: u64,
    /// Input assembly primitives processed
    pub input_assembly_primitives: u64,
    /// Vertex shader invocations
    pub vertex_shader_invocations: u64,
    /// Geometry shader invocations
    pub geometry_shader_invocations: u64,
    /// Primitives output by geometry shader
    pub geometry_shader_primitives: u64,
    /// Primitives processed by clipping
    pub clipping_invocations: u64,
    /// Primitives output by clipping
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
    /// Size of the result structure
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Creates a new empty result
    #[inline]
    pub const fn new() -> Self {
        Self {
            input_assembly_vertices: 0,
            input_assembly_primitives: 0,
            vertex_shader_invocations: 0,
            geometry_shader_invocations: 0,
            geometry_shader_primitives: 0,
            clipping_invocations: 0,
            clipping_primitives: 0,
            fragment_shader_invocations: 0,
            tessellation_control_shader_patches: 0,
            tessellation_evaluation_shader_invocations: 0,
            compute_shader_invocations: 0,
            task_shader_invocations: 0,
            mesh_shader_invocations: 0,
        }
    }

    /// Average vertices per primitive
    #[inline]
    pub fn avg_vertices_per_primitive(&self) -> f64 {
        if self.input_assembly_primitives == 0 {
            0.0
        } else {
            self.input_assembly_vertices as f64 / self.input_assembly_primitives as f64
        }
    }

    /// Geometry amplification ratio
    #[inline]
    pub fn geometry_amplification(&self) -> f64 {
        if self.geometry_shader_invocations == 0 {
            1.0
        } else {
            self.geometry_shader_primitives as f64 / self.geometry_shader_invocations as f64
        }
    }

    /// Clipping efficiency (ratio of primitives surviving clipping)
    #[inline]
    pub fn clipping_efficiency(&self) -> f64 {
        if self.clipping_invocations == 0 {
            1.0
        } else {
            self.clipping_primitives as f64 / self.clipping_invocations as f64
        }
    }

    /// Overdraw estimate (fragments per input primitive)
    #[inline]
    pub fn overdraw_estimate(&self) -> f64 {
        if self.clipping_primitives == 0 {
            0.0
        } else {
            self.fragment_shader_invocations as f64 / self.clipping_primitives as f64
        }
    }

    /// Total shader invocations
    #[inline]
    pub fn total_shader_invocations(&self) -> u64 {
        self.vertex_shader_invocations
            + self.geometry_shader_invocations
            + self.fragment_shader_invocations
            + self.tessellation_control_shader_patches
            + self.tessellation_evaluation_shader_invocations
            + self.compute_shader_invocations
            + self.task_shader_invocations
            + self.mesh_shader_invocations
    }

    /// Accumulates another result
    #[inline]
    pub fn accumulate(&mut self, other: &Self) {
        self.input_assembly_vertices += other.input_assembly_vertices;
        self.input_assembly_primitives += other.input_assembly_primitives;
        self.vertex_shader_invocations += other.vertex_shader_invocations;
        self.geometry_shader_invocations += other.geometry_shader_invocations;
        self.geometry_shader_primitives += other.geometry_shader_primitives;
        self.clipping_invocations += other.clipping_invocations;
        self.clipping_primitives += other.clipping_primitives;
        self.fragment_shader_invocations += other.fragment_shader_invocations;
        self.tessellation_control_shader_patches += other.tessellation_control_shader_patches;
        self.tessellation_evaluation_shader_invocations +=
            other.tessellation_evaluation_shader_invocations;
        self.compute_shader_invocations += other.compute_shader_invocations;
        self.task_shader_invocations += other.task_shader_invocations;
        self.mesh_shader_invocations += other.mesh_shader_invocations;
    }

    /// Clears all statistics
    #[inline]
    pub fn clear(&mut self) {
        *self = Self::new();
    }
}

// ============================================================================
// GPU Timestamp
// ============================================================================

/// Timestamp query handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TimestampQueryHandle(pub u64);

impl TimestampQueryHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates a new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Checks if the handle is valid
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

/// Timestamp query pool configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TimestampQueryConfig {
    /// Number of timestamp slots
    pub query_count: u32,
    /// Pipeline stage for timestamps
    pub pipeline_stage: TimestampPipelineStage,
}

impl TimestampQueryConfig {
    /// Creates a new configuration
    #[inline]
    pub const fn new(query_count: u32) -> Self {
        Self {
            query_count,
            pipeline_stage: TimestampPipelineStage::AllCommands,
        }
    }

    /// With specific pipeline stage
    #[inline]
    pub const fn with_stage(mut self, stage: TimestampPipelineStage) -> Self {
        self.pipeline_stage = stage;
        self
    }
}

/// Pipeline stage for timestamp writes
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TimestampPipelineStage {
    /// Top of pipe
    TopOfPipe           = 0,
    /// Draw indirect
    DrawIndirect        = 1,
    /// Vertex input
    VertexInput         = 2,
    /// Vertex shader
    VertexShader        = 3,
    /// Tessellation control
    TessellationControl = 4,
    /// Tessellation evaluation
    TessellationEvaluation = 5,
    /// Geometry shader
    GeometryShader      = 6,
    /// Fragment shader
    FragmentShader      = 7,
    /// Early fragment tests
    EarlyFragmentTests  = 8,
    /// Late fragment tests
    LateFragmentTests   = 9,
    /// Color attachment output
    ColorAttachmentOutput = 10,
    /// Compute shader
    ComputeShader       = 11,
    /// Transfer
    Transfer            = 12,
    /// Bottom of pipe
    BottomOfPipe        = 13,
    /// Host
    Host                = 14,
    /// All graphics
    AllGraphics         = 15,
    /// All commands
    #[default]
    AllCommands         = 16,
    /// Acceleration structure build
    AccelerationStructureBuild = 17,
    /// Ray tracing shader
    RayTracingShader    = 18,
    /// Task shader
    TaskShader          = 19,
    /// Mesh shader
    MeshShader          = 20,
}

/// Timestamp result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TimestampResult {
    /// Raw timestamp value
    pub value: u64,
    /// Is the timestamp available
    pub available: bool,
}

impl TimestampResult {
    /// Creates a new timestamp result
    #[inline]
    pub const fn new(value: u64) -> Self {
        Self {
            value,
            available: true,
        }
    }

    /// Unavailable timestamp
    pub const UNAVAILABLE: Self = Self {
        value: 0,
        available: false,
    };

    /// Calculates duration between two timestamps in nanoseconds
    #[inline]
    pub fn duration_ns(&self, other: &Self, timestamp_period: f32) -> f64 {
        if !self.available || !other.available {
            return 0.0;
        }
        let diff = if other.value > self.value {
            other.value - self.value
        } else {
            self.value - other.value
        };
        diff as f64 * timestamp_period as f64
    }

    /// Calculates duration in milliseconds
    #[inline]
    pub fn duration_ms(&self, other: &Self, timestamp_period: f32) -> f64 {
        self.duration_ns(other, timestamp_period) / 1_000_000.0
    }
}

// ============================================================================
// GPU Timing Query
// ============================================================================

/// GPU timing measurement
#[derive(Clone, Debug)]
#[repr(C)]
pub struct GpuTiming {
    /// Name/label for the measurement
    pub name: String,
    /// Start timestamp
    pub start: u64,
    /// End timestamp
    pub end: u64,
    /// Timestamp period (nanoseconds per tick)
    pub timestamp_period: f32,
    /// Pipeline stage
    pub stage: TimestampPipelineStage,
}

impl GpuTiming {
    /// Creates a new GPU timing
    #[inline]
    pub fn new(name: String, timestamp_period: f32) -> Self {
        Self {
            name,
            start: 0,
            end: 0,
            timestamp_period,
            stage: TimestampPipelineStage::AllCommands,
        }
    }

    /// Duration in nanoseconds
    #[inline]
    pub fn duration_ns(&self) -> f64 {
        if self.end < self.start {
            return 0.0;
        }
        (self.end - self.start) as f64 * self.timestamp_period as f64
    }

    /// Duration in microseconds
    #[inline]
    pub fn duration_us(&self) -> f64 {
        self.duration_ns() / 1_000.0
    }

    /// Duration in milliseconds
    #[inline]
    pub fn duration_ms(&self) -> f64 {
        self.duration_ns() / 1_000_000.0
    }

    /// Is measurement complete
    #[inline]
    pub const fn is_complete(&self) -> bool {
        self.end > self.start
    }
}

// ============================================================================
// Performance Counters
// ============================================================================

/// Hardware performance counter handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PerfCounterHandle(pub u64);

impl PerfCounterHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates a new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Performance counter description
#[derive(Clone, Debug)]
#[repr(C)]
pub struct PerfCounterDescription {
    /// Counter name
    pub name: String,
    /// Counter description
    pub description: String,
    /// Counter category
    pub category: PerfCounterCategory,
    /// Data type
    pub data_type: PerfCounterDataType,
    /// Unit
    pub unit: PerfCounterUnit,
    /// Unique ID
    pub id: u32,
}

impl PerfCounterDescription {
    /// Creates a new counter description
    #[inline]
    pub fn new(
        name: String,
        description: String,
        category: PerfCounterCategory,
        data_type: PerfCounterDataType,
        unit: PerfCounterUnit,
    ) -> Self {
        Self {
            name,
            description,
            category,
            data_type,
            unit,
            id: 0,
        }
    }
}

/// Performance counter category
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PerfCounterCategory {
    /// General GPU metrics
    #[default]
    General       = 0,
    /// Memory metrics
    Memory        = 1,
    /// Compute metrics
    Compute       = 2,
    /// Shader metrics
    Shader        = 3,
    /// Texture metrics
    Texture       = 4,
    /// Cache metrics
    Cache         = 5,
    /// Rasterization metrics
    Rasterization = 6,
    /// Bandwidth metrics
    Bandwidth     = 7,
    /// Occupancy metrics
    Occupancy     = 8,
    /// Stall metrics
    Stalls        = 9,
    /// Ray tracing metrics
    RayTracing    = 10,
}

/// Performance counter data type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PerfCounterDataType {
    /// 64-bit unsigned integer
    #[default]
    UInt64  = 0,
    /// 64-bit signed integer
    Int64   = 1,
    /// 32-bit float
    Float32 = 2,
    /// 64-bit float
    Float64 = 3,
}

/// Performance counter unit
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PerfCounterUnit {
    /// Raw count
    #[default]
    Count          = 0,
    /// Bytes
    Bytes          = 1,
    /// Bytes per second
    BytesPerSecond = 2,
    /// Nanoseconds
    Nanoseconds    = 3,
    /// Percentage (0-100)
    Percentage     = 4,
    /// Cycles
    Cycles         = 5,
    /// Megahertz
    Megahertz      = 6,
    /// Watts
    Watts          = 7,
    /// Temperature (Celsius)
    Celsius        = 8,
    /// Ratio
    Ratio          = 9,
}

/// Performance counter value
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub union PerfCounterValue {
    /// Unsigned 64-bit value
    pub uint64: u64,
    /// Signed 64-bit value
    pub int64: i64,
    /// 32-bit float value
    pub float32: f32,
    /// 64-bit float value
    pub float64: f64,
}

impl Default for PerfCounterValue {
    fn default() -> Self {
        Self { uint64: 0 }
    }
}

/// Performance counter sample
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PerfCounterSample {
    /// Counter ID
    pub counter_id: u32,
    /// Value
    pub value: PerfCounterValue,
    /// Timestamp when sampled
    pub timestamp: u64,
}

// ============================================================================
// GPU Profiling
// ============================================================================

/// GPU profiling session configuration
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ProfilingConfig {
    /// Enable pipeline statistics
    pub pipeline_statistics: bool,
    /// Enable timestamps
    pub timestamps: bool,
    /// Enable performance counters
    pub perf_counters: bool,
    /// Maximum profiling regions
    pub max_regions: u32,
    /// Timestamp query count
    pub timestamp_count: u32,
    /// Statistics flags
    pub statistics_flags: PipelineStatisticsFlags,
    /// Performance counters to enable
    pub enabled_counters: Vec<u32>,
}

impl ProfilingConfig {
    /// Creates a new configuration
    #[inline]
    pub fn new() -> Self {
        Self {
            pipeline_statistics: true,
            timestamps: true,
            perf_counters: false,
            max_regions: 256,
            timestamp_count: 512,
            statistics_flags: PipelineStatisticsFlags::ALL,
            enabled_counters: Vec::new(),
        }
    }

    /// Minimal profiling
    #[inline]
    pub fn minimal() -> Self {
        Self {
            pipeline_statistics: false,
            timestamps: true,
            perf_counters: false,
            max_regions: 64,
            timestamp_count: 128,
            statistics_flags: PipelineStatisticsFlags::NONE,
            enabled_counters: Vec::new(),
        }
    }

    /// Full profiling
    #[inline]
    pub fn full() -> Self {
        Self {
            pipeline_statistics: true,
            timestamps: true,
            perf_counters: true,
            max_regions: 1024,
            timestamp_count: 2048,
            statistics_flags: PipelineStatisticsFlags::ALL,
            enabled_counters: Vec::new(),
        }
    }

    /// With performance counters
    #[inline]
    pub fn with_perf_counters(mut self, counters: Vec<u32>) -> Self {
        self.perf_counters = true;
        self.enabled_counters = counters;
        self
    }

    /// With pipeline statistics
    #[inline]
    pub fn with_statistics(mut self, flags: PipelineStatisticsFlags) -> Self {
        self.pipeline_statistics = true;
        self.statistics_flags = flags;
        self
    }
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Profiling region marker
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ProfilingRegion {
    /// Region name
    pub name: String,
    /// Start timestamp index
    pub start_timestamp: u32,
    /// End timestamp index
    pub end_timestamp: u32,
    /// Color for visualization (RGBA)
    pub color: [f32; 4],
    /// Parent region index (u32::MAX for root)
    pub parent: u32,
    /// Depth level
    pub depth: u32,
}

impl ProfilingRegion {
    /// Creates a new profiling region
    #[inline]
    pub fn new(name: String, start_timestamp: u32) -> Self {
        Self {
            name,
            start_timestamp,
            end_timestamp: 0,
            color: [1.0, 1.0, 1.0, 1.0],
            parent: u32::MAX,
            depth: 0,
        }
    }

    /// With custom color
    #[inline]
    pub fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.color = [r, g, b, a];
        self
    }

    /// With parent region
    #[inline]
    pub fn with_parent(mut self, parent: u32, depth: u32) -> Self {
        self.parent = parent;
        self.depth = depth;
        self
    }

    /// Is complete
    #[inline]
    pub const fn is_complete(&self) -> bool {
        self.end_timestamp > self.start_timestamp
    }

    /// Is root region
    #[inline]
    pub const fn is_root(&self) -> bool {
        self.parent == u32::MAX
    }
}

// ============================================================================
// Frame Statistics
// ============================================================================

/// Per-frame statistics
#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct FrameStatistics {
    /// Frame number
    pub frame_number: u64,
    /// GPU time in nanoseconds
    pub gpu_time_ns: u64,
    /// CPU time in nanoseconds
    pub cpu_time_ns: u64,
    /// Draw call count
    pub draw_calls: u32,
    /// Dispatch call count
    pub dispatch_calls: u32,
    /// Triangle count
    pub triangles: u64,
    /// Vertex count
    pub vertices: u64,
    /// State changes
    pub state_changes: u32,
    /// Texture bindings
    pub texture_bindings: u32,
    /// Buffer bindings
    pub buffer_bindings: u32,
    /// Pipeline bindings
    pub pipeline_bindings: u32,
    /// Render pass count
    pub render_passes: u32,
    /// Memory allocated this frame
    pub memory_allocated: u64,
    /// Memory freed this frame
    pub memory_freed: u64,
}

impl FrameStatistics {
    /// Creates new frame statistics
    #[inline]
    pub const fn new(frame_number: u64) -> Self {
        Self {
            frame_number,
            gpu_time_ns: 0,
            cpu_time_ns: 0,
            draw_calls: 0,
            dispatch_calls: 0,
            triangles: 0,
            vertices: 0,
            state_changes: 0,
            texture_bindings: 0,
            buffer_bindings: 0,
            pipeline_bindings: 0,
            render_passes: 0,
            memory_allocated: 0,
            memory_freed: 0,
        }
    }

    /// GPU time in milliseconds
    #[inline]
    pub fn gpu_time_ms(&self) -> f64 {
        self.gpu_time_ns as f64 / 1_000_000.0
    }

    /// CPU time in milliseconds
    #[inline]
    pub fn cpu_time_ms(&self) -> f64 {
        self.cpu_time_ns as f64 / 1_000_000.0
    }

    /// Frame time in milliseconds (max of CPU/GPU)
    #[inline]
    pub fn frame_time_ms(&self) -> f64 {
        self.gpu_time_ms().max(self.cpu_time_ms())
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

    /// Triangles per draw call
    #[inline]
    pub fn triangles_per_draw(&self) -> f64 {
        if self.draw_calls == 0 {
            0.0
        } else {
            self.triangles as f64 / self.draw_calls as f64
        }
    }

    /// Total binding changes
    #[inline]
    pub fn total_bindings(&self) -> u32 {
        self.texture_bindings + self.buffer_bindings + self.pipeline_bindings
    }

    /// Memory delta (allocated - freed)
    #[inline]
    pub fn memory_delta(&self) -> i64 {
        self.memory_allocated as i64 - self.memory_freed as i64
    }

    /// Accumulates draw calls
    #[inline]
    pub fn record_draw(&mut self, vertex_count: u32, triangle_count: u32) {
        self.draw_calls += 1;
        self.vertices += vertex_count as u64;
        self.triangles += triangle_count as u64;
    }

    /// Records a dispatch
    #[inline]
    pub fn record_dispatch(&mut self) {
        self.dispatch_calls += 1;
    }

    /// Resets for new frame
    #[inline]
    pub fn reset(&mut self, frame_number: u64) {
        *self = Self::new(frame_number);
    }
}

// ============================================================================
// Statistics Aggregation
// ============================================================================

/// Aggregated statistics over multiple frames
#[derive(Clone, Debug)]
#[repr(C)]
pub struct AggregatedStatistics {
    /// Number of frames aggregated
    pub frame_count: u64,
    /// Min GPU time
    pub min_gpu_time_ns: u64,
    /// Max GPU time
    pub max_gpu_time_ns: u64,
    /// Total GPU time
    pub total_gpu_time_ns: u64,
    /// Min CPU time
    pub min_cpu_time_ns: u64,
    /// Max CPU time
    pub max_cpu_time_ns: u64,
    /// Total CPU time
    pub total_cpu_time_ns: u64,
    /// Total draw calls
    pub total_draw_calls: u64,
    /// Total triangles
    pub total_triangles: u64,
}

impl AggregatedStatistics {
    /// Creates new aggregated statistics
    #[inline]
    pub const fn new() -> Self {
        Self {
            frame_count: 0,
            min_gpu_time_ns: u64::MAX,
            max_gpu_time_ns: 0,
            total_gpu_time_ns: 0,
            min_cpu_time_ns: u64::MAX,
            max_cpu_time_ns: 0,
            total_cpu_time_ns: 0,
            total_draw_calls: 0,
            total_triangles: 0,
        }
    }

    /// Adds a frame to the aggregation
    #[inline]
    pub fn add_frame(&mut self, frame: &FrameStatistics) {
        self.frame_count += 1;
        self.min_gpu_time_ns = self.min_gpu_time_ns.min(frame.gpu_time_ns);
        self.max_gpu_time_ns = self.max_gpu_time_ns.max(frame.gpu_time_ns);
        self.total_gpu_time_ns += frame.gpu_time_ns;
        self.min_cpu_time_ns = self.min_cpu_time_ns.min(frame.cpu_time_ns);
        self.max_cpu_time_ns = self.max_cpu_time_ns.max(frame.cpu_time_ns);
        self.total_cpu_time_ns += frame.cpu_time_ns;
        self.total_draw_calls += frame.draw_calls as u64;
        self.total_triangles += frame.triangles;
    }

    /// Average GPU time in milliseconds
    #[inline]
    pub fn avg_gpu_time_ms(&self) -> f64 {
        if self.frame_count == 0 {
            0.0
        } else {
            (self.total_gpu_time_ns as f64 / self.frame_count as f64) / 1_000_000.0
        }
    }

    /// Average CPU time in milliseconds
    #[inline]
    pub fn avg_cpu_time_ms(&self) -> f64 {
        if self.frame_count == 0 {
            0.0
        } else {
            (self.total_cpu_time_ns as f64 / self.frame_count as f64) / 1_000_000.0
        }
    }

    /// Average FPS
    #[inline]
    pub fn avg_fps(&self) -> f64 {
        let avg_time = self.avg_gpu_time_ms().max(self.avg_cpu_time_ms());
        if avg_time > 0.0 {
            1000.0 / avg_time
        } else {
            0.0
        }
    }

    /// Average draw calls per frame
    #[inline]
    pub fn avg_draw_calls(&self) -> f64 {
        if self.frame_count == 0 {
            0.0
        } else {
            self.total_draw_calls as f64 / self.frame_count as f64
        }
    }

    /// Average triangles per frame
    #[inline]
    pub fn avg_triangles(&self) -> f64 {
        if self.frame_count == 0 {
            0.0
        } else {
            self.total_triangles as f64 / self.frame_count as f64
        }
    }

    /// Resets the aggregation
    #[inline]
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for AggregatedStatistics {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Memory Statistics
// ============================================================================

/// GPU memory statistics
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MemoryStatistics {
    /// Total device memory
    pub total_device_memory: u64,
    /// Used device memory
    pub used_device_memory: u64,
    /// Total host visible memory
    pub total_host_memory: u64,
    /// Used host visible memory
    pub used_host_memory: u64,
    /// Number of allocations
    pub allocation_count: u32,
    /// Number of buffer allocations
    pub buffer_count: u32,
    /// Number of image allocations
    pub image_count: u32,
    /// Memory fragmentation estimate (0-1)
    pub fragmentation: f32,
}

impl MemoryStatistics {
    /// Creates new memory statistics
    #[inline]
    pub const fn new() -> Self {
        Self {
            total_device_memory: 0,
            used_device_memory: 0,
            total_host_memory: 0,
            used_host_memory: 0,
            allocation_count: 0,
            buffer_count: 0,
            image_count: 0,
            fragmentation: 0.0,
        }
    }

    /// Device memory usage percentage
    #[inline]
    pub fn device_memory_usage(&self) -> f64 {
        if self.total_device_memory == 0 {
            0.0
        } else {
            self.used_device_memory as f64 / self.total_device_memory as f64 * 100.0
        }
    }

    /// Host memory usage percentage
    #[inline]
    pub fn host_memory_usage(&self) -> f64 {
        if self.total_host_memory == 0 {
            0.0
        } else {
            self.used_host_memory as f64 / self.total_host_memory as f64 * 100.0
        }
    }

    /// Available device memory
    #[inline]
    pub fn available_device_memory(&self) -> u64 {
        self.total_device_memory
            .saturating_sub(self.used_device_memory)
    }

    /// Average allocation size
    #[inline]
    pub fn avg_allocation_size(&self) -> u64 {
        if self.allocation_count == 0 {
            0
        } else {
            (self.used_device_memory + self.used_host_memory) / self.allocation_count as u64
        }
    }
}

// ============================================================================
// Bandwidth Statistics
// ============================================================================

/// Bandwidth statistics
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct BandwidthStatistics {
    /// Bytes read from device memory
    pub device_read_bytes: u64,
    /// Bytes written to device memory
    pub device_write_bytes: u64,
    /// Bytes transferred host to device
    pub host_to_device_bytes: u64,
    /// Bytes transferred device to host
    pub device_to_host_bytes: u64,
    /// Time period in nanoseconds
    pub time_period_ns: u64,
}

impl BandwidthStatistics {
    /// Creates new bandwidth statistics
    #[inline]
    pub const fn new() -> Self {
        Self {
            device_read_bytes: 0,
            device_write_bytes: 0,
            host_to_device_bytes: 0,
            device_to_host_bytes: 0,
            time_period_ns: 1_000_000_000, // 1 second default
        }
    }

    /// Device read bandwidth in GB/s
    #[inline]
    pub fn device_read_bandwidth_gbps(&self) -> f64 {
        if self.time_period_ns == 0 {
            0.0
        } else {
            (self.device_read_bytes as f64 / self.time_period_ns as f64) * 1_000_000_000.0
                / 1_073_741_824.0
        }
    }

    /// Device write bandwidth in GB/s
    #[inline]
    pub fn device_write_bandwidth_gbps(&self) -> f64 {
        if self.time_period_ns == 0 {
            0.0
        } else {
            (self.device_write_bytes as f64 / self.time_period_ns as f64) * 1_000_000_000.0
                / 1_073_741_824.0
        }
    }

    /// Total device bandwidth in GB/s
    #[inline]
    pub fn total_device_bandwidth_gbps(&self) -> f64 {
        self.device_read_bandwidth_gbps() + self.device_write_bandwidth_gbps()
    }

    /// PCIe upload bandwidth in GB/s
    #[inline]
    pub fn pcie_upload_bandwidth_gbps(&self) -> f64 {
        if self.time_period_ns == 0 {
            0.0
        } else {
            (self.host_to_device_bytes as f64 / self.time_period_ns as f64) * 1_000_000_000.0
                / 1_073_741_824.0
        }
    }

    /// PCIe download bandwidth in GB/s
    #[inline]
    pub fn pcie_download_bandwidth_gbps(&self) -> f64 {
        if self.time_period_ns == 0 {
            0.0
        } else {
            (self.device_to_host_bytes as f64 / self.time_period_ns as f64) * 1_000_000_000.0
                / 1_073_741_824.0
        }
    }

    /// Accumulates bandwidth
    #[inline]
    pub fn accumulate(&mut self, other: &Self) {
        self.device_read_bytes += other.device_read_bytes;
        self.device_write_bytes += other.device_write_bytes;
        self.host_to_device_bytes += other.host_to_device_bytes;
        self.device_to_host_bytes += other.device_to_host_bytes;
        self.time_period_ns += other.time_period_ns;
    }

    /// Resets statistics
    #[inline]
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}
