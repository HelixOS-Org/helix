//! Memory access latency prediction.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::types::NodeId;
use crate::core::NexusTimestamp;

// ============================================================================
// LATENCY SAMPLE
// ============================================================================

/// Latency sample
#[derive(Debug, Clone, Copy)]
struct LatencySample {
    /// Timestamp
    timestamp: u64,
    /// Latency (ns)
    latency: f64,
    /// Memory load
    load: f64,
}

// ============================================================================
// LATENCY MODEL
// ============================================================================

/// Latency model
#[derive(Debug, Clone)]
pub struct LatencyModel {
    /// Node ID
    pub node: NodeId,
    /// Base latency (ns)
    pub base_latency: f64,
    /// Load coefficient
    pub load_coef: f64,
    /// Contention factor
    pub contention_factor: f64,
}

// ============================================================================
// LATENCY PREDICTOR
// ============================================================================

/// Predicts memory access latency
pub struct LatencyPredictor {
    /// Latency models
    models: BTreeMap<NodeId, LatencyModel>,
    /// Cross-node latency
    cross_latency: BTreeMap<(NodeId, NodeId), f64>,
    /// Samples
    samples: BTreeMap<NodeId, Vec<LatencySample>>,
}

impl LatencyPredictor {
    /// Create new predictor
    pub fn new() -> Self {
        Self {
            models: BTreeMap::new(),
            cross_latency: BTreeMap::new(),
            samples: BTreeMap::new(),
        }
    }

    /// Record latency sample
    pub fn record(&mut self, node: NodeId, latency_ns: u64, load: f64) {
        let sample = LatencySample {
            timestamp: NexusTimestamp::now().raw(),
            latency: latency_ns as f64,
            load,
        };

        let samples = self.samples.entry(node).or_default();
        samples.push(sample);
        if samples.len() > 1000 {
            samples.pop_front();
        }

        // Update model
        self.update_model(node);
    }

    /// Record cross-node latency
    #[inline]
    pub fn record_cross(&mut self, from: NodeId, to: NodeId, latency_ns: u64) {
        let key = (from, to);
        let prev = self
            .cross_latency
            .get(&key)
            .copied()
            .unwrap_or(latency_ns as f64);
        let alpha = 0.1;
        let current = alpha * latency_ns as f64 + (1.0 - alpha) * prev;
        self.cross_latency.insert(key, current);
    }

    /// Update latency model
    fn update_model(&mut self, node: NodeId) {
        let samples = match self.samples.get(&node) {
            Some(s) if s.len() >= 10 => s,
            _ => return,
        };

        // Simple linear regression
        let n = samples.len() as f64;
        let x_mean = samples.iter().map(|s| s.load).sum::<f64>() / n;
        let y_mean = samples.iter().map(|s| s.latency).sum::<f64>() / n;

        let mut numerator = 0.0;
        let mut denominator = 0.0;

        for s in samples {
            numerator += (s.load - x_mean) * (s.latency - y_mean);
            denominator += (s.load - x_mean) * (s.load - x_mean);
        }

        let load_coef = if denominator > 0.0 {
            numerator / denominator
        } else {
            0.0
        };

        let base = y_mean - load_coef * x_mean;

        let model = LatencyModel {
            node,
            base_latency: base.max(0.0),
            load_coef: load_coef.max(0.0),
            contention_factor: 1.0,
        };

        self.models.insert(node, model);
    }

    /// Predict latency
    #[inline]
    pub fn predict(&self, node: NodeId, load: f64) -> f64 {
        if let Some(model) = self.models.get(&node) {
            model.base_latency + model.load_coef * load * model.contention_factor
        } else {
            100.0 // Default 100ns
        }
    }

    /// Predict cross-node latency
    #[inline]
    pub fn predict_cross(&self, from: NodeId, to: NodeId) -> f64 {
        self.cross_latency
            .get(&(from, to))
            .copied()
            .unwrap_or(300.0) // Default 300ns for cross-node
    }

    /// Get model
    #[inline(always)]
    pub fn get_model(&self, node: NodeId) -> Option<&LatencyModel> {
        self.models.get(&node)
    }
}

impl Default for LatencyPredictor {
    fn default() -> Self {
        Self::new()
    }
}
