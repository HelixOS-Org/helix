//! Profiling Types for Lumina
//!
//! This module provides profiling and performance monitoring infrastructure
//! for graphics applications including GPU timing, memory tracking, and statistics.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Profiler Handles
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

/// Profile scope handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ProfileScopeHandle(pub u64);

impl ProfileScopeHandle {
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

impl Default for ProfileScopeHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// GPU timestamp handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TimestampHandle(pub u64);

impl TimestampHandle {
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

impl Default for TimestampHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Profiler Settings
// ============================================================================

/// Profiler create info
#[derive(Clone, Debug)]
pub struct ProfilerCreateInfo {
    /// Enable CPU profiling
    pub cpu_profiling: bool,
    /// Enable GPU profiling
    pub gpu_profiling: bool,
    /// Enable memory profiling
    pub memory_profiling: bool,
    /// History size (frames)
    pub history_size: u32,
    /// Max scopes
    pub max_scopes: u32,
    /// Max timestamps per frame
    pub max_timestamps: u32,
}

impl ProfilerCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            cpu_profiling: true,
            gpu_profiling: true,
            memory_profiling: true,
            history_size: 120,
            max_scopes: 256,
            max_timestamps: 512,
        }
    }

    /// CPU only
    pub fn cpu_only() -> Self {
        Self {
            gpu_profiling: false,
            ..Self::new()
        }
    }

    /// GPU only
    pub fn gpu_only() -> Self {
        Self {
            cpu_profiling: false,
            memory_profiling: false,
            ..Self::new()
        }
    }

    /// With history size
    pub fn with_history(mut self, frames: u32) -> Self {
        self.history_size = frames;
        self
    }
}

impl Default for ProfilerCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Profile Scope
// ============================================================================

/// Profile scope info
#[derive(Clone, Debug)]
pub struct ProfileScopeInfo {
    /// Scope name
    pub name: &'static str,
    /// Category
    pub category: ProfileCategory,
    /// Color
    pub color: [f32; 4],
}

impl ProfileScopeInfo {
    /// Creates info
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            category: ProfileCategory::General,
            color: [0.4, 0.6, 0.8, 1.0],
        }
    }

    /// With category
    pub fn with_category(mut self, category: ProfileCategory) -> Self {
        self.category = category;
        self.color = category.default_color();
        self
    }

    /// With color
    pub fn with_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.color = [r, g, b, 1.0];
        self
    }
}

impl Default for ProfileScopeInfo {
    fn default() -> Self {
        Self::new("Unknown")
    }
}

/// Profile category
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ProfileCategory {
    /// General
    #[default]
    General = 0,
    /// Rendering
    Rendering = 1,
    /// Compute
    Compute = 2,
    /// Transfer
    Transfer = 3,
    /// Physics
    Physics = 4,
    /// Animation
    Animation = 5,
    /// Audio
    Audio = 6,
    /// UI
    Ui = 7,
    /// Script
    Script = 8,
    /// IO
    Io = 9,
}

impl ProfileCategory {
    /// Default color
    pub const fn default_color(&self) -> [f32; 4] {
        match self {
            Self::General => [0.5, 0.5, 0.5, 1.0],
            Self::Rendering => [0.2, 0.6, 0.9, 1.0],
            Self::Compute => [0.9, 0.4, 0.2, 1.0],
            Self::Transfer => [0.9, 0.9, 0.2, 1.0],
            Self::Physics => [0.2, 0.9, 0.4, 1.0],
            Self::Animation => [0.8, 0.2, 0.8, 1.0],
            Self::Audio => [0.2, 0.8, 0.8, 1.0],
            Self::Ui => [0.9, 0.6, 0.2, 1.0],
            Self::Script => [0.6, 0.2, 0.9, 1.0],
            Self::Io => [0.4, 0.4, 0.8, 1.0],
        }
    }
}

/// Profile scope result
#[derive(Clone, Debug)]
pub struct ProfileScopeResult {
    /// Scope name
    pub name: String,
    /// Category
    pub category: ProfileCategory,
    /// Start time (microseconds)
    pub start_us: u64,
    /// Duration (microseconds)
    pub duration_us: u64,
    /// Depth
    pub depth: u32,
    /// Thread ID
    pub thread_id: u64,
    /// Children
    pub children: Vec<ProfileScopeResult>,
}

impl ProfileScopeResult {
    /// Duration in milliseconds
    pub fn duration_ms(&self) -> f32 {
        self.duration_us as f32 / 1000.0
    }

    /// Percentage of parent
    pub fn percentage_of(&self, parent_duration: u64) -> f32 {
        if parent_duration == 0 {
            0.0
        } else {
            self.duration_us as f32 / parent_duration as f32 * 100.0
        }
    }
}

// ============================================================================
// GPU Timing
// ============================================================================

/// GPU timing query
#[derive(Clone, Debug)]
pub struct GpuTimingQuery {
    /// Query name
    pub name: &'static str,
    /// Start timestamp
    pub start: TimestampHandle,
    /// End timestamp
    pub end: TimestampHandle,
    /// Is pending
    pub pending: bool,
}

impl GpuTimingQuery {
    /// Creates query
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: TimestampHandle::NULL,
            end: TimestampHandle::NULL,
            pending: false,
        }
    }
}

/// GPU timing result
#[derive(Clone, Debug)]
pub struct GpuTimingResult {
    /// Name
    pub name: String,
    /// Duration (microseconds)
    pub duration_us: u64,
    /// Start offset (microseconds)
    pub start_us: u64,
}

impl GpuTimingResult {
    /// Duration in milliseconds
    pub fn duration_ms(&self) -> f32 {
        self.duration_us as f32 / 1000.0
    }
}

/// GPU pipeline statistics
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuPipelineStats {
    /// Input assembly vertices
    pub input_vertices: u64,
    /// Input assembly primitives
    pub input_primitives: u64,
    /// Vertex shader invocations
    pub vs_invocations: u64,
    /// Geometry shader invocations
    pub gs_invocations: u64,
    /// Geometry shader primitives
    pub gs_primitives: u64,
    /// Clipping invocations
    pub clipping_invocations: u64,
    /// Clipping primitives
    pub clipping_primitives: u64,
    /// Fragment shader invocations
    pub fs_invocations: u64,
    /// Tessellation control invocations
    pub tcs_patches: u64,
    /// Tessellation evaluation invocations
    pub tes_invocations: u64,
    /// Compute shader invocations
    pub cs_invocations: u64,
}

// ============================================================================
// Memory Profiling
// ============================================================================

/// Memory allocation info
#[derive(Clone, Debug)]
pub struct MemoryAllocationInfo {
    /// Allocation ID
    pub id: u64,
    /// Size (bytes)
    pub size: u64,
    /// Alignment
    pub alignment: u64,
    /// Memory type
    pub memory_type: MemoryType,
    /// Timestamp (microseconds)
    pub timestamp_us: u64,
    /// Name/tag
    pub name: String,
}

/// Memory type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MemoryType {
    /// GPU local (device only)
    #[default]
    GpuLocal = 0,
    /// CPU visible (host visible)
    CpuVisible = 1,
    /// CPU-GPU shared (host coherent)
    Shared = 2,
    /// Staging (upload/download)
    Staging = 3,
}

/// Memory pool stats
#[derive(Clone, Debug, Default)]
pub struct MemoryPoolStats {
    /// Pool name
    pub name: String,
    /// Memory type
    pub memory_type: MemoryType,
    /// Total size
    pub total_size: u64,
    /// Used size
    pub used_size: u64,
    /// Allocation count
    pub allocation_count: u32,
    /// Peak usage
    pub peak_usage: u64,
}

impl MemoryPoolStats {
    /// Free size
    pub fn free_size(&self) -> u64 {
        self.total_size.saturating_sub(self.used_size)
    }

    /// Usage percentage
    pub fn usage_percent(&self) -> f32 {
        if self.total_size == 0 {
            0.0
        } else {
            self.used_size as f32 / self.total_size as f32 * 100.0
        }
    }
}

/// Memory budget
#[derive(Clone, Copy, Debug, Default)]
pub struct MemoryBudget {
    /// Available memory (bytes)
    pub available: u64,
    /// Used memory (bytes)
    pub used: u64,
    /// Reserved by system (bytes)
    pub reserved: u64,
}

impl MemoryBudget {
    /// Free memory
    pub fn free(&self) -> u64 {
        self.available.saturating_sub(self.used)
    }

    /// Usage percentage
    pub fn usage_percent(&self) -> f32 {
        if self.available == 0 {
            0.0
        } else {
            self.used as f32 / self.available as f32 * 100.0
        }
    }
}

// ============================================================================
// Frame Statistics
// ============================================================================

/// Frame statistics
#[derive(Clone, Debug, Default)]
pub struct FrameStats {
    /// Frame number
    pub frame_number: u64,
    /// Frame time (microseconds)
    pub frame_time_us: u64,
    /// CPU time (microseconds)
    pub cpu_time_us: u64,
    /// GPU time (microseconds)
    pub gpu_time_us: u64,
    /// Present time (microseconds)
    pub present_time_us: u64,
    /// Draw calls
    pub draw_calls: u32,
    /// Dispatches
    pub dispatches: u32,
    /// Triangles
    pub triangles: u64,
    /// Vertices
    pub vertices: u64,
    /// State changes
    pub state_changes: u32,
    /// Pipeline binds
    pub pipeline_binds: u32,
    /// Buffer binds
    pub buffer_binds: u32,
    /// Texture binds
    pub texture_binds: u32,
}

impl FrameStats {
    /// FPS
    pub fn fps(&self) -> f32 {
        if self.frame_time_us == 0 {
            0.0
        } else {
            1_000_000.0 / self.frame_time_us as f32
        }
    }

    /// Frame time in milliseconds
    pub fn frame_time_ms(&self) -> f32 {
        self.frame_time_us as f32 / 1000.0
    }

    /// CPU time in milliseconds
    pub fn cpu_time_ms(&self) -> f32 {
        self.cpu_time_us as f32 / 1000.0
    }

    /// GPU time in milliseconds
    pub fn gpu_time_ms(&self) -> f32 {
        self.gpu_time_us as f32 / 1000.0
    }
}

/// Frame time history
#[derive(Clone, Debug)]
pub struct FrameTimeHistory {
    /// Times (microseconds)
    pub times: Vec<u64>,
    /// Current index
    pub index: usize,
    /// Capacity
    pub capacity: usize,
}

impl FrameTimeHistory {
    /// Creates history
    pub fn new(capacity: usize) -> Self {
        Self {
            times: Vec::from_iter(core::iter::repeat(0).take(capacity)),
            index: 0,
            capacity,
        }
    }

    /// Push time
    pub fn push(&mut self, time_us: u64) {
        self.times[self.index] = time_us;
        self.index = (self.index + 1) % self.capacity;
    }

    /// Average time
    pub fn average(&self) -> u64 {
        let sum: u64 = self.times.iter().sum();
        sum / self.capacity as u64
    }

    /// Min time
    pub fn min(&self) -> u64 {
        self.times.iter().copied().min().unwrap_or(0)
    }

    /// Max time
    pub fn max(&self) -> u64 {
        self.times.iter().copied().max().unwrap_or(0)
    }

    /// As slice (in order)
    pub fn as_slice(&self) -> Vec<u64> {
        let mut result = Vec::with_capacity(self.capacity);
        for i in 0..self.capacity {
            let idx = (self.index + i) % self.capacity;
            result.push(self.times[idx]);
        }
        result
    }
}

impl Default for FrameTimeHistory {
    fn default() -> Self {
        Self::new(120)
    }
}

// ============================================================================
// Counter Types
// ============================================================================

/// Counter
#[derive(Clone, Debug)]
pub struct Counter {
    /// Name
    pub name: String,
    /// Value
    pub value: i64,
    /// Unit
    pub unit: CounterUnit,
}

impl Counter {
    /// Creates counter
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            value: 0,
            unit: CounterUnit::Count,
        }
    }

    /// With unit
    pub fn with_unit(mut self, unit: CounterUnit) -> Self {
        self.unit = unit;
        self
    }

    /// Increment
    pub fn increment(&mut self) {
        self.value += 1;
    }

    /// Add
    pub fn add(&mut self, amount: i64) {
        self.value += amount;
    }

    /// Reset
    pub fn reset(&mut self) {
        self.value = 0;
    }

    /// Formatted value
    pub fn formatted(&self) -> String {
        match self.unit {
            CounterUnit::Count => {
                if self.value > 1_000_000 {
                    alloc::format!("{:.1}M", self.value as f64 / 1_000_000.0)
                } else if self.value > 1_000 {
                    alloc::format!("{:.1}K", self.value as f64 / 1_000.0)
                } else {
                    alloc::format!("{}", self.value)
                }
            }
            CounterUnit::Bytes => {
                if self.value > 1_073_741_824 {
                    alloc::format!("{:.2} GB", self.value as f64 / 1_073_741_824.0)
                } else if self.value > 1_048_576 {
                    alloc::format!("{:.2} MB", self.value as f64 / 1_048_576.0)
                } else if self.value > 1024 {
                    alloc::format!("{:.2} KB", self.value as f64 / 1024.0)
                } else {
                    alloc::format!("{} B", self.value)
                }
            }
            CounterUnit::Microseconds => {
                if self.value > 1_000_000 {
                    alloc::format!("{:.2} s", self.value as f64 / 1_000_000.0)
                } else if self.value > 1_000 {
                    alloc::format!("{:.2} ms", self.value as f64 / 1_000.0)
                } else {
                    alloc::format!("{} us", self.value)
                }
            }
            CounterUnit::Percent => {
                alloc::format!("{:.1}%", self.value as f64 / 10.0)
            }
        }
    }
}

/// Counter unit
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CounterUnit {
    /// Count
    #[default]
    Count = 0,
    /// Bytes
    Bytes = 1,
    /// Microseconds
    Microseconds = 2,
    /// Percent (value / 10)
    Percent = 3,
}

// ============================================================================
// Profile Report
// ============================================================================

/// Profile report
#[derive(Clone, Debug, Default)]
pub struct ProfileReport {
    /// Frame number
    pub frame_number: u64,
    /// CPU scopes
    pub cpu_scopes: Vec<ProfileScopeResult>,
    /// GPU timings
    pub gpu_timings: Vec<GpuTimingResult>,
    /// GPU pipeline stats
    pub pipeline_stats: GpuPipelineStats,
    /// Memory stats
    pub memory_pools: Vec<MemoryPoolStats>,
    /// Frame stats
    pub frame_stats: FrameStats,
}

impl ProfileReport {
    /// Total CPU time
    pub fn total_cpu_time_us(&self) -> u64 {
        self.cpu_scopes.iter().map(|s| s.duration_us).sum()
    }

    /// Total GPU time
    pub fn total_gpu_time_us(&self) -> u64 {
        self.gpu_timings.iter().map(|t| t.duration_us).sum()
    }

    /// Find scope by name
    pub fn find_cpu_scope(&self, name: &str) -> Option<&ProfileScopeResult> {
        self.cpu_scopes.iter().find(|s| s.name == name)
    }

    /// Find GPU timing by name
    pub fn find_gpu_timing(&self, name: &str) -> Option<&GpuTimingResult> {
        self.gpu_timings.iter().find(|t| t.name == name)
    }
}

// ============================================================================
// Debug Markers
// ============================================================================

/// Debug marker
#[derive(Clone, Debug)]
pub struct DebugMarker {
    /// Name
    pub name: &'static str,
    /// Color
    pub color: [f32; 4],
}

impl DebugMarker {
    /// Creates marker
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// With color
    pub fn with_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.color = [r, g, b, 1.0];
        self
    }

    /// Red
    pub fn red(name: &'static str) -> Self {
        Self::new(name).with_color(1.0, 0.2, 0.2)
    }

    /// Green
    pub fn green(name: &'static str) -> Self {
        Self::new(name).with_color(0.2, 1.0, 0.2)
    }

    /// Blue
    pub fn blue(name: &'static str) -> Self {
        Self::new(name).with_color(0.2, 0.2, 1.0)
    }

    /// Yellow
    pub fn yellow(name: &'static str) -> Self {
        Self::new(name).with_color(1.0, 1.0, 0.2)
    }
}

// ============================================================================
// Profiler GPU Data
// ============================================================================

/// Profiler GPU params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ProfilerGpuParams {
    /// Timestamp period (nanoseconds)
    pub timestamp_period: f32,
    /// Current query index
    pub query_index: u32,
    /// Max queries
    pub max_queries: u32,
    /// Padding
    pub _padding: f32,
}
