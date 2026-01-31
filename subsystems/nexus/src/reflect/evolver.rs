//! Evolver — Cognitive improvement engine
//!
//! The evolver generates improvement suggestions based on health
//! and calibration data, tracking their application and outcomes.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::types::*;
use crate::bus::Domain;
use super::introspector::{CognitiveHealth, CognitiveIssue, IssueType};
use super::calibrator::CalibrationReport;

// ============================================================================
// SUGGESTION ID
// ============================================================================

/// Suggestion ID type
define_id!(SuggestionId, "Suggestion identifier");

// ============================================================================
// IMPROVEMENT CATEGORY
// ============================================================================

/// Improvement category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImprovementCategory {
    /// Performance optimization
    Performance,
    /// Accuracy improvement
    Accuracy,
    /// Resource efficiency
    Efficiency,
    /// Reliability improvement
    Reliability,
    /// Capability addition
    Capability,
    /// Configuration tuning
    Configuration,
}

impl ImprovementCategory {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Performance => "Performance",
            Self::Accuracy => "Accuracy",
            Self::Efficiency => "Efficiency",
            Self::Reliability => "Reliability",
            Self::Capability => "Capability",
            Self::Configuration => "Configuration",
        }
    }
}

// ============================================================================
// EFFORT LEVEL
// ============================================================================

/// Effort level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EffortLevel {
    /// Trivial change
    Trivial,
    /// Low effort
    Low,
    /// Medium effort
    Medium,
    /// High effort
    High,
    /// Major undertaking
    Major,
}

impl EffortLevel {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Trivial => "Trivial",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Major => "Major",
        }
    }
}

// ============================================================================
// SUGGESTION STATUS
// ============================================================================

/// Suggestion status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionStatus {
    /// Proposed, not yet reviewed
    Proposed,
    /// Under review
    UnderReview,
    /// Approved for implementation
    Approved,
    /// Applied
    Applied,
    /// Rejected
    Rejected,
    /// Deferred for later
    Deferred,
}

impl SuggestionStatus {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Proposed => "Proposed",
            Self::UnderReview => "Under Review",
            Self::Approved => "Approved",
            Self::Applied => "Applied",
            Self::Rejected => "Rejected",
            Self::Deferred => "Deferred",
        }
    }
}

// ============================================================================
// IMPROVEMENT SUGGESTION
// ============================================================================

/// Improvement suggestion
#[derive(Debug, Clone)]
pub struct ImprovementSuggestion {
    /// Suggestion ID
    pub id: SuggestionId,
    /// Category
    pub category: ImprovementCategory,
    /// Target domain
    pub target_domain: Option<Domain>,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Expected benefit
    pub expected_benefit: String,
    /// Effort required
    pub effort: EffortLevel,
    /// Priority
    pub priority: Priority,
    /// Created at
    pub created_at: Timestamp,
    /// Status
    pub status: SuggestionStatus,
}

impl ImprovementSuggestion {
    /// Create new suggestion
    pub fn new(
        category: ImprovementCategory,
        title: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: SuggestionId::generate(),
            category,
            target_domain: None,
            title: title.into(),
            description: description.into(),
            expected_benefit: String::new(),
            effort: EffortLevel::Medium,
            priority: Priority::Normal,
            created_at: Timestamp::now(),
            status: SuggestionStatus::Proposed,
        }
    }

    /// Set target domain
    pub fn for_domain(mut self, domain: Domain) -> Self {
        self.target_domain = Some(domain);
        self
    }

    /// Set expected benefit
    pub fn with_benefit(mut self, benefit: impl Into<String>) -> Self {
        self.expected_benefit = benefit.into();
        self
    }

    /// Set effort level
    pub fn with_effort(mut self, effort: EffortLevel) -> Self {
        self.effort = effort;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Is pending?
    pub fn is_pending(&self) -> bool {
        self.status == SuggestionStatus::Proposed
    }
}

// ============================================================================
// APPLIED IMPROVEMENT
// ============================================================================

/// Applied improvement
#[derive(Debug, Clone)]
pub struct AppliedImprovement {
    /// Original suggestion ID
    pub suggestion_id: SuggestionId,
    /// Applied at
    pub applied_at: Timestamp,
    /// Baseline metrics before
    pub baseline: Option<String>,
    /// Result metrics after
    pub result: Option<String>,
    /// Was successful
    pub successful: bool,
}

impl AppliedImprovement {
    /// Create new applied improvement
    pub fn new(suggestion_id: SuggestionId, successful: bool) -> Self {
        Self {
            suggestion_id,
            applied_at: Timestamp::now(),
            baseline: None,
            result: None,
            successful,
        }
    }

    /// Set baseline
    pub fn with_baseline(mut self, baseline: impl Into<String>) -> Self {
        self.baseline = Some(baseline.into());
        self
    }

    /// Set result
    pub fn with_result(mut self, result: impl Into<String>) -> Self {
        self.result = Some(result.into());
        self
    }
}

// ============================================================================
// EVOLVER
// ============================================================================

/// Evolver - drives cognitive improvement
pub struct Evolver {
    /// Improvement suggestions
    suggestions: Vec<ImprovementSuggestion>,
    /// Applied improvements
    applied: Vec<AppliedImprovement>,
    /// Maximum suggestions
    max_suggestions: usize,
}

impl Evolver {
    /// Create new evolver
    pub fn new(max_suggestions: usize) -> Self {
        Self {
            suggestions: Vec::new(),
            applied: Vec::new(),
            max_suggestions,
        }
    }

    /// Add a suggestion
    pub fn suggest(&mut self, suggestion: ImprovementSuggestion) -> SuggestionId {
        let id = suggestion.id;
        self.suggestions.push(suggestion);
        if self.suggestions.len() > self.max_suggestions {
            // Remove oldest non-applied suggestion
            if let Some(idx) = self.suggestions.iter().position(|s| s.status != SuggestionStatus::Applied) {
                self.suggestions.remove(idx);
            }
        }
        id
    }

    /// Generate suggestions from health and calibration data
    pub fn generate_suggestions(
        &mut self,
        health: &CognitiveHealth,
        calibration: &CalibrationReport,
    ) -> Vec<SuggestionId> {
        let mut ids = Vec::new();

        // From health issues
        for issue in &health.issues {
            let suggestion = self.suggestion_for_issue(issue);
            ids.push(self.suggest(suggestion));
        }

        // From calibration recommendations
        for rec in &calibration.recommendations {
            let suggestion = ImprovementSuggestion {
                id: SuggestionId::generate(),
                category: ImprovementCategory::Accuracy,
                target_domain: None,
                title: format!("Adjust {}", rec.parameter),
                description: rec.reason.clone(),
                expected_benefit: format!(
                    "Change {} from {:.2} to {:.2}",
                    rec.parameter, rec.current_value, rec.recommended_value
                ),
                effort: EffortLevel::Low,
                priority: Priority::High,
                created_at: Timestamp::now(),
                status: SuggestionStatus::Proposed,
            };
            ids.push(self.suggest(suggestion));
        }

        ids
    }

    /// Generate suggestion for a cognitive issue
    fn suggestion_for_issue(&self, issue: &CognitiveIssue) -> ImprovementSuggestion {
        let (category, title, description, effort) = match issue.issue_type {
            IssueType::HighLatency => (
                ImprovementCategory::Performance,
                String::from("Reduce processing latency"),
                format!("Optimize {:?} domain processing", issue.domain),
                EffortLevel::Medium,
            ),
            IssueType::HighErrorRate => (
                ImprovementCategory::Reliability,
                String::from("Reduce error rate"),
                format!("Fix errors in {:?} domain", issue.domain),
                EffortLevel::Medium,
            ),
            IssueType::QueueBacklog => (
                ImprovementCategory::Performance,
                String::from("Clear queue backlog"),
                format!("Increase processing capacity for {:?}", issue.domain),
                EffortLevel::Low,
            ),
            IssueType::DecliningHealth => (
                ImprovementCategory::Reliability,
                String::from("Stabilize domain health"),
                format!("Investigate and fix {:?} domain issues", issue.domain),
                EffortLevel::High,
            ),
            _ => (
                ImprovementCategory::Reliability,
                String::from("Address cognitive issue"),
                issue.description.clone(),
                EffortLevel::Medium,
            ),
        };

        ImprovementSuggestion {
            id: SuggestionId::generate(),
            category,
            target_domain: Some(issue.domain),
            title,
            description,
            expected_benefit: String::from("Improved system health"),
            effort,
            priority: match issue.severity {
                Severity::Critical => Priority::Critical,
                Severity::Error => Priority::High,
                Severity::Warning => Priority::Normal,
                _ => Priority::Low,
            },
            created_at: Timestamp::now(),
            status: SuggestionStatus::Proposed,
        }
    }

    /// Mark suggestion as applied
    pub fn mark_applied(
        &mut self,
        id: SuggestionId,
        successful: bool,
        baseline: Option<String>,
        result: Option<String>,
    ) {
        if let Some(suggestion) = self.suggestions.iter_mut().find(|s| s.id == id) {
            suggestion.status = SuggestionStatus::Applied;
        }

        self.applied.push(AppliedImprovement {
            suggestion_id: id,
            applied_at: Timestamp::now(),
            baseline,
            result,
            successful,
        });
    }

    /// Get pending suggestions
    pub fn pending(&self) -> Vec<&ImprovementSuggestion> {
        self.suggestions
            .iter()
            .filter(|s| s.status == SuggestionStatus::Proposed)
            .collect()
    }

    /// Get all suggestions
    pub fn suggestions(&self) -> &[ImprovementSuggestion] {
        &self.suggestions
    }

    /// Get applied improvements
    pub fn applied(&self) -> &[AppliedImprovement] {
        &self.applied
    }

    /// Get statistics
    pub fn stats(&self) -> EvolverStats {
        EvolverStats {
            suggestions_total: self.suggestions.len(),
            suggestions_pending: self.pending().len(),
            improvements_applied: self.applied.len(),
            improvements_successful: self.applied.iter().filter(|a| a.successful).count(),
        }
    }
}

impl Default for Evolver {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Evolver statistics
#[derive(Debug, Clone)]
pub struct EvolverStats {
    /// Total suggestions
    pub suggestions_total: usize,
    /// Pending suggestions
    pub suggestions_pending: usize,
    /// Applied improvements
    pub improvements_applied: usize,
    /// Successful improvements
    pub improvements_successful: usize,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evolver() {
        let mut evolver = Evolver::new(100);

        let suggestion = ImprovementSuggestion::new(
            ImprovementCategory::Performance,
            "Test improvement",
            "Test description",
        );

        let id = evolver.suggest(suggestion);
        assert!(!evolver.pending().is_empty());

        evolver.mark_applied(id, true, None, None);
        assert!(evolver.pending().is_empty());
    }

    #[test]
    fn test_improvement_suggestion() {
        let suggestion = ImprovementSuggestion::new(
            ImprovementCategory::Accuracy,
            "Improve prediction",
            "Better model",
        )
        .for_domain(Domain::Reason)
        .with_benefit("Higher accuracy")
        .with_effort(EffortLevel::High)
        .with_priority(Priority::High);

        assert!(suggestion.is_pending());
        assert!(suggestion.target_domain.is_some());
    }

    #[test]
    fn test_applied_improvement() {
        let applied = AppliedImprovement::new(SuggestionId::generate(), true)
            .with_baseline("50%")
            .with_result("80%");

        assert!(applied.successful);
        assert!(applied.baseline.is_some());
        assert!(applied.result.is_some());
    }
}
