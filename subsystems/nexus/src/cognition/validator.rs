//! # Cognitive Validator
//!
//! Validation framework for cognitive operations and data.
//! Ensures correctness, consistency, and constraint satisfaction.

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// VALIDATION TYPES
// ============================================================================

/// Validation rule
#[derive(Debug, Clone)]
pub struct ValidationRule {
    /// Rule ID
    pub id: u64,
    /// Rule name
    pub name: String,
    /// Rule type
    pub rule_type: RuleType,
    /// Severity
    pub severity: RuleSeverity,
    /// Description
    pub description: String,
    /// Enabled
    pub enabled: bool,
    /// Tags
    pub tags: Vec<String>,
}

/// Rule type
#[derive(Debug, Clone)]
pub enum RuleType {
    /// Schema validation
    Schema(SchemaRule),
    /// Range validation
    Range(RangeRule),
    /// Pattern validation
    Pattern(PatternRule),
    /// Constraint validation
    Constraint(ConstraintRule),
    /// Custom validation
    Custom(CustomRule),
    /// Composite rule
    Composite(CompositeRule),
}

/// Schema rule
#[derive(Debug, Clone)]
pub struct SchemaRule {
    /// Required fields
    pub required: Vec<String>,
    /// Field types
    pub field_types: BTreeMap<String, FieldType>,
    /// Allow extra fields
    pub allow_extra: bool,
}

/// Field type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Bool,
    Int,
    Float,
    String,
    Array,
    Object,
    Any,
}

/// Range rule
#[derive(Debug, Clone)]
pub struct RangeRule {
    /// Field path
    pub field: String,
    /// Minimum value
    pub min: Option<f64>,
    /// Maximum value
    pub max: Option<f64>,
    /// Inclusive
    pub inclusive: bool,
}

/// Pattern rule
#[derive(Debug, Clone)]
pub struct PatternRule {
    /// Field path
    pub field: String,
    /// Pattern
    pub pattern: String,
    /// Pattern type
    pub pattern_type: PatternType,
}

/// Pattern type
#[derive(Debug, Clone, Copy)]
pub enum PatternType {
    Exact,
    Prefix,
    Suffix,
    Contains,
    Regex,
}

/// Constraint rule
#[derive(Debug, Clone)]
pub struct ConstraintRule {
    /// Constraint expression
    pub expression: String,
    /// Referenced fields
    pub fields: Vec<String>,
}

/// Custom rule
#[derive(Debug, Clone)]
pub struct CustomRule {
    /// Handler name
    pub handler: String,
    /// Parameters
    pub params: BTreeMap<String, String>,
}

/// Composite rule
#[derive(Debug, Clone)]
pub struct CompositeRule {
    /// Operator
    pub operator: CompositeOp,
    /// Child rule IDs
    pub rules: Vec<u64>,
}

/// Composite operator
#[derive(Debug, Clone, Copy)]
pub enum CompositeOp {
    And,
    Or,
    Not,
}

/// Rule severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuleSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

// ============================================================================
// VALIDATION DATA
// ============================================================================

/// Data to validate
#[derive(Debug, Clone)]
pub enum ValidationData {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<ValidationData>),
    Object(BTreeMap<String, ValidationData>),
}

impl ValidationData {
    /// Get field by path
    pub fn get_field(&self, path: &str) -> Option<&ValidationData> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = self;

        for part in parts {
            match current {
                ValidationData::Object(obj) => {
                    current = obj.get(part)?;
                }
                ValidationData::Array(arr) => {
                    let index: usize = part.parse().ok()?;
                    current = arr.get(index)?;
                }
                _ => return None,
            }
        }

        Some(current)
    }

    /// Get as f64
    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ValidationData::Int(i) => Some(*i as f64),
            ValidationData::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Get as string
    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ValidationData::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get field type
    #[inline]
    pub fn field_type(&self) -> FieldType {
        match self {
            ValidationData::Null => FieldType::Any,
            ValidationData::Bool(_) => FieldType::Bool,
            ValidationData::Int(_) => FieldType::Int,
            ValidationData::Float(_) => FieldType::Float,
            ValidationData::String(_) => FieldType::String,
            ValidationData::Array(_) => FieldType::Array,
            ValidationData::Object(_) => FieldType::Object,
        }
    }
}

// ============================================================================
// VALIDATION RESULT
// ============================================================================

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Valid
    pub valid: bool,
    /// Violations
    pub violations: Vec<Violation>,
    /// Warnings
    pub warnings: Vec<Violation>,
    /// Info
    pub info: Vec<Violation>,
    /// Validation time (ns)
    pub validation_time_ns: u64,
}

impl ValidationResult {
    /// Create valid result
    #[inline]
    pub fn valid() -> Self {
        Self {
            valid: true,
            violations: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
            validation_time_ns: 0,
        }
    }

    /// Create invalid result
    #[inline]
    pub fn invalid(violations: Vec<Violation>) -> Self {
        Self {
            valid: false,
            violations,
            warnings: Vec::new(),
            info: Vec::new(),
            validation_time_ns: 0,
        }
    }

    /// Add violation
    #[inline(always)]
    pub fn add_violation(&mut self, violation: Violation) {
        self.valid = false;
        self.violations.push(violation);
    }

    /// Add warning
    #[inline(always)]
    pub fn add_warning(&mut self, warning: Violation) {
        self.warnings.push(warning);
    }

    /// Merge with another result
    #[inline]
    pub fn merge(&mut self, other: ValidationResult) {
        if !other.valid {
            self.valid = false;
        }
        self.violations.extend(other.violations);
        self.warnings.extend(other.warnings);
        self.info.extend(other.info);
    }
}

/// Violation
#[derive(Debug, Clone)]
pub struct Violation {
    /// Rule ID
    pub rule_id: u64,
    /// Rule name
    pub rule_name: String,
    /// Field path
    pub field: Option<String>,
    /// Message
    pub message: String,
    /// Severity
    pub severity: RuleSeverity,
    /// Expected value (if applicable)
    pub expected: Option<String>,
    /// Actual value (if applicable)
    pub actual: Option<String>,
}

// ============================================================================
// VALIDATOR
// ============================================================================

/// Cognitive validator
pub struct CognitiveValidator {
    /// Rules
    rules: BTreeMap<u64, ValidationRule>,
    /// Rules by name
    rules_by_name: BTreeMap<String, u64>,
    /// Rule groups
    groups: BTreeMap<String, Vec<u64>>,
    /// Next rule ID
    next_id: AtomicU64,
    /// Configuration
    config: ValidatorConfig,
    /// Statistics
    stats: ValidatorStats,
}

/// Validator configuration
#[derive(Debug, Clone)]
pub struct ValidatorConfig {
    /// Maximum rules
    pub max_rules: usize,
    /// Fail fast (stop on first error)
    pub fail_fast: bool,
    /// Include info messages
    pub include_info: bool,
    /// Maximum violations
    pub max_violations: usize,
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            max_rules: 1000,
            fail_fast: false,
            include_info: true,
            max_violations: 100,
        }
    }
}

/// Validator statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ValidatorStats {
    /// Total validations
    pub total_validations: u64,
    /// Passed validations
    pub passed: u64,
    /// Failed validations
    pub failed: u64,
    /// Total violations
    pub total_violations: u64,
}

impl CognitiveValidator {
    /// Create new validator
    pub fn new(config: ValidatorConfig) -> Self {
        Self {
            rules: BTreeMap::new(),
            rules_by_name: BTreeMap::new(),
            groups: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: ValidatorStats::default(),
        }
    }

    /// Add rule
    pub fn add_rule(
        &mut self,
        name: &str,
        rule_type: RuleType,
        severity: RuleSeverity,
        description: &str,
    ) -> Result<u64, &'static str> {
        if self.rules.len() >= self.config.max_rules {
            return Err("Rule limit exceeded");
        }

        if self.rules_by_name.contains_key(name) {
            return Err("Rule name already exists");
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let rule = ValidationRule {
            id,
            name: name.into(),
            rule_type,
            severity,
            description: description.into(),
            enabled: true,
            tags: Vec::new(),
        };

        self.rules.insert(id, rule);
        self.rules_by_name.insert(name.into(), id);

        Ok(id)
    }

    /// Remove rule
    #[inline]
    pub fn remove_rule(&mut self, id: u64) {
        if let Some(rule) = self.rules.remove(&id) {
            self.rules_by_name.remove(&rule.name);
        }
    }

    /// Enable/disable rule
    #[inline]
    pub fn set_rule_enabled(&mut self, id: u64, enabled: bool) {
        if let Some(rule) = self.rules.get_mut(&id) {
            rule.enabled = enabled;
        }
    }

    /// Add rule to group
    #[inline]
    pub fn add_to_group(&mut self, rule_id: u64, group: &str) {
        self.groups
            .entry(group.into())
            .or_insert_with(Vec::new)
            .push(rule_id);
    }

    /// Validate data against all enabled rules
    pub fn validate(&mut self, data: &ValidationData) -> ValidationResult {
        let start = Timestamp::now();
        self.stats.total_validations += 1;

        let mut result = ValidationResult::valid();

        for rule in self.rules.values() {
            if !rule.enabled {
                continue;
            }

            let rule_result = self.validate_rule(rule, data);
            result.merge(rule_result);

            if self.config.fail_fast && !result.valid {
                break;
            }

            if result.violations.len() >= self.config.max_violations {
                break;
            }
        }

        result.validation_time_ns = Timestamp::now().elapsed_since(start);

        // Update stats
        if result.valid {
            self.stats.passed += 1;
        } else {
            self.stats.failed += 1;
            self.stats.total_violations += result.violations.len() as u64;
        }

        result
    }

    /// Validate data against specific rules
    pub fn validate_with_rules(&mut self, data: &ValidationData, rule_ids: &[u64]) -> ValidationResult {
        let start = Timestamp::now();
        self.stats.total_validations += 1;

        let mut result = ValidationResult::valid();

        for rule_id in rule_ids {
            if let Some(rule) = self.rules.get(rule_id) {
                if !rule.enabled {
                    continue;
                }

                let rule_result = self.validate_rule(rule, data);
                result.merge(rule_result);

                if self.config.fail_fast && !result.valid {
                    break;
                }
            }
        }

        result.validation_time_ns = Timestamp::now().elapsed_since(start);

        if result.valid {
            self.stats.passed += 1;
        } else {
            self.stats.failed += 1;
            self.stats.total_violations += result.violations.len() as u64;
        }

        result
    }

    /// Validate data against a group of rules
    #[inline]
    pub fn validate_group(&mut self, data: &ValidationData, group: &str) -> ValidationResult {
        let rule_ids: Vec<u64> = self.groups.get(group)
            .cloned()
            .unwrap_or_default();

        self.validate_with_rules(data, &rule_ids)
    }

    /// Validate single rule
    fn validate_rule(&self, rule: &ValidationRule, data: &ValidationData) -> ValidationResult {
        match &rule.rule_type {
            RuleType::Schema(schema) => self.validate_schema(rule, schema, data),
            RuleType::Range(range) => self.validate_range(rule, range, data),
            RuleType::Pattern(pattern) => self.validate_pattern(rule, pattern, data),
            RuleType::Constraint(_) => ValidationResult::valid(), // Would need expression evaluation
            RuleType::Custom(_) => ValidationResult::valid(), // Would need handler execution
            RuleType::Composite(composite) => self.validate_composite(rule, composite, data),
        }
    }

    fn validate_schema(&self, rule: &ValidationRule, schema: &SchemaRule, data: &ValidationData) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // Check if data is an object
        let obj = match data {
            ValidationData::Object(o) => o,
            _ => {
                result.add_violation(Violation {
                    rule_id: rule.id,
                    rule_name: rule.name.clone(),
                    field: None,
                    message: "Expected object".into(),
                    severity: rule.severity,
                    expected: Some("object".into()),
                    actual: Some(format!("{:?}", data.field_type())),
                });
                return result;
            }
        };

        // Check required fields
        for field in &schema.required {
            if !obj.contains_key(field) {
                result.add_violation(Violation {
                    rule_id: rule.id,
                    rule_name: rule.name.clone(),
                    field: Some(field.clone()),
                    message: format!("Missing required field: {}", field),
                    severity: rule.severity,
                    expected: Some("present".into()),
                    actual: Some("missing".into()),
                });
            }
        }

        // Check field types
        for (field, expected_type) in &schema.field_types {
            if let Some(value) = obj.get(field) {
                if *expected_type != FieldType::Any && value.field_type() != *expected_type {
                    result.add_violation(Violation {
                        rule_id: rule.id,
                        rule_name: rule.name.clone(),
                        field: Some(field.clone()),
                        message: format!("Invalid type for field: {}", field),
                        severity: rule.severity,
                        expected: Some(format!("{:?}", expected_type)),
                        actual: Some(format!("{:?}", value.field_type())),
                    });
                }
            }
        }

        // Check extra fields
        if !schema.allow_extra {
            for field in obj.keys() {
                if !schema.required.contains(field) && !schema.field_types.contains_key(field) {
                    result.add_warning(Violation {
                        rule_id: rule.id,
                        rule_name: rule.name.clone(),
                        field: Some(field.clone()),
                        message: format!("Unexpected field: {}", field),
                        severity: RuleSeverity::Warning,
                        expected: None,
                        actual: None,
                    });
                }
            }
        }

        result
    }

    fn validate_range(&self, rule: &ValidationRule, range: &RangeRule, data: &ValidationData) -> ValidationResult {
        let mut result = ValidationResult::valid();

        let value = match data.get_field(&range.field) {
            Some(v) => v,
            None => return result, // Field not present
        };

        let num = match value.as_f64() {
            Some(n) => n,
            None => {
                result.add_violation(Violation {
                    rule_id: rule.id,
                    rule_name: rule.name.clone(),
                    field: Some(range.field.clone()),
                    message: "Expected numeric value".into(),
                    severity: rule.severity,
                    expected: Some("number".into()),
                    actual: Some(format!("{:?}", value.field_type())),
                });
                return result;
            }
        };

        if let Some(min) = range.min {
            let valid = if range.inclusive { num >= min } else { num > min };
            if !valid {
                result.add_violation(Violation {
                    rule_id: rule.id,
                    rule_name: rule.name.clone(),
                    field: Some(range.field.clone()),
                    message: format!("Value {} is below minimum {}", num, min),
                    severity: rule.severity,
                    expected: Some(format!(">= {}", min)),
                    actual: Some(format!("{}", num)),
                });
            }
        }

        if let Some(max) = range.max {
            let valid = if range.inclusive { num <= max } else { num < max };
            if !valid {
                result.add_violation(Violation {
                    rule_id: rule.id,
                    rule_name: rule.name.clone(),
                    field: Some(range.field.clone()),
                    message: format!("Value {} is above maximum {}", num, max),
                    severity: rule.severity,
                    expected: Some(format!("<= {}", max)),
                    actual: Some(format!("{}", num)),
                });
            }
        }

        result
    }

    fn validate_pattern(&self, rule: &ValidationRule, pattern: &PatternRule, data: &ValidationData) -> ValidationResult {
        let mut result = ValidationResult::valid();

        let value = match data.get_field(&pattern.field) {
            Some(ValidationData::String(s)) => s,
            Some(_) => {
                result.add_violation(Violation {
                    rule_id: rule.id,
                    rule_name: rule.name.clone(),
                    field: Some(pattern.field.clone()),
                    message: "Expected string value".into(),
                    severity: rule.severity,
                    expected: Some("string".into()),
                    actual: None,
                });
                return result;
            }
            None => return result,
        };

        let matches = match pattern.pattern_type {
            PatternType::Exact => value == &pattern.pattern,
            PatternType::Prefix => value.starts_with(&pattern.pattern),
            PatternType::Suffix => value.ends_with(&pattern.pattern),
            PatternType::Contains => value.contains(&pattern.pattern),
            PatternType::Regex => true, // Would need regex support
        };

        if !matches {
            result.add_violation(Violation {
                rule_id: rule.id,
                rule_name: rule.name.clone(),
                field: Some(pattern.field.clone()),
                message: format!("Value does not match pattern: {}", pattern.pattern),
                severity: rule.severity,
                expected: Some(pattern.pattern.clone()),
                actual: Some(value.clone()),
            });
        }

        result
    }

    fn validate_composite(&self, rule: &ValidationRule, composite: &CompositeRule, data: &ValidationData) -> ValidationResult {
        let child_results: Vec<_> = composite.rules.iter()
            .filter_map(|id| self.rules.get(id))
            .map(|r| self.validate_rule(r, data))
            .collect();

        match composite.operator {
            CompositeOp::And => {
                let mut result = ValidationResult::valid();
                for child in child_results {
                    result.merge(child);
                }
                result
            }
            CompositeOp::Or => {
                if child_results.iter().any(|r| r.valid) {
                    ValidationResult::valid()
                } else {
                    let mut result = ValidationResult::valid();
                    result.add_violation(Violation {
                        rule_id: rule.id,
                        rule_name: rule.name.clone(),
                        field: None,
                        message: "None of the OR conditions were met".into(),
                        severity: rule.severity,
                        expected: None,
                        actual: None,
                    });
                    result
                }
            }
            CompositeOp::Not => {
                let mut result = ValidationResult::valid();
                for child in child_results {
                    if child.valid {
                        result.add_violation(Violation {
                            rule_id: rule.id,
                            rule_name: rule.name.clone(),
                            field: None,
                            message: "NOT condition was unexpectedly true".into(),
                            severity: rule.severity,
                            expected: None,
                            actual: None,
                        });
                    }
                }
                result
            }
        }
    }

    /// Get rule
    #[inline(always)]
    pub fn get_rule(&self, id: u64) -> Option<&ValidationRule> {
        self.rules.get(&id)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &ValidatorStats {
        &self.stats
    }
}

impl Default for CognitiveValidator {
    fn default() -> Self {
        Self::new(ValidatorConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_validation() {
        let mut validator = CognitiveValidator::default();

        let mut field_types = BTreeMap::new();
        field_types.insert("name".into(), FieldType::String);
        field_types.insert("age".into(), FieldType::Int);

        validator.add_rule(
            "user_schema",
            RuleType::Schema(SchemaRule {
                required: vec!["name".into(), "age".into()],
                field_types,
                allow_extra: false,
            }),
            RuleSeverity::Error,
            "User schema validation",
        ).unwrap();

        // Valid data
        let mut data = BTreeMap::new();
        data.insert("name".into(), ValidationData::String("John".into()));
        data.insert("age".into(), ValidationData::Int(30));
        let result = validator.validate(&ValidationData::Object(data));
        assert!(result.valid);

        // Missing field
        let mut data = BTreeMap::new();
        data.insert("name".into(), ValidationData::String("John".into()));
        let result = validator.validate(&ValidationData::Object(data));
        assert!(!result.valid);
    }

    #[test]
    fn test_range_validation() {
        let mut validator = CognitiveValidator::default();

        validator.add_rule(
            "age_range",
            RuleType::Range(RangeRule {
                field: "age".into(),
                min: Some(0.0),
                max: Some(150.0),
                inclusive: true,
            }),
            RuleSeverity::Error,
            "Age must be 0-150",
        ).unwrap();

        let mut data = BTreeMap::new();
        data.insert("age".into(), ValidationData::Int(30));
        let result = validator.validate(&ValidationData::Object(data));
        assert!(result.valid);

        let mut data = BTreeMap::new();
        data.insert("age".into(), ValidationData::Int(200));
        let result = validator.validate(&ValidationData::Object(data));
        assert!(!result.valid);
    }

    #[test]
    fn test_pattern_validation() {
        let mut validator = CognitiveValidator::default();

        validator.add_rule(
            "email_pattern",
            RuleType::Pattern(PatternRule {
                field: "email".into(),
                pattern: "@".into(),
                pattern_type: PatternType::Contains,
            }),
            RuleSeverity::Error,
            "Email must contain @",
        ).unwrap();

        let mut data = BTreeMap::new();
        data.insert("email".into(), ValidationData::String("test@example.com".into()));
        let result = validator.validate(&ValidationData::Object(data));
        assert!(result.valid);

        let mut data = BTreeMap::new();
        data.insert("email".into(), ValidationData::String("invalid".into()));
        let result = validator.validate(&ValidationData::Object(data));
        assert!(!result.valid);
    }
}
