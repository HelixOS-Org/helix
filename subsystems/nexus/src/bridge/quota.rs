//! # Bridge Quota Enforcer
//!
//! Per-process and per-group syscall quota enforcement:
//! - Time-windowed quota tracking
//! - Hierarchical quota limits
//! - Burst allowance with penalty
//! - Quota transfer between processes
//! - Usage forecasting for proactive throttling

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// QUOTA TYPES
// ============================================================================

/// Quota resource
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaResource {
    /// Syscalls per second
    SyscallRate,
    /// CPU time (ns)
    CpuTime,
    /// Memory allocations
    MemoryAlloc,
    /// File operations
    FileOps,
    /// Network operations
    NetworkOps,
    /// IPC messages
    IpcMessages,
}

/// Quota state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaState {
    /// Under quota
    UnderLimit,
    /// Near limit (>80%)
    NearLimit,
    /// At limit
    AtLimit,
    /// Over limit (burst)
    Burst,
    /// Hard limit exceeded
    Exceeded,
}

/// Enforcement action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaAction {
    /// Allow
    Allow,
    /// Allow but warn
    AllowWarn,
    /// Throttle (delay)
    Throttle,
    /// Deny
    Deny,
}

// ============================================================================
// QUOTA DEFINITION
// ============================================================================

/// Quota definition
#[derive(Debug, Clone)]
pub struct QuotaDefinition {
    /// Resource type
    pub resource: QuotaResource,
    /// Limit per window
    pub limit: u64,
    /// Window duration (ns)
    pub window_ns: u64,
    /// Burst allowance (extra above limit)
    pub burst_allowance: u64,
    /// Hard limit (absolute max)
    pub hard_limit: u64,
    /// Penalty duration for burst (ns)
    pub burst_penalty_ns: u64,
}

impl QuotaDefinition {
    pub fn new(resource: QuotaResource, limit: u64, window_ns: u64) -> Self {
        Self {
            resource,
            limit,
            window_ns,
            burst_allowance: limit / 10, // 10% burst
            hard_limit: limit + limit / 5, // 20% hard cap
            burst_penalty_ns: window_ns / 2,
        }
    }
}

// ============================================================================
// QUOTA TRACKER
// ============================================================================

/// Windowed usage tracker
#[derive(Debug, Clone)]
pub struct WindowedUsage {
    /// Window start
    pub window_start_ns: u64,
    /// Window duration
    pub window_ns: u64,
    /// Current usage
    pub current_usage: u64,
    /// Previous window usage
    pub prev_usage: u64,
    /// Peak usage (across windows)
    pub peak_usage: u64,
    /// Windows completed
    pub windows_completed: u64,
}

impl WindowedUsage {
    pub fn new(window_ns: u64, now: u64) -> Self {
        Self {
            window_start_ns: now,
            window_ns,
            current_usage: 0,
            prev_usage: 0,
            peak_usage: 0,
            windows_completed: 0,
        }
    }

    /// Check and rotate window
    fn maybe_rotate(&mut self, now: u64) {
        if now >= self.window_start_ns + self.window_ns {
            if self.current_usage > self.peak_usage {
                self.peak_usage = self.current_usage;
            }
            self.prev_usage = self.current_usage;
            self.current_usage = 0;
            self.window_start_ns = now;
            self.windows_completed += 1;
        }
    }

    /// Record usage
    pub fn record(&mut self, amount: u64, now: u64) {
        self.maybe_rotate(now);
        self.current_usage += amount;
    }

    /// Current usage ratio (0.0-1.0+)
    pub fn usage_ratio(&self, limit: u64) -> f64 {
        if limit == 0 {
            return 0.0;
        }
        self.current_usage as f64 / limit as f64
    }

    /// Forecast usage at end of window (linear extrapolation)
    pub fn forecast_end_of_window(&self, now: u64) -> u64 {
        let elapsed = now.saturating_sub(self.window_start_ns);
        if elapsed == 0 {
            return self.current_usage;
        }
        let rate = self.current_usage as f64 / elapsed as f64;
        (rate * self.window_ns as f64) as u64
    }
}

/// Per-process quota state
#[derive(Debug)]
pub struct ProcessQuota {
    /// Process ID
    pub pid: u64,
    /// Definitions
    definitions: BTreeMap<u8, QuotaDefinition>,
    /// Usage trackers
    usage: BTreeMap<u8, WindowedUsage>,
    /// Burst penalty until (ns)
    burst_penalty_until: u64,
    /// Total violations
    pub total_violations: u64,
}

impl ProcessQuota {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            definitions: BTreeMap::new(),
            usage: BTreeMap::new(),
            burst_penalty_until: 0,
            total_violations: 0,
        }
    }

    /// Set quota
    pub fn set_quota(&mut self, def: QuotaDefinition, now: u64) {
        let key = def.resource as u8;
        let window_ns = def.window_ns;
        self.definitions.insert(key, def);
        self.usage.entry(key).or_insert_with(|| WindowedUsage::new(window_ns, now));
    }

    /// Check and record usage
    pub fn check_and_record(&mut self, resource: QuotaResource, amount: u64, now: u64) -> QuotaAction {
        let key = resource as u8;
        let def = match self.definitions.get(&key) {
            Some(d) => d.clone(),
            None => return QuotaAction::Allow, // no quota set
        };

        let tracker = match self.usage.get_mut(&key) {
            Some(t) => t,
            None => return QuotaAction::Allow,
        };

        tracker.record(amount, now);

        // Check burst penalty
        if now < self.burst_penalty_until {
            if tracker.current_usage > def.limit * 8 / 10 {
                return QuotaAction::Throttle;
            }
        }

        let usage = tracker.current_usage;
        if usage > def.hard_limit {
            self.total_violations += 1;
            QuotaAction::Deny
        } else if usage > def.limit + def.burst_allowance {
            self.total_violations += 1;
            self.burst_penalty_until = now + def.burst_penalty_ns;
            QuotaAction::Deny
        } else if usage > def.limit {
            QuotaAction::Throttle
        } else if usage > def.limit * 8 / 10 {
            QuotaAction::AllowWarn
        } else {
            QuotaAction::Allow
        }
    }

    /// Get state for resource
    pub fn state(&self, resource: QuotaResource) -> QuotaState {
        let key = resource as u8;
        let def = match self.definitions.get(&key) {
            Some(d) => d,
            None => return QuotaState::UnderLimit,
        };
        let tracker = match self.usage.get(&key) {
            Some(t) => t,
            None => return QuotaState::UnderLimit,
        };
        let usage = tracker.current_usage;
        if usage > def.hard_limit {
            QuotaState::Exceeded
        } else if usage > def.limit + def.burst_allowance {
            QuotaState::Exceeded
        } else if usage > def.limit {
            QuotaState::Burst
        } else if usage == def.limit {
            QuotaState::AtLimit
        } else if usage > def.limit * 8 / 10 {
            QuotaState::NearLimit
        } else {
            QuotaState::UnderLimit
        }
    }
}

// ============================================================================
// GROUP QUOTA
// ============================================================================

/// Group quota (hierarchical)
#[derive(Debug)]
pub struct GroupQuota {
    /// Group ID
    pub group_id: u64,
    /// Member PIDs
    pub members: Vec<u64>,
    /// Aggregate usage
    aggregate: BTreeMap<u8, WindowedUsage>,
    /// Group limits
    limits: BTreeMap<u8, QuotaDefinition>,
}

impl GroupQuota {
    pub fn new(group_id: u64) -> Self {
        Self {
            group_id,
            members: Vec::new(),
            aggregate: BTreeMap::new(),
            limits: BTreeMap::new(),
        }
    }

    /// Add member
    pub fn add_member(&mut self, pid: u64) {
        if !self.members.contains(&pid) {
            self.members.push(pid);
        }
    }

    /// Set group limit
    pub fn set_limit(&mut self, def: QuotaDefinition, now: u64) {
        let key = def.resource as u8;
        let window_ns = def.window_ns;
        self.limits.insert(key, def);
        self.aggregate.entry(key).or_insert_with(|| WindowedUsage::new(window_ns, now));
    }

    /// Record group usage
    pub fn record(&mut self, resource: QuotaResource, amount: u64, now: u64) -> QuotaAction {
        let key = resource as u8;
        if let Some(tracker) = self.aggregate.get_mut(&key) {
            tracker.record(amount, now);
        }
        if let (Some(def), Some(tracker)) = (self.limits.get(&key), self.aggregate.get(&key)) {
            if tracker.current_usage > def.hard_limit {
                QuotaAction::Deny
            } else if tracker.current_usage > def.limit {
                QuotaAction::Throttle
            } else {
                QuotaAction::Allow
            }
        } else {
            QuotaAction::Allow
        }
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Quota enforcer stats
#[derive(Debug, Clone, Default)]
pub struct BridgeQuotaStats {
    /// Tracked processes
    pub tracked_processes: usize,
    /// Groups
    pub groups: usize,
    /// Total checks
    pub total_checks: u64,
    /// Denials
    pub denials: u64,
    /// Throttles
    pub throttles: u64,
    /// Violations
    pub violations: u64,
}

/// Bridge quota enforcer
pub struct BridgeQuotaEnforcer {
    /// Per-process quotas
    processes: BTreeMap<u64, ProcessQuota>,
    /// Group quotas
    groups: BTreeMap<u64, GroupQuota>,
    /// Process-to-group mapping
    pid_to_group: BTreeMap<u64, u64>,
    /// Stats
    stats: BridgeQuotaStats,
}

impl BridgeQuotaEnforcer {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            groups: BTreeMap::new(),
            pid_to_group: BTreeMap::new(),
            stats: BridgeQuotaStats::default(),
        }
    }

    /// Set process quota
    pub fn set_process_quota(&mut self, pid: u64, def: QuotaDefinition, now: u64) {
        let proc = self.processes.entry(pid).or_insert_with(|| ProcessQuota::new(pid));
        proc.set_quota(def, now);
        self.update_stats();
    }

    /// Check and record
    pub fn check(&mut self, pid: u64, resource: QuotaResource, amount: u64, now: u64) -> QuotaAction {
        self.stats.total_checks += 1;

        // Check group quota first
        if let Some(&group_id) = self.pid_to_group.get(&pid) {
            if let Some(group) = self.groups.get_mut(&group_id) {
                let group_action = group.record(resource, amount, now);
                if group_action == QuotaAction::Deny {
                    self.stats.denials += 1;
                    return QuotaAction::Deny;
                }
            }
        }

        // Check process quota
        let proc = self.processes.entry(pid).or_insert_with(|| ProcessQuota::new(pid));
        let action = proc.check_and_record(resource, amount, now);
        match action {
            QuotaAction::Deny => self.stats.denials += 1,
            QuotaAction::Throttle => self.stats.throttles += 1,
            _ => {}
        }
        action
    }

    /// Create group
    pub fn create_group(&mut self, group_id: u64) {
        self.groups.insert(group_id, GroupQuota::new(group_id));
        self.update_stats();
    }

    /// Add process to group
    pub fn add_to_group(&mut self, pid: u64, group_id: u64) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.add_member(pid);
            self.pid_to_group.insert(pid, group_id);
        }
    }

    /// Set group quota
    pub fn set_group_quota(&mut self, group_id: u64, def: QuotaDefinition, now: u64) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.set_limit(def, now);
        }
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.groups = self.groups.len();
        self.stats.violations = self.processes.values().map(|p| p.total_violations).sum();
    }

    /// Stats
    pub fn stats(&self) -> &BridgeQuotaStats {
        &self.stats
    }
}
