//! GPU Timeline Management Types for Lumina
//!
//! This module provides GPU timeline and synchronization
//! infrastructure for advanced multi-queue operations.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Timeline Handles
// ============================================================================

/// GPU timeline handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuTimelineHandle(pub u64);

impl GpuTimelineHandle {
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

impl Default for GpuTimelineHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Timeline semaphore handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TimelineSemaphoreHandle(pub u64);

impl TimelineSemaphoreHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for TimelineSemaphoreHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Timeline point handle (represents a specific point on the timeline)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TimelinePointHandle(pub u64);

impl TimelinePointHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for TimelinePointHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Sync point handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SyncPointHandle(pub u64);

impl SyncPointHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SyncPointHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Timeline Creation
// ============================================================================

/// GPU timeline create info
#[derive(Clone, Debug)]
pub struct GpuTimelineCreateInfo {
    /// Name
    pub name: String,
    /// Initial value
    pub initial_value: u64,
    /// Max pending signals
    pub max_pending: u32,
    /// Timeline type
    pub timeline_type: TimelineType,
    /// Features
    pub features: TimelineFeatures,
}

impl GpuTimelineCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            initial_value: 0,
            max_pending: 256,
            timeline_type: TimelineType::GpuOnly,
            features: TimelineFeatures::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With initial value
    pub fn with_initial_value(mut self, value: u64) -> Self {
        self.initial_value = value;
        self
    }

    /// With max pending
    pub fn with_max_pending(mut self, max: u32) -> Self {
        self.max_pending = max;
        self
    }

    /// With timeline type
    pub fn with_type(mut self, timeline_type: TimelineType) -> Self {
        self.timeline_type = timeline_type;
        self
    }

    /// With features
    pub fn with_features(mut self, features: TimelineFeatures) -> Self {
        self.features |= features;
        self
    }

    /// GPU-only timeline
    pub fn gpu_only() -> Self {
        Self::new()
            .with_type(TimelineType::GpuOnly)
    }

    /// Shared timeline (cross-queue)
    pub fn shared() -> Self {
        Self::new()
            .with_type(TimelineType::Shared)
            .with_features(TimelineFeatures::CROSS_QUEUE)
    }

    /// External timeline (for cross-process/device sharing)
    pub fn external() -> Self {
        Self::new()
            .with_type(TimelineType::External)
            .with_features(TimelineFeatures::EXPORTABLE | TimelineFeatures::IMPORTABLE)
    }
}

impl Default for GpuTimelineCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Timeline type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TimelineType {
    /// GPU-only (single queue)
    #[default]
    GpuOnly = 0,
    /// Shared (cross-queue)
    Shared = 1,
    /// External (cross-process/device)
    External = 2,
    /// Host-signaled
    HostSignaled = 3,
}

bitflags::bitflags! {
    /// Timeline features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct TimelineFeatures: u32 {
        /// None
        const NONE = 0;
        /// Cross-queue synchronization
        const CROSS_QUEUE = 1 << 0;
        /// Exportable to external handle
        const EXPORTABLE = 1 << 1;
        /// Importable from external handle
        const IMPORTABLE = 1 << 2;
        /// CPU wait support
        const CPU_WAIT = 1 << 3;
        /// CPU signal support
        const CPU_SIGNAL = 1 << 4;
        /// Query support
        const QUERYABLE = 1 << 5;
    }
}

// ============================================================================
// Timeline Operations
// ============================================================================

/// Timeline signal operation
#[derive(Clone, Debug)]
pub struct TimelineSignal {
    /// Timeline handle
    pub timeline: GpuTimelineHandle,
    /// Signal value
    pub value: u64,
    /// Stage mask (when to signal)
    pub stage_mask: PipelineStageFlags,
}

impl TimelineSignal {
    /// Creates new signal
    pub fn new(timeline: GpuTimelineHandle, value: u64) -> Self {
        Self {
            timeline,
            value,
            stage_mask: PipelineStageFlags::ALL_COMMANDS,
        }
    }

    /// With stage mask
    pub fn with_stage(mut self, stage: PipelineStageFlags) -> Self {
        self.stage_mask = stage;
        self
    }

    /// After all graphics
    pub fn after_graphics(timeline: GpuTimelineHandle, value: u64) -> Self {
        Self::new(timeline, value)
            .with_stage(PipelineStageFlags::ALL_GRAPHICS)
    }

    /// After compute
    pub fn after_compute(timeline: GpuTimelineHandle, value: u64) -> Self {
        Self::new(timeline, value)
            .with_stage(PipelineStageFlags::COMPUTE_SHADER)
    }

    /// After transfer
    pub fn after_transfer(timeline: GpuTimelineHandle, value: u64) -> Self {
        Self::new(timeline, value)
            .with_stage(PipelineStageFlags::TRANSFER)
    }
}

impl Default for TimelineSignal {
    fn default() -> Self {
        Self::new(GpuTimelineHandle::NULL, 0)
    }
}

/// Timeline wait operation
#[derive(Clone, Debug)]
pub struct TimelineWait {
    /// Timeline handle
    pub timeline: GpuTimelineHandle,
    /// Wait value
    pub value: u64,
    /// Stage mask (which stages to wait before)
    pub stage_mask: PipelineStageFlags,
}

impl TimelineWait {
    /// Creates new wait
    pub fn new(timeline: GpuTimelineHandle, value: u64) -> Self {
        Self {
            timeline,
            value,
            stage_mask: PipelineStageFlags::TOP_OF_PIPE,
        }
    }

    /// With stage mask
    pub fn with_stage(mut self, stage: PipelineStageFlags) -> Self {
        self.stage_mask = stage;
        self
    }

    /// Before all
    pub fn before_all(timeline: GpuTimelineHandle, value: u64) -> Self {
        Self::new(timeline, value)
            .with_stage(PipelineStageFlags::TOP_OF_PIPE)
    }

    /// Before graphics
    pub fn before_graphics(timeline: GpuTimelineHandle, value: u64) -> Self {
        Self::new(timeline, value)
            .with_stage(PipelineStageFlags::ALL_GRAPHICS)
    }

    /// Before compute
    pub fn before_compute(timeline: GpuTimelineHandle, value: u64) -> Self {
        Self::new(timeline, value)
            .with_stage(PipelineStageFlags::COMPUTE_SHADER)
    }
}

impl Default for TimelineWait {
    fn default() -> Self {
        Self::new(GpuTimelineHandle::NULL, 0)
    }
}

bitflags::bitflags! {
    /// Pipeline stage flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct PipelineStageFlags: u64 {
        /// None
        const NONE = 0;
        /// Top of pipe
        const TOP_OF_PIPE = 1 << 0;
        /// Draw indirect
        const DRAW_INDIRECT = 1 << 1;
        /// Vertex input
        const VERTEX_INPUT = 1 << 2;
        /// Vertex shader
        const VERTEX_SHADER = 1 << 3;
        /// Tessellation control
        const TESSELLATION_CONTROL = 1 << 4;
        /// Tessellation evaluation
        const TESSELLATION_EVALUATION = 1 << 5;
        /// Geometry shader
        const GEOMETRY_SHADER = 1 << 6;
        /// Fragment shader
        const FRAGMENT_SHADER = 1 << 7;
        /// Early fragment tests
        const EARLY_FRAGMENT_TESTS = 1 << 8;
        /// Late fragment tests
        const LATE_FRAGMENT_TESTS = 1 << 9;
        /// Color attachment output
        const COLOR_ATTACHMENT_OUTPUT = 1 << 10;
        /// Compute shader
        const COMPUTE_SHADER = 1 << 11;
        /// Transfer
        const TRANSFER = 1 << 12;
        /// Bottom of pipe
        const BOTTOM_OF_PIPE = 1 << 13;
        /// Host
        const HOST = 1 << 14;
        /// All graphics
        const ALL_GRAPHICS = 1 << 15;
        /// All commands
        const ALL_COMMANDS = 1 << 16;
        /// Acceleration structure build
        const ACCELERATION_STRUCTURE_BUILD = 1 << 17;
        /// Ray tracing shader
        const RAY_TRACING_SHADER = 1 << 18;
        /// Task shader
        const TASK_SHADER = 1 << 19;
        /// Mesh shader
        const MESH_SHADER = 1 << 20;
    }
}

// ============================================================================
// Multi-Queue Synchronization
// ============================================================================

/// Queue submit info with timeline
#[derive(Clone, Debug)]
pub struct TimelineSubmitInfo {
    /// Queue family
    pub queue_family: QueueFamily,
    /// Queue index
    pub queue_index: u32,
    /// Command buffers
    pub command_buffers: Vec<u64>,
    /// Wait operations
    pub waits: Vec<TimelineWait>,
    /// Signal operations
    pub signals: Vec<TimelineSignal>,
}

impl TimelineSubmitInfo {
    /// Creates new info
    pub fn new(queue_family: QueueFamily) -> Self {
        Self {
            queue_family,
            queue_index: 0,
            command_buffers: Vec::new(),
            waits: Vec::new(),
            signals: Vec::new(),
        }
    }

    /// With queue index
    pub fn with_queue(mut self, index: u32) -> Self {
        self.queue_index = index;
        self
    }

    /// Add command buffer
    pub fn add_command_buffer(mut self, cmd: u64) -> Self {
        self.command_buffers.push(cmd);
        self
    }

    /// Add wait
    pub fn add_wait(mut self, wait: TimelineWait) -> Self {
        self.waits.push(wait);
        self
    }

    /// Add signal
    pub fn add_signal(mut self, signal: TimelineSignal) -> Self {
        self.signals.push(signal);
        self
    }

    /// Graphics queue
    pub fn graphics() -> Self {
        Self::new(QueueFamily::Graphics)
    }

    /// Compute queue
    pub fn compute() -> Self {
        Self::new(QueueFamily::Compute)
    }

    /// Transfer queue
    pub fn transfer() -> Self {
        Self::new(QueueFamily::Transfer)
    }
}

impl Default for TimelineSubmitInfo {
    fn default() -> Self {
        Self::graphics()
    }
}

/// Queue family
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum QueueFamily {
    /// Graphics queue
    #[default]
    Graphics = 0,
    /// Compute queue
    Compute = 1,
    /// Transfer queue
    Transfer = 2,
    /// Video decode queue
    VideoDecode = 3,
    /// Video encode queue
    VideoEncode = 4,
}

impl QueueFamily {
    /// Supports graphics
    pub const fn supports_graphics(&self) -> bool {
        matches!(self, Self::Graphics)
    }

    /// Supports compute
    pub const fn supports_compute(&self) -> bool {
        matches!(self, Self::Graphics | Self::Compute)
    }

    /// Supports transfer
    pub const fn supports_transfer(&self) -> bool {
        matches!(self, Self::Graphics | Self::Compute | Self::Transfer)
    }
}

// ============================================================================
// CPU-GPU Synchronization
// ============================================================================

/// CPU wait request
#[derive(Clone, Debug)]
pub struct CpuWaitRequest {
    /// Timelines to wait on
    pub timelines: Vec<(GpuTimelineHandle, u64)>,
    /// Wait mode
    pub wait_mode: WaitMode,
    /// Timeout (nanoseconds, 0 = infinite)
    pub timeout_ns: u64,
}

impl CpuWaitRequest {
    /// Creates new request
    pub fn new() -> Self {
        Self {
            timelines: Vec::new(),
            wait_mode: WaitMode::All,
            timeout_ns: u64::MAX,
        }
    }

    /// Add timeline
    pub fn add_timeline(mut self, timeline: GpuTimelineHandle, value: u64) -> Self {
        self.timelines.push((timeline, value));
        self
    }

    /// With wait mode
    pub fn with_mode(mut self, mode: WaitMode) -> Self {
        self.wait_mode = mode;
        self
    }

    /// With timeout
    pub fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ns = ms * 1_000_000;
        self
    }

    /// Wait for single timeline
    pub fn single(timeline: GpuTimelineHandle, value: u64) -> Self {
        Self::new().add_timeline(timeline, value)
    }

    /// Wait for any
    pub fn any(mut self) -> Self {
        self.wait_mode = WaitMode::Any;
        self
    }
}

impl Default for CpuWaitRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Wait mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum WaitMode {
    /// Wait for all
    #[default]
    All = 0,
    /// Wait for any
    Any = 1,
}

/// Wait result
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum WaitResult {
    /// Success
    #[default]
    Success = 0,
    /// Timeout
    Timeout = 1,
    /// Error
    Error = 2,
}

// ============================================================================
// Sync Point
// ============================================================================

/// Sync point (multi-timeline checkpoint)
#[derive(Clone, Debug)]
pub struct SyncPoint {
    /// Handle
    pub handle: SyncPointHandle,
    /// Name
    pub name: String,
    /// Timeline values at this point
    pub timeline_values: Vec<(GpuTimelineHandle, u64)>,
}

impl SyncPoint {
    /// Creates new sync point
    pub fn new() -> Self {
        Self {
            handle: SyncPointHandle::NULL,
            name: String::new(),
            timeline_values: Vec::new(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add timeline value
    pub fn add(mut self, timeline: GpuTimelineHandle, value: u64) -> Self {
        self.timeline_values.push((timeline, value));
        self
    }

    /// Is reached
    pub fn is_reached(&self, current_values: &[(GpuTimelineHandle, u64)]) -> bool {
        for (timeline, required_value) in &self.timeline_values {
            let current = current_values
                .iter()
                .find(|(t, _)| t == timeline)
                .map(|(_, v)| *v)
                .unwrap_or(0);
            if current < *required_value {
                return false;
            }
        }
        true
    }
}

impl Default for SyncPoint {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Timeline State
// ============================================================================

/// Timeline state
#[derive(Clone, Debug, Default)]
pub struct TimelineState {
    /// Handle
    pub handle: GpuTimelineHandle,
    /// Name
    pub name: String,
    /// Current value
    pub current_value: u64,
    /// Pending signal value
    pub pending_signal: u64,
    /// Last completed value
    pub completed_value: u64,
    /// Is external
    pub is_external: bool,
}

impl TimelineState {
    /// Is caught up
    pub fn is_caught_up(&self) -> bool {
        self.current_value >= self.pending_signal
    }

    /// Pending count
    pub fn pending_count(&self) -> u64 {
        self.pending_signal.saturating_sub(self.current_value)
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Timeline statistics
#[derive(Clone, Debug, Default)]
pub struct TimelineStats {
    /// Total timelines
    pub total_timelines: u32,
    /// Total signals
    pub total_signals: u64,
    /// Total waits
    pub total_waits: u64,
    /// CPU waits
    pub cpu_waits: u64,
    /// GPU waits
    pub gpu_waits: u64,
    /// Average wait time (ms)
    pub avg_wait_time_ms: f32,
    /// Max wait time (ms)
    pub max_wait_time_ms: f32,
    /// Cross-queue syncs
    pub cross_queue_syncs: u32,
}

impl TimelineStats {
    /// Total sync operations
    pub fn total_syncs(&self) -> u64 {
        self.total_signals + self.total_waits
    }
}
