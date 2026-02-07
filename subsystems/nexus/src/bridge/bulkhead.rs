//! # Bridge Bulkhead Pattern
//!
//! Bulkhead isolation for syscall processing:
//! - Resource partitioning per syscall class
//! - Failure isolation between bulkheads
//! - Capacity limiting per partition
//! - Overflow handling
//! - Health monitoring per bulkhead

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// BULKHEAD TYPES
// ============================================================================

/// Bulkhead class (partition category)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BulkheadClass {
    /// File system operations
    FileSystem,
    /// Network operations
    Network,
    /// Memory management
    Memory,
    /// Process management
    Process,
    /// IPC operations
    Ipc,
    /// Device I/O
    DeviceIo,
    /// Security/auth
    Security,
    /// Misc/other
    Other,
}

/// Bulkhead state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkheadState {
    /// Operating normally
    Normal,
    /// Approaching capacity
    Warning,
    /// At capacity, rejecting new requests
    Full,
    /// Failed, all requests rejected
    Failed,
    /// Draining (graceful shutdown)
    Draining,
}

/// Overflow policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowPolicy {
    /// Reject immediately
    Reject,
    /// Queue up to limit
    Queue,
    /// Redirect to another bulkhead
    Redirect,
    /// Drop oldest
    DropOldest,
}

// ============================================================================
// BULKHEAD PARTITION
// ============================================================================

/// A bulkhead partition
#[derive(Debug)]
pub struct Bulkhead {
    /// Class
    pub class: BulkheadClass,
    /// State
    pub state: BulkheadState,
    /// Max concurrent requests
    pub max_concurrent: u32,
    /// Current in-flight
    pub in_flight: u32,
    /// Queue depth
    pub queue_depth: u32,
    /// Max queue
    pub max_queue: u32,
    /// Total accepted
    pub total_accepted: u64,
    /// Total rejected
    pub total_rejected: u64,
    /// Total completed
    pub total_completed: u64,
    /// Total failed
    pub total_failed: u64,
    /// Overflow policy
    pub overflow_policy: OverflowPolicy,
    /// Average latency (ns) EMA
    pub avg_latency_ns: f64,
}

impl Bulkhead {
    pub fn new(class: BulkheadClass, max_concurrent: u32, max_queue: u32) -> Self {
        Self {
            class,
            state: BulkheadState::Normal,
            max_concurrent,
            in_flight: 0,
            queue_depth: 0,
            max_queue,
            total_accepted: 0,
            total_rejected: 0,
            total_completed: 0,
            total_failed: 0,
            overflow_policy: OverflowPolicy::Reject,
            avg_latency_ns: 0.0,
        }
    }

    /// Try to acquire a slot
    pub fn try_acquire(&mut self) -> bool {
        if self.state == BulkheadState::Failed || self.state == BulkheadState::Draining {
            self.total_rejected += 1;
            return false;
        }
        if self.in_flight < self.max_concurrent {
            self.in_flight += 1;
            self.total_accepted += 1;
            self.update_state();
            return true;
        }
        // At capacity
        match self.overflow_policy {
            OverflowPolicy::Queue => {
                if self.queue_depth < self.max_queue {
                    self.queue_depth += 1;
                    self.total_accepted += 1;
                    self.update_state();
                    true
                } else {
                    self.total_rejected += 1;
                    false
                }
            }
            _ => {
                self.total_rejected += 1;
                false
            }
        }
    }

    /// Release a slot
    pub fn release(&mut self, success: bool, latency_ns: u64) {
        if self.in_flight > 0 {
            self.in_flight -= 1;
        }
        if success {
            self.total_completed += 1;
        } else {
            self.total_failed += 1;
        }
        // EMA latency
        let alpha = 0.1;
        self.avg_latency_ns = alpha * latency_ns as f64 + (1.0 - alpha) * self.avg_latency_ns;
        // Dequeue if possible
        if self.queue_depth > 0 && self.in_flight < self.max_concurrent {
            self.queue_depth -= 1;
            self.in_flight += 1;
        }
        self.update_state();
    }

    /// Utilization
    pub fn utilization(&self) -> f64 {
        if self.max_concurrent == 0 {
            return 0.0;
        }
        self.in_flight as f64 / self.max_concurrent as f64
    }

    /// Error rate
    pub fn error_rate(&self) -> f64 {
        let total = self.total_completed + self.total_failed;
        if total == 0 {
            return 0.0;
        }
        self.total_failed as f64 / total as f64
    }

    /// Rejection rate
    pub fn rejection_rate(&self) -> f64 {
        let total = self.total_accepted + self.total_rejected;
        if total == 0 {
            return 0.0;
        }
        self.total_rejected as f64 / total as f64
    }

    fn update_state(&mut self) {
        if self.state == BulkheadState::Draining {
            return;
        }
        if self.error_rate() > 0.5 && self.total_completed + self.total_failed > 10 {
            self.state = BulkheadState::Failed;
        } else if self.in_flight >= self.max_concurrent {
            self.state = BulkheadState::Full;
        } else if self.utilization() > 0.8 {
            self.state = BulkheadState::Warning;
        } else {
            self.state = BulkheadState::Normal;
        }
    }

    /// Reset failure state
    pub fn reset(&mut self) {
        self.state = BulkheadState::Normal;
        self.total_failed = 0;
        self.total_rejected = 0;
    }
}

// ============================================================================
// BULKHEAD ENGINE
// ============================================================================

/// Bulkhead stats
#[derive(Debug, Clone, Default)]
pub struct BridgeBulkheadStats {
    /// Active bulkheads
    pub active_bulkheads: usize,
    /// Total in-flight
    pub total_in_flight: u32,
    /// Failed bulkheads
    pub failed_bulkheads: usize,
    /// Total rejections
    pub total_rejections: u64,
}

/// Bridge bulkhead manager
pub struct BridgeBulkheadManager {
    /// Bulkheads
    bulkheads: BTreeMap<u8, Bulkhead>,
    /// Stats
    stats: BridgeBulkheadStats,
}

impl BridgeBulkheadManager {
    pub fn new() -> Self {
        Self {
            bulkheads: BTreeMap::new(),
            stats: BridgeBulkheadStats::default(),
        }
    }

    /// Register bulkhead
    pub fn register(&mut self, class: BulkheadClass, max_concurrent: u32, max_queue: u32) {
        let key = class as u8;
        self.bulkheads.insert(key, Bulkhead::new(class, max_concurrent, max_queue));
        self.update_stats();
    }

    /// Acquire slot
    pub fn acquire(&mut self, class: BulkheadClass) -> bool {
        let key = class as u8;
        if let Some(bh) = self.bulkheads.get_mut(&key) {
            let result = bh.try_acquire();
            self.update_stats();
            result
        } else {
            false
        }
    }

    /// Release slot
    pub fn release(&mut self, class: BulkheadClass, success: bool, latency_ns: u64) {
        let key = class as u8;
        if let Some(bh) = self.bulkheads.get_mut(&key) {
            bh.release(success, latency_ns);
            self.update_stats();
        }
    }

    /// Get bulkhead
    pub fn bulkhead(&self, class: BulkheadClass) -> Option<&Bulkhead> {
        self.bulkheads.get(&(class as u8))
    }

    /// Failed bulkheads
    pub fn failed(&self) -> Vec<BulkheadClass> {
        self.bulkheads.values()
            .filter(|b| b.state == BulkheadState::Failed)
            .map(|b| b.class)
            .collect()
    }

    fn update_stats(&mut self) {
        self.stats.active_bulkheads = self.bulkheads.len();
        self.stats.total_in_flight = self.bulkheads.values().map(|b| b.in_flight).sum();
        self.stats.failed_bulkheads = self.bulkheads.values()
            .filter(|b| b.state == BulkheadState::Failed).count();
        self.stats.total_rejections = self.bulkheads.values().map(|b| b.total_rejected).sum();
    }

    /// Stats
    pub fn stats(&self) -> &BridgeBulkheadStats {
        &self.stats
    }
}
