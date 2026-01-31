//! Central telemetry registry.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::alert::{Alert, AlertRule, AlertState};
use super::histogram::TelemetryHistogram;
use super::series::TimeSeries;

// ============================================================================
// TELEMETRY REGISTRY
// ============================================================================

/// Central telemetry registry
pub struct TelemetryRegistry {
    /// Time series by name
    series: BTreeMap<String, TimeSeries>,
    /// Histograms by name
    histograms: BTreeMap<String, TelemetryHistogram>,
    /// Counters by name
    counters: BTreeMap<String, u64>,
    /// Gauges by name
    gauges: BTreeMap<String, f64>,
    /// Alert rules
    alert_rules: Vec<AlertRule>,
    /// Active alerts
    active_alerts: Vec<Alert>,
    /// Is enabled?
    enabled: AtomicBool,
    /// Total samples recorded
    total_samples: AtomicU64,
}

impl TelemetryRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            series: BTreeMap::new(),
            histograms: BTreeMap::new(),
            counters: BTreeMap::new(),
            gauges: BTreeMap::new(),
            alert_rules: Vec::new(),
            active_alerts: Vec::new(),
            enabled: AtomicBool::new(true),
            total_samples: AtomicU64::new(0),
        }
    }

    /// Register time series
    pub fn register_series(&mut self, name: impl Into<String>, max_points: usize) {
        let name = name.into();
        self.series
            .insert(name.clone(), TimeSeries::new(name, max_points));
    }

    /// Register histogram
    pub fn register_histogram(&mut self, name: impl Into<String>) {
        self.histograms
            .insert(name.into(), TelemetryHistogram::new());
    }

    /// Record to time series
    pub fn record(&mut self, name: &str, value: f64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        if let Some(series) = self.series.get_mut(name) {
            series.add_value(value);
            self.total_samples.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Observe histogram value
    pub fn observe(&mut self, name: &str, value: f64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        if let Some(histogram) = self.histograms.get_mut(name) {
            histogram.observe(value);
            self.total_samples.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Increment counter
    pub fn inc_counter(&mut self, name: &str) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        *self.counters.entry(name.into()).or_insert(0) += 1;
        self.total_samples.fetch_add(1, Ordering::Relaxed);
    }

    /// Add to counter
    pub fn add_counter(&mut self, name: &str, amount: u64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        *self.counters.entry(name.into()).or_insert(0) += amount;
        self.total_samples.fetch_add(1, Ordering::Relaxed);
    }

    /// Set gauge
    pub fn set_gauge(&mut self, name: &str, value: f64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        self.gauges.insert(name.into(), value);
        self.total_samples.fetch_add(1, Ordering::Relaxed);
    }

    /// Get time series
    pub fn get_series(&self, name: &str) -> Option<&TimeSeries> {
        self.series.get(name)
    }

    /// Get histogram
    pub fn get_histogram(&self, name: &str) -> Option<&TelemetryHistogram> {
        self.histograms.get(name)
    }

    /// Get counter
    pub fn get_counter(&self, name: &str) -> Option<u64> {
        self.counters.get(name).copied()
    }

    /// Get gauge
    pub fn get_gauge(&self, name: &str) -> Option<f64> {
        self.gauges.get(name).copied()
    }

    /// Add alert rule
    pub fn add_alert_rule(&mut self, rule: AlertRule) {
        self.alert_rules.push(rule);
    }

    /// Evaluate alerts
    pub fn evaluate_alerts(&mut self) {
        for rule in &self.alert_rules {
            // Get current value
            let value = if let Some(series) = self.series.get(&rule.metric) {
                series.latest()
            } else if let Some(&gauge) = self.gauges.get(&rule.metric) {
                Some(gauge)
            } else if let Some(&counter) = self.counters.get(&rule.metric) {
                Some(counter as f64)
            } else {
                None
            };

            if let Some(value) = value {
                if rule.condition.evaluate(value, rule.threshold) {
                    // Check if alert already exists
                    let existing = self
                        .active_alerts
                        .iter_mut()
                        .find(|a| a.rule == rule.name && a.state != AlertState::Resolved);

                    if let Some(alert) = existing {
                        // Update value
                        alert.value = value;
                        // Check if should fire
                        if alert.state == AlertState::Pending {
                            if alert.duration() >= rule.for_duration {
                                alert.fire();
                            }
                        }
                    } else {
                        // Create new alert
                        self.active_alerts.push(Alert::new(rule, value));
                    }
                } else {
                    // Resolve any existing alert
                    for alert in &mut self.active_alerts {
                        if alert.rule == rule.name && alert.state != AlertState::Resolved {
                            alert.resolve();
                        }
                    }
                }
            }
        }

        // Clean up old resolved alerts (keep last 100)
        self.active_alerts
            .retain(|a| a.state != AlertState::Resolved || a.duration() < 3600_000_000_000);

        if self.active_alerts.len() > 100 {
            // Remove oldest resolved
            self.active_alerts
                .retain(|a| a.state != AlertState::Resolved);
        }
    }

    /// Get firing alerts
    pub fn firing_alerts(&self) -> Vec<&Alert> {
        self.active_alerts
            .iter()
            .filter(|a| a.state == AlertState::Firing)
            .collect()
    }

    /// Get all alerts
    pub fn all_alerts(&self) -> &[Alert] {
        &self.active_alerts
    }

    /// Enable registry
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable registry
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Get statistics
    pub fn stats(&self) -> TelemetryStats {
        TelemetryStats {
            series_count: self.series.len(),
            histogram_count: self.histograms.len(),
            counter_count: self.counters.len(),
            gauge_count: self.gauges.len(),
            alert_rule_count: self.alert_rules.len(),
            active_alert_count: self
                .active_alerts
                .iter()
                .filter(|a| a.state == AlertState::Firing)
                .count(),
            total_samples: self.total_samples.load(Ordering::Relaxed),
        }
    }

    /// Export as text (Prometheus-like format)
    pub fn export_text(&self) -> String {
        let mut output = String::new();

        // Counters
        for (name, &value) in &self.counters {
            output.push_str(&format!("{} {}\n", name, value));
        }

        // Gauges
        for (name, &value) in &self.gauges {
            output.push_str(&format!("{} {}\n", name, value));
        }

        // Series (latest value)
        for (name, series) in &self.series {
            if let Some(value) = series.latest() {
                output.push_str(&format!("{} {}\n", name, value));
            }
        }

        // Histograms
        for (name, histogram) in &self.histograms {
            output.push_str(&format!("{}_count {}\n", name, histogram.count()));
            output.push_str(&format!("{}_sum {}\n", name, histogram.sum()));
            for (boundary, count) in histogram.buckets() {
                if boundary.is_finite() {
                    output.push_str(&format!(
                        "{}_bucket{{le=\"{}\"}} {}\n",
                        name, boundary, count
                    ));
                } else {
                    output.push_str(&format!("{}_bucket{{le=\"+Inf\"}} {}\n", name, count));
                }
            }
        }

        output
    }
}

impl Default for TelemetryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TELEMETRY STATISTICS
// ============================================================================

/// Telemetry statistics
#[derive(Debug, Clone)]
pub struct TelemetryStats {
    /// Number of time series
    pub series_count: usize,
    /// Number of histograms
    pub histogram_count: usize,
    /// Number of counters
    pub counter_count: usize,
    /// Number of gauges
    pub gauge_count: usize,
    /// Number of alert rules
    pub alert_rule_count: usize,
    /// Number of active (firing) alerts
    pub active_alert_count: usize,
    /// Total samples recorded
    pub total_samples: u64,
}
