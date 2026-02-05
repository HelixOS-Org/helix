//! System load prediction.

use alloc::vec::Vec;

use crate::core::NexusTimestamp;

// ============================================================================
// LOAD PREDICTOR
// ============================================================================

/// System load predictor
pub struct LoadPredictor {
    /// Historical load samples
    history: Vec<LoadSample>,
    /// Seasonal patterns (by hour)
    hourly_patterns: [f64; 24],
    /// Trend component
    trend: f64,
    /// Prediction horizon
    #[allow(dead_code)]
    horizon: u64,
}

/// Load sample
#[derive(Debug, Clone)]
struct LoadSample {
    #[allow(dead_code)]
    timestamp: NexusTimestamp,
    load: f64,
    #[allow(dead_code)]
    hour: u8,
}

impl LoadPredictor {
    /// Create new load predictor
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            hourly_patterns: [0.5; 24],
            trend: 0.0,
            horizon: 3_600_000_000_000,
        }
    }

    /// Record current load
    pub fn record(&mut self, load: f64, hour: u8) {
        let sample = LoadSample {
            timestamp: NexusTimestamp::now(),
            load,
            hour,
        };

        self.history.push(sample);

        self.hourly_patterns[hour as usize] =
            0.9 * self.hourly_patterns[hour as usize] + 0.1 * load;

        if self.history.len() >= 2 {
            let recent: f64 = self
                .history
                .iter()
                .rev()
                .take(10)
                .map(|s| s.load)
                .sum::<f64>()
                / 10.0;
            let older: f64 = if self.history.len() >= 20 {
                self.history
                    .iter()
                    .rev()
                    .skip(10)
                    .take(10)
                    .map(|s| s.load)
                    .sum::<f64>()
                    / 10.0
            } else {
                recent
            };
            self.trend = 0.9 * self.trend + 0.1 * (recent - older);
        }

        if self.history.len() > 10000 {
            self.history.remove(0);
        }
    }

    /// Predict load at future hour
    pub fn predict(&self, future_hour: u8) -> f64 {
        let base = self.hourly_patterns[future_hour as usize];
        let adjusted = base + self.trend;
        adjusted.clamp(0.0, 1.0)
    }

    /// Predict load for next N hours
    pub fn predict_range(&self, current_hour: u8, n_hours: usize) -> Vec<f64> {
        (0..n_hours)
            .map(|offset| {
                let hour = ((current_hour as usize + offset) % 24) as u8;
                self.predict(hour)
            })
            .collect()
    }

    /// Get current trend
    pub fn trend(&self) -> f64 {
        self.trend
    }

    /// Is load increasing?
    pub fn is_increasing(&self) -> bool {
        self.trend > 0.05
    }

    /// Is load decreasing?
    pub fn is_decreasing(&self) -> bool {
        self.trend < -0.05
    }

    /// Detect anomalous load
    pub fn detect_anomaly(&self, current_load: f64, current_hour: u8) -> bool {
        let expected = self.hourly_patterns[current_hour as usize];
        let diff = (current_load - expected).abs();
        diff > 0.3
    }
}

impl Default for LoadPredictor {
    fn default() -> Self {
        Self::new()
    }
}
