// SPDX-License-Identifier: GPL-2.0
//! Apps audit trail — per-application security audit logging and compliance tracking.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// Audit event severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditSeverity {
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Emergency,
}

/// Audit event category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditCategory {
    /// Process lifecycle (exec, fork, exit)
    Process,
    /// File access (open, read, write, unlink)
    FileAccess,
    /// Network operations (connect, bind, accept)
    Network,
    /// Credential changes (setuid, setgid, capabilities)
    Credential,
    /// Security policy (seccomp, LSM, landlock)
    Security,
    /// IPC operations (signals, semaphores, shared memory)
    Ipc,
    /// Device access
    Device,
    /// Mount operations
    Mount,
    /// Module operations
    Module,
    /// Administrative actions
    Admin,
}

/// An audit record
#[derive(Debug, Clone)]
pub struct AuditRecord {
    pub seq: u64,
    pub timestamp_ns: u64,
    pub pid: u64,
    pub uid: u32,
    pub gid: u32,
    pub severity: AuditSeverity,
    pub category: AuditCategory,
    pub syscall_nr: i32,
    pub result: i32,
    pub subject: String,
    pub object: String,
    pub details: String,
}

impl AuditRecord {
    pub fn new(
        seq: u64,
        pid: u64,
        severity: AuditSeverity,
        category: AuditCategory,
        subject: String,
    ) -> Self {
        Self {
            seq,
            timestamp_ns: 0,
            pid,
            uid: 0,
            gid: 0,
            severity,
            category,
            syscall_nr: -1,
            result: 0,
            subject,
            object: String::new(),
            details: String::new(),
        }
    }

    #[inline(always)]
    pub fn with_object(mut self, object: String) -> Self {
        self.object = object;
        self
    }

    #[inline(always)]
    pub fn with_details(mut self, details: String) -> Self {
        self.details = details;
        self
    }

    #[inline]
    pub fn with_creds(mut self, uid: u32, gid: u32) -> Self {
        self.uid = uid;
        self.gid = gid;
        self
    }

    /// FNV-1a record fingerprint
    pub fn fingerprint(&self) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for &b in &self.pid.to_le_bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash ^= self.category as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= self.syscall_nr as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash
    }
}

/// Audit filter rule
#[derive(Debug, Clone)]
pub struct AuditFilter {
    pub name: String,
    pub action: FilterAction,
    pub min_severity: Option<AuditSeverity>,
    pub category: Option<AuditCategory>,
    pub pid_match: Option<u64>,
    pub uid_match: Option<u32>,
    pub syscall_match: Option<i32>,
    pub enabled: bool,
    match_count: u64,
}

/// Filter action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterAction {
    /// Log the event
    Log,
    /// Suppress the event
    Suppress,
    /// Alert (high-priority notification)
    Alert,
    /// Both log and alert
    LogAndAlert,
}

impl AuditFilter {
    pub fn new(name: String, action: FilterAction) -> Self {
        Self {
            name,
            action,
            min_severity: None,
            category: None,
            pid_match: None,
            uid_match: None,
            syscall_match: None,
            enabled: true,
            match_count: 0,
        }
    }

    pub fn matches(&self, record: &AuditRecord) -> bool {
        if !self.enabled {
            return false;
        }
        if let Some(min_sev) = self.min_severity {
            if record.severity < min_sev {
                return false;
            }
        }
        if let Some(cat) = self.category {
            if record.category != cat {
                return false;
            }
        }
        if let Some(pid) = self.pid_match {
            if record.pid != pid {
                return false;
            }
        }
        if let Some(uid) = self.uid_match {
            if record.uid != uid {
                return false;
            }
        }
        if let Some(nr) = self.syscall_match {
            if record.syscall_nr != nr {
                return false;
            }
        }
        true
    }
}

/// Per-process audit state
#[derive(Debug)]
#[repr(align(64))]
pub struct ProcessAuditState {
    pub pid: u64,
    pub audit_enabled: bool,
    pub record_count: u64,
    pub alerts: u64,
    pub last_record_ns: u64,
    per_category: BTreeMap<u8, u64>,
}

impl ProcessAuditState {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            audit_enabled: true,
            record_count: 0,
            alerts: 0,
            last_record_ns: 0,
            per_category: BTreeMap::new(),
        }
    }

    #[inline]
    pub fn record(&mut self, category: AuditCategory, timestamp_ns: u64) {
        self.record_count += 1;
        self.last_record_ns = timestamp_ns;
        *self.per_category.entry(category as u8).or_insert(0) += 1;
    }

    #[inline(always)]
    pub fn rate_per_second(&self, elapsed_ns: u64) -> f64 {
        if elapsed_ns == 0 { return 0.0; }
        self.record_count as f64 / (elapsed_ns as f64 / 1_000_000_000.0)
    }
}

/// Audit trail stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AuditTrailStats {
    pub total_records: u64,
    pub total_alerts: u64,
    pub total_suppressed: u64,
    pub buffer_overflows: u64,
    pub filter_matches: u64,
    pub processes_audited: u64,
}

/// Main apps audit trail manager
pub struct AppAuditTrail {
    records: VecDeque<AuditRecord>,
    processes: BTreeMap<u64, ProcessAuditState>,
    filters: Vec<AuditFilter>,
    next_seq: u64,
    max_records: usize,
    rate_limit_per_sec: u64,
    stats: AuditTrailStats,
}

impl AppAuditTrail {
    pub fn new() -> Self {
        Self {
            records: VecDeque::new(),
            processes: BTreeMap::new(),
            filters: Vec::new(),
            next_seq: 1,
            max_records: 65536,
            rate_limit_per_sec: 10000,
            stats: AuditTrailStats {
                total_records: 0,
                total_alerts: 0,
                total_suppressed: 0,
                buffer_overflows: 0,
                filter_matches: 0,
                processes_audited: 0,
            },
        }
    }

    #[inline]
    pub fn register_process(&mut self, pid: u64) {
        if !self.processes.contains_key(&pid) {
            self.processes.insert(pid, ProcessAuditState::new(pid));
            self.stats.processes_audited += 1;
        }
    }

    #[inline(always)]
    pub fn add_filter(&mut self, filter: AuditFilter) {
        self.filters.push(filter);
    }

    pub fn emit(&mut self, mut record: AuditRecord) -> Option<u64> {
        record.seq = self.next_seq;
        self.next_seq += 1;

        // Apply filters
        let mut action = FilterAction::Log;
        for filter in &mut self.filters {
            if filter.matches(&record) {
                filter.match_count += 1;
                self.stats.filter_matches += 1;
                action = filter.action;
                break;
            }
        }

        if action == FilterAction::Suppress {
            self.stats.total_suppressed += 1;
            return None;
        }

        if action == FilterAction::Alert || action == FilterAction::LogAndAlert {
            self.stats.total_alerts += 1;
            if let Some(proc_state) = self.processes.get_mut(&record.pid) {
                proc_state.alerts += 1;
            }
        }

        // Update process state
        if let Some(proc_state) = self.processes.get_mut(&record.pid) {
            proc_state.record(record.category, record.timestamp_ns);
        }

        let seq = record.seq;
        if self.records.len() >= self.max_records {
            self.records.remove(0);
            self.stats.buffer_overflows += 1;
        }
        self.records.push_back(record);
        self.stats.total_records += 1;
        Some(seq)
    }

    #[inline(always)]
    pub fn query_by_pid(&self, pid: u64, max: usize) -> Vec<&AuditRecord> {
        self.records.iter().filter(|r| r.pid == pid).rev().take(max).collect()
    }

    #[inline(always)]
    pub fn query_by_category(&self, cat: AuditCategory, max: usize) -> Vec<&AuditRecord> {
        self.records.iter().filter(|r| r.category == cat).rev().take(max).collect()
    }

    #[inline(always)]
    pub fn query_by_severity(&self, min_sev: AuditSeverity, max: usize) -> Vec<&AuditRecord> {
        self.records.iter().filter(|r| r.severity >= min_sev).rev().take(max).collect()
    }

    #[inline(always)]
    pub fn stats(&self) -> &AuditTrailStats {
        &self.stats
    }
}
