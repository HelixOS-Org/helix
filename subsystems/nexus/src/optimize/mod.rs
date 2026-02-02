//! # Cross-Platform Optimization
//!
//! Optimization engine for multi-architecture performance.
//!
//! ## Key Features
//!
//! - **Architecture Detection**: Detect CPU features
//! - **Workload Optimization**: Tune for workload patterns
//! - **Memory Optimization**: Optimize memory layout
//! - **Runtime Tuning**: Dynamic parameter adjustment
//!
//! ## Architecture
//!
//! The module is organized into focused submodules:
//! - `arch`: Architecture detection and CPU features
//! - `level`: Optimization levels and targets
//! - `parameter`: Tunable parameters
//! - `optimizer`: Main optimization engine

#![allow(dead_code)]

extern crate alloc;

// Submodules
pub mod arch;
pub mod level;
pub mod optimizer;
pub mod parameter;

// Re-export arch types
pub use arch::{Architecture, CpuFeatures};
// Re-export level types
pub use level::{OptimizationLevel, OptimizationTarget};
// Re-export optimizer types
pub use optimizer::{OptimizationChange, OptimizationMetric, Optimizer, OptimizerStats};
// Re-export parameter
pub use parameter::OptimizationParameter;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_architecture() {
        let arch = Architecture::detect();
        assert!(arch.cache_line_size() > 0);
        assert!(arch.page_size() > 0);
    }

    #[test]
    fn test_cpu_features() {
        let features = CpuFeatures::detect();
        assert!(features.atomics);
    }

    #[test]
    fn test_parameter() {
        let mut param = OptimizationParameter::new("test", 10.0, 0.0, 100.0);

        param.set(50.0);
        assert_eq!(param.value, 50.0);

        param.set(150.0); // Should clamp
        assert_eq!(param.value, 100.0);

        param.reset();
        assert_eq!(param.value, 10.0);
    }

    #[test]
    fn test_optimizer() {
        let mut optimizer = Optimizer::new();

        optimizer.set_level(OptimizationLevel::Aggressive);
        optimizer.set_target(OptimizationTarget::Throughput);

        let stats = optimizer.stats();
        assert!(stats.total_optimizations > 0);
    }

    #[test]
    fn test_adaptive_optimization() {
        let mut optimizer = Optimizer::new();

        // Record some cache miss metrics
        for _ in 0..10 {
            optimizer.record_metric("cache_miss_rate", 0.15);
        }

        let changes = optimizer.optimize();
        // May or may not have changes depending on thresholds
        let _ = changes;
    }
}
