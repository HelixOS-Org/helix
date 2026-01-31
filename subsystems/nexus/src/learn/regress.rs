//! # Regression Detection
//!
//! Detects performance and behavior regressions in learned knowledge.
//! Monitors for degradation and catastrophic forgetting.
//!
//! Part of Year 2 COGNITION - Q4: Continuous Learning Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// REGRESSION TYPES
// ============================================================================

/// Metric to monitor
#[derive(Debug, Clone)]
pub struct Metric {
    /// Metric ID
    pub id: u64,
    /// Metric name
    pub name: String,
    /// Metric type
    pub metric_type: MetricType,
    /// Target value
    pub target: f64,
    /// Tolerance (acceptable deviation)
    pub tolerance: f64,
    /// Direction (higher/lower is better)
    pub direction: Direction,
    /// History
    pub history: Vec<MetricValue>,
}

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    Accuracy,
    Precision,
    Recall,
    F1Score,
    Latency,
    Throughput,
    MemoryUsage,
    Coverage,
    Custom,
}

/// Direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Higher is better
    Maximize,
    /// Lower is better
    Minimize,
}

/// Metric value
#[derive(Debug, Clone)]
pub struct MetricValue {
    /// Value
    pub value: f64,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Context
    pub context: Option<String>,
}

/// Regression
#[derive(Debug, Clone)]
pub struct Regression {
    /// Regression ID
    pub id: u64,
    /// Affected metric
    pub metric_id: u64,
    /// Severity
    pub severity: RegressionSeverity,
    /// Previous value
    pub previous: f64,
    /// Current value
    pub current: f64,
    /// Change percentage
    pub change_percent: f64,
    /// Detected at
    pub detected_at: Timestamp,
    /// Status
    pub status: RegressionStatus,
    /// Probable causes
    pub causes: Vec<ProbableCause>,
}

/// Regression severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegressionSeverity {
    Minor,
    Moderate,
    Major,
    Critical,
}

/// Regression status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegressionStatus {
    New,
    Investigating,
    Confirmed,
    Fixed,
    Accepted,
    FalsePositive,
}

/// Probable cause
#[derive(Debug, Clone)]
pub struct ProbableCause {
    /// Description
    pub description: String,
    /// Confidence
    pub confidence: f64,
    /// Related changes
    pub related_changes: Vec<u64>,
}

// ============================================================================
// BASELINE
// ============================================================================

/// Baseline for comparison
#[derive(Debug, Clone)]
pub struct Baseline {
    /// Baseline ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Metric values
    pub values: BTreeMap<u64, f64>,
    /// Created
    pub created: Timestamp,
    /// Is current baseline
    pub is_current: bool,
}

// ============================================================================
// REGRESSION DETECTOR
// ============================================================================

/// Regression detector
pub struct RegressionDetector {
    /// Metrics being monitored
    metrics: BTreeMap<u64, Metric>,
    /// Detected regressions
    regressions: BTreeMap<u64, Regression>,
    /// Baselines
    baselines: BTreeMap<u64, Baseline>,
    /// Current baseline
    current_baseline: Option<u64>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: DetectorConfig,
    /// Statistics
    stats: DetectorStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct DetectorConfig {
    /// Sensitivity (lower = more sensitive)
    pub sensitivity: f64,
    /// Minimum samples for baseline
    pub min_samples: usize,
    /// Enable trend analysis
    pub trend_analysis: bool,
    /// Window size for trend
    pub trend_window: usize,
    /// Auto-accept minor regressions
    pub auto_accept_minor: bool,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            sensitivity: 0.05, // 5% change threshold
            min_samples: 10,
            trend_analysis: true,
            trend_window: 10,
            auto_accept_minor: false,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct DetectorStats {
    /// Metrics monitored
    pub metrics_monitored: u64,
    /// Samples recorded
    pub samples_recorded: u64,
    /// Regressions detected
    pub regressions_detected: u64,
    /// False positives
    pub false_positives: u64,
    /// Average response time (ns)
    pub avg_response_ns: f64,
}

impl RegressionDetector {
    /// Create new detector
    pub fn new(config: DetectorConfig) -> Self {
        Self {
            metrics: BTreeMap::new(),
            regressions: BTreeMap::new(),
            baselines: BTreeMap::new(),
            current_baseline: None,
            next_id: AtomicU64::new(1),
            config,
            stats: DetectorStats::default(),
        }
    }

    /// Register metric
    pub fn register_metric(
        &mut self,
        name: &str,
        metric_type: MetricType,
        target: f64,
        tolerance: f64,
        direction: Direction,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let metric = Metric {
            id,
            name: name.into(),
            metric_type,
            target,
            tolerance,
            direction,
            history: Vec::new(),
        };

        self.metrics.insert(id, metric);
        self.stats.metrics_monitored += 1;

        id
    }

    /// Record metric value
    pub fn record(&mut self, metric_id: u64, value: f64, context: Option<&str>) {
        if let Some(metric) = self.metrics.get_mut(&metric_id) {
            metric.history.push(MetricValue {
                value,
                timestamp: Timestamp::now(),
                context: context.map(String::from),
            });

            self.stats.samples_recorded += 1;
        }
    }

    /// Check for regressions
    pub fn check(&mut self) -> Vec<u64> {
        let mut detected = Vec::new();

        let baseline_values = self
            .current_baseline
            .and_then(|id| self.baselines.get(&id))
            .map(|b| b.values.clone())
            .unwrap_or_default();

        for metric in self.metrics.values() {
            if metric.history.is_empty() {
                continue;
            }

            let current = metric.history.last().unwrap().value;

            // Compare against baseline or target
            let baseline = baseline_values
                .get(&metric.id)
                .copied()
                .unwrap_or(metric.target);

            let change = current - baseline;
            let change_percent = if baseline.abs() > f64::EPSILON {
                (change / baseline).abs() * 100.0
            } else {
                0.0
            };

            // Determine if regression
            let is_regression = match metric.direction {
                Direction::Maximize => current < baseline - metric.tolerance,
                Direction::Minimize => current > baseline + metric.tolerance,
            };

            if is_regression && change_percent > self.config.sensitivity * 100.0 {
                let regression_id =
                    self.create_regression(metric.id, baseline, current, change_percent);
                detected.push(regression_id);
            }
        }

        detected
    }

    fn create_regression(
        &mut self,
        metric_id: u64,
        previous: f64,
        current: f64,
        change_percent: f64,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let severity = if change_percent > 50.0 {
            RegressionSeverity::Critical
        } else if change_percent > 25.0 {
            RegressionSeverity::Major
        } else if change_percent > 10.0 {
            RegressionSeverity::Moderate
        } else {
            RegressionSeverity::Minor
        };

        let status = if self.config.auto_accept_minor && severity == RegressionSeverity::Minor {
            RegressionStatus::Accepted
        } else {
            RegressionStatus::New
        };

        let causes = self.analyze_causes(metric_id);

        let regression = Regression {
            id,
            metric_id,
            severity,
            previous,
            current,
            change_percent,
            detected_at: Timestamp::now(),
            status,
            causes,
        };

        self.regressions.insert(id, regression);
        self.stats.regressions_detected += 1;

        id
    }

    fn analyze_causes(&self, _metric_id: u64) -> Vec<ProbableCause> {
        // Simplified cause analysis
        vec![ProbableCause {
            description: "Recent model update".into(),
            confidence: 0.6,
            related_changes: Vec::new(),
        }]
    }

    /// Analyze trend
    pub fn analyze_trend(&self, metric_id: u64) -> Option<TrendAnalysis> {
        let metric = self.metrics.get(&metric_id)?;

        if metric.history.len() < self.config.trend_window {
            return None;
        }

        let window: Vec<f64> = metric
            .history
            .iter()
            .rev()
            .take(self.config.trend_window)
            .map(|v| v.value)
            .collect();

        // Calculate trend
        let n = window.len() as f64;
        let sum_x: f64 = (0..window.len()).map(|i| i as f64).sum();
        let sum_y: f64 = window.iter().sum();
        let sum_xy: f64 = window.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
        let sum_xx: f64 = (0..window.len()).map(|i| (i * i) as f64).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        let direction = if slope.abs() < 0.01 {
            TrendDirection::Stable
        } else if slope > 0.0 {
            match metric.direction {
                Direction::Maximize => TrendDirection::Improving,
                Direction::Minimize => TrendDirection::Degrading,
            }
        } else {
            match metric.direction {
                Direction::Maximize => TrendDirection::Degrading,
                Direction::Minimize => TrendDirection::Improving,
            }
        };

        // Variance
        let mean = sum_y / n;
        let variance = window.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;

        Some(TrendAnalysis {
            metric_id,
            direction,
            slope,
            intercept,
            variance,
            confidence: 1.0 / (1.0 + variance),
        })
    }

    /// Create baseline
    pub fn create_baseline(&mut self, name: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Collect current values
        let values: BTreeMap<u64, f64> = self
            .metrics
            .iter()
            .filter_map(|(&metric_id, metric)| metric.history.last().map(|v| (metric_id, v.value)))
            .collect();

        // Mark old baseline as not current
        if let Some(old_id) = self.current_baseline {
            if let Some(old) = self.baselines.get_mut(&old_id) {
                old.is_current = false;
            }
        }

        let baseline = Baseline {
            id,
            name: name.into(),
            values,
            created: Timestamp::now(),
            is_current: true,
        };

        self.baselines.insert(id, baseline);
        self.current_baseline = Some(id);

        id
    }

    /// Update regression status
    pub fn update_status(&mut self, regression_id: u64, status: RegressionStatus) {
        if let Some(regression) = self.regressions.get_mut(&regression_id) {
            regression.status = status;

            if status == RegressionStatus::FalsePositive {
                self.stats.false_positives += 1;
            }
        }
    }

    /// Get metric
    pub fn get_metric(&self, id: u64) -> Option<&Metric> {
        self.metrics.get(&id)
    }

    /// Get regression
    pub fn get_regression(&self, id: u64) -> Option<&Regression> {
        self.regressions.get(&id)
    }

    /// Get active regressions
    pub fn active_regressions(&self) -> Vec<&Regression> {
        self.regressions
            .values()
            .filter(|r| {
                matches!(
                    r.status,
                    RegressionStatus::New
                        | RegressionStatus::Investigating
                        | RegressionStatus::Confirmed
                )
            })
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &DetectorStats {
        &self.stats
    }
}

impl Default for RegressionDetector {
    fn default() -> Self {
        Self::new(DetectorConfig::default())
    }
}

/// Trend analysis
#[derive(Debug, Clone)]
pub struct TrendAnalysis {
    /// Metric ID
    pub metric_id: u64,
    /// Direction
    pub direction: TrendDirection,
    /// Slope
    pub slope: f64,
    /// Intercept
    pub intercept: f64,
    /// Variance
    pub variance: f64,
    /// Confidence
    pub confidence: f64,
}

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirection {
    Improving,
    Degrading,
    Stable,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_registration() {
        let mut detector = RegressionDetector::default();

        let id = detector.register_metric(
            "accuracy",
            MetricType::Accuracy,
            0.95,
            0.02,
            Direction::Maximize,
        );

        assert!(detector.get_metric(id).is_some());
    }

    #[test]
    fn test_record_and_check() {
        let mut detector = RegressionDetector::default();

        let id = detector.register_metric(
            "accuracy",
            MetricType::Accuracy,
            0.95,
            0.02,
            Direction::Maximize,
        );

        // Record good value
        detector.record(id, 0.95, None);
        detector.create_baseline("initial");

        // Record bad value (significant drop)
        detector.record(id, 0.70, Some("after update"));

        let regressions = detector.check();
        assert!(!regressions.is_empty());
    }

    #[test]
    fn test_trend_analysis() {
        let mut detector = RegressionDetector::new(DetectorConfig {
            trend_window: 5,
            ..Default::default()
        });

        let id = detector.register_metric(
            "accuracy",
            MetricType::Accuracy,
            0.95,
            0.02,
            Direction::Maximize,
        );

        // Degrading trend
        for i in 0..10 {
            detector.record(id, 0.95 - (i as f64 * 0.02), None);
        }

        let trend = detector.analyze_trend(id);
        assert!(trend.is_some());
        assert_eq!(trend.unwrap().direction, TrendDirection::Degrading);
    }

    #[test]
    fn test_baseline_creation() {
        let mut detector = RegressionDetector::default();

        let id = detector.register_metric("m1", MetricType::Custom, 1.0, 0.1, Direction::Maximize);
        detector.record(id, 0.9, None);

        let baseline_id = detector.create_baseline("v1");
        assert!(detector.baselines.get(&baseline_id).is_some());
    }
}
