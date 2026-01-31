//! # NEXUS Configuration
//!
//! Configuration system for NEXUS with sensible defaults and validation.

#![allow(dead_code)]

mod chaos;
mod error;
mod healing;
mod main;
mod performance;
mod prediction;
mod tracing;

// Re-export main config
pub use main::NexusConfig;

// Re-export prediction config
pub use prediction::PredictionConfig;

// Re-export healing config
pub use healing::HealingConfig;

// Re-export tracing config
pub use tracing::TracingConfig;

// Re-export chaos config
pub use chaos::ChaosConfig;

// Re-export performance config
pub use performance::PerformanceConfig;

// Re-export error
pub use error::ConfigError;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NexusConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_minimal_config() {
        let config = NexusConfig::minimal();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_full_config() {
        let config = NexusConfig::full();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_memory_budget() {
        let config = NexusConfig {
            memory_budget: 100,
            ..Default::default()
        };
        assert_eq!(config.validate(), Err(ConfigError::MemoryBudgetTooLow));
    }

    #[test]
    fn test_invalid_cpu_budget() {
        let config = NexusConfig {
            cpu_budget_percent: 50,
            ..Default::default()
        };
        assert_eq!(config.validate(), Err(ConfigError::CpuBudgetTooHigh));
    }
}
