//! Main NEXUS configuration.

use super::chaos::ChaosConfig;
use super::error::ConfigError;
use super::healing::HealingConfig;
use super::performance::PerformanceConfig;
use super::prediction::PredictionConfig;
use super::tracing::TracingConfig;
use crate::core::NexusLevel;

/// Main NEXUS configuration
#[derive(Debug, Clone)]
pub struct NexusConfig {
    /// Intelligence level
    pub level: NexusLevel,

    /// Memory budget in bytes (max memory NEXUS can use)
    pub memory_budget: usize,

    /// CPU budget as percentage (0-100)
    pub cpu_budget_percent: u8,

    /// Maximum decision time in cycles
    pub max_decision_cycles: u64,

    /// Event queue capacity
    pub event_queue_size: usize,

    /// Decision history size
    pub decision_history_size: usize,

    /// Prediction configuration
    pub prediction: PredictionConfig,

    /// Healing configuration
    pub healing: HealingConfig,

    /// Tracing configuration
    pub tracing: TracingConfig,

    /// Chaos testing configuration
    pub chaos: ChaosConfig,

    /// Performance configuration
    pub performance: PerformanceConfig,
}

impl Default for NexusConfig {
    fn default() -> Self {
        Self {
            level: NexusLevel::Healing,
            memory_budget: 16 * 1024 * 1024, // 16 MB
            cpu_budget_percent: 1,           // 1% max
            max_decision_cycles: 100_000,    // ~100µs at 1GHz
            event_queue_size: 10_000,
            decision_history_size: 1_000,
            prediction: PredictionConfig::default(),
            healing: HealingConfig::default(),
            tracing: TracingConfig::default(),
            chaos: ChaosConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

impl NexusConfig {
    /// Create minimal configuration
    pub fn minimal() -> Self {
        Self {
            level: NexusLevel::Prediction,
            memory_budget: 4 * 1024 * 1024, // 4 MB
            cpu_budget_percent: 1,
            max_decision_cycles: 50_000,
            event_queue_size: 1_000,
            decision_history_size: 100,
            prediction: PredictionConfig::minimal(),
            healing: HealingConfig::minimal(),
            tracing: TracingConfig::minimal(),
            chaos: ChaosConfig::disabled(),
            performance: PerformanceConfig::default(),
        }
    }

    /// Create full configuration
    pub fn full() -> Self {
        Self {
            level: NexusLevel::Autonomous,
            memory_budget: 64 * 1024 * 1024, // 64 MB
            cpu_budget_percent: 2,
            max_decision_cycles: 200_000,
            event_queue_size: 100_000,
            decision_history_size: 10_000,
            prediction: PredictionConfig::full(),
            healing: HealingConfig::full(),
            tracing: TracingConfig::full(),
            chaos: ChaosConfig::default(),
            performance: PerformanceConfig::aggressive(),
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.memory_budget < 1024 * 1024 {
            return Err(ConfigError::MemoryBudgetTooLow);
        }
        if self.cpu_budget_percent > 10 {
            return Err(ConfigError::CpuBudgetTooHigh);
        }
        if self.max_decision_cycles < 1000 {
            return Err(ConfigError::DecisionTimeTooShort);
        }
        if self.event_queue_size < 100 {
            return Err(ConfigError::EventQueueTooSmall);
        }
        Ok(())
    }
}
