//! # Orchestration Engine
//!
//! Coordinates optimization actions across all subsystems,
//! ensuring that actions don't conflict and are applied in the correct order.

use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// OPTIMIZATION ACTIONS
// ============================================================================

/// An optimization action to execute
#[derive(Debug, Clone)]
pub enum OptAction {
    /// Adjust CPU scheduling parameters for a process
    AdjustCpuScheduling {
        pid: u64,
        priority_delta: i8,
        affinity_mask: Option<u64>,
    },
    /// Adjust memory parameters for a process
    AdjustMemory {
        pid: u64,
        reclaim_bytes: u64,
        enable_huge_pages: bool,
    },
    /// Throttle I/O for a process
    ThrottleIo {
        pid: u64,
        max_iops: u64,
        max_bps: u64,
    },
    /// Migrate process to a different CPU
    MigrateCpu { pid: u64, target_cpu: u32 },
    /// Send advisory to cooperative process
    SendAdvisory {
        pid: u64,
        session_id: u64,
        pressure_level: u8,
    },
    /// Compact memory
    CompactMemory { urgency: u8 },
    /// Adjust global scheduler parameters
    AdjustScheduler {
        quantum_us: u64,
        preemption_threshold: u8,
    },
    /// Activate power-saving mode
    PowerSaving { level: u8 },
    /// Rebalance resources across all processes
    GlobalRebalance,
}

/// Priority of an action (lower = higher priority)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ActionPriority(pub u8);

impl ActionPriority {
    pub const CRITICAL: Self = Self(0);
    pub const HIGH: Self = Self(1);
    pub const MEDIUM: Self = Self(2);
    pub const LOW: Self = Self(3);
    pub const BACKGROUND: Self = Self(4);
}

/// A queued optimization action
#[derive(Debug, Clone)]
pub struct QueuedAction {
    /// The action to perform
    pub action: OptAction,
    /// Priority
    pub priority: ActionPriority,
    /// Timestamp when enqueued
    pub enqueued_at: u64,
    /// Deadline (0 = no deadline)
    pub deadline: u64,
    /// Source subsystem
    pub source: ActionSource,
}

/// What subsystem generated this action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionSource {
    /// From bridge/syscall analysis
    Bridge,
    /// From apps/profiling
    Apps,
    /// From coop/negotiation
    Coop,
    /// From holistic/policy engine
    Policy,
    /// From holistic/prediction
    Prediction,
    /// From holistic/balancer
    Balancer,
}

/// Result of executing an action
#[derive(Debug, Clone, Copy)]
pub struct ActionResult {
    pub success: bool,
    pub latency_us: u64,
}

// ============================================================================
// ORCHESTRATOR
// ============================================================================

const MAX_QUEUED_ACTIONS: usize = 512;

/// The orchestrator coordinates all optimization actions
pub struct Orchestrator {
    /// Action queue sorted by priority
    queue: VecDeque<QueuedAction>,
    /// Actions executed
    executed: u64,
    /// Actions dropped (deadline missed or overflow)
    dropped: u64,
    /// Actions that succeeded
    succeeded: u64,
    /// Actions that failed
    failed: u64,
    /// Whether the orchestrator is paused
    paused: bool,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            executed: 0,
            dropped: 0,
            succeeded: 0,
            failed: 0,
            paused: false,
        }
    }

    /// Enqueue an optimization action
    pub fn enqueue(&mut self, action: OptAction, priority: ActionPriority, source: ActionSource) {
        if self.queue.len() >= MAX_QUEUED_ACTIONS {
            // Drop lowest priority action
            if let Some(worst_idx) = self
                .queue
                .iter()
                .enumerate()
                .max_by_key(|(_, a)| a.priority)
                .map(|(i, _)| i)
            {
                if self.queue[worst_idx].priority > priority {
                    self.queue.remove(worst_idx);
                    self.dropped += 1;
                } else {
                    self.dropped += 1;
                    return;
                }
            }
        }

        let queued = QueuedAction {
            action,
            priority,
            enqueued_at: 0,
            deadline: 0,
            source,
        };

        // Insert in priority order
        let pos = self
            .queue
            .iter()
            .position(|a| a.priority > priority)
            .unwrap_or(self.queue.len());
        self.queue.insert(pos, queued);
    }

    /// Drain ready actions up to a limit
    pub fn drain_actions(&mut self, max: usize) -> Vec<QueuedAction> {
        if self.paused {
            return Vec::new();
        }
        let count = max.min(self.queue.len());
        let mut actions = Vec::with_capacity(count);
        for _ in 0..count {
            if let Some(action) = self.queue.pop_front() {
                actions.push(action);
            }
        }
        self.executed += actions.len() as u64;
        actions
    }

    /// Record execution result
    pub fn record_result(&mut self, result: ActionResult) {
        if result.success {
            self.succeeded += 1;
        } else {
            self.failed += 1;
        }
    }

    /// Drop expired actions
    pub fn drop_expired(&mut self, current_time: u64) -> usize {
        let before = self.queue.len();
        self.queue
            .retain(|a| a.deadline == 0 || a.deadline > current_time);
        let dropped = before - self.queue.len();
        self.dropped += dropped as u64;
        dropped
    }

    /// Pending action count
    pub fn pending_actions(&self) -> usize {
        self.queue.len()
    }

    /// Pause the orchestrator
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume the orchestrator
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Get execution statistics
    pub fn stats(&self) -> (u64, u64, u64, u64) {
        (self.executed, self.succeeded, self.failed, self.dropped)
    }
}
