//! Queue Depth Predictor
//!
//! This module provides time-series analysis for predicting queue depth and overflow.

use alloc::vec::Vec;
use super::WorkQueueId;

/// Queue depth history sample
#[derive(Debug, Clone, Copy)]
pub struct DepthSample {
    /// Timestamp
    pub timestamp: u64,
    /// Queue depth
    pub depth: u64,
    /// Arrival rate (items/sec)
    pub arrival_rate: f32,
    /// Processing rate (items/sec)
    pub processing_rate: f32,
}

/// Queue depth predictor using time series analysis
pub struct QueueDepthPredictor {
    /// Queue ID
    queue_id: WorkQueueId,
    /// Historical samples
    samples: Vec<DepthSample>,
    /// Maximum samples to keep
    max_samples: usize,
    /// Exponential smoothing alpha
    alpha: f32,
    /// Smoothed arrival rate
    smoothed_arrival: f32,
    /// Smoothed processing rate
    smoothed_processing: f32,
    /// Prediction accuracy (0-1)
    accuracy: f32,
    /// Total predictions made
    prediction_count: u64,
    /// Correct predictions (within threshold)
    correct_predictions: u64,
}

impl QueueDepthPredictor {
    /// Create new queue depth predictor
    pub fn new(queue_id: WorkQueueId) -> Self {
        Self {
            queue_id,
            samples: Vec::with_capacity(1024),
            max_samples: 1024,
            alpha: 0.3,
            smoothed_arrival: 0.0,
            smoothed_processing: 0.0,
            accuracy: 0.5,
            prediction_count: 0,
            correct_predictions: 0,
        }
    }

    /// Record queue depth sample
    pub fn record_sample(
        &mut self,
        timestamp: u64,
        depth: u64,
        arrival_rate: f32,
        processing_rate: f32,
    ) {
        let sample = DepthSample {
            timestamp,
            depth,
            arrival_rate,
            processing_rate,
        };

        // Update exponential smoothing
        self.smoothed_arrival =
            self.alpha * arrival_rate + (1.0 - self.alpha) * self.smoothed_arrival;
        self.smoothed_processing =
            self.alpha * processing_rate + (1.0 - self.alpha) * self.smoothed_processing;

        // Add sample
        if self.samples.len() >= self.max_samples {
            self.samples.remove(0);
        }
        self.samples.push(sample);
    }

    /// Predict queue depth at future time
    pub fn predict_depth(&self, current_depth: u64, future_ns: u64) -> u64 {
        if self.samples.is_empty() {
            return current_depth;
        }

        // Calculate net rate (arrivals - processing)
        let net_rate = self.smoothed_arrival - self.smoothed_processing;

        // Predict using queueing theory: depth = current + net_rate * time
        let future_seconds = future_ns as f32 / 1_000_000_000.0;
        let predicted_change = net_rate * future_seconds;

        if predicted_change > 0.0 {
            current_depth + predicted_change as u64
        } else {
            current_depth.saturating_sub((-predicted_change) as u64)
        }
    }

    /// Predict if queue will overflow
    pub fn predict_overflow(
        &self,
        current_depth: u64,
        max_depth: u64,
        horizon_ns: u64,
    ) -> Option<u64> {
        if current_depth >= max_depth {
            return Some(0);
        }

        let net_rate = self.smoothed_arrival - self.smoothed_processing;
        if net_rate <= 0.0 {
            return None; // Queue is draining
        }

        let remaining_capacity = max_depth - current_depth;
        let time_to_overflow_sec = remaining_capacity as f32 / net_rate;
        let time_to_overflow_ns = (time_to_overflow_sec * 1_000_000_000.0) as u64;

        if time_to_overflow_ns <= horizon_ns {
            Some(time_to_overflow_ns)
        } else {
            None
        }
    }

    /// Predict time to drain queue
    pub fn predict_drain_time(&self, current_depth: u64) -> Option<u64> {
        if current_depth == 0 {
            return Some(0);
        }

        let net_rate = self.smoothed_processing - self.smoothed_arrival;
        if net_rate <= 0.0 {
            return None; // Queue is not draining
        }

        let drain_time_sec = current_depth as f32 / net_rate;
        Some((drain_time_sec * 1_000_000_000.0) as u64)
    }

    /// Update prediction accuracy
    pub fn update_accuracy(&mut self, predicted: u64, actual: u64, threshold: u64) {
        self.prediction_count += 1;
        let error = if predicted > actual {
            predicted - actual
        } else {
            actual - predicted
        };

        if error <= threshold {
            self.correct_predictions += 1;
        }

        self.accuracy = self.correct_predictions as f32 / self.prediction_count as f32;
    }

    /// Get current prediction accuracy
    pub fn accuracy(&self) -> f32 {
        self.accuracy
    }

    /// Get queue ID
    pub fn queue_id(&self) -> WorkQueueId {
        self.queue_id
    }

    /// Get sample count
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }
}
