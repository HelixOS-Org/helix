//! Timeline Semaphores
//!
//! Monotonic counter-based synchronization for advanced scheduling.

use alloc::{string::String, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};

use lumina_core::Handle;

// ============================================================================
// Timeline Semaphore Handle
// ============================================================================

/// Handle to a timeline semaphore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimelineSemaphoreHandle(Handle<TimelineSemaphore>);

impl TimelineSemaphoreHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self(Handle::from_raw_parts(index, generation))
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.0.index()
    }

    /// Get the generation.
    pub fn generation(&self) -> u32 {
        self.0.generation()
    }

    /// Invalid handle.
    pub const INVALID: Self = Self(Handle::INVALID);
}

// ============================================================================
// Timeline Semaphore Description
// ============================================================================

/// Description for timeline semaphore creation.
#[derive(Debug, Clone)]
pub struct TimelineSemaphoreDesc {
    /// Initial value.
    pub initial_value: u64,
    /// Maximum pending signals.
    pub max_pending_signals: u32,
    /// Debug label.
    pub label: Option<String>,
}

impl Default for TimelineSemaphoreDesc {
    fn default() -> Self {
        Self {
            initial_value: 0,
            max_pending_signals: 64,
            label: None,
        }
    }
}

impl TimelineSemaphoreDesc {
    /// Create with initial value.
    pub fn new(initial_value: u64) -> Self {
        Self {
            initial_value,
            ..Default::default()
        }
    }

    /// Set debug label.
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = Some(String::from(label));
        self
    }
}

// ============================================================================
// Timeline Semaphore
// ============================================================================

/// A timeline semaphore for monotonic counter-based synchronization.
pub struct TimelineSemaphore {
    /// Handle.
    pub handle: TimelineSemaphoreHandle,
    /// Current value.
    value: AtomicU64,
    /// Pending value (last signaled).
    pending_value: AtomicU64,
    /// Maximum value seen.
    max_value: AtomicU64,
    /// Signal count.
    signal_count: AtomicU64,
    /// Wait count.
    wait_count: AtomicU64,
    /// Debug label.
    pub label: Option<String>,
}

impl TimelineSemaphore {
    /// Create a new timeline semaphore.
    pub fn new(handle: TimelineSemaphoreHandle, desc: &TimelineSemaphoreDesc) -> Self {
        Self {
            handle,
            value: AtomicU64::new(desc.initial_value),
            pending_value: AtomicU64::new(desc.initial_value),
            max_value: AtomicU64::new(desc.initial_value),
            signal_count: AtomicU64::new(0),
            wait_count: AtomicU64::new(0),
            label: desc.label.clone(),
        }
    }

    /// Get current value.
    pub fn current_value(&self) -> u64 {
        self.value.load(Ordering::Acquire)
    }

    /// Get pending value.
    pub fn pending_value(&self) -> u64 {
        self.pending_value.load(Ordering::Acquire)
    }

    /// Get maximum value.
    pub fn max_value(&self) -> u64 {
        self.max_value.load(Ordering::Relaxed)
    }

    /// Signal with value.
    pub fn signal(&self, value: u64) {
        // Update pending value
        let mut current = self.pending_value.load(Ordering::Relaxed);
        loop {
            if current >= value {
                break;
            }
            match self.pending_value.compare_exchange_weak(
                current,
                value,
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(c) => current = c,
            }
        }

        // Update max value
        let mut max = self.max_value.load(Ordering::Relaxed);
        while max < value {
            match self.max_value.compare_exchange_weak(
                max,
                value,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(m) => max = m,
            }
        }

        // Update current value (simulating GPU completion)
        self.value.store(value, Ordering::Release);
        self.signal_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Wait for value.
    pub fn wait(&self, target: u64) {
        self.wait_count.fetch_add(1, Ordering::Relaxed);
        while self.value.load(Ordering::Acquire) < target {
            core::hint::spin_loop();
        }
    }

    /// Check if value is reached.
    pub fn is_reached(&self, target: u64) -> bool {
        self.current_value() >= target
    }

    /// Get next signal value.
    pub fn next_value(&self) -> u64 {
        self.max_value.load(Ordering::Relaxed) + 1
    }

    /// Signal next value.
    pub fn signal_next(&self) -> u64 {
        let next = self.next_value();
        self.signal(next);
        next
    }

    /// Get signal count.
    pub fn signal_count(&self) -> u64 {
        self.signal_count.load(Ordering::Relaxed)
    }

    /// Get wait count.
    pub fn wait_count(&self) -> u64 {
        self.wait_count.load(Ordering::Relaxed)
    }
}

// ============================================================================
// Timeline Wait/Signal Info
// ============================================================================

/// Timeline wait info.
#[derive(Debug, Clone)]
pub struct TimelineWaitInfo {
    /// Semaphore handle.
    pub semaphore: TimelineSemaphoreHandle,
    /// Value to wait for.
    pub value: u64,
}

impl TimelineWaitInfo {
    /// Create a new wait info.
    pub fn new(semaphore: TimelineSemaphoreHandle, value: u64) -> Self {
        Self { semaphore, value }
    }
}

/// Timeline signal info.
#[derive(Debug, Clone)]
pub struct TimelineSignalInfo {
    /// Semaphore handle.
    pub semaphore: TimelineSemaphoreHandle,
    /// Value to signal.
    pub value: u64,
}

impl TimelineSignalInfo {
    /// Create a new signal info.
    pub fn new(semaphore: TimelineSemaphoreHandle, value: u64) -> Self {
        Self { semaphore, value }
    }
}

// ============================================================================
// Timeline Manager
// ============================================================================

/// Statistics for timeline manager.
#[derive(Debug, Clone, Default)]
pub struct TimelineStatistics {
    /// Total semaphores created.
    pub created: u64,
    /// Total semaphores destroyed.
    pub destroyed: u64,
    /// Active semaphores.
    pub active: u32,
    /// Total signals.
    pub total_signals: u64,
    /// Total waits.
    pub total_waits: u64,
}

/// Manages timeline semaphores.
pub struct TimelineManager {
    /// Semaphores.
    semaphores: Vec<Option<TimelineSemaphore>>,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Generations.
    generations: Vec<u32>,
    /// Statistics.
    stats: TimelineStatistics,
}

impl TimelineManager {
    /// Create a new timeline manager.
    pub fn new() -> Self {
        Self {
            semaphores: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
            stats: TimelineStatistics::default(),
        }
    }

    /// Create a timeline semaphore.
    pub fn create(&mut self, desc: &TimelineSemaphoreDesc) -> TimelineSemaphoreHandle {
        let index = if let Some(index) = self.free_indices.pop() {
            index
        } else {
            let index = self.semaphores.len() as u32;
            self.semaphores.push(None);
            self.generations.push(0);
            index
        };

        let generation = self.generations[index as usize];
        let handle = TimelineSemaphoreHandle::new(index, generation);
        let semaphore = TimelineSemaphore::new(handle, desc);

        self.semaphores[index as usize] = Some(semaphore);
        self.stats.created += 1;
        self.stats.active += 1;

        handle
    }

    /// Create with initial value.
    pub fn create_with_value(&mut self, initial_value: u64) -> TimelineSemaphoreHandle {
        self.create(&TimelineSemaphoreDesc::new(initial_value))
    }

    /// Destroy a timeline semaphore.
    pub fn destroy(&mut self, handle: TimelineSemaphoreHandle) {
        let index = handle.index() as usize;
        if index < self.semaphores.len() && self.generations[index] == handle.generation() {
            if let Some(semaphore) = self.semaphores[index].take() {
                self.stats.total_signals += semaphore.signal_count();
                self.stats.total_waits += semaphore.wait_count();
            }
            self.generations[index] = self.generations[index].wrapping_add(1);
            self.free_indices.push(index as u32);
            self.stats.destroyed += 1;
            self.stats.active = self.stats.active.saturating_sub(1);
        }
    }

    /// Get a timeline semaphore.
    pub fn get(&self, handle: TimelineSemaphoreHandle) -> Option<&TimelineSemaphore> {
        let index = handle.index() as usize;
        if index >= self.semaphores.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.semaphores[index].as_ref()
    }

    /// Get a timeline semaphore mutably.
    pub fn get_mut(&mut self, handle: TimelineSemaphoreHandle) -> Option<&mut TimelineSemaphore> {
        let index = handle.index() as usize;
        if index >= self.semaphores.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.semaphores[index].as_mut()
    }

    /// Signal a timeline semaphore.
    pub fn signal(&self, handle: TimelineSemaphoreHandle, value: u64) {
        if let Some(semaphore) = self.get(handle) {
            semaphore.signal(value);
        }
    }

    /// Wait for a timeline semaphore.
    pub fn wait(&self, handle: TimelineSemaphoreHandle, value: u64) {
        if let Some(semaphore) = self.get(handle) {
            semaphore.wait(value);
        }
    }

    /// Get current value.
    pub fn current_value(&self, handle: TimelineSemaphoreHandle) -> u64 {
        self.get(handle).map(|s| s.current_value()).unwrap_or(0)
    }

    /// Check if value is reached.
    pub fn is_reached(&self, handle: TimelineSemaphoreHandle, value: u64) -> bool {
        self.get(handle).map(|s| s.is_reached(value)).unwrap_or(false)
    }

    /// Wait for multiple timeline semaphores.
    pub fn wait_all(&self, waits: &[TimelineWaitInfo]) {
        for wait in waits {
            self.wait(wait.semaphore, wait.value);
        }
    }

    /// Wait for any timeline semaphore.
    pub fn wait_any(&self, waits: &[TimelineWaitInfo]) -> Option<usize> {
        loop {
            for (i, wait) in waits.iter().enumerate() {
                if self.is_reached(wait.semaphore, wait.value) {
                    return Some(i);
                }
            }
            core::hint::spin_loop();
        }
    }

    /// Signal multiple timeline semaphores.
    pub fn signal_all(&self, signals: &[TimelineSignalInfo]) {
        for signal in signals {
            self.signal(signal.semaphore, signal.value);
        }
    }

    /// Get statistics.
    pub fn statistics(&self) -> &TimelineStatistics {
        &self.stats
    }
}

impl Default for TimelineManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Frame Timeline
// ============================================================================

/// Timeline for frame synchronization.
pub struct FrameTimeline {
    /// Semaphore handle.
    pub semaphore: TimelineSemaphoreHandle,
    /// Current frame.
    current_frame: AtomicU64,
    /// Completed frame.
    completed_frame: AtomicU64,
}

impl FrameTimeline {
    /// Create a new frame timeline.
    pub fn new(manager: &mut TimelineManager) -> Self {
        let semaphore = manager.create_with_value(0);
        Self {
            semaphore,
            current_frame: AtomicU64::new(0),
            completed_frame: AtomicU64::new(0),
        }
    }

    /// Begin next frame.
    pub fn begin_frame(&self) -> u64 {
        self.current_frame.fetch_add(1, Ordering::Relaxed)
    }

    /// End frame (signal completion).
    pub fn end_frame(&self, manager: &TimelineManager) {
        let frame = self.current_frame.load(Ordering::Relaxed);
        manager.signal(self.semaphore, frame);
        self.completed_frame.store(frame, Ordering::Relaxed);
    }

    /// Get current frame.
    pub fn current_frame(&self) -> u64 {
        self.current_frame.load(Ordering::Relaxed)
    }

    /// Get completed frame.
    pub fn completed_frame(&self) -> u64 {
        self.completed_frame.load(Ordering::Relaxed)
    }

    /// Wait for frame completion.
    pub fn wait_frame(&self, manager: &TimelineManager, frame: u64) {
        manager.wait(self.semaphore, frame);
    }

    /// Check if frame is complete.
    pub fn is_frame_complete(&self, manager: &TimelineManager, frame: u64) -> bool {
        manager.is_reached(self.semaphore, frame)
    }

    /// Get frames in flight.
    pub fn frames_in_flight(&self) -> u64 {
        let current = self.current_frame.load(Ordering::Relaxed);
        let completed = self.completed_frame.load(Ordering::Relaxed);
        current.saturating_sub(completed)
    }
}

// ============================================================================
// Pipeline Timeline
// ============================================================================

/// Timeline for pipeline stages.
pub struct PipelineTimeline {
    /// GPU work semaphore.
    pub gpu_work: TimelineSemaphoreHandle,
    /// Copy semaphore.
    pub copy: TimelineSemaphoreHandle,
    /// Compute semaphore.
    pub compute: TimelineSemaphoreHandle,
    /// Graphics semaphore.
    pub graphics: TimelineSemaphoreHandle,
}

impl PipelineTimeline {
    /// Create a new pipeline timeline.
    pub fn new(manager: &mut TimelineManager) -> Self {
        Self {
            gpu_work: manager.create_with_value(0),
            copy: manager.create_with_value(0),
            compute: manager.create_with_value(0),
            graphics: manager.create_with_value(0),
        }
    }

    /// Signal copy completion.
    pub fn signal_copy(&self, manager: &TimelineManager, value: u64) {
        manager.signal(self.copy, value);
    }

    /// Signal compute completion.
    pub fn signal_compute(&self, manager: &TimelineManager, value: u64) {
        manager.signal(self.compute, value);
    }

    /// Signal graphics completion.
    pub fn signal_graphics(&self, manager: &TimelineManager, value: u64) {
        manager.signal(self.graphics, value);
    }

    /// Wait for copy.
    pub fn wait_copy(&self, manager: &TimelineManager, value: u64) {
        manager.wait(self.copy, value);
    }

    /// Wait for compute.
    pub fn wait_compute(&self, manager: &TimelineManager, value: u64) {
        manager.wait(self.compute, value);
    }

    /// Wait for graphics.
    pub fn wait_graphics(&self, manager: &TimelineManager, value: u64) {
        manager.wait(self.graphics, value);
    }
}
