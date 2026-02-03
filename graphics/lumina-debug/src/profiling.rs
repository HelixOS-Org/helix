//! GPU Profiling
//!
//! GPU timing and performance analysis.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// GPU Timestamp
// ============================================================================

/// A GPU timestamp.
#[derive(Debug, Clone, Copy, Default)]
pub struct GpuTimestamp {
    /// Timestamp value.
    pub value: u64,
    /// Is valid.
    pub valid: bool,
}

impl GpuTimestamp {
    /// Create a new timestamp.
    pub fn new(value: u64) -> Self {
        Self { value, valid: true }
    }

    /// Create an invalid timestamp.
    pub fn invalid() -> Self {
        Self {
            value: 0,
            valid: false,
        }
    }

    /// Convert to nanoseconds.
    pub fn to_nanos(&self, timestamp_period: f64) -> u64 {
        if self.valid {
            (self.value as f64 * timestamp_period) as u64
        } else {
            0
        }
    }
}

// ============================================================================
// Timer Query
// ============================================================================

/// A timer query result.
#[derive(Debug, Clone)]
pub struct TimerQuery {
    /// Name.
    pub name: String,
    /// Start timestamp.
    pub start: GpuTimestamp,
    /// End timestamp.
    pub end: GpuTimestamp,
    /// Depth (for nested queries).
    pub depth: u32,
    /// Frame index.
    pub frame: u64,
}

impl TimerQuery {
    /// Create a new timer query.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            start: GpuTimestamp::invalid(),
            end: GpuTimestamp::invalid(),
            depth: 0,
            frame: 0,
        }
    }

    /// Check if complete.
    pub fn is_complete(&self) -> bool {
        self.start.valid && self.end.valid
    }

    /// Get duration in timestamp units.
    pub fn duration(&self) -> u64 {
        if self.is_complete() {
            self.end.value.saturating_sub(self.start.value)
        } else {
            0
        }
    }

    /// Get duration in nanoseconds.
    pub fn duration_nanos(&self, timestamp_period: f64) -> u64 {
        (self.duration() as f64 * timestamp_period) as u64
    }

    /// Get duration in milliseconds.
    pub fn duration_ms(&self, timestamp_period: f64) -> f64 {
        self.duration_nanos(timestamp_period) as f64 / 1_000_000.0
    }
}

// ============================================================================
// Timing Result
// ============================================================================

/// Timing result for a scope.
#[derive(Debug, Clone)]
pub struct TimingResult {
    /// Name.
    pub name: String,
    /// Duration in nanoseconds.
    pub duration_nanos: u64,
    /// Sample count.
    pub sample_count: u32,
    /// Min duration.
    pub min_nanos: u64,
    /// Max duration.
    pub max_nanos: u64,
    /// Average duration.
    pub avg_nanos: u64,
}

impl TimingResult {
    /// Create a new timing result.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            duration_nanos: 0,
            sample_count: 0,
            min_nanos: u64::MAX,
            max_nanos: 0,
            avg_nanos: 0,
        }
    }

    /// Add a sample.
    pub fn add_sample(&mut self, nanos: u64) {
        self.duration_nanos = nanos;
        self.sample_count += 1;
        self.min_nanos = self.min_nanos.min(nanos);
        self.max_nanos = self.max_nanos.max(nanos);

        // Running average
        let total = self.avg_nanos as u128 * (self.sample_count - 1) as u128 + nanos as u128;
        self.avg_nanos = (total / self.sample_count as u128) as u64;
    }

    /// Get duration in milliseconds.
    pub fn duration_ms(&self) -> f64 {
        self.duration_nanos as f64 / 1_000_000.0
    }

    /// Get average in milliseconds.
    pub fn avg_ms(&self) -> f64 {
        self.avg_nanos as f64 / 1_000_000.0
    }

    /// Get min in milliseconds.
    pub fn min_ms(&self) -> f64 {
        if self.min_nanos == u64::MAX {
            0.0
        } else {
            self.min_nanos as f64 / 1_000_000.0
        }
    }

    /// Get max in milliseconds.
    pub fn max_ms(&self) -> f64 {
        self.max_nanos as f64 / 1_000_000.0
    }
}

// ============================================================================
// Profile Scope
// ============================================================================

/// A profile scope.
#[derive(Debug, Clone)]
pub struct ProfileScope {
    /// Name.
    pub name: String,
    /// Query index.
    pub query_index: u32,
    /// Depth.
    pub depth: u32,
    /// Is ended.
    pub ended: bool,
}

impl ProfileScope {
    /// Create a new scope.
    pub fn new(name: impl Into<String>, query_index: u32, depth: u32) -> Self {
        Self {
            name: name.into(),
            query_index,
            depth,
            ended: false,
        }
    }
}

// ============================================================================
// Pipeline Statistics
// ============================================================================

/// Pipeline statistics.
#[derive(Debug, Clone, Copy, Default)]
pub struct PipelineStatistics {
    /// Input assembly vertices.
    pub input_assembly_vertices: u64,
    /// Input assembly primitives.
    pub input_assembly_primitives: u64,
    /// Vertex shader invocations.
    pub vertex_shader_invocations: u64,
    /// Geometry shader invocations.
    pub geometry_shader_invocations: u64,
    /// Geometry shader primitives.
    pub geometry_shader_primitives: u64,
    /// Clipping invocations.
    pub clipping_invocations: u64,
    /// Clipping primitives.
    pub clipping_primitives: u64,
    /// Fragment shader invocations.
    pub fragment_shader_invocations: u64,
    /// Tessellation control shader patches.
    pub tessellation_control_shader_patches: u64,
    /// Tessellation evaluation shader invocations.
    pub tessellation_evaluation_shader_invocations: u64,
    /// Compute shader invocations.
    pub compute_shader_invocations: u64,
    /// Task shader invocations.
    pub task_shader_invocations: u64,
    /// Mesh shader invocations.
    pub mesh_shader_invocations: u64,
}

impl PipelineStatistics {
    /// Get vertex amplification ratio.
    pub fn vertex_amplification(&self) -> f32 {
        if self.input_assembly_vertices == 0 {
            0.0
        } else {
            self.vertex_shader_invocations as f32 / self.input_assembly_vertices as f32
        }
    }

    /// Get overdraw ratio.
    pub fn overdraw(&self, pixel_count: u64) -> f32 {
        if pixel_count == 0 {
            0.0
        } else {
            self.fragment_shader_invocations as f32 / pixel_count as f32
        }
    }
}

// ============================================================================
// GPU Timer
// ============================================================================

/// A GPU timer for timing operations.
pub struct GpuTimer {
    /// Timer name.
    pub name: String,
    /// Accumulated time in nanoseconds.
    accumulated_nanos: AtomicU64,
    /// Sample count.
    sample_count: AtomicU64,
    /// Min time.
    min_nanos: AtomicU64,
    /// Max time.
    max_nanos: AtomicU64,
}

impl GpuTimer {
    /// Create a new GPU timer.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            accumulated_nanos: AtomicU64::new(0),
            sample_count: AtomicU64::new(0),
            min_nanos: AtomicU64::new(u64::MAX),
            max_nanos: AtomicU64::new(0),
        }
    }

    /// Add a sample.
    pub fn add_sample(&self, nanos: u64) {
        self.accumulated_nanos.fetch_add(nanos, Ordering::Relaxed);
        self.sample_count.fetch_add(1, Ordering::Relaxed);

        // Update min/max (not perfectly atomic but good enough)
        let mut current_min = self.min_nanos.load(Ordering::Relaxed);
        while nanos < current_min {
            match self.min_nanos.compare_exchange_weak(
                current_min,
                nanos,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(v) => current_min = v,
            }
        }

        let mut current_max = self.max_nanos.load(Ordering::Relaxed);
        while nanos > current_max {
            match self.max_nanos.compare_exchange_weak(
                current_max,
                nanos,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(v) => current_max = v,
            }
        }
    }

    /// Get timing result.
    pub fn result(&self) -> TimingResult {
        let accumulated = self.accumulated_nanos.load(Ordering::Relaxed);
        let count = self.sample_count.load(Ordering::Relaxed);
        let min = self.min_nanos.load(Ordering::Relaxed);
        let max = self.max_nanos.load(Ordering::Relaxed);

        TimingResult {
            name: self.name.clone(),
            duration_nanos: if count > 0 { accumulated / count } else { 0 },
            sample_count: count as u32,
            min_nanos: min,
            max_nanos: max,
            avg_nanos: if count > 0 { accumulated / count } else { 0 },
        }
    }

    /// Reset the timer.
    pub fn reset(&self) {
        self.accumulated_nanos.store(0, Ordering::Relaxed);
        self.sample_count.store(0, Ordering::Relaxed);
        self.min_nanos.store(u64::MAX, Ordering::Relaxed);
        self.max_nanos.store(0, Ordering::Relaxed);
    }
}

// ============================================================================
// Profiler Statistics
// ============================================================================

/// Profiler statistics.
#[derive(Debug, Clone, Default)]
pub struct ProfilerStatistics {
    /// Total queries.
    pub total_queries: u32,
    /// Active queries.
    pub active_queries: u32,
    /// Completed queries.
    pub completed_queries: u32,
    /// Total frame time (ms).
    pub total_frame_time_ms: f64,
    /// GPU time (ms).
    pub gpu_time_ms: f64,
}

// ============================================================================
// GPU Profiler
// ============================================================================

/// GPU profiler for performance analysis.
pub struct GpuProfiler {
    /// Is enabled.
    pub enabled: bool,
    /// Timer queries.
    queries: Vec<TimerQuery>,
    /// Timing results.
    results: BTreeMap<String, TimingResult>,
    /// Active scopes.
    active_scopes: Vec<ProfileScope>,
    /// Current depth.
    current_depth: u32,
    /// Current frame.
    current_frame: u64,
    /// Timestamp period (ns per tick).
    pub timestamp_period: f64,
    /// Statistics.
    pub stats: ProfilerStatistics,
    /// Max queries per frame.
    pub max_queries_per_frame: u32,
}

impl GpuProfiler {
    /// Create a new profiler.
    pub fn new() -> Self {
        Self {
            enabled: true,
            queries: Vec::new(),
            results: BTreeMap::new(),
            active_scopes: Vec::new(),
            current_depth: 0,
            current_frame: 0,
            timestamp_period: 1.0, // Default 1ns per tick
            stats: ProfilerStatistics::default(),
            max_queries_per_frame: 256,
        }
    }

    /// Enable profiling.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable profiling.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Set timestamp period.
    pub fn set_timestamp_period(&mut self, period: f64) {
        self.timestamp_period = period;
    }

    /// Begin a frame.
    pub fn begin_frame(&mut self, frame_index: u64) {
        if !self.enabled {
            return;
        }

        self.current_frame = frame_index;
        self.queries.clear();
        self.active_scopes.clear();
        self.current_depth = 0;
    }

    /// End a frame.
    pub fn end_frame(&mut self) {
        if !self.enabled {
            return;
        }

        // Process completed queries
        for query in &self.queries {
            if query.is_complete() {
                let nanos = query.duration_nanos(self.timestamp_period);

                let result = self
                    .results
                    .entry(query.name.clone())
                    .or_insert_with(|| TimingResult::new(&query.name));

                result.add_sample(nanos);
            }
        }

        // Update statistics
        self.stats.total_queries = self.queries.len() as u32;
        self.stats.completed_queries =
            self.queries.iter().filter(|q| q.is_complete()).count() as u32;
    }

    /// Begin a timing scope.
    pub fn begin_scope(&mut self, name: impl Into<String>) -> u32 {
        if !self.enabled {
            return u32::MAX;
        }

        if self.queries.len() >= self.max_queries_per_frame as usize {
            return u32::MAX;
        }

        let query_index = self.queries.len() as u32;
        let name = name.into();

        let mut query = TimerQuery::new(&name);
        query.depth = self.current_depth;
        query.frame = self.current_frame;
        self.queries.push(query);

        let scope = ProfileScope::new(&name, query_index, self.current_depth);
        self.active_scopes.push(scope);

        self.current_depth += 1;
        self.stats.active_queries += 1;

        query_index
    }

    /// End a timing scope.
    pub fn end_scope(&mut self, query_index: u32) {
        if !self.enabled || query_index == u32::MAX {
            return;
        }

        if let Some(scope) = self.active_scopes.pop() {
            if scope.query_index == query_index {
                self.current_depth = self.current_depth.saturating_sub(1);
                self.stats.active_queries = self.stats.active_queries.saturating_sub(1);
            }
        }
    }

    /// Set query timestamps.
    pub fn set_timestamps(&mut self, query_index: u32, start: u64, end: u64) {
        if let Some(query) = self.queries.get_mut(query_index as usize) {
            query.start = GpuTimestamp::new(start);
            query.end = GpuTimestamp::new(end);
        }
    }

    /// Get timing result.
    pub fn get_result(&self, name: &str) -> Option<&TimingResult> {
        self.results.get(name)
    }

    /// Get all results.
    pub fn all_results(&self) -> impl Iterator<Item = (&String, &TimingResult)> {
        self.results.iter()
    }

    /// Get frame queries.
    pub fn frame_queries(&self) -> &[TimerQuery] {
        &self.queries
    }

    /// Reset all results.
    pub fn reset(&mut self) {
        self.results.clear();
        self.stats = ProfilerStatistics::default();
    }

    /// Get statistics.
    pub fn statistics(&self) -> &ProfilerStatistics {
        &self.stats
    }
}

impl Default for GpuProfiler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Scoped Timer
// ============================================================================

/// RAII scoped timer.
pub struct ScopedTimer<'a> {
    profiler: &'a mut GpuProfiler,
    query_index: u32,
}

impl<'a> ScopedTimer<'a> {
    /// Create a new scoped timer.
    pub fn new(profiler: &'a mut GpuProfiler, name: &str) -> Self {
        let query_index = profiler.begin_scope(name);
        Self {
            profiler,
            query_index,
        }
    }
}

impl<'a> Drop for ScopedTimer<'a> {
    fn drop(&mut self) {
        self.profiler.end_scope(self.query_index);
    }
}

// ============================================================================
// Frame Graph Profiler
// ============================================================================

/// Profiler for frame graph passes.
#[derive(Debug, Clone)]
pub struct PassTiming {
    /// Pass name.
    pub name: String,
    /// GPU time in nanoseconds.
    pub gpu_nanos: u64,
    /// CPU time in nanoseconds.
    pub cpu_nanos: u64,
    /// Draw call count.
    pub draw_calls: u32,
    /// Dispatch count.
    pub dispatches: u32,
}

impl PassTiming {
    /// Create new pass timing.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            gpu_nanos: 0,
            cpu_nanos: 0,
            draw_calls: 0,
            dispatches: 0,
        }
    }

    /// GPU time in milliseconds.
    pub fn gpu_ms(&self) -> f64 {
        self.gpu_nanos as f64 / 1_000_000.0
    }

    /// CPU time in milliseconds.
    pub fn cpu_ms(&self) -> f64 {
        self.cpu_nanos as f64 / 1_000_000.0
    }
}

/// Frame profiler for tracking pass timings.
pub struct FrameProfiler {
    /// Pass timings.
    passes: Vec<PassTiming>,
    /// Total GPU time.
    pub total_gpu_nanos: u64,
    /// Total CPU time.
    pub total_cpu_nanos: u64,
}

impl FrameProfiler {
    /// Create a new frame profiler.
    pub fn new() -> Self {
        Self {
            passes: Vec::new(),
            total_gpu_nanos: 0,
            total_cpu_nanos: 0,
        }
    }

    /// Add pass timing.
    pub fn add_pass(&mut self, timing: PassTiming) {
        self.total_gpu_nanos += timing.gpu_nanos;
        self.total_cpu_nanos += timing.cpu_nanos;
        self.passes.push(timing);
    }

    /// Get passes.
    pub fn passes(&self) -> &[PassTiming] {
        &self.passes
    }

    /// Reset for new frame.
    pub fn reset(&mut self) {
        self.passes.clear();
        self.total_gpu_nanos = 0;
        self.total_cpu_nanos = 0;
    }

    /// Total GPU time in milliseconds.
    pub fn total_gpu_ms(&self) -> f64 {
        self.total_gpu_nanos as f64 / 1_000_000.0
    }

    /// Total CPU time in milliseconds.
    pub fn total_cpu_ms(&self) -> f64 {
        self.total_cpu_nanos as f64 / 1_000_000.0
    }
}

impl Default for FrameProfiler {
    fn default() -> Self {
        Self::new()
    }
}
