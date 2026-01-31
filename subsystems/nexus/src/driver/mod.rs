//! # Driver Intelligence Module
//!
//! AI-powered driver monitoring and optimization.
//!
//! ## Key Features
//!
//! - **Health Monitoring**: Monitor driver health
//! - **Performance Profiling**: Profile driver performance
//! - **Fault Prediction**: Predict driver failures
//! - **Resource Tracking**: Track driver resource usage
//! - **Hot-reload Support**: Support for driver hot-reload
//! - **Compatibility Analysis**: Analyze driver compatibility

#![allow(dead_code)]

extern crate alloc;

// Submodules
mod compat;
mod fault;
mod health;
mod intelligence;
mod metrics;
mod resource;
mod types;

// Re-exports
pub use compat::{
    CompatibilityAnalyzer, CompatibilityIssue, CompatibilityIssueType, CompatibilitySeverity,
    DriverConflict,
};
pub use fault::{DriverFaultPredictor, FaultPrediction, FaultType};
pub use health::{DriverHealthMonitor, HealthEvent, HealthEventType};
pub use intelligence::DriverIntelligence;
pub use metrics::DriverMetrics;
pub use resource::{
    DriverResourceTracker, ResourceLimits, ResourceViolation, ResourceViolationType,
};
pub use types::{DeviceClass, DriverId, DriverInfo, DriverState, HealthLevel};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_info() {
        let mut info = DriverInfo::new(1, "test_driver", DeviceClass::Storage);
        assert_eq!(info.state, DriverState::Unloaded);

        info.mark_loaded();
        assert_eq!(info.state, DriverState::Running);
        assert!(info.loaded_at.is_some());
    }

    #[test]
    fn test_driver_metrics() {
        let mut metrics = DriverMetrics::default();

        metrics.record_operation(true, 1000);
        metrics.record_operation(true, 2000);
        metrics.record_operation(false, 5000);

        assert_eq!(metrics.total_ops, 3);
        assert_eq!(metrics.successful_ops, 2);
        assert_eq!(metrics.failed_ops, 1);
    }

    #[test]
    fn test_health_monitor() {
        let mut monitor = DriverHealthMonitor::default();
        let mut metrics = DriverMetrics::default();

        for _ in 0..10 {
            metrics.record_operation(true, 1000);
        }

        monitor.record(1, &metrics);
        let score = monitor.get_score(1);
        assert!(score > 0.9);
    }

    #[test]
    fn test_resource_limits() {
        let mut tracker = DriverResourceTracker::default();
        tracker.set_limits(1, ResourceLimits::default());

        // Record normal usage
        let violations = tracker.record(1, 100_000_000, 5.0, 10, 2);
        assert!(violations.is_empty());

        // Record excessive memory
        let violations = tracker.record(1, 500_000_000, 5.0, 10, 2);
        assert!(!violations.is_empty());
    }
}
