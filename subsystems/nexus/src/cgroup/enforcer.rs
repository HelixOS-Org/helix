//! Limits Enforcer
//!
//! Resource limits enforcement and OOM handling.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{CgroupId, CgroupInfo};

/// Enforcement action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforcementAction {
    /// No action needed
    None,
    /// Throttle CPU
    ThrottleCpu,
    /// Reclaim memory
    ReclaimMemory,
    /// Kill OOM
    OomKill,
    /// Throttle I/O
    ThrottleIo,
    /// Freeze cgroup
    Freeze,
    /// Block fork
    BlockFork,
}

/// Enforcement result
#[derive(Debug, Clone)]
pub struct EnforcementResult {
    /// Cgroup ID
    pub cgroup: CgroupId,
    /// Action taken
    pub action: EnforcementAction,
    /// Success
    pub success: bool,
    /// Details
    pub details: String,
    /// Timestamp
    pub timestamp: u64,
}

/// Limits enforcer
pub struct LimitsEnforcer {
    /// Enforcement history
    history: Vec<EnforcementResult>,
    /// Maximum history
    max_history: usize,
    /// Total enforcements
    total_enforcements: AtomicU64,
    /// OOM kills
    oom_kills: AtomicU64,
    /// Throttle events
    throttle_events: AtomicU64,
}

impl LimitsEnforcer {
    /// Create new limits enforcer
    pub fn new() -> Self {
        Self {
            history: Vec::with_capacity(1000),
            max_history: 1000,
            total_enforcements: AtomicU64::new(0),
            oom_kills: AtomicU64::new(0),
            throttle_events: AtomicU64::new(0),
        }
    }

    /// Check and enforce CPU limits
    pub fn enforce_cpu(&mut self, cgroup: &CgroupInfo, timestamp: u64) -> EnforcementAction {
        if !cgroup.cpu_limits.is_throttled() {
            return EnforcementAction::None;
        }

        let throttle_percent = cgroup.cpu_usage.throttle_percent();
        if throttle_percent > 50.0 {
            self.record_enforcement(
                cgroup.id,
                EnforcementAction::ThrottleCpu,
                true,
                String::from("CPU throttling active"),
                timestamp,
            );
            self.throttle_events.fetch_add(1, Ordering::Relaxed);
            return EnforcementAction::ThrottleCpu;
        }

        EnforcementAction::None
    }

    /// Check and enforce memory limits
    pub fn enforce_memory(&mut self, cgroup: &CgroupInfo, timestamp: u64) -> EnforcementAction {
        if !cgroup.memory_limits.is_limited() {
            return EnforcementAction::None;
        }

        let limit = cgroup.memory_limits.effective_limit();
        let usage = cgroup.memory_usage.usage;

        if usage >= limit {
            if cgroup.memory_limits.oom_kill_enabled {
                self.record_enforcement(
                    cgroup.id,
                    EnforcementAction::OomKill,
                    true,
                    String::from("OOM kill triggered"),
                    timestamp,
                );
                self.oom_kills.fetch_add(1, Ordering::Relaxed);
                return EnforcementAction::OomKill;
            }
        }

        if usage >= cgroup.memory_limits.high && cgroup.memory_limits.high < u64::MAX {
            self.record_enforcement(
                cgroup.id,
                EnforcementAction::ReclaimMemory,
                true,
                String::from("Memory reclaim triggered"),
                timestamp,
            );
            return EnforcementAction::ReclaimMemory;
        }

        EnforcementAction::None
    }

    /// Check and enforce PIDs limits
    pub fn enforce_pids(&mut self, cgroup: &CgroupInfo, timestamp: u64) -> EnforcementAction {
        if cgroup.pids_limits.is_at_limit() {
            self.record_enforcement(
                cgroup.id,
                EnforcementAction::BlockFork,
                true,
                String::from("PIDs limit reached, blocking fork"),
                timestamp,
            );
            return EnforcementAction::BlockFork;
        }

        EnforcementAction::None
    }

    /// Record enforcement
    fn record_enforcement(
        &mut self,
        cgroup: CgroupId,
        action: EnforcementAction,
        success: bool,
        details: String,
        timestamp: u64,
    ) {
        self.total_enforcements.fetch_add(1, Ordering::Relaxed);

        let result = EnforcementResult {
            cgroup,
            action,
            success,
            details,
            timestamp,
        };

        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(result);
    }

    /// Get enforcement history
    pub fn history(&self) -> &[EnforcementResult] {
        &self.history
    }

    /// Get total enforcements
    pub fn total_enforcements(&self) -> u64 {
        self.total_enforcements.load(Ordering::Relaxed)
    }

    /// Get OOM kills
    pub fn oom_kills(&self) -> u64 {
        self.oom_kills.load(Ordering::Relaxed)
    }

    /// Get throttle events
    pub fn throttle_events(&self) -> u64 {
        self.throttle_events.load(Ordering::Relaxed)
    }
}

impl Default for LimitsEnforcer {
    fn default() -> Self {
        Self::new()
    }
}
