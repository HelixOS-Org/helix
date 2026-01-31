//! Capability Analysis
//!
//! Security analysis and recommendations.

use alloc::string::String;
use alloc::vec::Vec;

use super::{Capability, Pid};

/// Capability analysis
#[derive(Debug, Clone)]
pub struct CapabilityAnalysis {
    /// Security score (0-100)
    pub security_score: f32,
    /// Risk score (0-100)
    pub risk_score: f32,
    /// Privileged processes
    pub privileged_count: usize,
    /// Issues detected
    pub issues: Vec<CapabilityIssue>,
    /// Recommendations
    pub recommendations: Vec<CapabilityRecommendation>,
}

/// Capability issue
#[derive(Debug, Clone)]
pub struct CapabilityIssue {
    /// Issue type
    pub issue_type: CapIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
    /// Process
    pub pid: Option<Pid>,
    /// Capability
    pub capability: Option<Capability>,
}

/// Capability issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapIssueType {
    /// Excessive privileges
    ExcessivePrivileges,
    /// Critical capability
    CriticalCapability,
    /// Unused capability
    UnusedCapability,
    /// Privilege escalation
    PrivilegeEscalation,
    /// Missing no_new_privs
    MissingNoNewPrivs,
    /// Ambient capabilities
    AmbientCapabilities,
}

impl CapIssueType {
    /// Get issue type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::ExcessivePrivileges => "excessive_privileges",
            Self::CriticalCapability => "critical_capability",
            Self::UnusedCapability => "unused_capability",
            Self::PrivilegeEscalation => "privilege_escalation",
            Self::MissingNoNewPrivs => "missing_no_new_privs",
            Self::AmbientCapabilities => "ambient_capabilities",
        }
    }
}

/// Capability recommendation
#[derive(Debug, Clone)]
pub struct CapabilityRecommendation {
    /// Action
    pub action: CapAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
    /// Target process
    pub pid: Option<Pid>,
    /// Target capability
    pub capability: Option<Capability>,
}

/// Capability action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapAction {
    /// Drop capability
    DropCap,
    /// Set no_new_privs
    SetNoNewPrivs,
    /// Drop bounding set
    DropBounding,
    /// Clear ambient
    ClearAmbient,
    /// Apply template
    ApplyTemplate,
}

impl CapAction {
    /// Get action name
    pub fn name(&self) -> &'static str {
        match self {
            Self::DropCap => "drop_cap",
            Self::SetNoNewPrivs => "set_no_new_privs",
            Self::DropBounding => "drop_bounding",
            Self::ClearAmbient => "clear_ambient",
            Self::ApplyTemplate => "apply_template",
        }
    }
}
