//! Regression detection for performance monitoring
//!
//! This module provides regression detection capabilities.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use super::types::Timestamp;

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    /// Latency
    Latency,
    /// Throughput
    Throughput,
    /// Accuracy
    Accuracy,
    /// Memory usage
    MemoryUsage,
    /// CPU usage
    CpuUsage,
    /// Error rate
    ErrorRate,
    /// Success rate
    SuccessRate,
}

impl MetricType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Latency => "latency",
            Self::Throughput => "throughput",
            Self::Accuracy => "accuracy",
            Self::MemoryUsage => "memory_usage",
            Self::CpuUsage => "cpu_usage",
            Self::ErrorRate => "error_rate",
            Self::SuccessRate => "success_rate",
        }
    }

    /// Is higher better
    pub fn higher_is_better(&self) -> bool {
        matches!(self, Self::Throughput | Self::Accuracy | Self::SuccessRate)
    }
}

/// Metric sample
#[derive(Debug, Clone, Copy)]
pub struct MetricSample {
    /// Timestamp
    pub timestamp: Timestamp,
    /// Value
    pub value: f32,
}

/// Metric history
#[derive(Debug, Clone)]
pub struct MetricHistory {
    /// Metric type
    pub metric_type: MetricType,
    /// Samples
    pub samples: Vec<MetricSample>,
    /// Baseline mean
    pub baseline_mean: f32,
    /// Baseline std dev
    pub baseline_std: f32,
    /// Max samples
    max_samples: usize,
}

impl MetricHistory {
    /// Create new history
    pub fn new(metric_type: MetricType) -> Self {
        Self {
            metric_type,
            samples: Vec::new(),
            baseline_mean: 0.0,
            baseline_std: 0.0,
            max_samples: 10000,
        }
    }

    /// Add sample
    pub fn add_sample(&mut self, timestamp: u64, value: f32) {
        self.samples.push(MetricSample {
            timestamp: Timestamp::new(timestamp),
            value,
        });

        if self.samples.len() > self.max_samples {
            self.samples.drain(0..self.max_samples / 10);
        }
    }

    /// Compute statistics
    pub fn compute_stats(&mut self) {
        if self.samples.is_empty() {
            return;
        }

        let sum: f32 = self.samples.iter().map(|s| s.value).sum();
        self.baseline_mean = sum / self.samples.len() as f32;

        let variance: f32 = self
            .samples
            .iter()
            .map(|s| (s.value - self.baseline_mean).powi(2))
            .sum::<f32>()
            / self.samples.len() as f32;

        // Manual sqrt approximation for no_std
        self.baseline_std = sqrt_approx(variance);
    }

    /// Recent mean
    pub fn recent_mean(&self, window: usize) -> f32 {
        let start = self.samples.len().saturating_sub(window);
        let recent = &self.samples[start..];
        if recent.is_empty() {
            return 0.0;
        }
        recent.iter().map(|s| s.value).sum::<f32>() / recent.len() as f32
    }
}

/// Approximate square root for no_std
fn sqrt_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut guess = x / 2.0;
    for _ in 0..10 {
        guess = (guess + x / guess) / 2.0;
    }
    guess
}

/// Regression severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegressionSeverity {
    /// Minor regression
    Minor,
    /// Moderate regression
    Moderate,
    /// Severe regression
    Severe,
    /// Critical regression
    Critical,
}

impl RegressionSeverity {
    /// From z-score
    pub fn from_zscore(zscore: f32) -> Option<Self> {
        match zscore.abs() {
            z if z >= 5.0 => Some(Self::Critical),
            z if z >= 3.0 => Some(Self::Severe),
            z if z >= 2.0 => Some(Self::Moderate),
            z if z >= 1.5 => Some(Self::Minor),
            _ => None,
        }
    }
}

/// Regression event
#[derive(Debug, Clone)]
pub struct RegressionEvent {
    /// Metric that regressed
    pub metric: MetricType,
    /// Timestamp detected
    pub timestamp: Timestamp,
    /// Baseline value
    pub baseline: f32,
    /// Current value
    pub current: f32,
    /// Z-score
    pub zscore: f32,
    /// Severity
    pub severity: RegressionSeverity,
}

/// Regression detector
pub struct RegressionDetector {
    /// Metric histories
    metrics: BTreeMap<String, MetricHistory>,
    /// Detected regressions
    regressions: Vec<RegressionEvent>,
    /// Detection threshold (z-score)
    threshold: f32,
    /// Window size for comparison
    window_size: usize,
    /// Is enabled
    enabled: AtomicBool,
}

impl RegressionDetector {
    /// Create new detector
    pub fn new() -> Self {
        Self {
            metrics: BTreeMap::new(),
            regressions: Vec::new(),
            threshold: 2.0,
            window_size: 100,
            enabled: AtomicBool::new(true),
        }
    }

    /// Set threshold
    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold.max(0.5);
    }

    /// Add metric
    pub fn add_metric(&mut self, name: &str, metric_type: MetricType) {
        self.metrics
            .insert(String::from(name), MetricHistory::new(metric_type));
    }

    /// Record value
    pub fn record(&mut self, name: &str, value: f32, timestamp: u64) {
        if let Some(history) = self.metrics.get_mut(name) {
            history.add_sample(timestamp, value);
        }
    }

    /// Check for regressions
    pub fn check(&mut self, timestamp: u64) -> Vec<RegressionEvent> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Vec::new();
        }

        let mut found = Vec::new();

        for history in self.metrics.values_mut() {
            history.compute_stats();

            if history.baseline_std == 0.0 {
                continue;
            }

            let recent = history.recent_mean(self.window_size);
            let zscore = (recent - history.baseline_mean) / history.baseline_std;

            // Check if regression (depends on metric direction)
            let is_regression = if history.metric_type.higher_is_better() {
                zscore < -self.threshold
            } else {
                zscore > self.threshold
            };

            if is_regression {
                if let Some(severity) = RegressionSeverity::from_zscore(zscore) {
                    let event = RegressionEvent {
                        metric: history.metric_type,
                        timestamp: Timestamp::new(timestamp),
                        baseline: history.baseline_mean,
                        current: recent,
                        zscore,
                        severity,
                    };
                    found.push(event.clone());
                    self.regressions.push(event);
                }
            }
        }

        found
    }

    /// Regression count
    pub fn regression_count(&self) -> usize {
        self.regressions.len()
    }

    /// Recent regressions
    pub fn recent_regressions(&self, limit: usize) -> &[RegressionEvent] {
        let start = self.regressions.len().saturating_sub(limit);
        &self.regressions[start..]
    }
}

impl Default for RegressionDetector {
    fn default() -> Self {
        Self::new()
    }
}
