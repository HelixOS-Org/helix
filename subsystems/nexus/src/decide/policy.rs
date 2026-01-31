//! Policy Engine — Governance for decisions
//!
//! The policy engine evaluates options against defined policies,
//! enforcing safety constraints, rate limits, and access controls.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::*;
use super::options::{ActionType, ActionParameters};

// ============================================================================
// POLICY ID
// ============================================================================

/// Policy ID type
define_id!(PolicyId, "Policy identifier");

// ============================================================================
// POLICY
// ============================================================================

/// A policy that governs decision making
#[derive(Debug, Clone)]
pub struct Policy {
    /// Policy ID
    pub id: PolicyId,
    /// Policy name
    pub name: String,
    /// Description
    pub description: String,
    /// Priority (higher = more important)
    pub priority: u32,
    /// Is enabled
    pub enabled: bool,
    /// Conditions for this policy
    pub conditions: Vec<PolicyCondition>,
    /// Effect when policy applies
    pub effect: PolicyEffect,
    /// Exceptions
    pub exceptions: Vec<PolicyException>,
}

impl Policy {
    /// Create a new policy
    pub fn new(id: PolicyId, name: impl Into<String>, effect: PolicyEffect) -> Self {
        Self {
            id,
            name: name.into(),
            description: String::new(),
            priority: 50,
            enabled: true,
            conditions: Vec::new(),
            effect,
            exceptions: Vec::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Add condition
    pub fn with_condition(mut self, condition: PolicyCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Add exception
    pub fn with_exception(mut self, exception: PolicyException) -> Self {
        self.exceptions.push(exception);
        self
    }

    /// Disable policy
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Enable policy
    pub fn enable(&mut self) {
        self.enabled = true;
    }
}

// ============================================================================
// POLICY CONDITION
// ============================================================================

/// Policy condition
#[derive(Debug, Clone)]
pub struct PolicyCondition {
    /// Condition type
    pub condition_type: ConditionType,
    /// Parameters
    pub parameters: ConditionParameters,
}

impl PolicyCondition {
    /// Create new condition
    pub fn new(condition_type: ConditionType) -> Self {
        Self {
            condition_type,
            parameters: ConditionParameters::default(),
        }
    }

    /// Set parameters
    pub fn with_parameters(mut self, parameters: ConditionParameters) -> Self {
        self.parameters = parameters;
        self
    }

    /// Create action type condition
    pub fn action_type(action_types: Vec<ActionType>) -> Self {
        Self {
            condition_type: ConditionType::ActionType,
            parameters: ConditionParameters {
                action_types,
                ..Default::default()
            },
        }
    }

    /// Create severity condition
    pub fn severity(severities: Vec<Severity>) -> Self {
        Self {
            condition_type: ConditionType::Severity,
            parameters: ConditionParameters {
                severities,
                ..Default::default()
            },
        }
    }

    /// Create confidence threshold condition
    pub fn confidence_above(threshold: f64) -> Self {
        Self {
            condition_type: ConditionType::ConfidenceAbove,
            parameters: ConditionParameters {
                threshold: Some(threshold),
                ..Default::default()
            },
        }
    }

    /// Create always-true condition
    pub fn always() -> Self {
        Self {
            condition_type: ConditionType::Always,
            parameters: ConditionParameters::default(),
        }
    }
}

/// Condition type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionType {
    /// Action type matches
    ActionType,
    /// Severity matches
    Severity,
    /// Confidence above threshold
    ConfidenceAbove,
    /// Confidence below threshold
    ConfidenceBelow,
    /// Time of day
    TimeOfDay,
    /// System load
    SystemLoad,
    /// Target matches
    Target,
    /// Rate limit
    RateLimit,
    /// Resource available
    ResourceAvailable,
    /// Always true
    Always,
}

/// Condition parameters
#[derive(Debug, Clone, Default)]
pub struct ConditionParameters {
    /// Action types
    pub action_types: Vec<ActionType>,
    /// Severity levels
    pub severities: Vec<Severity>,
    /// Threshold value
    pub threshold: Option<f64>,
    /// Target patterns
    pub targets: Vec<String>,
    /// Time range
    pub time_range: Option<TimeRange>,
    /// Rate limit
    pub rate: Option<u64>,
}

// ============================================================================
// POLICY EFFECT
// ============================================================================

/// Policy effect
#[derive(Debug, Clone)]
pub enum PolicyEffect {
    /// Allow the action
    Allow,
    /// Deny the action
    Deny,
    /// Require confirmation
    RequireConfirmation,
    /// Rate limit
    RateLimit { max_per_minute: u32 },
    /// Modify action
    Modify(ActionModification),
    /// Log only
    LogOnly,
}

/// Action modification
#[derive(Debug, Clone)]
pub struct ActionModification {
    /// Override parameters
    pub parameter_overrides: ActionParameters,
    /// Add warnings
    pub add_warnings: Vec<String>,
}

impl ActionModification {
    /// Create empty modification
    pub fn new() -> Self {
        Self {
            parameter_overrides: ActionParameters::new(),
            add_warnings: Vec::new(),
        }
    }

    /// Add warning
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.add_warnings.push(warning.into());
        self
    }
}

impl Default for ActionModification {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// POLICY EXCEPTION
// ============================================================================

/// Policy exception
#[derive(Debug, Clone)]
pub struct PolicyException {
    /// Description
    pub description: String,
    /// Exception condition
    pub condition: PolicyCondition,
}

impl PolicyException {
    /// Create new exception
    pub fn new(description: impl Into<String>, condition: PolicyCondition) -> Self {
        Self {
            description: description.into(),
            condition,
        }
    }
}

// ============================================================================
// POLICY RESULT
// ============================================================================

/// Policy evaluation result
#[derive(Debug, Clone)]
pub struct PolicyResult {
    /// Is action allowed
    pub allowed: bool,
    /// Requires human confirmation
    pub requires_confirmation: bool,
    /// Is rate limited
    pub rate_limited: bool,
    /// Modifications to apply
    pub modifications: Option<ActionModification>,
    /// Policies that were applied
    pub applied_policies: Vec<PolicyId>,
    /// Warnings
    pub warnings: Vec<String>,
}

impl PolicyResult {
    /// Create allowed result
    pub fn allowed() -> Self {
        Self {
            allowed: true,
            requires_confirmation: false,
            rate_limited: false,
            modifications: None,
            applied_policies: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create denied result
    pub fn denied(policy_id: PolicyId) -> Self {
        Self {
            allowed: false,
            requires_confirmation: false,
            rate_limited: false,
            modifications: None,
            applied_policies: vec![policy_id],
            warnings: Vec::new(),
        }
    }
}

// ============================================================================
// POLICY ENGINE
// ============================================================================

/// Policy engine - evaluates options against policies
pub struct PolicyEngine {
    /// Registered policies
    policies: BTreeMap<PolicyId, Policy>,
    /// Policy evaluations
    evaluations: AtomicU64,
    /// Denials
    denials: AtomicU64,
}

impl PolicyEngine {
    /// Create new policy engine
    pub fn new() -> Self {
        let mut engine = Self {
            policies: BTreeMap::new(),
            evaluations: AtomicU64::new(0),
            denials: AtomicU64::new(0),
        };
        engine.load_default_policies();
        engine
    }

    /// Load default policies
    fn load_default_policies(&mut self) {
        // Safety policy - deny destructive actions without high confidence
        self.register(Policy {
            id: PolicyId::new(1),
            name: String::from("safety_destructive"),
            description: String::from("Require high confidence for destructive actions"),
            priority: 100,
            enabled: true,
            conditions: vec![PolicyCondition {
                condition_type: ConditionType::ActionType,
                parameters: ConditionParameters {
                    action_types: vec![ActionType::Kill, ActionType::Deallocate],
                    ..Default::default()
                },
            }],
            effect: PolicyEffect::RequireConfirmation,
            exceptions: Vec::new(),
        });

        // Rate limit policy
        self.register(Policy {
            id: PolicyId::new(2),
            name: String::from("rate_limit_restarts"),
            description: String::from("Rate limit restart actions"),
            priority: 90,
            enabled: true,
            conditions: vec![PolicyCondition {
                condition_type: ConditionType::ActionType,
                parameters: ConditionParameters {
                    action_types: vec![ActionType::Restart],
                    ..Default::default()
                },
            }],
            effect: PolicyEffect::RateLimit { max_per_minute: 5 },
            exceptions: Vec::new(),
        });

        // Allow logging always
        self.register(Policy {
            id: PolicyId::new(3),
            name: String::from("allow_logging"),
            description: String::from("Always allow logging"),
            priority: 1000,
            enabled: true,
            conditions: vec![PolicyCondition {
                condition_type: ConditionType::ActionType,
                parameters: ConditionParameters {
                    action_types: vec![ActionType::Log],
                    ..Default::default()
                },
            }],
            effect: PolicyEffect::Allow,
            exceptions: Vec::new(),
        });
    }

    /// Register a policy
    pub fn register(&mut self, policy: Policy) {
        self.policies.insert(policy.id, policy);
    }

    /// Unregister a policy
    pub fn unregister(&mut self, id: PolicyId) -> Option<Policy> {
        self.policies.remove(&id)
    }

    /// Get a policy
    pub fn get(&self, id: PolicyId) -> Option<&Policy> {
        self.policies.get(&id)
    }

    /// Get mutable policy
    pub fn get_mut(&mut self, id: PolicyId) -> Option<&mut Policy> {
        self.policies.get_mut(&id)
    }

    /// List all policies
    pub fn list(&self) -> impl Iterator<Item = &Policy> {
        self.policies.values()
    }

    /// Evaluate an option against policies
    pub fn evaluate(&self, option: &super::options::Option, confidence: Confidence) -> PolicyResult {
        self.evaluations.fetch_add(1, Ordering::Relaxed);

        let mut result = PolicyResult {
            allowed: true,
            requires_confirmation: false,
            rate_limited: false,
            modifications: None,
            applied_policies: Vec::new(),
            warnings: Vec::new(),
        };

        // Sort policies by priority (descending)
        let mut sorted_policies: Vec<_> = self.policies.values().filter(|p| p.enabled).collect();
        sorted_policies.sort_by(|a, b| b.priority.cmp(&a.priority));

        for policy in sorted_policies {
            if self.policy_applies(policy, option, confidence) {
                result.applied_policies.push(policy.id);

                match &policy.effect {
                    PolicyEffect::Allow => {
                        // Continue checking other policies
                    }
                    PolicyEffect::Deny => {
                        result.allowed = false;
                        self.denials.fetch_add(1, Ordering::Relaxed);
                        break;
                    }
                    PolicyEffect::RequireConfirmation => {
                        result.requires_confirmation = true;
                    }
                    PolicyEffect::RateLimit { .. } => {
                        result.rate_limited = true;
                    }
                    PolicyEffect::Modify(modification) => {
                        result.modifications = Some(modification.clone());
                    }
                    PolicyEffect::LogOnly => {
                        result.warnings.push(format!(
                            "Policy '{}' triggered logging",
                            policy.name
                        ));
                    }
                }
            }
        }

        result
    }

    /// Check if policy applies
    fn policy_applies(&self, policy: &Policy, option: &super::options::Option, confidence: Confidence) -> bool {
        policy.conditions.iter().all(|condition| {
            self.condition_matches(condition, option, confidence)
        })
    }

    /// Check if condition matches
    fn condition_matches(
        &self,
        condition: &PolicyCondition,
        option: &super::options::Option,
        confidence: Confidence,
    ) -> bool {
        match condition.condition_type {
            ConditionType::ActionType => {
                condition.parameters.action_types.contains(&option.action_type)
            }
            ConditionType::ConfidenceAbove => {
                condition
                    .parameters
                    .threshold
                    .map(|t| confidence.value() > t as f32)
                    .unwrap_or(false)
            }
            ConditionType::ConfidenceBelow => {
                condition
                    .parameters
                    .threshold
                    .map(|t| confidence.value() < t as f32)
                    .unwrap_or(false)
            }
            ConditionType::Always => true,
            _ => false,
        }
    }

    /// Get statistics
    pub fn stats(&self) -> PolicyStats {
        PolicyStats {
            policies_registered: self.policies.len(),
            evaluations: self.evaluations.load(Ordering::Relaxed),
            denials: self.denials.load(Ordering::Relaxed),
        }
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Policy statistics
#[derive(Debug, Clone)]
pub struct PolicyStats {
    /// Number of registered policies
    pub policies_registered: usize,
    /// Total evaluations
    pub evaluations: u64,
    /// Total denials
    pub denials: u64,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::options::{Option, OptionSource, ActionTarget, ExpectedOutcome, ActionCost};

    fn make_test_option(action_type: ActionType) -> Option {
        Option {
            id: super::super::options::OptionId::generate(),
            action_type,
            description: String::from("Test option"),
            target: ActionTarget::System,
            parameters: ActionParameters::new(),
            expected_outcome: ExpectedOutcome::default(),
            reversible: true,
            cost: ActionCost::default(),
            source: OptionSource::Default,
        }
    }

    #[test]
    fn test_policy_engine() {
        let engine = PolicyEngine::new();
        let option = make_test_option(ActionType::Log);

        let result = engine.evaluate(&option, Confidence::HIGH);
        assert!(result.allowed);
    }

    #[test]
    fn test_policy_registration() {
        let mut engine = PolicyEngine::new();
        let policy = Policy::new(PolicyId::new(100), "test_policy", PolicyEffect::Allow);

        engine.register(policy);
        assert!(engine.get(PolicyId::new(100)).is_some());
    }

    #[test]
    fn test_destructive_requires_confirmation() {
        let engine = PolicyEngine::new();
        let option = make_test_option(ActionType::Kill);

        let result = engine.evaluate(&option, Confidence::HIGH);
        assert!(result.requires_confirmation);
    }
}
