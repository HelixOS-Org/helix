//! Benchmark result and statistics

#![allow(dead_code)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::math;

// ============================================================================
// BENCHMARK RESULT
// ============================================================================

/// Result of a benchmark run
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Benchmark name
    pub name: String,
    /// Number of iterations
    pub iterations: u64,
    /// Total time (cycles)
    pub total_time: u64,
    /// Mean time per iteration (cycles)
    pub mean: f64,
    /// Median time per iteration (cycles)
    pub median: f64,
    /// Standard deviation
    pub std_dev: f64,
    /// Minimum time
    pub min: u64,
    /// Maximum time
    pub max: u64,
    /// 95th percentile
    pub p95: f64,
    /// 99th percentile
    pub p99: f64,
    /// Throughput (ops per second, estimated at 1GHz)
    pub throughput: f64,
}

impl BenchmarkResult {
    /// Create from samples
    pub fn from_samples(name: impl Into<String>, samples: &[u64]) -> Self {
        if samples.is_empty() {
            return Self::empty(name);
        }

        let n = samples.len();
        let iterations = n as u64;
        let total: u64 = samples.iter().sum();
        let mean = total as f64 / n as f64;

        // Calculate std dev
        let variance: f64 = samples
            .iter()
            .map(|&x| {
                let diff = x as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / (n - 1).max(1) as f64;
        let std_dev = math::sqrt(variance);

        // Sort for percentiles
        let mut sorted = samples.to_vec();
        sorted.sort();

        let median = if n % 2 == 0 {
            (sorted[n / 2 - 1] + sorted[n / 2]) as f64 / 2.0
        } else {
            sorted[n / 2] as f64
        };

        let min = *sorted.first().unwrap();
        let max = *sorted.last().unwrap();

        let p95_idx = ((n as f64 * 0.95) as usize).min(n - 1);
        let p99_idx = ((n as f64 * 0.99) as usize).min(n - 1);
        let p95 = sorted[p95_idx] as f64;
        let p99 = sorted[p99_idx] as f64;

        // Throughput assuming 1GHz CPU
        let throughput = if mean > 0.0 {
            1_000_000_000.0 / mean
        } else {
            0.0
        };

        Self {
            name: name.into(),
            iterations,
            total_time: total,
            mean,
            median,
            std_dev,
            min,
            max,
            p95,
            p99,
            throughput,
        }
    }

    /// Create empty result
    pub fn empty(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            iterations: 0,
            total_time: 0,
            mean: 0.0,
            median: 0.0,
            std_dev: 0.0,
            min: 0,
            max: 0,
            p95: 0.0,
            p99: 0.0,
            throughput: 0.0,
        }
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        format!(
            "{}: mean={:.0} cycles, median={:.0}, std_dev={:.0}, min={}, max={}, p95={:.0}, p99={:.0}, throughput={:.0} ops/sec",
            self.name, self.mean, self.median, self.std_dev, self.min, self.max, self.p95, self.p99, self.throughput
        )
    }

    /// Compare with another result (returns speedup factor)
    pub fn compare(&self, other: &BenchmarkResult) -> f64 {
        if self.mean == 0.0 {
            return 0.0;
        }
        other.mean / self.mean
    }

    /// Is this faster than another result?
    pub fn is_faster_than(&self, other: &BenchmarkResult) -> bool {
        self.mean < other.mean
    }

    /// Check if regression occurred (within threshold)
    pub fn is_regression(&self, baseline: &BenchmarkResult, threshold: f64) -> bool {
        if baseline.mean == 0.0 {
            return false;
        }
        let ratio = self.mean / baseline.mean;
        ratio > (1.0 + threshold)
    }
}

impl Default for BenchmarkResult {
    fn default() -> Self {
        Self::empty("unnamed")
    }
}
