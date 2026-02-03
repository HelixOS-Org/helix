//! GPU Fences
//!
//! CPU-GPU synchronization primitives for frame pacing and resource management.

use alloc::{string::String, vec::Vec};
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use core::time::Duration;

use lumina_core::Handle;

// ============================================================================
// Fence State
// ============================================================================

/// Fence state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceState {
    /// Fence is unsignaled.
    Unsignaled,
    /// Fence is signaled.
    Signaled,
    /// Fence is pending (submitted but not yet signaled).
    Pending,
    /// Fence is in error state.
    Error,
}

impl FenceState {
    /// Check if signaled.
    pub fn is_signaled(&self) -> bool {
        *self == FenceState::Signaled
    }

    /// Check if pending.
    pub fn is_pending(&self) -> bool {
        *self == FenceState::Pending
    }
}

// ============================================================================
// Fence Handle
// ============================================================================

/// Handle to a fence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FenceHandle(Handle<Fence>);

impl FenceHandle {
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
// Fence Description
// ============================================================================

/// Description for fence creation.
#[derive(Debug, Clone)]
pub struct FenceDesc {
    /// Create signaled.
    pub signaled: bool,
    /// Debug label.
    pub label: Option<String>,
}

impl Default for FenceDesc {
    fn default() -> Self {
        Self {
            signaled: false,
            label: None,
        }
    }
}

impl FenceDesc {
    /// Create a new fence description.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create signaled.
    pub fn signaled() -> Self {
        Self {
            signaled: true,
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
// Fence
// ============================================================================

/// A CPU-GPU synchronization fence.
pub struct Fence {
    /// Handle.
    pub handle: FenceHandle,
    /// State.
    state: AtomicU32,
    /// Signal count.
    signal_count: AtomicU64,
    /// Wait count.
    wait_count: AtomicU64,
    /// Creation time.
    pub created_at: u64,
    /// Last signal time.
    pub last_signaled_at: AtomicU64,
    /// Debug label.
    pub label: Option<String>,
}

impl Fence {
    /// Create a new fence.
    pub fn new(handle: FenceHandle, desc: &FenceDesc, created_at: u64) -> Self {
        let state = if desc.signaled {
            FenceState::Signaled as u32
        } else {
            FenceState::Unsignaled as u32
        };

        Self {
            handle,
            state: AtomicU32::new(state),
            signal_count: AtomicU64::new(0),
            wait_count: AtomicU64::new(0),
            created_at,
            last_signaled_at: AtomicU64::new(0),
            label: desc.label.clone(),
        }
    }

    /// Get current state.
    pub fn state(&self) -> FenceState {
        match self.state.load(Ordering::Acquire) {
            0 => FenceState::Unsignaled,
            1 => FenceState::Signaled,
            2 => FenceState::Pending,
            _ => FenceState::Error,
        }
    }

    /// Check if signaled.
    pub fn is_signaled(&self) -> bool {
        self.state() == FenceState::Signaled
    }

    /// Check if pending.
    pub fn is_pending(&self) -> bool {
        self.state() == FenceState::Pending
    }

    /// Signal the fence.
    pub fn signal(&self, timestamp: u64) {
        self.state.store(FenceState::Signaled as u32, Ordering::Release);
        self.signal_count.fetch_add(1, Ordering::Relaxed);
        self.last_signaled_at.store(timestamp, Ordering::Relaxed);
    }

    /// Reset the fence.
    pub fn reset(&self) {
        self.state.store(FenceState::Unsignaled as u32, Ordering::Release);
    }

    /// Mark as pending.
    pub fn mark_pending(&self) {
        self.state.store(FenceState::Pending as u32, Ordering::Release);
    }

    /// Wait for signal (blocking).
    pub fn wait(&self) {
        self.wait_count.fetch_add(1, Ordering::Relaxed);
        while !self.is_signaled() {
            core::hint::spin_loop();
        }
    }

    /// Wait with timeout.
    pub fn wait_timeout(&self, _timeout: Duration) -> bool {
        self.wait_count.fetch_add(1, Ordering::Relaxed);
        // In a real implementation, this would use proper timing
        self.is_signaled()
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
// Fence Pool
// ============================================================================

/// A pool of reusable fences.
pub struct FencePool {
    /// Available fences.
    available: Vec<FenceHandle>,
    /// In-use fences.
    in_use: Vec<FenceHandle>,
    /// Maximum pool size.
    max_size: usize,
}

impl FencePool {
    /// Create a new fence pool.
    pub fn new(max_size: usize) -> Self {
        Self {
            available: Vec::with_capacity(max_size),
            in_use: Vec::with_capacity(max_size),
            max_size,
        }
    }

    /// Acquire a fence from the pool.
    pub fn acquire(&mut self) -> Option<FenceHandle> {
        if let Some(handle) = self.available.pop() {
            self.in_use.push(handle);
            Some(handle)
        } else {
            None
        }
    }

    /// Release a fence back to the pool.
    pub fn release(&mut self, handle: FenceHandle) {
        if let Some(pos) = self.in_use.iter().position(|h| *h == handle) {
            self.in_use.swap_remove(pos);
            if self.available.len() < self.max_size {
                self.available.push(handle);
            }
        }
    }

    /// Add a fence to the pool.
    pub fn add(&mut self, handle: FenceHandle) {
        if self.available.len() < self.max_size {
            self.available.push(handle);
        }
    }

    /// Get available count.
    pub fn available_count(&self) -> usize {
        self.available.len()
    }

    /// Get in-use count.
    pub fn in_use_count(&self) -> usize {
        self.in_use.len()
    }
}

// ============================================================================
// Fence Manager
// ============================================================================

/// Statistics for fence manager.
#[derive(Debug, Clone, Default)]
pub struct FenceStatistics {
    /// Total fences created.
    pub created: u64,
    /// Total fences destroyed.
    pub destroyed: u64,
    /// Total signals.
    pub total_signals: u64,
    /// Total waits.
    pub total_waits: u64,
    /// Current active fences.
    pub active: u32,
}

/// Manages GPU fences.
pub struct FenceManager {
    /// Fences.
    fences: Vec<Option<Fence>>,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Generations.
    generations: Vec<u32>,
    /// Statistics.
    stats: FenceStatistics,
    /// Current timestamp.
    current_timestamp: AtomicU64,
}

impl FenceManager {
    /// Create a new fence manager.
    pub fn new() -> Self {
        Self {
            fences: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
            stats: FenceStatistics::default(),
            current_timestamp: AtomicU64::new(0),
        }
    }

    /// Create a fence.
    pub fn create(&mut self, desc: &FenceDesc) -> FenceHandle {
        let index = if let Some(index) = self.free_indices.pop() {
            index
        } else {
            let index = self.fences.len() as u32;
            self.fences.push(None);
            self.generations.push(0);
            index
        };

        let generation = self.generations[index as usize];
        let handle = FenceHandle::new(index, generation);
        let timestamp = self.current_timestamp.load(Ordering::Relaxed);
        let fence = Fence::new(handle, desc, timestamp);

        self.fences[index as usize] = Some(fence);
        self.stats.created += 1;
        self.stats.active += 1;

        handle
    }

    /// Create a signaled fence.
    pub fn create_signaled(&mut self) -> FenceHandle {
        self.create(&FenceDesc::signaled())
    }

    /// Destroy a fence.
    pub fn destroy(&mut self, handle: FenceHandle) {
        let index = handle.index() as usize;
        if index < self.fences.len() && self.generations[index] == handle.generation() {
            if let Some(fence) = self.fences[index].take() {
                self.stats.total_signals += fence.signal_count();
                self.stats.total_waits += fence.wait_count();
            }
            self.generations[index] = self.generations[index].wrapping_add(1);
            self.free_indices.push(index as u32);
            self.stats.destroyed += 1;
            self.stats.active = self.stats.active.saturating_sub(1);
        }
    }

    /// Get a fence.
    pub fn get(&self, handle: FenceHandle) -> Option<&Fence> {
        let index = handle.index() as usize;
        if index >= self.fences.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.fences[index].as_ref()
    }

    /// Get a fence mutably.
    pub fn get_mut(&mut self, handle: FenceHandle) -> Option<&mut Fence> {
        let index = handle.index() as usize;
        if index >= self.fences.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.fences[index].as_mut()
    }

    /// Check if signaled.
    pub fn is_signaled(&self, handle: FenceHandle) -> bool {
        self.get(handle).map(|f| f.is_signaled()).unwrap_or(false)
    }

    /// Signal a fence.
    pub fn signal(&self, handle: FenceHandle) {
        let timestamp = self.current_timestamp.fetch_add(1, Ordering::Relaxed);
        if let Some(fence) = self.get(handle) {
            fence.signal(timestamp);
        }
    }

    /// Reset a fence.
    pub fn reset(&self, handle: FenceHandle) {
        if let Some(fence) = self.get(handle) {
            fence.reset();
        }
    }

    /// Reset multiple fences.
    pub fn reset_fences(&self, handles: &[FenceHandle]) {
        for handle in handles {
            self.reset(*handle);
        }
    }

    /// Wait for a fence.
    pub fn wait(&self, handle: FenceHandle) {
        if let Some(fence) = self.get(handle) {
            fence.wait();
        }
    }

    /// Wait for multiple fences.
    pub fn wait_all(&self, handles: &[FenceHandle]) {
        for handle in handles {
            self.wait(*handle);
        }
    }

    /// Wait for any fence.
    pub fn wait_any(&self, handles: &[FenceHandle]) -> Option<FenceHandle> {
        loop {
            for handle in handles {
                if self.is_signaled(*handle) {
                    return Some(*handle);
                }
            }
            core::hint::spin_loop();
        }
    }

    /// Get statistics.
    pub fn statistics(&self) -> &FenceStatistics {
        &self.stats
    }

    /// Advance timestamp.
    pub fn advance_timestamp(&self) -> u64 {
        self.current_timestamp.fetch_add(1, Ordering::Relaxed)
    }
}

impl Default for FenceManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Frame Fence Ring
// ============================================================================

/// Ring buffer of fences for frame pacing.
pub struct FrameFenceRing {
    /// Fence handles.
    fences: Vec<FenceHandle>,
    /// Current index.
    current: usize,
    /// Frame count.
    frame_count: u64,
}

impl FrameFenceRing {
    /// Create a new frame fence ring.
    pub fn new(manager: &mut FenceManager, frames_in_flight: usize) -> Self {
        let fences = (0..frames_in_flight)
            .map(|_| manager.create_signaled())
            .collect();

        Self {
            fences,
            current: 0,
            frame_count: 0,
        }
    }

    /// Get current fence.
    pub fn current(&self) -> FenceHandle {
        self.fences[self.current]
    }

    /// Advance to next frame.
    pub fn advance(&mut self) -> FenceHandle {
        let handle = self.fences[self.current];
        self.current = (self.current + 1) % self.fences.len();
        self.frame_count += 1;
        handle
    }

    /// Get frame count.
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get frames in flight.
    pub fn frames_in_flight(&self) -> usize {
        self.fences.len()
    }

    /// Wait for current fence.
    pub fn wait_current(&self, manager: &FenceManager) {
        manager.wait(self.current());
    }

    /// Reset current fence.
    pub fn reset_current(&self, manager: &FenceManager) {
        manager.reset(self.current());
    }
}
