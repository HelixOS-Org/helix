//! Power-aware workload prediction.

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;
use crate::math;

// ============================================================================
// WORKLOAD PREDICTOR
// ============================================================================

/// Power-aware workload predictor
pub struct WorkloadPredictor {
    /// CPU utilization history
    cpu_history: VecDeque<(u64, f64)>, // (timestamp, utilization)
    /// Predicted utilization
    predicted: f64,
    /// Prediction confidence
    confidence: f64,
    /// Seasonal patterns (by minute of hour)
    minute_patterns: [f64; 60],
    /// Max history
    max_history: usize,
}

impl WorkloadPredictor {
    /// Create new workload predictor
    pub fn new() -> Self {
        Self {
            cpu_history: VecDeque::new(),
            predicted: 0.5,
            confidence: 0.5,
            minute_patterns: [0.5; 60],
            max_history: 1000,
        }
    }

    /// Record CPU utilization
    pub fn record(&mut self, utilization: f64, minute_of_hour: u8) {
        let now = NexusTimestamp::now().raw();
        self.cpu_history.push_back((now, utilization));

        // Update minute pattern
        let minute = (minute_of_hour % 60) as usize;
        self.minute_patterns[minute] = 0.9 * self.minute_patterns[minute] + 0.1 * utilization;

        // Update prediction
        self.update_prediction();

        // Evict old entries
        if self.cpu_history.len() > self.max_history {
            self.cpu_history.pop_front();
        }
    }

    /// Update prediction
    fn update_prediction(&mut self) {
        if self.cpu_history.len() < 10 {
            return;
        }

        // Simple exponential smoothing
        let recent_avg: f64 = self
            .cpu_history
            .iter()
            .rev()
            .take(10)
            .map(|(_, u)| u)
            .sum::<f64>()
            / 10.0;

        // Trend
        let older_avg: f64 = if self.cpu_history.len() >= 20 {
            self.cpu_history
                .iter()
                .rev()
                .skip(10)
                .take(10)
                .map(|(_, u)| u)
                .sum::<f64>()
                / 10.0
        } else {
            recent_avg
        };

        let trend = recent_avg - older_avg;
        self.predicted = (recent_avg + trend * 0.5).clamp(0.0, 1.0);

        // Update confidence based on variance
        let variance: f64 = self
            .cpu_history
            .iter()
            .rev()
            .take(10)
            .map(|(_, u)| math::powi(u - recent_avg, 2))
            .sum::<f64>()
            / 10.0;

        self.confidence = (1.0 - math::sqrt(variance)).clamp(0.2, 1.0);
    }

    /// Get predicted utilization
    #[inline(always)]
    pub fn predict(&self) -> f64 {
        self.predicted
    }

    /// Get prediction for specific minute
    #[inline(always)]
    pub fn predict_minute(&self, minute: u8) -> f64 {
        self.minute_patterns[(minute % 60) as usize]
    }

    /// Get confidence
    #[inline(always)]
    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    /// Is workload increasing?
    pub fn is_increasing(&self) -> bool {
        if self.cpu_history.len() < 10 {
            return false;
        }

        let recent: f64 = self
            .cpu_history
            .iter()
            .rev()
            .take(5)
            .map(|(_, u)| u)
            .sum::<f64>()
            / 5.0;

        let older: f64 = self
            .cpu_history
            .iter()
            .rev()
            .skip(5)
            .take(5)
            .map(|(_, u)| u)
            .sum::<f64>()
            / 5.0;

        recent > older + 0.05
    }

    /// Is workload idle?
    #[inline(always)]
    pub fn is_idle(&self) -> bool {
        self.predicted < 0.1
    }
}

impl Default for WorkloadPredictor {
    fn default() -> Self {
        Self::new()
    }
}
