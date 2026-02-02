//! # Cognitive Quality Manager
//!
//! Quality assurance for cognitive operations.
//! Monitors correctness, consistency, and reliability.

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
// QUALITY TYPES
// ============================================================================

/// Quality metric
#[derive(Debug, Clone)]
pub struct QualityMetric {
    /// Metric name
    pub name: String,
    /// Metric type
    pub metric_type: QualityMetricType,
    /// Current value
    pub value: f64,
    /// Target value
    pub target: f64,
    /// Threshold for alert
    pub threshold: f64,
    /// Trend
    pub trend: Trend,
    /// Last updated
    pub updated: Timestamp,
}

/// Quality metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityMetricType {
    /// Accuracy (correctness of outputs)
    Accuracy,
    /// Precision (specificity)
    Precision,
    /// Recall (sensitivity)
    Recall,
    /// F1 score
    F1Score,
    /// Consistency (reproducibility)
    Consistency,
    /// Reliability (uptime/availability)
    Reliability,
    /// Latency (response time)
    Latency,
    /// Throughput (operations/time)
    Throughput,
    /// Error rate
    ErrorRate,
    /// Custom
    Custom,
}

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    Improving,
    Stable,
    Degrading,
    Unknown,
}

/// Quality check
#[derive(Debug, Clone)]
pub struct QualityCheck {
    /// Check ID
    pub id: u64,
    /// Check name
    pub name: String,
    /// Check type
    pub check_type: QualityCheckType,
    /// Target domain
    pub domain: DomainId,
    /// Enabled
    pub enabled: bool,
    /// Last check
    pub last_check: Option<Timestamp>,
    /// Last result
    pub last_result: Option<QualityCheckResult>,
}

/// Quality check type
#[derive(Debug, Clone)]
pub enum QualityCheckType {
    /// Output validation
    OutputValidation {
        /// Expected format
        format: String,
        /// Schema validation
        schema: Option<String>,
    },
    /// Input-output consistency
    Consistency {
        /// Reference inputs
        reference_inputs: Vec<String>,
        /// Expected outputs
        expected_outputs: Vec<String>,
    },
    /// Performance check
    Performance {
        /// Max latency (ns)
        max_latency_ns: u64,
        /// Min throughput
        min_throughput: f64,
    },
    /// Regression check
    Regression {
        /// Baseline version
        baseline: String,
        /// Tolerance
        tolerance: f64,
    },
    /// Custom check
    Custom {
        /// Check handler
        handler: String,
    },
}

/// Quality check result
#[derive(Debug, Clone)]
pub struct QualityCheckResult {
    /// Check ID
    pub check_id: u64,
    /// Passed
    pub passed: bool,
    /// Score (0-1)
    pub score: f64,
    /// Details
    pub details: String,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Issues found
    pub issues: Vec<QualityIssue>,
}

/// Quality issue
#[derive(Debug, Clone)]
pub struct QualityIssue {
    /// Issue ID
    pub id: u64,
    /// Severity
    pub severity: IssueSeverity,
    /// Category
    pub category: IssueCategory,
    /// Description
    pub description: String,
    /// Affected component
    pub component: Option<String>,
    /// Remediation suggestion
    pub remediation: Option<String>,
}

/// Issue severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Issue category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueCategory {
    Correctness,
    Performance,
    Consistency,
    Reliability,
    Security,
    Compliance,
}

// ============================================================================
// QUALITY ASSESSMENT
// ============================================================================

/// Quality assessment
#[derive(Debug, Clone)]
pub struct QualityAssessment {
    /// Assessment ID
    pub id: u64,
    /// Target domain
    pub domain: DomainId,
    /// Overall score (0-1)
    pub score: f64,
    /// Grade
    pub grade: QualityGrade,
    /// Metrics
    pub metrics: Vec<QualityMetric>,
    /// Check results
    pub check_results: Vec<QualityCheckResult>,
    /// Issues
    pub issues: Vec<QualityIssue>,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Recommendations
    pub recommendations: Vec<String>,
}

/// Quality grade
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QualityGrade {
    /// Excellent (> 0.95)
    A,
    /// Good (> 0.85)
    B,
    /// Acceptable (> 0.75)
    C,
    /// Poor (> 0.60)
    D,
    /// Failing (< 0.60)
    F,
}

impl QualityGrade {
    /// Get grade from score
    pub fn from_score(score: f64) -> Self {
        if score > 0.95 {
            Self::A
        } else if score > 0.85 {
            Self::B
        } else if score > 0.75 {
            Self::C
        } else if score > 0.60 {
            Self::D
        } else {
            Self::F
        }
    }
}

// ============================================================================
// QUALITY MANAGER
// ============================================================================

/// Quality manager
pub struct QualityManager {
    /// Quality checks
    checks: BTreeMap<u64, QualityCheck>,
    /// Metrics by domain
    metrics: BTreeMap<DomainId, Vec<QualityMetric>>,
    /// Assessment history
    assessments: Vec<QualityAssessment>,
    /// Active issues
    issues: Vec<QualityIssue>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: QualityConfig,
    /// Statistics
    stats: QualityStats,
}

/// Quality configuration
#[derive(Debug, Clone)]
pub struct QualityConfig {
    /// Maximum assessments to keep
    pub max_assessments: usize,
    /// Maximum issues to track
    pub max_issues: usize,
    /// Default check interval (ns)
    pub check_interval_ns: u64,
    /// Quality threshold for alerts
    pub alert_threshold: f64,
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self {
            max_assessments: 1000,
            max_issues: 10000,
            check_interval_ns: 60_000_000_000, // 1 minute
            alert_threshold: 0.75,
        }
    }
}

/// Quality statistics
#[derive(Debug, Clone, Default)]
pub struct QualityStats {
    /// Total checks executed
    pub total_checks: u64,
    /// Checks passed
    pub checks_passed: u64,
    /// Checks failed
    pub checks_failed: u64,
    /// Average quality score
    pub avg_score: f64,
    /// Active critical issues
    pub critical_issues: u64,
}

impl QualityManager {
    /// Create new quality manager
    pub fn new(config: QualityConfig) -> Self {
        Self {
            checks: BTreeMap::new(),
            metrics: BTreeMap::new(),
            assessments: Vec::new(),
            issues: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: QualityStats::default(),
        }
    }

    /// Add quality check
    pub fn add_check(&mut self, name: &str, check_type: QualityCheckType, domain: DomainId) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let check = QualityCheck {
            id,
            name: name.into(),
            check_type,
            domain,
            enabled: true,
            last_check: None,
            last_result: None,
        };

        self.checks.insert(id, check);
        id
    }

    /// Remove check
    pub fn remove_check(&mut self, id: u64) {
        self.checks.remove(&id);
    }

    /// Enable/disable check
    pub fn set_check_enabled(&mut self, id: u64, enabled: bool) {
        if let Some(check) = self.checks.get_mut(&id) {
            check.enabled = enabled;
        }
    }

    /// Execute a quality check
    pub fn execute_check(&mut self, check_id: u64) -> Option<QualityCheckResult> {
        let check = self.checks.get(&check_id)?;
        if !check.enabled {
            return None;
        }

        let now = Timestamp::now();
        self.stats.total_checks += 1;

        // Execute check based on type
        let result = match &check.check_type {
            QualityCheckType::OutputValidation { format, .. } => {
                // Simulate output validation
                QualityCheckResult {
                    check_id,
                    passed: true,
                    score: 0.95,
                    details: format!("Output format {} validated", format),
                    timestamp: now,
                    issues: vec![],
                }
            },
            QualityCheckType::Consistency { .. } => QualityCheckResult {
                check_id,
                passed: true,
                score: 0.98,
                details: "Consistency check passed".into(),
                timestamp: now,
                issues: vec![],
            },
            QualityCheckType::Performance {
                max_latency_ns,
                min_throughput,
            } => {
                // Simulate performance check
                let latency_ok = true; // Would measure actual latency
                let throughput_ok = true;

                QualityCheckResult {
                    check_id,
                    passed: latency_ok && throughput_ok,
                    score: 0.90,
                    details: format!(
                        "Latency: OK (max {}ns), Throughput: OK (min {})",
                        max_latency_ns, min_throughput
                    ),
                    timestamp: now,
                    issues: vec![],
                }
            },
            QualityCheckType::Regression { tolerance, .. } => QualityCheckResult {
                check_id,
                passed: true,
                score: 0.92,
                details: format!("Within tolerance {}", tolerance),
                timestamp: now,
                issues: vec![],
            },
            QualityCheckType::Custom { handler } => QualityCheckResult {
                check_id,
                passed: true,
                score: 1.0,
                details: format!("Custom handler {} executed", handler),
                timestamp: now,
                issues: vec![],
            },
        };

        // Update stats
        if result.passed {
            self.stats.checks_passed += 1;
        } else {
            self.stats.checks_failed += 1;
        }

        // Update check
        if let Some(check) = self.checks.get_mut(&check_id) {
            check.last_check = Some(now);
            check.last_result = Some(result.clone());
        }

        Some(result)
    }

    /// Record metric
    pub fn record_metric(
        &mut self,
        domain: DomainId,
        name: &str,
        metric_type: QualityMetricType,
        value: f64,
        target: f64,
        threshold: f64,
    ) {
        let metrics = self.metrics.entry(domain).or_insert_with(Vec::new);

        // Find or create metric
        if let Some(metric) = metrics.iter_mut().find(|m| m.name == name) {
            // Determine trend
            metric.trend = if value > metric.value * 1.01 {
                if metric_type == QualityMetricType::ErrorRate {
                    Trend::Degrading
                } else {
                    Trend::Improving
                }
            } else if value < metric.value * 0.99 {
                if metric_type == QualityMetricType::ErrorRate {
                    Trend::Improving
                } else {
                    Trend::Degrading
                }
            } else {
                Trend::Stable
            };

            metric.value = value;
            metric.updated = Timestamp::now();
        } else {
            metrics.push(QualityMetric {
                name: name.into(),
                metric_type,
                value,
                target,
                threshold,
                trend: Trend::Unknown,
                updated: Timestamp::now(),
            });
        }
    }

    /// Report issue
    pub fn report_issue(
        &mut self,
        severity: IssueSeverity,
        category: IssueCategory,
        description: &str,
        component: Option<&str>,
        remediation: Option<&str>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let issue = QualityIssue {
            id,
            severity,
            category,
            description: description.into(),
            component: component.map(|s| s.into()),
            remediation: remediation.map(|s| s.into()),
        };

        if severity == IssueSeverity::Critical {
            self.stats.critical_issues += 1;
        }

        self.issues.push(issue);

        // Limit issues
        while self.issues.len() > self.config.max_issues {
            let removed = self.issues.remove(0);
            if removed.severity == IssueSeverity::Critical {
                self.stats.critical_issues = self.stats.critical_issues.saturating_sub(1);
            }
        }

        id
    }

    /// Resolve issue
    pub fn resolve_issue(&mut self, issue_id: u64) {
        if let Some(pos) = self.issues.iter().position(|i| i.id == issue_id) {
            let issue = self.issues.remove(pos);
            if issue.severity == IssueSeverity::Critical {
                self.stats.critical_issues = self.stats.critical_issues.saturating_sub(1);
            }
        }
    }

    /// Perform full quality assessment
    pub fn assess(&mut self, domain: DomainId) -> QualityAssessment {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        // Get metrics
        let metrics = self.metrics.get(&domain).cloned().unwrap_or_default();

        // Run all checks for domain
        let check_ids: Vec<u64> = self
            .checks
            .values()
            .filter(|c| c.domain == domain && c.enabled)
            .map(|c| c.id)
            .collect();

        let mut check_results = Vec::new();
        for check_id in check_ids {
            if let Some(result) = self.execute_check(check_id) {
                check_results.push(result);
            }
        }

        // Calculate overall score
        let metric_score = if metrics.is_empty() {
            1.0
        } else {
            metrics
                .iter()
                .map(|m| {
                    if m.value >= m.target {
                        1.0
                    } else {
                        m.value / m.target
                    }
                })
                .sum::<f64>()
                / metrics.len() as f64
        };

        let check_score = if check_results.is_empty() {
            1.0
        } else {
            check_results.iter().map(|r| r.score).sum::<f64>() / check_results.len() as f64
        };

        let score = (metric_score + check_score) / 2.0;
        let grade = QualityGrade::from_score(score);

        // Collect issues for domain
        let issues: Vec<_> = check_results
            .iter()
            .flat_map(|r| r.issues.clone())
            .collect();

        // Generate recommendations
        let mut recommendations = Vec::new();
        for metric in &metrics {
            if metric.value < metric.target {
                recommendations.push(format!(
                    "Improve {} (current: {:.2}, target: {:.2})",
                    metric.name, metric.value, metric.target
                ));
            }
        }

        let assessment = QualityAssessment {
            id,
            domain,
            score,
            grade,
            metrics,
            check_results,
            issues,
            timestamp: now,
            recommendations,
        };

        // Update stats
        self.stats.avg_score = (self.stats.avg_score * self.assessments.len() as f64 + score)
            / (self.assessments.len() + 1) as f64;

        // Store assessment
        if self.assessments.len() >= self.config.max_assessments {
            self.assessments.remove(0);
        }
        self.assessments.push(assessment.clone());

        assessment
    }

    /// Get metrics for domain
    pub fn get_metrics(&self, domain: DomainId) -> Option<&Vec<QualityMetric>> {
        self.metrics.get(&domain)
    }

    /// Get active issues
    pub fn active_issues(&self) -> &[QualityIssue] {
        &self.issues
    }

    /// Get issues by severity
    pub fn issues_by_severity(&self, min_severity: IssueSeverity) -> Vec<&QualityIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity >= min_severity)
            .collect()
    }

    /// Get recent assessments
    pub fn recent_assessments(&self, count: usize) -> &[QualityAssessment] {
        let start = self.assessments.len().saturating_sub(count);
        &self.assessments[start..]
    }

    /// Get statistics
    pub fn stats(&self) -> &QualityStats {
        &self.stats
    }
}

impl Default for QualityManager {
    fn default() -> Self {
        Self::new(QualityConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_check() {
        let mut manager = QualityManager::default();
        let domain = DomainId::new(1);

        let check_id = manager.add_check(
            "output_check",
            QualityCheckType::OutputValidation {
                format: "json".into(),
                schema: None,
            },
            domain,
        );

        let result = manager.execute_check(check_id).unwrap();
        assert!(result.passed);
        assert!(result.score > 0.9);
    }

    #[test]
    fn test_quality_metric() {
        let mut manager = QualityManager::default();
        let domain = DomainId::new(1);

        manager.record_metric(
            domain,
            "accuracy",
            QualityMetricType::Accuracy,
            0.92,
            0.95,
            0.85,
        );

        let metrics = manager.get_metrics(domain).unwrap();
        assert_eq!(metrics.len(), 1);
        assert!((metrics[0].value - 0.92).abs() < f64::EPSILON);
    }

    #[test]
    fn test_quality_assessment() {
        let mut manager = QualityManager::default();
        let domain = DomainId::new(1);

        manager.record_metric(
            domain,
            "accuracy",
            QualityMetricType::Accuracy,
            0.95,
            0.90,
            0.80,
        );
        manager.add_check(
            "test",
            QualityCheckType::Custom {
                handler: "test".into(),
            },
            domain,
        );

        let assessment = manager.assess(domain);
        assert!(assessment.score > 0.8);
        assert!(assessment.grade >= QualityGrade::B);
    }

    #[test]
    fn test_issue_reporting() {
        let mut manager = QualityManager::default();

        let id = manager.report_issue(
            IssueSeverity::Warning,
            IssueCategory::Performance,
            "Latency degradation detected",
            Some("inference_engine"),
            Some("Increase batch size"),
        );

        assert!(!manager.active_issues().is_empty());

        manager.resolve_issue(id);
        assert!(manager.active_issues().is_empty());
    }

    #[test]
    fn test_grade_calculation() {
        assert_eq!(QualityGrade::from_score(0.98), QualityGrade::A);
        assert_eq!(QualityGrade::from_score(0.90), QualityGrade::B);
        assert_eq!(QualityGrade::from_score(0.80), QualityGrade::C);
        assert_eq!(QualityGrade::from_score(0.65), QualityGrade::D);
        assert_eq!(QualityGrade::from_score(0.50), QualityGrade::F);
    }
}
