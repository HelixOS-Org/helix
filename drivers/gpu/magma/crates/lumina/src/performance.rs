//! Performance counters and queries
//!
//! This module provides types for GPU performance monitoring.

extern crate alloc;
use alloc::vec::Vec;

/// Performance counter handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PerformanceCounterHandle(pub u64);

impl PerformanceCounterHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// Performance query handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PerformanceQueryHandle(pub u64);

impl PerformanceQueryHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// Performance counter unit
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PerformanceCounterUnit {
    /// Generic counter
    Generic,
    /// Percentage (0-100)
    Percentage,
    /// Nanoseconds
    Nanoseconds,
    /// Bytes
    Bytes,
    /// Bytes per second
    BytesPerSecond,
    /// Kelvin (temperature)
    Kelvin,
    /// Watts
    Watts,
    /// Volts
    Volts,
    /// Amps
    Amps,
    /// Hertz
    Hertz,
    /// Cycles
    Cycles,
}

/// Performance counter scope
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PerformanceCounterScope {
    /// Command buffer scope
    CommandBuffer,
    /// Render pass scope
    RenderPass,
    /// Command scope
    Command,
}

/// Performance counter storage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PerformanceCounterStorage {
    /// 32-bit integer
    Int32,
    /// 64-bit integer
    Int64,
    /// 32-bit unsigned
    Uint32,
    /// 64-bit unsigned
    Uint64,
    /// 32-bit float
    Float32,
    /// 64-bit float
    Float64,
}

/// Performance counter description
#[derive(Clone, Debug)]
pub struct PerformanceCounterDescription {
    /// Counter ID
    pub id: u32,
    /// Name
    pub name: Vec<u8>,
    /// Category
    pub category: Vec<u8>,
    /// Description
    pub description: Vec<u8>,
    /// Unit
    pub unit: PerformanceCounterUnit,
    /// Scope
    pub scope: PerformanceCounterScope,
    /// Storage type
    pub storage: PerformanceCounterStorage,
    /// UUID
    pub uuid: [u8; 16],
}

/// Performance counter value
#[derive(Clone, Copy, Debug)]
pub enum PerformanceCounterValue {
    /// 32-bit signed
    Int32(i32),
    /// 64-bit signed
    Int64(i64),
    /// 32-bit unsigned
    Uint32(u32),
    /// 64-bit unsigned
    Uint64(u64),
    /// 32-bit float
    Float32(f32),
    /// 64-bit float
    Float64(f64),
}

impl PerformanceCounterValue {
    /// Gets as f64
    pub fn as_f64(&self) -> f64 {
        match self {
            Self::Int32(v) => *v as f64,
            Self::Int64(v) => *v as f64,
            Self::Uint32(v) => *v as f64,
            Self::Uint64(v) => *v as f64,
            Self::Float32(v) => *v as f64,
            Self::Float64(v) => *v,
        }
    }

    /// Gets as u64
    pub fn as_u64(&self) -> u64 {
        match self {
            Self::Int32(v) => *v as u64,
            Self::Int64(v) => *v as u64,
            Self::Uint32(v) => *v as u64,
            Self::Uint64(v) => *v,
            Self::Float32(v) => *v as u64,
            Self::Float64(v) => *v as u64,
        }
    }
}

/// Performance query create info
#[derive(Clone, Debug)]
pub struct PerformanceQueryCreateInfo {
    /// Queue family index
    pub queue_family_index: u32,
    /// Counter indices
    pub counter_indices: Vec<u32>,
}

impl PerformanceQueryCreateInfo {
    /// Creates new performance query info
    pub fn new(queue_family_index: u32) -> Self {
        Self {
            queue_family_index,
            counter_indices: Vec::new(),
        }
    }

    /// Adds a counter
    pub fn with_counter(mut self, counter_index: u32) -> Self {
        self.counter_indices.push(counter_index);
        self
    }

    /// Adds multiple counters
    pub fn with_counters(mut self, indices: &[u32]) -> Self {
        self.counter_indices.extend_from_slice(indices);
        self
    }
}

/// Performance query pool create info
#[derive(Clone, Debug)]
pub struct PerformanceQueryPoolCreateInfo {
    /// Queue family index
    pub queue_family_index: u32,
    /// Query count
    pub query_count: u32,
    /// Counter indices
    pub counter_indices: Vec<u32>,
}

/// Acquire profiling lock info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AcquireProfilingLockInfo {
    /// Timeout in nanoseconds
    pub timeout: u64,
    /// Flags
    pub flags: AcquireProfilingLockFlags,
}

impl AcquireProfilingLockInfo {
    /// Creates with infinite timeout
    pub const fn infinite() -> Self {
        Self {
            timeout: u64::MAX,
            flags: AcquireProfilingLockFlags::NONE,
        }
    }

    /// Creates with timeout
    pub const fn with_timeout(timeout_ns: u64) -> Self {
        Self {
            timeout: timeout_ns,
            flags: AcquireProfilingLockFlags::NONE,
        }
    }
}

/// Acquire profiling lock flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct AcquireProfilingLockFlags(pub u32);

impl AcquireProfilingLockFlags {
    /// No flags
    pub const NONE: Self = Self(0);
}

/// GPU timing sample
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuTimingSample {
    /// Start timestamp
    pub start: u64,
    /// End timestamp
    pub end: u64,
    /// Timestamp period (nanoseconds per tick)
    pub timestamp_period: f32,
}

impl GpuTimingSample {
    /// Duration in nanoseconds
    pub fn duration_ns(&self) -> f64 {
        (self.end - self.start) as f64 * self.timestamp_period as f64
    }

    /// Duration in microseconds
    pub fn duration_us(&self) -> f64 {
        self.duration_ns() / 1000.0
    }

    /// Duration in milliseconds
    pub fn duration_ms(&self) -> f64 {
        self.duration_ns() / 1_000_000.0
    }
}

/// GPU statistics
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuStatistics {
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
    pub c_invocations: u64,
    /// Clipping primitives
    pub c_primitives: u64,
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

impl GpuStatistics {
    /// Primitive culling ratio
    pub fn culling_ratio(&self) -> f64 {
        if self.c_invocations > 0 {
            1.0 - (self.c_primitives as f64 / self.c_invocations as f64)
        } else {
            0.0
        }
    }

    /// Average vertices per primitive
    pub fn vertices_per_primitive(&self) -> f64 {
        if self.ia_primitives > 0 {
            self.ia_vertices as f64 / self.ia_primitives as f64
        } else {
            0.0
        }
    }
}

/// Pipeline statistics flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PipelineStatisticsFlags(pub u32);

impl PipelineStatisticsFlags {
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
    /// Tessellation control patches
    pub const TESSELLATION_CONTROL_PATCHES: Self = Self(1 << 8);
    /// Tessellation evaluation invocations
    pub const TESSELLATION_EVALUATION_INVOCATIONS: Self = Self(1 << 9);
    /// Compute shader invocations
    pub const COMPUTE_SHADER_INVOCATIONS: Self = Self(1 << 10);
    /// Task shader invocations
    pub const TASK_SHADER_INVOCATIONS: Self = Self(1 << 11);
    /// Mesh shader invocations
    pub const MESH_SHADER_INVOCATIONS: Self = Self(1 << 12);

    /// All statistics
    pub const ALL: Self = Self(0x1FFF);

    /// Checks if contains flag
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    /// Combines flags
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Memory statistics
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MemoryStatistics {
    /// Allocated bytes
    pub allocated_bytes: u64,
    /// Used bytes
    pub used_bytes: u64,
    /// Allocation count
    pub allocation_count: u32,
    /// Block count
    pub block_count: u32,
}

impl MemoryStatistics {
    /// Fragmentation ratio (0 = perfect, 1 = worst)
    pub fn fragmentation(&self) -> f64 {
        if self.allocated_bytes > 0 {
            1.0 - (self.used_bytes as f64 / self.allocated_bytes as f64)
        } else {
            0.0
        }
    }

    /// Average allocation size
    pub fn avg_allocation_size(&self) -> u64 {
        if self.allocation_count > 0 {
            self.used_bytes / self.allocation_count as u64
        } else {
            0
        }
    }
}

/// Memory budget
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MemoryBudget {
    /// Heap index
    pub heap_index: u32,
    /// Total budget in bytes
    pub budget_bytes: u64,
    /// Current usage in bytes
    pub usage_bytes: u64,
}

impl MemoryBudget {
    /// Available bytes
    pub const fn available(&self) -> u64 {
        if self.budget_bytes > self.usage_bytes {
            self.budget_bytes - self.usage_bytes
        } else {
            0
        }
    }

    /// Usage percentage
    pub fn usage_percent(&self) -> f64 {
        if self.budget_bytes > 0 {
            (self.usage_bytes as f64 / self.budget_bytes as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Is over budget
    pub const fn is_over_budget(&self) -> bool {
        self.usage_bytes > self.budget_bytes
    }
}

/// Frame timing info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FrameTimingInfo {
    /// Frame number
    pub frame_number: u64,
    /// CPU frame start time
    pub cpu_start_ns: u64,
    /// CPU frame end time
    pub cpu_end_ns: u64,
    /// GPU frame start time
    pub gpu_start_ns: u64,
    /// GPU frame end time
    pub gpu_end_ns: u64,
    /// Present time
    pub present_ns: u64,
}

impl FrameTimingInfo {
    /// CPU frame time in ms
    pub fn cpu_time_ms(&self) -> f64 {
        (self.cpu_end_ns - self.cpu_start_ns) as f64 / 1_000_000.0
    }

    /// GPU frame time in ms
    pub fn gpu_time_ms(&self) -> f64 {
        (self.gpu_end_ns - self.gpu_start_ns) as f64 / 1_000_000.0
    }

    /// Total frame time in ms
    pub fn total_time_ms(&self) -> f64 {
        (self.present_ns - self.cpu_start_ns) as f64 / 1_000_000.0
    }

    /// Frame rate
    pub fn fps(&self) -> f64 {
        let time_ms = self.total_time_ms();
        if time_ms > 0.0 {
            1000.0 / time_ms
        } else {
            0.0
        }
    }

    /// Is GPU bound
    pub fn is_gpu_bound(&self) -> bool {
        self.gpu_time_ms() > self.cpu_time_ms()
    }
}

/// GPU power state
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuPowerState {
    /// Current power draw in milliwatts
    pub power_mw: u32,
    /// Temperature in millicelsius
    pub temperature_mc: u32,
    /// Current clock speed in MHz
    pub core_clock_mhz: u32,
    /// Memory clock speed in MHz
    pub memory_clock_mhz: u32,
    /// Fan speed in RPM (0 if passive)
    pub fan_rpm: u32,
    /// GPU utilization percentage
    pub gpu_utilization: u8,
    /// Memory utilization percentage
    pub memory_utilization: u8,
}

impl GpuPowerState {
    /// Power in watts
    pub fn power_watts(&self) -> f32 {
        self.power_mw as f32 / 1000.0
    }

    /// Temperature in celsius
    pub fn temperature_celsius(&self) -> f32 {
        self.temperature_mc as f32 / 1000.0
    }

    /// Is thermal throttling likely
    pub fn is_thermal_throttling(&self) -> bool {
        self.temperature_mc > 85000 // 85°C
    }
}

/// Profiling marker type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ProfilingMarkerType {
    /// Generic marker
    Generic,
    /// Render marker
    Render,
    /// Compute marker
    Compute,
    /// Transfer marker
    Transfer,
    /// Present marker
    Present,
}

/// Profiling region
#[derive(Clone, Debug)]
pub struct ProfilingRegion {
    /// Name
    pub name: Vec<u8>,
    /// Type
    pub marker_type: ProfilingMarkerType,
    /// Start time (ns)
    pub start_ns: u64,
    /// End time (ns)
    pub end_ns: u64,
    /// Color
    pub color: [f32; 4],
    /// Thread ID
    pub thread_id: u64,
    /// Depth
    pub depth: u32,
}

impl ProfilingRegion {
    /// Duration in nanoseconds
    pub fn duration_ns(&self) -> u64 {
        self.end_ns - self.start_ns
    }

    /// Duration in milliseconds
    pub fn duration_ms(&self) -> f64 {
        self.duration_ns() as f64 / 1_000_000.0
    }
}

/// Profiling session
#[derive(Clone, Debug, Default)]
pub struct ProfilingSession {
    /// Regions
    pub regions: Vec<ProfilingRegion>,
    /// Start time
    pub start_ns: u64,
    /// End time
    pub end_ns: u64,
    /// GPU timing samples
    pub gpu_timing: Vec<GpuTimingSample>,
}

impl ProfilingSession {
    /// Total duration in ms
    pub fn duration_ms(&self) -> f64 {
        (self.end_ns - self.start_ns) as f64 / 1_000_000.0
    }

    /// Find hotspots (regions taking > threshold_ms)
    pub fn find_hotspots(&self, threshold_ms: f64) -> Vec<&ProfilingRegion> {
        self.regions
            .iter()
            .filter(|r| r.duration_ms() > threshold_ms)
            .collect()
    }
}
