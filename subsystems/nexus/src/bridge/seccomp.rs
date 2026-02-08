//! # Bridge Seccomp Engine
//!
//! Syscall filtering via seccomp-BPF style rules:
//! - Per-process filter programs
//! - Rule-based syscall allow/deny/trap/log
//! - Argument-level filtering
//! - Filter inheritance on fork
//! - Audit log for denied syscalls
//! - Performance-optimized bitmap fast path

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Seccomp action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompAction {
    Allow,
    Kill,
    Trap,
    Errno(u16),
    Trace,
    Log,
    UserNotif,
}

/// Argument comparator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgCmp {
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
    MaskedEqual(u64),
}

/// Argument condition
#[derive(Debug, Clone)]
pub struct ArgCondition {
    pub arg_index: u8,
    pub comparator: ArgCmp,
    pub value: u64,
}

impl ArgCondition {
    pub fn matches(&self, arg_val: u64) -> bool {
        match self.comparator {
            ArgCmp::Equal => arg_val == self.value,
            ArgCmp::NotEqual => arg_val != self.value,
            ArgCmp::LessThan => arg_val < self.value,
            ArgCmp::LessEqual => arg_val <= self.value,
            ArgCmp::GreaterThan => arg_val > self.value,
            ArgCmp::GreaterEqual => arg_val >= self.value,
            ArgCmp::MaskedEqual(mask) => (arg_val & mask) == self.value,
        }
    }
}

/// Seccomp rule
#[derive(Debug, Clone)]
pub struct SeccompRule {
    pub syscall_nr: u32,
    pub action: SeccompAction,
    pub conditions: Vec<ArgCondition>,
    pub priority: u16,
    pub hit_count: u64,
}

impl SeccompRule {
    pub fn simple(syscall_nr: u32, action: SeccompAction) -> Self {
        Self {
            syscall_nr,
            action,
            conditions: Vec::new(),
            priority: 0,
            hit_count: 0,
        }
    }

    pub fn with_condition(mut self, cond: ArgCondition) -> Self {
        self.conditions.push(cond);
        self
    }

    pub fn matches(&self, syscall_nr: u32, args: &[u64; 6]) -> bool {
        if self.syscall_nr != syscall_nr { return false; }
        self.conditions.iter().all(|c| {
            if (c.arg_index as usize) < 6 {
                c.matches(args[c.arg_index as usize])
            } else { false }
        })
    }
}

/// Per-process seccomp filter
#[derive(Debug, Clone)]
pub struct SeccompFilter {
    pub pid: u64,
    pub rules: Vec<SeccompRule>,
    pub default_action: SeccompAction,
    /// Bitmap fast-path: bit set = syscall explicitly allowed
    pub allow_bitmap: [u64; 8], // 512 syscalls
    pub strict_mode: bool,
    pub inherit_on_fork: bool,
    pub total_checks: u64,
    pub total_denied: u64,
}

impl SeccompFilter {
    pub fn new(pid: u64, default_action: SeccompAction) -> Self {
        Self {
            pid,
            rules: Vec::new(),
            default_action,
            allow_bitmap: [0; 8],
            strict_mode: false,
            inherit_on_fork: true,
            total_checks: 0,
            total_denied: 0,
        }
    }

    pub fn add_rule(&mut self, rule: SeccompRule) {
        // Update bitmap for simple allow rules
        if matches!(rule.action, SeccompAction::Allow) && rule.conditions.is_empty() {
            let nr = rule.syscall_nr as usize;
            if nr < 512 {
                self.allow_bitmap[nr / 64] |= 1u64 << (nr % 64);
            }
        }
        self.rules.push(rule);
        // Sort by priority (higher first)
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Fast path: check bitmap
    fn bitmap_allows(&self, syscall_nr: u32) -> bool {
        let nr = syscall_nr as usize;
        if nr >= 512 { return false; }
        (self.allow_bitmap[nr / 64] >> (nr % 64)) & 1 == 1
    }

    /// Check syscall against filter
    pub fn check(&mut self, syscall_nr: u32, args: &[u64; 6]) -> SeccompAction {
        self.total_checks += 1;

        // Fast path
        if self.bitmap_allows(syscall_nr) {
            return SeccompAction::Allow;
        }

        // Slow path: check rules
        for rule in &mut self.rules {
            if rule.matches(syscall_nr, args) {
                rule.hit_count += 1;
                if !matches!(rule.action, SeccompAction::Allow) {
                    self.total_denied += 1;
                }
                return rule.action;
            }
        }

        if !matches!(self.default_action, SeccompAction::Allow) {
            self.total_denied += 1;
        }
        self.default_action
    }

    pub fn denial_rate(&self) -> f64 {
        if self.total_checks == 0 { return 0.0; }
        self.total_denied as f64 / self.total_checks as f64
    }
}

/// Audit log entry
#[derive(Debug, Clone)]
pub struct SeccompAuditEntry {
    pub pid: u64,
    pub syscall_nr: u32,
    pub action: SeccompAction,
    pub timestamp: u64,
    pub args: [u64; 6],
}

/// Seccomp engine stats
#[derive(Debug, Clone, Default)]
pub struct BridgeSeccompStats {
    pub filtered_processes: usize,
    pub total_rules: usize,
    pub total_checks: u64,
    pub total_denied: u64,
    pub audit_log_size: usize,
    pub strict_mode_count: usize,
}

/// Bridge Seccomp Engine
pub struct BridgeSeccompEngine {
    filters: BTreeMap<u64, SeccompFilter>,
    audit_log: Vec<SeccompAuditEntry>,
    max_audit: usize,
    stats: BridgeSeccompStats,
}

impl BridgeSeccompEngine {
    pub fn new() -> Self {
        Self {
            filters: BTreeMap::new(),
            audit_log: Vec::new(),
            max_audit: 1024,
            stats: BridgeSeccompStats::default(),
        }
    }

    pub fn install_filter(&mut self, filter: SeccompFilter) {
        self.filters.insert(filter.pid, filter);
        self.recompute();
    }

    pub fn remove_filter(&mut self, pid: u64) {
        self.filters.remove(&pid);
        self.recompute();
    }

    /// Inherit filter on fork
    pub fn fork_inherit(&mut self, parent_pid: u64, child_pid: u64) {
        if let Some(parent) = self.filters.get(&parent_pid) {
            if parent.inherit_on_fork {
                let mut child_filter = parent.clone();
                child_filter.pid = child_pid;
                child_filter.total_checks = 0;
                child_filter.total_denied = 0;
                self.filters.insert(child_pid, child_filter);
            }
        }
        self.recompute();
    }

    /// Check a syscall for a process
    pub fn check(&mut self, pid: u64, syscall_nr: u32, args: &[u64; 6], now: u64) -> SeccompAction {
        let action = if let Some(filter) = self.filters.get_mut(&pid) {
            filter.check(syscall_nr, args)
        } else {
            SeccompAction::Allow
        };

        // Audit non-allow actions
        if !matches!(action, SeccompAction::Allow) {
            let entry = SeccompAuditEntry {
                pid,
                syscall_nr,
                action,
                timestamp: now,
                args: *args,
            };
            if self.audit_log.len() >= self.max_audit {
                self.audit_log.remove(0);
            }
            self.audit_log.push(entry);
        }

        self.recompute();
        action
    }

    /// Get hot denied syscalls (most frequently denied)
    pub fn hot_denials(&self) -> Vec<(u32, u64)> {
        let mut counts: BTreeMap<u32, u64> = BTreeMap::new();
        for entry in &self.audit_log {
            *counts.entry(entry.syscall_nr).or_insert(0) += 1;
        }
        let mut sorted: Vec<(u32, u64)> = counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(10);
        sorted
    }

    fn recompute(&mut self) {
        self.stats.filtered_processes = self.filters.len();
        self.stats.total_rules = self.filters.values().map(|f| f.rules.len()).sum();
        self.stats.total_checks = self.filters.values().map(|f| f.total_checks).sum();
        self.stats.total_denied = self.filters.values().map(|f| f.total_denied).sum();
        self.stats.audit_log_size = self.audit_log.len();
        self.stats.strict_mode_count = self.filters.values().filter(|f| f.strict_mode).count();
    }

    pub fn stats(&self) -> &BridgeSeccompStats {
        &self.stats
    }

    pub fn audit_log(&self) -> &[SeccompAuditEntry] {
        &self.audit_log
    }
}
