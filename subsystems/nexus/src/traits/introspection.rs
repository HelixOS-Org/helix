//! Introspection and Reflection Traits
//!
//! Traits for the REFLECT domain - self-monitoring and meta-cognition.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

use crate::types::{ComponentId, Metric, Severity, Timestamp};
use super::component::{ComponentStatus, NexusComponent};

// ============================================================================
// INTROSPECTABLE TRAIT
// ============================================================================

/// Trait for introspectable components
pub trait Introspectable: NexusComponent {
    /// Get internal state summary
    fn introspect(&self) -> IntrospectionReport;

    /// Get health metrics
    fn health_metrics(&self) -> Vec<Metric>;

    /// Get performance metrics
    fn performance_metrics(&self) -> Vec<Metric>;

    /// Self-diagnose
    fn diagnose(&self) -> DiagnosisReport;
}

// ============================================================================
// INTROSPECTION REPORT
// ============================================================================

/// Introspection report
#[derive(Debug, Clone)]
pub struct IntrospectionReport {
    /// Component ID
    pub component: ComponentId,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Status
    pub status: ComponentStatus,
    /// Health score (0-100)
    pub health_score: u8,
    /// Active tasks
    pub active_tasks: u32,
    /// Memory usage (bytes)
    pub memory_bytes: u64,
    /// CPU usage (0-100)
    pub cpu_percent: f32,
    /// Custom metrics
    pub metrics: Vec<Metric>,
    /// Issues detected
    pub issues: Vec<String>,
}

impl IntrospectionReport {
    /// Is healthy?
    pub fn is_healthy(&self) -> bool {
        self.health_score >= 70 && self.issues.is_empty()
    }

    /// Create basic report
    pub fn new(component: ComponentId, status: ComponentStatus) -> Self {
        Self {
            component,
            timestamp: Timestamp::now(),
            status,
            health_score: 100,
            active_tasks: 0,
            memory_bytes: 0,
            cpu_percent: 0.0,
            metrics: Vec::new(),
            issues: Vec::new(),
        }
    }

    /// With health score
    pub fn with_health(mut self, score: u8) -> Self {
        self.health_score = score.min(100);
        self
    }

    /// Add issue
    pub fn with_issue(mut self, issue: impl Into<String>) -> Self {
        self.issues.push(issue.into());
        self
    }
}

// ============================================================================
// DIAGNOSIS REPORT
// ============================================================================

/// Diagnosis report
#[derive(Debug, Clone)]
pub struct DiagnosisReport {
    /// Component ID
    pub component: ComponentId,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Overall health status
    pub health: HealthStatus,
    /// Findings
    pub findings: Vec<DiagnosisFinding>,
    /// Recommendations
    pub recommendations: Vec<String>,
}

impl DiagnosisReport {
    /// Create new report
    pub fn new(component: ComponentId, health: HealthStatus) -> Self {
        Self {
            component,
            timestamp: Timestamp::now(),
            health,
            findings: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    /// Add finding
    pub fn with_finding(mut self, finding: DiagnosisFinding) -> Self {
        self.findings.push(finding);
        self
    }

    /// Add recommendation
    pub fn with_recommendation(mut self, rec: impl Into<String>) -> Self {
        self.recommendations.push(rec.into());
        self
    }

    /// Is healthy?
    pub fn is_healthy(&self) -> bool {
        matches!(self.health, HealthStatus::Healthy)
    }

    /// Has critical issues?
    pub fn has_critical(&self) -> bool {
        self.findings.iter().any(|f| f.severity.value() >= 9)
    }
}

// ============================================================================
// HEALTH STATUS
// ============================================================================

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HealthStatus {
    /// Fully healthy
    Healthy,
    /// Minor issues
    Warning,
    /// Degraded performance
    Degraded,
    /// Critical issues
    Critical,
    /// Unknown (can't determine)
    Unknown,
}

impl HealthStatus {
    /// Get status name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Warning => "warning",
            Self::Degraded => "degraded",
            Self::Critical => "critical",
            Self::Unknown => "unknown",
        }
    }

    /// Is operational?
    pub const fn is_operational(&self) -> bool {
        matches!(self, Self::Healthy | Self::Warning | Self::Degraded)
    }

    /// From health score
    pub fn from_score(score: u8) -> Self {
        match score {
            90..=100 => Self::Healthy,
            70..=89 => Self::Warning,
            50..=69 => Self::Degraded,
            0..=49 => Self::Critical,
            _ => Self::Unknown,
        }
    }
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

// ============================================================================
// DIAGNOSIS FINDING
// ============================================================================

/// Diagnosis finding
#[derive(Debug, Clone)]
pub struct DiagnosisFinding {
    /// Finding type
    pub finding_type: FindingType,
    /// Description
    pub description: String,
    /// Severity
    pub severity: Severity,
    /// Evidence
    pub evidence: Vec<String>,
}

impl DiagnosisFinding {
    /// Create new finding
    pub fn new(finding_type: FindingType, description: impl Into<String>) -> Self {
        Self {
            finding_type,
            description: description.into(),
            severity: Severity::MEDIUM,
            evidence: Vec::new(),
        }
    }

    /// With severity
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// With evidence
    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence.push(evidence.into());
        self
    }
}

/// Types of findings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FindingType {
    /// Performance issue
    Performance,
    /// Resource issue
    Resource,
    /// Configuration issue
    Configuration,
    /// Behavioral anomaly
    Anomaly,
    /// Drift from baseline
    Drift,
    /// Error pattern
    ErrorPattern,
    /// Capacity issue
    Capacity,
}

impl FindingType {
    /// Get type name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Performance => "performance",
            Self::Resource => "resource",
            Self::Configuration => "configuration",
            Self::Anomaly => "anomaly",
            Self::Drift => "drift",
            Self::ErrorPattern => "error_pattern",
            Self::Capacity => "capacity",
        }
    }
}

// ============================================================================
// META-COGNITIVE MONITOR
// ============================================================================

/// Meta-cognitive monitor trait
pub trait MetaCognitiveMonitor: Introspectable {
    /// Assess overall cognitive health
    fn assess(&self) -> CognitiveAssessment;

    /// Detect cognitive biases
    fn detect_biases(&self) -> Vec<BiasReport>;

    /// Calibrate confidence estimations
    fn calibrate(&mut self) -> CalibrationResult;

    /// Get cognitive load
    fn cognitive_load(&self) -> f32;
}

/// Cognitive assessment
#[derive(Debug, Clone)]
pub struct CognitiveAssessment {
    /// Overall score (0-100)
    pub score: u8,
    /// Perception quality
    pub perception_quality: f32,
    /// Comprehension accuracy
    pub comprehension_accuracy: f32,
    /// Reasoning validity
    pub reasoning_validity: f32,
    /// Decision quality
    pub decision_quality: f32,
    /// Execution success rate
    pub execution_success: f32,
    /// Memory utilization
    pub memory_utilization: f32,
    /// Timestamp
    pub timestamp: Timestamp,
}

impl CognitiveAssessment {
    /// Is cognitive system healthy?
    pub fn is_healthy(&self) -> bool {
        self.score >= 70
    }

    /// Get weakest domain
    pub fn weakest_domain(&self) -> &'static str {
        let domains = [
            ("perception", self.perception_quality),
            ("comprehension", self.comprehension_accuracy),
            ("reasoning", self.reasoning_validity),
            ("decision", self.decision_quality),
            ("execution", self.execution_success),
            ("memory", self.memory_utilization),
        ];
        domains.iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal))
            .map(|(name, _)| *name)
            .unwrap_or("unknown")
    }
}

impl Default for CognitiveAssessment {
    fn default() -> Self {
        Self {
            score: 100,
            perception_quality: 1.0,
            comprehension_accuracy: 1.0,
            reasoning_validity: 1.0,
            decision_quality: 1.0,
            execution_success: 1.0,
            memory_utilization: 0.5,
            timestamp: Timestamp::now(),
        }
    }
}

/// Bias report
#[derive(Debug, Clone)]
pub struct BiasReport {
    /// Bias type
    pub bias_type: BiasType,
    /// Magnitude (0.0 to 1.0)
    pub magnitude: f32,
    /// Description
    pub description: String,
    /// Affected component
    pub affected: ComponentId,
}

/// Types of cognitive biases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiasType {
    /// Recency bias (over-weighing recent data)
    Recency,
    /// Confirmation bias (seeking confirming evidence)
    Confirmation,
    /// Anchoring bias (over-relying on first data)
    Anchoring,
    /// Availability bias (over-weighing available data)
    Availability,
    /// Overconfidence
    Overconfidence,
    /// Underconfidence
    Underconfidence,
    /// Status quo bias
    StatusQuo,
    /// Action bias (preferring action over inaction)
    Action,
}

impl BiasType {
    /// Get bias name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Recency => "recency",
            Self::Confirmation => "confirmation",
            Self::Anchoring => "anchoring",
            Self::Availability => "availability",
            Self::Overconfidence => "overconfidence",
            Self::Underconfidence => "underconfidence",
            Self::StatusQuo => "status_quo",
            Self::Action => "action",
        }
    }
}

/// Calibration result
#[derive(Debug, Clone)]
pub struct CalibrationResult {
    /// Success
    pub success: bool,
    /// Adjustments made
    pub adjustments: Vec<String>,
    /// New confidence calibration factor
    pub new_calibration: f32,
    /// Previous calibration factor
    pub old_calibration: f32,
}

impl CalibrationResult {
    /// No changes needed
    pub fn unchanged() -> Self {
        Self {
            success: true,
            adjustments: Vec::new(),
            new_calibration: 1.0,
            old_calibration: 1.0,
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
    fn test_health_status() {
        assert_eq!(HealthStatus::from_score(95), HealthStatus::Healthy);
        assert_eq!(HealthStatus::from_score(75), HealthStatus::Warning);
        assert_eq!(HealthStatus::from_score(55), HealthStatus::Degraded);
        assert_eq!(HealthStatus::from_score(30), HealthStatus::Critical);
    }

    #[test]
    fn test_cognitive_assessment() {
        let assessment = CognitiveAssessment {
            score: 80,
            perception_quality: 0.9,
            comprehension_accuracy: 0.85,
            reasoning_validity: 0.7, // Weakest
            decision_quality: 0.8,
            execution_success: 0.95,
            memory_utilization: 0.6,
            timestamp: Timestamp::now(),
        };

        assert!(assessment.is_healthy());
        assert_eq!(assessment.weakest_domain(), "reasoning");
    }

    #[test]
    fn test_diagnosis_finding() {
        let finding = DiagnosisFinding::new(FindingType::Performance, "High latency")
            .with_severity(Severity::HIGH)
            .with_evidence("P99 > 100ms");

        assert_eq!(finding.finding_type, FindingType::Performance);
        assert_eq!(finding.evidence.len(), 1);
    }
}
