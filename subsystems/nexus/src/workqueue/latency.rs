//! Work Latency Analyzer
//!
//! This module provides histogram-based latency analysis with SLA tracking.

use alloc::vec::Vec;
use super::WorkQueueId;

/// Latency distribution bucket
#[derive(Debug, Clone, Copy, Default)]
pub struct LatencyBucket {
    /// Count in this bucket
    pub count: u64,
    /// Sum of latencies in this bucket
    pub sum_ns: u64,
}

/// Work latency statistics
#[derive(Debug, Clone)]
pub struct LatencyStats {
    /// Minimum latency (nanoseconds)
    pub min_ns: u64,
    /// Maximum latency (nanoseconds)
    pub max_ns: u64,
    /// Mean latency (nanoseconds)
    pub mean_ns: f64,
    /// P50 latency (nanoseconds)
    pub p50_ns: u64,
    /// P90 latency (nanoseconds)
    pub p90_ns: u64,
    /// P99 latency (nanoseconds)
    pub p99_ns: u64,
    /// P999 latency (nanoseconds)
    pub p999_ns: u64,
    /// Standard deviation
    pub std_dev_ns: f64,
    /// Sample count
    pub sample_count: u64,
}

/// Latency trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatencyTrend {
    /// Latency is increasing
    Increasing,
    /// Latency is stable
    Stable,
    /// Latency is decreasing
    Decreasing,
}

/// Work latency analyzer with histogram
pub struct WorkLatencyAnalyzer {
    /// Queue ID being analyzed
    queue_id: WorkQueueId,
    /// Latency buckets (exponential)
    buckets: [LatencyBucket; 64],
    /// Bucket boundaries
    bucket_bounds: [u64; 64],
    /// Total samples
    total_samples: u64,
    /// Sum of all latencies
    sum_ns: u64,
    /// Sum of squared latencies for variance
    sum_sq_ns: u128,
    /// Minimum observed latency
    min_ns: u64,
    /// Maximum observed latency
    max_ns: u64,
    /// Recent latencies for trend analysis
    recent_latencies: Vec<u64>,
    /// Maximum recent samples
    max_recent: usize,
    /// SLA threshold (nanoseconds)
    sla_threshold_ns: u64,
    /// SLA violations count
    sla_violations: u64,
}

impl WorkLatencyAnalyzer {
    /// Create new latency analyzer
    pub fn new(queue_id: WorkQueueId) -> Self {
        // Initialize exponential bucket boundaries
        let mut bounds = [0u64; 64];
        let mut value = 1000u64; // Start at 1µs
        for bound in &mut bounds {
            *bound = value;
            value = value.saturating_mul(12) / 10; // ~1.2x per bucket
        }

        Self {
            queue_id,
            buckets: [LatencyBucket::default(); 64],
            bucket_bounds: bounds,
            total_samples: 0,
            sum_ns: 0,
            sum_sq_ns: 0,
            min_ns: u64::MAX,
            max_ns: 0,
            recent_latencies: Vec::with_capacity(1000),
            max_recent: 1000,
            sla_threshold_ns: 10_000_000, // 10ms default
            sla_violations: 0,
        }
    }

    /// Find bucket index for latency value
    fn find_bucket(&self, latency_ns: u64) -> usize {
        for (i, &bound) in self.bucket_bounds.iter().enumerate() {
            if latency_ns < bound {
                return i;
            }
        }
        self.bucket_bounds.len() - 1
    }

    /// Record a latency sample
    pub fn record_latency(&mut self, latency_ns: u64) {
        // Update bucket
        let bucket_idx = self.find_bucket(latency_ns);
        self.buckets[bucket_idx].count += 1;
        self.buckets[bucket_idx].sum_ns += latency_ns;

        // Update statistics
        self.total_samples += 1;
        self.sum_ns += latency_ns;
        self.sum_sq_ns += (latency_ns as u128) * (latency_ns as u128);
        self.min_ns = self.min_ns.min(latency_ns);
        self.max_ns = self.max_ns.max(latency_ns);

        // Update recent latencies
        if self.recent_latencies.len() >= self.max_recent {
            self.recent_latencies.remove(0);
        }
        self.recent_latencies.push(latency_ns);

        // Check SLA
        if latency_ns > self.sla_threshold_ns {
            self.sla_violations += 1;
        }
    }

    /// Set SLA threshold
    pub fn set_sla_threshold(&mut self, threshold_ns: u64) {
        self.sla_threshold_ns = threshold_ns;
    }

    /// Get percentile value
    pub fn percentile(&self, p: f32) -> u64 {
        if self.total_samples == 0 {
            return 0;
        }

        let target = ((p / 100.0) * self.total_samples as f32) as u64;
        let mut cumulative = 0u64;

        for (i, bucket) in self.buckets.iter().enumerate() {
            cumulative += bucket.count;
            if cumulative >= target {
                // Return bucket midpoint
                let lower = if i > 0 { self.bucket_bounds[i - 1] } else { 0 };
                return (lower + self.bucket_bounds[i]) / 2;
            }
        }

        self.max_ns
    }

    /// Calculate latency statistics
    pub fn calculate_stats(&self) -> LatencyStats {
        let mean_ns = if self.total_samples > 0 {
            self.sum_ns as f64 / self.total_samples as f64
        } else {
            0.0
        };

        let variance = if self.total_samples > 1 {
            let mean_sq = mean_ns * mean_ns;
            let sq_mean = self.sum_sq_ns as f64 / self.total_samples as f64;
            (sq_mean - mean_sq).max(0.0)
        } else {
            0.0
        };

        let std_dev_ns = libm::sqrt(variance);

        LatencyStats {
            min_ns: if self.min_ns == u64::MAX {
                0
            } else {
                self.min_ns
            },
            max_ns: self.max_ns,
            mean_ns,
            p50_ns: self.percentile(50.0),
            p90_ns: self.percentile(90.0),
            p99_ns: self.percentile(99.0),
            p999_ns: self.percentile(99.9),
            std_dev_ns,
            sample_count: self.total_samples,
        }
    }

    /// Calculate SLA compliance rate
    pub fn sla_compliance_rate(&self) -> f32 {
        if self.total_samples == 0 {
            return 1.0;
        }
        1.0 - (self.sla_violations as f32 / self.total_samples as f32)
    }

    /// Detect latency trend
    pub fn detect_trend(&self) -> LatencyTrend {
        if self.recent_latencies.len() < 10 {
            return LatencyTrend::Stable;
        }

        let len = self.recent_latencies.len();
        let half = len / 2;

        let first_half_avg: f64 = self.recent_latencies[..half]
            .iter()
            .map(|&v| v as f64)
            .sum::<f64>()
            / half as f64;

        let second_half_avg: f64 = self.recent_latencies[half..]
            .iter()
            .map(|&v| v as f64)
            .sum::<f64>()
            / (len - half) as f64;

        let change_ratio = second_half_avg / first_half_avg.max(1.0);

        if change_ratio > 1.2 {
            LatencyTrend::Increasing
        } else if change_ratio < 0.8 {
            LatencyTrend::Decreasing
        } else {
            LatencyTrend::Stable
        }
    }

    /// Get queue ID
    pub fn queue_id(&self) -> WorkQueueId {
        self.queue_id
    }

    /// Get total sample count
    pub fn sample_count(&self) -> u64 {
        self.total_samples
    }
}
