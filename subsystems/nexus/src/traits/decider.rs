//! Decider Traits
//!
//! Traits for the DECIDE domain - decision making and policy evaluation.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

use super::component::NexusComponent;
use crate::types::{Confidence, Intent, NexusResult, PolicyId, Severity};

// ============================================================================
// DECIDER TRAIT
// ============================================================================

/// Trait for decision-making components
pub trait Decider: NexusComponent {
    /// Context type (input)
    type Context;
    /// Intent type (output)
    type Intent;

    /// Make a decision given context
    fn decide(&self, context: &Self::Context) -> NexusResult<Self::Intent>;

    /// Get decision confidence
    fn confidence(&self) -> Confidence;

    /// Explain decision
    fn explain(&self, intent: &Self::Intent) -> String;

    /// Validate decision against policies
    fn validate(&self, intent: &Self::Intent) -> ValidationResult;
}

// ============================================================================
// VALIDATION RESULT
// ============================================================================

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Is valid
    pub valid: bool,
    /// Violations
    pub violations: Vec<PolicyViolation>,
    /// Warnings (non-blocking)
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create valid result
    pub fn valid() -> Self {
        Self {
            valid: true,
            violations: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create invalid result with violations
    pub fn invalid(violations: Vec<PolicyViolation>) -> Self {
        Self {
            valid: false,
            violations,
            warnings: Vec::new(),
        }
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Add a violation (makes result invalid)
    pub fn with_violation(mut self, violation: PolicyViolation) -> Self {
        self.valid = false;
        self.violations.push(violation);
        self
    }

    /// Has warnings?
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Total issue count (violations + warnings)
    pub fn issue_count(&self) -> usize {
        self.violations.len() + self.warnings.len()
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::valid()
    }
}

// ============================================================================
// POLICY VIOLATION
// ============================================================================

/// Policy violation
#[derive(Debug, Clone)]
pub struct PolicyViolation {
    /// Policy that was violated
    pub policy: PolicyId,
    /// Policy name
    pub policy_name: String,
    /// Violation description
    pub description: String,
    /// Severity of violation
    pub severity: Severity,
    /// Is blocking (must be resolved)?
    pub blocking: bool,
}

impl PolicyViolation {
    /// Create new policy violation
    pub fn new(policy: PolicyId, description: impl Into<String>) -> Self {
        Self {
            policy,
            policy_name: String::new(),
            description: description.into(),
            severity: Severity::MEDIUM,
            blocking: true,
        }
    }

    /// With policy name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.policy_name = name.into();
        self
    }

    /// With severity
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// As non-blocking warning
    pub fn as_warning(mut self) -> Self {
        self.blocking = false;
        self
    }
}

// ============================================================================
// POLICY ENGINE TRAIT
// ============================================================================

/// Policy engine trait
pub trait PolicyEngine: NexusComponent {
    /// Evaluate intent against all policies
    fn evaluate(&self, intent: &Intent) -> ValidationResult;

    /// Get active policies
    fn active_policies(&self) -> Vec<PolicyId>;

    /// Get all policies
    fn all_policies(&self) -> Vec<PolicyId>;

    /// Enable policy
    fn enable_policy(&mut self, policy: PolicyId) -> NexusResult<()>;

    /// Disable policy
    fn disable_policy(&mut self, policy: PolicyId) -> NexusResult<()>;

    /// Is policy enabled?
    fn is_enabled(&self, policy: PolicyId) -> bool;

    /// Get policy priority
    fn priority(&self, policy: PolicyId) -> Option<u32>;
}

// ============================================================================
// OPTION GENERATOR TRAIT
// ============================================================================

/// Option generator trait
pub trait OptionGenerator: NexusComponent {
    /// Context type
    type Context;
    /// Option type
    type Option;

    /// Generate options for a given context
    fn generate(&self, context: &Self::Context) -> Vec<Self::Option>;

    /// Prune invalid options
    fn prune(&self, options: Vec<Self::Option>) -> Vec<Self::Option>;

    /// Score an option (higher = better)
    fn score(&self, option: &Self::Option) -> f64;

    /// Rank options by score
    fn rank(&self, options: Vec<Self::Option>) -> Vec<Self::Option> {
        let mut scored: Vec<_> = options
            .into_iter()
            .map(|o| {
                let s = self.score(&o);
                (o, s)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        scored.into_iter().map(|(o, _)| o).collect()
    }
}

// ============================================================================
// CONFLICT RESOLVER TRAIT
// ============================================================================

/// Conflict resolver trait
pub trait ConflictResolver: NexusComponent {
    /// Option type
    type Option;

    /// Detect conflicts between options
    fn detect_conflicts(&self, options: &[Self::Option]) -> Vec<Conflict>;

    /// Resolve a conflict
    fn resolve(&self, conflict: &Conflict, options: &[Self::Option]) -> Resolution;

    /// Get resolution strategy
    fn strategy(&self) -> ResolutionStrategy;

    /// Set resolution strategy
    fn set_strategy(&mut self, strategy: ResolutionStrategy);
}

/// A conflict between options
#[derive(Debug, Clone)]
pub struct Conflict {
    /// Conflicting option indices
    pub options: Vec<usize>,
    /// Conflict type
    pub conflict_type: ConflictType,
    /// Description
    pub description: String,
    /// Severity
    pub severity: Severity,
}

/// Types of conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConflictType {
    /// Resource contention
    Resource,
    /// Mutually exclusive options
    MutuallyExclusive,
    /// Priority conflict
    Priority,
    /// Timing conflict
    Timing,
    /// Policy conflict
    Policy,
}

/// Resolution of a conflict
#[derive(Debug, Clone)]
pub struct Resolution {
    /// Which option(s) to keep
    pub keep: Vec<usize>,
    /// Which option(s) to drop
    pub drop: Vec<usize>,
    /// Rationale
    pub rationale: String,
    /// Confidence in resolution
    pub confidence: Confidence,
}

/// Resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolutionStrategy {
    /// Pick highest priority
    Priority,
    /// Pick most recent
    MostRecent,
    /// Pick most confident
    MostConfident,
    /// Try to merge
    Merge,
    /// Abort all
    AbortAll,
    /// Human decision required
    Escalate,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result() {
        let valid = ValidationResult::valid();
        assert!(valid.valid);
        assert!(valid.violations.is_empty());

        let with_warning = valid.with_warning("Minor issue");
        assert!(with_warning.valid);
        assert!(with_warning.has_warnings());
    }

    #[test]
    fn test_policy_violation() {
        let violation = PolicyViolation::new(PolicyId::generate(), "Rate limit exceeded")
            .with_severity(Severity::HIGH);

        assert!(violation.blocking);
        assert_eq!(violation.severity, Severity::HIGH);
    }
}
