//! Object Lifetime Predictor
//!
//! This module provides object lifetime analysis for placement optimization.

use super::SlabCacheId;

/// Lifetime bucket (logarithmic)
#[derive(Debug, Clone, Copy, Default)]
pub struct LifetimeBucket {
    /// Count in bucket
    pub count: u64,
    /// Sum of lifetimes
    pub sum_ns: u64,
}

/// Object lifetime statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct LifetimeStats {
    /// Minimum lifetime (nanoseconds)
    pub min_ns: u64,
    /// Maximum lifetime (nanoseconds)
    pub max_ns: u64,
    /// Mean lifetime (nanoseconds)
    pub mean_ns: f64,
    /// Median lifetime (nanoseconds)
    pub median_ns: u64,
    /// P90 lifetime (nanoseconds)
    pub p90_ns: u64,
    /// P99 lifetime (nanoseconds)
    pub p99_ns: u64,
    /// Standard deviation
    pub std_dev_ns: f64,
    /// Total samples
    pub sample_count: u64,
}

/// Object placement strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementStrategy {
    /// Default placement
    Default,
    /// CPU-local placement (for short-lived)
    CpuLocal,
    /// NUMA-aware placement (for long-lived)
    NumaAware,
}

/// Object lifetime predictor
pub struct ObjectLifetimePredictor {
    /// Cache ID
    cache_id: SlabCacheId,
    /// Lifetime histogram buckets
    buckets: [LifetimeBucket; 32],
    /// Bucket boundaries (nanoseconds)
    bucket_bounds: [u64; 32],
    /// Total samples
    total_samples: u64,
    /// Sum of lifetimes
    sum_ns: u64,
    /// Sum of squared lifetimes
    sum_sq_ns: u128,
    /// Minimum lifetime
    min_ns: u64,
    /// Maximum lifetime
    max_ns: u64,
    /// Short-lived threshold
    short_lived_threshold_ns: u64,
    /// Long-lived threshold
    long_lived_threshold_ns: u64,
    /// Short-lived count
    short_lived_count: u64,
    /// Long-lived count
    long_lived_count: u64,
}

impl ObjectLifetimePredictor {
    /// Create new lifetime predictor
    pub fn new(cache_id: SlabCacheId) -> Self {
        // Initialize logarithmic bucket boundaries
        let mut bounds = [0u64; 32];
        let mut value = 1000u64; // Start at 1µs
        for bound in &mut bounds {
            *bound = value;
            value = value.saturating_mul(2); // Double each bucket
        }

        Self {
            cache_id,
            buckets: [LifetimeBucket::default(); 32],
            bucket_bounds: bounds,
            total_samples: 0,
            sum_ns: 0,
            sum_sq_ns: 0,
            min_ns: u64::MAX,
            max_ns: 0,
            short_lived_threshold_ns: 1_000_000, // 1ms
            long_lived_threshold_ns: 1_000_000_000, // 1s
            short_lived_count: 0,
            long_lived_count: 0,
        }
    }

    /// Find bucket index for lifetime
    fn find_bucket(&self, lifetime_ns: u64) -> usize {
        for (i, &bound) in self.bucket_bounds.iter().enumerate() {
            if lifetime_ns < bound {
                return i;
            }
        }
        self.bucket_bounds.len() - 1
    }

    /// Record object lifetime
    pub fn record_lifetime(&mut self, lifetime_ns: u64) {
        // Update bucket
        let bucket_idx = self.find_bucket(lifetime_ns);
        self.buckets[bucket_idx].count += 1;
        self.buckets[bucket_idx].sum_ns += lifetime_ns;

        // Update statistics
        self.total_samples += 1;
        self.sum_ns += lifetime_ns;
        self.sum_sq_ns += (lifetime_ns as u128) * (lifetime_ns as u128);
        self.min_ns = self.min_ns.min(lifetime_ns);
        self.max_ns = self.max_ns.max(lifetime_ns);

        // Categorize
        if lifetime_ns < self.short_lived_threshold_ns {
            self.short_lived_count += 1;
        }
        if lifetime_ns > self.long_lived_threshold_ns {
            self.long_lived_count += 1;
        }
    }

    /// Get percentile lifetime
    pub fn percentile(&self, p: f32) -> u64 {
        if self.total_samples == 0 {
            return 0;
        }

        let target = ((p / 100.0) * self.total_samples as f32) as u64;
        let mut cumulative = 0u64;

        for (i, bucket) in self.buckets.iter().enumerate() {
            cumulative += bucket.count;
            if cumulative >= target {
                let lower = if i > 0 { self.bucket_bounds[i - 1] } else { 0 };
                return (lower + self.bucket_bounds[i]) / 2;
            }
        }

        self.max_ns
    }

    /// Calculate lifetime statistics
    pub fn calculate_stats(&self) -> LifetimeStats {
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

        LifetimeStats {
            min_ns: if self.min_ns == u64::MAX { 0 } else { self.min_ns },
            max_ns: self.max_ns,
            mean_ns,
            median_ns: self.percentile(50.0),
            p90_ns: self.percentile(90.0),
            p99_ns: self.percentile(99.0),
            std_dev_ns,
            sample_count: self.total_samples,
        }
    }

    /// Predict if object will be short-lived
    #[inline]
    pub fn predict_short_lived(&self) -> f32 {
        if self.total_samples == 0 {
            return 0.5;
        }
        self.short_lived_count as f32 / self.total_samples as f32
    }

    /// Predict if object will be long-lived
    #[inline]
    pub fn predict_long_lived(&self) -> f32 {
        if self.total_samples == 0 {
            return 0.5;
        }
        self.long_lived_count as f32 / self.total_samples as f32
    }

    /// Recommend object placement strategy
    pub fn recommend_placement(&self) -> PlacementStrategy {
        let short_ratio = self.predict_short_lived();
        let long_ratio = self.predict_long_lived();

        if short_ratio > 0.8 {
            PlacementStrategy::CpuLocal
        } else if long_ratio > 0.5 {
            PlacementStrategy::NumaAware
        } else {
            PlacementStrategy::Default
        }
    }

    /// Get cache ID
    #[inline(always)]
    pub fn cache_id(&self) -> SlabCacheId {
        self.cache_id
    }

    /// Set thresholds
    #[inline(always)]
    pub fn set_thresholds(&mut self, short_ns: u64, long_ns: u64) {
        self.short_lived_threshold_ns = short_ns;
        self.long_lived_threshold_ns = long_ns;
    }

    /// Get sample count
    #[inline(always)]
    pub fn sample_count(&self) -> u64 {
        self.total_samples
    }
}
