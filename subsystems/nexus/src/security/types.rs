//! Core security types and threat definitions.

use alloc::collections::BTreeMap;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::core::NexusTimestamp;

// ============================================================================
// THREAT TYPES
// ============================================================================

/// Security threat severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreatSeverity {
    /// Informational - logging only
    Info     = 0,
    /// Low severity
    Low      = 1,
    /// Medium severity
    Medium   = 2,
    /// High severity
    High     = 3,
    /// Critical severity
    Critical = 4,
}

impl ThreatSeverity {
    /// Should block operation?
    pub fn should_block(&self) -> bool {
        matches!(self, Self::High | Self::Critical)
    }

    /// Get display name
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Critical => "Critical",
        }
    }
}

/// Type of security threat
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreatType {
    /// Buffer overflow attempt
    BufferOverflow,
    /// Privilege escalation
    PrivilegeEscalation,
    /// Unauthorized memory access
    UnauthorizedMemoryAccess,
    /// Code injection attempt
    CodeInjection,
    /// Return-oriented programming
    Rop,
    /// Suspicious syscall pattern
    SuspiciousSyscall,
    /// Brute force attempt
    BruteForce,
    /// Denial of service
    DoS,
    /// Data exfiltration
    DataExfiltration,
    /// Kernel module tampering
    ModuleTampering,
    /// Process injection
    ProcessInjection,
    /// Credential theft
    CredentialTheft,
    /// Network anomaly
    NetworkAnomaly,
    /// File system anomaly
    FileSystemAnomaly,
    /// Unknown threat
    Unknown,
}

impl ThreatType {
    /// Get default severity
    pub fn default_severity(&self) -> ThreatSeverity {
        match self {
            Self::BufferOverflow => ThreatSeverity::Critical,
            Self::PrivilegeEscalation => ThreatSeverity::Critical,
            Self::UnauthorizedMemoryAccess => ThreatSeverity::High,
            Self::CodeInjection => ThreatSeverity::Critical,
            Self::Rop => ThreatSeverity::Critical,
            Self::SuspiciousSyscall => ThreatSeverity::Medium,
            Self::BruteForce => ThreatSeverity::Medium,
            Self::DoS => ThreatSeverity::High,
            Self::DataExfiltration => ThreatSeverity::High,
            Self::ModuleTampering => ThreatSeverity::Critical,
            Self::ProcessInjection => ThreatSeverity::High,
            Self::CredentialTheft => ThreatSeverity::High,
            Self::NetworkAnomaly => ThreatSeverity::Medium,
            Self::FileSystemAnomaly => ThreatSeverity::Medium,
            Self::Unknown => ThreatSeverity::Low,
        }
    }

    /// Get category
    pub fn category(&self) -> &'static str {
        match self {
            Self::BufferOverflow | Self::CodeInjection | Self::Rop => "Memory",
            Self::PrivilegeEscalation | Self::CredentialTheft => "Access",
            Self::SuspiciousSyscall | Self::ProcessInjection => "Process",
            Self::NetworkAnomaly | Self::DataExfiltration => "Network",
            Self::FileSystemAnomaly | Self::ModuleTampering => "FileSystem",
            Self::BruteForce | Self::DoS => "Availability",
            Self::UnauthorizedMemoryAccess => "Memory",
            Self::Unknown => "Unknown",
        }
    }
}

/// Security threat record
#[derive(Debug, Clone)]
pub struct Threat {
    /// Unique ID
    pub id: u64,
    /// Threat type
    pub threat_type: ThreatType,
    /// Severity
    pub severity: ThreatSeverity,
    /// Source (process/task ID)
    pub source_id: u64,
    /// Target (if applicable)
    pub target_id: Option<u64>,
    /// Description
    pub description: String,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Is confirmed
    pub confirmed: bool,
    /// Mitigation applied
    pub mitigated: bool,
    /// Additional context
    pub context: BTreeMap<String, String>,
}

impl Threat {
    /// Create new threat
    pub fn new(threat_type: ThreatType, source_id: u64) -> Self {
        static THREAT_ID: AtomicU64 = AtomicU64::new(1);

        Self {
            id: THREAT_ID.fetch_add(1, Ordering::Relaxed),
            threat_type,
            severity: threat_type.default_severity(),
            source_id,
            target_id: None,
            description: String::new(),
            timestamp: NexusTimestamp::now(),
            confirmed: false,
            mitigated: false,
            context: BTreeMap::new(),
        }
    }

    /// Set severity
    pub fn with_severity(mut self, severity: ThreatSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Set target
    pub fn with_target(mut self, target: u64) -> Self {
        self.target_id = Some(target);
        self
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add context
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Mark as confirmed
    pub fn confirm(&mut self) {
        self.confirmed = true;
    }

    /// Mark as mitigated
    pub fn mitigate(&mut self) {
        self.mitigated = true;
    }
}
