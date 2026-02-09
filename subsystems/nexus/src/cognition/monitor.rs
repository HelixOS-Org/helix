//! # Cognitive Monitor
//!
//! Real-time monitoring of cognitive systems.
//! Provides dashboards, alerts, and health checks.

#![allow(dead_code)]

extern crate alloc;
use alloc::format;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// MONITOR TYPES
// ============================================================================

/// A monitored entity
#[derive(Debug, Clone)]
pub struct MonitoredEntity {
    /// Entity ID
    pub id: u64,
    /// Entity name
    pub name: String,
    /// Entity type
    pub entity_type: EntityType,
    /// Owner domain
    pub owner: DomainId,
    /// Metrics
    pub metrics: BTreeMap<String, MetricValue>,
    /// Status
    pub status: EntityStatus,
    /// Last update
    pub last_update: Timestamp,
    /// Alert thresholds
    pub thresholds: BTreeMap<String, AlertThreshold>,
}

/// Entity type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    /// Cognitive domain
    Domain,
    /// Component
    Component,
    /// Service
    Service,
    /// Pipeline
    Pipeline,
    /// Queue
    Queue,
    /// Session
    Session,
}

/// Entity status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityStatus {
    /// Unknown
    Unknown,
    /// Healthy
    Healthy,
    /// Warning
    Warning,
    /// Critical
    Critical,
    /// Down
    Down,
}

/// Metric value
#[derive(Debug, Clone)]
pub enum MetricValue {
    /// Counter
    Counter(u64),
    /// Gauge
    Gauge(f64),
    /// Rate
    Rate(f64),
    /// Percentage
    Percentage(f64),
    /// Duration (ns)
    Duration(u64),
    /// Boolean
    Bool(bool),
}

impl MetricValue {
    /// Get as f64
    pub fn as_f64(&self) -> f64 {
        match self {
            Self::Counter(v) => *v as f64,
            Self::Gauge(v) => *v,
            Self::Rate(v) => *v,
            Self::Percentage(v) => *v,
            Self::Duration(v) => *v as f64,
            Self::Bool(v) => {
                if *v {
                    1.0
                } else {
                    0.0
                }
            },
        }
    }
}

/// Alert threshold
#[derive(Debug, Clone)]
pub struct AlertThreshold {
    /// Threshold name
    pub name: String,
    /// Warning level
    pub warning: f64,
    /// Critical level
    pub critical: f64,
    /// Comparison operator
    pub operator: ThresholdOperator,
    /// Cooldown (ns)
    pub cooldown_ns: u64,
    /// Last alert time
    pub last_alert: Option<Timestamp>,
}

/// Threshold operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdOperator {
    /// Greater than
    GreaterThan,
    /// Less than
    LessThan,
    /// Equal
    Equal,
    /// Not equal
    NotEqual,
}

/// An alert
#[derive(Debug, Clone)]
pub struct Alert {
    /// Alert ID
    pub id: u64,
    /// Entity ID
    pub entity_id: u64,
    /// Entity name
    pub entity_name: String,
    /// Metric name
    pub metric: String,
    /// Current value
    pub value: f64,
    /// Threshold
    pub threshold: f64,
    /// Severity
    pub severity: AlertSeverity,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Message
    pub message: String,
    /// Acknowledged
    pub acknowledged: bool,
}

/// Alert severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

// ============================================================================
// HEALTH CHECK
// ============================================================================

/// Health check definition
#[derive(Debug, Clone)]
pub struct HealthCheck {
    /// Check ID
    pub id: u64,
    /// Check name
    pub name: String,
    /// Target entity
    pub entity_id: u64,
    /// Check type
    pub check_type: HealthCheckType,
    /// Interval (ns)
    pub interval_ns: u64,
    /// Timeout (ns)
    pub timeout_ns: u64,
    /// Last check time
    pub last_check: Option<Timestamp>,
    /// Last result
    pub last_result: Option<HealthCheckResult>,
    /// Failure count
    pub failure_count: u32,
    /// Enabled
    pub enabled: bool,
}

/// Health check type
#[derive(Debug, Clone)]
pub enum HealthCheckType {
    /// Liveness check
    Liveness,
    /// Readiness check
    Readiness,
    /// Custom metric check
    Metric {
        name: String,
        operator: ThresholdOperator,
        threshold: f64,
    },
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    /// Check passed
    pub passed: bool,
    /// Duration (ns)
    pub duration_ns: u64,
    /// Message
    pub message: String,
    /// Timestamp
    pub timestamp: Timestamp,
}

// ============================================================================
// MONITOR
// ============================================================================

/// Cognitive system monitor
pub struct CognitiveMonitor {
    /// Monitored entities
    entities: BTreeMap<u64, MonitoredEntity>,
    /// Health checks
    health_checks: BTreeMap<u64, HealthCheck>,
    /// Active alerts
    alerts: VecDeque<Alert>,
    /// Alert history
    alert_history: Vec<Alert>,
    /// Next entity ID
    next_entity_id: AtomicU64,
    /// Next check ID
    next_check_id: AtomicU64,
    /// Next alert ID
    next_alert_id: AtomicU64,
    /// Configuration
    config: MonitorConfig,
    /// Statistics
    stats: MonitorStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// Maximum entities
    pub max_entities: usize,
    /// Maximum alerts
    pub max_alerts: usize,
    /// Maximum history
    pub max_history: usize,
    /// Default check interval (ns)
    pub default_check_interval_ns: u64,
    /// Aggregation period (ns)
    pub aggregation_period_ns: u64,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            max_entities: 10000,
            max_alerts: 1000,
            max_history: 10000,
            default_check_interval_ns: 30_000_000_000, // 30 seconds
            aggregation_period_ns: 60_000_000_000,     // 1 minute
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct MonitorStats {
    /// Total entities
    pub total_entities: u64,
    /// Healthy entities
    pub healthy_entities: u64,
    /// Warning entities
    pub warning_entities: u64,
    /// Critical entities
    pub critical_entities: u64,
    /// Total alerts
    pub total_alerts: u64,
    /// Active alerts
    pub active_alerts: u64,
    /// Health checks executed
    pub checks_executed: u64,
    /// Health checks passed
    pub checks_passed: u64,
}

impl CognitiveMonitor {
    /// Create a new monitor
    pub fn new(config: MonitorConfig) -> Self {
        Self {
            entities: BTreeMap::new(),
            health_checks: BTreeMap::new(),
            alerts: VecDeque::new(),
            alert_history: Vec::new(),
            next_entity_id: AtomicU64::new(1),
            next_check_id: AtomicU64::new(1),
            next_alert_id: AtomicU64::new(1),
            config,
            stats: MonitorStats::default(),
        }
    }

    // ========================================================================
    // ENTITY MANAGEMENT
    // ========================================================================

    /// Register an entity
    pub fn register_entity(&mut self, name: &str, entity_type: EntityType, owner: DomainId) -> u64 {
        let id = self.next_entity_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let entity = MonitoredEntity {
            id,
            name: name.into(),
            entity_type,
            owner,
            metrics: BTreeMap::new(),
            status: EntityStatus::Unknown,
            last_update: now,
            thresholds: BTreeMap::new(),
        };

        self.entities.insert(id, entity);
        self.update_stats();

        id
    }

    /// Unregister an entity
    #[inline]
    pub fn unregister_entity(&mut self, id: u64) -> bool {
        // Remove health checks for this entity
        self.health_checks.retain(|_, check| check.entity_id != id);

        let removed = self.entities.remove(&id).is_some();
        if removed {
            self.update_stats();
        }
        removed
    }

    /// Update entity metrics
    #[inline]
    pub fn update_metrics(&mut self, entity_id: u64, metrics: BTreeMap<String, MetricValue>) {
        if let Some(entity) = self.entities.get_mut(&entity_id) {
            entity.metrics = metrics;
            entity.last_update = Timestamp::now();

            // Check thresholds
            self.check_thresholds(entity_id);
        }
    }

    /// Update single metric
    #[inline]
    pub fn update_metric(&mut self, entity_id: u64, name: &str, value: MetricValue) {
        if let Some(entity) = self.entities.get_mut(&entity_id) {
            entity.metrics.insert(name.into(), value);
            entity.last_update = Timestamp::now();

            self.check_thresholds(entity_id);
        }
    }

    /// Set entity status
    #[inline]
    pub fn set_status(&mut self, entity_id: u64, status: EntityStatus) {
        if let Some(entity) = self.entities.get_mut(&entity_id) {
            entity.status = status;
        }
        self.update_stats();
    }

    /// Set threshold
    #[inline]
    pub fn set_threshold(&mut self, entity_id: u64, metric: &str, threshold: AlertThreshold) {
        if let Some(entity) = self.entities.get_mut(&entity_id) {
            entity.thresholds.insert(metric.into(), threshold);
        }
    }

    /// Check thresholds for entity
    fn check_thresholds(&mut self, entity_id: u64) {
        let entity = match self.entities.get(&entity_id) {
            Some(e) => e.clone(),
            None => return,
        };

        let now = Timestamp::now();
        let mut new_alerts = Vec::new();
        let mut worst_status = EntityStatus::Healthy;

        for (metric_name, metric_value) in &entity.metrics {
            if let Some(threshold) = entity.thresholds.get(metric_name) {
                // Check cooldown
                if let Some(last) = threshold.last_alert {
                    if now.elapsed_since(last) < threshold.cooldown_ns {
                        continue;
                    }
                }

                let value = metric_value.as_f64();
                let triggered = match threshold.operator {
                    ThresholdOperator::GreaterThan => value > threshold.warning,
                    ThresholdOperator::LessThan => value < threshold.warning,
                    ThresholdOperator::Equal => (value - threshold.warning).abs() < f64::EPSILON,
                    ThresholdOperator::NotEqual => (value - threshold.warning).abs() > f64::EPSILON,
                };

                if triggered {
                    let is_critical = match threshold.operator {
                        ThresholdOperator::GreaterThan => value > threshold.critical,
                        ThresholdOperator::LessThan => value < threshold.critical,
                        _ => false,
                    };

                    let severity = if is_critical {
                        worst_status = EntityStatus::Critical;
                        AlertSeverity::Critical
                    } else {
                        if worst_status != EntityStatus::Critical {
                            worst_status = EntityStatus::Warning;
                        }
                        AlertSeverity::Warning
                    };

                    let alert = Alert {
                        id: self.next_alert_id.fetch_add(1, Ordering::Relaxed),
                        entity_id,
                        entity_name: entity.name.clone(),
                        metric: metric_name.clone(),
                        value,
                        threshold: if is_critical {
                            threshold.critical
                        } else {
                            threshold.warning
                        },
                        severity,
                        timestamp: now,
                        message: format!(
                            "{} {} is {:.2} (threshold: {:.2})",
                            entity.name,
                            metric_name,
                            value,
                            if is_critical {
                                threshold.critical
                            } else {
                                threshold.warning
                            }
                        ),
                        acknowledged: false,
                    };

                    new_alerts.push(alert);
                }
            }
        }

        // Add alerts
        for alert in new_alerts {
            // Update last alert time for threshold
            if let Some(entity) = self.entities.get_mut(&entity_id) {
                if let Some(threshold) = entity.thresholds.get_mut(&alert.metric) {
                    threshold.last_alert = Some(now);
                }
            }

            self.alerts.push_back(alert.clone());
            self.stats.total_alerts += 1;

            // Limit active alerts
            if self.alerts.len() > self.config.max_alerts {
                let oldest = self.alerts.pop_front().unwrap();
                self.alert_history.push(oldest);
            }
        }

        // Update entity status
        if let Some(entity) = self.entities.get_mut(&entity_id) {
            entity.status = worst_status;
        }

        self.update_stats();
    }

    // ========================================================================
    // HEALTH CHECKS
    // ========================================================================

    /// Add health check
    pub fn add_health_check(
        &mut self,
        name: &str,
        entity_id: u64,
        check_type: HealthCheckType,
        interval_ns: Option<u64>,
    ) -> u64 {
        let id = self.next_check_id.fetch_add(1, Ordering::Relaxed);

        let check = HealthCheck {
            id,
            name: name.into(),
            entity_id,
            check_type,
            interval_ns: interval_ns.unwrap_or(self.config.default_check_interval_ns),
            timeout_ns: 5_000_000_000, // 5 seconds
            last_check: None,
            last_result: None,
            failure_count: 0,
            enabled: true,
        };

        self.health_checks.insert(id, check);
        id
    }

    /// Remove health check
    #[inline(always)]
    pub fn remove_health_check(&mut self, check_id: u64) -> bool {
        self.health_checks.remove(&check_id).is_some()
    }

    /// Execute health check
    pub fn execute_check(&mut self, check_id: u64) -> Option<HealthCheckResult> {
        let check = self.health_checks.get(&check_id)?;

        if !check.enabled {
            return None;
        }

        let entity = self.entities.get(&check.entity_id)?;
        let now = Timestamp::now();

        // Execute check based on type
        let result = match &check.check_type {
            HealthCheckType::Liveness => {
                // Entity responded recently?
                let age = now.elapsed_since(entity.last_update);
                let passed = age < check.timeout_ns;

                HealthCheckResult {
                    passed,
                    duration_ns: age,
                    message: if passed {
                        "Entity is alive".into()
                    } else {
                        format!("Entity not updated in {} ns", age)
                    },
                    timestamp: now,
                }
            },
            HealthCheckType::Readiness => {
                let passed = entity.status == EntityStatus::Healthy;

                HealthCheckResult {
                    passed,
                    duration_ns: 0,
                    message: format!("Entity status: {:?}", entity.status),
                    timestamp: now,
                }
            },
            HealthCheckType::Metric {
                name,
                operator,
                threshold,
            } => {
                let value = entity.metrics.get(name).map(|v| v.as_f64());

                let passed = match (value, operator) {
                    (Some(v), ThresholdOperator::GreaterThan) => v > *threshold,
                    (Some(v), ThresholdOperator::LessThan) => v < *threshold,
                    (Some(v), ThresholdOperator::Equal) => (v - threshold).abs() < f64::EPSILON,
                    (Some(v), ThresholdOperator::NotEqual) => (v - threshold).abs() > f64::EPSILON,
                    (None, _) => false,
                };

                HealthCheckResult {
                    passed,
                    duration_ns: 0,
                    message: format!("Metric {} = {:?} (threshold: {})", name, value, threshold),
                    timestamp: now,
                }
            },
        };

        // Update check
        if let Some(check) = self.health_checks.get_mut(&check_id) {
            check.last_check = Some(now);
            check.last_result = Some(result.clone());

            if result.passed {
                check.failure_count = 0;
            } else {
                check.failure_count += 1;
            }
        }

        // Update stats
        self.stats.checks_executed += 1;
        if result.passed {
            self.stats.checks_passed += 1;
        }

        Some(result)
    }

    /// Get checks due for execution
    pub fn get_due_checks(&self) -> Vec<u64> {
        let now = Timestamp::now();

        self.health_checks
            .values()
            .filter(|check| {
                check.enabled
                    && check
                        .last_check
                        .map(|last| now.elapsed_since(last) >= check.interval_ns)
                        .unwrap_or(true)
            })
            .map(|check| check.id)
            .collect()
    }

    // ========================================================================
    // ALERTS
    // ========================================================================

    /// Get active alerts
    #[inline(always)]
    pub fn active_alerts(&self) -> &[Alert] {
        &self.alerts
    }

    /// Get alerts by severity
    #[inline]
    pub fn alerts_by_severity(&self, min_severity: AlertSeverity) -> Vec<&Alert> {
        self.alerts
            .iter()
            .filter(|a| a.severity >= min_severity)
            .collect()
    }

    /// Acknowledge alert
    #[inline]
    pub fn acknowledge_alert(&mut self, alert_id: u64) {
        if let Some(alert) = self.alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.acknowledged = true;
        }
    }

    /// Clear alert
    #[inline]
    pub fn clear_alert(&mut self, alert_id: u64) {
        if let Some(pos) = self.alerts.iter().position(|a| a.id == alert_id) {
            let alert = self.alerts.remove(pos);
            self.alert_history.push(alert);
        }
        self.update_stats();
    }

    // ========================================================================
    // QUERIES
    // ========================================================================

    /// Get entity
    #[inline(always)]
    pub fn get_entity(&self, id: u64) -> Option<&MonitoredEntity> {
        self.entities.get(&id)
    }

    /// Get entities by type
    #[inline]
    pub fn entities_by_type(&self, entity_type: EntityType) -> Vec<&MonitoredEntity> {
        self.entities
            .values()
            .filter(|e| e.entity_type == entity_type)
            .collect()
    }

    /// Get entities by status
    #[inline]
    pub fn entities_by_status(&self, status: EntityStatus) -> Vec<&MonitoredEntity> {
        self.entities
            .values()
            .filter(|e| e.status == status)
            .collect()
    }

    /// Get entities by owner
    #[inline]
    pub fn entities_by_owner(&self, owner: DomainId) -> Vec<&MonitoredEntity> {
        self.entities
            .values()
            .filter(|e| e.owner == owner)
            .collect()
    }

    /// Get system summary
    pub fn get_summary(&self) -> MonitorSummary {
        MonitorSummary {
            total_entities: self.entities.len(),
            healthy: self
                .entities
                .values()
                .filter(|e| e.status == EntityStatus::Healthy)
                .count(),
            warning: self
                .entities
                .values()
                .filter(|e| e.status == EntityStatus::Warning)
                .count(),
            critical: self
                .entities
                .values()
                .filter(|e| e.status == EntityStatus::Critical)
                .count(),
            down: self
                .entities
                .values()
                .filter(|e| e.status == EntityStatus::Down)
                .count(),
            active_alerts: self.alerts.len(),
            unacknowledged_alerts: self.alerts.iter().filter(|a| !a.acknowledged).count(),
        }
    }

    /// Update statistics
    fn update_stats(&mut self) {
        self.stats.total_entities = self.entities.len() as u64;
        self.stats.healthy_entities = self
            .entities
            .values()
            .filter(|e| e.status == EntityStatus::Healthy)
            .count() as u64;
        self.stats.warning_entities = self
            .entities
            .values()
            .filter(|e| e.status == EntityStatus::Warning)
            .count() as u64;
        self.stats.critical_entities = self
            .entities
            .values()
            .filter(|e| e.status == EntityStatus::Critical)
            .count() as u64;
        self.stats.active_alerts = self.alerts.len() as u64;
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &MonitorStats {
        &self.stats
    }
}

/// Monitor summary
#[derive(Debug, Clone)]
pub struct MonitorSummary {
    pub total_entities: usize,
    pub healthy: usize,
    pub warning: usize,
    pub critical: usize,
    pub down: usize,
    pub active_alerts: usize,
    pub unacknowledged_alerts: usize,
}

impl Default for CognitiveMonitor {
    fn default() -> Self {
        Self::new(MonitorConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_registration() {
        let mut monitor = CognitiveMonitor::default();
        let domain = DomainId::new(1);

        let id = monitor.register_entity("test_domain", EntityType::Domain, domain);
        let entity = monitor.get_entity(id).unwrap();

        assert_eq!(entity.name, "test_domain");
        assert_eq!(entity.entity_type, EntityType::Domain);
    }

    #[test]
    fn test_metric_update() {
        let mut monitor = CognitiveMonitor::default();
        let domain = DomainId::new(1);

        let id = monitor.register_entity("test", EntityType::Component, domain);
        monitor.update_metric(id, "cpu_usage", MetricValue::Percentage(45.5));

        let entity = monitor.get_entity(id).unwrap();
        let metric = entity.metrics.get("cpu_usage").unwrap();
        assert!((metric.as_f64() - 45.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_threshold_alert() {
        let mut monitor = CognitiveMonitor::default();
        let domain = DomainId::new(1);

        let id = monitor.register_entity("test", EntityType::Service, domain);

        monitor.set_threshold(id, "cpu", AlertThreshold {
            name: "cpu_threshold".into(),
            warning: 80.0,
            critical: 95.0,
            operator: ThresholdOperator::GreaterThan,
            cooldown_ns: 0,
            last_alert: None,
        });

        // Normal value
        monitor.update_metric(id, "cpu", MetricValue::Percentage(50.0));
        assert!(monitor.active_alerts().is_empty());

        // Warning value
        monitor.update_metric(id, "cpu", MetricValue::Percentage(85.0));
        assert!(!monitor.active_alerts().is_empty());
        assert_eq!(monitor.active_alerts()[0].severity, AlertSeverity::Warning);
    }

    #[test]
    fn test_health_check() {
        let mut monitor = CognitiveMonitor::default();
        let domain = DomainId::new(1);

        let entity_id = monitor.register_entity("test", EntityType::Component, domain);
        monitor.set_status(entity_id, EntityStatus::Healthy);

        let check_id =
            monitor.add_health_check("readiness", entity_id, HealthCheckType::Readiness, None);

        let result = monitor.execute_check(check_id).unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_summary() {
        let mut monitor = CognitiveMonitor::default();
        let domain = DomainId::new(1);

        let id1 = monitor.register_entity("healthy", EntityType::Service, domain);
        let id2 = monitor.register_entity("warning", EntityType::Service, domain);
        let id3 = monitor.register_entity("critical", EntityType::Service, domain);

        monitor.set_status(id1, EntityStatus::Healthy);
        monitor.set_status(id2, EntityStatus::Warning);
        monitor.set_status(id3, EntityStatus::Critical);

        let summary = monitor.get_summary();
        assert_eq!(summary.total_entities, 3);
        assert_eq!(summary.healthy, 1);
        assert_eq!(summary.warning, 1);
        assert_eq!(summary.critical, 1);
    }
}
