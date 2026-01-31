//! Introspector — Cognitive system health monitoring
//!
//! The introspector observes all domains and tracks their health,
//! detecting anomalies and issues in the cognitive system.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::*;
use crate::bus::Domain;
use super::metrics::DomainMetrics;

// ============================================================================
// ISSUE ID
// ============================================================================

/// Issue ID type
define_id!(IssueId, "Cognitive issue identifier");

// ============================================================================
// COGNITIVE STATUS
// ============================================================================

/// Cognitive status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CognitiveStatus {
    /// Everything is working optimally
    Optimal,
    /// Healthy operation
    Healthy,
    /// Some degradation
    Degraded,
    /// Significantly impaired
    Impaired,
    /// Critical issues
    Critical,
}

impl CognitiveStatus {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Optimal => "Optimal",
            Self::Healthy => "Healthy",
            Self::Degraded => "Degraded",
            Self::Impaired => "Impaired",
            Self::Critical => "Critical",
        }
    }

    /// From health score
    pub fn from_score(score: u8) -> Self {
        if score >= 90 {
            Self::Optimal
        } else if score >= 70 {
            Self::Healthy
        } else if score >= 50 {
            Self::Degraded
        } else if score >= 30 {
            Self::Impaired
        } else {
            Self::Critical
        }
    }
}

// ============================================================================
// ISSUE TYPE
// ============================================================================

/// Issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueType {
    /// High latency in processing
    HighLatency,
    /// High error rate
    HighErrorRate,
    /// Queue backlog building up
    QueueBacklog,
    /// Health is declining
    DecliningHealth,
    /// Capacity exceeded
    CapacityExceeded,
    /// Stalled processing
    Stalled,
    /// Oscillation detected
    Oscillation,
    /// Feedback loop
    FeedbackLoop,
}

impl IssueType {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::HighLatency => "High Latency",
            Self::HighErrorRate => "High Error Rate",
            Self::QueueBacklog => "Queue Backlog",
            Self::DecliningHealth => "Declining Health",
            Self::CapacityExceeded => "Capacity Exceeded",
            Self::Stalled => "Stalled",
            Self::Oscillation => "Oscillation",
            Self::FeedbackLoop => "Feedback Loop",
        }
    }
}

// ============================================================================
// COGNITIVE ISSUE
// ============================================================================

/// A cognitive issue
#[derive(Debug, Clone)]
pub struct CognitiveIssue {
    /// Issue ID
    pub id: IssueId,
    /// Affected domain
    pub domain: Domain,
    /// Issue type
    pub issue_type: IssueType,
    /// Severity
    pub severity: Severity,
    /// Description
    pub description: String,
    /// When detected
    pub detected_at: Timestamp,
}

impl CognitiveIssue {
    /// Create new issue
    pub fn new(
        domain: Domain,
        issue_type: IssueType,
        severity: Severity,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: IssueId::generate(),
            domain,
            issue_type,
            severity,
            description: description.into(),
            detected_at: Timestamp::now(),
        }
    }

    /// Is critical?
    pub fn is_critical(&self) -> bool {
        self.severity == Severity::Critical
    }
}

// ============================================================================
// DOMAIN HEALTH
// ============================================================================

/// Domain health
#[derive(Debug, Clone)]
pub struct DomainHealth {
    /// Domain
    pub domain: Domain,
    /// Status
    pub status: CognitiveStatus,
    /// Health score
    pub health_score: u8,
    /// Issues
    pub issues: Vec<CognitiveIssue>,
}

impl DomainHealth {
    /// Create healthy status
    pub fn healthy(domain: Domain, score: u8) -> Self {
        Self {
            domain,
            status: CognitiveStatus::from_score(score),
            health_score: score,
            issues: Vec::new(),
        }
    }

    /// Add issue
    pub fn add_issue(&mut self, issue: CognitiveIssue) {
        self.issues.push(issue);
    }

    /// Has issues?
    pub fn has_issues(&self) -> bool {
        !self.issues.is_empty()
    }
}

// ============================================================================
// COGNITIVE HEALTH
// ============================================================================

/// Cognitive health report
#[derive(Debug, Clone)]
pub struct CognitiveHealth {
    /// Overall score (0-100)
    pub overall_score: u8,
    /// Overall status
    pub status: CognitiveStatus,
    /// Per-domain health
    pub domain_health: Vec<DomainHealth>,
    /// Active issues
    pub issues: Vec<CognitiveIssue>,
    /// Timestamp
    pub timestamp: Timestamp,
}

impl CognitiveHealth {
    /// Create new health report
    pub fn new(overall_score: u8, domain_health: Vec<DomainHealth>) -> Self {
        let issues: Vec<_> = domain_health
            .iter()
            .flat_map(|d| d.issues.clone())
            .collect();

        Self {
            overall_score,
            status: CognitiveStatus::from_score(overall_score),
            domain_health,
            issues,
            timestamp: Timestamp::now(),
        }
    }

    /// Is healthy?
    pub fn is_healthy(&self) -> bool {
        matches!(self.status, CognitiveStatus::Optimal | CognitiveStatus::Healthy)
    }

    /// Get critical issues
    pub fn critical_issues(&self) -> Vec<&CognitiveIssue> {
        self.issues.iter().filter(|i| i.is_critical()).collect()
    }
}

// ============================================================================
// INTROSPECTOR
// ============================================================================

/// Introspector - monitors cognitive system health
pub struct Introspector {
    /// Domain metrics history
    history: BTreeMap<Domain, Vec<DomainMetrics>>,
    /// Maximum history per domain
    max_history: usize,
    /// Anomalies detected
    anomalies_detected: AtomicU64,
}

impl Introspector {
    /// Create new introspector
    pub fn new(max_history: usize) -> Self {
        Self {
            history: BTreeMap::new(),
            max_history,
            anomalies_detected: AtomicU64::new(0),
        }
    }

    /// Record domain metrics
    pub fn record(&mut self, metrics: DomainMetrics) {
        let domain = metrics.domain;
        let history = self.history.entry(domain).or_default();
        history.push(metrics);

        if history.len() > self.max_history {
            history.remove(0);
        }
    }

    /// Get current metrics for domain
    pub fn current(&self, domain: Domain) -> Option<&DomainMetrics> {
        self.history.get(&domain)?.last()
    }

    /// Get metrics history for domain
    pub fn history(&self, domain: Domain) -> Option<&[DomainMetrics]> {
        self.history.get(&domain).map(|v| v.as_slice())
    }

    /// Analyze cognitive health
    pub fn analyze(&self) -> CognitiveHealth {
        let mut domain_health = Vec::new();
        let mut issues = Vec::new();

        for (&domain, history) in &self.history {
            if let Some(latest) = history.last() {
                let health = DomainHealth {
                    domain,
                    status: self.assess_domain_status(latest),
                    health_score: latest.health_score,
                    issues: self.find_domain_issues(domain, history),
                };

                if !health.issues.is_empty() {
                    for issue in &health.issues {
                        issues.push(issue.clone());
                    }
                }

                domain_health.push(health);
            }
        }

        let overall_score = if domain_health.is_empty() {
            100
        } else {
            let sum: u32 = domain_health.iter().map(|d| d.health_score as u32).sum();
            (sum / domain_health.len() as u32) as u8
        };

        CognitiveHealth {
            overall_score,
            status: self.score_to_status(overall_score),
            domain_health,
            issues,
            timestamp: Timestamp::now(),
        }
    }

    /// Assess domain status from metrics
    fn assess_domain_status(&self, metrics: &DomainMetrics) -> CognitiveStatus {
        CognitiveStatus::from_score(metrics.health_score)
    }

    /// Find issues in domain
    fn find_domain_issues(&self, domain: Domain, history: &[DomainMetrics]) -> Vec<CognitiveIssue> {
        let mut issues = Vec::new();

        if let Some(latest) = history.last() {
            // High latency
            if latest.avg_latency_us > 10000 {
                issues.push(CognitiveIssue {
                    id: IssueId::generate(),
                    domain,
                    issue_type: IssueType::HighLatency,
                    severity: if latest.avg_latency_us > 100000 {
                        Severity::Error
                    } else {
                        Severity::Warning
                    },
                    description: format!(
                        "{:?} domain has high latency: {}us",
                        domain, latest.avg_latency_us
                    ),
                    detected_at: Timestamp::now(),
                });
            }

            // High error rate
            if latest.error_rate > 0.1 {
                issues.push(CognitiveIssue {
                    id: IssueId::generate(),
                    domain,
                    issue_type: IssueType::HighErrorRate,
                    severity: if latest.error_rate > 0.5 {
                        Severity::Error
                    } else {
                        Severity::Warning
                    },
                    description: format!(
                        "{:?} domain has high error rate: {:.1}%",
                        domain, latest.error_rate * 100.0
                    ),
                    detected_at: Timestamp::now(),
                });
            }

            // Queue buildup
            if latest.queue_depth > 1000 {
                issues.push(CognitiveIssue {
                    id: IssueId::generate(),
                    domain,
                    issue_type: IssueType::QueueBacklog,
                    severity: Severity::Warning,
                    description: format!(
                        "{:?} domain has queue backlog: {} messages",
                        domain, latest.queue_depth
                    ),
                    detected_at: Timestamp::now(),
                });
            }

            // Trend analysis
            if history.len() >= 5 {
                let recent: Vec<_> = history.iter().rev().take(5).collect();
                let health_trend: i32 = recent.windows(2)
                    .map(|w| w[0].health_score as i32 - w[1].health_score as i32)
                    .sum();

                if health_trend < -20 {
                    issues.push(CognitiveIssue {
                        id: IssueId::generate(),
                        domain,
                        issue_type: IssueType::DecliningHealth,
                        severity: Severity::Warning,
                        description: format!(
                            "{:?} domain health is declining",
                            domain
                        ),
                        detected_at: Timestamp::now(),
                    });
                }
            }
        }

        issues
    }

    /// Convert score to status
    pub fn score_to_status(&self, score: u8) -> CognitiveStatus {
        CognitiveStatus::from_score(score)
    }

    /// Get statistics
    pub fn stats(&self) -> IntrospectorStats {
        IntrospectorStats {
            domains_monitored: self.history.len(),
            anomalies_detected: self.anomalies_detected.load(Ordering::Relaxed),
            total_samples: self.history.values().map(|v| v.len()).sum(),
        }
    }
}

impl Default for Introspector {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Introspector statistics
#[derive(Debug, Clone)]
pub struct IntrospectorStats {
    /// Domains being monitored
    pub domains_monitored: usize,
    /// Anomalies detected
    pub anomalies_detected: u64,
    /// Total samples collected
    pub total_samples: usize,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_metrics(domain: Domain, health_score: u8) -> DomainMetrics {
        DomainMetrics {
            domain,
            health_score,
            messages_processed: 1000,
            avg_latency_us: 500,
            p99_latency_us: 2000,
            error_rate: 0.01,
            queue_depth: 10,
            last_tick: 100,
            timestamp: Timestamp::now(),
        }
    }

    #[test]
    fn test_introspector() {
        let mut introspector = Introspector::new(100);

        let metrics = make_test_metrics(Domain::Sense, 90);
        introspector.record(metrics);

        assert!(introspector.current(Domain::Sense).is_some());

        let health = introspector.analyze();
        assert!(health.overall_score >= 90);
    }

    #[test]
    fn test_issue_detection() {
        let mut introspector = Introspector::new(100);

        let metrics = DomainMetrics {
            domain: Domain::Understand,
            health_score: 60,
            messages_processed: 100,
            avg_latency_us: 50000,
            p99_latency_us: 100000,
            error_rate: 0.2,
            queue_depth: 2000,
            last_tick: 100,
            timestamp: Timestamp::now(),
        };
        introspector.record(metrics);

        let health = introspector.analyze();
        assert!(!health.issues.is_empty());
    }

    #[test]
    fn test_cognitive_status() {
        assert_eq!(CognitiveStatus::from_score(95), CognitiveStatus::Optimal);
        assert_eq!(CognitiveStatus::from_score(80), CognitiveStatus::Healthy);
        assert_eq!(CognitiveStatus::from_score(60), CognitiveStatus::Degraded);
        assert_eq!(CognitiveStatus::from_score(40), CognitiveStatus::Impaired);
        assert_eq!(CognitiveStatus::from_score(20), CognitiveStatus::Critical);
    }
}
