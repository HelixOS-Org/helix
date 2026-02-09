//! I/O latency prediction.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;

use super::types::DeviceType;
use crate::math;

// ============================================================================
// DEVICE LATENCY MODEL
// ============================================================================

/// Per-device latency model
#[derive(Debug, Clone)]
struct DeviceLatencyModel {
    /// Device type
    device_type: DeviceType,
    /// Base latency (nanoseconds)
    base_latency: u64,
    /// Per-byte latency
    per_byte_latency: f64,
    /// Queue depth factor
    queue_depth_factor: f64,
    /// Current queue depth
    current_queue_depth: u32,
    /// Recent latencies
    recent_latencies: VecDeque<u64>,
    /// Average latency
    avg_latency: f64,
}

impl DeviceLatencyModel {
    fn new(device_type: DeviceType) -> Self {
        let base = device_type.typical_latency_us() * 1000; // Convert to ns
        let bw = device_type.typical_bandwidth_mbs() as f64 * 1_000_000.0; // bytes/s
        let per_byte = 1_000_000_000.0 / bw; // ns per byte

        Self {
            device_type,
            base_latency: base,
            per_byte_latency: per_byte,
            queue_depth_factor: 1.1,
            current_queue_depth: 0,
            recent_latencies: VecDeque::new(),
            avg_latency: base as f64,
        }
    }

    fn predict(&self, size: u32) -> u64 {
        let base = self.base_latency as f64;
        let transfer = self.per_byte_latency * size as f64;
        let queue_penalty = math::powi(self.queue_depth_factor, self.current_queue_depth as i32);

        ((base + transfer) * queue_penalty) as u64
    }

    fn record(&mut self, latency: u64) {
        self.recent_latencies.push_back(latency);
        if self.recent_latencies.len() > 100 {
            self.recent_latencies.pop_front();
        }

        // Update average
        self.avg_latency =
            self.recent_latencies.iter().sum::<u64>() as f64 / self.recent_latencies.len() as f64;

        // Adjust model if predictions are off
        if self.recent_latencies.len() >= 10 {
            let predicted = self.predict(4096) as f64;
            let error_ratio = self.avg_latency / predicted;

            // Gradually adjust base latency
            self.base_latency = (self.base_latency as f64 * 0.9
                + self.base_latency as f64 * error_ratio * 0.1)
                as u64;
        }
    }
}

// ============================================================================
// LATENCY PREDICTOR
// ============================================================================

/// Predicts I/O latencies
pub struct LatencyPredictor {
    /// Device latency models
    device_models: BTreeMap<u32, DeviceLatencyModel>,
    /// Global latency history
    global_history: VecDeque<(u64, u64)>, // (size, latency)
    /// Max history size
    max_history: usize,
}

impl LatencyPredictor {
    /// Create new latency predictor
    pub fn new() -> Self {
        Self {
            device_models: BTreeMap::new(),
            global_history: VecDeque::new(),
            max_history: 1000,
        }
    }

    /// Register device
    #[inline(always)]
    pub fn register_device(&mut self, device_id: u32, device_type: DeviceType) {
        self.device_models
            .insert(device_id, DeviceLatencyModel::new(device_type));
    }

    /// Predict latency for request
    #[inline]
    pub fn predict(&self, device_id: u32, size: u32) -> u64 {
        self.device_models
            .get(&device_id)
            .map(|m| m.predict(size))
            .unwrap_or(100_000) // Default 100us
    }

    /// Record actual latency
    #[inline]
    pub fn record(&mut self, device_id: u32, size: u32, latency_ns: u64) {
        if let Some(model) = self.device_models.get_mut(&device_id) {
            model.record(latency_ns);
        }

        self.global_history.push_back((size as u64, latency_ns));
        if self.global_history.len() > self.max_history {
            self.global_history.pop_front();
        }
    }

    /// Update queue depth
    #[inline]
    pub fn update_queue_depth(&mut self, device_id: u32, depth: u32) {
        if let Some(model) = self.device_models.get_mut(&device_id) {
            model.current_queue_depth = depth;
        }
    }

    /// Get average latency for device
    #[inline(always)]
    pub fn average_latency(&self, device_id: u32) -> Option<f64> {
        self.device_models.get(&device_id).map(|m| m.avg_latency)
    }

    /// Get prediction accuracy
    pub fn accuracy(&self, device_id: u32) -> Option<f64> {
        let model = self.device_models.get(&device_id)?;
        if model.recent_latencies.len() < 10 {
            return None;
        }

        let mut errors = 0.0;
        let len = model.recent_latencies.len();

        for &actual in &model.recent_latencies {
            // Use recent average as size estimate (simplified)
            let predicted = model.predict(4096);
            let error = (actual as f64 - predicted as f64).abs() / actual as f64;
            errors += error;
        }

        Some(1.0 - (errors / len as f64).min(1.0))
    }
}

impl Default for LatencyPredictor {
    fn default() -> Self {
        Self::new()
    }
}
