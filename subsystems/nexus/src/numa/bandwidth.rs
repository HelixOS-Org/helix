//! Inter-node bandwidth monitoring.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::types::NodeId;
use crate::core::NexusTimestamp;

// ============================================================================
// BANDWIDTH SAMPLE
// ============================================================================

/// Bandwidth sample
#[derive(Debug, Clone, Copy)]
struct BandwidthSample {
    /// Timestamp
    timestamp: u64,
    /// Bandwidth (bytes/sec)
    bandwidth: f64,
    /// Utilization (0.0 - 1.0)
    utilization: f64,
}

// ============================================================================
// BANDWIDTH MONITOR
// ============================================================================

/// Monitors inter-node bandwidth
pub struct BandwidthMonitor {
    /// Bandwidth samples
    samples: BTreeMap<(NodeId, NodeId), Vec<BandwidthSample>>,
    /// Max samples per pair
    max_samples: usize,
    /// Current bandwidth estimates
    current: BTreeMap<(NodeId, NodeId), f64>,
    /// Peak bandwidth observed
    peak: BTreeMap<(NodeId, NodeId), f64>,
}

impl BandwidthMonitor {
    /// Create new monitor
    pub fn new() -> Self {
        Self {
            samples: BTreeMap::new(),
            max_samples: 1000,
            current: BTreeMap::new(),
            peak: BTreeMap::new(),
        }
    }

    /// Record bandwidth sample
    #[inline]
    pub fn record(&mut self, from: NodeId, to: NodeId, bytes: u64, duration_ns: u64) {
        let bandwidth = if duration_ns > 0 {
            bytes as f64 * 1_000_000_000.0 / duration_ns as f64
        } else {
            0.0
        };

        let sample = BandwidthSample {
            timestamp: NexusTimestamp::now().raw(),
            bandwidth,
            utilization: 0.0, // Would need max bandwidth to calculate
        };

        let key = (from, to);
        let samples = self.samples.entry(key).or_default();
        samples.push(sample);
        if samples.len() > self.max_samples {
            samples.pop_front();
        }

        // Update current
        let prev = self.current.get(&key).copied().unwrap_or(bandwidth);
        let alpha = 0.2;
        let current = alpha * bandwidth + (1.0 - alpha) * prev;
        self.current.insert(key, current);

        // Update peak
        let peak = self.peak.entry(key).or_insert(0.0);
        if bandwidth > *peak {
            *peak = bandwidth;
        }
    }

    /// Get current bandwidth estimate
    #[inline(always)]
    pub fn get_bandwidth(&self, from: NodeId, to: NodeId) -> f64 {
        self.current.get(&(from, to)).copied().unwrap_or(0.0)
    }

    /// Get peak bandwidth
    #[inline(always)]
    pub fn get_peak(&self, from: NodeId, to: NodeId) -> f64 {
        self.peak.get(&(from, to)).copied().unwrap_or(0.0)
    }

    /// Get average bandwidth
    #[inline]
    pub fn get_average(&self, from: NodeId, to: NodeId) -> f64 {
        let samples = match self.samples.get(&(from, to)) {
            Some(s) if !s.is_empty() => s,
            _ => return 0.0,
        };

        samples.iter().map(|s| s.bandwidth).sum::<f64>() / samples.len() as f64
    }

    /// Get bandwidth trend
    pub fn get_trend(&self, from: NodeId, to: NodeId) -> f64 {
        let samples = match self.samples.get(&(from, to)) {
            Some(s) if s.len() >= 10 => s,
            _ => return 0.0,
        };

        let len = samples.len();
        let first_half: f64 =
            samples[..len / 2].iter().map(|s| s.bandwidth).sum::<f64>() / (len / 2) as f64;
        let second_half: f64 =
            samples[len / 2..].iter().map(|s| s.bandwidth).sum::<f64>() / (len - len / 2) as f64;

        second_half - first_half
    }
}

impl Default for BandwidthMonitor {
    fn default() -> Self {
        Self::new()
    }
}
