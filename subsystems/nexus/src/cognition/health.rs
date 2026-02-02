//! # Cognitive Health Monitor
//!
//! Monitors the health of cognitive domains and overall system.
//! Detects degradation and triggers recovery actions.

#![allow(dead_code)]

extern crate alloc;
use alloc::format;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// HEALTH TYPES
// ============================================================================

/// Health status of a domain
#[derive(Debug, Clone)]
pub struct DomainHealth {
    /// Domain ID
    pub domain_id: DomainId,
    /// Overall health score (0.0 - 1.0)
    pub health_score: f32,
    /// Status
    pub status: HealthStatus,
    /// Metrics
    pub metrics: HealthMetrics,
    /// Active issues
    pub issues: Vec<HealthIssue>,
    /// Last check timestamp
    pub last_check: Timestamp,
    /// Trend
    pub trend: HealthTrend,
}

/// Health status levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// All systems normal
    Healthy,
    /// Minor issues detected
    Degraded,
    /// Significant problems
    Unhealthy,
    /// Critical state
    Critical,
    /// Not responding
    Unresponsive,
    /// Unknown state
    Unknown,
}

impl HealthStatus {
    /// Convert to numeric level
    pub fn level(&self) -> u8 {
        match self {
            Self::Healthy => 4,
            Self::Degraded => 3,
            Self::Unhealthy => 2,
            Self::Critical => 1,
            Self::Unresponsive => 0,
            Self::Unknown => 0,
        }
    }

    /// From health score
    pub fn from_score(score: f32) -> Self {
        match score {
            s if s >= 0.9 => Self::Healthy,
            s if s >= 0.7 => Self::Degraded,
            s if s >= 0.4 => Self::Unhealthy,
            s if s > 0.0 => Self::Critical,
            _ => Self::Unresponsive,
        }
    }
}

/// Health metrics
#[derive(Debug, Clone, Default)]
pub struct HealthMetrics {
    /// Response time (ns)
    pub response_time_ns: u64,
    /// Error rate (0.0 - 1.0)
    pub error_rate: f32,
    /// Throughput (items/cycle)
    pub throughput: f64,
    /// Resource usage (0.0 - 1.0)
    pub resource_usage: f32,
    /// Queue depth
    pub queue_depth: u32,
    /// Memory usage (bytes)
    pub memory_bytes: u64,
    /// Consecutive failures
    pub consecutive_failures: u32,
}

/// Health issue
#[derive(Debug, Clone)]
pub struct HealthIssue {
    /// Issue ID
    pub id: u64,
    /// Issue type
    pub issue_type: IssueType,
    /// Severity
    pub severity: IssueSeverity,
    /// Description
    pub description: String,
    /// First detected
    pub first_detected: Timestamp,
    /// Impact score
    pub impact: f32,
    /// Suggested action
    pub suggested_action: Option<String>,
}

/// Issue types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueType {
    /// High latency
    HighLatency,
    /// High error rate
    HighErrorRate,
    /// Low throughput
    LowThroughput,
    /// Resource exhaustion
    ResourceExhaustion,
    /// Memory pressure
    MemoryPressure,
    /// Timeout
    Timeout,
    /// Stall detected
    Stall,
    /// Dependency failure
    DependencyFailure,
    /// Configuration issue
    ConfigIssue,
}

/// Issue severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Health trend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthTrend {
    /// Improving
    Improving,
    /// Stable
    Stable,
    /// Degrading
    Degrading,
    /// Unknown (not enough data)
    Unknown,
}

// ============================================================================
// HEALTH MONITOR
// ============================================================================

/// Monitors cognitive health
pub struct HealthMonitor {
    /// Health status by domain
    domain_health: BTreeMap<DomainId, DomainHealth>,
    /// System-wide health
    system_health: SystemHealth,
    /// Health history
    history: Vec<HealthSnapshot>,
    /// Next issue ID
    next_issue_id: AtomicU64,
    /// Configuration
    config: HealthConfig,
    /// Thresholds
    thresholds: HealthThresholds,
    /// Alert callbacks
    alerts: Vec<HealthAlert>,
}

/// System-wide health
#[derive(Debug, Clone)]
pub struct SystemHealth {
    /// Overall health score
    pub score: f32,
    /// Status
    pub status: HealthStatus,
    /// Domain count
    pub domain_count: u32,
    /// Healthy domain count
    pub healthy_domains: u32,
    /// Active issue count
    pub active_issues: u32,
    /// Last update
    pub last_update: Timestamp,
}

impl Default for SystemHealth {
    fn default() -> Self {
        Self {
            score: 1.0,
            status: HealthStatus::Unknown,
            domain_count: 0,
            healthy_domains: 0,
            active_issues: 0,
            last_update: Timestamp::now(),
        }
    }
}

/// Health snapshot for history
#[derive(Debug, Clone)]
pub struct HealthSnapshot {
    /// Timestamp
    pub timestamp: Timestamp,
    /// System score
    pub system_score: f32,
    /// Domain scores
    pub domain_scores: BTreeMap<DomainId, f32>,
    /// Active issues
    pub issue_count: u32,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Check interval (cycles)
    pub check_interval: u64,
    /// History retention (snapshots)
    pub history_size: usize,
    /// Trend window (snapshots)
    pub trend_window: usize,
    /// Enable automatic recovery
    pub auto_recovery: bool,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            check_interval: 10,
            history_size: 100,
            trend_window: 10,
            auto_recovery: true,
        }
    }
}

/// Health thresholds
#[derive(Debug, Clone)]
pub struct HealthThresholds {
    /// Maximum response time (ns)
    pub max_response_time_ns: u64,
    /// Maximum error rate
    pub max_error_rate: f32,
    /// Minimum throughput
    pub min_throughput: f64,
    /// Maximum resource usage
    pub max_resource_usage: f32,
    /// Maximum consecutive failures
    pub max_consecutive_failures: u32,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            max_response_time_ns: 1_000_000, // 1ms
            max_error_rate: 0.05,            // 5%
            min_throughput: 10.0,
            max_resource_usage: 0.9, // 90%
            max_consecutive_failures: 3,
        }
    }
}

/// Health alert
#[derive(Debug, Clone)]
pub struct HealthAlert {
    /// Alert ID
    pub id: u64,
    /// Domain (None for system-wide)
    pub domain: Option<DomainId>,
    /// Status threshold
    pub threshold: HealthStatus,
    /// Callback tag
    pub callback_tag: String,
    /// Triggered
    pub triggered: bool,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new(config: HealthConfig, thresholds: HealthThresholds) -> Self {
        Self {
            domain_health: BTreeMap::new(),
            system_health: SystemHealth::default(),
            history: Vec::new(),
            next_issue_id: AtomicU64::new(1),
            config,
            thresholds,
            alerts: Vec::new(),
        }
    }

    /// Register a domain for monitoring
    pub fn register_domain(&mut self, domain_id: DomainId) {
        let health = DomainHealth {
            domain_id,
            health_score: 1.0,
            status: HealthStatus::Unknown,
            metrics: HealthMetrics::default(),
            issues: Vec::new(),
            last_check: Timestamp::now(),
            trend: HealthTrend::Unknown,
        };
        self.domain_health.insert(domain_id, health);
        self.update_system_health();
    }

    /// Unregister a domain
    pub fn unregister_domain(&mut self, domain_id: DomainId) {
        self.domain_health.remove(&domain_id);
        self.update_system_health();
    }

    /// Update domain metrics
    pub fn update_metrics(&mut self, domain_id: DomainId, metrics: HealthMetrics) {
        if let Some(health) = self.domain_health.get_mut(&domain_id) {
            health.metrics = metrics;
            health.last_check = Timestamp::now();
            self.evaluate_domain_health(domain_id);
        }
    }

    /// Evaluate domain health
    fn evaluate_domain_health(&mut self, domain_id: DomainId) {
        let health = match self.domain_health.get_mut(&domain_id) {
            Some(h) => h,
            None => return,
        };

        let mut issues = Vec::new();
        let mut score = 1.0f32;

        // Check response time
        if health.metrics.response_time_ns > self.thresholds.max_response_time_ns {
            let ratio = health.metrics.response_time_ns as f32
                / self.thresholds.max_response_time_ns as f32;
            score -= (ratio - 1.0).min(0.3);

            issues.push(HealthIssue {
                id: self.next_issue_id.fetch_add(1, Ordering::Relaxed),
                issue_type: IssueType::HighLatency,
                severity: if ratio > 5.0 {
                    IssueSeverity::Critical
                } else if ratio > 2.0 {
                    IssueSeverity::Error
                } else {
                    IssueSeverity::Warning
                },
                description: format!(
                    "Response time {}ns exceeds threshold",
                    health.metrics.response_time_ns
                ),
                first_detected: Timestamp::now(),
                impact: (ratio - 1.0).min(1.0),
                suggested_action: Some("Optimize processing or reduce load".into()),
            });
        }

        // Check error rate
        if health.metrics.error_rate > self.thresholds.max_error_rate {
            score -= (health.metrics.error_rate - self.thresholds.max_error_rate) * 2.0;

            issues.push(HealthIssue {
                id: self.next_issue_id.fetch_add(1, Ordering::Relaxed),
                issue_type: IssueType::HighErrorRate,
                severity: if health.metrics.error_rate > 0.2 {
                    IssueSeverity::Critical
                } else if health.metrics.error_rate > 0.1 {
                    IssueSeverity::Error
                } else {
                    IssueSeverity::Warning
                },
                description: format!(
                    "Error rate {:.1}% exceeds threshold",
                    health.metrics.error_rate * 100.0
                ),
                first_detected: Timestamp::now(),
                impact: health.metrics.error_rate,
                suggested_action: Some("Investigate error causes".into()),
            });
        }

        // Check throughput
        if health.metrics.throughput < self.thresholds.min_throughput {
            let ratio = self.thresholds.min_throughput / health.metrics.throughput.max(0.1);
            score -= (ratio - 1.0).min(0.3) as f32;

            issues.push(HealthIssue {
                id: self.next_issue_id.fetch_add(1, Ordering::Relaxed),
                issue_type: IssueType::LowThroughput,
                severity: if ratio > 5.0 {
                    IssueSeverity::Error
                } else {
                    IssueSeverity::Warning
                },
                description: format!(
                    "Throughput {:.1} below threshold",
                    health.metrics.throughput
                ),
                first_detected: Timestamp::now(),
                impact: (ratio - 1.0).min(1.0) as f32,
                suggested_action: Some("Check for bottlenecks".into()),
            });
        }

        // Check resource usage
        if health.metrics.resource_usage > self.thresholds.max_resource_usage {
            score -= (health.metrics.resource_usage - self.thresholds.max_resource_usage) * 3.0;

            issues.push(HealthIssue {
                id: self.next_issue_id.fetch_add(1, Ordering::Relaxed),
                issue_type: IssueType::ResourceExhaustion,
                severity: if health.metrics.resource_usage > 0.95 {
                    IssueSeverity::Critical
                } else {
                    IssueSeverity::Error
                },
                description: format!(
                    "Resource usage {:.1}%",
                    health.metrics.resource_usage * 100.0
                ),
                first_detected: Timestamp::now(),
                impact: health.metrics.resource_usage,
                suggested_action: Some("Free resources or scale up".into()),
            });
        }

        // Check consecutive failures
        if health.metrics.consecutive_failures > self.thresholds.max_consecutive_failures {
            score -= 0.5;

            issues.push(HealthIssue {
                id: self.next_issue_id.fetch_add(1, Ordering::Relaxed),
                issue_type: IssueType::Stall,
                severity: IssueSeverity::Critical,
                description: format!(
                    "{} consecutive failures",
                    health.metrics.consecutive_failures
                ),
                first_detected: Timestamp::now(),
                impact: 1.0,
                suggested_action: Some("Restart domain".into()),
            });
        }

        health.health_score = score.max(0.0).min(1.0);
        health.status = HealthStatus::from_score(health.health_score);
        health.issues = issues;
        health.trend = self.calculate_trend(domain_id);

        self.update_system_health();
        self.check_alerts(Some(domain_id));
    }

    /// Calculate health trend
    fn calculate_trend(&self, domain_id: DomainId) -> HealthTrend {
        let scores: Vec<f32> = self
            .history
            .iter()
            .rev()
            .take(self.config.trend_window)
            .filter_map(|s| s.domain_scores.get(&domain_id).copied())
            .collect();

        if scores.len() < 3 {
            return HealthTrend::Unknown;
        }

        let first_half: f32 =
            scores.iter().skip(scores.len() / 2).sum::<f32>() / (scores.len() / 2) as f32;
        let second_half: f32 =
            scores.iter().take(scores.len() / 2).sum::<f32>() / (scores.len() / 2) as f32;

        let diff = second_half - first_half;
        if diff > 0.05 {
            HealthTrend::Improving
        } else if diff < -0.05 {
            HealthTrend::Degrading
        } else {
            HealthTrend::Stable
        }
    }

    /// Update system-wide health
    fn update_system_health(&mut self) {
        let count = self.domain_health.len() as u32;
        if count == 0 {
            self.system_health = SystemHealth::default();
            return;
        }

        let total_score: f32 = self.domain_health.values().map(|h| h.health_score).sum();

        let healthy = self
            .domain_health
            .values()
            .filter(|h| h.status == HealthStatus::Healthy)
            .count() as u32;

        let issues: u32 = self
            .domain_health
            .values()
            .map(|h| h.issues.len() as u32)
            .sum();

        self.system_health = SystemHealth {
            score: total_score / count as f32,
            status: HealthStatus::from_score(total_score / count as f32),
            domain_count: count,
            healthy_domains: healthy,
            active_issues: issues,
            last_update: Timestamp::now(),
        };

        // Take snapshot
        self.take_snapshot();
    }

    /// Take health snapshot
    fn take_snapshot(&mut self) {
        let snapshot = HealthSnapshot {
            timestamp: Timestamp::now(),
            system_score: self.system_health.score,
            domain_scores: self
                .domain_health
                .iter()
                .map(|(id, h)| (*id, h.health_score))
                .collect(),
            issue_count: self.system_health.active_issues,
        };

        if self.history.len() >= self.config.history_size {
            self.history.remove(0);
        }
        self.history.push(snapshot);
    }

    /// Add alert
    pub fn add_alert(
        &mut self,
        domain: Option<DomainId>,
        threshold: HealthStatus,
        callback_tag: String,
    ) -> u64 {
        let id = self.alerts.len() as u64 + 1;
        self.alerts.push(HealthAlert {
            id,
            domain,
            threshold,
            callback_tag,
            triggered: false,
        });
        id
    }

    /// Check alerts
    fn check_alerts(&mut self, domain: Option<DomainId>) {
        for alert in &mut self.alerts {
            let status = if let Some(domain_id) = alert.domain {
                if domain != Some(domain_id) {
                    continue;
                }
                self.domain_health
                    .get(&domain_id)
                    .map(|h| h.status)
                    .unwrap_or(HealthStatus::Unknown)
            } else {
                self.system_health.status
            };

            if status.level() <= alert.threshold.level() {
                alert.triggered = true;
            }
        }
    }

    /// Get triggered alerts
    pub fn get_triggered_alerts(&self) -> Vec<&HealthAlert> {
        self.alerts.iter().filter(|a| a.triggered).collect()
    }

    /// Clear triggered alerts
    pub fn clear_alerts(&mut self) {
        for alert in &mut self.alerts {
            alert.triggered = false;
        }
    }

    /// Get domain health
    pub fn get_domain_health(&self, domain_id: DomainId) -> Option<&DomainHealth> {
        self.domain_health.get(&domain_id)
    }

    /// Get system health
    pub fn get_system_health(&self) -> &SystemHealth {
        &self.system_health
    }

    /// Get all domain healths
    pub fn all_domain_healths(&self) -> &BTreeMap<DomainId, DomainHealth> {
        &self.domain_health
    }

    /// Get health history
    pub fn history(&self) -> &[HealthSnapshot] {
        &self.history
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_from_score() {
        assert_eq!(HealthStatus::from_score(0.95), HealthStatus::Healthy);
        assert_eq!(HealthStatus::from_score(0.8), HealthStatus::Degraded);
        assert_eq!(HealthStatus::from_score(0.5), HealthStatus::Unhealthy);
        assert_eq!(HealthStatus::from_score(0.2), HealthStatus::Critical);
        assert_eq!(HealthStatus::from_score(0.0), HealthStatus::Unresponsive);
    }

    #[test]
    fn test_health_monitor() {
        let config = HealthConfig::default();
        let thresholds = HealthThresholds::default();
        let mut monitor = HealthMonitor::new(config, thresholds);

        let domain = DomainId::new(1);
        monitor.register_domain(domain);

        // Update with good metrics
        let metrics = HealthMetrics {
            response_time_ns: 500_000,
            error_rate: 0.01,
            throughput: 50.0,
            resource_usage: 0.5,
            queue_depth: 10,
            memory_bytes: 1024,
            consecutive_failures: 0,
        };
        monitor.update_metrics(domain, metrics);

        let health = monitor.get_domain_health(domain).unwrap();
        assert_eq!(health.status, HealthStatus::Healthy);
    }

    #[test]
    fn test_issue_detection() {
        let config = HealthConfig::default();
        let thresholds = HealthThresholds::default();
        let mut monitor = HealthMonitor::new(config, thresholds);

        let domain = DomainId::new(1);
        monitor.register_domain(domain);

        // Update with bad metrics
        let metrics = HealthMetrics {
            response_time_ns: 10_000_000, // 10ms, way over threshold
            error_rate: 0.2,              // 20%, way over threshold
            throughput: 1.0,              // Very low
            resource_usage: 0.95,         // Very high
            queue_depth: 1000,
            memory_bytes: 1024 * 1024 * 100,
            consecutive_failures: 5,
        };
        monitor.update_metrics(domain, metrics);

        let health = monitor.get_domain_health(domain).unwrap();
        assert!(health.status != HealthStatus::Healthy);
        assert!(!health.issues.is_empty());
    }

    #[test]
    fn test_alerts() {
        let config = HealthConfig::default();
        let thresholds = HealthThresholds::default();
        let mut monitor = HealthMonitor::new(config, thresholds);

        let domain = DomainId::new(1);
        monitor.register_domain(domain);
        monitor.add_alert(Some(domain), HealthStatus::Degraded, "degraded".into());

        // Trigger degradation
        let metrics = HealthMetrics {
            response_time_ns: 5_000_000,
            error_rate: 0.15,
            throughput: 5.0,
            resource_usage: 0.85,
            queue_depth: 100,
            memory_bytes: 1024 * 1024,
            consecutive_failures: 2,
        };
        monitor.update_metrics(domain, metrics);

        let triggered = monitor.get_triggered_alerts();
        assert!(!triggered.is_empty());
    }
}
