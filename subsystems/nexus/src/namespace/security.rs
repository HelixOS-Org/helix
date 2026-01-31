//! Security Boundary Enforcer
//!
//! Enforcing namespace security boundaries.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{NamespaceId, ProcessId, UserId};

/// Security boundary violation
#[derive(Debug, Clone)]
pub struct BoundaryViolation {
    /// Violation type
    pub violation_type: ViolationType,
    /// Source process
    pub source_pid: ProcessId,
    /// Target (process or resource)
    pub target: String,
    /// Timestamp
    pub timestamp: u64,
    /// Blocked
    pub blocked: bool,
    /// Details
    pub details: String,
}

/// Violation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationType {
    /// Cross-namespace signal
    CrossNsSignal,
    /// Cross-namespace ptrace
    CrossNsPtrace,
    /// Cross-namespace file access
    CrossNsFileAccess,
    /// Privilege escalation attempt
    PrivilegeEscalation,
    /// Capability violation
    CapabilityViolation,
    /// User mapping violation
    UserMappingViolation,
}

/// Security boundary enforcer
pub struct SecurityEnforcer {
    /// Violations history
    violations: Vec<BoundaryViolation>,
    /// Maximum violations to track
    max_violations: usize,
    /// Total violations
    total_violations: AtomicU64,
    /// Blocked violations
    blocked_violations: AtomicU64,
    /// Strict mode
    strict_mode: bool,
}

impl SecurityEnforcer {
    /// Create new security enforcer
    pub fn new() -> Self {
        Self {
            violations: Vec::with_capacity(1000),
            max_violations: 1000,
            total_violations: AtomicU64::new(0),
            blocked_violations: AtomicU64::new(0),
            strict_mode: false,
        }
    }

    /// Check cross-namespace signal
    pub fn check_signal(
        &mut self,
        source_pid: ProcessId,
        target_pid: ProcessId,
        source_ns: NamespaceId,
        target_ns: NamespaceId,
        timestamp: u64,
    ) -> bool {
        if source_ns == target_ns {
            return true; // Same namespace, allowed
        }

        let blocked = self.strict_mode;
        self.record_violation(
            ViolationType::CrossNsSignal,
            source_pid,
            format!("PID {}", target_pid.raw()),
            timestamp,
            blocked,
            String::from("Cross-namespace signal attempt"),
        );

        !blocked
    }

    /// Check cross-namespace ptrace
    pub fn check_ptrace(
        &mut self,
        tracer: ProcessId,
        tracee: ProcessId,
        tracer_user_ns: NamespaceId,
        tracee_user_ns: NamespaceId,
        timestamp: u64,
    ) -> bool {
        if tracer_user_ns == tracee_user_ns {
            return true;
        }

        // Cross-user-namespace ptrace is more restricted
        let blocked = true;
        self.record_violation(
            ViolationType::CrossNsPtrace,
            tracer,
            format!("PID {}", tracee.raw()),
            timestamp,
            blocked,
            String::from("Cross-user-namespace ptrace blocked"),
        );

        !blocked
    }

    /// Check privilege escalation
    pub fn check_privilege_escalation(
        &mut self,
        pid: ProcessId,
        requested_uid: UserId,
        current_uid: UserId,
        timestamp: u64,
    ) -> bool {
        if requested_uid.raw() >= current_uid.raw() {
            return true; // Not escalation
        }

        // Escalating to lower UID (more privileged)
        let blocked = self.strict_mode && current_uid != UserId::ROOT;
        self.record_violation(
            ViolationType::PrivilegeEscalation,
            pid,
            format!("UID {} -> {}", current_uid.raw(), requested_uid.raw()),
            timestamp,
            blocked,
            String::from("Privilege escalation attempt"),
        );

        !blocked
    }

    /// Record violation
    fn record_violation(
        &mut self,
        violation_type: ViolationType,
        source_pid: ProcessId,
        target: String,
        timestamp: u64,
        blocked: bool,
        details: String,
    ) {
        self.total_violations.fetch_add(1, Ordering::Relaxed);
        if blocked {
            self.blocked_violations.fetch_add(1, Ordering::Relaxed);
        }

        let violation = BoundaryViolation {
            violation_type,
            source_pid,
            target,
            timestamp,
            blocked,
            details,
        };

        if self.violations.len() >= self.max_violations {
            self.violations.remove(0);
        }
        self.violations.push(violation);
    }

    /// Get recent violations
    pub fn recent_violations(&self, limit: usize) -> &[BoundaryViolation] {
        let start = self.violations.len().saturating_sub(limit);
        &self.violations[start..]
    }

    /// Get total violations
    pub fn total_violations(&self) -> u64 {
        self.total_violations.load(Ordering::Relaxed)
    }

    /// Get blocked violations
    pub fn blocked_violations(&self) -> u64 {
        self.blocked_violations.load(Ordering::Relaxed)
    }

    /// Set strict mode
    pub fn set_strict_mode(&mut self, strict: bool) {
        self.strict_mode = strict;
    }

    /// Check if strict mode
    pub fn is_strict_mode(&self) -> bool {
        self.strict_mode
    }
}

impl Default for SecurityEnforcer {
    fn default() -> Self {
        Self::new()
    }
}
