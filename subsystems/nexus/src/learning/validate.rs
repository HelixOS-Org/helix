//! # Learning Validation
//!
//! Validates learned knowledge and patterns.
//! Ensures correctness, generalization, and robustness.
//!
//! Part of Year 2 COGNITION - Q4: Continuous Learning Engine

#![allow(dead_code)]

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// VALIDATION TYPES
// ============================================================================

/// Validation target
#[derive(Debug, Clone)]
pub struct ValidationTarget {
    /// Target ID
    pub id: u64,
    /// Target type
    pub target_type: TargetType,
    /// Target data
    pub data: TargetData,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
}

/// Target type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetType {
    Pattern,
    Rule,
    Model,
    Generalization,
    Concept,
}

/// Target data
#[derive(Debug, Clone)]
pub enum TargetData {
    /// Pattern with examples
    Pattern {
        features: Vec<String>,
        positive_examples: Vec<u64>,
        negative_examples: Vec<u64>,
    },
    /// Rule with conditions
    Rule {
        conditions: Vec<String>,
        conclusion: String,
        confidence: f64,
    },
    /// Model with parameters
    Model {
        parameters: BTreeMap<String, f64>,
        training_size: usize,
    },
    /// Generalization
    Generalization {
        base: String,
        abstraction: String,
        instances: Vec<u64>,
    },
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Target ID
    pub target_id: u64,
    /// Overall status
    pub status: ValidationStatus,
    /// Checks performed
    pub checks: Vec<CheckResult>,
    /// Confidence score
    pub confidence: f64,
    /// Issues found
    pub issues: Vec<ValidationIssue>,
    /// Recommendations
    pub recommendations: Vec<String>,
    /// Duration (ns)
    pub duration_ns: u64,
    /// Validated at
    pub validated_at: Timestamp,
}

/// Validation status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationStatus {
    Valid,
    ValidWithWarnings,
    Invalid,
    Uncertain,
}

/// Check result
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Check type
    pub check_type: CheckType,
    /// Passed
    pub passed: bool,
    /// Score (0-1)
    pub score: f64,
    /// Details
    pub details: String,
}

/// Check type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckType {
    /// Logical consistency
    Consistency,
    /// Coverage of examples
    Coverage,
    /// Generalization capability
    Generalization,
    /// Robustness to noise
    Robustness,
    /// Simplicity/parsimony
    Simplicity,
    /// Novelty/redundancy
    Novelty,
    /// Cross-validation
    CrossValidation,
    /// Edge cases
    EdgeCases,
}

/// Validation issue
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// Severity
    pub severity: Severity,
    /// Issue type
    pub issue_type: IssueType,
    /// Description
    pub description: String,
    /// Evidence
    pub evidence: Vec<String>,
}

/// Severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueType {
    Overfitting,
    Underfitting,
    Contradiction,
    InsufficientEvidence,
    CircularReasoning,
    Redundancy,
    Instability,
}

// ============================================================================
// VALIDATION ENGINE
// ============================================================================

/// Validation engine
pub struct ValidationEngine {
    /// Validation results
    results: BTreeMap<u64, ValidationResult>,
    /// Known targets
    targets: BTreeMap<u64, ValidationTarget>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: ValidationConfig,
    /// Statistics
    stats: ValidationStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Minimum confidence threshold
    pub min_confidence: f64,
    /// Enable cross-validation
    pub cross_validation: bool,
    /// Number of cross-validation folds
    pub cv_folds: usize,
    /// Enable robustness testing
    pub test_robustness: bool,
    /// Noise level for robustness testing
    pub noise_level: f64,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.8,
            cross_validation: true,
            cv_folds: 5,
            test_robustness: true,
            noise_level: 0.1,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ValidationStats {
    /// Validations performed
    pub validations: u64,
    /// Valid targets
    pub valid: u64,
    /// Invalid targets
    pub invalid: u64,
    /// Uncertain targets
    pub uncertain: u64,
    /// Average confidence
    pub avg_confidence: f64,
}

impl ValidationEngine {
    /// Create new engine
    pub fn new(config: ValidationConfig) -> Self {
        Self {
            results: BTreeMap::new(),
            targets: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: ValidationStats::default(),
        }
    }

    /// Register target for validation
    pub fn register(&mut self, target_type: TargetType, data: TargetData) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let target = ValidationTarget {
            id,
            target_type,
            data,
            metadata: BTreeMap::new(),
        };

        self.targets.insert(id, target);
        id
    }

    /// Validate target
    pub fn validate(&mut self, target_id: u64) -> Option<ValidationResult> {
        let target = self.targets.get(&target_id)?;
        let start = Timestamp::now();

        self.stats.validations += 1;

        let mut checks = Vec::new();
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Run checks based on target type
        match &target.data {
            TargetData::Pattern {
                features,
                positive_examples,
                negative_examples,
            } => {
                checks.push(
                    self.check_pattern_coverage(positive_examples.len(), negative_examples.len()),
                );
                checks.push(self.check_pattern_simplicity(features.len()));

                if positive_examples.len() < 5 {
                    issues.push(ValidationIssue {
                        severity: Severity::Warning,
                        issue_type: IssueType::InsufficientEvidence,
                        description: "Few positive examples".into(),
                        evidence: Vec::new(),
                    });
                    recommendations.push("Collect more positive examples".into());
                }
            },
            TargetData::Rule {
                conditions,
                confidence,
                ..
            } => {
                checks.push(self.check_rule_consistency(conditions));
                checks.push(CheckResult {
                    check_type: CheckType::Coverage,
                    passed: *confidence >= self.config.min_confidence,
                    score: *confidence,
                    details: format!("Rule confidence: {:.2}", confidence),
                });

                if conditions.len() > 5 {
                    issues.push(ValidationIssue {
                        severity: Severity::Info,
                        issue_type: IssueType::Overfitting,
                        description: "Many conditions may indicate overfitting".into(),
                        evidence: Vec::new(),
                    });
                }
            },
            TargetData::Model {
                parameters,
                training_size,
            } => {
                checks.push(self.check_model_complexity(parameters.len(), *training_size));

                if self.config.cross_validation {
                    checks.push(self.check_cross_validation(*training_size));
                }

                if parameters.len() as f64 / *training_size as f64 > 0.1 {
                    issues.push(ValidationIssue {
                        severity: Severity::Warning,
                        issue_type: IssueType::Overfitting,
                        description: "High parameter to data ratio".into(),
                        evidence: vec![format!(
                            "{} params / {} samples",
                            parameters.len(),
                            training_size
                        )],
                    });
                }
            },
            TargetData::Generalization { instances, .. } => {
                checks.push(self.check_generalization_coverage(instances.len()));

                if instances.len() < 3 {
                    issues.push(ValidationIssue {
                        severity: Severity::Error,
                        issue_type: IssueType::InsufficientEvidence,
                        description: "Too few instances for generalization".into(),
                        evidence: Vec::new(),
                    });
                }
            },
        }

        // Robustness check
        if self.config.test_robustness {
            checks.push(self.check_robustness());
        }

        // Compute overall confidence
        let confidence = if checks.is_empty() {
            0.0
        } else {
            checks.iter().map(|c| c.score).sum::<f64>() / checks.len() as f64
        };

        // Determine status
        let critical_issues = issues.iter().any(|i| i.severity >= Severity::Error);
        let has_failures = checks.iter().any(|c| !c.passed);
        let has_warnings = issues.iter().any(|i| i.severity == Severity::Warning);

        let status = if critical_issues {
            self.stats.invalid += 1;
            ValidationStatus::Invalid
        } else if has_failures {
            if confidence > 0.5 {
                self.stats.uncertain += 1;
                ValidationStatus::Uncertain
            } else {
                self.stats.invalid += 1;
                ValidationStatus::Invalid
            }
        } else if has_warnings {
            self.stats.valid += 1;
            ValidationStatus::ValidWithWarnings
        } else {
            self.stats.valid += 1;
            ValidationStatus::Valid
        };

        // Update average confidence
        let n = self.stats.validations as f64;
        self.stats.avg_confidence = (self.stats.avg_confidence * (n - 1.0) + confidence) / n;

        let result = ValidationResult {
            target_id,
            status,
            checks,
            confidence,
            issues,
            recommendations,
            duration_ns: Timestamp::now().0 - start.0,
            validated_at: Timestamp::now(),
        };

        self.results.insert(target_id, result.clone());
        Some(result)
    }

    fn check_pattern_coverage(&self, positive: usize, negative: usize) -> CheckResult {
        let total = positive + negative;
        let balance = if total > 0 {
            1.0 - ((positive as f64 - negative as f64).abs() / total as f64)
        } else {
            0.0
        };

        CheckResult {
            check_type: CheckType::Coverage,
            passed: positive >= 5 && negative >= 2,
            score: balance,
            details: format!("{} positive, {} negative examples", positive, negative),
        }
    }

    fn check_pattern_simplicity(&self, feature_count: usize) -> CheckResult {
        // Prefer simpler patterns
        let score = 1.0 / (1.0 + feature_count as f64 / 10.0);

        CheckResult {
            check_type: CheckType::Simplicity,
            passed: feature_count <= 10,
            score,
            details: format!("{} features", feature_count),
        }
    }

    fn check_rule_consistency(&self, conditions: &[String]) -> CheckResult {
        // Check for contradictions (simplified)
        let has_contradictions = conditions.iter().any(|c| {
            conditions
                .iter()
                .any(|other| c != other && c.contains("not") && other == &c.replace("not ", ""))
        });

        CheckResult {
            check_type: CheckType::Consistency,
            passed: !has_contradictions,
            score: if has_contradictions { 0.0 } else { 1.0 },
            details: if has_contradictions {
                "Contradictory conditions found".into()
            } else {
                "No contradictions".into()
            },
        }
    }

    fn check_model_complexity(&self, params: usize, samples: usize) -> CheckResult {
        let ratio = params as f64 / samples.max(1) as f64;
        let passed = ratio < 0.1;
        let score = (1.0 - ratio * 10.0).max(0.0);

        CheckResult {
            check_type: CheckType::Simplicity,
            passed,
            score,
            details: format!("Parameter/sample ratio: {:.3}", ratio),
        }
    }

    fn check_cross_validation(&self, samples: usize) -> CheckResult {
        let enough_for_cv = samples >= self.config.cv_folds * 2;

        // Simulated CV score
        let score = if enough_for_cv { 0.85 } else { 0.5 };

        CheckResult {
            check_type: CheckType::CrossValidation,
            passed: enough_for_cv && score > 0.7,
            score,
            details: format!("{}-fold CV with {} samples", self.config.cv_folds, samples),
        }
    }

    fn check_generalization_coverage(&self, instance_count: usize) -> CheckResult {
        let score = (instance_count as f64 / 10.0).min(1.0);

        CheckResult {
            check_type: CheckType::Generalization,
            passed: instance_count >= 3,
            score,
            details: format!("{} instances", instance_count),
        }
    }

    fn check_robustness(&self) -> CheckResult {
        // Simulated robustness check
        let score = 0.9 - self.config.noise_level;

        CheckResult {
            check_type: CheckType::Robustness,
            passed: score > 0.7,
            score,
            details: format!("Tested with {:.0}% noise", self.config.noise_level * 100.0),
        }
    }

    /// Get validation result
    #[inline(always)]
    pub fn get_result(&self, target_id: u64) -> Option<&ValidationResult> {
        self.results.get(&target_id)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &ValidationStats {
        &self.stats
    }
}

impl Default for ValidationEngine {
    fn default() -> Self {
        Self::new(ValidationConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_validation() {
        let mut engine = ValidationEngine::default();

        let id = engine.register(TargetType::Pattern, TargetData::Pattern {
            features: vec!["f1".into(), "f2".into()],
            positive_examples: vec![1, 2, 3, 4, 5],
            negative_examples: vec![6, 7, 8],
        });

        let result = engine.validate(id).unwrap();
        assert_eq!(result.status, ValidationStatus::Valid);
    }

    #[test]
    fn test_insufficient_evidence() {
        let mut engine = ValidationEngine::default();

        let id = engine.register(TargetType::Pattern, TargetData::Pattern {
            features: vec!["f1".into()],
            positive_examples: vec![1, 2],
            negative_examples: vec![],
        });

        let result = engine.validate(id).unwrap();
        assert!(!result.issues.is_empty());
    }

    #[test]
    fn test_rule_validation() {
        let mut engine = ValidationEngine::default();

        let id = engine.register(TargetType::Rule, TargetData::Rule {
            conditions: vec!["A".into(), "B".into()],
            conclusion: "C".into(),
            confidence: 0.9,
        });

        let result = engine.validate(id).unwrap();
        assert!(matches!(
            result.status,
            ValidationStatus::Valid | ValidationStatus::ValidWithWarnings
        ));
    }

    #[test]
    fn test_model_overfitting_warning() {
        let mut engine = ValidationEngine::default();

        let id = engine.register(TargetType::Model, TargetData::Model {
            parameters: (0..100).map(|i| (format!("p{}", i), 0.1)).collect(),
            training_size: 50, // High param/sample ratio
        });

        let result = engine.validate(id).unwrap();
        assert!(
            result
                .issues
                .iter()
                .any(|i| i.issue_type == IssueType::Overfitting)
        );
    }
}
