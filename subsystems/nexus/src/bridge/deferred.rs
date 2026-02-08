//! # Bridge Deferred Syscall Engine
//!
//! Deferred and asynchronous syscall execution:
//! - Batch syscalls for deferred execution
//! - Priority-based deferred queue
//! - Deadline-aware scheduling
//! - Completion notification
//! - Resource-aware deferral decisions

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Deferral reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeferralReason {
    /// Resource temporarily unavailable
    ResourceBusy,
    /// Batching optimization
    BatchOptimize,
    /// Low priority, yield to urgent work
    LowPriority,
    /// Rate limited
    RateLimited,
    /// Waiting for dependency
    DependencyWait,
    /// User requested async
    UserAsync,
}

/// Deferred syscall state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeferredState {
    /// Waiting in queue
    Queued,
    /// Being executed
    Executing,
    /// Completed successfully
    Completed,
    /// Failed
    Failed,
    /// Cancelled
    Cancelled,
    /// Expired (deadline passed)
    Expired,
}

/// Deferred syscall entry
#[derive(Debug, Clone)]
pub struct DeferredSyscall {
    /// Unique ID
    pub id: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// PID
    pub pid: u64,
    /// Arguments (up to 6)
    pub args: [u64; 6],
    /// Priority (lower = higher priority)
    pub priority: u32,
    /// Submission time (ns)
    pub submit_ns: u64,
    /// Deadline (ns, 0 = no deadline)
    pub deadline_ns: u64,
    /// State
    pub state: DeferredState,
    /// Deferral reason
    pub reason: DeferralReason,
    /// Result (if completed)
    pub result: Option<i64>,
    /// Execution latency (ns)
    pub exec_latency_ns: u64,
    /// Retry count
    pub retries: u32,
    /// Max retries
    pub max_retries: u32,
}

impl DeferredSyscall {
    pub fn new(id: u64, syscall_nr: u32, pid: u64, reason: DeferralReason) -> Self {
        Self {
            id,
            syscall_nr,
            pid,
            args: [0; 6],
            priority: 100,
            submit_ns: 0,
            deadline_ns: 0,
            state: DeferredState::Queued,
            reason,
            result: None,
            exec_latency_ns: 0,
            retries: 0,
            max_retries: 3,
        }
    }

    /// Is expired?
    pub fn is_expired(&self, now_ns: u64) -> bool {
        self.deadline_ns > 0 && now_ns > self.deadline_ns
    }

    /// Can retry?
    pub fn can_retry(&self) -> bool {
        self.retries < self.max_retries
    }

    /// Time in queue (ns)
    pub fn queue_time(&self, now_ns: u64) -> u64 {
        now_ns.saturating_sub(self.submit_ns)
    }
}

/// Completion notification
#[derive(Debug, Clone)]
pub struct DeferredCompletion {
    /// Deferred ID
    pub id: u64,
    /// PID to notify
    pub pid: u64,
    /// Result
    pub result: i64,
    /// Was it successful
    pub success: bool,
}

/// Deferred queue stats
#[derive(Debug, Clone, Default)]
pub struct BridgeDeferredStats {
    pub queued: usize,
    pub executing: usize,
    pub completed: u64,
    pub failed: u64,
    pub expired: u64,
    pub cancelled: u64,
    pub avg_queue_time_ns: f64,
}

/// Bridge deferred syscall engine
pub struct BridgeDeferredEngine {
    /// Queued syscalls
    queue: Vec<DeferredSyscall>,
    /// Completed notifications
    completions: Vec<DeferredCompletion>,
    /// Next ID
    next_id: u64,
    /// Max queue depth
    pub max_queue: usize,
    /// Total completed
    total_completed: u64,
    /// Total failed
    total_failed: u64,
    /// Total expired
    total_expired: u64,
    /// Queue time accumulator
    queue_time_sum: f64,
    /// Queue time count
    queue_time_count: u64,
    /// Stats
    stats: BridgeDeferredStats,
}

impl BridgeDeferredEngine {
    pub fn new(max_queue: usize) -> Self {
        Self {
            queue: Vec::new(),
            completions: Vec::new(),
            next_id: 1,
            max_queue,
            total_completed: 0,
            total_failed: 0,
            total_expired: 0,
            queue_time_sum: 0.0,
            queue_time_count: 0,
            stats: BridgeDeferredStats::default(),
        }
    }

    /// Defer a syscall
    pub fn defer(&mut self, syscall_nr: u32, pid: u64, args: [u64; 6],
                 reason: DeferralReason, now_ns: u64) -> Option<u64> {
        if self.queue.len() >= self.max_queue {
            return None;
        }
        let id = self.next_id;
        self.next_id += 1;
        let mut entry = DeferredSyscall::new(id, syscall_nr, pid, reason);
        entry.args = args;
        entry.submit_ns = now_ns;
        self.queue.push(entry);
        self.update_stats();
        Some(id)
    }

    /// Set deadline
    pub fn set_deadline(&mut self, id: u64, deadline_ns: u64) {
        if let Some(entry) = self.queue.iter_mut().find(|e| e.id == id) {
            entry.deadline_ns = deadline_ns;
        }
    }

    /// Set priority
    pub fn set_priority(&mut self, id: u64, priority: u32) {
        if let Some(entry) = self.queue.iter_mut().find(|e| e.id == id) {
            entry.priority = priority;
        }
    }

    /// Cancel a deferred syscall
    pub fn cancel(&mut self, id: u64) -> bool {
        if let Some(entry) = self.queue.iter_mut().find(|e| e.id == id && e.state == DeferredState::Queued) {
            entry.state = DeferredState::Cancelled;
            return true;
        }
        false
    }

    /// Dequeue next (highest priority, respecting deadlines)
    pub fn dequeue_next(&mut self, now_ns: u64) -> Option<DeferredSyscall> {
        // First, expire any past-deadline entries
        for entry in &mut self.queue {
            if entry.state == DeferredState::Queued && entry.is_expired(now_ns) {
                entry.state = DeferredState::Expired;
                self.total_expired += 1;
            }
        }

        // Find highest priority queued entry
        let mut best_idx = None;
        let mut best_priority = u32::MAX;
        for (i, entry) in self.queue.iter().enumerate() {
            if entry.state == DeferredState::Queued && entry.priority < best_priority {
                best_priority = entry.priority;
                best_idx = Some(i);
            }
        }

        if let Some(idx) = best_idx {
            let mut entry = self.queue.remove(idx);
            let qt = entry.queue_time(now_ns) as f64;
            self.queue_time_sum += qt;
            self.queue_time_count += 1;
            entry.state = DeferredState::Executing;
            self.update_stats();
            Some(entry)
        } else {
            None
        }
    }

    /// Record completion
    pub fn complete(&mut self, id: u64, pid: u64, result: i64, success: bool) {
        if success {
            self.total_completed += 1;
        } else {
            self.total_failed += 1;
        }
        self.completions.push(DeferredCompletion { id, pid, result, success });
        self.update_stats();
    }

    /// Drain completions
    pub fn drain_completions(&mut self) -> Vec<DeferredCompletion> {
        core::mem::take(&mut self.completions)
    }

    /// Clean up finished entries
    pub fn cleanup(&mut self) {
        self.queue.retain(|e| matches!(e.state, DeferredState::Queued | DeferredState::Executing));
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.queued = self.queue.iter().filter(|e| e.state == DeferredState::Queued).count();
        self.stats.executing = self.queue.iter().filter(|e| e.state == DeferredState::Executing).count();
        self.stats.completed = self.total_completed;
        self.stats.failed = self.total_failed;
        self.stats.expired = self.total_expired;
        self.stats.avg_queue_time_ns = if self.queue_time_count > 0 {
            self.queue_time_sum / self.queue_time_count as f64
        } else {
            0.0
        };
    }

    pub fn stats(&self) -> &BridgeDeferredStats {
        &self.stats
    }
}
