//! # Action Validation
//!
//! Validates actions before execution.
//! Checks preconditions, safety, and consistency.
//!
//! Part of Year 2 COGNITION - Action Engine

#![allow(dead_code)]

extern crate alloc;
use alloc::format;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// VALIDATION TYPES
// ============================================================================

/// Action to validate
#[derive(Debug, Clone)]
pub struct ActionSpec {
    /// Action ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Type
    pub action_type: String,
    /// Parameters
    pub params: BTreeMap<String, ParamValue>,
    /// Preconditions
    pub preconditions: Vec<Precondition>,
    /// Expected effects
    pub expected_effects: Vec<Effect>,
}

/// Parameter value
#[derive(Debug, Clone)]
pub enum ParamValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    Text(String),
    List(Vec<ParamValue>),
}

/// Precondition
#[derive(Debug, Clone)]
pub struct Precondition {
    /// Name
    pub name: String,
    /// Condition type
    pub condition_type: ConditionType,
    /// Required value
    pub required: ParamValue,
    /// Is critical
    pub critical: bool,
}

/// Condition type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionType {
    StateEquals,
    StateExists,
    StateNotExists,
    ValueGreaterThan,
    ValueLessThan,
    ValueInRange,
    ResourceAvailable,
    PermissionGranted,
}

/// Effect
#[derive(Debug, Clone)]
pub struct Effect {
    /// Target
    pub target: String,
    /// Operation
    pub operation: EffectOp,
    /// Value
    pub value: ParamValue,
}

/// Effect operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectOp {
    Set,
    Add,
    Remove,
    Increment,
    Decrement,
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Action ID
    pub action_id: u64,
    /// Is valid
    pub valid: bool,
    /// Errors
    pub errors: Vec<ValidationError>,
    /// Warnings
    pub warnings: Vec<ValidationWarning>,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error code
    pub code: String,
    /// Message
    pub message: String,
    /// Severity
    pub severity: ErrorSeverity,
    /// Failed precondition
    pub precondition: Option<String>,
}

/// Error severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Critical,
    Error,
    Warning,
}

/// Validation warning
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Code
    pub code: String,
    /// Message
    pub message: String,
}

/// Validation rule
#[derive(Debug, Clone)]
pub struct ValidationRule {
    /// Rule ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Applies to action types
    pub applies_to: Vec<String>,
    /// Check function ID
    pub check_id: String,
    /// Error on failure
    pub error_message: String,
}

// ============================================================================
// VALIDATOR
// ============================================================================

/// Action validator
pub struct ActionValidator {
    /// Current state
    state: BTreeMap<String, ParamValue>,
    /// Resources
    resources: BTreeMap<String, f64>,
    /// Permissions
    permissions: BTreeMap<String, bool>,
    /// Rules
    rules: Vec<ValidationRule>,
    /// History
    history: Vec<ValidationResult>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: ValidatorConfig,
    /// Statistics
    stats: ValidatorStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct ValidatorConfig {
    /// Strict mode
    pub strict: bool,
    /// Check effects
    pub check_effects: bool,
    /// Maximum history
    pub max_history: usize,
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            strict: true,
            check_effects: true,
            max_history: 1000,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct ValidatorStats {
    /// Validations performed
    pub validations: u64,
    /// Valid actions
    pub valid: u64,
    /// Invalid actions
    pub invalid: u64,
}

impl ActionValidator {
    /// Create new validator
    pub fn new(config: ValidatorConfig) -> Self {
        Self {
            state: BTreeMap::new(),
            resources: BTreeMap::new(),
            permissions: BTreeMap::new(),
            rules: Vec::new(),
            history: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: ValidatorStats::default(),
        }
    }

    /// Set state
    pub fn set_state(&mut self, key: &str, value: ParamValue) {
        self.state.insert(key.into(), value);
    }

    /// Get state
    pub fn get_state(&self, key: &str) -> Option<&ParamValue> {
        self.state.get(key)
    }

    /// Set resource
    pub fn set_resource(&mut self, name: &str, amount: f64) {
        self.resources.insert(name.into(), amount);
    }

    /// Grant permission
    pub fn grant_permission(&mut self, name: &str) {
        self.permissions.insert(name.into(), true);
    }

    /// Revoke permission
    pub fn revoke_permission(&mut self, name: &str) {
        self.permissions.insert(name.into(), false);
    }

    /// Add rule
    pub fn add_rule(&mut self, rule: ValidationRule) {
        self.rules.push(rule);
    }

    /// Validate action
    pub fn validate(&mut self, action: &ActionSpec) -> ValidationResult {
        self.stats.validations += 1;

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check preconditions
        for precond in &action.preconditions {
            if let Some(error) = self.check_precondition(precond) {
                if precond.critical {
                    errors.push(error);
                } else {
                    warnings.push(ValidationWarning {
                        code: "PRECOND_SOFT_FAIL".into(),
                        message: error.message,
                    });
                }
            }
        }

        // Check rules
        for rule in &self.rules {
            if rule.applies_to.is_empty() || rule.applies_to.contains(&action.action_type) {
                if let Some(error) = self.check_rule(rule, action) {
                    errors.push(error);
                }
            }
        }

        // Check effects feasibility
        if self.config.check_effects {
            for effect in &action.expected_effects {
                if let Some(error) = self.check_effect_feasibility(effect) {
                    errors.push(error);
                }
            }
        }

        let valid = errors.is_empty() || (!self.config.strict && errors.iter().all(|e| e.severity != ErrorSeverity::Critical));

        if valid {
            self.stats.valid += 1;
        } else {
            self.stats.invalid += 1;
        }

        let result = ValidationResult {
            action_id: action.id,
            valid,
            errors,
            warnings,
            timestamp: Timestamp::now(),
        };

        self.record_result(result.clone());

        result
    }

    fn check_precondition(&self, precond: &Precondition) -> Option<ValidationError> {
        match precond.condition_type {
            ConditionType::StateEquals => {
                let current = self.state.get(&precond.name)?;

                if !self.values_equal(current, &precond.required) {
                    return Some(ValidationError {
                        code: "STATE_MISMATCH".into(),
                        message: format!("State {} does not match required value", precond.name),
                        severity: if precond.critical { ErrorSeverity::Critical } else { ErrorSeverity::Error },
                        precondition: Some(precond.name.clone()),
                    });
                }
            }

            ConditionType::StateExists => {
                if !self.state.contains_key(&precond.name) {
                    return Some(ValidationError {
                        code: "STATE_MISSING".into(),
                        message: format!("Required state {} does not exist", precond.name),
                        severity: ErrorSeverity::Error,
                        precondition: Some(precond.name.clone()),
                    });
                }
            }

            ConditionType::StateNotExists => {
                if self.state.contains_key(&precond.name) {
                    return Some(ValidationError {
                        code: "STATE_EXISTS".into(),
                        message: format!("State {} should not exist", precond.name),
                        severity: ErrorSeverity::Error,
                        precondition: Some(precond.name.clone()),
                    });
                }
            }

            ConditionType::ValueGreaterThan => {
                let current = self.state.get(&precond.name)?;

                if let (ParamValue::Float(c), ParamValue::Float(r)) = (current, &precond.required) {
                    if *c <= *r {
                        return Some(ValidationError {
                            code: "VALUE_TOO_LOW".into(),
                            message: format!("{} must be greater than {}", precond.name, r),
                            severity: ErrorSeverity::Error,
                            precondition: Some(precond.name.clone()),
                        });
                    }
                }
            }

            ConditionType::ValueLessThan => {
                let current = self.state.get(&precond.name)?;

                if let (ParamValue::Float(c), ParamValue::Float(r)) = (current, &precond.required) {
                    if *c >= *r {
                        return Some(ValidationError {
                            code: "VALUE_TOO_HIGH".into(),
                            message: format!("{} must be less than {}", precond.name, r),
                            severity: ErrorSeverity::Error,
                            precondition: Some(precond.name.clone()),
                        });
                    }
                }
            }

            ConditionType::ResourceAvailable => {
                if let ParamValue::Float(required) = &precond.required {
                    let available = self.resources.get(&precond.name).copied().unwrap_or(0.0);

                    if available < *required {
                        return Some(ValidationError {
                            code: "INSUFFICIENT_RESOURCE".into(),
                            message: format!("Insufficient {} (need {}, have {})", precond.name, required, available),
                            severity: ErrorSeverity::Critical,
                            precondition: Some(precond.name.clone()),
                        });
                    }
                }
            }

            ConditionType::PermissionGranted => {
                let granted = self.permissions.get(&precond.name).copied().unwrap_or(false);

                if !granted {
                    return Some(ValidationError {
                        code: "PERMISSION_DENIED".into(),
                        message: format!("Permission {} not granted", precond.name),
                        severity: ErrorSeverity::Critical,
                        precondition: Some(precond.name.clone()),
                    });
                }
            }

            _ => {}
        }

        None
    }

    fn check_rule(&self, _rule: &ValidationRule, _action: &ActionSpec) -> Option<ValidationError> {
        // Simplified: rules are checked externally
        None
    }

    fn check_effect_feasibility(&self, effect: &Effect) -> Option<ValidationError> {
        match effect.operation {
            EffectOp::Remove => {
                if !self.state.contains_key(&effect.target) {
                    return Some(ValidationError {
                        code: "EFFECT_INFEASIBLE".into(),
                        message: format!("Cannot remove non-existent {}", effect.target),
                        severity: ErrorSeverity::Warning,
                        precondition: None,
                    });
                }
            }

            EffectOp::Decrement => {
                if let Some(ParamValue::Float(current)) = self.state.get(&effect.target) {
                    if let ParamValue::Float(amount) = &effect.value {
                        if current < amount {
                            return Some(ValidationError {
                                code: "EFFECT_INFEASIBLE".into(),
                                message: format!("Cannot decrement {} below zero", effect.target),
                                severity: ErrorSeverity::Error,
                                precondition: None,
                            });
                        }
                    }
                }
            }

            _ => {}
        }

        None
    }

    fn values_equal(&self, a: &ParamValue, b: &ParamValue) -> bool {
        match (a, b) {
            (ParamValue::Null, ParamValue::Null) => true,
            (ParamValue::Boolean(x), ParamValue::Boolean(y)) => x == y,
            (ParamValue::Integer(x), ParamValue::Integer(y)) => x == y,
            (ParamValue::Float(x), ParamValue::Float(y)) => (x - y).abs() < f64::EPSILON,
            (ParamValue::Text(x), ParamValue::Text(y)) => x == y,
            _ => false,
        }
    }

    fn record_result(&mut self, result: ValidationResult) {
        self.history.push(result);

        while self.history.len() > self.config.max_history {
            self.history.remove(0);
        }
    }

    /// Get history
    pub fn history(&self) -> &[ValidationResult] {
        &self.history
    }

    /// Get statistics
    pub fn stats(&self) -> &ValidatorStats {
        &self.stats
    }
}

impl Default for ActionValidator {
    fn default() -> Self {
        Self::new(ValidatorConfig::default())
    }
}

// ============================================================================
// ACTION BUILDER
// ============================================================================

/// Action builder
pub struct ActionBuilder {
    id: u64,
    name: String,
    action_type: String,
    params: BTreeMap<String, ParamValue>,
    preconditions: Vec<Precondition>,
    effects: Vec<Effect>,
}

impl ActionBuilder {
    /// Create new builder
    pub fn new(id: u64, name: &str) -> Self {
        Self {
            id,
            name: name.into(),
            action_type: "default".into(),
            params: BTreeMap::new(),
            preconditions: Vec::new(),
            effects: Vec::new(),
        }
    }

    /// Set type
    pub fn action_type(mut self, t: &str) -> Self {
        self.action_type = t.into();
        self
    }

    /// Add parameter
    pub fn param(mut self, name: &str, value: ParamValue) -> Self {
        self.params.insert(name.into(), value);
        self
    }

    /// Require state equals
    pub fn require_state(mut self, name: &str, value: ParamValue, critical: bool) -> Self {
        self.preconditions.push(Precondition {
            name: name.into(),
            condition_type: ConditionType::StateEquals,
            required: value,
            critical,
        });
        self
    }

    /// Require permission
    pub fn require_permission(mut self, name: &str) -> Self {
        self.preconditions.push(Precondition {
            name: name.into(),
            condition_type: ConditionType::PermissionGranted,
            required: ParamValue::Boolean(true),
            critical: true,
        });
        self
    }

    /// Add effect
    pub fn effect(mut self, target: &str, op: EffectOp, value: ParamValue) -> Self {
        self.effects.push(Effect {
            target: target.into(),
            operation: op,
            value,
        });
        self
    }

    /// Build
    pub fn build(self) -> ActionSpec {
        ActionSpec {
            id: self.id,
            name: self.name,
            action_type: self.action_type,
            params: self.params,
            preconditions: self.preconditions,
            expected_effects: self.effects,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_simple() {
        let mut validator = ActionValidator::default();

        let action = ActionBuilder::new(1, "test").build();
        let result = validator.validate(&action);

        assert!(result.valid);
    }

    #[test]
    fn test_state_precondition() {
        let mut validator = ActionValidator::default();

        validator.set_state("status", ParamValue::Text("ready".into()));

        let action = ActionBuilder::new(1, "run")
            .require_state("status", ParamValue::Text("ready".into()), true)
            .build();

        let result = validator.validate(&action);
        assert!(result.valid);
    }

    #[test]
    fn test_state_precondition_fail() {
        let mut validator = ActionValidator::default();

        validator.set_state("status", ParamValue::Text("busy".into()));

        let action = ActionBuilder::new(1, "run")
            .require_state("status", ParamValue::Text("ready".into()), true)
            .build();

        let result = validator.validate(&action);
        assert!(!result.valid);
    }

    #[test]
    fn test_permission() {
        let mut validator = ActionValidator::default();

        validator.grant_permission("admin");

        let action = ActionBuilder::new(1, "admin_action")
            .require_permission("admin")
            .build();

        let result = validator.validate(&action);
        assert!(result.valid);
    }

    #[test]
    fn test_permission_denied() {
        let mut validator = ActionValidator::default();

        let action = ActionBuilder::new(1, "admin_action")
            .require_permission("admin")
            .build();

        let result = validator.validate(&action);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.code == "PERMISSION_DENIED"));
    }
}
