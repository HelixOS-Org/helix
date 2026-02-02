//! # Chaos Engineering
//!
//! Controlled fault injection to improve system resilience.
//!
//! ## Key Features
//!
//! - **Fault Injection**: Memory, CPU, I/O, and latency faults
//! - **Chaos Experiments**: Reproducible failure scenarios
//! - **Blast Radius Control**: Limit impact of faults
//! - **Automatic Recovery**: Ensure faults are cleaned up
//!
//! ## Architecture
//!
//! The module is organized into focused submodules:
//! - `types`: Fault type definitions
//! - `target`: Fault targeting
//! - `config`: Fault configuration
//! - `fault`: Active fault representation
//! - `experiment`: Chaos experiments and results
//! - `engine`: Main chaos engine

#![allow(dead_code)]

extern crate alloc;

// Submodules
pub mod config;
pub mod engine;
pub mod experiment;
pub mod fault;
pub mod target;
pub mod types;

// Re-export types
// Re-export config
pub use config::FaultConfig;
// Re-export engine
pub use engine::{ChaosEngine, ChaosSafety};
// Re-export experiment
pub use experiment::{ChaosExperiment, ExperimentResults};
// Re-export fault
pub use fault::Fault;
// Re-export target
pub use target::FaultTarget;
pub use types::FaultType;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fault_type_severity() {
        assert!(FaultType::Panic.severity() > FaultType::Latency.severity());
        assert!(FaultType::Panic.is_destructive());
        assert!(!FaultType::Latency.is_destructive());
    }

    #[test]
    fn test_fault_config() {
        let config = FaultConfig::latency(1000)
            .with_probability(0.05)
            .with_max_occurrences(10);

        assert_eq!(config.fault_type, FaultType::Latency);
        assert_eq!(config.latency_cycles, Some(1000));
        assert!((config.probability - 0.05).abs() < 0.001);
        assert_eq!(config.max_occurrences, Some(10));
    }

    #[test]
    fn test_chaos_engine_disabled() {
        let mut engine = ChaosEngine::new();

        // Engine is disabled by default
        assert!(!engine.is_enabled());

        // Injection should fail when disabled
        let result = engine.inject(FaultConfig::latency(1000));
        assert!(result.is_none());
    }

    #[test]
    fn test_chaos_engine_enabled() {
        let mut engine = ChaosEngine::new();
        engine.enable();

        let result = engine.inject(FaultConfig::latency(1000));
        assert!(result.is_some());

        assert_eq!(engine.active_faults().len(), 1);
    }

    #[test]
    fn test_safety_blocks_destructive() {
        let mut engine = ChaosEngine::new();
        engine.enable();

        // Destructive faults should be blocked
        let config = FaultConfig {
            fault_type: FaultType::Panic,
            ..Default::default()
        };

        let result = engine.inject(config);
        assert!(result.is_none());
    }

    #[test]
    fn test_experiment() {
        let mut exp = ChaosExperiment::new("test");
        exp.add_fault(FaultConfig::latency(1000));

        assert!(!exp.running);
        exp.start();
        assert!(exp.running);
        exp.stop();
        assert!(!exp.running);
    }
}
