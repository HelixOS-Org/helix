//! # Kernel Telemetry System
//!
//! Comprehensive telemetry collection and analysis for kernel intelligence.
//!
//! ## Key Features
//!
//! - **Metric Collection**: High-performance metric collection
//! - **Time Series Storage**: Efficient time series database
//! - **Aggregation**: Statistical aggregation at multiple granularities
//! - **Alerting**: Threshold-based alerting
//! - **Export**: Export to external monitoring systems

#![allow(dead_code)]

extern crate alloc;

// Submodules
mod alert;
mod histogram;
mod metrics;
mod registry;
mod series;
mod types;

// Re-exports
pub use alert::{Alert, AlertCondition, AlertRule, AlertSeverity, AlertState};
pub use histogram::TelemetryHistogram;
pub use metrics::{Counter, Gauge};
pub use registry::{TelemetryRegistry, TelemetryStats};
pub use series::{DataPoint, TimeSeries};
pub use types::{MetricDef, MetricType, MetricValue};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_series() {
        let mut ts = TimeSeries::new("test", 100);

        ts.add_value(1.0);
        ts.add_value(2.0);
        ts.add_value(3.0);

        assert_eq!(ts.len(), 3);
        assert_eq!(ts.mean(), 2.0);
        assert_eq!(ts.min(), 1.0);
        assert_eq!(ts.max(), 3.0);
    }

    #[test]
    fn test_histogram() {
        let mut h = TelemetryHistogram::new();

        h.observe(0.005);
        h.observe(0.05);
        h.observe(0.5);
        h.observe(5.0);

        assert_eq!(h.count(), 4);
        assert!((h.mean() - 1.38875).abs() < 0.001);
    }

    #[test]
    fn test_gauge() {
        let gauge = Gauge::new("test");

        gauge.set(10.0);
        assert_eq!(gauge.get(), 10.0);

        gauge.add(5.0);
        assert_eq!(gauge.get(), 15.0);

        gauge.sub(3.0);
        assert_eq!(gauge.get(), 12.0);
    }

    #[test]
    fn test_alert_condition() {
        assert!(AlertCondition::GreaterThan.evaluate(10.0, 5.0));
        assert!(!AlertCondition::GreaterThan.evaluate(5.0, 10.0));
        assert!(AlertCondition::LessThan.evaluate(5.0, 10.0));
        assert!(AlertCondition::Equal.evaluate(5.0, 5.0));
    }

    #[test]
    fn test_registry() {
        let mut registry = TelemetryRegistry::new();

        registry.register_series("cpu_usage", 100);
        registry.record("cpu_usage", 50.0);
        registry.record("cpu_usage", 60.0);

        let series = registry.get_series("cpu_usage").unwrap();
        assert_eq!(series.len(), 2);
        assert_eq!(series.mean(), 55.0);
    }

    #[test]
    fn test_downsample() {
        let mut ts = TimeSeries::new("test", 1000);

        for i in 0..100 {
            ts.add(DataPoint::new(i * 10, i as f64));
        }

        let downsampled = ts.downsample(100);
        assert!(downsampled.len() < ts.len());
    }
}
