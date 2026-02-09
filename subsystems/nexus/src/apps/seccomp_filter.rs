// SPDX-License-Identifier: GPL-2.0
//! Apps seccomp_filter — seccomp BPF filter engine for syscall sandboxing.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Seccomp mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompMode {
    Disabled,
    Strict,
    Filter,
}

/// Seccomp action (return value)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SeccompAction {
    KillProcess,
    KillThread,
    Trap,
    Errno(u16),
    Trace(u16),
    Log,
    Allow,
    UserNotif,
}

impl SeccompAction {
    pub fn priority(&self) -> u32 {
        match self {
            Self::KillProcess => 0,
            Self::KillThread => 1,
            Self::Trap => 2,
            Self::Errno(_) => 3,
            Self::Trace(_) => 4,
            Self::Log => 5,
            Self::Allow => 6,
            Self::UserNotif => 7,
        }
    }

    #[inline(always)]
    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::KillProcess | Self::KillThread | Self::Trap | Self::Errno(_))
    }
}

/// BPF instruction (simplified)
#[derive(Debug, Clone, Copy)]
pub struct BpfInsn {
    pub code: u16,
    pub jt: u8,
    pub jf: u8,
    pub k: u32,
}

/// Seccomp filter program
#[derive(Debug, Clone)]
pub struct SeccompFilter {
    pub id: u32,
    pub instructions: Vec<BpfInsn>,
    pub default_action: SeccompAction,
    pub syscall_rules: BTreeMap<u32, SeccompAction>,
    pub log_enabled: bool,
    pub tsync: bool,
    pub created_at: u64,
    pub eval_count: u64,
    pub block_count: u64,
}

impl SeccompFilter {
    pub fn new(id: u32, default_action: SeccompAction, now: u64) -> Self {
        Self {
            id, instructions: Vec::new(), default_action,
            syscall_rules: BTreeMap::new(), log_enabled: false, tsync: false,
            created_at: now, eval_count: 0, block_count: 0,
        }
    }

    #[inline(always)]
    pub fn add_rule(&mut self, syscall_nr: u32, action: SeccompAction) {
        self.syscall_rules.insert(syscall_nr, action);
    }

    #[inline]
    pub fn evaluate(&mut self, syscall_nr: u32, _arch: u32) -> SeccompAction {
        self.eval_count += 1;
        let action = self.syscall_rules.get(&syscall_nr)
            .copied()
            .unwrap_or(self.default_action);
        if action.is_blocking() { self.block_count += 1; }
        action
    }

    #[inline(always)]
    pub fn instruction_count(&self) -> usize { self.instructions.len() }

    #[inline(always)]
    pub fn block_rate(&self) -> f64 {
        if self.eval_count == 0 { return 0.0; }
        self.block_count as f64 / self.eval_count as f64
    }
}

/// Seccomp filter chain (stack of filters)
#[derive(Debug)]
pub struct FilterChain {
    pub filters: Vec<SeccompFilter>,
}

impl FilterChain {
    pub fn new() -> Self { Self { filters: Vec::new() } }

    #[inline(always)]
    pub fn push_filter(&mut self, filter: SeccompFilter) {
        self.filters.push(filter);
    }

    #[inline]
    pub fn evaluate(&mut self, syscall_nr: u32, arch: u32) -> SeccompAction {
        let mut most_restrictive = SeccompAction::Allow;
        for filter in self.filters.iter_mut() {
            let action = filter.evaluate(syscall_nr, arch);
            if action.priority() < most_restrictive.priority() {
                most_restrictive = action;
            }
        }
        most_restrictive
    }

    #[inline(always)]
    pub fn filter_count(&self) -> usize { self.filters.len() }
    #[inline(always)]
    pub fn total_instructions(&self) -> usize {
        self.filters.iter().map(|f| f.instruction_count()).sum()
    }
}

/// Per-process seccomp state
#[derive(Debug)]
#[repr(align(64))]
pub struct ProcessSeccompState {
    pub pid: u32,
    pub mode: SeccompMode,
    pub chain: FilterChain,
    pub total_evals: u64,
    pub total_blocked: u64,
    pub notif_pending: u32,
}

impl ProcessSeccompState {
    pub fn new(pid: u32) -> Self {
        Self {
            pid, mode: SeccompMode::Disabled,
            chain: FilterChain::new(),
            total_evals: 0, total_blocked: 0, notif_pending: 0,
        }
    }

    #[inline(always)]
    pub fn install_filter(&mut self, filter: SeccompFilter) {
        self.mode = SeccompMode::Filter;
        self.chain.push_filter(filter);
    }

    #[inline(always)]
    pub fn set_strict(&mut self) {
        self.mode = SeccompMode::Strict;
    }

    pub fn check_syscall(&mut self, syscall_nr: u32, arch: u32) -> SeccompAction {
        self.total_evals += 1;
        match self.mode {
            SeccompMode::Disabled => SeccompAction::Allow,
            SeccompMode::Strict => {
                match syscall_nr {
                    0 | 1 | 3 | 60 => SeccompAction::Allow,  // read, write, close, exit
                    _ => {
                        self.total_blocked += 1;
                        SeccompAction::KillThread
                    }
                }
            }
            SeccompMode::Filter => {
                let action = self.chain.evaluate(syscall_nr, arch);
                if action.is_blocking() { self.total_blocked += 1; }
                action
            }
        }
    }
}

/// Seccomp notification (for user-notif)
#[derive(Debug, Clone)]
pub struct SeccompNotif {
    pub id: u64,
    pub pid: u32,
    pub syscall_nr: u32,
    pub args: [u64; 6],
    pub timestamp: u64,
}

/// Seccomp filter stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SeccompFilterStats {
    pub tracked_processes: u32,
    pub filter_mode_count: u32,
    pub strict_mode_count: u32,
    pub total_evals: u64,
    pub total_blocked: u64,
    pub total_filters_installed: u64,
}

/// Main seccomp filter manager
pub struct AppSeccompFilter {
    processes: BTreeMap<u32, ProcessSeccompState>,
    notifications: Vec<SeccompNotif>,
    max_notifications: usize,
    next_filter_id: u32,
    total_filters_installed: u64,
}

impl AppSeccompFilter {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(), notifications: Vec::new(),
            max_notifications: 1024, next_filter_id: 1,
            total_filters_installed: 0,
        }
    }

    #[inline(always)]
    pub fn create_process(&mut self, pid: u32) {
        self.processes.insert(pid, ProcessSeccompState::new(pid));
    }

    #[inline(always)]
    pub fn remove_process(&mut self, pid: u32) -> bool {
        self.processes.remove(&pid).is_some()
    }

    #[inline]
    pub fn install_filter(&mut self, pid: u32, default_action: SeccompAction, now: u64) -> Option<u32> {
        let id = self.next_filter_id;
        self.next_filter_id += 1;
        let filter = SeccompFilter::new(id, default_action, now);
        self.processes.get_mut(&pid)?.install_filter(filter);
        self.total_filters_installed += 1;
        Some(id)
    }

    #[inline]
    pub fn add_rule(&mut self, pid: u32, filter_idx: usize, syscall_nr: u32, action: SeccompAction) -> bool {
        if let Some(state) = self.processes.get_mut(&pid) {
            if let Some(filter) = state.chain.filters.get_mut(filter_idx) {
                filter.add_rule(syscall_nr, action);
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn check_syscall(&mut self, pid: u32, syscall_nr: u32, arch: u32) -> SeccompAction {
        if let Some(state) = self.processes.get_mut(&pid) {
            state.check_syscall(syscall_nr, arch)
        } else { SeccompAction::Allow }
    }

    #[inline]
    pub fn fork_filters(&mut self, parent: u32, child: u32) -> bool {
        if let Some(parent_state) = self.processes.get(&parent) {
            let mut child_state = ProcessSeccompState::new(child);
            child_state.mode = parent_state.mode;
            for f in &parent_state.chain.filters {
                child_state.chain.push_filter(f.clone());
            }
            self.processes.insert(child, child_state);
            true
        } else { false }
    }

    pub fn stats(&self) -> SeccompFilterStats {
        let filter_count = self.processes.values()
            .filter(|p| p.mode == SeccompMode::Filter).count() as u32;
        let strict_count = self.processes.values()
            .filter(|p| p.mode == SeccompMode::Strict).count() as u32;
        let total_evals: u64 = self.processes.values().map(|p| p.total_evals).sum();
        let total_blocked: u64 = self.processes.values().map(|p| p.total_blocked).sum();
        SeccompFilterStats {
            tracked_processes: self.processes.len() as u32,
            filter_mode_count: filter_count,
            strict_mode_count: strict_count,
            total_evals, total_blocked,
            total_filters_installed: self.total_filters_installed,
        }
    }
}
