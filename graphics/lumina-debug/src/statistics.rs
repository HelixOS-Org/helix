//! Debug Statistics
//!
//! Resource and performance statistics collection.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

// ============================================================================
// Resource Statistics
// ============================================================================

/// Resource statistics.
#[derive(Debug, Clone, Default)]
pub struct ResourceStatistics {
    /// Buffer count.
    pub buffer_count: u32,
    /// Image count.
    pub image_count: u32,
    /// Sampler count.
    pub sampler_count: u32,
    /// Pipeline count.
    pub pipeline_count: u32,
    /// Descriptor set count.
    pub descriptor_set_count: u32,
    /// Render pass count.
    pub render_pass_count: u32,
    /// Framebuffer count.
    pub framebuffer_count: u32,
    /// Command buffer count.
    pub command_buffer_count: u32,
    /// Shader module count.
    pub shader_module_count: u32,
    /// Query pool count.
    pub query_pool_count: u32,
    /// Fence count.
    pub fence_count: u32,
    /// Semaphore count.
    pub semaphore_count: u32,
}

impl ResourceStatistics {
    /// Total resource count.
    pub fn total(&self) -> u32 {
        self.buffer_count
            + self.image_count
            + self.sampler_count
            + self.pipeline_count
            + self.descriptor_set_count
            + self.render_pass_count
            + self.framebuffer_count
            + self.command_buffer_count
            + self.shader_module_count
            + self.query_pool_count
            + self.fence_count
            + self.semaphore_count
    }
}

// ============================================================================
// Memory Statistics
// ============================================================================

/// Memory statistics.
#[derive(Debug, Clone, Default)]
pub struct MemoryStatistics {
    /// Total allocated memory.
    pub total_allocated: u64,
    /// Device local memory.
    pub device_local_memory: u64,
    /// Host visible memory.
    pub host_visible_memory: u64,
    /// Buffer memory.
    pub buffer_memory: u64,
    /// Image memory.
    pub image_memory: u64,
    /// Staging memory.
    pub staging_memory: u64,
    /// Allocation count.
    pub allocation_count: u32,
    /// Peak memory.
    pub peak_memory: u64,
    /// Block count.
    pub block_count: u32,
    /// Fragmentation ratio.
    pub fragmentation: f32,
}

impl MemoryStatistics {
    /// Get utilization ratio.
    pub fn utilization(&self) -> f32 {
        if self.total_allocated == 0 {
            0.0
        } else {
            (self.buffer_memory + self.image_memory) as f32 / self.total_allocated as f32
        }
    }
}

// ============================================================================
// Draw Statistics
// ============================================================================

/// Draw call statistics.
#[derive(Debug, Clone, Default)]
pub struct DrawStatistics {
    /// Draw calls.
    pub draw_calls: u32,
    /// Indexed draw calls.
    pub indexed_draw_calls: u32,
    /// Indirect draw calls.
    pub indirect_draw_calls: u32,
    /// Instanced draw calls.
    pub instanced_draw_calls: u32,
    /// Total instances.
    pub total_instances: u32,
    /// Total vertices.
    pub total_vertices: u64,
    /// Total primitives.
    pub total_primitives: u64,
    /// Dispatches.
    pub dispatches: u32,
    /// Indirect dispatches.
    pub indirect_dispatches: u32,
    /// Total workgroups.
    pub total_workgroups: u64,
    /// Ray trace dispatches.
    pub ray_trace_dispatches: u32,
}

impl DrawStatistics {
    /// Total draw calls (all types).
    pub fn total_draw_calls(&self) -> u32 {
        self.draw_calls + self.indexed_draw_calls + self.indirect_draw_calls
    }

    /// Total dispatches (all types).
    pub fn total_dispatches(&self) -> u32 {
        self.dispatches + self.indirect_dispatches
    }
}

// ============================================================================
// Frame Statistics
// ============================================================================

/// Per-frame statistics.
#[derive(Debug, Clone, Default)]
pub struct FrameStatistics {
    /// Frame index.
    pub frame_index: u64,
    /// Frame time (ms).
    pub frame_time_ms: f64,
    /// GPU time (ms).
    pub gpu_time_ms: f64,
    /// CPU time (ms).
    pub cpu_time_ms: f64,
    /// Draw statistics.
    pub draw: DrawStatistics,
    /// Resource statistics.
    pub resources: ResourceStatistics,
    /// Memory statistics.
    pub memory: MemoryStatistics,
    /// Pipeline binds.
    pub pipeline_binds: u32,
    /// Descriptor set binds.
    pub descriptor_binds: u32,
    /// Vertex buffer binds.
    pub vertex_buffer_binds: u32,
    /// Index buffer binds.
    pub index_buffer_binds: u32,
    /// Push constant updates.
    pub push_constant_updates: u32,
    /// Render passes.
    pub render_passes: u32,
    /// Buffer uploads.
    pub buffer_uploads: u32,
    /// Buffer upload bytes.
    pub buffer_upload_bytes: u64,
    /// Image uploads.
    pub image_uploads: u32,
    /// Image upload bytes.
    pub image_upload_bytes: u64,
}

impl FrameStatistics {
    /// Create for a frame.
    pub fn new(frame_index: u64) -> Self {
        Self {
            frame_index,
            ..Default::default()
        }
    }

    /// Total binds.
    pub fn total_binds(&self) -> u32 {
        self.pipeline_binds
            + self.descriptor_binds
            + self.vertex_buffer_binds
            + self.index_buffer_binds
    }

    /// Total uploads.
    pub fn total_uploads(&self) -> u32 {
        self.buffer_uploads + self.image_uploads
    }

    /// Total upload bytes.
    pub fn total_upload_bytes(&self) -> u64 {
        self.buffer_upload_bytes + self.image_upload_bytes
    }
}

// ============================================================================
// Statistics Snapshot
// ============================================================================

/// Snapshot of statistics at a point in time.
#[derive(Debug, Clone, Default)]
pub struct StatisticsSnapshot {
    /// Frame index.
    pub frame_index: u64,
    /// Timestamp.
    pub timestamp: u64,
    /// Frame statistics.
    pub frame: FrameStatistics,
    /// Average frame time (ms).
    pub avg_frame_time_ms: f64,
    /// Min frame time (ms).
    pub min_frame_time_ms: f64,
    /// Max frame time (ms).
    pub max_frame_time_ms: f64,
    /// FPS.
    pub fps: f64,
    /// Average FPS.
    pub avg_fps: f64,
}

impl StatisticsSnapshot {
    /// Create a new snapshot.
    pub fn new(frame_index: u64) -> Self {
        Self {
            frame_index,
            ..Default::default()
        }
    }
}

// ============================================================================
// Statistics Collector
// ============================================================================

/// Collector for gathering statistics.
pub struct StatisticsCollector {
    /// Is enabled.
    pub enabled: bool,
    /// Current frame statistics.
    current_frame: FrameStatistics,
    /// Frame history.
    history: Vec<FrameStatistics>,
    /// Maximum history size.
    pub max_history: usize,
    /// Current frame index.
    frame_index: u64,
    /// Accumulated frame time.
    accumulated_frame_time: f64,
    /// Min frame time.
    min_frame_time: f64,
    /// Max frame time.
    max_frame_time: f64,
    /// Frame count for averaging.
    frame_count: u32,
}

impl StatisticsCollector {
    /// Create a new collector.
    pub fn new() -> Self {
        Self {
            enabled: true,
            current_frame: FrameStatistics::default(),
            history: Vec::new(),
            max_history: 120, // ~2 seconds at 60fps
            frame_index: 0,
            accumulated_frame_time: 0.0,
            min_frame_time: f64::MAX,
            max_frame_time: 0.0,
            frame_count: 0,
        }
    }

    /// Enable collection.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable collection.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Begin a frame.
    pub fn begin_frame(&mut self, frame_index: u64) {
        if !self.enabled {
            return;
        }

        self.frame_index = frame_index;
        self.current_frame = FrameStatistics::new(frame_index);
    }

    /// End a frame.
    pub fn end_frame(&mut self) {
        if !self.enabled {
            return;
        }

        // Update statistics
        self.accumulated_frame_time += self.current_frame.frame_time_ms;
        self.min_frame_time = self.min_frame_time.min(self.current_frame.frame_time_ms);
        self.max_frame_time = self.max_frame_time.max(self.current_frame.frame_time_ms);
        self.frame_count += 1;

        // Store in history
        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(self.current_frame.clone());
    }

    /// Record frame time.
    pub fn record_frame_time(&mut self, frame_time_ms: f64) {
        self.current_frame.frame_time_ms = frame_time_ms;
    }

    /// Record GPU time.
    pub fn record_gpu_time(&mut self, gpu_time_ms: f64) {
        self.current_frame.gpu_time_ms = gpu_time_ms;
    }

    /// Record CPU time.
    pub fn record_cpu_time(&mut self, cpu_time_ms: f64) {
        self.current_frame.cpu_time_ms = cpu_time_ms;
    }

    /// Record a draw call.
    pub fn record_draw(&mut self, vertices: u32, instances: u32) {
        self.current_frame.draw.draw_calls += 1;
        self.current_frame.draw.total_vertices += vertices as u64;
        self.current_frame.draw.total_instances += instances;
        if instances > 1 {
            self.current_frame.draw.instanced_draw_calls += 1;
        }
    }

    /// Record an indexed draw call.
    pub fn record_draw_indexed(&mut self, indices: u32, instances: u32) {
        self.current_frame.draw.indexed_draw_calls += 1;
        self.current_frame.draw.total_vertices += indices as u64;
        self.current_frame.draw.total_instances += instances;
        if instances > 1 {
            self.current_frame.draw.instanced_draw_calls += 1;
        }
    }

    /// Record an indirect draw call.
    pub fn record_draw_indirect(&mut self, draw_count: u32) {
        self.current_frame.draw.indirect_draw_calls += draw_count;
    }

    /// Record a dispatch.
    pub fn record_dispatch(&mut self, x: u32, y: u32, z: u32) {
        self.current_frame.draw.dispatches += 1;
        self.current_frame.draw.total_workgroups += (x * y * z) as u64;
    }

    /// Record an indirect dispatch.
    pub fn record_dispatch_indirect(&mut self) {
        self.current_frame.draw.indirect_dispatches += 1;
    }

    /// Record a pipeline bind.
    pub fn record_pipeline_bind(&mut self) {
        self.current_frame.pipeline_binds += 1;
    }

    /// Record a descriptor set bind.
    pub fn record_descriptor_bind(&mut self) {
        self.current_frame.descriptor_binds += 1;
    }

    /// Record a buffer upload.
    pub fn record_buffer_upload(&mut self, bytes: u64) {
        self.current_frame.buffer_uploads += 1;
        self.current_frame.buffer_upload_bytes += bytes;
    }

    /// Record an image upload.
    pub fn record_image_upload(&mut self, bytes: u64) {
        self.current_frame.image_uploads += 1;
        self.current_frame.image_upload_bytes += bytes;
    }

    /// Record a render pass.
    pub fn record_render_pass(&mut self) {
        self.current_frame.render_passes += 1;
    }

    /// Get current frame statistics.
    pub fn current(&self) -> &FrameStatistics {
        &self.current_frame
    }

    /// Get current frame statistics (mutable).
    pub fn current_mut(&mut self) -> &mut FrameStatistics {
        &mut self.current_frame
    }

    /// Get frame history.
    pub fn history(&self) -> &[FrameStatistics] {
        &self.history
    }

    /// Get snapshot.
    pub fn snapshot(&self) -> StatisticsSnapshot {
        let avg_frame_time = if self.frame_count > 0 {
            self.accumulated_frame_time / self.frame_count as f64
        } else {
            0.0
        };

        let avg_fps = if avg_frame_time > 0.0 {
            1000.0 / avg_frame_time
        } else {
            0.0
        };

        let fps = if self.current_frame.frame_time_ms > 0.0 {
            1000.0 / self.current_frame.frame_time_ms
        } else {
            0.0
        };

        StatisticsSnapshot {
            frame_index: self.frame_index,
            timestamp: 0,
            frame: self.current_frame.clone(),
            avg_frame_time_ms: avg_frame_time,
            min_frame_time_ms: if self.min_frame_time == f64::MAX {
                0.0
            } else {
                self.min_frame_time
            },
            max_frame_time_ms: self.max_frame_time,
            fps,
            avg_fps,
        }
    }

    /// Reset statistics.
    pub fn reset(&mut self) {
        self.current_frame = FrameStatistics::default();
        self.history.clear();
        self.accumulated_frame_time = 0.0;
        self.min_frame_time = f64::MAX;
        self.max_frame_time = 0.0;
        self.frame_count = 0;
    }

    /// Get average frame time.
    pub fn avg_frame_time(&self) -> f64 {
        if self.frame_count > 0 {
            self.accumulated_frame_time / self.frame_count as f64
        } else {
            0.0
        }
    }

    /// Get average FPS.
    pub fn avg_fps(&self) -> f64 {
        let avg = self.avg_frame_time();
        if avg > 0.0 {
            1000.0 / avg
        } else {
            0.0
        }
    }
}

impl Default for StatisticsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Atomic Counters
// ============================================================================

/// Atomic counter for thread-safe statistics.
pub struct AtomicCounter {
    value: AtomicU64,
}

impl AtomicCounter {
    /// Create a new counter.
    pub const fn new() -> Self {
        Self {
            value: AtomicU64::new(0),
        }
    }

    /// Increment by 1.
    pub fn increment(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Add value.
    pub fn add(&self, value: u64) {
        self.value.fetch_add(value, Ordering::Relaxed);
    }

    /// Get current value.
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Reset to zero.
    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }

    /// Swap value and return old.
    pub fn swap(&self, value: u64) -> u64 {
        self.value.swap(value, Ordering::Relaxed)
    }
}

impl Default for AtomicCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Global atomic counters for statistics.
pub mod counters {
    use super::AtomicCounter;

    /// Draw call counter.
    pub static DRAW_CALLS: AtomicCounter = AtomicCounter::new();
    /// Dispatch counter.
    pub static DISPATCHES: AtomicCounter = AtomicCounter::new();
    /// Pipeline bind counter.
    pub static PIPELINE_BINDS: AtomicCounter = AtomicCounter::new();
    /// Descriptor bind counter.
    pub static DESCRIPTOR_BINDS: AtomicCounter = AtomicCounter::new();
    /// Buffer upload counter.
    pub static BUFFER_UPLOADS: AtomicCounter = AtomicCounter::new();
    /// Buffer upload bytes.
    pub static BUFFER_UPLOAD_BYTES: AtomicCounter = AtomicCounter::new();
    /// Vertices drawn.
    pub static VERTICES_DRAWN: AtomicCounter = AtomicCounter::new();
    /// Triangles drawn.
    pub static TRIANGLES_DRAWN: AtomicCounter = AtomicCounter::new();

    /// Reset all counters.
    pub fn reset_all() {
        DRAW_CALLS.reset();
        DISPATCHES.reset();
        PIPELINE_BINDS.reset();
        DESCRIPTOR_BINDS.reset();
        BUFFER_UPLOADS.reset();
        BUFFER_UPLOAD_BYTES.reset();
        VERTICES_DRAWN.reset();
        TRIANGLES_DRAWN.reset();
    }
}
