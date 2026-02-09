//! Anomaly Detection
//!
//! Security anomaly detection and baseline statistics.

use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use super::{AuditEvent, AuditEventId, AuditMessageType, AuditResult, Pid, Uid};

/// Anomaly type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalyType {
    /// Privilege escalation
    PrivilegeEscalation,
    /// Unusual syscall sequence
    UnusualSyscalls,
    /// Brute force attempt
    BruteForce,
    /// Data exfiltration
    DataExfiltration,
    /// Unusual process
    UnusualProcess,
    /// Time-based anomaly
    TimeAnomaly,
    /// Path traversal
    PathTraversal,
    /// Resource abuse
    ResourceAbuse,
}

impl AnomalyType {
    /// Get anomaly name
    pub fn name(&self) -> &'static str {
        match self {
            Self::PrivilegeEscalation => "privilege_escalation",
            Self::UnusualSyscalls => "unusual_syscalls",
            Self::BruteForce => "brute_force",
            Self::DataExfiltration => "data_exfiltration",
            Self::UnusualProcess => "unusual_process",
            Self::TimeAnomaly => "time_anomaly",
            Self::PathTraversal => "path_traversal",
            Self::ResourceAbuse => "resource_abuse",
        }
    }
}

/// Detected anomaly
#[derive(Debug, Clone)]
pub struct Anomaly {
    /// Anomaly type
    pub anomaly_type: AnomalyType,
    /// Confidence score (0-100)
    pub confidence: f32,
    /// Related events
    pub events: Vec<AuditEventId>,
    /// Description
    pub description: String,
    /// Detection timestamp
    pub detected_at: u64,
    /// Severity (1-10)
    pub severity: u8,
    /// User involved
    pub user: Option<Uid>,
    /// Process involved
    pub process: Option<Pid>,
}

impl Anomaly {
    /// Create new anomaly
    pub fn new(
        anomaly_type: AnomalyType,
        confidence: f32,
        description: String,
        timestamp: u64,
    ) -> Self {
        Self {
            anomaly_type,
            confidence,
            events: Vec::new(),
            description,
            detected_at: timestamp,
            severity: 5,
            user: None,
            process: None,
        }
    }

    /// Set severity
    #[inline(always)]
    pub fn with_severity(mut self, severity: u8) -> Self {
        self.severity = severity;
        self
    }

    /// Set user
    #[inline(always)]
    pub fn with_user(mut self, user: Uid) -> Self {
        self.user = Some(user);
        self
    }

    /// Set process
    #[inline(always)]
    pub fn with_process(mut self, process: Pid) -> Self {
        self.process = Some(process);
        self
    }

    /// Add related event
    #[inline(always)]
    pub fn add_event(&mut self, event_id: AuditEventId) {
        self.events.push(event_id);
    }
}

/// Baseline statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BaselineStats {
    /// Syscall frequency
    pub syscall_freq: ArrayMap<u64, 32>,
    /// User activity
    pub user_activity: ArrayMap<u64, 32>,
    /// Failed auth attempts
    pub failed_auth: ArrayMap<u64, 32>,
    /// Process launches
    pub process_launches: u64,
    /// File access count
    pub file_accesses: u64,
    /// Network connections
    pub network_connections: u64,
    /// Period start
    pub period_start: u64,
    /// Period end
    pub period_end: u64,
}

/// Anomaly detector
pub struct AnomalyDetector {
    /// Baseline statistics
    baseline: BaselineStats,
    /// Current statistics
    current: BaselineStats,
    /// Detected anomalies
    anomalies: VecDeque<Anomaly>,
    /// Max anomalies to track
    max_anomalies: usize,
    /// Thresholds
    priv_esc_threshold: f32,
    brute_force_threshold: u32,
    /// Enabled
    enabled: AtomicBool,
}

impl AnomalyDetector {
    /// Create new anomaly detector
    pub fn new() -> Self {
        Self {
            baseline: BaselineStats::default(),
            current: BaselineStats::default(),
            anomalies: VecDeque::new(),
            max_anomalies: 1000,
            priv_esc_threshold: 0.8,
            brute_force_threshold: 10,
            enabled: AtomicBool::new(true),
        }
    }

    /// Process event
    pub fn process_event(&mut self, event: &AuditEvent, timestamp: u64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        // Update current stats
        if let Some(ref syscall) = event.syscall {
            *self
                .current
                .syscall_freq
                .entry(syscall.syscall.raw())
                .or_insert(0) += 1;
        }

        *self
            .current
            .user_activity
            .entry(event.process.uid.raw())
            .or_insert(0) += 1;

        // Check for privilege escalation
        if event.process.has_escalation() {
            self.detect_privilege_escalation(event, timestamp);
        }

        // Check for brute force
        if matches!(event.result, AuditResult::Failure) {
            if matches!(
                event.msg_type,
                AuditMessageType::UserAuth | AuditMessageType::UserLogin
            ) {
                self.detect_brute_force(event, timestamp);
            }
        }
    }

    /// Detect privilege escalation
    fn detect_privilege_escalation(&mut self, event: &AuditEvent, timestamp: u64) {
        let mut anomaly = Anomaly::new(
            AnomalyType::PrivilegeEscalation,
            90.0,
            alloc::format!(
                "User {} escalated to root via process {}",
                event.process.uid.raw(),
                event.process.pid.raw()
            ),
            timestamp,
        );
        anomaly.severity = 9;
        anomaly.user = Some(event.process.uid);
        anomaly.process = Some(event.process.pid);
        anomaly.events.push(event.id);

        self.add_anomaly(anomaly);
    }

    /// Detect brute force
    fn detect_brute_force(&mut self, event: &AuditEvent, timestamp: u64) {
        let uid = event.process.uid.raw();
        let count = self.current.failed_auth.entry(uid).or_insert(0);
        *count += 1;

        if *count >= self.brute_force_threshold as u64 {
            let mut anomaly = Anomaly::new(
                AnomalyType::BruteForce,
                85.0,
                alloc::format!("User {} has {} failed auth attempts", uid, count),
                timestamp,
            );
            anomaly.severity = 7;
            anomaly.user = Some(event.process.uid);
            anomaly.events.push(event.id);

            self.add_anomaly(anomaly);
        }
    }

    /// Add anomaly
    fn add_anomaly(&mut self, anomaly: Anomaly) {
        if self.anomalies.len() >= self.max_anomalies {
            self.anomalies.pop_front();
        }
        self.anomalies.push_back(anomaly);
    }

    /// Get recent anomalies
    #[inline(always)]
    pub fn recent_anomalies(&self, count: usize) -> &[Anomaly] {
        let start = self.anomalies.len().saturating_sub(count);
        &self.anomalies[start..]
    }

    /// Get all anomalies
    #[inline(always)]
    pub fn all_anomalies(&self) -> &[Anomaly] {
        &self.anomalies
    }

    /// Get anomalies by type
    #[inline]
    pub fn get_by_type(&self, anomaly_type: AnomalyType) -> Vec<&Anomaly> {
        self.anomalies
            .iter()
            .filter(|a| a.anomaly_type == anomaly_type)
            .collect()
    }

    /// Total anomalies
    #[inline(always)]
    pub fn total_anomalies(&self) -> usize {
        self.anomalies.len()
    }

    /// Enable/disable
    #[inline(always)]
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Is enabled
    #[inline(always)]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Set brute force threshold
    #[inline(always)]
    pub fn set_brute_force_threshold(&mut self, threshold: u32) {
        self.brute_force_threshold = threshold;
    }

    /// Get baseline stats
    #[inline(always)]
    pub fn baseline(&self) -> &BaselineStats {
        &self.baseline
    }

    /// Get current stats
    #[inline(always)]
    pub fn current(&self) -> &BaselineStats {
        &self.current
    }
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self::new()
    }
}
