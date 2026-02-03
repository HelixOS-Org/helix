//! GPU Semaphores
//!
//! GPU-GPU synchronization primitives for queue operations.

use alloc::{string::String, vec::Vec};
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use lumina_core::Handle;

// ============================================================================
// Semaphore Handle
// ============================================================================

/// Handle to a semaphore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SemaphoreHandle(Handle<Semaphore>);

impl SemaphoreHandle {
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
// Semaphore Type
// ============================================================================

/// Semaphore type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemaphoreType {
    /// Binary semaphore.
    Binary,
    /// Timeline semaphore.
    Timeline,
}

// ============================================================================
// Semaphore Description
// ============================================================================

/// Description for semaphore creation.
#[derive(Debug, Clone)]
pub struct SemaphoreDesc {
    /// Semaphore type.
    pub semaphore_type: SemaphoreType,
    /// Initial value (for timeline).
    pub initial_value: u64,
    /// Debug label.
    pub label: Option<String>,
}

impl Default for SemaphoreDesc {
    fn default() -> Self {
        Self {
            semaphore_type: SemaphoreType::Binary,
            initial_value: 0,
            label: None,
        }
    }
}

impl SemaphoreDesc {
    /// Create a binary semaphore.
    pub fn binary() -> Self {
        Self::default()
    }

    /// Create a timeline semaphore.
    pub fn timeline(initial_value: u64) -> Self {
        Self {
            semaphore_type: SemaphoreType::Timeline,
            initial_value,
            label: None,
        }
    }

    /// Set debug label.
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = Some(String::from(label));
        self
    }
}

// ============================================================================
// Semaphore
// ============================================================================

/// A GPU synchronization semaphore.
pub struct Semaphore {
    /// Handle.
    pub handle: SemaphoreHandle,
    /// Semaphore type.
    pub semaphore_type: SemaphoreType,
    /// Internal state (signaled for binary).
    state: AtomicU32,
    /// Value (for timeline).
    value: AtomicU64,
    /// Signal count.
    signal_count: AtomicU64,
    /// Wait count.
    wait_count: AtomicU64,
    /// Debug label.
    pub label: Option<String>,
}

impl Semaphore {
    /// Create a new semaphore.
    pub fn new(handle: SemaphoreHandle, desc: &SemaphoreDesc) -> Self {
        Self {
            handle,
            semaphore_type: desc.semaphore_type,
            state: AtomicU32::new(0),
            value: AtomicU64::new(desc.initial_value),
            signal_count: AtomicU64::new(0),
            wait_count: AtomicU64::new(0),
            label: desc.label.clone(),
        }
    }

    /// Check if binary semaphore is signaled.
    pub fn is_signaled(&self) -> bool {
        self.state.load(Ordering::Acquire) != 0
    }

    /// Signal (binary).
    pub fn signal(&self) {
        self.state.store(1, Ordering::Release);
        self.signal_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Wait (binary).
    pub fn wait(&self) {
        self.wait_count.fetch_add(1, Ordering::Relaxed);
        self.state.store(0, Ordering::Release);
    }

    /// Get current value (timeline).
    pub fn current_value(&self) -> u64 {
        self.value.load(Ordering::Acquire)
    }

    /// Signal with value (timeline).
    pub fn signal_value(&self, value: u64) {
        self.value.store(value, Ordering::Release);
        self.signal_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Wait for value (timeline).
    pub fn wait_value(&self, target: u64) {
        self.wait_count.fetch_add(1, Ordering::Relaxed);
        while self.value.load(Ordering::Acquire) < target {
            core::hint::spin_loop();
        }
    }

    /// Get signal count.
    pub fn signal_count(&self) -> u64 {
        self.signal_count.load(Ordering::Relaxed)
    }

    /// Get wait count.
    pub fn wait_count(&self) -> u64 {
        self.wait_count.load(Ordering::Relaxed)
    }

    /// Check if timeline type.
    pub fn is_timeline(&self) -> bool {
        self.semaphore_type == SemaphoreType::Timeline
    }
}

// ============================================================================
// Semaphore Wait Info
// ============================================================================

/// Pipeline stage flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemaphoreWaitStage {
    /// Top of pipe.
    TopOfPipe,
    /// Draw indirect.
    DrawIndirect,
    /// Vertex input.
    VertexInput,
    /// Vertex shader.
    VertexShader,
    /// Fragment shader.
    FragmentShader,
    /// Color attachment output.
    ColorAttachmentOutput,
    /// Compute shader.
    ComputeShader,
    /// Transfer.
    Transfer,
    /// Bottom of pipe.
    BottomOfPipe,
    /// All graphics.
    AllGraphics,
    /// All commands.
    AllCommands,
}

/// Semaphore wait info.
#[derive(Debug, Clone)]
pub struct SemaphoreWaitInfo {
    /// Semaphore to wait on.
    pub semaphore: SemaphoreHandle,
    /// Stage to wait at.
    pub stage: SemaphoreWaitStage,
    /// Value to wait for (timeline only).
    pub value: u64,
}

impl SemaphoreWaitInfo {
    /// Create a binary wait.
    pub fn binary(semaphore: SemaphoreHandle, stage: SemaphoreWaitStage) -> Self {
        Self {
            semaphore,
            stage,
            value: 0,
        }
    }

    /// Create a timeline wait.
    pub fn timeline(semaphore: SemaphoreHandle, stage: SemaphoreWaitStage, value: u64) -> Self {
        Self {
            semaphore,
            stage,
            value,
        }
    }
}

/// Semaphore signal info.
#[derive(Debug, Clone)]
pub struct SemaphoreSignalInfo {
    /// Semaphore to signal.
    pub semaphore: SemaphoreHandle,
    /// Value to signal (timeline only).
    pub value: u64,
}

impl SemaphoreSignalInfo {
    /// Create a binary signal.
    pub fn binary(semaphore: SemaphoreHandle) -> Self {
        Self {
            semaphore,
            value: 0,
        }
    }

    /// Create a timeline signal.
    pub fn timeline(semaphore: SemaphoreHandle, value: u64) -> Self {
        Self {
            semaphore,
            value,
        }
    }
}

// ============================================================================
// Semaphore Manager
// ============================================================================

/// Statistics for semaphore manager.
#[derive(Debug, Clone, Default)]
pub struct SemaphoreStatistics {
    /// Total binary semaphores.
    pub binary_count: u32,
    /// Total timeline semaphores.
    pub timeline_count: u32,
    /// Total signals.
    pub total_signals: u64,
    /// Total waits.
    pub total_waits: u64,
}

/// Manages GPU semaphores.
pub struct SemaphoreManager {
    /// Semaphores.
    semaphores: Vec<Option<Semaphore>>,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Generations.
    generations: Vec<u32>,
    /// Statistics.
    stats: SemaphoreStatistics,
}

impl SemaphoreManager {
    /// Create a new semaphore manager.
    pub fn new() -> Self {
        Self {
            semaphores: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
            stats: SemaphoreStatistics::default(),
        }
    }

    /// Create a semaphore.
    pub fn create(&mut self, desc: &SemaphoreDesc) -> SemaphoreHandle {
        let index = if let Some(index) = self.free_indices.pop() {
            index
        } else {
            let index = self.semaphores.len() as u32;
            self.semaphores.push(None);
            self.generations.push(0);
            index
        };

        let generation = self.generations[index as usize];
        let handle = SemaphoreHandle::new(index, generation);
        let semaphore = Semaphore::new(handle, desc);

        match desc.semaphore_type {
            SemaphoreType::Binary => self.stats.binary_count += 1,
            SemaphoreType::Timeline => self.stats.timeline_count += 1,
        }

        self.semaphores[index as usize] = Some(semaphore);

        handle
    }

    /// Create a binary semaphore.
    pub fn create_binary(&mut self) -> SemaphoreHandle {
        self.create(&SemaphoreDesc::binary())
    }

    /// Create a timeline semaphore.
    pub fn create_timeline(&mut self, initial_value: u64) -> SemaphoreHandle {
        self.create(&SemaphoreDesc::timeline(initial_value))
    }

    /// Destroy a semaphore.
    pub fn destroy(&mut self, handle: SemaphoreHandle) {
        let index = handle.index() as usize;
        if index < self.semaphores.len() && self.generations[index] == handle.generation() {
            if let Some(semaphore) = self.semaphores[index].take() {
                match semaphore.semaphore_type {
                    SemaphoreType::Binary => {
                        self.stats.binary_count = self.stats.binary_count.saturating_sub(1);
                    }
                    SemaphoreType::Timeline => {
                        self.stats.timeline_count = self.stats.timeline_count.saturating_sub(1);
                    }
                }
                self.stats.total_signals += semaphore.signal_count();
                self.stats.total_waits += semaphore.wait_count();
            }
            self.generations[index] = self.generations[index].wrapping_add(1);
            self.free_indices.push(index as u32);
        }
    }

    /// Get a semaphore.
    pub fn get(&self, handle: SemaphoreHandle) -> Option<&Semaphore> {
        let index = handle.index() as usize;
        if index >= self.semaphores.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.semaphores[index].as_ref()
    }

    /// Get a semaphore mutably.
    pub fn get_mut(&mut self, handle: SemaphoreHandle) -> Option<&mut Semaphore> {
        let index = handle.index() as usize;
        if index >= self.semaphores.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.semaphores[index].as_mut()
    }

    /// Signal a semaphore.
    pub fn signal(&self, handle: SemaphoreHandle) {
        if let Some(semaphore) = self.get(handle) {
            semaphore.signal();
        }
    }

    /// Signal timeline semaphore with value.
    pub fn signal_value(&self, handle: SemaphoreHandle, value: u64) {
        if let Some(semaphore) = self.get(handle) {
            semaphore.signal_value(value);
        }
    }

    /// Get current value of timeline semaphore.
    pub fn current_value(&self, handle: SemaphoreHandle) -> u64 {
        self.get(handle).map(|s| s.current_value()).unwrap_or(0)
    }

    /// Get statistics.
    pub fn statistics(&self) -> &SemaphoreStatistics {
        &self.stats
    }

    /// Get total semaphore count.
    pub fn count(&self) -> u32 {
        self.stats.binary_count + self.stats.timeline_count
    }
}

impl Default for SemaphoreManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Semaphore Chain
// ============================================================================

/// A chain of semaphore operations for submission.
#[derive(Debug, Clone, Default)]
pub struct SemaphoreChain {
    /// Wait operations.
    pub waits: Vec<SemaphoreWaitInfo>,
    /// Signal operations.
    pub signals: Vec<SemaphoreSignalInfo>,
}

impl SemaphoreChain {
    /// Create a new chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a wait.
    pub fn wait(mut self, info: SemaphoreWaitInfo) -> Self {
        self.waits.push(info);
        self
    }

    /// Add a binary wait.
    pub fn wait_binary(self, semaphore: SemaphoreHandle, stage: SemaphoreWaitStage) -> Self {
        self.wait(SemaphoreWaitInfo::binary(semaphore, stage))
    }

    /// Add a timeline wait.
    pub fn wait_timeline(self, semaphore: SemaphoreHandle, stage: SemaphoreWaitStage, value: u64) -> Self {
        self.wait(SemaphoreWaitInfo::timeline(semaphore, stage, value))
    }

    /// Add a signal.
    pub fn signal(mut self, info: SemaphoreSignalInfo) -> Self {
        self.signals.push(info);
        self
    }

    /// Add a binary signal.
    pub fn signal_binary(self, semaphore: SemaphoreHandle) -> Self {
        self.signal(SemaphoreSignalInfo::binary(semaphore))
    }

    /// Add a timeline signal.
    pub fn signal_timeline(self, semaphore: SemaphoreHandle, value: u64) -> Self {
        self.signal(SemaphoreSignalInfo::timeline(semaphore, value))
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.waits.is_empty() && self.signals.is_empty()
    }
}
