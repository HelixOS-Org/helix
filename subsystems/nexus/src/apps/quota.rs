//! # Application Resource Quota Management
//!
//! Per-application resource quota system:
//! - Multi-resource quota definitions
//! - Usage tracking and enforcement
//! - Quota sharing groups
//! - Burst allowance with recovery
//! - Quota transfer between processes
//! - Billing and chargeback

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// QUOTA RESOURCE
// ============================================================================

/// Resource type for quotas
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QuotaResource {
    /// CPU time (ms per period)
    CpuTime,
    /// Memory (bytes)
    Memory,
    /// Disk I/O (bytes per period)
    DiskIo,
    /// Network I/O (bytes per period)
    NetworkIo,
    /// File descriptors
    FileDescriptors,
    /// Processes / threads
    Processes,
    /// IPC messages per period
    IpcMessages,
    /// Syscalls per period
    Syscalls,
    /// GPU time (ms per period)
    GpuTime,
    /// Hugepages
    HugePages,
}

/// Enforcement action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforcementAction {
    /// Allow (under quota)
    Allow,
    /// Warn (approaching quota)
    Warn,
    /// Throttle (at quota, slow down)
    Throttle,
    /// Deny (over quota, hard limit)
    Deny,
    /// Kill (egregious over-use)
    Kill,
}

// ============================================================================
// QUOTA DEFINITION
// ============================================================================

/// A single resource quota
#[derive(Debug, Clone)]
pub struct ResourceQuota {
    /// Resource type
    pub resource: QuotaResource,
    /// Soft limit (warning)
    pub soft_limit: u64,
    /// Hard limit (enforcement)
    pub hard_limit: u64,
    /// Burst allowance above hard limit
    pub burst_limit: u64,
    /// Burst recovery rate (units per period)
    pub burst_recovery_rate: u64,
    /// Period length (ms, 0 = absolute limit)
    pub period_ms: u64,
    /// Current usage
    pub usage: u64,
    /// Burst used
    pub burst_used: u64,
    /// Peak usage this period
    pub peak_usage: u64,
}

impl ResourceQuota {
    pub fn new(resource: QuotaResource, soft: u64, hard: u64, period_ms: u64) -> Self {
        Self {
            resource,
            soft_limit: soft,
            hard_limit: hard,
            burst_limit: 0,
            burst_recovery_rate: 0,
            period_ms,
            usage: 0,
            burst_used: 0,
            peak_usage: 0,
        }
    }

    /// Set burst configuration
    pub fn with_burst(mut self, limit: u64, recovery_rate: u64) -> Self {
        self.burst_limit = limit;
        self.burst_recovery_rate = recovery_rate;
        self
    }

    /// Usage percentage
    pub fn usage_pct(&self) -> u32 {
        if self.hard_limit == 0 {
            return 0;
        }
        ((self.usage * 100) / self.hard_limit) as u32
    }

    /// Remaining before hard limit
    pub fn remaining(&self) -> u64 {
        self.hard_limit.saturating_sub(self.usage)
    }

    /// Remaining including burst
    pub fn remaining_with_burst(&self) -> u64 {
        let burst_remaining = self.burst_limit.saturating_sub(self.burst_used);
        self.remaining() + burst_remaining
    }

    /// Check enforcement action
    pub fn check(&self, additional: u64) -> EnforcementAction {
        let new_usage = self.usage + additional;

        if new_usage <= self.soft_limit {
            EnforcementAction::Allow
        } else if new_usage <= self.hard_limit {
            EnforcementAction::Warn
        } else if new_usage <= self.hard_limit + self.burst_limit.saturating_sub(self.burst_used) {
            EnforcementAction::Throttle
        } else {
            EnforcementAction::Deny
        }
    }

    /// Consume quota
    pub fn consume(&mut self, amount: u64) -> EnforcementAction {
        let action = self.check(amount);

        match action {
            EnforcementAction::Allow | EnforcementAction::Warn => {
                self.usage += amount;
            },
            EnforcementAction::Throttle => {
                let over = (self.usage + amount).saturating_sub(self.hard_limit);
                self.burst_used += over;
                self.usage += amount;
            },
            EnforcementAction::Deny | EnforcementAction::Kill => {
                // Don't consume
            },
        }

        if self.usage > self.peak_usage {
            self.peak_usage = self.usage;
        }

        action
    }

    /// Reset for new period
    pub fn reset_period(&mut self) {
        self.usage = 0;
        // Recover burst
        self.burst_used = self.burst_used.saturating_sub(self.burst_recovery_rate);
    }
}

// ============================================================================
// QUOTA SET
// ============================================================================

/// Complete quota set for a process/group
#[derive(Debug, Clone)]
pub struct QuotaSet {
    /// Owner (PID or group ID)
    pub owner_id: u64,
    /// Name
    pub name: String,
    /// Resource quotas
    pub quotas: Vec<ResourceQuota>,
    /// Active
    pub active: bool,
    /// Created timestamp
    pub created_at: u64,
    /// Last reset timestamp
    pub last_reset: u64,
}

impl QuotaSet {
    pub fn new(owner_id: u64, name: String) -> Self {
        Self {
            owner_id,
            name,
            quotas: Vec::new(),
            active: true,
            created_at: 0,
            last_reset: 0,
        }
    }

    /// Add quota
    pub fn add(&mut self, quota: ResourceQuota) {
        // Replace if exists
        self.quotas.retain(|q| q.resource != quota.resource);
        self.quotas.push(quota);
    }

    /// Get quota for resource
    pub fn get(&self, resource: QuotaResource) -> Option<&ResourceQuota> {
        self.quotas.iter().find(|q| q.resource == resource)
    }

    /// Get mutable quota
    pub fn get_mut(&mut self, resource: QuotaResource) -> Option<&mut ResourceQuota> {
        self.quotas.iter_mut().find(|q| q.resource == resource)
    }

    /// Check if resource consumption is allowed
    pub fn check_resource(&self, resource: QuotaResource, amount: u64) -> EnforcementAction {
        match self.get(resource) {
            Some(q) => q.check(amount),
            None => EnforcementAction::Allow,
        }
    }

    /// Consume resource
    pub fn consume(&mut self, resource: QuotaResource, amount: u64) -> EnforcementAction {
        match self.get_mut(resource) {
            Some(q) => q.consume(amount),
            None => EnforcementAction::Allow,
        }
    }

    /// Reset periodic quotas
    pub fn reset_periodic(&mut self, now: u64) {
        for quota in &mut self.quotas {
            if quota.period_ms > 0 {
                quota.reset_period();
            }
        }
        self.last_reset = now;
    }

    /// Most constrained resource
    pub fn most_constrained(&self) -> Option<&ResourceQuota> {
        self.quotas.iter().max_by_key(|q| q.usage_pct())
    }
}

// ============================================================================
// QUOTA GROUP
// ============================================================================

/// Shared quota group
#[derive(Debug, Clone)]
pub struct QuotaGroup {
    /// Group ID
    pub id: u64,
    /// Group name
    pub name: String,
    /// Shared quota set
    pub shared_quota: QuotaSet,
    /// Members
    pub members: Vec<u64>,
    /// Max members
    pub max_members: usize,
}

impl QuotaGroup {
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id,
            name: name.clone(),
            shared_quota: QuotaSet::new(id, name),
            members: Vec::new(),
            max_members: 64,
        }
    }

    /// Add member
    pub fn add_member(&mut self, pid: u64) -> bool {
        if self.members.len() >= self.max_members {
            return false;
        }
        if !self.members.contains(&pid) {
            self.members.push(pid);
        }
        true
    }

    /// Remove member
    pub fn remove_member(&mut self, pid: u64) {
        self.members.retain(|&p| p != pid);
    }

    /// Per-member fair share
    pub fn fair_share(&self, resource: QuotaResource) -> u64 {
        if self.members.is_empty() {
            return 0;
        }
        self.shared_quota
            .get(resource)
            .map(|q| q.hard_limit / self.members.len() as u64)
            .unwrap_or(0)
    }
}

// ============================================================================
// QUOTA TRANSFER
// ============================================================================

/// Transfer quota between processes
#[derive(Debug, Clone)]
pub struct QuotaTransfer {
    /// Source process
    pub from_pid: u64,
    /// Destination process
    pub to_pid: u64,
    /// Resource
    pub resource: QuotaResource,
    /// Amount transferred
    pub amount: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Temporary (will revert)
    pub temporary: bool,
    /// Revert timestamp
    pub revert_at: u64,
}

// ============================================================================
// QUOTA VIOLATION
// ============================================================================

/// Quota violation event
#[derive(Debug, Clone)]
pub struct QuotaViolation {
    /// Process ID
    pub pid: u64,
    /// Resource
    pub resource: QuotaResource,
    /// Requested amount
    pub requested: u64,
    /// Available amount
    pub available: u64,
    /// Action taken
    pub action: EnforcementAction,
    /// Timestamp
    pub timestamp: u64,
}

// ============================================================================
// QUOTA MANAGER
// ============================================================================

/// Quota manager stats
#[derive(Debug, Clone, Default)]
pub struct QuotaManagerStats {
    /// Total quota sets
    pub total_sets: usize,
    /// Total groups
    pub total_groups: usize,
    /// Violations count
    pub violations: u64,
    /// Denials count
    pub denials: u64,
    /// Throttles count
    pub throttles: u64,
    /// Transfers count
    pub transfers: u64,
}

/// Application quota manager
pub struct AppQuotaManager {
    /// Per-process quota sets
    process_quotas: BTreeMap<u64, QuotaSet>,
    /// Quota groups
    groups: BTreeMap<u64, QuotaGroup>,
    /// Process to group mapping
    pid_to_group: BTreeMap<u64, u64>,
    /// Active transfers
    transfers: Vec<QuotaTransfer>,
    /// Recent violations
    violations: Vec<QuotaViolation>,
    /// Stats
    stats: QuotaManagerStats,
    /// Max violations to keep
    max_violations: usize,
}

impl AppQuotaManager {
    pub fn new() -> Self {
        Self {
            process_quotas: BTreeMap::new(),
            groups: BTreeMap::new(),
            pid_to_group: BTreeMap::new(),
            transfers: Vec::new(),
            violations: Vec::new(),
            stats: QuotaManagerStats::default(),
            max_violations: 1024,
        }
    }

    /// Set quota for process
    pub fn set_quota(&mut self, pid: u64, quota_set: QuotaSet) {
        self.process_quotas.insert(pid, quota_set);
        self.stats.total_sets = self.process_quotas.len();
    }

    /// Add single resource quota to process
    pub fn add_resource_quota(&mut self, pid: u64, quota: ResourceQuota) {
        if let Some(set) = self.process_quotas.get_mut(&pid) {
            set.add(quota);
        }
    }

    /// Create quota group
    pub fn create_group(&mut self, group: QuotaGroup) {
        self.groups.insert(group.id, group);
        self.stats.total_groups = self.groups.len();
    }

    /// Add process to group
    pub fn add_to_group(&mut self, pid: u64, group_id: u64) -> bool {
        if let Some(group) = self.groups.get_mut(&group_id) {
            if group.add_member(pid) {
                self.pid_to_group.insert(pid, group_id);
                return true;
            }
        }
        false
    }

    /// Check resource consumption
    pub fn check(&self, pid: u64, resource: QuotaResource, amount: u64) -> EnforcementAction {
        // Check process quota first
        if let Some(set) = self.process_quotas.get(&pid) {
            let action = set.check_resource(resource, amount);
            if matches!(action, EnforcementAction::Deny | EnforcementAction::Kill) {
                return action;
            }
        }

        // Check group quota
        if let Some(group_id) = self.pid_to_group.get(&pid) {
            if let Some(group) = self.groups.get(group_id) {
                let action = group.shared_quota.check_resource(resource, amount);
                if matches!(action, EnforcementAction::Deny | EnforcementAction::Kill) {
                    return action;
                }
            }
        }

        EnforcementAction::Allow
    }

    /// Consume resource
    pub fn consume(
        &mut self,
        pid: u64,
        resource: QuotaResource,
        amount: u64,
        now: u64,
    ) -> EnforcementAction {
        let mut worst_action = EnforcementAction::Allow;

        // Consume from process quota
        if let Some(set) = self.process_quotas.get_mut(&pid) {
            let action = set.consume(resource, amount);
            if (action as u8) > (worst_action as u8) {
                worst_action = action;
            }
        }

        // Consume from group quota
        let group_id = self.pid_to_group.get(&pid).copied();
        if let Some(gid) = group_id {
            if let Some(group) = self.groups.get_mut(&gid) {
                let action = group.shared_quota.consume(resource, amount);
                if (action as u8) > (worst_action as u8) {
                    worst_action = action;
                }
            }
        }

        // Record violation if needed
        if matches!(
            worst_action,
            EnforcementAction::Throttle | EnforcementAction::Deny
        ) {
            let available = self
                .process_quotas
                .get(&pid)
                .and_then(|s| s.get(resource))
                .map(|q| q.remaining())
                .unwrap_or(0);

            self.violations.push(QuotaViolation {
                pid,
                resource,
                requested: amount,
                available,
                action: worst_action,
                timestamp: now,
            });

            if self.violations.len() > self.max_violations {
                self.violations.remove(0);
            }

            self.stats.violations += 1;
            if matches!(worst_action, EnforcementAction::Deny) {
                self.stats.denials += 1;
            } else {
                self.stats.throttles += 1;
            }
        }

        worst_action
    }

    /// Transfer quota
    pub fn transfer(
        &mut self,
        from_pid: u64,
        to_pid: u64,
        resource: QuotaResource,
        amount: u64,
        now: u64,
        temporary: bool,
        duration_ms: u64,
    ) -> bool {
        // Reduce source limit
        let source_ok = if let Some(set) = self.process_quotas.get_mut(&from_pid) {
            if let Some(quota) = set.get_mut(resource) {
                if quota.remaining() >= amount {
                    quota.hard_limit -= amount;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if !source_ok {
            return false;
        }

        // Increase destination limit
        if let Some(set) = self.process_quotas.get_mut(&to_pid) {
            if let Some(quota) = set.get_mut(resource) {
                quota.hard_limit += amount;
            }
        }

        self.transfers.push(QuotaTransfer {
            from_pid,
            to_pid,
            resource,
            amount,
            timestamp: now,
            temporary,
            revert_at: if temporary { now + duration_ms } else { 0 },
        });

        self.stats.transfers += 1;
        true
    }

    /// Process periodic resets
    pub fn tick(&mut self, now: u64) {
        // Reset periodic quotas
        for set in self.process_quotas.values_mut() {
            if set.active {
                // Check if any periodic quotas need reset
                let needs_reset = set
                    .quotas
                    .iter()
                    .any(|q| q.period_ms > 0 && now.saturating_sub(set.last_reset) >= q.period_ms);
                if needs_reset {
                    set.reset_periodic(now);
                }
            }
        }

        // Revert expired transfers
        let expired: Vec<QuotaTransfer> = self
            .transfers
            .iter()
            .filter(|t| t.temporary && now >= t.revert_at)
            .cloned()
            .collect();

        for transfer in &expired {
            // Reverse: give back to source, take from dest
            if let Some(set) = self.process_quotas.get_mut(&transfer.from_pid) {
                if let Some(q) = set.get_mut(transfer.resource) {
                    q.hard_limit += transfer.amount;
                }
            }
            if let Some(set) = self.process_quotas.get_mut(&transfer.to_pid) {
                if let Some(q) = set.get_mut(transfer.resource) {
                    q.hard_limit = q.hard_limit.saturating_sub(transfer.amount);
                }
            }
        }

        self.transfers
            .retain(|t| !(t.temporary && now >= t.revert_at));
    }

    /// Get process quota set
    pub fn quota_set(&self, pid: u64) -> Option<&QuotaSet> {
        self.process_quotas.get(&pid)
    }

    /// Get stats
    pub fn stats(&self) -> &QuotaManagerStats {
        &self.stats
    }

    /// Unregister process
    pub fn unregister(&mut self, pid: u64) {
        self.process_quotas.remove(&pid);
        if let Some(gid) = self.pid_to_group.remove(&pid) {
            if let Some(group) = self.groups.get_mut(&gid) {
                group.remove_member(pid);
            }
        }
        self.stats.total_sets = self.process_quotas.len();
    }
}
