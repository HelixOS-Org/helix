//! # Integration Layer
//!
//! Integration between NEXUS and the rest of Helix OS.
//!
//! ## Key Features
//!
//! - **System Hooks**: Integration points for kernel subsystems
//! - **Health Probes**: Standard health check interface
//! - **Metrics Export**: Export metrics for monitoring
//! - **Event Forwarding**: Forward events to kernel systems
//!
//! ## Architecture
//!
//! The module is organized into focused submodules:
//! - `health`: Health probes and checks
//! - `metrics`: Metric export interface
//! - `hooks`: System hooks
//! - `runtime`: Main NEXUS runtime

#![allow(dead_code)]

extern crate alloc;

// Submodules
pub mod health;
pub mod hooks;
pub mod metrics;
pub mod runtime;

// Re-export health types
pub use health::{HealthCheckResult, HealthProbe, HealthStatus};

// Re-export metrics types
pub use metrics::{Metric, MetricExporter, MetricValue};

// Re-export hooks
pub use hooks::SystemHook;

// Re-export runtime
pub use runtime::{NexusRuntime, RuntimeStats};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::NexusConfig;
    use crate::core::{ComponentId, NexusState};

    #[test]
    fn test_health_check_result() {
        let result = HealthCheckResult::healthy(ComponentId::MEMORY);
        assert_eq!(result.status, HealthStatus::Healthy);
        assert_eq!(result.health, 1.0);

        let result = HealthCheckResult::unhealthy(ComponentId::MEMORY, "Error");
        assert_eq!(result.status, HealthStatus::Unhealthy);
        assert_eq!(result.health, 0.0);
    }

    #[test]
    fn test_health_status_from_health() {
        let result = HealthCheckResult::healthy(ComponentId::MEMORY).with_health(0.85);
        assert_eq!(result.status, HealthStatus::Healthy);

        let result = HealthCheckResult::healthy(ComponentId::MEMORY).with_health(0.6);
        assert_eq!(result.status, HealthStatus::Degraded);

        let result = HealthCheckResult::healthy(ComponentId::MEMORY).with_health(0.3);
        assert_eq!(result.status, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_nexus_runtime() {
        let config = NexusConfig::default();
        let mut runtime = NexusRuntime::new(config);

        assert!(runtime.init().is_ok());
        assert!(runtime.state() == NexusState::Running);

        // Run a few ticks
        for _ in 0..10 {
            assert!(runtime.tick().is_ok());
        }

        let stats = runtime.stats();
        assert_eq!(stats.tick_count, 10);

        assert!(runtime.shutdown().is_ok());
    }

    #[test]
    fn test_metrics_export() {
        let config = NexusConfig::default();
        let runtime = NexusRuntime::new(config);

        let metrics = runtime.export_metrics().unwrap();
        assert!(!metrics.is_empty());

        // Check for expected metrics
        let tick_metric = metrics.iter().find(|m| m.name == "nexus_ticks_total");
        assert!(tick_metric.is_some());
    }
}
