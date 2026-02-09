//! # Apps Seccomp Manager
//!
//! Application seccomp filter management:
//! - BPF filter compilation and installation
//! - Syscall allow/deny policy
//! - Audit mode logging
//! - Per-process filter chains
//! - Violation tracking and notification
//! - Filter performance statistics

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// Seccomp action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompAction {
    Allow,
    Kill,
    KillProcess,
    Trap,
    Errno(u16),
    Trace,
    Log,
    UserNotif,
}

/// Seccomp filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Strict,
    Filter,
    AuditOnly,
}

/// BPF instruction (simplified)
#[derive(Debug, Clone)]
pub struct BpfInsn {
    pub opcode: u16,
    pub jt: u8,
    pub jf: u8,
    pub k: u32,
}

/// Syscall rule
#[derive(Debug, Clone)]
pub struct SyscallRule {
    pub syscall_nr: u32,
    pub action: SeccompAction,
    pub arg_checks: Vec<ArgCheck>,
}

/// Argument check
#[derive(Debug, Clone)]
pub struct ArgCheck {
    pub arg_index: u8,
    pub op: ArgOp,
    pub value: u64,
    pub mask: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgOp {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    MaskedEqual,
}

/// Seccomp filter
#[derive(Debug, Clone)]
pub struct SeccompFilter {
    pub id: u64,
    pub mode: FilterMode,
    pub default_action: SeccompAction,
    pub rules: Vec<SyscallRule>,
    pub insn_count: u32,
    pub match_count: u64,
    pub miss_count: u64,
    pub created_at: u64,
    pub tsync: bool,
}

impl SeccompFilter {
    pub fn new(id: u64, mode: FilterMode, default: SeccompAction) -> Self {
        Self {
            id, mode, default_action: default, rules: Vec::new(),
            insn_count: 0, match_count: 0, miss_count: 0, created_at: 0, tsync: false,
        }
    }

    #[inline(always)]
    pub fn add_rule(&mut self, rule: SyscallRule) { self.rules.push(rule); self.insn_count += 1; }

    #[inline]
    pub fn eval(&mut self, syscall_nr: u32) -> SeccompAction {
        for r in &self.rules {
            if r.syscall_nr == syscall_nr { self.match_count += 1; return r.action; }
        }
        self.miss_count += 1;
        self.default_action
    }

    #[inline(always)]
    pub fn hit_rate(&self) -> f64 {
        let total = self.match_count + self.miss_count;
        if total == 0 { 0.0 } else { self.match_count as f64 / total as f64 * 100.0 }
    }
}

/// Per-process seccomp state
#[derive(Debug, Clone)]
pub struct ProcessSeccomp {
    pub pid: u64,
    pub filters: Vec<u64>,
    pub strict_mode: bool,
    pub violations: u64,
    pub last_violation_ts: u64,
    pub last_violation_nr: u32,
    pub audit_log: Vec<SeccompViolation>,
}

impl ProcessSeccomp {
    pub fn new(pid: u64) -> Self {
        Self { pid, filters: Vec::new(), strict_mode: false, violations: 0, last_violation_ts: 0, last_violation_nr: 0, audit_log: Vec::new() }
    }
}

/// Seccomp violation record
#[derive(Debug, Clone)]
pub struct SeccompViolation {
    pub pid: u64,
    pub syscall_nr: u32,
    pub action_taken: SeccompAction,
    pub ts: u64,
    pub ip: u64,
}

/// Seccomp stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SeccompStats {
    pub tracked_processes: usize,
    pub total_filters: usize,
    pub total_rules: usize,
    pub total_evaluations: u64,
    pub total_violations: u64,
    pub processes_with_filters: usize,
}

/// Apps seccomp manager
pub struct AppsSeccompMgr {
    filters: BTreeMap<u64, SeccompFilter>,
    processes: BTreeMap<u64, ProcessSeccomp>,
    stats: SeccompStats,
    next_id: u64,
}

impl AppsSeccompMgr {
    pub fn new() -> Self { Self { filters: BTreeMap::new(), processes: BTreeMap::new(), stats: SeccompStats::default(), next_id: 1 } }

    #[inline]
    pub fn create_filter(&mut self, mode: FilterMode, default: SeccompAction, ts: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let mut f = SeccompFilter::new(id, mode, default);
        f.created_at = ts;
        self.filters.insert(id, f);
        id
    }

    #[inline(always)]
    pub fn add_rule(&mut self, filter_id: u64, rule: SyscallRule) {
        if let Some(f) = self.filters.get_mut(&filter_id) { f.add_rule(rule); }
    }

    #[inline(always)]
    pub fn install_filter(&mut self, pid: u64, filter_id: u64) {
        let proc_sec = self.processes.entry(pid).or_insert_with(|| ProcessSeccomp::new(pid));
        proc_sec.filters.push(filter_id);
    }

    pub fn eval_syscall(&mut self, pid: u64, syscall_nr: u32, ts: u64) -> SeccompAction {
        let filter_ids: Vec<u64> = self.processes.get(&pid).map(|p| p.filters.clone()).unwrap_or_default();
        for fid in filter_ids.iter().rev() {
            if let Some(f) = self.filters.get_mut(fid) {
                let action = f.eval(syscall_nr);
                if action != SeccompAction::Allow {
                    if let Some(p) = self.processes.get_mut(&pid) {
                        p.violations += 1;
                        p.last_violation_ts = ts;
                        p.last_violation_nr = syscall_nr;
                        p.audit_log.push(SeccompViolation { pid, syscall_nr, action_taken: action, ts, ip: 0 });
                    }
                    return action;
                }
            }
        }
        SeccompAction::Allow
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.total_filters = self.filters.len();
        self.stats.total_rules = self.filters.values().map(|f| f.rules.len()).sum();
        self.stats.total_evaluations = self.filters.values().map(|f| f.match_count + f.miss_count).sum();
        self.stats.total_violations = self.processes.values().map(|p| p.violations).sum();
        self.stats.processes_with_filters = self.processes.values().filter(|p| !p.filters.is_empty()).count();
    }

    #[inline(always)]
    pub fn filter(&self, id: u64) -> Option<&SeccompFilter> { self.filters.get(&id) }
    #[inline(always)]
    pub fn process(&self, pid: u64) -> Option<&ProcessSeccomp> { self.processes.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &SeccompStats { &self.stats }
}
