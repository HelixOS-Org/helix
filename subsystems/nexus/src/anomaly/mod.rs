//! # Anomaly Detection Engine
//!
//! Real-time anomaly detection using statistical methods and pattern analysis.
//!
//! ## Key Innovations
//!
//! - **Z-Score Detection**: Statistical outlier detection
//! - **IQR Method**: Interquartile range for robust outlier detection
//! - **Moving Average**: Trend-adjusted anomaly detection
//! - **Pattern Matching**: Detect known failure patterns
//!
//! ## Architecture
//!
//! The module is organized into focused submodules:
//! - `types`: Severity and anomaly type definitions
//! - `anomaly`: Anomaly representation
//! - `config`: Detector configuration
//! - `stats`: Metric statistics
//! - `detector`: Main anomaly detector
//! - `pattern`: Pattern library for known anomalies

#![allow(dead_code)]
#![allow(clippy::module_inception)]

extern crate alloc;

// Submodules
pub mod anomaly;
pub mod config;
pub mod detector;
pub mod pattern;
pub mod stats;
pub mod types;

// Re-export types
// Re-export anomaly
pub use anomaly::Anomaly;
// Re-export config
pub use config::DetectorConfig;
// Re-export detector
pub use detector::AnomalyDetector;
// Re-export pattern
pub use pattern::{AnomalyPattern, PatternLibrary};
// Re-export stats
pub use stats::MetricStats;
pub use types::{AnomalySeverity, AnomalyType};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anomaly_severity() {
        assert_eq!(AnomalySeverity::from_score(0.95), AnomalySeverity::Critical);
        assert_eq!(AnomalySeverity::from_score(0.5), AnomalySeverity::Moderate);
        assert_eq!(AnomalySeverity::from_score(0.1), AnomalySeverity::Warning);
    }

    #[test]
    fn test_metric_stats() {
        let mut stats = MetricStats::new("test", 10);

        for i in 0..10 {
            stats.add(i as f64);
        }

        assert!((stats.mean() - 4.5).abs() < 0.001);
        assert!(stats.std_dev() > 0.0);
    }

    #[test]
    fn test_zscore_detection() {
        let stats = MetricStats::new("test", 10);
        // Empty stats should return 0
        assert_eq!(stats.z_score(100.0), 0.0);
    }

    #[test]
    fn test_anomaly_detector() {
        let mut detector = AnomalyDetector::default();
        detector.register_metric("cpu");

        // Add normal values
        for i in 0..50 {
            detector.record("cpu", 50.0 + (i % 5) as f64, None);
        }

        // Add anomalous value
        let anomaly = detector.record("cpu", 500.0, None);
        assert!(anomaly.is_some());
    }

    #[test]
    fn test_quartiles() {
        let mut stats = MetricStats::new("test", 100);

        for i in 1..=100 {
            stats.add(i as f64);
        }

        let (q1, q2, q3) = stats.quartiles();
        assert!(q1 < q2);
        assert!(q2 < q3);
    }
}
