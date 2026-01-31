//! Healing configuration.

/// Configuration for self-healing
#[derive(Debug, Clone)]
pub struct HealingConfig {
    /// Enable self-healing
    pub enabled: bool,

    /// Enable micro-rollback
    pub enable_micro_rollback: bool,

    /// Enable state reconstruction
    pub enable_reconstruction: bool,

    /// Enable component substitution
    pub enable_substitution: bool,

    /// Maximum rollback depth
    pub max_rollback_depth: usize,

    /// Checkpoint interval in milliseconds
    pub checkpoint_interval_ms: u64,

    /// Maximum healing attempts before escalation
    pub max_healing_attempts: u32,

    /// Healing timeout in milliseconds
    pub healing_timeout_ms: u64,

    /// Enable quarantine for failing components
    pub enable_quarantine: bool,
}

impl Default for HealingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_micro_rollback: true,
            enable_reconstruction: true,
            enable_substitution: false,
            max_rollback_depth: 10,
            checkpoint_interval_ms: 1000,
            max_healing_attempts: 3,
            healing_timeout_ms: 500,
            enable_quarantine: true,
        }
    }
}

impl HealingConfig {
    /// Minimal healing configuration
    pub fn minimal() -> Self {
        Self {
            enabled: true,
            enable_micro_rollback: true,
            enable_reconstruction: false,
            enable_substitution: false,
            max_rollback_depth: 3,
            checkpoint_interval_ms: 5000,
            max_healing_attempts: 2,
            healing_timeout_ms: 200,
            enable_quarantine: false,
        }
    }

    /// Full healing configuration
    pub fn full() -> Self {
        Self {
            enabled: true,
            enable_micro_rollback: true,
            enable_reconstruction: true,
            enable_substitution: true,
            max_rollback_depth: 50,
            checkpoint_interval_ms: 500,
            max_healing_attempts: 5,
            healing_timeout_ms: 1000,
            enable_quarantine: true,
        }
    }
}
