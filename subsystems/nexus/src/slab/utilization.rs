//! Cache Utilization Analyzer
//!
//! This module provides utilization analysis for slab caches.

use alloc::string::String;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::SlabCacheId;

/// Utilization sample
#[derive(Debug, Clone, Copy)]
pub struct UtilizationSample {
    /// Timestamp
    pub timestamp: u64,
    /// Utilization (0-1)
    pub utilization: f32,
    /// Active objects
    pub active_objects: u64,
    /// Total objects
    pub total_objects: u64,
    /// Memory used
    pub memory_bytes: u64,
}

/// Utilization trend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtilizationTrend {
    /// Increasing utilization
    Increasing,
    /// Stable utilization
    Stable,
    /// Decreasing utilization
    Decreasing,
}

/// Resize recommendation
#[derive(Debug, Clone)]
pub enum ResizeRecommendation {
    /// Shrink cache
    Shrink { factor: f32, reason: String },
    /// Grow cache
    Grow { factor: f32, reason: String },
}

/// Cache utilization analyzer
#[repr(align(64))]
pub struct CacheUtilizationAnalyzer {
    /// Cache ID
    cache_id: SlabCacheId,
    /// Historical samples
    samples: VecDeque<UtilizationSample>,
    /// Maximum samples
    max_samples: usize,
    /// Exponential smoothing alpha
    alpha: f32,
    /// Smoothed utilization
    smoothed_utilization: f32,
    /// Low utilization threshold
    low_threshold: f32,
    /// High utilization threshold
    high_threshold: f32,
    /// Time below low threshold
    time_underutilized: u64,
    /// Time above high threshold
    time_overutilized: u64,
}

impl CacheUtilizationAnalyzer {
    /// Create new utilization analyzer
    pub fn new(cache_id: SlabCacheId) -> Self {
        Self {
            cache_id,
            samples: Vec::with_capacity(256),
            max_samples: 256,
            alpha: 0.2,
            smoothed_utilization: 0.5,
            low_threshold: 0.25,
            high_threshold: 0.90,
            time_underutilized: 0,
            time_overutilized: 0,
        }
    }

    /// Record utilization sample
    pub fn record_sample(&mut self, sample: UtilizationSample) {
        // Update exponential smoothing
        self.smoothed_utilization =
            self.alpha * sample.utilization + (1.0 - self.alpha) * self.smoothed_utilization;

        // Track time in states
        if let Some(prev) = self.samples.back() {
            let duration = sample.timestamp.saturating_sub(prev.timestamp);
            if sample.utilization < self.low_threshold {
                self.time_underutilized += duration;
            }
            if sample.utilization > self.high_threshold {
                self.time_overutilized += duration;
            }
        }

        // Store sample
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    /// Get smoothed utilization
    #[inline(always)]
    pub fn smoothed_utilization(&self) -> f32 {
        self.smoothed_utilization
    }

    /// Detect utilization trend
    pub fn detect_trend(&self) -> UtilizationTrend {
        if self.samples.len() < 10 {
            return UtilizationTrend::Stable;
        }

        let len = self.samples.len();
        let half = len / 2;

        let first_avg: f32 = self.samples[..half]
            .iter()
            .map(|s| s.utilization)
            .sum::<f32>()
            / half as f32;

        let second_avg: f32 = self.samples[half..]
            .iter()
            .map(|s| s.utilization)
            .sum::<f32>()
            / (len - half) as f32;

        let change = second_avg - first_avg;

        if change > 0.1 {
            UtilizationTrend::Increasing
        } else if change < -0.1 {
            UtilizationTrend::Decreasing
        } else {
            UtilizationTrend::Stable
        }
    }

    /// Recommend cache resize
    pub fn recommend_resize(&self) -> Option<ResizeRecommendation> {
        let util = self.smoothed_utilization;

        if util < self.low_threshold && self.time_underutilized > 60_000_000_000 {
            // 1 minute
            return Some(ResizeRecommendation::Shrink {
                factor: (self.low_threshold / util).min(2.0),
                reason: String::from("Persistently underutilized"),
            });
        }

        if util > self.high_threshold && self.time_overutilized > 30_000_000_000 {
            // 30 seconds
            return Some(ResizeRecommendation::Grow {
                factor: (util / self.high_threshold).min(2.0),
                reason: String::from("High utilization, risk of allocation failures"),
            });
        }

        None
    }

    /// Get cache ID
    #[inline(always)]
    pub fn cache_id(&self) -> SlabCacheId {
        self.cache_id
    }

    /// Set thresholds
    #[inline(always)]
    pub fn set_thresholds(&mut self, low: f32, high: f32) {
        self.low_threshold = low;
        self.high_threshold = high;
    }

    /// Get sample count
    #[inline(always)]
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }
}
