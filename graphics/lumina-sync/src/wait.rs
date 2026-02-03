//! Wait Utilities
//!
//! Advanced waiting utilities for synchronization objects.

use alloc::vec::Vec;
use core::time::Duration;

use crate::fence::FenceHandle;
use crate::semaphore::SemaphoreHandle;
use crate::timeline::TimelineSemaphoreHandle;

// ============================================================================
// Wait Result
// ============================================================================

/// Wait result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitResult {
    /// Wait completed successfully.
    Success,
    /// Wait timed out.
    Timeout,
    /// Device was lost.
    DeviceLost,
    /// Object was destroyed.
    Destroyed,
    /// Error occurred.
    Error,
}

impl WaitResult {
    /// Check if successful.
    pub fn is_success(&self) -> bool {
        *self == WaitResult::Success
    }

    /// Check if timed out.
    pub fn is_timeout(&self) -> bool {
        *self == WaitResult::Timeout
    }
}

// ============================================================================
// Wait Timeout
// ============================================================================

/// Wait timeout specification.
#[derive(Debug, Clone, Copy)]
pub enum WaitTimeout {
    /// No timeout (return immediately).
    None,
    /// Wait with timeout.
    Duration(Duration),
    /// Wait indefinitely.
    Infinite,
}

impl WaitTimeout {
    /// Create a timeout in milliseconds.
    pub fn millis(ms: u64) -> Self {
        WaitTimeout::Duration(Duration::from_millis(ms))
    }

    /// Create a timeout in seconds.
    pub fn secs(s: u64) -> Self {
        WaitTimeout::Duration(Duration::from_secs(s))
    }

    /// Create a timeout in nanoseconds.
    pub fn nanos(ns: u64) -> Self {
        WaitTimeout::Duration(Duration::from_nanos(ns))
    }

    /// Get duration or None for infinite.
    pub fn as_duration(&self) -> Option<Duration> {
        match self {
            WaitTimeout::None => Some(Duration::ZERO),
            WaitTimeout::Duration(d) => Some(*d),
            WaitTimeout::Infinite => None,
        }
    }

    /// Check if infinite.
    pub fn is_infinite(&self) -> bool {
        matches!(self, WaitTimeout::Infinite)
    }
}

impl Default for WaitTimeout {
    fn default() -> Self {
        WaitTimeout::Infinite
    }
}

impl From<Duration> for WaitTimeout {
    fn from(d: Duration) -> Self {
        WaitTimeout::Duration(d)
    }
}

// ============================================================================
// Wait All
// ============================================================================

/// Wait for all synchronization objects.
pub struct WaitAll<T> {
    /// Objects to wait on.
    pub objects: Vec<T>,
    /// Timeout.
    pub timeout: WaitTimeout,
}

impl<T> WaitAll<T> {
    /// Create a new wait all.
    pub fn new(objects: Vec<T>) -> Self {
        Self {
            objects,
            timeout: WaitTimeout::Infinite,
        }
    }

    /// Set timeout.
    pub fn with_timeout(mut self, timeout: WaitTimeout) -> Self {
        self.timeout = timeout;
        self
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }
}

impl WaitAll<FenceHandle> {
    /// Create from fence handles.
    pub fn fences(fences: &[FenceHandle]) -> Self {
        Self::new(fences.to_vec())
    }
}

impl WaitAll<SemaphoreHandle> {
    /// Create from semaphore handles.
    pub fn semaphores(semaphores: &[SemaphoreHandle]) -> Self {
        Self::new(semaphores.to_vec())
    }
}

// ============================================================================
// Wait Any
// ============================================================================

/// Wait for any synchronization object.
pub struct WaitAny<T> {
    /// Objects to wait on.
    pub objects: Vec<T>,
    /// Timeout.
    pub timeout: WaitTimeout,
}

impl<T> WaitAny<T> {
    /// Create a new wait any.
    pub fn new(objects: Vec<T>) -> Self {
        Self {
            objects,
            timeout: WaitTimeout::Infinite,
        }
    }

    /// Set timeout.
    pub fn with_timeout(mut self, timeout: WaitTimeout) -> Self {
        self.timeout = timeout;
        self
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }
}

impl WaitAny<FenceHandle> {
    /// Create from fence handles.
    pub fn fences(fences: &[FenceHandle]) -> Self {
        Self::new(fences.to_vec())
    }
}

// ============================================================================
// Timeline Wait Info
// ============================================================================

/// Timeline semaphore wait info.
#[derive(Debug, Clone, Copy)]
pub struct TimelineWaitValue {
    /// Semaphore handle.
    pub semaphore: TimelineSemaphoreHandle,
    /// Value to wait for.
    pub value: u64,
}

impl TimelineWaitValue {
    /// Create a new wait value.
    pub fn new(semaphore: TimelineSemaphoreHandle, value: u64) -> Self {
        Self { semaphore, value }
    }
}

/// Wait for timeline semaphores.
#[derive(Debug, Clone)]
pub struct TimelineWait {
    /// Wait values.
    pub values: Vec<TimelineWaitValue>,
    /// Wait all or any.
    pub wait_all: bool,
    /// Timeout.
    pub timeout: WaitTimeout,
}

impl TimelineWait {
    /// Create a wait all.
    pub fn all(values: Vec<TimelineWaitValue>) -> Self {
        Self {
            values,
            wait_all: true,
            timeout: WaitTimeout::Infinite,
        }
    }

    /// Create a wait any.
    pub fn any(values: Vec<TimelineWaitValue>) -> Self {
        Self {
            values,
            wait_all: false,
            timeout: WaitTimeout::Infinite,
        }
    }

    /// Set timeout.
    pub fn with_timeout(mut self, timeout: WaitTimeout) -> Self {
        self.timeout = timeout;
        self
    }
}

// ============================================================================
// Spin Wait
// ============================================================================

/// Spin wait configuration.
#[derive(Debug, Clone, Copy)]
pub struct SpinWaitConfig {
    /// Initial spin iterations.
    pub spin_iterations: u32,
    /// Yield iterations.
    pub yield_iterations: u32,
    /// Sleep duration after yield.
    pub sleep_duration: Duration,
}

impl Default for SpinWaitConfig {
    fn default() -> Self {
        Self {
            spin_iterations: 10,
            yield_iterations: 100,
            sleep_duration: Duration::from_micros(1),
        }
    }
}

/// Spin wait helper.
pub struct SpinWait {
    /// Configuration.
    config: SpinWaitConfig,
    /// Current count.
    count: u32,
}

impl SpinWait {
    /// Create a new spin wait.
    pub fn new() -> Self {
        Self {
            config: SpinWaitConfig::default(),
            count: 0,
        }
    }

    /// Create with configuration.
    pub fn with_config(config: SpinWaitConfig) -> Self {
        Self { config, count: 0 }
    }

    /// Spin once.
    pub fn spin_once(&mut self) {
        self.count += 1;

        if self.count <= self.config.spin_iterations {
            core::hint::spin_loop();
        } else if self.count <= self.config.yield_iterations {
            // In a real implementation, yield to scheduler
            core::hint::spin_loop();
        } else {
            // In a real implementation, sleep
            self.count = 0;
        }
    }

    /// Reset counter.
    pub fn reset(&mut self) {
        self.count = 0;
    }

    /// Get spin count.
    pub fn spin_count(&self) -> u32 {
        self.count
    }
}

impl Default for SpinWait {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Waitable
// ============================================================================

/// Trait for waitable objects.
pub trait Waitable {
    /// Check if ready.
    fn is_ready(&self) -> bool;

    /// Wait for completion.
    fn wait(&self);

    /// Wait with timeout.
    fn wait_timeout(&self, timeout: Duration) -> WaitResult;
}

// ============================================================================
// Combined Wait
// ============================================================================

/// Combined synchronization wait.
#[derive(Debug, Clone, Default)]
pub struct CombinedWait {
    /// Fence handles.
    pub fences: Vec<FenceHandle>,
    /// Binary semaphore handles.
    pub semaphores: Vec<SemaphoreHandle>,
    /// Timeline semaphore waits.
    pub timelines: Vec<TimelineWaitValue>,
    /// Wait all or any.
    pub wait_all: bool,
    /// Timeout.
    pub timeout: WaitTimeout,
}

impl CombinedWait {
    /// Create a new combined wait.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a fence.
    pub fn fence(mut self, fence: FenceHandle) -> Self {
        self.fences.push(fence);
        self
    }

    /// Add a semaphore.
    pub fn semaphore(mut self, semaphore: SemaphoreHandle) -> Self {
        self.semaphores.push(semaphore);
        self
    }

    /// Add a timeline semaphore.
    pub fn timeline(mut self, semaphore: TimelineSemaphoreHandle, value: u64) -> Self {
        self.timelines
            .push(TimelineWaitValue::new(semaphore, value));
        self
    }

    /// Set wait all.
    pub fn all(mut self) -> Self {
        self.wait_all = true;
        self
    }

    /// Set wait any.
    pub fn any(mut self) -> Self {
        self.wait_all = false;
        self
    }

    /// Set timeout.
    pub fn with_timeout(mut self, timeout: WaitTimeout) -> Self {
        self.timeout = timeout;
        self
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.fences.is_empty() && self.semaphores.is_empty() && self.timelines.is_empty()
    }

    /// Get total wait count.
    pub fn wait_count(&self) -> usize {
        self.fences.len() + self.semaphores.len() + self.timelines.len()
    }
}
