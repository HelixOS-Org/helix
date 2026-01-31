//! # Degradation Detection
//!
//! Detect and respond to system degradation before failures occur.
//!
//! ## Key Features
//!
//! - **Performance Degradation**: Detect slowdowns
//! - **Resource Degradation**: Memory leaks, handle exhaustion
//! - **Quality Degradation**: Error rate increases
//! - **Automatic Response**: Mitigate degradation automatically

#![allow(dead_code)]

extern crate alloc;

mod baseline;
mod detector;
mod event;
mod types;

// Re-export types
pub use types::{DegradationSeverity, DegradationType};

// Re-export event
pub use event::DegradationEvent;

// Re-export detector
pub use detector::{DegradationDetector, DegradationStats};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_degradation_severity() {
        assert_eq!(
            DegradationSeverity::from_percentage(5.0),
            DegradationSeverity::Minor
        );
        assert_eq!(
            DegradationSeverity::from_percentage(15.0),
            DegradationSeverity::Moderate
        );
        assert_eq!(
            DegradationSeverity::from_percentage(35.0),
            DegradationSeverity::Significant
        );
        assert_eq!(
            DegradationSeverity::from_percentage(60.0),
            DegradationSeverity::Severe
        );
        assert_eq!(
            DegradationSeverity::from_percentage(80.0),
            DegradationSeverity::Critical
        );
    }

    #[test]
    fn test_degradation_event() {
        let event = DegradationEvent::new(DegradationType::Performance, 100.0, 150.0);

        assert_eq!(event.degradation_pct, 50.0);
        assert_eq!(event.severity, DegradationSeverity::Significant);
    }

    #[test]
    fn test_metric_baseline() {
        use baseline::MetricBaseline;

        let mut baseline = MetricBaseline::new(100.0);

        // Update with same values
        for _ in 0..10 {
            baseline.update(100.0, 0.1);
        }

        assert!(baseline.mean > 99.0 && baseline.mean < 101.0);

        // Update with higher values
        for _ in 0..10 {
            baseline.update(150.0, 0.1);
        }

        // EMA should have moved toward 150
        assert!(baseline.ema > 100.0);
    }

    #[test]
    fn test_detector() {
        let mut detector = DegradationDetector::new().with_pct_threshold(10.0);

        // Warmup
        for _ in 0..100 {
            detector.record("test", 100.0, DegradationType::Performance, None);
        }

        // Should not detect during warmup
        assert!(detector.events().is_empty());

        // Record degraded values
        for _ in 0..10 {
            detector.record("test", 150.0, DegradationType::Performance, None);
        }

        // Should have detected degradation
        assert!(!detector.events().is_empty());
    }
}
