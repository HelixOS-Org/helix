//! Namespace Analysis
//!
//! Analysis results and recommendations.

use alloc::string::String;
use alloc::vec::Vec;

use super::{IsolationAnalysis, NamespaceId};

/// Namespace analysis result
#[derive(Debug, Clone)]
pub struct NamespaceAnalysis {
    /// Namespace ID
    pub ns_id: NamespaceId,
    /// Health score (0-100)
    pub health_score: f32,
    /// Issues detected
    pub issues: Vec<NamespaceIssue>,
    /// Recommendations
    pub recommendations: Vec<NamespaceRecommendation>,
    /// Isolation analysis
    pub isolation: Option<IsolationAnalysis>,
}

/// Namespace issue
#[derive(Debug, Clone)]
pub struct NamespaceIssue {
    /// Issue type
    pub issue_type: NamespaceIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
}

/// Namespace issue types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceIssueType {
    /// Empty namespace
    EmptyNamespace,
    /// Orphaned namespace
    OrphanedNamespace,
    /// Missing user mapping
    MissingUserMapping,
    /// Security violation
    SecurityViolation,
    /// Too many processes
    TooManyProcesses,
    /// Deep hierarchy
    DeepHierarchy,
}

impl NamespaceIssueType {
    /// Get issue type name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::EmptyNamespace => "empty_namespace",
            Self::OrphanedNamespace => "orphaned_namespace",
            Self::MissingUserMapping => "missing_user_mapping",
            Self::SecurityViolation => "security_violation",
            Self::TooManyProcesses => "too_many_processes",
            Self::DeepHierarchy => "deep_hierarchy",
        }
    }
}

/// Namespace recommendation
#[derive(Debug, Clone)]
pub struct NamespaceRecommendation {
    /// Action
    pub action: NamespaceAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// Namespace actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceAction {
    /// Cleanup empty namespace
    CleanupEmpty,
    /// Add user mapping
    AddUserMapping,
    /// Merge namespaces
    MergeNamespaces,
    /// Split namespace
    SplitNamespace,
    /// Increase isolation
    IncreaseIsolation,
}

impl NamespaceAction {
    /// Get action name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::CleanupEmpty => "cleanup_empty",
            Self::AddUserMapping => "add_user_mapping",
            Self::MergeNamespaces => "merge_namespaces",
            Self::SplitNamespace => "split_namespace",
            Self::IncreaseIsolation => "increase_isolation",
        }
    }
}
