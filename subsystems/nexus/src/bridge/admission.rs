//! # Bridge Admission Controller
//!
//! Syscall admission control and load shedding:
//! - Request admission decisions
//! - Load-based shedding
//! - Priority-based admission
//! - Fair queuing
//! - Overload protection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// ADMISSION TYPES
// ============================================================================

/// Admission decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionDecision {
    /// Admit (process normally)
    Admit,
    /// Queue (wait in line)
    Queue,
    /// Throttle (slow down)
    Throttle,
    /// Shed (reject)
    Shed,
}

/// Admission priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdmissionPriority {
    /// Background (sheddable)
    Background = 0,
    /// Low
    Low = 1,
    /// Normal
    Normal = 2,
    /// High
    High = 3,
    /// Critical (never shed)
    Critical = 4,
}

/// Load level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadLevel {
    /// Normal
    Normal,
    /// Elevated
    Elevated,
    /// High
    High,
    /// Overloaded
    Overloaded,
}

impl LoadLevel {
    /// From utilization
    pub fn from_utilization(util: f64) -> Self {
        if util < 0.7 {
            Self::Normal
        } else if util < 0.85 {
            Self::Elevated
        } else if util < 0.95 {
            Self::High
        } else {
            Self::Overloaded
        }
    }

    /// Min priority to admit
    pub fn min_admit_priority(&self) -> AdmissionPriority {
        match self {
            Self::Normal => AdmissionPriority::Background,
            Self::Elevated => AdmissionPriority::Low,
            Self::High => AdmissionPriority::Normal,
            Self::Overloaded => AdmissionPriority::Critical,
        }
    }
}

// ============================================================================
// QUEUE ENTRY
// ============================================================================

/// Queued request
#[derive(Debug, Clone)]
pub struct QueuedRequest {
    /// Request id
    pub id: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// Process id
    pub pid: u64,
    /// Priority
    pub priority: AdmissionPriority,
    /// Enqueue time
    pub enqueued_at: u64,
    /// Deadline (if any)
    pub deadline_ns: Option<u64>,
}

impl QueuedRequest {
    /// Wait time
    pub fn wait_time(&self, now: u64) -> u64 {
        now.saturating_sub(self.enqueued_at)
    }

    /// Is expired?
    pub fn is_expired(&self, now: u64) -> bool {
        self.deadline_ns.map(|d| now >= d).unwrap_or(false)
    }
}

// ============================================================================
// ADMISSION CONFIG
// ============================================================================

/// Admission configuration
#[derive(Debug, Clone)]
pub struct AdmissionConfig {
    /// Max concurrent requests
    pub max_concurrent: usize,
    /// Max queue depth
    pub max_queue_depth: usize,
    /// Queue timeout (ns)
    pub queue_timeout_ns: u64,
    /// Enable load-based shedding
    pub load_shedding: bool,
    /// Shedding starts at this utilization
    pub shed_threshold: f64,
}

impl AdmissionConfig {
    pub fn default_config() -> Self {
        Self {
            max_concurrent: 1024,
            max_queue_depth: 4096,
            queue_timeout_ns: 500_000_000, // 500ms
            load_shedding: true,
            shed_threshold: 0.9,
        }
    }
}

// ============================================================================
// PROCESS CREDIT
// ============================================================================

/// Per-process admission credit
#[derive(Debug, Clone)]
pub struct ProcessCredit {
    /// Process id
    pub pid: u64,
    /// Credits remaining
    pub credits: u32,
    /// Max credits
    pub max_credits: u32,
    /// Requests admitted this window
    pub admitted_count: u64,
    /// Requests shed this window
    pub shed_count: u64,
    /// Last refill time
    pub last_refill: u64,
    /// Refill interval (ns)
    pub refill_interval_ns: u64,
}

impl ProcessCredit {
    pub fn new(pid: u64, max_credits: u32) -> Self {
        Self {
            pid,
            credits: max_credits,
            max_credits,
            admitted_count: 0,
            shed_count: 0,
            last_refill: 0,
            refill_interval_ns: 1_000_000_000, // 1s
        }
    }

    /// Try consume credit
    pub fn try_consume(&mut self, now: u64) -> bool {
        self.maybe_refill(now);
        if self.credits > 0 {
            self.credits -= 1;
            self.admitted_count += 1;
            true
        } else {
            self.shed_count += 1;
            false
        }
    }

    fn maybe_refill(&mut self, now: u64) {
        if now.saturating_sub(self.last_refill) >= self.refill_interval_ns {
            self.credits = self.max_credits;
            self.last_refill = now;
        }
    }

    /// Admission rate
    pub fn admission_rate(&self) -> f64 {
        let total = self.admitted_count + self.shed_count;
        if total == 0 {
            return 1.0;
        }
        self.admitted_count as f64 / total as f64
    }
}

// ============================================================================
// ADMISSION CONTROLLER
// ============================================================================

/// Admission stats
#[derive(Debug, Clone, Default)]
pub struct BridgeAdmissionStats {
    /// Requests admitted
    pub admitted: u64,
    /// Requests queued
    pub queued: u64,
    /// Requests shed
    pub shed: u64,
    /// Requests throttled
    pub throttled: u64,
    /// Current concurrent
    pub current_concurrent: usize,
    /// Current queue depth
    pub queue_depth: usize,
    /// Current load level
    pub load_level: u8,
    /// Expired from queue
    pub expired: u64,
}

/// Bridge admission controller
pub struct BridgeAdmissionController {
    /// Config
    config: AdmissionConfig,
    /// Current concurrent count
    concurrent: usize,
    /// Queue
    queue: Vec<QueuedRequest>,
    /// Per-process credits
    credits: BTreeMap<u64, ProcessCredit>,
    /// Current utilization
    utilization: f64,
    /// Next request id
    next_id: u64,
    /// Stats
    stats: BridgeAdmissionStats,
}

impl BridgeAdmissionController {
    pub fn new(config: AdmissionConfig) -> Self {
        Self {
            config,
            concurrent: 0,
            queue: Vec::new(),
            credits: BTreeMap::new(),
            utilization: 0.0,
            next_id: 1,
            stats: BridgeAdmissionStats::default(),
        }
    }

    /// Update utilization
    pub fn update_utilization(&mut self, util: f64) {
        self.utilization = util;
        self.stats.load_level = LoadLevel::from_utilization(util) as u8;
    }

    /// Set credits for process
    pub fn set_credits(&mut self, pid: u64, max_credits: u32) {
        self.credits
            .insert(pid, ProcessCredit::new(pid, max_credits));
    }

    /// Admit syscall
    pub fn admit(
        &mut self,
        syscall_nr: u32,
        pid: u64,
        priority: AdmissionPriority,
        now: u64,
    ) -> AdmissionDecision {
        let load = LoadLevel::from_utilization(self.utilization);
        let min_priority = load.min_admit_priority();

        // Critical always admitted
        if matches!(priority, AdmissionPriority::Critical) {
            self.concurrent += 1;
            self.stats.admitted += 1;
            self.stats.current_concurrent = self.concurrent;
            return AdmissionDecision::Admit;
        }

        // Load shedding
        if self.config.load_shedding && priority < min_priority {
            self.stats.shed += 1;
            return AdmissionDecision::Shed;
        }

        // Credit check
        if let Some(credit) = self.credits.get_mut(&pid) {
            if !credit.try_consume(now) {
                self.stats.throttled += 1;
                return AdmissionDecision::Throttle;
            }
        }

        // Capacity check
        if self.concurrent < self.config.max_concurrent {
            self.concurrent += 1;
            self.stats.admitted += 1;
            self.stats.current_concurrent = self.concurrent;
            return AdmissionDecision::Admit;
        }

        // Queue if possible
        if self.queue.len() < self.config.max_queue_depth {
            let req = QueuedRequest {
                id: self.next_id,
                syscall_nr,
                pid,
                priority,
                enqueued_at: now,
                deadline_ns: Some(now + self.config.queue_timeout_ns),
            };
            self.next_id += 1;
            self.queue.push(req);
            self.stats.queued += 1;
            self.stats.queue_depth = self.queue.len();
            return AdmissionDecision::Queue;
        }

        // Shed
        self.stats.shed += 1;
        AdmissionDecision::Shed
    }

    /// Release (syscall completed)
    pub fn release(&mut self) {
        if self.concurrent > 0 {
            self.concurrent -= 1;
        }
        self.stats.current_concurrent = self.concurrent;
    }

    /// Dequeue next (if capacity available)
    pub fn dequeue(&mut self, now: u64) -> Option<QueuedRequest> {
        // Expire old entries
        let before = self.queue.len();
        self.queue.retain(|r| !r.is_expired(now));
        self.stats.expired += (before - self.queue.len()) as u64;

        if self.concurrent >= self.config.max_concurrent {
            return None;
        }

        if self.queue.is_empty() {
            return None;
        }

        // Find highest priority
        let mut best_idx = 0;
        let mut best_pri = self.queue[0].priority;
        for (i, req) in self.queue.iter().enumerate().skip(1) {
            if req.priority > best_pri {
                best_pri = req.priority;
                best_idx = i;
            }
        }

        let req = self.queue.remove(best_idx);
        self.concurrent += 1;
        self.stats.current_concurrent = self.concurrent;
        self.stats.queue_depth = self.queue.len();
        Some(req)
    }

    /// Stats
    pub fn stats(&self) -> &BridgeAdmissionStats {
        &self.stats
    }
}
