//! Component, prediction, and healing statistics

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;

use crate::core::NexusTimestamp;

// ============================================================================
// COMPONENT STATS
// ============================================================================

/// Statistics for a single component
#[derive(Debug, Clone)]
pub struct ComponentStats {
    /// Component name
    pub name: String,
    /// Current health (0.0 - 1.0)
    pub health: f32,
    /// Events processed
    pub events_processed: u64,
    /// Errors encountered
    pub errors: u64,
    /// Last error timestamp
    pub last_error: Option<NexusTimestamp>,
    /// Healing count
    pub healed_count: u64,
    /// Rollback count
    pub rollback_count: u64,
    /// Is quarantined
    pub is_quarantined: bool,
    /// Average processing time (cycles)
    pub avg_processing_time: u64,
}

impl Default for ComponentStats {
    fn default() -> Self {
        Self {
            name: String::new(),
            health: 1.0,
            events_processed: 0,
            errors: 0,
            last_error: None,
            healed_count: 0,
            rollback_count: 0,
            is_quarantined: false,
            avg_processing_time: 0,
        }
    }
}

// ============================================================================
// PREDICTION STATS
// ============================================================================

/// Statistics for prediction engine
#[derive(Debug, Clone, Default)]
pub struct PredictionStats {
    /// Total predictions made
    pub total_predictions: u64,
    /// Correct predictions
    pub correct_predictions: u64,
    /// False positives
    pub false_positives: u64,
    /// False negatives
    pub false_negatives: u64,
    /// True positives
    pub true_positives: u64,
    /// True negatives
    pub true_negatives: u64,
    /// Average prediction time (cycles)
    pub avg_prediction_time: u64,
    /// Average confidence
    pub avg_confidence: f32,
    /// Highest confidence prediction
    pub max_confidence: f32,
    /// Lowest confidence prediction
    pub min_confidence: f32,
}

impl PredictionStats {
    /// Calculate accuracy
    pub fn accuracy(&self) -> f32 {
        if self.total_predictions == 0 {
            return 0.0;
        }
        self.correct_predictions as f32 / self.total_predictions as f32
    }

    /// Calculate precision
    pub fn precision(&self) -> f32 {
        let denom = self.true_positives + self.false_positives;
        if denom == 0 {
            return 0.0;
        }
        self.true_positives as f32 / denom as f32
    }

    /// Calculate recall
    pub fn recall(&self) -> f32 {
        let denom = self.true_positives + self.false_negatives;
        if denom == 0 {
            return 0.0;
        }
        self.true_positives as f32 / denom as f32
    }

    /// Calculate F1 score
    pub fn f1_score(&self) -> f32 {
        let precision = self.precision();
        let recall = self.recall();
        let denom = precision + recall;
        if denom == 0.0 {
            return 0.0;
        }
        2.0 * precision * recall / denom
    }
}

// ============================================================================
// HEALING STATS
// ============================================================================

/// Statistics for healing engine
#[derive(Debug, Clone, Default)]
pub struct HealingStats {
    /// Total healing attempts
    pub total_attempts: u64,
    /// Successful healings
    pub successful: u64,
    /// Failed healings
    pub failed: u64,
    /// Soft resets
    pub soft_resets: u64,
    /// Hard resets
    pub hard_resets: u64,
    /// Rollbacks
    pub rollbacks: u64,
    /// Substitutions
    pub substitutions: u64,
    /// Quarantines
    pub quarantines: u64,
    /// Average healing time (cycles)
    pub avg_healing_time: u64,
    /// Maximum healing time
    pub max_healing_time: u64,
    /// Cascade preventions
    pub cascade_preventions: u64,
}

impl HealingStats {
    /// Calculate success rate
    pub fn success_rate(&self) -> f32 {
        if self.total_attempts == 0 {
            return 0.0;
        }
        self.successful as f32 / self.total_attempts as f32
    }
}
