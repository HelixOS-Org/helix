//! # Bridge Audit Engine
//!
//! Syscall audit logging and compliance:
//! - Comprehensive audit trail
//! - Per-process audit policies
//! - Audit log integrity
//! - Compliance rules
//! - Audit event aggregation
//! - Real-time audit alerts

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// AUDIT EVENT
// ============================================================================

/// Audit event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditEventType {
    /// Syscall execution
    SyscallExec,
    /// Syscall denied
    SyscallDenied,
    /// File access
    FileAccess,
    /// Network operation
    NetworkOp,
    /// Process creation
    ProcessCreate,
    /// Process exit
    ProcessExit,
    /// Privilege change
    PrivilegeChange,
    /// Security violation
    SecurityViolation,
    /// Resource limit hit
    ResourceLimit,
    /// IPC operation
    IpcOperation,
}

/// Audit severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditSeverity {
    /// Debug (verbose)
    Debug,
    /// Info (routine)
    Info,
    /// Notice (notable)
    Notice,
    /// Warning (potential issue)
    Warning,
    /// Alert (immediate attention)
    Alert,
    /// Critical (security breach)
    Critical,
}

/// Audit event
#[derive(Debug, Clone)]
pub struct AuditEvent {
    /// Event ID (monotonic)
    pub id: u64,
    /// Event type
    pub event_type: AuditEventType,
    /// Severity
    pub severity: AuditSeverity,
    /// Timestamp (ns)
    pub timestamp: u64,
    /// Process ID
    pub pid: u64,
    /// Thread ID
    pub tid: u64,
    /// User ID
    pub uid: u32,
    /// Syscall number (if applicable)
    pub syscall_nr: Option<u32>,
    /// Result code
    pub result: i64,
    /// Extra data fields
    pub fields: BTreeMap<u32, u64>,
    /// Hash for integrity
    pub hash: u64,
}

impl AuditEvent {
    pub fn new(
        id: u64,
        event_type: AuditEventType,
        severity: AuditSeverity,
        timestamp: u64,
        pid: u64,
    ) -> Self {
        Self {
            id,
            event_type,
            severity,
            timestamp,
            pid,
            tid: 0,
            uid: 0,
            syscall_nr: None,
            result: 0,
            fields: BTreeMap::new(),
            hash: 0,
        }
    }

    pub fn with_syscall(mut self, nr: u32) -> Self {
        self.syscall_nr = Some(nr);
        self
    }

    pub fn with_result(mut self, result: i64) -> Self {
        self.result = result;
        self
    }

    pub fn with_uid(mut self, uid: u32) -> Self {
        self.uid = uid;
        self
    }

    pub fn add_field(&mut self, key: u32, value: u64) {
        self.fields.insert(key, value);
    }

    /// Compute integrity hash
    pub fn compute_hash(&mut self) {
        let mut h: u64 = 0xcbf29ce484222325;
        h ^= self.id;
        h = h.wrapping_mul(0x100000001b3);
        h ^= self.event_type as u64;
        h = h.wrapping_mul(0x100000001b3);
        h ^= self.timestamp;
        h = h.wrapping_mul(0x100000001b3);
        h ^= self.pid;
        h = h.wrapping_mul(0x100000001b3);
        h ^= self.result as u64;
        h = h.wrapping_mul(0x100000001b3);
        self.hash = h;
    }

    /// Verify integrity
    pub fn verify_hash(&self) -> bool {
        let mut h: u64 = 0xcbf29ce484222325;
        h ^= self.id;
        h = h.wrapping_mul(0x100000001b3);
        h ^= self.event_type as u64;
        h = h.wrapping_mul(0x100000001b3);
        h ^= self.timestamp;
        h = h.wrapping_mul(0x100000001b3);
        h ^= self.pid;
        h = h.wrapping_mul(0x100000001b3);
        h ^= self.result as u64;
        h = h.wrapping_mul(0x100000001b3);
        h == self.hash
    }
}

// ============================================================================
// AUDIT POLICY
// ============================================================================

/// Audit rule match
#[derive(Debug, Clone)]
pub struct AuditRule {
    /// Rule ID
    pub id: u64,
    /// Match event type (None = all)
    pub event_type: Option<AuditEventType>,
    /// Match syscall number (None = all)
    pub syscall_nr: Option<u32>,
    /// Match PID (0 = all)
    pub pid: u64,
    /// Match UID (u32::MAX = all)
    pub uid: u32,
    /// Minimum severity to log
    pub min_severity: AuditSeverity,
    /// Enabled
    pub enabled: bool,
    /// Hits
    pub hits: u64,
}

impl AuditRule {
    pub fn new(id: u64, min_severity: AuditSeverity) -> Self {
        Self {
            id,
            event_type: None,
            syscall_nr: None,
            pid: 0,
            uid: u32::MAX,
            min_severity,
            enabled: true,
            hits: 0,
        }
    }

    pub fn for_event_type(mut self, event_type: AuditEventType) -> Self {
        self.event_type = Some(event_type);
        self
    }

    pub fn for_pid(mut self, pid: u64) -> Self {
        self.pid = pid;
        self
    }

    pub fn for_uid(mut self, uid: u32) -> Self {
        self.uid = uid;
        self
    }

    /// Check if event matches
    pub fn matches(&self, event: &AuditEvent) -> bool {
        if !self.enabled {
            return false;
        }
        if event.severity < self.min_severity {
            return false;
        }
        if let Some(et) = self.event_type {
            if event.event_type != et {
                return false;
            }
        }
        if let Some(nr) = self.syscall_nr {
            if event.syscall_nr != Some(nr) {
                return false;
            }
        }
        if self.pid != 0 && event.pid != self.pid {
            return false;
        }
        if self.uid != u32::MAX && event.uid != self.uid {
            return false;
        }
        true
    }
}

// ============================================================================
// AUDIT ALERT
// ============================================================================

/// Alert condition
#[derive(Debug, Clone)]
pub struct AlertCondition {
    /// Condition ID
    pub id: u64,
    /// Event type to watch
    pub event_type: AuditEventType,
    /// Minimum severity
    pub min_severity: AuditSeverity,
    /// Threshold count in window
    pub threshold: u64,
    /// Window size (ns)
    pub window_ns: u64,
    /// Current count
    pub current_count: u64,
    /// Window start
    pub window_start: u64,
    /// Alert triggered
    pub triggered: bool,
}

impl AlertCondition {
    pub fn new(
        id: u64,
        event_type: AuditEventType,
        min_severity: AuditSeverity,
        threshold: u64,
        window_ns: u64,
    ) -> Self {
        Self {
            id,
            event_type,
            min_severity,
            threshold,
            window_ns,
            current_count: 0,
            window_start: 0,
            triggered: false,
        }
    }

    /// Check event
    pub fn check(&mut self, event: &AuditEvent) -> bool {
        if event.event_type != self.event_type || event.severity < self.min_severity {
            return false;
        }

        // Reset window if expired
        if event.timestamp.saturating_sub(self.window_start) > self.window_ns {
            self.window_start = event.timestamp;
            self.current_count = 0;
            self.triggered = false;
        }

        self.current_count += 1;
        if self.current_count >= self.threshold && !self.triggered {
            self.triggered = true;
            return true;
        }
        false
    }
}

// ============================================================================
// AUDIT MANAGER
// ============================================================================

/// Audit manager stats
#[derive(Debug, Clone, Default)]
pub struct AuditManagerStats {
    /// Total events logged
    pub total_events: u64,
    /// Events by severity
    pub events_by_severity: BTreeMap<u8, u64>,
    /// Active rules
    pub active_rules: usize,
    /// Alerts triggered
    pub alerts_triggered: u64,
    /// Log integrity errors
    pub integrity_errors: u64,
}

/// Bridge audit manager
pub struct BridgeAuditManager {
    /// Audit log (ring buffer)
    log: Vec<AuditEvent>,
    /// Max log size
    max_log: usize,
    /// Rules
    rules: Vec<AuditRule>,
    /// Alert conditions
    alerts: Vec<AlertCondition>,
    /// Next event ID
    next_id: u64,
    /// Chain hash (for log integrity)
    chain_hash: u64,
    /// Stats
    stats: AuditManagerStats,
}

impl BridgeAuditManager {
    pub fn new() -> Self {
        Self {
            log: Vec::new(),
            max_log: 10_000,
            rules: Vec::new(),
            alerts: Vec::new(),
            next_id: 1,
            chain_hash: 0,
            stats: AuditManagerStats::default(),
        }
    }

    /// Add rule
    pub fn add_rule(&mut self, rule: AuditRule) {
        self.rules.push(rule);
        self.stats.active_rules = self.rules.iter().filter(|r| r.enabled).count();
    }

    /// Add alert condition
    pub fn add_alert(&mut self, alert: AlertCondition) {
        self.alerts.push(alert);
    }

    /// Log event
    pub fn log_event(&mut self, mut event: AuditEvent) -> bool {
        // Check rules
        let mut should_log = false;
        for rule in &mut self.rules {
            if rule.matches(&event) {
                rule.hits += 1;
                should_log = true;
                break;
            }
        }

        if !should_log && self.rules.is_empty() {
            should_log = true; // Log everything if no rules
        }

        if should_log {
            event.id = self.next_id;
            self.next_id += 1;
            event.compute_hash();

            // Chain hash
            self.chain_hash ^= event.hash;
            self.chain_hash = self.chain_hash.wrapping_mul(0x100000001b3);

            // Check alerts
            for alert in &mut self.alerts {
                if alert.check(&event) {
                    self.stats.alerts_triggered += 1;
                }
            }

            // Update stats
            *self
                .stats
                .events_by_severity
                .entry(event.severity as u8)
                .or_insert(0) += 1;
            self.stats.total_events += 1;

            // Store
            self.log.push(event);
            if self.log.len() > self.max_log {
                self.log.remove(0);
            }
        }

        should_log
    }

    /// Verify log integrity
    pub fn verify_integrity(&mut self) -> bool {
        for event in &self.log {
            if !event.verify_hash() {
                self.stats.integrity_errors += 1;
                return false;
            }
        }
        true
    }

    /// Query events by type
    pub fn query_by_type(&self, event_type: AuditEventType) -> Vec<&AuditEvent> {
        self.log
            .iter()
            .filter(|e| e.event_type == event_type)
            .collect()
    }

    /// Query events by severity
    pub fn query_by_severity(&self, min_severity: AuditSeverity) -> Vec<&AuditEvent> {
        self.log
            .iter()
            .filter(|e| e.severity >= min_severity)
            .collect()
    }

    /// Stats
    pub fn stats(&self) -> &AuditManagerStats {
        &self.stats
    }
}
