//! Pre-Validator — Action validation before execution
//!
//! The pre-validator checks all preconditions before allowing
//! an action to execute, ensuring safety and correctness.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use alloc::format;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{ActionType, Intent};

// ============================================================================
// VALIDATION RULE
// ============================================================================

/// A validation rule
#[derive(Debug, Clone)]
pub struct ValidationRule {
    /// Rule ID
    pub id: u32,
    /// Rule name
    pub name: String,
    /// Action types this applies to
    pub applies_to: Vec<ActionType>,
    /// The check to perform
    pub check: ValidationCheck,
    /// Is blocking (reject on failure)
    pub blocking: bool,
}

impl ValidationRule {
    /// Create new rule
    pub fn new(id: u32, name: impl Into<String>, check: ValidationCheck) -> Self {
        Self {
            id,
            name: name.into(),
            applies_to: Vec::new(),
            check,
            blocking: true,
        }
    }

    /// Set action types
    pub fn for_actions(mut self, actions: Vec<ActionType>) -> Self {
        self.applies_to = actions;
        self
    }

    /// Set as non-blocking
    pub fn non_blocking(mut self) -> Self {
        self.blocking = false;
        self
    }

    /// Check if rule applies to action type
    pub fn applies(&self, action_type: ActionType) -> bool {
        self.applies_to.is_empty() || self.applies_to.contains(&action_type)
    }
}

// ============================================================================
// VALIDATION CHECK
// ============================================================================

/// Validation check type
#[derive(Debug, Clone)]
pub enum ValidationCheck {
    /// Check if target exists
    TargetExists,
    /// Check permissions
    HasPermission,
    /// Check resources available
    ResourcesAvailable,
    /// Check no conflicts
    NoConflicts,
    /// Check parameters valid
    ParametersValid,
    /// Check rate limit
    RateLimitOk,
    /// Check cooldown
    CooldownElapsed,
    /// Custom check
    Custom(String),
}

impl ValidationCheck {
    /// Get display name
    pub fn name(&self) -> &str {
        match self {
            Self::TargetExists => "Target Exists",
            Self::HasPermission => "Has Permission",
            Self::ResourcesAvailable => "Resources Available",
            Self::NoConflicts => "No Conflicts",
            Self::ParametersValid => "Parameters Valid",
            Self::RateLimitOk => "Rate Limit OK",
            Self::CooldownElapsed => "Cooldown Elapsed",
            Self::Custom(name) => name,
        }
    }
}

// ============================================================================
// VALIDATION RESULT
// ============================================================================

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Is valid
    pub valid: bool,
    /// Rules passed
    pub passed: Vec<u32>,
    /// Rules failed
    pub failed: Vec<ValidationFailure>,
    /// Warnings
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create valid result
    pub fn ok() -> Self {
        Self {
            valid: true,
            passed: Vec::new(),
            failed: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create invalid result
    pub fn invalid(failure: ValidationFailure) -> Self {
        Self {
            valid: false,
            passed: Vec::new(),
            failed: vec![failure],
            warnings: Vec::new(),
        }
    }

    /// Add warning
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Get blocking failures
    pub fn blocking_failures(&self) -> impl Iterator<Item = &ValidationFailure> {
        self.failed.iter().filter(|f| f.blocking)
    }

    /// Get failure reasons
    pub fn failure_reasons(&self) -> Vec<&str> {
        self.failed.iter().map(|f| f.reason.as_str()).collect()
    }
}

/// Validation failure
#[derive(Debug, Clone)]
pub struct ValidationFailure {
    /// Rule ID
    pub rule_id: u32,
    /// Rule name
    pub rule_name: String,
    /// Failure reason
    pub reason: String,
    /// Is blocking
    pub blocking: bool,
}

impl ValidationFailure {
    /// Create new failure
    pub fn new(rule_id: u32, rule_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            rule_id,
            rule_name: rule_name.into(),
            reason: reason.into(),
            blocking: true,
        }
    }

    /// Set as non-blocking
    pub fn non_blocking(mut self) -> Self {
        self.blocking = false;
        self
    }
}

// ============================================================================
// PRE-VALIDATOR
// ============================================================================

/// Pre-execution validator
pub struct PreValidator {
    /// Validation rules
    rules: Vec<ValidationRule>,
    /// Validations performed
    validations: AtomicU64,
    /// Rejections
    rejections: AtomicU64,
}

impl PreValidator {
    /// Create new validator
    pub fn new() -> Self {
        Self {
            rules: Self::default_rules(),
            validations: AtomicU64::new(0),
            rejections: AtomicU64::new(0),
        }
    }

    /// Default validation rules
    fn default_rules() -> Vec<ValidationRule> {
        vec![
            ValidationRule {
                id: 1,
                name: String::from("target_exists"),
                applies_to: vec![
                    ActionType::Restart,
                    ActionType::Kill,
                    ActionType::Migrate,
                    ActionType::Reconfigure,
                ],
                check: ValidationCheck::TargetExists,
                blocking: true,
            },
            ValidationRule {
                id: 2,
                name: String::from("has_permission"),
                applies_to: vec![
                    ActionType::Kill,
                    ActionType::Allocate,
                    ActionType::Deallocate,
                    ActionType::Quarantine,
                ],
                check: ValidationCheck::HasPermission,
                blocking: true,
            },
            ValidationRule {
                id: 3,
                name: String::from("resources_available"),
                applies_to: vec![ActionType::Allocate, ActionType::Scale],
                check: ValidationCheck::ResourcesAvailable,
                blocking: true,
            },
            ValidationRule {
                id: 4,
                name: String::from("parameters_valid"),
                applies_to: vec![
                    ActionType::Reconfigure,
                    ActionType::Scale,
                    ActionType::Throttle,
                ],
                check: ValidationCheck::ParametersValid,
                blocking: true,
            },
        ]
    }

    /// Add custom rule
    pub fn add_rule(&mut self, rule: ValidationRule) {
        self.rules.push(rule);
    }

    /// Remove rule by ID
    pub fn remove_rule(&mut self, id: u32) -> bool {
        let len_before = self.rules.len();
        self.rules.retain(|r| r.id != id);
        self.rules.len() != len_before
    }

    /// Get rule count
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Validate an intent
    pub fn validate(&self, intent: &Intent) -> ValidationResult {
        self.validations.fetch_add(1, Ordering::Relaxed);

        let mut result = ValidationResult {
            valid: true,
            passed: Vec::new(),
            failed: Vec::new(),
            warnings: Vec::new(),
        };

        for rule in &self.rules {
            if rule.applies_to.contains(&intent.selected_option.action_type) {
                let check_result = self.perform_check(&rule.check, intent);

                if check_result {
                    result.passed.push(rule.id);
                } else {
                    let failure = ValidationFailure {
                        rule_id: rule.id,
                        rule_name: rule.name.clone(),
                        reason: format!("Failed check: {:?}", rule.check),
                        blocking: rule.blocking,
                    };
                    result.failed.push(failure);

                    if rule.blocking {
                        result.valid = false;
                    }
                }
            }
        }

        if !result.valid {
            self.rejections.fetch_add(1, Ordering::Relaxed);
        }

        result
    }

    /// Perform a specific check
    fn perform_check(&self, check: &ValidationCheck, _intent: &Intent) -> bool {
        match check {
            ValidationCheck::TargetExists => {
                // In real implementation, check if target exists
                true
            }
            ValidationCheck::HasPermission => {
                // In real implementation, check permissions
                true
            }
            ValidationCheck::ResourcesAvailable => {
                // In real implementation, check resources
                true
            }
            ValidationCheck::NoConflicts => {
                // In real implementation, check conflicts
                true
            }
            ValidationCheck::ParametersValid => {
                // In real implementation, validate parameters
                true
            }
            ValidationCheck::RateLimitOk => true,
            ValidationCheck::CooldownElapsed => true,
            ValidationCheck::Custom(_) => true,
        }
    }

    /// Get statistics
    pub fn stats(&self) -> ValidatorStats {
        ValidatorStats {
            validations: self.validations.load(Ordering::Relaxed),
            rejections: self.rejections.load(Ordering::Relaxed),
            rules_count: self.rules.len(),
        }
    }
}

impl Default for PreValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Validator statistics
#[derive(Debug, Clone)]
pub struct ValidatorStats {
    /// Total validations
    pub validations: u64,
    /// Total rejections
    pub rejections: u64,
    /// Number of rules
    pub rules_count: usize,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pre_validator() {
        let validator = PreValidator::new();
        assert!(validator.stats().rules_count > 0);
    }

    #[test]
    fn test_validation_rule() {
        let rule = ValidationRule::new(1, "test_rule", ValidationCheck::TargetExists)
            .for_actions(vec![ActionType::Restart]);

        assert!(rule.applies(ActionType::Restart));
        assert!(!rule.applies(ActionType::Log));
    }

    #[test]
    fn test_validation_result() {
        let result = ValidationResult::ok()
            .with_warning("Minor issue");

        assert!(result.valid);
        assert_eq!(result.warnings.len(), 1);
    }
}
