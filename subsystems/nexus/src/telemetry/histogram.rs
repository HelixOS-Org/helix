//! Histogram for distribution tracking.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// HISTOGRAM
// ============================================================================

/// A histogram for distribution tracking
#[derive(Debug, Clone)]
pub struct TelemetryHistogram {
    /// Bucket boundaries
    boundaries: Vec<f64>,
    /// Counts per bucket
    counts: Vec<u64>,
    /// Sum of all values
    sum: f64,
    /// Count of all values
    count: u64,
    /// Min value
    min: f64,
    /// Max value
    max: f64,
}

impl TelemetryHistogram {
    /// Create new histogram with default boundaries
    pub fn new() -> Self {
        Self::with_boundaries(vec![
            0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ])
    }

    /// Create with custom boundaries
    pub fn with_boundaries(boundaries: Vec<f64>) -> Self {
        let n_buckets = boundaries.len() + 1; // +1 for overflow bucket
        Self {
            boundaries,
            counts: vec![0; n_buckets],
            sum: 0.0,
            count: 0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }

    /// Create histogram for latency (microseconds)
    pub fn for_latency() -> Self {
        Self::with_boundaries(vec![
            1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0,
        ])
    }

    /// Create histogram for sizes (bytes)
    pub fn for_size() -> Self {
        Self::with_boundaries(vec![
            64.0, 256.0, 1024.0, 4096.0, 16384.0, 65536.0, 262144.0, 1048576.0,
        ])
    }

    /// Observe a value
    pub fn observe(&mut self, value: f64) {
        // Find bucket
        let bucket = self
            .boundaries
            .iter()
            .position(|&b| value <= b)
            .unwrap_or(self.boundaries.len());

        self.counts[bucket] += 1;
        self.sum += value;
        self.count += 1;
        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }

    /// Get bucket counts
    pub fn buckets(&self) -> Vec<(f64, u64)> {
        let mut result = Vec::new();
        for (i, &boundary) in self.boundaries.iter().enumerate() {
            result.push((boundary, self.counts[i]));
        }
        // Overflow bucket
        result.push((f64::INFINITY, *self.counts.last().unwrap_or(&0)));
        result
    }

    /// Get count
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Get sum
    pub fn sum(&self) -> f64 {
        self.sum
    }

    /// Get mean
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }

    /// Get min
    pub fn min(&self) -> f64 {
        self.min
    }

    /// Get max
    pub fn max(&self) -> f64 {
        self.max
    }

    /// Estimate quantile (approximate)
    pub fn quantile(&self, q: f64) -> f64 {
        if self.count == 0 {
            return 0.0;
        }

        let target = (q * self.count as f64) as u64;
        let mut cumsum = 0u64;

        for (i, &count) in self.counts.iter().enumerate() {
            cumsum += count;
            if cumsum >= target {
                if i == 0 {
                    return self.boundaries.first().copied().unwrap_or(0.0) / 2.0;
                } else if i >= self.boundaries.len() {
                    return *self.boundaries.last().unwrap_or(&0.0) * 2.0;
                } else {
                    // Interpolate
                    let lower = self.boundaries.get(i - 1).copied().unwrap_or(0.0);
                    let upper = self.boundaries[i];
                    return (lower + upper) / 2.0;
                }
            }
        }

        self.max
    }

    /// Get p50 (median)
    pub fn p50(&self) -> f64 {
        self.quantile(0.5)
    }

    /// Get p90
    pub fn p90(&self) -> f64 {
        self.quantile(0.9)
    }

    /// Get p99
    pub fn p99(&self) -> f64 {
        self.quantile(0.99)
    }

    /// Reset histogram
    pub fn reset(&mut self) {
        for count in &mut self.counts {
            *count = 0;
        }
        self.sum = 0.0;
        self.count = 0;
        self.min = f64::INFINITY;
        self.max = f64::NEG_INFINITY;
    }
}

impl Default for TelemetryHistogram {
    fn default() -> Self {
        Self::new()
    }
}
