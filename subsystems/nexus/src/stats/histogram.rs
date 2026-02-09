//! Histogram for latency/timing measurements

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// HISTOGRAM
// ============================================================================

/// Simple histogram for latency/timing measurements
pub struct Histogram {
    /// Bucket boundaries (exclusive upper bounds)
    boundaries: &'static [u64],
    /// Counts per bucket
    buckets: Vec<AtomicU64>,
    /// Total count
    count: AtomicU64,
    /// Sum of all values
    sum: AtomicU64,
    /// Minimum value seen
    min: AtomicU64,
    /// Maximum value seen
    max: AtomicU64,
}

impl Histogram {
    /// Default latency boundaries in nanoseconds
    pub const DEFAULT_LATENCY_BOUNDARIES: &'static [u64] = &[
        100,        // 100ns
        500,        // 500ns
        1_000,      // 1µs
        5_000,      // 5µs
        10_000,     // 10µs
        50_000,     // 50µs
        100_000,    // 100µs
        500_000,    // 500µs
        1_000_000,  // 1ms
        5_000_000,  // 5ms
        10_000_000, // 10ms
    ];

    /// Create a new histogram with given boundaries
    pub fn new(boundaries: &'static [u64]) -> Self {
        let mut buckets = Vec::with_capacity(boundaries.len() + 1);
        for _ in 0..=boundaries.len() {
            buckets.push(AtomicU64::new(0));
        }
        Self {
            boundaries,
            buckets,
            count: AtomicU64::new(0),
            sum: AtomicU64::new(0),
            min: AtomicU64::new(u64::MAX),
            max: AtomicU64::new(0),
        }
    }

    /// Create a histogram for latency measurements
    #[inline(always)]
    pub fn for_latency() -> Self {
        Self::new(Self::DEFAULT_LATENCY_BOUNDARIES)
    }

    /// Record a value
    pub fn record(&self, value: u64) {
        // Find bucket
        let bucket = self
            .boundaries
            .iter()
            .position(|&b| value < b)
            .unwrap_or(self.boundaries.len());

        self.buckets[bucket].fetch_add(1, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add(value, Ordering::Relaxed);

        // Update min
        let mut current_min = self.min.load(Ordering::Relaxed);
        while value < current_min {
            match self.min.compare_exchange_weak(
                current_min,
                value,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(c) => current_min = c,
            }
        }

        // Update max
        let mut current_max = self.max.load(Ordering::Relaxed);
        while value > current_max {
            match self.max.compare_exchange_weak(
                current_max,
                value,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(c) => current_max = c,
            }
        }
    }

    /// Get total count
    #[inline(always)]
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// Get sum of all values
    #[inline(always)]
    pub fn sum(&self) -> u64 {
        self.sum.load(Ordering::Relaxed)
    }

    /// Get mean value
    #[inline]
    pub fn mean(&self) -> f64 {
        let count = self.count();
        if count == 0 {
            return 0.0;
        }
        self.sum() as f64 / count as f64
    }

    /// Get minimum value
    #[inline]
    pub fn min(&self) -> Option<u64> {
        let min = self.min.load(Ordering::Relaxed);
        if min == u64::MAX {
            None
        } else {
            Some(min)
        }
    }

    /// Get maximum value
    #[inline]
    pub fn max(&self) -> Option<u64> {
        let max = self.max.load(Ordering::Relaxed);
        if max == 0 && self.count() == 0 {
            None
        } else {
            Some(max)
        }
    }

    /// Get bucket counts
    #[inline]
    pub fn bucket_counts(&self) -> Vec<u64> {
        self.buckets
            .iter()
            .map(|b| b.load(Ordering::Relaxed))
            .collect()
    }

    /// Estimate percentile value
    pub fn percentile(&self, p: f64) -> Option<u64> {
        let count = self.count();
        if count == 0 {
            return None;
        }

        let target = (count as f64 * p) as u64;
        let mut cumulative = 0u64;

        for (i, bucket) in self.buckets.iter().enumerate() {
            cumulative += bucket.load(Ordering::Relaxed);
            if cumulative >= target {
                if i == 0 {
                    return Some(self.boundaries.first().copied().unwrap_or(0));
                } else if i >= self.boundaries.len() {
                    return self.max();
                } else {
                    return Some(self.boundaries[i - 1]);
                }
            }
        }

        self.max()
    }

    /// Reset all counters
    #[inline]
    pub fn reset(&self) {
        for bucket in &self.buckets {
            bucket.store(0, Ordering::Relaxed);
        }
        self.count.store(0, Ordering::Relaxed);
        self.sum.store(0, Ordering::Relaxed);
        self.min.store(u64::MAX, Ordering::Relaxed);
        self.max.store(0, Ordering::Relaxed);
    }
}
