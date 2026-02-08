// SPDX-License-Identifier: GPL-2.0
//! Apps rlimit_mgr — resource limit management per process.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Resource limit type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RlimitResource {
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

/// Resource limit value
pub const RLIM_INFINITY: u64 = u64::MAX;

/// Soft/hard limit pair
#[derive(Debug, Clone, Copy)]
pub struct Rlimit {
    pub soft: u64,
    pub hard: u64,
}

impl Rlimit {
    pub fn new(soft: u64, hard: u64) -> Self { Self { soft, hard } }
    pub fn unlimited() -> Self { Self { soft: RLIM_INFINITY, hard: RLIM_INFINITY } }
    pub fn is_unlimited_soft(&self) -> bool { self.soft == RLIM_INFINITY }
    pub fn is_unlimited_hard(&self) -> bool { self.hard == RLIM_INFINITY }

    pub fn can_set_soft(&self, val: u64) -> bool {
        val <= self.hard
    }

    pub fn can_set_hard(&self, val: u64, privileged: bool) -> bool {
        if privileged { return true; }
        val <= self.hard
    }

    pub fn headroom(&self, current: u64) -> u64 {
        if self.soft == RLIM_INFINITY { return RLIM_INFINITY; }
        self.soft.saturating_sub(current)
    }

    pub fn utilization(&self, current: u64) -> f64 {
        if self.soft == RLIM_INFINITY || self.soft == 0 { return 0.0; }
        current as f64 / self.soft as f64
    }
}

/// Limit violation event
#[derive(Debug, Clone, Copy)]
pub struct LimitViolation {
    pub pid: u32,
    pub resource: RlimitResource,
    pub current: u64,
    pub limit: u64,
    pub timestamp: u64,
    pub was_hard: bool,
}

/// Per-process resource limits
#[derive(Debug)]
pub struct ProcessLimits {
    pub pid: u32,
    pub limits: BTreeMap<u32, Rlimit>,
    pub current_usage: BTreeMap<u32, u64>,
    pub violation_count: u64,
    pub last_violation: u64,
}

impl ProcessLimits {
    pub fn new(pid: u32) -> Self {
        Self {
            pid, limits: BTreeMap::new(), current_usage: BTreeMap::new(),
            violation_count: 0, last_violation: 0,
        }
    }

    pub fn with_defaults(pid: u32) -> Self {
        let mut s = Self::new(pid);
        s.limits.insert(RlimitResource::Nofile as u32, Rlimit::new(1024, 4096));
        s.limits.insert(RlimitResource::Nproc as u32, Rlimit::new(4096, 4096));
        s.limits.insert(RlimitResource::Stack as u32, Rlimit::new(8 * 1024 * 1024, RLIM_INFINITY));
        s.limits.insert(RlimitResource::Core as u32, Rlimit::new(0, RLIM_INFINITY));
        s.limits.insert(RlimitResource::Cpu as u32, Rlimit::unlimited());
        s.limits.insert(RlimitResource::Fsize as u32, Rlimit::unlimited());
        s.limits.insert(RlimitResource::Data as u32, Rlimit::unlimited());
        s.limits.insert(RlimitResource::Rss as u32, Rlimit::unlimited());
        s.limits.insert(RlimitResource::Memlock as u32, Rlimit::new(65536, 65536));
        s.limits.insert(RlimitResource::As as u32, Rlimit::unlimited());
        s.limits.insert(RlimitResource::Locks as u32, Rlimit::unlimited());
        s.limits.insert(RlimitResource::Sigpending as u32, Rlimit::new(7680, 7680));
        s.limits.insert(RlimitResource::Msgqueue as u32, Rlimit::new(819200, 819200));
        s.limits.insert(RlimitResource::Nice as u32, Rlimit::new(0, 0));
        s.limits.insert(RlimitResource::Rtprio as u32, Rlimit::new(0, 0));
        s.limits.insert(RlimitResource::Rttime as u32, Rlimit::unlimited());
        s
    }

    pub fn get_limit(&self, res: RlimitResource) -> Option<Rlimit> {
        self.limits.get(&(res as u32)).copied()
    }

    pub fn set_limit(&mut self, res: RlimitResource, soft: u64, hard: u64, privileged: bool) -> bool {
        let key = res as u32;
        if let Some(current) = self.limits.get(&key) {
            if soft > hard { return false; }
            if !current.can_set_hard(hard, privileged) { return false; }
        }
        self.limits.insert(key, Rlimit::new(soft, hard));
        true
    }

    pub fn check_soft(&self, res: RlimitResource, value: u64) -> bool {
        if let Some(lim) = self.limits.get(&(res as u32)) {
            if lim.soft == RLIM_INFINITY { return true; }
            value <= lim.soft
        } else { true }
    }

    pub fn check_hard(&self, res: RlimitResource, value: u64) -> bool {
        if let Some(lim) = self.limits.get(&(res as u32)) {
            if lim.hard == RLIM_INFINITY { return true; }
            value <= lim.hard
        } else { true }
    }

    pub fn update_usage(&mut self, res: RlimitResource, value: u64) {
        self.current_usage.insert(res as u32, value);
    }

    pub fn near_limit_resources(&self, threshold: f64) -> Vec<(RlimitResource, f64)> {
        let mut result = Vec::new();
        let resources = [
            RlimitResource::Cpu, RlimitResource::Fsize, RlimitResource::Data,
            RlimitResource::Stack, RlimitResource::Core, RlimitResource::Rss,
            RlimitResource::Nproc, RlimitResource::Nofile, RlimitResource::Memlock,
            RlimitResource::As, RlimitResource::Locks, RlimitResource::Sigpending,
            RlimitResource::Msgqueue,
        ];
        for &res in &resources {
            let key = res as u32;
            if let (Some(lim), Some(&usage)) = (self.limits.get(&key), self.current_usage.get(&key)) {
                let util = lim.utilization(usage);
                if util >= threshold { result.push((res, util)); }
            }
        }
        result
    }
}

/// Rlimit manager stats
#[derive(Debug, Clone)]
pub struct RlimitMgrStats {
    pub tracked_processes: u32,
    pub total_violations: u64,
    pub total_setrlimit_calls: u64,
    pub total_getrlimit_calls: u64,
}

/// Main rlimit manager
pub struct AppRlimitMgr {
    processes: BTreeMap<u32, ProcessLimits>,
    violations: Vec<LimitViolation>,
    max_violations: usize,
    total_violations: u64,
    total_sets: u64,
    total_gets: u64,
}

impl AppRlimitMgr {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(), violations: Vec::new(),
            max_violations: 2048, total_violations: 0,
            total_sets: 0, total_gets: 0,
        }
    }

    pub fn create_process(&mut self, pid: u32) {
        self.processes.insert(pid, ProcessLimits::with_defaults(pid));
    }

    pub fn remove_process(&mut self, pid: u32) -> bool {
        self.processes.remove(&pid).is_some()
    }

    pub fn get_limit(&mut self, pid: u32, res: RlimitResource) -> Option<Rlimit> {
        self.total_gets += 1;
        self.processes.get(&pid)?.get_limit(res)
    }

    pub fn set_limit(&mut self, pid: u32, res: RlimitResource, soft: u64, hard: u64, privileged: bool) -> bool {
        self.total_sets += 1;
        if let Some(proc_limits) = self.processes.get_mut(&pid) {
            proc_limits.set_limit(res, soft, hard, privileged)
        } else { false }
    }

    pub fn check_and_record(&mut self, pid: u32, res: RlimitResource, value: u64, now: u64) -> bool {
        if let Some(proc_limits) = self.processes.get_mut(&pid) {
            if !proc_limits.check_soft(res, value) {
                self.total_violations += 1;
                proc_limits.violation_count += 1;
                proc_limits.last_violation = now;
                let soft = proc_limits.get_limit(res).map(|l| l.soft).unwrap_or(0);
                let violation = LimitViolation {
                    pid, resource: res, current: value, limit: soft,
                    timestamp: now, was_hard: !proc_limits.check_hard(res, value),
                };
                if self.violations.len() >= self.max_violations { self.violations.remove(0); }
                self.violations.push(violation);
                return false;
            }
        }
        true
    }

    pub fn fork_limits(&mut self, parent: u32, child: u32) -> bool {
        if let Some(parent_limits) = self.processes.get(&parent) {
            let mut child_limits = ProcessLimits::new(child);
            child_limits.limits = parent_limits.limits.clone();
            self.processes.insert(child, child_limits);
            true
        } else { false }
    }

    pub fn stats(&self) -> RlimitMgrStats {
        RlimitMgrStats {
            tracked_processes: self.processes.len() as u32,
            total_violations: self.total_violations,
            total_setrlimit_calls: self.total_sets,
            total_getrlimit_calls: self.total_gets,
        }
    }
}
