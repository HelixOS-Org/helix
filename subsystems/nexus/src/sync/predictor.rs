//! Wait Time Predictor
//!
//! Predicts lock wait times using linear regression.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;

use super::LockId;

/// Wait time model
#[derive(Debug, Clone)]
pub struct WaitTimeModel {
    /// Lock ID
    pub lock_id: LockId,
    /// Base wait time (ns)
    pub base_wait_ns: f64,
    /// Wait time per waiter (ns)
    pub wait_per_waiter_ns: f64,
    /// Sample count
    pub samples: u32,
}

/// Wait sample
#[derive(Debug, Clone, Copy)]
struct WaitSample {
    /// Timestamp
    timestamp: u64,
    /// Wait time (ns)
    wait_ns: u64,
    /// Number of waiters
    waiters: u32,
}

/// Predicts lock wait times
pub struct WaitTimePredictor {
    /// Models per lock
    models: BTreeMap<LockId, WaitTimeModel>,
    /// Samples
    samples: BTreeMap<LockId, Vec<WaitSample>>,
    /// Max samples
    max_samples: usize,
}

impl WaitTimePredictor {
    /// Create new predictor
    pub fn new() -> Self {
        Self {
            models: BTreeMap::new(),
            samples: BTreeMap::new(),
            max_samples: 1000,
        }
    }

    /// Record wait
    pub fn record(&mut self, lock_id: LockId, wait_ns: u64, waiters: u32) {
        let sample = WaitSample {
            timestamp: NexusTimestamp::now().raw(),
            wait_ns,
            waiters,
        };

        let samples = self.samples.entry(lock_id).or_default();
        samples.push(sample);
        if samples.len() > self.max_samples {
            samples.remove(0);
        }

        self.update_model(lock_id);
    }

    /// Update model
    fn update_model(&mut self, lock_id: LockId) {
        let samples = match self.samples.get(&lock_id) {
            Some(s) if s.len() >= 10 => s,
            _ => return,
        };

        // Linear regression: wait_time = base + waiters * coefficient
        let n = samples.len() as f64;
        let x_mean = samples.iter().map(|s| s.waiters as f64).sum::<f64>() / n;
        let y_mean = samples.iter().map(|s| s.wait_ns as f64).sum::<f64>() / n;

        let mut numerator = 0.0;
        let mut denominator = 0.0;

        for s in samples {
            let x = s.waiters as f64;
            let y = s.wait_ns as f64;
            numerator += (x - x_mean) * (y - y_mean);
            denominator += (x - x_mean) * (x - x_mean);
        }

        let slope = if denominator > 0.0 {
            numerator / denominator
        } else {
            0.0
        };

        let base = y_mean - slope * x_mean;

        self.models.insert(
            lock_id,
            WaitTimeModel {
                lock_id,
                base_wait_ns: base.max(0.0),
                wait_per_waiter_ns: slope.max(0.0),
                samples: samples.len() as u32,
            },
        );
    }

    /// Predict wait time
    pub fn predict(&self, lock_id: LockId, waiters: u32) -> f64 {
        if let Some(model) = self.models.get(&lock_id) {
            model.base_wait_ns + model.wait_per_waiter_ns * waiters as f64
        } else {
            // Default estimate
            1000.0 + 500.0 * waiters as f64
        }
    }

    /// Get model
    pub fn get_model(&self, lock_id: LockId) -> Option<&WaitTimeModel> {
        self.models.get(&lock_id)
    }
}

impl Default for WaitTimePredictor {
    fn default() -> Self {
        Self::new()
    }
}
