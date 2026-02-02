//! Diagnostician — Cognitive failure diagnosis
//!
//! The diagnostician records cognitive failures, identifies root causes,
//! and finds patterns to guide system improvement.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};
use core::sync::atomic::{AtomicU64, Ordering};

use crate::bus::Domain;
use crate::types::*;

// ============================================================================
// FAILURE TYPE
// ============================================================================

/// Failure type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FailureType {
    /// False positive - predicted issue that didn't happen
    FalsePositive,
    /// False negative - missed issue
    FalseNegative,
    /// Wrong action - took incorrect action
    WrongAction,
    /// Late detection - detected too late
    LateDetection,
    /// Oscillation - repeated flip-flop
    Oscillation,
    /// Cascade - failure cascaded to other domains
    Cascade,
    /// Timeout - processing took too long
    Timeout,
    /// Resource exhaustion
    ResourceExhaustion,
}

impl FailureType {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::FalsePositive => "False Positive",
            Self::FalseNegative => "False Negative",
            Self::WrongAction => "Wrong Action",
            Self::LateDetection => "Late Detection",
            Self::Oscillation => "Oscillation",
            Self::Cascade => "Cascade",
            Self::Timeout => "Timeout",
            Self::ResourceExhaustion => "Resource Exhaustion",
        }
    }
}

// ============================================================================
// ROOT CAUSE
// ============================================================================

/// Root cause category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootCause {
    /// Insufficient data
    InsufficientData,
    /// Model error
    ModelError,
    /// Configuration issue
    Configuration,
    /// Resource constraint
    ResourceConstraint,
    /// Timing issue
    Timing,
    /// Edge case
    EdgeCase,
    /// External factor
    ExternalFactor,
    /// Unknown
    Unknown,
}

impl RootCause {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::InsufficientData => "Insufficient Data",
            Self::ModelError => "Model Error",
            Self::Configuration => "Configuration",
            Self::ResourceConstraint => "Resource Constraint",
            Self::Timing => "Timing",
            Self::EdgeCase => "Edge Case",
            Self::ExternalFactor => "External Factor",
            Self::Unknown => "Unknown",
        }
    }
}

// ============================================================================
// COGNITIVE FAILURE
// ============================================================================

/// A cognitive failure
#[derive(Debug, Clone)]
pub struct CognitiveFailure {
    /// Failure ID
    pub id: FailureId,
    /// Domain where failure occurred
    pub domain: Domain,
    /// Failure type
    pub failure_type: FailureType,
    /// Description
    pub description: String,
    /// Occurred at
    pub occurred_at: Timestamp,
    /// Context
    pub context: BTreeMap<String, String>,
    /// Diagnosis
    pub diagnosis: Option<Diagnosis>,
}

impl CognitiveFailure {
    /// Create new failure
    pub fn new(domain: Domain, failure_type: FailureType, description: impl Into<String>) -> Self {
        Self {
            id: FailureId::generate(),
            domain,
            failure_type,
            description: description.into(),
            occurred_at: Timestamp::now(),
            context: BTreeMap::new(),
            diagnosis: None,
        }
    }

    /// Add context
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Is diagnosed?
    pub fn is_diagnosed(&self) -> bool {
        self.diagnosis.is_some()
    }
}

// ============================================================================
// DIAGNOSIS
// ============================================================================

/// Diagnosis of a failure
#[derive(Debug, Clone)]
pub struct Diagnosis {
    /// Root cause
    pub root_cause: RootCause,
    /// Contributing factors
    pub contributing_factors: Vec<String>,
    /// Recommendations
    pub recommendations: Vec<String>,
    /// Confidence in diagnosis
    pub confidence: Confidence,
    /// Diagnosed at
    pub diagnosed_at: Timestamp,
}

impl Diagnosis {
    /// Create new diagnosis
    pub fn new(root_cause: RootCause, confidence: Confidence) -> Self {
        Self {
            root_cause,
            contributing_factors: Vec::new(),
            recommendations: Vec::new(),
            confidence,
            diagnosed_at: Timestamp::now(),
        }
    }

    /// Add factor
    pub fn add_factor(&mut self, factor: impl Into<String>) {
        self.contributing_factors.push(factor.into());
    }

    /// Add recommendation
    pub fn add_recommendation(&mut self, recommendation: impl Into<String>) {
        self.recommendations.push(recommendation.into());
    }
}

// ============================================================================
// FAILURE PATTERN
// ============================================================================

/// Failure pattern
#[derive(Debug, Clone)]
pub struct FailurePattern {
    /// Pattern type
    pub pattern_type: PatternType,
    /// Failure type
    pub failure_type: FailureType,
    /// Number of occurrences
    pub occurrences: usize,
    /// Domains affected
    pub domains_affected: Vec<Domain>,
    /// Description
    pub description: String,
}

/// Pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    /// Repeated failures
    Repeated,
    /// Domain-specific issues
    DomainSpecific,
    /// Time-correlated
    TimeCorrelated,
    /// Cascading
    Cascading,
}

impl PatternType {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Repeated => "Repeated",
            Self::DomainSpecific => "Domain Specific",
            Self::TimeCorrelated => "Time Correlated",
            Self::Cascading => "Cascading",
        }
    }
}

// ============================================================================
// DIAGNOSTICIAN
// ============================================================================

/// Diagnostician - diagnoses cognitive failures
pub struct Diagnostician {
    /// Failure records
    failures: Vec<CognitiveFailure>,
    /// Maximum failures
    max_failures: usize,
    /// Diagnoses made
    diagnoses_made: AtomicU64,
}

impl Diagnostician {
    /// Create new diagnostician
    pub fn new(max_failures: usize) -> Self {
        Self {
            failures: Vec::new(),
            max_failures,
            diagnoses_made: AtomicU64::new(0),
        }
    }

    /// Record a failure
    pub fn record_failure(&mut self, failure: CognitiveFailure) -> FailureId {
        let id = failure.id;
        self.failures.push(failure);
        if self.failures.len() > self.max_failures {
            self.failures.remove(0);
        }
        id
    }

    /// Diagnose a failure
    pub fn diagnose(&mut self, failure_id: FailureId) -> Option<Diagnosis> {
        let failure = self.failures.iter().find(|f| f.id == failure_id)?;

        let diagnosis = self.perform_diagnosis(failure);
        self.diagnoses_made.fetch_add(1, Ordering::Relaxed);

        // Update failure with diagnosis
        if let Some(f) = self.failures.iter_mut().find(|f| f.id == failure_id) {
            f.diagnosis = Some(diagnosis.clone());
        }

        Some(diagnosis)
    }

    /// Perform diagnosis
    fn perform_diagnosis(&self, failure: &CognitiveFailure) -> Diagnosis {
        let (root_cause, factors, recommendations) = match failure.failure_type {
            FailureType::FalsePositive => (
                RootCause::ModelError,
                vec![
                    String::from("Pattern matching too aggressive"),
                    String::from("Threshold too low"),
                ],
                vec![
                    String::from("Increase detection threshold"),
                    String::from("Add more confirmation checks"),
                ],
            ),
            FailureType::FalseNegative => (
                RootCause::InsufficientData,
                vec![
                    String::from("Insufficient monitoring coverage"),
                    String::from("Pattern not in training data"),
                ],
                vec![
                    String::from("Expand probe coverage"),
                    String::from("Lower detection threshold"),
                ],
            ),
            FailureType::WrongAction => (
                RootCause::ModelError,
                vec![
                    String::from("Incorrect action mapping"),
                    String::from("Context misunderstood"),
                ],
                vec![
                    String::from("Review action selection rules"),
                    String::from("Add more context to decisions"),
                ],
            ),
            FailureType::LateDetection => (
                RootCause::Timing,
                vec![
                    String::from("Processing latency too high"),
                    String::from("Polling interval too long"),
                ],
                vec![
                    String::from("Reduce sampling interval"),
                    String::from("Optimize processing pipeline"),
                ],
            ),
            FailureType::Oscillation => (
                RootCause::Configuration,
                vec![
                    String::from("Competing rules"),
                    String::from("Insufficient hysteresis"),
                ],
                vec![
                    String::from("Add hysteresis to decisions"),
                    String::from("Increase cooldown periods"),
                ],
            ),
            FailureType::Cascade => (
                RootCause::ResourceConstraint,
                vec![
                    String::from("Insufficient isolation"),
                    String::from("Missing circuit breakers"),
                ],
                vec![
                    String::from("Add circuit breakers"),
                    String::from("Improve domain isolation"),
                ],
            ),
            FailureType::Timeout => (
                RootCause::ResourceConstraint,
                vec![
                    String::from("Processing too complex"),
                    String::from("Resource contention"),
                ],
                vec![
                    String::from("Simplify processing"),
                    String::from("Increase timeouts"),
                ],
            ),
            FailureType::ResourceExhaustion => (
                RootCause::ResourceConstraint,
                vec![
                    String::from("Insufficient capacity"),
                    String::from("Memory leak"),
                ],
                vec![
                    String::from("Increase resource limits"),
                    String::from("Add backpressure"),
                ],
            ),
        };

        Diagnosis {
            root_cause,
            contributing_factors: factors,
            recommendations,
            confidence: Confidence::new(0.75),
            diagnosed_at: Timestamp::now(),
        }
    }

    /// Get failure by ID
    pub fn get_failure(&self, id: FailureId) -> Option<&CognitiveFailure> {
        self.failures.iter().find(|f| f.id == id)
    }

    /// Get all failures
    pub fn failures(&self) -> &[CognitiveFailure] {
        &self.failures
    }

    /// Find failure patterns
    pub fn find_patterns(&self) -> Vec<FailurePattern> {
        let mut patterns = Vec::new();

        // Group failures by type
        let mut by_type: BTreeMap<FailureType, Vec<&CognitiveFailure>> = BTreeMap::new();
        for failure in &self.failures {
            by_type
                .entry(failure.failure_type)
                .or_default()
                .push(failure);
        }

        // Find repeated patterns
        for (&failure_type, failures) in &by_type {
            if failures.len() >= 3 {
                patterns.push(FailurePattern {
                    pattern_type: PatternType::Repeated,
                    failure_type,
                    occurrences: failures.len(),
                    domains_affected: failures.iter().map(|f| f.domain).collect(),
                    description: format!(
                        "{:?} failure occurred {} times",
                        failure_type,
                        failures.len()
                    ),
                });
            }
        }

        // Find domain-specific patterns
        let mut by_domain: BTreeMap<Domain, Vec<&CognitiveFailure>> = BTreeMap::new();
        for failure in &self.failures {
            by_domain.entry(failure.domain).or_default().push(failure);
        }

        for (&domain, failures) in &by_domain {
            if failures.len() >= 5 {
                patterns.push(FailurePattern {
                    pattern_type: PatternType::DomainSpecific,
                    failure_type: failures[0].failure_type,
                    occurrences: failures.len(),
                    domains_affected: vec![domain],
                    description: format!("{:?} domain has {} failures", domain, failures.len()),
                });
            }
        }

        patterns
    }

    /// Get statistics
    pub fn stats(&self) -> DiagnosticianStats {
        DiagnosticianStats {
            failures_recorded: self.failures.len(),
            diagnoses_made: self.diagnoses_made.load(Ordering::Relaxed),
            patterns_found: self.find_patterns().len(),
        }
    }
}

impl Default for Diagnostician {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Diagnostician statistics
#[derive(Debug, Clone)]
pub struct DiagnosticianStats {
    /// Failures recorded
    pub failures_recorded: usize,
    /// Diagnoses made
    pub diagnoses_made: u64,
    /// Patterns found
    pub patterns_found: usize,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostician() {
        let mut diagnostician = Diagnostician::new(100);

        let failure =
            CognitiveFailure::new(Domain::Reason, FailureType::FalsePositive, "Test failure");

        let id = diagnostician.record_failure(failure);
        let diagnosis = diagnostician.diagnose(id);

        assert!(diagnosis.is_some());
        assert_eq!(diagnosis.unwrap().root_cause, RootCause::ModelError);
    }

    #[test]
    fn test_cognitive_failure() {
        let failure = CognitiveFailure::new(
            Domain::Sense,
            FailureType::Timeout,
            "Processing took too long",
        )
        .with_context("duration_ms", "5000");

        assert!(!failure.is_diagnosed());
        assert_eq!(failure.context.len(), 1);
    }

    #[test]
    fn test_diagnosis() {
        let mut diagnosis = Diagnosis::new(RootCause::Timing, Confidence::new(0.8));
        diagnosis.add_factor("High load");
        diagnosis.add_recommendation("Increase timeout");

        assert_eq!(diagnosis.contributing_factors.len(), 1);
        assert_eq!(diagnosis.recommendations.len(), 1);
    }

    #[test]
    fn test_failure_patterns() {
        let mut diagnostician = Diagnostician::new(100);

        // Add multiple failures of same type
        for _ in 0..5 {
            let failure = CognitiveFailure::new(Domain::Act, FailureType::Timeout, "Timeout");
            diagnostician.record_failure(failure);
        }

        let patterns = diagnostician.find_patterns();
        assert!(!patterns.is_empty());
    }
}
