//! Main anomaly detector

#![allow(dead_code)]

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::anomaly::Anomaly;
use super::config::DetectorConfig;
use super::stats::MetricStats;
use super::types::{AnomalySeverity, AnomalyType};
use crate::core::ComponentId;

// ============================================================================
// ANOMALY DETECTOR
// ============================================================================

/// Main anomaly detector
pub struct AnomalyDetector {
    /// Configuration
    config: DetectorConfig,
    /// Metric statistics
    metrics: BTreeMap<String, MetricStats>,
    /// Recent anomalies
    anomalies: VecDeque<Anomaly>,
    /// Maximum anomalies to keep
    max_anomalies: usize,
    /// Total anomalies detected
    total_detected: AtomicU64,
    /// Is detector enabled
    enabled: bool,
}

impl AnomalyDetector {
    /// Create a new detector
    pub fn new(config: DetectorConfig) -> Self {
        Self {
            config,
            metrics: BTreeMap::new(),
            anomalies: VecDeque::new(),
            max_anomalies: 1000,
            total_detected: AtomicU64::new(0),
            enabled: true,
        }
    }

    /// Register a metric to monitor
    #[inline]
    pub fn register_metric(&mut self, name: impl Into<String>) {
        let name = name.into();
        if !self.metrics.contains_key(&name) {
            self.metrics.insert(
                name.clone(),
                MetricStats::new(name, self.config.window_size),
            );
        }
    }

    /// Record a metric value and check for anomalies
    pub fn record(
        &mut self,
        metric: &str,
        value: f64,
        component: Option<ComponentId>,
    ) -> Option<Anomaly> {
        if !self.enabled {
            return None;
        }

        // Get or create metric stats
        let stats = self
            .metrics
            .entry(metric.to_string())
            .or_insert_with(|| MetricStats::new(metric, self.config.window_size));

        // Don't detect until we have enough samples
        if stats.values.len() < self.config.min_samples {
            stats.add(value);
            return None;
        }

        let mean = stats.mean();
        let std_dev = stats.std_dev();

        // Check for anomalies
        let mut anomaly: Option<Anomaly> = None;

        // Z-score detection
        if self.config.enable_zscore && std_dev > 0.0 {
            let z = stats.z_score(value);
            if z.abs() > self.config.z_score_threshold {
                let atype = if z > 0.0 {
                    AnomalyType::Spike
                } else {
                    AnomalyType::Drop
                };
                anomaly = Some(
                    Anomaly::new(atype, metric, value, mean)
                        .with_context("z_score", z)
                        .with_context("std_dev", std_dev),
                );
            }
        }

        // IQR detection
        if anomaly.is_none()
            && self.config.enable_iqr
            && stats.is_iqr_outlier(value, self.config.iqr_multiplier)
        {
            let (q1, _q2, q3) = stats.quartiles();
            anomaly = Some(
                Anomaly::new(AnomalyType::OutOfRange, metric, value, mean)
                    .with_context("q1", q1)
                    .with_context("q3", q3)
                    .with_context("iqr", stats.iqr()),
            );
        }

        // Trend detection
        if anomaly.is_none() && self.config.enable_trend {
            let gradient = stats.gradient();
            if gradient.abs() > 0.1 {
                anomaly = Some(
                    Anomaly::new(AnomalyType::TrendChange, metric, value, mean)
                        .with_context("gradient", gradient),
                );
            }
        }

        // Add value to stats
        stats.add(value);

        // Process anomaly
        if let Some(mut anom) = anomaly.take() {
            if let Some(comp) = component {
                anom = anom.with_component(comp);
            }

            self.total_detected.fetch_add(1, Ordering::Relaxed);

            // Store anomaly
            if self.anomalies.len() >= self.max_anomalies {
                self.anomalies.pop_front();
            }
            self.anomalies.push_back(anom.clone());

            return Some(anom);
        }

        None
    }

    /// Get metric stats
    #[inline(always)]
    pub fn get_metric(&self, name: &str) -> Option<&MetricStats> {
        self.metrics.get(name)
    }

    /// Get recent anomalies
    #[inline(always)]
    pub fn recent_anomalies(&self) -> &[Anomaly] {
        &self.anomalies
    }

    /// Get anomalies for a specific component
    #[inline]
    pub fn anomalies_for(&self, component: ComponentId) -> Vec<&Anomaly> {
        self.anomalies
            .iter()
            .filter(|a| a.component == Some(component))
            .collect()
    }

    /// Get anomalies above severity
    #[inline]
    pub fn anomalies_above(&self, severity: AnomalySeverity) -> Vec<&Anomaly> {
        self.anomalies
            .iter()
            .filter(|a| a.severity >= severity)
            .collect()
    }

    /// Get total anomalies detected
    #[inline(always)]
    pub fn total_detected(&self) -> u64 {
        self.total_detected.load(Ordering::Relaxed)
    }

    /// Clear anomaly history
    #[inline(always)]
    pub fn clear_history(&mut self) {
        self.anomalies.clear();
    }

    /// Enable/disable detector
    #[inline(always)]
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Get configuration
    #[inline(always)]
    pub fn config(&self) -> &DetectorConfig {
        &self.config
    }
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self::new(DetectorConfig::default())
    }
}
