// SPDX-License-Identifier: GPL-2.0
//! Apps seccomp_v2 — advanced seccomp BPF filtering.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Seccomp action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompAction {
    Allow,
    Kill,
    KillProcess,
    Trap,
    Errno(u16),
    Trace(u16),
    Log,
    UserNotify,
}

/// BPF instruction
#[derive(Debug, Clone, Copy)]
pub struct BpfInsn {
    pub code: u16,
    pub jt: u8,
    pub jf: u8,
    pub k: u32,
}

/// Seccomp filter
#[derive(Debug)]
pub struct SeccompFilter {
    pub id: u64,
    pub instructions: Vec<BpfInsn>,
    pub default_action: SeccompAction,
    pub syscall_rules: BTreeMap<u32, SeccompAction>,
    pub match_count: u64,
    pub deny_count: u64,
}

impl SeccompFilter {
    pub fn new(id: u64, default: SeccompAction) -> Self {
        Self { id, instructions: Vec::new(), default_action: default, syscall_rules: BTreeMap::new(), match_count: 0, deny_count: 0 }
    }

    #[inline(always)]
    pub fn add_rule(&mut self, syscall_nr: u32, action: SeccompAction) { self.syscall_rules.insert(syscall_nr, action); }

    #[inline]
    pub fn evaluate(&mut self, syscall_nr: u32) -> SeccompAction {
        self.match_count += 1;
        let action = self.syscall_rules.get(&syscall_nr).copied().unwrap_or(self.default_action);
        if !matches!(action, SeccompAction::Allow) { self.deny_count += 1; }
        action
    }
}

/// Seccomp notify instance
#[derive(Debug)]
pub struct SeccompNotifyInstance {
    pub id: u64,
    pub pending: Vec<SeccompNotification>,
    pub total_notified: u64,
    pub total_responded: u64,
}

/// Notification
#[derive(Debug, Clone)]
pub struct SeccompNotification {
    pub notify_id: u64,
    pub pid: u64,
    pub syscall_nr: u32,
    pub args: [u64; 6],
    pub responded: bool,
    pub response_error: i32,
    pub response_val: u64,
}

impl SeccompNotifyInstance {
    pub fn new(id: u64) -> Self { Self { id, pending: Vec::new(), total_notified: 0, total_responded: 0 } }

    #[inline]
    pub fn notify(&mut self, pid: u64, syscall: u32, args: [u64; 6]) -> u64 {
        self.total_notified += 1;
        let nid = self.total_notified;
        self.pending.push(SeccompNotification { notify_id: nid, pid, syscall_nr: syscall, args, responded: false, response_error: 0, response_val: 0 });
        nid
    }

    #[inline]
    pub fn respond(&mut self, nid: u64, error: i32, val: u64) {
        if let Some(n) = self.pending.iter_mut().find(|n| n.notify_id == nid) {
            n.responded = true; n.response_error = error; n.response_val = val;
            self.total_responded += 1;
        }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SeccompV2Stats {
    pub total_filters: u32,
    pub total_evaluations: u64,
    pub total_denials: u64,
    pub total_notifications: u64,
    pub denial_rate: f64,
}

/// Main seccomp v2
pub struct AppSeccompV2 {
    filters: BTreeMap<u64, SeccompFilter>,
    notifiers: BTreeMap<u64, SeccompNotifyInstance>,
    next_id: u64,
}

impl AppSeccompV2 {
    pub fn new() -> Self { Self { filters: BTreeMap::new(), notifiers: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn install_filter(&mut self, default: SeccompAction) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.filters.insert(id, SeccompFilter::new(id, default));
        id
    }

    #[inline(always)]
    pub fn add_rule(&mut self, filter: u64, syscall: u32, action: SeccompAction) {
        if let Some(f) = self.filters.get_mut(&filter) { f.add_rule(syscall, action); }
    }

    #[inline(always)]
    pub fn evaluate(&mut self, filter: u64, syscall: u32) -> SeccompAction {
        self.filters.get_mut(&filter).map(|f| f.evaluate(syscall)).unwrap_or(SeccompAction::Kill)
    }

    #[inline]
    pub fn stats(&self) -> SeccompV2Stats {
        let evals: u64 = self.filters.values().map(|f| f.match_count).sum();
        let denials: u64 = self.filters.values().map(|f| f.deny_count).sum();
        let notifs: u64 = self.notifiers.values().map(|n| n.total_notified).sum();
        let rate = if evals == 0 { 0.0 } else { denials as f64 / evals as f64 };
        SeccompV2Stats { total_filters: self.filters.len() as u32, total_evaluations: evals, total_denials: denials, total_notifications: notifs, denial_rate: rate }
    }
}
