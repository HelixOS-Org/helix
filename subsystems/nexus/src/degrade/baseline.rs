//! Metric baseline tracking.

use crate::core::NexusTimestamp;
use crate::math;

/// Baseline for a metric
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MetricBaseline {
    /// Baseline mean
    pub(crate) mean: f64,
    /// Baseline std dev
    pub(crate) std_dev: f64,
    /// Number of samples
    pub(crate) samples: u64,
    /// Last updated
    updated: NexusTimestamp,
    /// Current EMA
    pub(crate) ema: f64,
    /// Current value
    pub(crate) current: f64,
}

impl MetricBaseline {
    /// Create a new baseline
    pub fn new(initial: f64) -> Self {
        Self {
            mean: initial,
            std_dev: 0.0,
            samples: 1,
            updated: NexusTimestamp::now(),
            ema: initial,
            current: initial,
        }
    }

    /// Update with new value
    #[inline]
    pub fn update(&mut self, value: f64, ema_alpha: f64) {
        // Update EMA
        self.ema = ema_alpha * value + (1.0 - ema_alpha) * self.ema;
        self.current = value;

        // Update running stats
        self.samples += 1;
        let delta = value - self.mean;
        self.mean += delta / self.samples as f64;
        let delta2 = value - self.mean;
        let variance = if self.samples > 1 {
            (self.std_dev * self.std_dev * (self.samples - 1) as f64 + delta * delta2)
                / (self.samples as f64)
        } else {
            0.0
        };
        self.std_dev = math::sqrt(variance);

        self.updated = NexusTimestamp::now();
    }

    /// Get degradation from baseline
    #[inline]
    pub fn degradation(&self) -> f64 {
        if self.mean == 0.0 {
            return 0.0;
        }
        ((self.current - self.mean) / self.mean) * 100.0
    }

    /// Get z-score of current value
    #[inline]
    pub fn z_score(&self) -> f64 {
        if self.std_dev == 0.0 {
            return 0.0;
        }
        (self.current - self.mean) / self.std_dev
    }
}
