//! Main NEXUS statistics

#![allow(dead_code)]

use crate::core::NexusTimestamp;

// ============================================================================
// MAIN STATS
// ============================================================================

/// NEXUS statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NexusStats {
    /// Boot timestamp
    pub boot_time: NexusTimestamp,
    /// Total ticks processed
    pub ticks: u64,
    /// Total events processed
    pub events_processed: u64,
    /// Total events dropped
    pub events_dropped: u64,
    /// Total decisions made
    pub decisions_made: u64,
    /// Successful decisions
    pub decisions_successful: u64,
    /// Failed decisions
    pub decisions_failed: u64,
    /// Predictions made
    pub predictions_made: u64,
    /// Predictions that were correct
    pub predictions_correct: u64,
    /// Healing attempts
    pub healing_attempts: u64,
    /// Successful healings
    pub healing_successful: u64,
    /// Failed healings
    pub healing_failed: u64,
    /// Rollbacks performed
    pub rollbacks: u64,
    /// Components quarantined
    pub quarantines: u64,
    /// Anomalies detected
    pub anomalies_detected: u64,
    /// Current system health (0.0 - 1.0)
    pub system_health: f32,
    /// Average decision time (cycles)
    pub avg_decision_time: u64,
    /// Maximum decision time seen
    pub max_decision_time: u64,
    /// Memory usage in bytes
    pub memory_usage: usize,
    /// CPU usage percentage (0-100)
    pub cpu_usage: f32,
}

impl Default for NexusStats {
    fn default() -> Self {
        Self {
            boot_time: NexusTimestamp::now(),
            ticks: 0,
            events_processed: 0,
            events_dropped: 0,
            decisions_made: 0,
            decisions_successful: 0,
            decisions_failed: 0,
            predictions_made: 0,
            predictions_correct: 0,
            healing_attempts: 0,
            healing_successful: 0,
            healing_failed: 0,
            rollbacks: 0,
            quarantines: 0,
            anomalies_detected: 0,
            system_health: 1.0,
            avg_decision_time: 0,
            max_decision_time: 0,
            memory_usage: 0,
            cpu_usage: 0.0,
        }
    }
}

impl NexusStats {
    /// Calculate uptime in cycles
    #[inline(always)]
    pub fn uptime(&self) -> u64 {
        NexusTimestamp::now().duration_since(self.boot_time)
    }

    /// Calculate prediction accuracy
    #[inline]
    pub fn prediction_accuracy(&self) -> f32 {
        if self.predictions_made == 0 {
            return 0.0;
        }
        self.predictions_correct as f32 / self.predictions_made as f32
    }

    /// Calculate healing success rate
    #[inline]
    pub fn healing_success_rate(&self) -> f32 {
        if self.healing_attempts == 0 {
            return 0.0;
        }
        self.healing_successful as f32 / self.healing_attempts as f32
    }

    /// Calculate decision success rate
    #[inline]
    pub fn decision_success_rate(&self) -> f32 {
        if self.decisions_made == 0 {
            return 0.0;
        }
        self.decisions_successful as f32 / self.decisions_made as f32
    }

    /// Calculate events per tick
    #[inline]
    pub fn events_per_tick(&self) -> f64 {
        if self.ticks == 0 {
            return 0.0;
        }
        self.events_processed as f64 / self.ticks as f64
    }

    /// Calculate drop rate
    #[inline]
    pub fn drop_rate(&self) -> f32 {
        let total = self.events_processed + self.events_dropped;
        if total == 0 {
            return 0.0;
        }
        self.events_dropped as f32 / total as f32
    }
}
