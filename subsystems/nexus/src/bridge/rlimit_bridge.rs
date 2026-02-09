//! # Bridge Resource Limit Enforcement
//!
//! Resource limit (rlimit) enforcement bridge:
//! - Per-process soft/hard limit tracking (RLIMIT_*)
//! - Limit inheritance on fork
//! - Usage vs limit comparison and enforcement
//! - Limit change auditing
//! - Cgroup-aware limit hierarchies
//! - Graceful limit exceeded notification

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Resource types (RLIMIT_* equivalents)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceType {
    Cpu,
    Fsize,
    Data,
    Stack,
    Core,
    Rss,
    Nproc,
    Nofile,
    Memlock,
    As,
    Locks,
    Sigpending,
    Msgqueue,
    Nice,
    Rtprio,
    Rttime,
}

impl ResourceType {
    #[inline]
    pub fn all() -> &'static [ResourceType] {
        &[
            ResourceType::Cpu, ResourceType::Fsize, ResourceType::Data,
            ResourceType::Stack, ResourceType::Core, ResourceType::Rss,
            ResourceType::Nproc, ResourceType::Nofile, ResourceType::Memlock,
            ResourceType::As, ResourceType::Locks, ResourceType::Sigpending,
            ResourceType::Msgqueue, ResourceType::Nice, ResourceType::Rtprio,
            ResourceType::Rttime,
        ]
    }
}

/// Limit value — infinite sentinel
pub const RLIM_INFINITY: u64 = u64::MAX;

/// Single resource limit
#[derive(Debug, Clone, Copy)]
pub struct Rlimit {
    pub soft: u64,
    pub hard: u64,
}

impl Rlimit {
    pub fn new(soft: u64, hard: u64) -> Self {
        Self { soft, hard }
    }
    #[inline(always)]
    pub fn unlimited() -> Self {
        Self { soft: RLIM_INFINITY, hard: RLIM_INFINITY }
    }
    #[inline(always)]
    pub fn is_unlimited(&self) -> bool { self.hard == RLIM_INFINITY }

    #[inline(always)]
    pub fn check_soft(&self, usage: u64) -> bool { usage <= self.soft || self.soft == RLIM_INFINITY }
    #[inline(always)]
    pub fn check_hard(&self, usage: u64) -> bool { usage <= self.hard || self.hard == RLIM_INFINITY }

    #[inline(always)]
    pub fn remaining_soft(&self, usage: u64) -> u64 {
        if self.soft == RLIM_INFINITY { return RLIM_INFINITY; }
        self.soft.saturating_sub(usage)
    }
}

/// Per-process resource usage
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ProcessRlimits {
    pub process_id: u64,
    pub limits: BTreeMap<ResourceType, Rlimit>,
    pub usage: BTreeMap<ResourceType, u64>,
    pub soft_violations: u64,
    pub hard_violations: u64,
}

impl ProcessRlimits {
    pub fn new(pid: u64) -> Self {
        let mut limits = BTreeMap::new();
        // Set defaults
        limits.insert(ResourceType::Nofile, Rlimit::new(1024, 4096));
        limits.insert(ResourceType::Stack, Rlimit::new(8 * 1024 * 1024, RLIM_INFINITY));
        limits.insert(ResourceType::Data, Rlimit::unlimited());
        limits.insert(ResourceType::Fsize, Rlimit::unlimited());
        limits.insert(ResourceType::Cpu, Rlimit::unlimited());
        limits.insert(ResourceType::Rss, Rlimit::unlimited());
        limits.insert(ResourceType::Nproc, Rlimit::new(32768, 32768));
        limits.insert(ResourceType::Memlock, Rlimit::new(65536, 65536));
        limits.insert(ResourceType::As, Rlimit::unlimited());
        limits.insert(ResourceType::Core, Rlimit::new(0, RLIM_INFINITY));
        limits.insert(ResourceType::Locks, Rlimit::unlimited());
        limits.insert(ResourceType::Sigpending, Rlimit::new(128204, 128204));
        limits.insert(ResourceType::Msgqueue, Rlimit::new(819200, 819200));
        limits.insert(ResourceType::Nice, Rlimit::new(0, 0));
        limits.insert(ResourceType::Rtprio, Rlimit::new(0, 0));
        limits.insert(ResourceType::Rttime, Rlimit::unlimited());

        Self {
            process_id: pid,
            limits,
            usage: BTreeMap::new(),
            soft_violations: 0,
            hard_violations: 0,
        }
    }

    #[inline(always)]
    pub fn get_limit(&self, res: ResourceType) -> Rlimit {
        self.limits.get(&res).copied().unwrap_or(Rlimit::unlimited())
    }

    #[inline]
    pub fn set_limit(&mut self, res: ResourceType, rlim: Rlimit) -> bool {
        // Cannot raise hard limit (unless privileged — we skip that check)
        if let Some(current) = self.limits.get(&res) {
            if rlim.hard > current.hard && current.hard != RLIM_INFINITY {
                return false; // permission denied
            }
        }
        if rlim.soft > rlim.hard { return false; }
        self.limits.insert(res, rlim);
        true
    }

    #[inline(always)]
    pub fn get_usage(&self, res: ResourceType) -> u64 {
        self.usage.get(&res).copied().unwrap_or(0)
    }

    #[inline(always)]
    pub fn update_usage(&mut self, res: ResourceType, value: u64) {
        self.usage.insert(res, value);
    }

    pub fn check(&mut self, res: ResourceType) -> LimitCheckResult {
        let limit = self.get_limit(res);
        let usage = self.get_usage(res);
        if !limit.check_hard(usage) {
            self.hard_violations += 1;
            LimitCheckResult::HardExceeded
        } else if !limit.check_soft(usage) {
            self.soft_violations += 1;
            LimitCheckResult::SoftExceeded
        } else {
            LimitCheckResult::Ok
        }
    }

    /// Clone limits for fork (usage is NOT inherited)
    #[inline]
    pub fn fork_limits(&self, child_pid: u64) -> ProcessRlimits {
        let mut child = ProcessRlimits::new(child_pid);
        child.limits = self.limits.clone();
        child
    }
}

/// Result of limit check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitCheckResult {
    Ok,
    SoftExceeded,
    HardExceeded,
}

/// Limit change audit entry
#[derive(Debug, Clone)]
pub struct LimitChangeAudit {
    pub process_id: u64,
    pub resource: ResourceType,
    pub old_soft: u64,
    pub old_hard: u64,
    pub new_soft: u64,
    pub new_hard: u64,
    pub timestamp_ns: u64,
    pub success: bool,
}

/// Bridge rlimit stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeRlimitStats {
    pub total_processes: usize,
    pub total_soft_violations: u64,
    pub total_hard_violations: u64,
    pub total_limit_changes: usize,
}

/// Bridge Rlimit Manager
#[repr(align(64))]
pub struct BridgeRlimitBridge {
    processes: BTreeMap<u64, ProcessRlimits>,
    audit_log: VecDeque<LimitChangeAudit>,
    max_audit: usize,
    stats: BridgeRlimitStats,
}

impl BridgeRlimitBridge {
    pub fn new(max_audit: usize) -> Self {
        Self {
            processes: BTreeMap::new(),
            audit_log: VecDeque::new(),
            max_audit,
            stats: BridgeRlimitStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.processes.entry(pid).or_insert_with(|| ProcessRlimits::new(pid));
    }

    #[inline]
    pub fn fork_process(&mut self, parent_pid: u64, child_pid: u64) {
        if let Some(parent) = self.processes.get(&parent_pid) {
            let child = parent.fork_limits(child_pid);
            self.processes.insert(child_pid, child);
        }
    }

    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) {
        self.processes.remove(&pid);
    }

    #[inline(always)]
    pub fn get_rlimit(&self, pid: u64, res: ResourceType) -> Option<Rlimit> {
        self.processes.get(&pid).map(|p| p.get_limit(res))
    }

    pub fn set_rlimit(&mut self, pid: u64, res: ResourceType, rlim: Rlimit, ts: u64) -> bool {
        if let Some(proc_limits) = self.processes.get_mut(&pid) {
            let old = proc_limits.get_limit(res);
            let success = proc_limits.set_limit(res, rlim);
            self.audit_log.push_back(LimitChangeAudit {
                process_id: pid, resource: res,
                old_soft: old.soft, old_hard: old.hard,
                new_soft: rlim.soft, new_hard: rlim.hard,
                timestamp_ns: ts, success,
            });
            while self.audit_log.len() > self.max_audit { self.audit_log.pop_front(); }
            success
        } else { false }
    }

    #[inline]
    pub fn check_limit(&mut self, pid: u64, res: ResourceType) -> LimitCheckResult {
        if let Some(proc_limits) = self.processes.get_mut(&pid) {
            proc_limits.check(res)
        } else { LimitCheckResult::Ok }
    }

    #[inline]
    pub fn update_usage(&mut self, pid: u64, res: ResourceType, value: u64) {
        if let Some(proc_limits) = self.processes.get_mut(&pid) {
            proc_limits.update_usage(res, value);
        }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_processes = self.processes.len();
        self.stats.total_soft_violations = self.processes.values().map(|p| p.soft_violations).sum();
        self.stats.total_hard_violations = self.processes.values().map(|p| p.hard_violations).sum();
        self.stats.total_limit_changes = self.audit_log.len();
    }

    #[inline(always)]
    pub fn process_limits(&self, pid: u64) -> Option<&ProcessRlimits> { self.processes.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &BridgeRlimitStats { &self.stats }
}

// ============================================================================
// Merged from rlimit_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RlimitV2Resource {
    Cpu,
    Fsize,
    Data,
    Stack,
    Core,
    Rss,
    Nproc,
    Nofile,
    Memlock,
    As,
    Locks,
    Sigpending,
    Msgqueue,
    Nice,
    Rtprio,
    Rttime,
}

/// A resource limit pair (soft/hard)
#[derive(Debug, Clone, Copy)]
pub struct RlimitV2Pair {
    pub soft: u64,
    pub hard: u64,
}

impl RlimitV2Pair {
    pub fn new(soft: u64, hard: u64) -> Self {
        Self { soft, hard }
    }

    #[inline(always)]
    pub fn unlimited() -> Self {
        Self { soft: u64::MAX, hard: u64::MAX }
    }

    #[inline(always)]
    pub fn is_unlimited(&self) -> bool {
        self.soft == u64::MAX && self.hard == u64::MAX
    }

    #[inline]
    pub fn check(&self, current: u64) -> RlimitV2Check {
        if current >= self.hard {
            RlimitV2Check::HardExceeded
        } else if current >= self.soft {
            RlimitV2Check::SoftExceeded
        } else {
            RlimitV2Check::Within
        }
    }
}

/// Result of a limit check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlimitV2Check {
    Within,
    SoftExceeded,
    HardExceeded,
}

/// A process's resource limits
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ProcessRlimitsV2 {
    pub pid: u64,
    pub limits: BTreeMap<RlimitV2Resource, RlimitV2Pair>,
    pub usage: BTreeMap<RlimitV2Resource, u64>,
    pub violations: u64,
    pub soft_warnings: u64,
}

impl ProcessRlimitsV2 {
    pub fn new(pid: u64) -> Self {
        let mut limits = BTreeMap::new();
        let mut usage = BTreeMap::new();
        let resources = [
            RlimitV2Resource::Cpu, RlimitV2Resource::Fsize,
            RlimitV2Resource::Data, RlimitV2Resource::Stack,
            RlimitV2Resource::Core, RlimitV2Resource::Rss,
            RlimitV2Resource::Nproc, RlimitV2Resource::Nofile,
            RlimitV2Resource::Memlock, RlimitV2Resource::As,
            RlimitV2Resource::Locks, RlimitV2Resource::Sigpending,
            RlimitV2Resource::Msgqueue, RlimitV2Resource::Nice,
            RlimitV2Resource::Rtprio, RlimitV2Resource::Rttime,
        ];
        for r in &resources {
            limits.insert(*r, RlimitV2Pair::unlimited());
            usage.insert(*r, 0);
        }
        Self { pid, limits, usage, violations: 0, soft_warnings: 0 }
    }

    #[inline]
    pub fn set_limit(&mut self, resource: RlimitV2Resource, soft: u64, hard: u64) -> bool {
        if soft > hard {
            return false;
        }
        self.limits.insert(resource, RlimitV2Pair::new(soft, hard));
        true
    }

    pub fn check_and_update(&mut self, resource: RlimitV2Resource, new_usage: u64) -> RlimitV2Check {
        self.usage.insert(resource, new_usage);
        if let Some(limit) = self.limits.get(&resource) {
            let result = limit.check(new_usage);
            match result {
                RlimitV2Check::HardExceeded => self.violations += 1,
                RlimitV2Check::SoftExceeded => self.soft_warnings += 1,
                _ => {}
            }
            result
        } else {
            RlimitV2Check::Within
        }
    }
}

/// Statistics for rlimit V2 bridge
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct RlimitV2BridgeStats {
    pub processes_tracked: u64,
    pub limits_set: u64,
    pub checks_performed: u64,
    pub soft_violations: u64,
    pub hard_violations: u64,
    pub inherited_limits: u64,
}

/// Main rlimit V2 bridge manager
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeRlimitV2 {
    processes: BTreeMap<u64, ProcessRlimitsV2>,
    stats: RlimitV2BridgeStats,
}

impl BridgeRlimitV2 {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: RlimitV2BridgeStats {
                processes_tracked: 0,
                limits_set: 0,
                checks_performed: 0,
                soft_violations: 0,
                hard_violations: 0,
                inherited_limits: 0,
            },
        }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.processes.insert(pid, ProcessRlimitsV2::new(pid));
        self.stats.processes_tracked += 1;
    }

    #[inline]
    pub fn set_limit(&mut self, pid: u64, resource: RlimitV2Resource, soft: u64, hard: u64) -> bool {
        if let Some(proc) = self.processes.get_mut(&pid) {
            if proc.set_limit(resource, soft, hard) {
                self.stats.limits_set += 1;
                return true;
            }
        }
        false
    }

    pub fn check_usage(&mut self, pid: u64, resource: RlimitV2Resource, usage: u64) -> RlimitV2Check {
        self.stats.checks_performed += 1;
        if let Some(proc) = self.processes.get_mut(&pid) {
            let result = proc.check_and_update(resource, usage);
            match result {
                RlimitV2Check::SoftExceeded => self.stats.soft_violations += 1,
                RlimitV2Check::HardExceeded => self.stats.hard_violations += 1,
                _ => {}
            }
            result
        } else {
            RlimitV2Check::Within
        }
    }

    #[inline]
    pub fn inherit_limits(&mut self, parent_pid: u64, child_pid: u64) -> bool {
        if let Some(parent) = self.processes.get(&parent_pid) {
            let mut child = ProcessRlimitsV2::new(child_pid);
            child.limits = parent.limits.clone();
            self.processes.insert(child_pid, child);
            self.stats.inherited_limits += 1;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &RlimitV2BridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from rlimit_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlimitV3Resource {
    Cpu,
    Fsize,
    Data,
    Stack,
    Core,
    Rss,
    Nproc,
    Nofile,
    Memlock,
    As,
    Locks,
    Sigpending,
    Msgqueue,
    Nice,
    Rtprio,
    Rttime,
    NprocCgroup,
    IoWeight,
    IoMax,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlimitV3Enforcement {
    Hard,
    Soft,
    Inherited,
    CgroupCapped,
}

#[derive(Debug, Clone)]
pub struct RlimitV3Value {
    pub resource: RlimitV3Resource,
    pub soft_limit: u64,
    pub hard_limit: u64,
    pub current_usage: u64,
    pub peak_usage: u64,
    pub enforcement: RlimitV3Enforcement,
}

impl RlimitV3Value {
    pub fn new(resource: RlimitV3Resource, soft: u64, hard: u64) -> Self {
        Self {
            resource,
            soft_limit: soft,
            hard_limit: hard,
            current_usage: 0,
            peak_usage: 0,
            enforcement: RlimitV3Enforcement::Hard,
        }
    }

    #[inline]
    pub fn update_usage(&mut self, usage: u64) {
        self.current_usage = usage;
        if usage > self.peak_usage {
            self.peak_usage = usage;
        }
    }

    #[inline]
    pub fn check_limit(&self, requested: u64) -> bool {
        match self.enforcement {
            RlimitV3Enforcement::Hard => self.current_usage + requested <= self.hard_limit,
            RlimitV3Enforcement::Soft => self.current_usage + requested <= self.soft_limit,
            _ => self.current_usage + requested <= self.hard_limit,
        }
    }

    #[inline(always)]
    pub fn utilization_pct(&self) -> u64 {
        if self.hard_limit == 0 || self.hard_limit == u64::MAX { 0 }
        else { (self.current_usage * 100) / self.hard_limit }
    }

    #[inline(always)]
    pub fn headroom(&self) -> u64 {
        if self.current_usage >= self.hard_limit { 0 }
        else { self.hard_limit - self.current_usage }
    }
}

#[derive(Debug, Clone)]
pub struct RlimitV3ProcessLimits {
    pub pid: u64,
    pub limits: Vec<RlimitV3Value>,
    pub inherited_from: Option<u64>,
    pub cgroup_capped: bool,
    pub violations: u64,
}

impl RlimitV3ProcessLimits {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            limits: Vec::new(),
            inherited_from: None,
            cgroup_capped: false,
            violations: 0,
        }
    }

    #[inline]
    pub fn set_limit(&mut self, resource: RlimitV3Resource, soft: u64, hard: u64) {
        if let Some(lim) = self.limits.iter_mut().find(|l| l.resource == resource) {
            lim.soft_limit = soft;
            lim.hard_limit = hard;
        } else {
            self.limits.push(RlimitV3Value::new(resource, soft, hard));
        }
    }

    #[inline(always)]
    pub fn get_limit(&self, resource: RlimitV3Resource) -> Option<&RlimitV3Value> {
        self.limits.iter().find(|l| l.resource == resource)
    }

    #[inline(always)]
    pub fn most_constrained(&self) -> Option<&RlimitV3Value> {
        self.limits.iter().max_by_key(|l| l.utilization_pct())
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct RlimitV3BridgeStats {
    pub total_processes: u64,
    pub total_limits_set: u64,
    pub total_violations: u64,
    pub cgroup_overrides: u64,
}

#[repr(align(64))]
pub struct BridgeRlimitV3 {
    processes: BTreeMap<u64, RlimitV3ProcessLimits>,
    stats: RlimitV3BridgeStats,
}

impl BridgeRlimitV3 {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: RlimitV3BridgeStats {
                total_processes: 0,
                total_limits_set: 0,
                total_violations: 0,
                cgroup_overrides: 0,
            },
        }
    }

    #[inline(always)]
    pub fn register(&mut self, pid: u64) {
        self.processes.insert(pid, RlimitV3ProcessLimits::new(pid));
        self.stats.total_processes += 1;
    }

    #[inline]
    pub fn set_limit(&mut self, pid: u64, resource: RlimitV3Resource, soft: u64, hard: u64) {
        if let Some(p) = self.processes.get_mut(&pid) {
            p.set_limit(resource, soft, hard);
            self.stats.total_limits_set += 1;
        }
    }

    pub fn check_and_record(&mut self, pid: u64, resource: RlimitV3Resource, requested: u64) -> bool {
        if let Some(p) = self.processes.get_mut(&pid) {
            if let Some(lim) = p.limits.iter().find(|l| l.resource == resource) {
                if !lim.check_limit(requested) {
                    p.violations += 1;
                    self.stats.total_violations += 1;
                    return false;
                }
            }
            return true;
        }
        false
    }

    #[inline(always)]
    pub fn stats(&self) -> &RlimitV3BridgeStats {
        &self.stats
    }
}
