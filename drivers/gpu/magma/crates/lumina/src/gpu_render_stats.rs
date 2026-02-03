//! GPU Render Statistics System for Lumina
//!
//! This module provides comprehensive GPU render statistics collection,
//! performance monitoring, and profiling infrastructure.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Statistics System Handles
// ============================================================================

/// GPU statistics system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuStatsSystemHandle(pub u64);

impl GpuStatsSystemHandle {
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

impl Default for GpuStatsSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Stats query handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct StatsQueryHandle(pub u64);

impl StatsQueryHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for StatsQueryHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Performance counter handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PerfCounterHandle(pub u64);

impl PerfCounterHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for PerfCounterHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Stats region handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct StatsRegionHandle(pub u64);

impl StatsRegionHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for StatsRegionHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Statistics System Creation
// ============================================================================

/// GPU statistics system create info
#[derive(Clone, Debug)]
pub struct GpuStatsSystemCreateInfo {
    /// Name
    pub name: String,
    /// History size (frames)
    pub history_size: u32,
    /// Max queries per frame
    pub max_queries: u32,
    /// Max regions
    pub max_regions: u32,
    /// Features
    pub features: StatsFeatures,
}

impl GpuStatsSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            history_size: 120,
            max_queries: 256,
            max_regions: 64,
            features: StatsFeatures::all(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With history size
    pub fn with_history_size(mut self, size: u32) -> Self {
        self.history_size = size;
        self
    }

    /// With max queries
    pub fn with_max_queries(mut self, count: u32) -> Self {
        self.max_queries = count;
        self
    }

    /// With max regions
    pub fn with_max_regions(mut self, count: u32) -> Self {
        self.max_regions = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: StatsFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Standard
    pub fn standard() -> Self {
        Self::new()
    }

    /// Detailed profiling
    pub fn detailed() -> Self {
        Self::new()
            .with_history_size(300)
            .with_max_queries(1024)
            .with_max_regions(256)
    }

    /// Minimal overhead
    pub fn minimal() -> Self {
        Self::new()
            .with_history_size(30)
            .with_max_queries(32)
            .with_max_regions(16)
    }
}

impl Default for GpuStatsSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Statistics features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct StatsFeatures: u32 {
        /// None
        const NONE = 0;
        /// GPU timing
        const GPU_TIMING = 1 << 0;
        /// Pipeline statistics
        const PIPELINE_STATS = 1 << 1;
        /// Memory tracking
        const MEMORY = 1 << 2;
        /// Draw call tracking
        const DRAW_CALLS = 1 << 3;
        /// Bandwidth estimation
        const BANDWIDTH = 1 << 4;
        /// Shader statistics
        const SHADER_STATS = 1 << 5;
        /// Async compute
        const ASYNC_COMPUTE = 1 << 6;
        /// History tracking
        const HISTORY = 1 << 7;
        /// All
        const ALL = 0xFF;
    }
}

impl Default for StatsFeatures {
    fn default() -> Self {
        Self::all()
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
    /// Frame time (ms)
    pub frame_time_ms: f32,
    /// GPU time (ms)
    pub gpu_time_ms: f32,
    /// CPU time (ms)
    pub cpu_time_ms: f32,
    /// Present time (ms)
    pub present_time_ms: f32,
    /// Draw calls
    pub draw_calls: u32,
    /// Dispatch calls
    pub dispatch_calls: u32,
    /// Triangles
    pub triangles: u64,
    /// Vertices
    pub vertices: u64,
    /// Primitives
    pub primitives: u64,
    /// Pipeline switches
    pub pipeline_switches: u32,
    /// Descriptor updates
    pub descriptor_updates: u32,
    /// Buffer uploads
    pub buffer_uploads: u32,
    /// Texture uploads
    pub texture_uploads: u32,
}

impl FrameStats {
    /// Average FPS
    pub fn fps(&self) -> f32 {
        if self.frame_time_ms > 0.0 {
            1000.0 / self.frame_time_ms
        } else {
            0.0
        }
    }

    /// CPU bound
    pub fn is_cpu_bound(&self) -> bool {
        self.cpu_time_ms > self.gpu_time_ms * 1.1
    }

    /// GPU bound
    pub fn is_gpu_bound(&self) -> bool {
        self.gpu_time_ms > self.cpu_time_ms * 1.1
    }

    /// Triangles per draw call
    pub fn triangles_per_draw(&self) -> f32 {
        if self.draw_calls > 0 {
            self.triangles as f32 / self.draw_calls as f32
        } else {
            0.0
        }
    }
}

// ============================================================================
// Memory Statistics
// ============================================================================

/// Memory statistics
#[derive(Clone, Debug, Default)]
pub struct MemoryStats {
    /// Total GPU memory (bytes)
    pub total_memory: u64,
    /// Used GPU memory (bytes)
    pub used_memory: u64,
    /// Available GPU memory (bytes)
    pub available_memory: u64,
    /// Buffer memory (bytes)
    pub buffer_memory: u64,
    /// Texture memory (bytes)
    pub texture_memory: u64,
    /// Render target memory (bytes)
    pub render_target_memory: u64,
    /// Shader memory (bytes)
    pub shader_memory: u64,
    /// Staging memory (bytes)
    pub staging_memory: u64,
    /// Peak memory (bytes)
    pub peak_memory: u64,
    /// Allocations this frame
    pub allocations: u32,
    /// Deallocations this frame
    pub deallocations: u32,
}

impl MemoryStats {
    /// Used percentage
    pub fn used_percentage(&self) -> f32 {
        if self.total_memory > 0 {
            (self.used_memory as f64 / self.total_memory as f64 * 100.0) as f32
        } else {
            0.0
        }
    }

    /// Used memory in MB
    pub fn used_mb(&self) -> f32 {
        self.used_memory as f32 / (1024.0 * 1024.0)
    }

    /// Total memory in MB
    pub fn total_mb(&self) -> f32 {
        self.total_memory as f32 / (1024.0 * 1024.0)
    }

    /// Buffer memory in MB
    pub fn buffer_mb(&self) -> f32 {
        self.buffer_memory as f32 / (1024.0 * 1024.0)
    }

    /// Texture memory in MB
    pub fn texture_mb(&self) -> f32 {
        self.texture_memory as f32 / (1024.0 * 1024.0)
    }
}

// ============================================================================
// Pipeline Statistics
// ============================================================================

/// Pipeline statistics
#[derive(Clone, Debug, Default)]
pub struct PipelineStats {
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
    pub clipping_invocations: u64,
    /// Clipping primitives
    pub clipping_primitives: u64,
    /// Fragment shader invocations
    pub fs_invocations: u64,
    /// Tessellation control shader patches
    pub tcs_patches: u64,
    /// Tessellation evaluation shader invocations
    pub tes_invocations: u64,
    /// Compute shader invocations
    pub cs_invocations: u64,
}

impl PipelineStats {
    /// Vertex reuse ratio
    pub fn vertex_reuse_ratio(&self) -> f32 {
        if self.ia_vertices > 0 && self.vs_invocations > 0 {
            self.ia_vertices as f32 / self.vs_invocations as f32
        } else {
            0.0
        }
    }

    /// Overdraw estimate
    pub fn overdraw_estimate(&self, pixels: u64) -> f32 {
        if pixels > 0 {
            self.fs_invocations as f32 / pixels as f32
        } else {
            0.0
        }
    }

    /// Tessellation factor
    pub fn tessellation_factor(&self) -> f32 {
        if self.tcs_patches > 0 && self.tes_invocations > 0 {
            self.tes_invocations as f32 / self.tcs_patches as f32
        } else {
            0.0
        }
    }
}

// ============================================================================
// Bandwidth Statistics
// ============================================================================

/// Bandwidth statistics
#[derive(Clone, Debug, Default)]
pub struct BandwidthStats {
    /// Read bandwidth (bytes/s)
    pub read_bandwidth: u64,
    /// Write bandwidth (bytes/s)
    pub write_bandwidth: u64,
    /// Texture read bandwidth
    pub texture_read: u64,
    /// Render target write bandwidth
    pub render_target_write: u64,
    /// Buffer read bandwidth
    pub buffer_read: u64,
    /// Buffer write bandwidth
    pub buffer_write: u64,
    /// Transfer bandwidth (CPU-GPU)
    pub transfer_bandwidth: u64,
}

impl BandwidthStats {
    /// Total bandwidth
    pub fn total(&self) -> u64 {
        self.read_bandwidth + self.write_bandwidth
    }

    /// Total in GB/s
    pub fn total_gbps(&self) -> f32 {
        self.total() as f32 / (1024.0 * 1024.0 * 1024.0)
    }

    /// Read in GB/s
    pub fn read_gbps(&self) -> f32 {
        self.read_bandwidth as f32 / (1024.0 * 1024.0 * 1024.0)
    }

    /// Write in GB/s
    pub fn write_gbps(&self) -> f32 {
        self.write_bandwidth as f32 / (1024.0 * 1024.0 * 1024.0)
    }
}

// ============================================================================
// Region Statistics
// ============================================================================

/// Stats region create info
#[derive(Clone, Debug)]
pub struct StatsRegionCreateInfo {
    /// Name
    pub name: String,
    /// Category
    pub category: StatsCategory,
    /// Color (for visualization)
    pub color: [f32; 4],
}

impl StatsRegionCreateInfo {
    /// Creates new info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            category: StatsCategory::General,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// With category
    pub fn with_category(mut self, category: StatsCategory) -> Self {
        self.category = category;
        self
    }

    /// With color
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// Shadow pass region
    pub fn shadow_pass() -> Self {
        Self::new("Shadow Pass")
            .with_category(StatsCategory::Shadows)
            .with_color([0.3, 0.3, 0.5, 1.0])
    }

    /// GBuffer region
    pub fn gbuffer() -> Self {
        Self::new("GBuffer")
            .with_category(StatsCategory::Geometry)
            .with_color([0.5, 0.8, 0.3, 1.0])
    }

    /// Lighting region
    pub fn lighting() -> Self {
        Self::new("Lighting")
            .with_category(StatsCategory::Lighting)
            .with_color([1.0, 0.9, 0.3, 1.0])
    }

    /// Post-processing region
    pub fn post_process() -> Self {
        Self::new("Post Process")
            .with_category(StatsCategory::PostProcess)
            .with_color([0.3, 0.7, 1.0, 1.0])
    }

    /// UI region
    pub fn ui() -> Self {
        Self::new("UI")
            .with_category(StatsCategory::UI)
            .with_color([1.0, 0.5, 0.3, 1.0])
    }
}

impl Default for StatsRegionCreateInfo {
    fn default() -> Self {
        Self::new("Region")
    }
}

/// Statistics category
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StatsCategory {
    /// General
    #[default]
    General = 0,
    /// Geometry
    Geometry = 1,
    /// Shadows
    Shadows = 2,
    /// Lighting
    Lighting = 3,
    /// Post-processing
    PostProcess = 4,
    /// UI
    UI = 5,
    /// Compute
    Compute = 6,
    /// Transfer
    Transfer = 7,
}

/// Region statistics
#[derive(Clone, Debug, Default)]
pub struct RegionStats {
    /// Region name
    pub name: String,
    /// GPU time (ms)
    pub gpu_time_ms: f32,
    /// Draw calls
    pub draw_calls: u32,
    /// Triangles
    pub triangles: u64,
    /// Dispatch calls
    pub dispatch_calls: u32,
}

// ============================================================================
// History
// ============================================================================

/// Statistics history
#[derive(Clone, Debug, Default)]
pub struct StatsHistory {
    /// Frame times (ms)
    pub frame_times: Vec<f32>,
    /// GPU times (ms)
    pub gpu_times: Vec<f32>,
    /// CPU times (ms)
    pub cpu_times: Vec<f32>,
    /// Draw call counts
    pub draw_calls: Vec<u32>,
    /// Triangle counts
    pub triangles: Vec<u64>,
    /// Memory usage (bytes)
    pub memory_usage: Vec<u64>,
}

impl StatsHistory {
    /// Creates new history with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            frame_times: Vec::with_capacity(capacity),
            gpu_times: Vec::with_capacity(capacity),
            cpu_times: Vec::with_capacity(capacity),
            draw_calls: Vec::with_capacity(capacity),
            triangles: Vec::with_capacity(capacity),
            memory_usage: Vec::with_capacity(capacity),
        }
    }

    /// Average frame time
    pub fn avg_frame_time(&self) -> f32 {
        if self.frame_times.is_empty() {
            0.0
        } else {
            self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32
        }
    }

    /// Average GPU time
    pub fn avg_gpu_time(&self) -> f32 {
        if self.gpu_times.is_empty() {
            0.0
        } else {
            self.gpu_times.iter().sum::<f32>() / self.gpu_times.len() as f32
        }
    }

    /// Max frame time
    pub fn max_frame_time(&self) -> f32 {
        self.frame_times.iter().cloned().fold(0.0, f32::max)
    }

    /// Min frame time
    pub fn min_frame_time(&self) -> f32 {
        self.frame_times.iter().cloned().fold(f32::MAX, f32::min)
    }

    /// Average FPS
    pub fn avg_fps(&self) -> f32 {
        let avg = self.avg_frame_time();
        if avg > 0.0 {
            1000.0 / avg
        } else {
            0.0
        }
    }

    /// 1% low FPS
    pub fn percentile_1_fps(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        let mut sorted = self.frame_times.clone();
        sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(core::cmp::Ordering::Equal));
        let idx = (sorted.len() as f32 * 0.01) as usize;
        let worst_1_percent = sorted.get(idx).copied().unwrap_or(0.0);
        if worst_1_percent > 0.0 {
            1000.0 / worst_1_percent
        } else {
            0.0
        }
    }
}

// ============================================================================
// Alerts
// ============================================================================

/// Performance alert
#[derive(Clone, Debug)]
pub struct PerformanceAlert {
    /// Alert type
    pub alert_type: AlertType,
    /// Severity
    pub severity: AlertSeverity,
    /// Message
    pub message: String,
    /// Current value
    pub value: f32,
    /// Threshold
    pub threshold: f32,
}

impl PerformanceAlert {
    /// Creates new alert
    pub fn new(alert_type: AlertType, message: impl Into<String>) -> Self {
        Self {
            alert_type,
            severity: AlertSeverity::Warning,
            message: message.into(),
            value: 0.0,
            threshold: 0.0,
        }
    }

    /// With severity
    pub fn with_severity(mut self, severity: AlertSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// With values
    pub fn with_values(mut self, value: f32, threshold: f32) -> Self {
        self.value = value;
        self.threshold = threshold;
        self
    }

    /// Low FPS alert
    pub fn low_fps(fps: f32, threshold: f32) -> Self {
        Self::new(AlertType::LowFps, "FPS below threshold")
            .with_severity(if fps < threshold / 2.0 {
                AlertSeverity::Critical
            } else {
                AlertSeverity::Warning
            })
            .with_values(fps, threshold)
    }

    /// High GPU time alert
    pub fn high_gpu_time(time_ms: f32, threshold_ms: f32) -> Self {
        Self::new(AlertType::HighGpuTime, "GPU time exceeded threshold")
            .with_severity(AlertSeverity::Warning)
            .with_values(time_ms, threshold_ms)
    }

    /// Memory pressure alert
    pub fn memory_pressure(used_percent: f32) -> Self {
        Self::new(AlertType::MemoryPressure, "GPU memory pressure detected")
            .with_severity(if used_percent > 95.0 {
                AlertSeverity::Critical
            } else {
                AlertSeverity::Warning
            })
            .with_values(used_percent, 90.0)
    }
}

/// Alert type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AlertType {
    /// Low FPS
    #[default]
    LowFps = 0,
    /// High GPU time
    HighGpuTime = 1,
    /// High CPU time
    HighCpuTime = 2,
    /// Memory pressure
    MemoryPressure = 3,
    /// High draw calls
    HighDrawCalls = 4,
    /// Bandwidth limit
    BandwidthLimit = 5,
    /// Frame stutter
    FrameStutter = 6,
}

/// Alert severity
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AlertSeverity {
    /// Info
    Info = 0,
    /// Warning
    #[default]
    Warning = 1,
    /// Critical
    Critical = 2,
}

// ============================================================================
// GPU Structures
// ============================================================================

/// GPU stats query data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuStatsQueryData {
    /// Timestamp start
    pub timestamp_start: u64,
    /// Timestamp end
    pub timestamp_end: u64,
    /// Pipeline stats offset
    pub pipeline_stats_offset: u32,
    /// Query type
    pub query_type: u32,
}

/// GPU timestamp query
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuTimestampQuery {
    /// Timestamp value
    pub timestamp: u64,
    /// Query valid
    pub valid: u32,
    /// Padding
    pub _pad: u32,
}

// ============================================================================
// Aggregate Statistics
// ============================================================================

/// Aggregate render statistics
#[derive(Clone, Debug, Default)]
pub struct AggregateStats {
    /// Frame stats
    pub frame: FrameStats,
    /// Memory stats
    pub memory: MemoryStats,
    /// Pipeline stats
    pub pipeline: PipelineStats,
    /// Bandwidth stats
    pub bandwidth: BandwidthStats,
    /// Region stats
    pub regions: Vec<RegionStats>,
    /// History
    pub history: StatsHistory,
    /// Active alerts
    pub alerts: Vec<PerformanceAlert>,
}

impl AggregateStats {
    /// Check for performance issues
    pub fn check_performance(&mut self, fps_threshold: f32, gpu_time_threshold: f32) {
        self.alerts.clear();

        let fps = self.frame.fps();
        if fps < fps_threshold {
            self.alerts.push(PerformanceAlert::low_fps(fps, fps_threshold));
        }

        if self.frame.gpu_time_ms > gpu_time_threshold {
            self.alerts.push(PerformanceAlert::high_gpu_time(
                self.frame.gpu_time_ms,
                gpu_time_threshold,
            ));
        }

        let memory_percent = self.memory.used_percentage();
        if memory_percent > 90.0 {
            self.alerts.push(PerformanceAlert::memory_pressure(memory_percent));
        }
    }

    /// Has critical alerts
    pub fn has_critical_alerts(&self) -> bool {
        self.alerts.iter().any(|a| a.severity == AlertSeverity::Critical)
    }
}
