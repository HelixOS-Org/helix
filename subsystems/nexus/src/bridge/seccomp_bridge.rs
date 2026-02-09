// SPDX-License-Identifier: GPL-2.0
//! Bridge seccomp_bridge — seccomp-bpf filter bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Seccomp action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompBridgeAction {
    Allow,
    Kill,
    KillProcess,
    Trap,
    Errno(u16),
    Trace,
    Log,
    UserNotif,
}

/// Seccomp filter instruction
#[derive(Debug, Clone, Copy)]
pub struct SeccompInsn {
    pub code: u16,
    pub jt: u8,
    pub jf: u8,
    pub k: u32,
}

/// Seccomp filter
#[derive(Debug)]
pub struct SeccompBridgeFilter {
    pub id: u64,
    pub pid: u64,
    pub instructions: Vec<SeccompInsn>,
    pub default_action: SeccompBridgeAction,
    pub syscall_checks: u64,
    pub denied: u64,
    pub allowed: u64,
}

impl SeccompBridgeFilter {
    pub fn new(id: u64, pid: u64, default: SeccompBridgeAction) -> Self {
        Self { id, pid, instructions: Vec::new(), default_action: default, syscall_checks: 0, denied: 0, allowed: 0 }
    }
}

/// Process seccomp state
#[derive(Debug)]
pub struct ProcessSeccomp {
    pub pid: u64,
    pub mode: u8,
    pub filter_count: u32,
    pub tsync: bool,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SeccompBridgeStats {
    pub total_filters: u32,
    pub total_processes: u32,
    pub total_checks: u64,
    pub total_denied: u64,
    pub total_allowed: u64,
}

/// Main bridge seccomp
#[repr(align(64))]
pub struct BridgeSeccomp {
    filters: BTreeMap<u64, SeccompBridgeFilter>,
    processes: BTreeMap<u64, ProcessSeccomp>,
    next_id: u64,
}

impl BridgeSeccomp {
    pub fn new() -> Self { Self { filters: BTreeMap::new(), processes: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn install_filter(&mut self, pid: u64, default: SeccompBridgeAction) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.filters.insert(id, SeccompBridgeFilter::new(id, pid, default));
        let proc = self.processes.entry(pid).or_insert(ProcessSeccomp { pid, mode: 2, filter_count: 0, tsync: false });
        proc.filter_count += 1;
        id
    }

    #[inline]
    pub fn check_syscall(&mut self, filter_id: u64, allowed: bool) {
        if let Some(f) = self.filters.get_mut(&filter_id) {
            f.syscall_checks += 1;
            if allowed { f.allowed += 1; } else { f.denied += 1; }
        }
    }

    #[inline]
    pub fn stats(&self) -> SeccompBridgeStats {
        let checks: u64 = self.filters.values().map(|f| f.syscall_checks).sum();
        let denied: u64 = self.filters.values().map(|f| f.denied).sum();
        let allowed: u64 = self.filters.values().map(|f| f.allowed).sum();
        SeccompBridgeStats { total_filters: self.filters.len() as u32, total_processes: self.processes.len() as u32, total_checks: checks, total_denied: denied, total_allowed: allowed }
    }
}

// ============================================================================
// Merged from seccomp_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompV2Action {
    Allow,
    Kill,
    KillProcess,
    Trap,
    Errno(u32),
    Trace,
    Log,
    UserNotif,
}

/// Seccomp filter comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompV2Cmp {
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
    MaskedEqual(u64),
}

/// A single seccomp filter rule
#[derive(Debug, Clone)]
pub struct SeccompV2Rule {
    pub syscall_nr: u32,
    pub arg_index: Option<u8>,
    pub cmp: Option<SeccompV2Cmp>,
    pub cmp_value: u64,
    pub action: SeccompV2Action,
    pub match_count: u64,
}

/// A seccomp filter program (collection of rules)
#[derive(Debug, Clone)]
pub struct SeccompV2Filter {
    pub id: u64,
    pub rules: Vec<SeccompV2Rule>,
    pub default_action: SeccompV2Action,
    pub log_enabled: bool,
    pub total_checks: u64,
    pub total_denials: u64,
}

impl SeccompV2Filter {
    pub fn new(id: u64, default: SeccompV2Action) -> Self {
        Self {
            id,
            rules: Vec::new(),
            default_action: default,
            log_enabled: false,
            total_checks: 0,
            total_denials: 0,
        }
    }

    #[inline]
    pub fn add_rule(&mut self, syscall_nr: u32, action: SeccompV2Action) {
        self.rules.push(SeccompV2Rule {
            syscall_nr,
            arg_index: None,
            cmp: None,
            cmp_value: 0,
            action,
            match_count: 0,
        });
    }

    #[inline]
    pub fn add_arg_rule(&mut self, syscall_nr: u32, arg_idx: u8, cmp: SeccompV2Cmp, value: u64, action: SeccompV2Action) {
        self.rules.push(SeccompV2Rule {
            syscall_nr,
            arg_index: Some(arg_idx),
            cmp: Some(cmp),
            cmp_value: value,
            action,
            match_count: 0,
        });
    }

    pub fn check_syscall(&mut self, syscall_nr: u32) -> SeccompV2Action {
        self.total_checks += 1;
        for rule in self.rules.iter_mut() {
            if rule.syscall_nr == syscall_nr && rule.arg_index.is_none() {
                rule.match_count += 1;
                match rule.action {
                    SeccompV2Action::Allow => {}
                    _ => self.total_denials += 1,
                }
                return rule.action;
            }
        }
        self.default_action
    }
}

/// A process's seccomp state
#[derive(Debug, Clone)]
pub struct ProcessSeccompV2 {
    pub pid: u64,
    pub filters: Vec<SeccompV2Filter>,
    pub strict_mode: bool,
    pub notif_fd: Option<u64>,
}

impl ProcessSeccompV2 {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            filters: Vec::new(),
            strict_mode: false,
            notif_fd: None,
        }
    }

    #[inline(always)]
    pub fn install_filter(&mut self, filter: SeccompV2Filter) {
        self.filters.push(filter);
    }

    pub fn check(&mut self, syscall_nr: u32) -> SeccompV2Action {
        let mut result = SeccompV2Action::Allow;
        for filter in self.filters.iter_mut() {
            let action = filter.check_syscall(syscall_nr);
            match action {
                SeccompV2Action::Kill | SeccompV2Action::KillProcess => return action,
                SeccompV2Action::Allow => {}
                _ => result = action,
            }
        }
        result
    }
}

/// Statistics for seccomp V2 bridge
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SeccompV2BridgeStats {
    pub processes_filtered: u64,
    pub filters_installed: u64,
    pub syscalls_checked: u64,
    pub syscalls_allowed: u64,
    pub syscalls_denied: u64,
    pub kills_triggered: u64,
    pub notifications_sent: u64,
}

/// Main seccomp V2 bridge manager
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeSeccompV2 {
    processes: BTreeMap<u64, ProcessSeccompV2>,
    next_filter_id: u64,
    stats: SeccompV2BridgeStats,
}

impl BridgeSeccompV2 {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            next_filter_id: 1,
            stats: SeccompV2BridgeStats {
                processes_filtered: 0,
                filters_installed: 0,
                syscalls_checked: 0,
                syscalls_allowed: 0,
                syscalls_denied: 0,
                kills_triggered: 0,
                notifications_sent: 0,
            },
        }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.processes.insert(pid, ProcessSeccompV2::new(pid));
        self.stats.processes_filtered += 1;
    }

    #[inline]
    pub fn install_filter(&mut self, pid: u64, default: SeccompV2Action) -> Option<u64> {
        if let Some(proc) = self.processes.get_mut(&pid) {
            let id = self.next_filter_id;
            self.next_filter_id += 1;
            proc.install_filter(SeccompV2Filter::new(id, default));
            self.stats.filters_installed += 1;
            Some(id)
        } else {
            None
        }
    }

    pub fn check_syscall(&mut self, pid: u64, syscall_nr: u32) -> SeccompV2Action {
        self.stats.syscalls_checked += 1;
        if let Some(proc) = self.processes.get_mut(&pid) {
            let result = proc.check(syscall_nr);
            match result {
                SeccompV2Action::Allow => self.stats.syscalls_allowed += 1,
                SeccompV2Action::Kill | SeccompV2Action::KillProcess => self.stats.kills_triggered += 1,
                _ => self.stats.syscalls_denied += 1,
            }
            result
        } else {
            SeccompV2Action::Allow
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &SeccompV2BridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from seccomp_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompV3Action {
    Allow,
    Kill,
    KillProcess,
    Trap,
    Errno(u16),
    Trace(u16),
    Log,
    UserNotif,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompV3Arch {
    X86_64,
    X86,
    Aarch64,
    Arm,
    Riscv64,
    Mips64,
    S390x,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompV3CacheResult {
    Hit,
    Miss,
    Bypass,
}

#[derive(Debug, Clone)]
pub struct SeccompV3Rule {
    pub syscall_nr: u32,
    pub arch: SeccompV3Arch,
    pub action: SeccompV3Action,
    pub arg_checks: Vec<SeccompV3ArgCheck>,
    pub priority: u16,
}

#[derive(Debug, Clone)]
pub struct SeccompV3ArgCheck {
    pub arg_index: u8,
    pub op: SeccompV3CmpOp,
    pub value: u64,
    pub mask: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompV3CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    MaskedEq,
}

impl SeccompV3ArgCheck {
    pub fn matches(&self, arg_val: u64) -> bool {
        let val = if self.op == SeccompV3CmpOp::MaskedEq {
            arg_val & self.mask
        } else {
            arg_val
        };
        match self.op {
            SeccompV3CmpOp::Eq | SeccompV3CmpOp::MaskedEq => val == self.value,
            SeccompV3CmpOp::Ne => val != self.value,
            SeccompV3CmpOp::Lt => val < self.value,
            SeccompV3CmpOp::Le => val <= self.value,
            SeccompV3CmpOp::Gt => val > self.value,
            SeccompV3CmpOp::Ge => val >= self.value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SeccompV3Filter {
    pub id: u64,
    pub rules: Vec<SeccompV3Rule>,
    pub default_action: SeccompV3Action,
    pub log_enabled: bool,
    pub notif_enabled: bool,
    pub total_evaluations: u64,
    pub cache_hits: u64,
}

impl SeccompV3Filter {
    pub fn new(id: u64, default_action: SeccompV3Action) -> Self {
        Self {
            id,
            rules: Vec::new(),
            default_action,
            log_enabled: false,
            notif_enabled: false,
            total_evaluations: 0,
            cache_hits: 0,
        }
    }

    #[inline(always)]
    pub fn add_rule(&mut self, rule: SeccompV3Rule) {
        self.rules.push(rule);
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn evaluate(&mut self, syscall: u32, arch: SeccompV3Arch, args: &[u64; 6]) -> SeccompV3Action {
        self.total_evaluations += 1;
        for rule in &self.rules {
            if rule.syscall_nr == syscall && rule.arch == arch {
                let all_match = rule.arg_checks.iter().all(|c| {
                    let idx = c.arg_index as usize;
                    idx < 6 && c.matches(args[idx])
                });
                if all_match {
                    return rule.action;
                }
            }
        }
        self.default_action
    }

    #[inline(always)]
    pub fn cache_hit_rate(&self) -> u64 {
        if self.total_evaluations == 0 { 0 } else { (self.cache_hits * 100) / self.total_evaluations }
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SeccompV3BridgeStats {
    pub total_filters: u64,
    pub total_rules: u64,
    pub total_evaluations: u64,
    pub total_kills: u64,
    pub total_notifs: u64,
}

#[repr(align(64))]
pub struct BridgeSeccompV3 {
    filters: BTreeMap<u64, SeccompV3Filter>,
    process_filters: BTreeMap<u64, Vec<u64>>,
    next_id: AtomicU64,
    stats: SeccompV3BridgeStats,
}

impl BridgeSeccompV3 {
    pub fn new() -> Self {
        Self {
            filters: BTreeMap::new(),
            process_filters: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            stats: SeccompV3BridgeStats {
                total_filters: 0,
                total_rules: 0,
                total_evaluations: 0,
                total_kills: 0,
                total_notifs: 0,
            },
        }
    }

    #[inline]
    pub fn create_filter(&mut self, default_action: SeccompV3Action) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let filter = SeccompV3Filter::new(id, default_action);
        self.filters.insert(id, filter);
        self.stats.total_filters += 1;
        id
    }

    #[inline(always)]
    pub fn attach_to_process(&mut self, filter_id: u64, pid: u64) {
        self.process_filters.entry(pid).or_insert_with(Vec::new).push(filter_id);
    }

    #[inline(always)]
    pub fn stats(&self) -> &SeccompV3BridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from seccomp_v4_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompV4Action {
    Allow,
    Kill,
    KillProcess,
    Trap,
    Errno,
    Trace,
    Log,
    UserNotif,
}

/// Seccomp v4 operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompV4Op {
    SetFilter,
    GetFilter,
    UserNotifRecv,
    UserNotifSend,
    UserNotifAddFd,
    GetNotifSizes,
}

/// Seccomp v4 record
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SeccompV4Record {
    pub op: SeccompV4Op,
    pub action: SeccompV4Action,
    pub syscall_nr: u32,
    pub pid: u32,
    pub filter_count: u32,
    pub latency_ns: u64,
}

impl SeccompV4Record {
    pub fn new(op: SeccompV4Op, syscall_nr: u32) -> Self {
        Self { op, action: SeccompV4Action::Allow, syscall_nr, pid: 0, filter_count: 0, latency_ns: 0 }
    }
}

/// Seccomp v4 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SeccompV4BridgeStats {
    pub total_ops: u64,
    pub filters_installed: u64,
    pub user_notifs: u64,
    pub kills: u64,
    pub errors: u64,
}

/// Main bridge seccomp v4
#[derive(Debug)]
pub struct BridgeSeccompV4 {
    pub stats: SeccompV4BridgeStats,
}

impl BridgeSeccompV4 {
    pub fn new() -> Self {
        Self { stats: SeccompV4BridgeStats { total_ops: 0, filters_installed: 0, user_notifs: 0, kills: 0, errors: 0 } }
    }

    #[inline]
    pub fn record(&mut self, rec: &SeccompV4Record) {
        self.stats.total_ops += 1;
        match rec.op {
            SeccompV4Op::SetFilter => self.stats.filters_installed += 1,
            SeccompV4Op::UserNotifRecv | SeccompV4Op::UserNotifSend => self.stats.user_notifs += 1,
            _ => {}
        }
        if matches!(rec.action, SeccompV4Action::Kill | SeccompV4Action::KillProcess) { self.stats.kills += 1; }
    }
}
