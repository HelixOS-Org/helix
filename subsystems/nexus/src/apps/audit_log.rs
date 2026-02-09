// SPDX-License-Identifier: GPL-2.0
//! Apps audit_log — kernel audit log manager for security event tracking.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// Audit event severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditSeverity {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

/// Audit event category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditCategory {
    Syscall,
    FileAccess,
    NetworkAccess,
    ProcessExec,
    UserAuth,
    CapabilityUse,
    PolicyLoad,
    ConfigChange,
    AnomalyDetect,
    Integrity,
}

/// Audit rule action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditAction {
    Log,
    Deny,
    Allow,
    Alert,
}

/// Audit rule matching field
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditField {
    Pid,
    Uid,
    Gid,
    Syscall,
    Path,
    Architecture,
    Success,
    DevMajor,
    DevMinor,
    Inode,
}

/// Audit rule
#[derive(Debug, Clone)]
pub struct AuditRule {
    pub id: u32,
    pub field: AuditField,
    pub value: u64,
    pub action: AuditAction,
    pub category: AuditCategory,
    pub enabled: bool,
    pub hit_count: u64,
}

impl AuditRule {
    pub fn new(id: u32, field: AuditField, value: u64, action: AuditAction, category: AuditCategory) -> Self {
        Self { id, field, value, action, category, enabled: true, hit_count: 0 }
    }

    pub fn matches_value(&mut self, field: AuditField, val: u64) -> bool {
        if !self.enabled || self.field != field { return false; }
        if self.value == val {
            self.hit_count += 1;
            true
        } else { false }
    }
}

/// Audit log entry
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub seq: u64,
    pub timestamp: u64,
    pub category: AuditCategory,
    pub severity: AuditSeverity,
    pub pid: u32,
    pub uid: u32,
    pub syscall: u32,
    pub result: i32,
    pub message: String,
}

/// Audit buffer status
#[derive(Debug, Clone, Copy)]
pub struct AuditBufferStatus {
    pub entries: u32,
    pub capacity: u32,
    pub lost: u64,
    pub backlog_wait_time_us: u64,
}

/// Audit log stats
#[derive(Debug, Clone)]
pub struct AuditLogStats {
    pub total_events: u64,
    pub total_lost: u64,
    pub rules_count: u32,
    pub events_per_category: BTreeMap<u32, u64>,
    pub severity_counts: [u64; 8],
}

/// Main audit log manager
pub struct AppAuditLog {
    entries: VecDeque<AuditEntry>,
    rules: Vec<AuditRule>,
    max_entries: usize,
    sequence: u64,
    enabled: bool,
    lost_count: u64,
    backlog_limit: usize,
    rate_limit_per_sec: u32,
    rate_window_start: u64,
    rate_window_count: u32,
    severity_counts: [u64; 8],
    category_counts: BTreeMap<u32, u64>,
}

impl AppAuditLog {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::new(), rules: Vec::new(),
            max_entries, sequence: 0, enabled: true,
            lost_count: 0, backlog_limit: 8192,
            rate_limit_per_sec: 0, rate_window_start: 0,
            rate_window_count: 0, severity_counts: [0u64; 8],
            category_counts: BTreeMap::new(),
        }
    }

    pub fn add_rule(&mut self, rule: AuditRule) {
        self.rules.push(rule);
    }

    pub fn remove_rule(&mut self, id: u32) -> bool {
        let before = self.rules.len();
        self.rules.retain(|r| r.id != id);
        self.rules.len() < before
    }

    pub fn log_event(&mut self, category: AuditCategory, severity: AuditSeverity,
                      pid: u32, uid: u32, syscall: u32, result: i32, message: String, now: u64) {
        if !self.enabled { return; }
        if self.rate_limit_per_sec > 0 {
            if now.saturating_sub(self.rate_window_start) >= 1_000_000_000 {
                self.rate_window_start = now;
                self.rate_window_count = 0;
            }
            self.rate_window_count += 1;
            if self.rate_window_count > self.rate_limit_per_sec {
                self.lost_count += 1;
                return;
            }
        }

        if self.entries.len() >= self.max_entries {
            if self.entries.len() >= self.backlog_limit {
                self.lost_count += 1;
                return;
            }
            self.entries.pop_front();
        }

        self.sequence += 1;
        let idx = severity as usize;
        if idx < 8 { self.severity_counts[idx] += 1; }
        *self.category_counts.entry(category as u32).or_insert(0) += 1;

        self.entries.push_back(AuditEntry {
            seq: self.sequence, timestamp: now, category, severity,
            pid, uid, syscall, result, message,
        });
    }

    pub fn search_by_pid(&self, pid: u32) -> Vec<&AuditEntry> {
        self.entries.iter().filter(|e| e.pid == pid).collect()
    }

    pub fn search_by_severity(&self, min_sev: AuditSeverity) -> Vec<&AuditEntry> {
        self.entries.iter().filter(|e| e.severity >= min_sev).collect()
    }

    pub fn search_by_category(&self, cat: AuditCategory) -> Vec<&AuditEntry> {
        self.entries.iter().filter(|e| e.category == cat).collect()
    }

    pub fn recent_entries(&self, n: usize) -> &[AuditEntry] {
        let start = self.entries.len().saturating_sub(n);
        &self.entries[start..]
    }

    pub fn buffer_status(&self) -> AuditBufferStatus {
        AuditBufferStatus {
            entries: self.entries.len() as u32,
            capacity: self.max_entries as u32,
            lost: self.lost_count,
            backlog_wait_time_us: 0,
        }
    }

    pub fn set_rate_limit(&mut self, per_sec: u32) { self.rate_limit_per_sec = per_sec; }
    pub fn set_enabled(&mut self, en: bool) { self.enabled = en; }

    pub fn stats(&self) -> AuditLogStats {
        AuditLogStats {
            total_events: self.sequence,
            total_lost: self.lost_count,
            rules_count: self.rules.len() as u32,
            events_per_category: self.category_counts.clone(),
            severity_counts: self.severity_counts,
        }
    }

    pub fn check_rules(&mut self, field: AuditField, value: u64) -> Option<AuditAction> {
        for rule in self.rules.iter_mut() {
            if rule.matches_value(field, value) {
                return Some(rule.action);
            }
        }
        None
    }

    pub fn flush(&mut self) -> Vec<AuditEntry> {
        let v = core::mem::take(&mut self.entries);
        v
    }
}
