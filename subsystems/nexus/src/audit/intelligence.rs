//! Audit Intelligence
//!
//! Comprehensive security audit analysis engine.

use alloc::string::String;
use alloc::vec::Vec;

use super::{
    AnomalyDetector, AuditEvent, AuditManager, AuditRuleId, ComplianceCheck, RuleAction, RuleList,
};

/// Audit analysis result
#[derive(Debug, Clone)]
pub struct AuditAnalysis {
    /// Health score (0-100)
    pub health_score: f32,
    /// Security score (0-100)
    pub security_score: f32,
    /// Anomalies detected
    pub anomaly_count: usize,
    /// Issues detected
    pub issues: Vec<AuditIssue>,
    /// Recommendations
    pub recommendations: Vec<AuditRecommendation>,
}

/// Audit issue
#[derive(Debug, Clone)]
pub struct AuditIssue {
    /// Issue type
    pub issue_type: AuditIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
}

impl AuditIssue {
    /// Create new issue
    pub fn new(issue_type: AuditIssueType, severity: u8, description: String) -> Self {
        Self {
            issue_type,
            severity,
            description,
        }
    }
}

/// Audit issue types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditIssueType {
    /// No rules configured
    NoRules,
    /// High event rate
    HighEventRate,
    /// Anomalies detected
    AnomaliesDetected,
    /// Compliance violation
    ComplianceViolation,
    /// Log overflow
    LogOverflow,
    /// Missing coverage
    MissingCoverage,
}

/// Audit recommendation
#[derive(Debug, Clone)]
pub struct AuditRecommendation {
    /// Action
    pub action: AuditAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

impl AuditRecommendation {
    /// Create new recommendation
    pub fn new(action: AuditAction, expected_improvement: f32, reason: String) -> Self {
        Self {
            action,
            expected_improvement,
            reason,
        }
    }
}

/// Audit actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditAction {
    /// Add rule
    AddRule,
    /// Increase log size
    IncreaseLogSize,
    /// Enable anomaly detection
    EnableAnomalyDetection,
    /// Review anomalies
    ReviewAnomalies,
    /// Add compliance check
    AddComplianceCheck,
}

/// Audit Intelligence - comprehensive security audit analysis
pub struct AuditIntelligence {
    /// Audit manager
    manager: AuditManager,
    /// Anomaly detector
    detector: AnomalyDetector,
    /// Compliance checks
    compliance_checks: Vec<ComplianceCheck>,
}

impl AuditIntelligence {
    /// Create new audit intelligence
    pub fn new() -> Self {
        Self {
            manager: AuditManager::new(100000),
            detector: AnomalyDetector::new(),
            compliance_checks: Vec::new(),
        }
    }

    /// Create with custom log size
    #[inline]
    pub fn with_log_size(log_size: usize) -> Self {
        Self {
            manager: AuditManager::new(log_size),
            detector: AnomalyDetector::new(),
            compliance_checks: Vec::new(),
        }
    }

    /// Process event
    #[inline(always)]
    pub fn process_event(&mut self, event: AuditEvent, timestamp: u64) {
        self.detector.process_event(&event, timestamp);
        self.manager.process_event(event);
    }

    /// Add rule
    #[inline(always)]
    pub fn add_rule(&mut self, action: RuleAction, list: RuleList, timestamp: u64) -> AuditRuleId {
        self.manager.add_rule(action, list, timestamp)
    }

    /// Add compliance check
    #[inline(always)]
    pub fn add_compliance_check(&mut self, check: ComplianceCheck) {
        self.compliance_checks.push(check);
    }

    /// Analyze security posture
    pub fn analyze(&self) -> AuditAnalysis {
        let mut health_score = 100.0f32;
        let mut security_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Check rules
        if !self.manager.has_rules() {
            health_score -= 20.0;
            security_score -= 30.0;
            issues.push(AuditIssue {
                issue_type: AuditIssueType::NoRules,
                severity: 7,
                description: String::from("No audit rules configured"),
            });
            recommendations.push(AuditRecommendation {
                action: AuditAction::AddRule,
                expected_improvement: 30.0,
                reason: String::from("Add rules to monitor security events"),
            });
        }

        // Check anomalies
        let anomaly_count = self.detector.total_anomalies();
        if anomaly_count > 0 {
            security_score -= (anomaly_count as f32).min(40.0);
            issues.push(AuditIssue {
                issue_type: AuditIssueType::AnomaliesDetected,
                severity: 8,
                description: alloc::format!("{} anomalies detected", anomaly_count),
            });
            recommendations.push(AuditRecommendation {
                action: AuditAction::ReviewAnomalies,
                expected_improvement: 20.0,
                reason: String::from("Review and investigate anomalies"),
            });
        }

        // Check log overflow
        if self.manager.log().events_dropped() > 0 {
            health_score -= 15.0;
            issues.push(AuditIssue {
                issue_type: AuditIssueType::LogOverflow,
                severity: 5,
                description: alloc::format!(
                    "{} events dropped",
                    self.manager.log().events_dropped()
                ),
            });
            recommendations.push(AuditRecommendation {
                action: AuditAction::IncreaseLogSize,
                expected_improvement: 10.0,
                reason: String::from("Increase log size to prevent event loss"),
            });
        }

        // Check compliance
        for check in &self.compliance_checks {
            if !check.passing {
                security_score -= 10.0;
                issues.push(AuditIssue {
                    issue_type: AuditIssueType::ComplianceViolation,
                    severity: 6,
                    description: alloc::format!(
                        "Compliance check {} failed: {}",
                        check.id,
                        check.description
                    ),
                });
            }
        }

        health_score = health_score.max(0.0);
        security_score = security_score.max(0.0);

        AuditAnalysis {
            health_score,
            security_score,
            anomaly_count,
            issues,
            recommendations,
        }
    }

    /// Get manager
    #[inline(always)]
    pub fn manager(&self) -> &AuditManager {
        &self.manager
    }

    /// Get manager mutably
    #[inline(always)]
    pub fn manager_mut(&mut self) -> &mut AuditManager {
        &mut self.manager
    }

    /// Get detector
    #[inline(always)]
    pub fn detector(&self) -> &AnomalyDetector {
        &self.detector
    }

    /// Get detector mutably
    #[inline(always)]
    pub fn detector_mut(&mut self) -> &mut AnomalyDetector {
        &mut self.detector
    }

    /// Get compliance checks
    #[inline(always)]
    pub fn compliance_checks(&self) -> &[ComplianceCheck] {
        &self.compliance_checks
    }
}

impl Default for AuditIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
