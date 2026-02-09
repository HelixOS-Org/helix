//! Isolation Analyzer
//!
//! Security boundary analysis for virtualized workloads.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::VirtId;
use crate::core::NexusTimestamp;

/// Analyzes isolation boundaries
pub struct IsolationAnalyzer {
    /// Isolation violations
    violations: Vec<IsolationViolation>,
    /// Security boundaries
    boundaries: BTreeMap<VirtId, SecurityBoundary>,
    /// Escape attempts
    escape_attempts: Vec<EscapeAttempt>,
}

/// Isolation violation
#[derive(Debug, Clone)]
pub struct IsolationViolation {
    /// Workload ID
    pub workload_id: VirtId,
    /// Violation type
    pub violation_type: ViolationType,
    /// Severity
    pub severity: ViolationSeverity,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Details
    pub details: String,
}

/// Violation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationType {
    /// Cross-namespace access
    CrossNamespace,
    /// Privilege escalation
    PrivilegeEscalation,
    /// Resource escape
    ResourceEscape,
    /// Network violation
    NetworkViolation,
    /// Filesystem violation
    FilesystemViolation,
}

/// Violation severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationSeverity {
    /// Low
    Low      = 0,
    /// Medium
    Medium   = 1,
    /// High
    High     = 2,
    /// Critical
    Critical = 3,
}

/// Security boundary
#[derive(Debug, Clone)]
pub struct SecurityBoundary {
    /// Workload ID
    pub workload_id: VirtId,
    /// Capabilities
    pub capabilities: Vec<String>,
    /// Seccomp enabled
    pub seccomp: bool,
    /// AppArmor/SELinux profile
    pub mac_profile: Option<String>,
    /// Read-only root
    pub readonly_root: bool,
    /// No new privileges
    pub no_new_privs: bool,
}

/// Escape attempt
#[derive(Debug, Clone)]
pub struct EscapeAttempt {
    /// Workload ID
    pub workload_id: VirtId,
    /// Attempt type
    pub attempt_type: EscapeType,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Was blocked
    pub blocked: bool,
}

/// Escape type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeType {
    /// Kernel exploit
    KernelExploit,
    /// Capability abuse
    CapabilityAbuse,
    /// Namespace escape
    NamespaceEscape,
    /// Device access
    DeviceAccess,
    /// Syscall abuse
    SyscallAbuse,
}

impl IsolationAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            violations: Vec::new(),
            boundaries: BTreeMap::new(),
            escape_attempts: Vec::new(),
        }
    }

    /// Record violation
    #[inline(always)]
    pub fn record_violation(&mut self, violation: IsolationViolation) {
        self.violations.push(violation);
    }

    /// Set security boundary
    #[inline(always)]
    pub fn set_boundary(&mut self, boundary: SecurityBoundary) {
        self.boundaries.insert(boundary.workload_id, boundary);
    }

    /// Record escape attempt
    #[inline(always)]
    pub fn record_escape(&mut self, attempt: EscapeAttempt) {
        self.escape_attempts.push(attempt);
    }

    /// Get violations for workload
    #[inline]
    pub fn get_violations(&self, workload_id: VirtId) -> Vec<&IsolationViolation> {
        self.violations
            .iter()
            .filter(|v| v.workload_id == workload_id)
            .collect()
    }

    /// Get boundary
    #[inline(always)]
    pub fn get_boundary(&self, workload_id: VirtId) -> Option<&SecurityBoundary> {
        self.boundaries.get(&workload_id)
    }

    /// Get critical violations
    #[inline]
    pub fn critical_violations(&self) -> Vec<&IsolationViolation> {
        self.violations
            .iter()
            .filter(|v| v.severity >= ViolationSeverity::Critical)
            .collect()
    }

    /// Get blocked escape count
    #[inline(always)]
    pub fn blocked_escapes(&self) -> usize {
        self.escape_attempts.iter().filter(|e| e.blocked).count()
    }

    /// Get escape attempts for workload
    #[inline]
    pub fn get_escape_attempts(&self, workload_id: VirtId) -> Vec<&EscapeAttempt> {
        self.escape_attempts
            .iter()
            .filter(|e| e.workload_id == workload_id)
            .collect()
    }

    /// Total violations
    #[inline(always)]
    pub fn total_violations(&self) -> usize {
        self.violations.len()
    }

    /// Total escape attempts
    #[inline(always)]
    pub fn total_escape_attempts(&self) -> usize {
        self.escape_attempts.len()
    }
}

impl Default for IsolationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
