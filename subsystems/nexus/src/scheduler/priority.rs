//! Priority learning.

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::TaskFeatures;
use crate::math;

// ============================================================================
// PRIORITY LEARNER
// ============================================================================

/// Learning-based priority adjustment
#[repr(align(64))]
pub struct PriorityLearner {
    /// Priority adjustments by task type
    type_adjustments: BTreeMap<u64, PriorityAdjustment>,
    /// Default adjustment
    default_adjustment: PriorityAdjustment,
    /// Learning rate
    learning_rate: f64,
    /// Total adjustments made
    total_adjustments: AtomicU64,
}

/// Priority adjustment record
#[derive(Debug, Clone)]
struct PriorityAdjustment {
    base_modifier: f64,
    dynamic_modifier: f64,
    #[allow(dead_code)]
    confidence: f64,
    samples: u64,
}

impl Default for PriorityAdjustment {
    fn default() -> Self {
        Self {
            base_modifier: 0.0,
            dynamic_modifier: 0.0,
            confidence: 0.5,
            samples: 0,
        }
    }
}

impl PriorityLearner {
    /// Create new priority learner
    pub fn new() -> Self {
        Self {
            type_adjustments: BTreeMap::new(),
            default_adjustment: PriorityAdjustment::default(),
            learning_rate: 0.1,
            total_adjustments: AtomicU64::new(0),
        }
    }

    /// Get priority adjustment for a task
    pub fn get_adjustment(&self, task_type_hash: u64, features: &TaskFeatures) -> i32 {
        let adjustment = self
            .type_adjustments
            .get(&task_type_hash)
            .unwrap_or(&self.default_adjustment);

        let mut modifier = adjustment.base_modifier + adjustment.dynamic_modifier;

        if features.voluntary_switches > 100.0 {
            modifier += 2.0;
        }

        if features.avg_cpu_usage > 0.9 && features.cache_miss_rate > 0.5 {
            modifier -= 1.0;
        }

        if features.ipc_frequency > 50.0 {
            modifier += 1.0;
        }

        math::round(modifier) as i32
    }

    /// Record outcome for learning
    pub fn record_outcome(
        &mut self,
        task_type_hash: u64,
        predicted_priority: i32,
        actual_performance: f64,
    ) {
        let adjustment = self.type_adjustments.entry(task_type_hash).or_default();

        let error = actual_performance - 0.7;

        adjustment.base_modifier += self.learning_rate * error * 0.5;
        adjustment.dynamic_modifier =
            0.8 * adjustment.dynamic_modifier + 0.2 * error * predicted_priority as f64;

        adjustment.base_modifier = adjustment.base_modifier.clamp(-10.0, 10.0);
        adjustment.dynamic_modifier = adjustment.dynamic_modifier.clamp(-5.0, 5.0);

        adjustment.samples += 1;
        adjustment.confidence =
            (adjustment.samples as f64 / (adjustment.samples + 10) as f64).min(0.95);

        self.total_adjustments.fetch_add(1, Ordering::Relaxed);
    }

    /// Set learning rate
    #[inline(always)]
    pub fn set_learning_rate(&mut self, rate: f64) {
        self.learning_rate = rate.clamp(0.01, 0.5);
    }

    /// Get total adjustments
    #[inline(always)]
    pub fn total_adjustments(&self) -> u64 {
        self.total_adjustments.load(Ordering::Relaxed)
    }

    /// Reset learner
    #[inline(always)]
    pub fn reset(&mut self) {
        self.type_adjustments.clear();
        self.total_adjustments.store(0, Ordering::Relaxed);
    }
}

impl Default for PriorityLearner {
    fn default() -> Self {
        Self::new()
    }
}
