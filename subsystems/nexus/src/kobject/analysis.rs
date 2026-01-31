//! Kobject Analysis
//!
//! Analysis results and recommendations.

use alloc::string::String;
use alloc::vec::Vec;

use super::KobjectId;

/// Kobject analysis result
#[derive(Debug, Clone)]
pub struct KobjectAnalysis {
    /// Kobject ID
    pub kobject_id: KobjectId,
    /// Health score (0-100)
    pub health_score: f32,
    /// Issues detected
    pub issues: Vec<KobjectIssue>,
    /// Recommendations
    pub recommendations: Vec<KobjectRecommendation>,
}

/// Kobject issue
#[derive(Debug, Clone)]
pub struct KobjectIssue {
    /// Issue type
    pub issue_type: KobjectIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
}

/// Kobject issue types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KobjectIssueType {
    /// Reference leak
    RefLeak,
    /// Reference underflow
    RefUnderflow,
    /// Missing release function
    MissingRelease,
    /// Orphaned object
    Orphaned,
    /// Sysfs registration failed
    SysfsRegFailed,
    /// Long-lived object
    LongLived,
}

impl KobjectIssueType {
    /// Get issue type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::RefLeak => "ref_leak",
            Self::RefUnderflow => "ref_underflow",
            Self::MissingRelease => "missing_release",
            Self::Orphaned => "orphaned",
            Self::SysfsRegFailed => "sysfs_reg_failed",
            Self::LongLived => "long_lived",
        }
    }
}

/// Kobject recommendation
#[derive(Debug, Clone)]
pub struct KobjectRecommendation {
    /// Action
    pub action: KobjectAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// Kobject actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KobjectAction {
    /// Fix reference counting
    FixRefcount,
    /// Add release function
    AddRelease,
    /// Cleanup orphan
    CleanupOrphan,
    /// Force unregister
    ForceUnregister,
}

impl KobjectAction {
    /// Get action name
    pub fn name(&self) -> &'static str {
        match self {
            Self::FixRefcount => "fix_refcount",
            Self::AddRelease => "add_release",
            Self::CleanupOrphan => "cleanup_orphan",
            Self::ForceUnregister => "force_unregister",
        }
    }
}
